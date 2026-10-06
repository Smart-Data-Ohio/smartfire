//! Bounded, resumable phase runner (`app/models/slack_import/runner.rb`).
//! Fetches happen outside SQLite's writer transaction. The store checks the run again before
//! applying each fetched page, so cancellation during a slow request cannot publish that page.
use std::future::Future;
use std::time::Duration;

use campfire_db::models::slack_import::SlackImport;
use serde_json::{Value, json};
use tokio::time::Instant;

use super::client::{Bounds, Client};

pub const STEP_BUDGET: Duration = Duration::from_secs(25);
pub const CATCHUP_LOOKBACK: i64 = 30 * 24 * 60 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Continue,
    Done,
    Stopped,
}

#[derive(Clone)]
pub struct Progress {
    pub run: SlackImport,
    pub state: Value,
    pub stats: Value,
}
impl Progress {
    pub fn new(run: SlackImport) -> Self {
        let mut state = json!({"phase":"users", "users_cursor":null,"users_done":false,
            "conversations_cursor":null,"conversations":[],"conversation_ids":null,
            "convo_index":0,"convo":null});
        merge(&mut state, &run.state);
        let mut stats = json!({"phase":"users","users":{"matched":0,"placeholders":0,
            "deactivated":0,"bots":0,"total":0},"conversations":[],
            "counts":{"rooms_created":0,"rooms_merged":0,"messages":0,"replies":0,
                "threads":0,"reactions":0,"pins":0,"files_linked":0,"skipped":0},
            "current":null,"samples":[],"issues_count":0,"api_calls":0});
        merge(&mut stats, &run.stats);
        Self { run, state, stats }
    }
    pub fn dry_run(&self) -> bool {
        self.run.mode == "dry_run"
    }
    pub fn conversation(&self) -> Value {
        let id = string(&self.state["convo"]["id"]);
        array(&self.state["conversations"])
            .iter()
            .find(|v| string(&v["id"]) == id)
            .cloned()
            .unwrap_or_else(|| json!({"id":id}))
    }
    pub fn entry_mut(&mut self) -> &mut Value {
        let id = string(&self.state["convo"]["id"]);
        self.stats["conversations"]
            .as_array_mut()
            .expect("conversation stats")
            .iter_mut()
            .find(|v| string(&v["id"]) == id)
            .expect("discovered conversation")
    }
    pub fn add_counts(&mut self, delta: &Value) {
        if let Some(delta) = delta.as_object() {
            for (key, count) in delta {
                add(&mut self.stats["counts"][key], integer(count));
            }
        }
    }
}

/// Each operation and its progress commit share the same transaction. The implementation
/// owns user/room/message writes, while this module owns paging and the persisted state machine.
#[derive(Debug)]
pub enum Operation {
    Save,
    Users(Value),
    Resolve,
    Bounds,
    History(Value),
    Replies(Value),
    FinishThread,
    FinishRooms,
}
pub trait Store: Send + Sync {
    fn load(&self, id: i64) -> impl Future<Output = anyhow::Result<Option<SlackImport>>> + Send;
    fn commit(
        &self,
        progress: Progress,
        operation: Operation,
    ) -> impl Future<Output = anyhow::Result<Option<Progress>>> + Send;
}

pub struct Runner<S> {
    store: S,
    client: Client,
    progress: Progress,
    budget: Duration,
    deadline: Instant,
    base_api_calls: i64,
}
impl<S: Store> Runner<S> {
    pub fn new(store: S, client: Client, run: SlackImport) -> Self {
        let progress = Progress::new(run);
        let base_api_calls = integer(&progress.stats["api_calls"]);
        Self {
            store,
            client,
            progress,
            budget: STEP_BUDGET,
            deadline: Instant::now(),
            base_api_calls,
        }
    }
    #[cfg(any(test, feature = "test-support"))]
    pub fn with_budget(mut self, budget: Duration) -> Self {
        self.budget = budget;
        self
    }

    pub async fn step(mut self) -> anyhow::Result<Outcome> {
        self.deadline = Instant::now() + self.budget;
        let before = without_lease(self.progress.state.clone());
        if !self.running().await? {
            return Ok(Outcome::Stopped);
        }
        let result = self.dispatch().await;
        // Rails tolerates a uniqueness loser only when another execution actually committed
        // progress; a competing lease alone cannot hide a real validation/identity conflict.
        if let Err(error) = &result
            && error.chain().any(|e| {
                e.downcast_ref::<campfire_db::Error>()
                    .is_some_and(campfire_db::Error::is_record_not_unique)
            })
            && let Some(run) = self.store.load(self.progress.run.id).await?
            && without_lease(run.state) != before
        {
            return Ok(Outcome::Continue);
        }
        result
    }
    async fn dispatch(&mut self) -> anyhow::Result<Outcome> {
        match string(&self.progress.state["phase"]).as_str() {
            "users" => self.users().await,
            "conversations" => self.conversations().await,
            "messages" => self.messages().await,
            _ => {
                if !self.commit(Operation::FinishRooms).await? {
                    return Ok(Outcome::Stopped);
                }
                Ok(Outcome::Done)
            }
        }
    }
    async fn running(&self) -> anyhow::Result<bool> {
        Ok(self
            .store
            .load(self.progress.run.id)
            .await?
            .is_some_and(|r| r.status == "running"))
    }
    async fn commit(&mut self, operation: Operation) -> anyhow::Result<bool> {
        self.progress.stats["api_calls"] =
            json!(self.base_api_calls + self.client.request_count() as i64);
        match self.store.commit(self.progress.clone(), operation).await? {
            Some(progress) => {
                self.progress = progress;
                Ok(true)
            }
            None => Ok(false),
        }
    }
    fn over_budget(&self) -> bool {
        Instant::now() >= self.deadline
    }
    async fn transition(&mut self, phase: &str) -> anyhow::Result<Outcome> {
        self.progress.state["phase"] = json!(phase);
        self.progress.stats["phase"] = json!(phase);
        Ok(if self.commit(Operation::Save).await? {
            Outcome::Continue
        } else {
            Outcome::Stopped
        })
    }
    async fn users(&mut self) -> anyhow::Result<Outcome> {
        if truthy(&self.progress.state["users_done"]) {
            return self.transition("conversations").await;
        }
        let cursor = present(&self.progress.state["users_cursor"]);
        let page = self.client.users_list(cursor.as_deref(), 200).await?;
        self.progress.state["users_cursor"] = next_cursor(&page);
        self.progress.state["users_done"] = json!(self.progress.state["users_cursor"].is_null());
        Ok(
            if self
                .commit(Operation::Users(page["members"].clone()))
                .await?
            {
                Outcome::Continue
            } else {
                Outcome::Stopped
            },
        )
    }
    async fn conversations(&mut self) -> anyhow::Result<Outcome> {
        let types = if self.progress.run.kind == "personal" {
            "im,mpim,private_channel"
        } else if self.progress.run.options["include_private"] == false {
            "public_channel"
        } else {
            "public_channel,private_channel"
        };
        let cursor = present(&self.progress.state["conversations_cursor"]);
        let page = self
            .client
            .conversations_list(types, cursor.as_deref(), 200)
            .await?;
        for channel in array(&page["channels"]) {
            let only = array(&self.progress.run.options["conversation_ids"]);
            if !only.is_empty() && !only.contains(&channel["id"]) {
                continue;
            }
            let name =
                present(&channel["name"]).map_or_else(|| channel["id"].clone(), |s| json!(s));
            let entry = json!({"id":channel["id"],"name":name,"type":conversation_type(channel),
                "archived":truthy(&channel["is_archived"]),"is_private":truthy(&channel["is_private"]),
                "is_archived":truthy(&channel["is_archived"]),"is_im":truthy(&channel["is_im"]),
                "is_mpim":truthy(&channel["is_mpim"]),"is_channel":truthy(&channel["is_channel"]),
                "user":channel["user"],"num_members":channel["num_members"]});
            self.progress.state["conversations"]
                .as_array_mut()
                .expect("conversations")
                .push(entry);
        }
        self.progress.state["conversations_cursor"] = next_cursor(&page);
        if !self.progress.state["conversations_cursor"].is_null() {
            return Ok(if self.commit(Operation::Save).await? {
                Outcome::Continue
            } else {
                Outcome::Stopped
            });
        }
        let conversations = self.progress.state["conversations"]
            .as_array_mut()
            .expect("conversations");
        conversations.sort_by_key(|v| string(&v["id"]));
        let ids: Vec<_> = conversations.iter().map(|v| v["id"].clone()).collect();
        self.progress.stats["conversations"] =
            json!(conversations.iter().map(|v| json!({
            "id":v["id"],"name":v["name"],"type":v["type"],"archived":v["archived"],
            "members":v["num_members"].as_i64().unwrap_or(0),"messages":0,"threads":0,
            "target":{"action":"pending","room_id":null,"room_name":v["name"]},"done":false
        })).collect::<Vec<_>>());
        self.progress.state["conversation_ids"] = json!(ids);
        self.progress.state["convo_index"] = json!(0);
        self.transition("messages").await
    }
    async fn messages(&mut self) -> anyhow::Result<Outcome> {
        loop {
            let index = integer(&self.progress.state["convo_index"]) as usize;
            let ids = array(&self.progress.state["conversation_ids"]);
            let Some(id) = ids.get(index).cloned() else {
                return self.transition("finishing").await;
            };
            if self.progress.state["convo"]["id"] != id {
                self.progress.state["convo"] = json!({"id":id,"member_ids":[],"members_cursor":null,
                    "members_done":false,"resolved":false,"room_id":null,"skipped":false,
                    "direct":false,"history_cursor":null,"history_done":false,"thread_queue":[],
                    "thread_ts":null,"thread_message_id":null,"thread_cursor":null,"thread_state":{}});
            }
            let name = self.progress.entry_mut()["name"].clone();
            self.progress.stats["current"] = name;
            let id = string(&id);
            if self.progress.run.options["room_targets"][&id] == "skip" {
                self.progress.entry_mut()["target"] = json!({"action":"skip","room_id":null,
                    "room_name":self.progress.entry_mut()["name"]});
                if !self.advance().await? {
                    return Ok(Outcome::Stopped);
                }
            } else {
                while !truthy(&self.progress.state["convo"]["members_done"]) {
                    if !self.running().await? {
                        return Ok(Outcome::Stopped);
                    }
                    let cursor = present(&self.progress.state["convo"]["members_cursor"]);
                    let page = self
                        .client
                        .conversations_members(&id, cursor.as_deref(), 1000)
                        .await?;
                    let members = self.progress.state["convo"]["member_ids"]
                        .as_array_mut()
                        .expect("members");
                    for member in array(&page["members"]) {
                        if !members.contains(member) {
                            members.push(member.clone());
                        }
                    }
                    self.progress.state["convo"]["members_cursor"] = next_cursor(&page);
                    let done = self.progress.state["convo"]["members_cursor"].is_null();
                    self.progress.state["convo"]["members_done"] = json!(done);
                    if done {
                        self.progress.entry_mut()["members"] =
                            json!(array(&self.progress.state["convo"]["member_ids"]).len());
                    }
                    if !self.commit(Operation::Save).await? {
                        return Ok(Outcome::Stopped);
                    }
                    if done || self.over_budget() {
                        break;
                    }
                }
                if truthy(&self.progress.state["convo"]["members_done"]) {
                    if !truthy(&self.progress.state["convo"]["resolved"])
                        && !self.commit(Operation::Resolve).await?
                    {
                        return Ok(Outcome::Stopped);
                    }
                    if truthy(&self.progress.state["convo"]["skipped"]) {
                        if !self.advance().await? {
                            return Ok(Outcome::Stopped);
                        }
                    } else {
                        if self.progress.state["convo"].get("bounds").is_none()
                            && !self.commit(Operation::Bounds).await?
                        {
                            return Ok(Outcome::Stopped);
                        }
                        loop {
                            if thread_pending(&self.progress.state["convo"]) {
                                if !self.replies(&id).await? {
                                    return Ok(Outcome::Stopped);
                                }
                            } else if !truthy(&self.progress.state["convo"]["history_done"]) {
                                if !self.history(&id).await? {
                                    return Ok(Outcome::Stopped);
                                }
                            } else {
                                break;
                            }
                            if self.over_budget() {
                                break;
                            }
                        }
                        if truthy(&self.progress.state["convo"]["history_done"])
                            && !thread_pending(&self.progress.state["convo"])
                            && !self.advance().await?
                        {
                            return Ok(Outcome::Stopped);
                        }
                    }
                }
            }
            if !self.running().await? {
                return Ok(Outcome::Stopped);
            }
            if self.over_budget() {
                break;
            }
        }
        Ok(if self.commit(Operation::Save).await? {
            Outcome::Continue
        } else {
            Outcome::Stopped
        })
    }
    async fn advance(&mut self) -> anyhow::Result<bool> {
        self.progress.entry_mut()["done"] = json!(true);
        self.progress.state["convo_index"] =
            json!(integer(&self.progress.state["convo_index"]) + 1);
        self.progress.state["convo"] = Value::Null;
        self.progress.stats["current"] = Value::Null;
        self.commit(Operation::Save).await
    }
    async fn history(&mut self, id: &str) -> anyhow::Result<bool> {
        let convo = &self.progress.state["convo"];
        let oldest = slack_ts(&convo["bounds"]["oldest"]);
        let latest = slack_ts(&convo["bounds"]["latest"]);
        let cursor = present(&convo["history_cursor"]);
        let page = self
            .client
            .conversations_history(
                id,
                Bounds {
                    oldest: oldest.as_deref(),
                    latest: latest.as_deref(),
                },
                cursor.as_deref(),
                200,
            )
            .await?;
        self.progress.state["convo"]["history_cursor"] = next_cursor(&page);
        self.progress.state["convo"]["history_done"] =
            json!(self.progress.state["convo"]["history_cursor"].is_null());
        self.commit(Operation::History(page["messages"].clone()))
            .await
    }
    async fn replies(&mut self, id: &str) -> anyhow::Result<bool> {
        if self.progress.state["convo"]["thread_ts"].is_null() {
            let parent = self.progress.state["convo"]["thread_queue"]
                .as_array_mut()
                .expect("thread queue")
                .remove(0);
            self.progress.state["convo"]["thread_ts"] = parent["ts"].clone();
            self.progress.state["convo"]["thread_message_id"] = parent["message_id"].clone();
            self.progress.state["convo"]["thread_cursor"] = Value::Null;
            self.progress.state["convo"]["thread_state"] = json!({});
        }
        let convo = &self.progress.state["convo"];
        let ts = string(&convo["thread_ts"]);
        let oldest = slack_ts(&convo["bounds"]["oldest"]);
        let latest = slack_ts(&convo["bounds"]["latest"]);
        let cursor = present(&convo["thread_cursor"]);
        let page = self
            .client
            .conversations_replies(
                id,
                &ts,
                Bounds {
                    oldest: oldest.as_deref(),
                    latest: latest.as_deref(),
                },
                cursor.as_deref(),
                200,
            )
            .await?;
        self.progress.state["convo"]["thread_cursor"] = next_cursor(&page);
        if !self
            .commit(Operation::Replies(page["messages"].clone()))
            .await?
        {
            return Ok(false);
        }
        if self.progress.state["convo"]["thread_cursor"].is_null() {
            if !self.commit(Operation::FinishThread).await? {
                return Ok(false);
            }
            for key in ["thread_ts", "thread_message_id", "thread_cursor"] {
                self.progress.state["convo"][key] = Value::Null;
            }
            self.progress.state["convo"]["thread_state"] = json!({});
            return self.commit(Operation::Save).await;
        }
        Ok(true)
    }
}

pub fn string(value: &Value) -> String {
    value.as_str().map(str::to_owned).unwrap_or_default()
}
pub fn present(value: &Value) -> Option<String> {
    value
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .map(str::to_owned)
}
pub fn integer(value: &Value) -> i64 {
    value.as_i64().unwrap_or(0)
}
pub fn truthy(value: &Value) -> bool {
    !value.is_null() && *value != false
}
pub fn array(value: &Value) -> &[Value] {
    value.as_array().map(Vec::as_slice).unwrap_or(&[])
}
pub fn add(value: &mut Value, delta: i64) {
    *value = json!(integer(value) + delta);
}
pub fn conversation_type(v: &Value) -> &'static str {
    if truthy(&v["is_im"]) {
        "im"
    } else if truthy(&v["is_mpim"]) {
        "mpim"
    } else if truthy(&v["is_private"]) {
        "private_channel"
    } else {
        "public_channel"
    }
}
fn thread_pending(convo: &Value) -> bool {
    present(&convo["thread_ts"]).is_some() || !array(&convo["thread_queue"]).is_empty()
}
fn next_cursor(page: &Value) -> Value {
    present(&page["response_metadata"]["next_cursor"]).map_or(Value::Null, |s| json!(s))
}
fn slack_ts(value: &Value) -> Option<String> {
    value.as_f64().map(|v| format!("{v:.6}"))
}
fn merge(target: &mut Value, source: &Value) {
    if let (Some(target), Some(source)) = (target.as_object_mut(), source.as_object()) {
        for (key, value) in source {
            if let Some(old) = target.get_mut(key)
                && old.is_object()
                && value.is_object()
            {
                merge(old, value);
            } else {
                target.insert(key.clone(), value.clone());
            }
        }
    }
}
fn without_lease(mut state: Value) -> Value {
    if let Some(state) = state.as_object_mut() {
        state.remove("step_lease_token");
        state.remove("step_started_at");
    }
    state
}
