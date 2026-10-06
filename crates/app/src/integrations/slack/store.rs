//! SQL-backed transactional operations for the phase runner. No rendering or HTTP lives here.
use super::runner::{
    CATCHUP_LOOKBACK, Operation, Progress, Store, add, array, integer, present, string, truthy,
};
use super::{conversations, users, writer};
use campfire_db::models::slack_import::{IssueLevel, SlackImport};
use campfire_db::{Database, Error, Result, Room, Timestamp, Tx};
use indexmap::{IndexMap, IndexSet};
use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};
use std::collections::HashSet;

#[derive(Clone)]
pub struct SqlStore {
    pub db: Database,
    pub lease: Option<String>,
    pub allowed_domains: HashSet<String>,
}
impl Store for SqlStore {
    async fn load(&self, id: i64) -> anyhow::Result<Option<SlackImport>> {
        Ok(self.db.read(move |c| SlackImport::find(c, id)).await?)
    }
    async fn commit(
        &self,
        mut progress: Progress,
        operation: Operation,
    ) -> anyhow::Result<Option<Progress>> {
        let lease = self.lease.clone();
        let domains = self.allowed_domains.clone();
        Ok(self.db.write(move |tx|{
            let Some(fresh)=SlackImport::find(tx.conn(),progress.run.id)?else{return Ok(None);};
            if fresh.status!="running"||lease.as_ref().is_some_and(|token|fresh.state["step_lease_token"]!=*token){return Ok(None);}
            progress.run=fresh.clone();
            // Lease fields are changed by separate JSON mutations, never by a stale progress
            // snapshot. A heartbeat refresh cannot erase a page committed by another step.
            for key in ["step_lease_token","step_started_at"]{progress.state[key]=fresh.state[key].clone();}
            apply(tx,&mut progress,operation,&domains)?;
            if progress.state["phase"]=="done"{
                progress.state.as_object_mut().expect("state").remove("step_lease_token");
                progress.state.as_object_mut().expect("state").remove("step_started_at");
                tx.conn().execute("UPDATE slack_imports SET status='completed',state=?,stats=?,finished_at=?,heartbeat_at=?,updated_at=? WHERE id=? AND status='running'",
                    params![progress.state.to_string(),progress.stats.to_string(),tx.now(),tx.now(),tx.now(),progress.run.id])?;
                SlackImport::kick_next_queued(tx)?;
            }else{
                tx.conn().execute("UPDATE slack_imports SET state=?,stats=?,heartbeat_at=?,updated_at=? WHERE id=? AND status='running'",
                    params![progress.state.to_string(),progress.stats.to_string(),tx.now(),tx.now(),progress.run.id])?;
                if let Some(lease)=lease {SlackImport::refresh_step_lease(tx,progress.run.id,&lease)?;}
            }
            Ok(Some(progress))
        }).await?)
    }
}
fn apply(
    tx: &mut Tx<'_>,
    p: &mut Progress,
    operation: Operation,
    domains: &HashSet<String>,
) -> Result<()> {
    match operation {
        Operation::Save => {
            if p.state["phase"] == "messages" && p.run.state["phase"] == "conversations" {
                for id in array(&p.run.options["conversation_ids"]) {
                    if !array(&p.state["conversation_ids"]).contains(id) {
                        let id = string(id);
                        SlackImport::record_issue(
                            tx,
                            p.run.id,
                            IssueLevel::Warning,
                            Some(&format!("channel:{id}")),
                            &format!(
                                "Requested conversation {id} was not found in this run's scope; skipped"
                            ),
                        )?;
                    }
                }
            }
        }
        Operation::Users(members) => {
            let delta = users::map_page(tx, &p.run, &members, p.dry_run(), domains)?;
            for (key, count) in delta.as_object().expect("user counts") {
                add(&mut p.stats["users"][key], integer(count));
            }
        }
        Operation::Resolve => resolve(tx, p)?,
        Operation::Bounds => {
            let mut oldest = time_bound(&p.run.options["oldest"])?;
            if let Some(catchup) = catchup_oldest(tx, &p.run, &string(&p.state["convo"]["id"]))? {
                oldest = Some(oldest.map_or(catchup, |old| old.max(catchup)));
            }
            p.state["convo"]["bounds"] =
                json!({"oldest":oldest,"latest":time_bound(&p.run.options["latest"])?});
        }
        Operation::History(messages) => {
            let conversation = string(&p.state["convo"]["id"]);
            let bounds = writer::Bounds::from_value(&p.state["convo"]["bounds"]);
            let result = if p.dry_run() {
                let name = string(&p.entry_mut()["name"]);
                writer::dry_history(
                    tx,
                    &p.run,
                    &name,
                    array(&messages),
                    bounds,
                    20usize.saturating_sub(array(&p.stats["samples"]).len()),
                )?
            } else {
                note_written(p, &conversation);
                let room = alive_room(tx, integer(&p.state["convo"]["room_id"]))?;
                let mut users = convo_users(tx, p)?;
                writer::history(
                    tx,
                    &p.run,
                    &room,
                    &conversation,
                    array(&messages),
                    bounds,
                    &mut users,
                )?
            };
            p.add_counts(&result.counts);
            add(
                &mut p.entry_mut()["messages"],
                integer(&result.counts["messages"]),
            );
            let thread_count = if p.dry_run() {
                integer(&result.counts["threads"])
            } else {
                result.parents.len() as i64
            };
            add(&mut p.entry_mut()["threads"], thread_count);
            p.stats["samples"]
                .as_array_mut()
                .expect("samples")
                .extend(result.samples);
            let queue = p.state["convo"]["thread_queue"]
                .as_array_mut()
                .expect("thread queue");
            for parent in result.parents {
                if !queue.contains(&parent) {
                    queue.push(parent);
                }
            }
        }
        Operation::Replies(messages) => {
            let room = alive_room(tx, integer(&p.state["convo"]["room_id"]))?;
            let conversation = string(&p.state["convo"]["id"]);
            let parent = string(&p.state["convo"]["thread_ts"]);
            let mut users = convo_users(tx, p)?;
            let bounds = writer::Bounds::from_value(&p.state["convo"]["bounds"]);
            let counts = writer::replies(
                tx,
                &p.run,
                &room,
                writer::Replies {
                    conversation: &conversation,
                    parent_ts: &parent,
                    parent_id: integer(&p.state["convo"]["thread_message_id"]),
                    messages: array(&messages),
                    bounds,
                    state: &mut p.state["convo"]["thread_state"],
                },
                &mut users,
            )?;
            p.add_counts(&counts);
        }
        Operation::FinishThread => writer::finish_thread(
            tx,
            &p.run,
            &string(&p.state["convo"]["id"]),
            &string(&p.state["convo"]["thread_ts"]),
        )?,
        Operation::FinishRooms => {
            if !p.dry_run() {
                finish_rooms(tx, p)?;
            }
            p.state["phase"] = json!("done");
            p.stats["phase"] = json!("done");
            p.stats["current"] = Value::Null;
        }
    }
    let count: i64 = tx.conn().query_row(
        "SELECT count(*) FROM slack_import_issues WHERE slack_import_id=?",
        [p.run.id],
        |r| r.get(0),
    )?;
    p.stats["issues_count"] = json!(count);
    Ok(())
}
fn alive_room(tx: &Tx<'_>, id: i64) -> Result<Room> {
    Room::find_by_id(tx.conn(), id)?
        .filter(|r| !r.deleted())
        .ok_or(Error::RecordNotFound("Room"))
}
pub fn convo_users(tx: &mut Tx<'_>, p: &Progress) -> Result<IndexMap<String, campfire_db::User>> {
    let ids = array(&p.state["convo"]["member_ids"]);
    let mut users = users::users_for(tx.conn(), &p.run, ids)?;
    for id in ids {
        let key = string(id);
        if !users.contains_key(&key) {
            users.insert(key.clone(), users::ensure_author(tx, &p.run, &key)?);
        }
    }
    Ok(users)
}
fn resolve(tx: &mut Tx<'_>, p: &mut Progress) -> Result<()> {
    let id = string(&p.state["convo"]["id"]);
    let conversation = p.conversation();
    if !p.dry_run()
        && let Some(mapped) =
            users::mapped_id(tx.conn(), p.run.slack_workspace_id, "conversation", &id)?
    {
        if let Some(room) = Room::find_by_id(tx.conn(), mapped)?.filter(|r| !r.deleted()) {
            p.state["convo"]["room_id"] = json!(room.id);
            p.state["convo"]["direct"] = json!(room.direct());
            p.state["convo"]["resolved"] = json!(true);
            let name = room
                .name
                .clone()
                .filter(|n| !n.trim().is_empty())
                .map_or_else(|| p.entry_mut()["name"].clone(), |n| json!(n));
            p.entry_mut()["target"] = json!({"action":"merge","room_id":room.id,"room_name":name});
            return Ok(());
        }
        p.state["convo"]["resolved"] = json!(true);
        p.state["convo"]["skipped"] = json!(true);
        let name = p.entry_mut()["name"].clone();
        p.entry_mut()["target"] = json!({"action":"skip","room_id":null,"room_name":name});
        SlackImport::record_issue(
            tx,
            p.run.id,
            IssueLevel::Warning,
            Some(&format!("channel:{id}")),
            "mapped room was deleted; undo the earlier run or remove the mapping to re-import",
        )?;
        return Ok(());
    }
    let members = array(&p.state["convo"]["member_ids"]).to_vec();
    let users = if p.dry_run() {
        conversations::dry_users(tx, &p.run, &members)?
    } else {
        convo_users(tx, p)?
    };
    let target = conversations::resolve(tx, &p.run, &conversation, &members, &users, p.dry_run())?;
    if p.dry_run() {
        let choice = &p.run.options["room_targets"][&id];
        let explicit = choice.as_i64().or_else(|| {
            choice
                .as_str()
                .filter(|s| !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit()))
                .and_then(|s| s.parse().ok())
        });
        if let Some(room_id) = explicit {
            let kind = super::runner::conversation_type(&conversation);
            let message = if kind == "im" || (kind == "mpim" && members.len() <= 10) {
                Some(
                    "Room targets only apply to channels; this DM keeps its own Direct room".into(),
                )
            } else if target.reason == Some("invalid room target") {
                Some(format!(
                    "Room target {room_id} for #{} is not an alive Open or Closed room; skipped",
                    string(&conversation["name"])
                ))
            } else {
                None
            };
            if let Some(message) = message {
                SlackImport::record_issue(
                    tx,
                    p.run.id,
                    IssueLevel::Error,
                    Some(&format!("channel:{id}")),
                    &message,
                )?;
            }
        }
    } else if target.action == "create" {
        add(&mut p.stats["counts"]["rooms_created"], 1);
    } else if target.action == "merge" {
        add(&mut p.stats["counts"]["rooms_merged"], 1);
    }
    let room_id = target.room.as_ref().map(|r| r.id);
    let name = target
        .room
        .as_ref()
        .and_then(|r| r.name.clone())
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| conversations::describe(&conversation, members.len()));
    p.entry_mut()["target"] = json!({"action":target.action,"room_id":room_id,"room_name":name});
    p.state["convo"]["room_id"] = json!(room_id);
    p.state["convo"]["direct"] = json!(target.room.is_some_and(|r| r.direct()));
    p.state["convo"]["resolved"] = json!(true);
    p.state["convo"]["skipped"] = json!(target.action == "skip");
    Ok(())
}
fn time_bound(value: &Value) -> Result<Option<f64>> {
    present(value)
        .map(|s| {
            s.parse::<jiff::Timestamp>()
                .map(|t| t.as_second() as f64 + f64::from(t.subsec_microsecond()) / 1_000_000.0)
                .map_err(|e| Error::Other(e.to_string()))
        })
        .transpose()
}
fn note_written(p: &mut Progress, id: &str) {
    if p.state.get("written_conversation_ids").is_none() {
        p.state["written_conversation_ids"] = json!([]);
    }
    let ids = p.state["written_conversation_ids"]
        .as_array_mut()
        .expect("written conversations");
    if !ids.contains(&json!(id)) {
        ids.push(json!(id));
    }
}
fn catchup_oldest(tx: &Tx<'_>, run: &SlackImport, conversation: &str) -> Result<Option<f64>> {
    let mut stmt=tx.conn().prepare("SELECT id FROM slack_imports WHERE slack_workspace_id=? AND mode='import' AND status='completed' AND id != ?")?;
    let ids = stmt
        .query_map(params![run.slack_workspace_id, run.id], |r| {
            r.get::<_, i64>(0)
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut full = Vec::new();
    for id in ids {
        let r = SlackImport::find(tx.conn(), id)?.expect("run");
        if present(&r.options["oldest"]).is_none()
            && array(&r.stats["conversations"]).iter().any(|e| {
                string(&e["id"]) == conversation
                    && truthy(&e["done"])
                    && e["target"]["action"] != "skip"
            })
        {
            full.push(r);
        }
    }
    let mut stmt=tx.conn().prepare("SELECT id FROM slack_imports WHERE slack_workspace_id=? AND mode='import' AND status='undone'")?;
    let ids = stmt
        .query_map([run.slack_workspace_id], |r| r.get::<_, i64>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let undone = ids
        .into_iter()
        .map(|id| SlackImport::find(tx.conn(), id))
        .collect::<Result<Vec<_>>>()?;
    if !full.iter().any(|r| {
        r.finished_at.is_none_or(|finished| {
            !undone.iter().flatten().any(|u| {
                u.touched_conversation_ids().contains(conversation)
                    && u.started_at.is_some_and(|t| t < finished)
                    && u.finished_at.is_some_and(|t| t > finished)
            })
        })
    }) {
        return Ok(None);
    }
    let latest:Option<f64>=tx.conn().query_row("SELECT MAX(CAST(substr(slack_key,?) AS REAL)) FROM slack_import_records WHERE slack_workspace_id=? AND slack_kind='message' AND slack_key>=? AND slack_key<? AND slack_import_id != ?",params![conversation.chars().count() as i64+2,run.slack_workspace_id,format!("{conversation}:"),format!("{conversation};"),run.id],|r|r.get(0))?;
    Ok(latest.map(|v| v - CATCHUP_LOOKBACK as f64))
}
pub fn finish_rooms(tx: &mut Tx<'_>, p: &Progress) -> Result<()> {
    // Runner#finish_rooms concatenates written ids and mapping rows, then uniqs.
    let mut ids: IndexSet<_> = array(&p.state["written_conversation_ids"])
        .iter()
        .map(string)
        .collect();
    let mut statement=tx.conn().prepare("SELECT slack_key FROM slack_import_records WHERE slack_import_id=? AND slack_kind='conversation'")?;
    ids.extend(
        statement
            .query_map([p.run.id], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?,
    );
    drop(statement);
    for (index, id) in ids.into_iter().enumerate() {
        if index % 25 == 0
            && let Some(token) = present(&p.run.state["step_lease_token"])
        {
            SlackImport::refresh_step_lease(tx, p.run.id, &token)?;
        }
        let Some(room_id) =
            users::mapped_id(tx.conn(), p.run.slack_workspace_id, "conversation", &id)?
        else {
            continue;
        };
        let Some(room) = Room::find_by_id(tx.conn(), room_id)?.filter(|r| !r.deleted()) else {
            continue;
        };
        let last:Option<(i64,Timestamp)>=tx.conn().query_row("SELECT m.id,m.created_at FROM messages m JOIN slack_import_records r ON r.record_id=m.id WHERE r.slack_workspace_id=? AND r.slack_import_id=? AND r.slack_kind='message' AND r.slack_key>=? AND r.slack_key<? ORDER BY m.created_at DESC,m.id DESC LIMIT 1",params![p.run.slack_workspace_id,p.run.id,format!("{id}:"),format!("{id};")],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
        let Some((last_id, created)) = last else {
            continue;
        };
        let created_here:bool=tx.conn().query_row("SELECT slack_import_id=? AND created_record FROM slack_import_records WHERE slack_workspace_id=? AND slack_kind='conversation' AND slack_key=?",params![p.run.id,p.run.slack_workspace_id,id],|r|r.get(0))?;
        tx.conn().execute(
            "UPDATE rooms SET updated_at=? WHERE id=?",
            params![
                if created_here {
                    created
                } else {
                    room.updated_at.max(created)
                },
                room.id
            ],
        )?;
        let mut statement=tx.conn().prepare("SELECT r.record_id,r.slack_import_id,m.last_read_message_id,m.unread_at FROM slack_import_records r JOIN memberships m ON m.id=r.record_id WHERE r.slack_workspace_id=? AND r.slack_kind='membership' AND r.slack_key>=? AND r.slack_key<?")?;
        let memberships = statement
            .query_map(
                params![p.run.slack_workspace_id, format!("{id}:"), format!("{id};")],
                |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, i64>(1)?,
                        r.get::<_, Option<i64>>(2)?,
                        r.get::<_, Option<Timestamp>>(3)?,
                    ))
                },
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(statement);
        for (membership, owner, last_read, unread) in memberships {
            if owner != p.run.id {
                if unread.is_some() {
                    continue;
                }
                if let Some(last_read) = last_read
                    && let Some(current) = campfire_db::Message::find_by_id(tx.conn(), last_read)?
                    && (current.created_at, current.id) >= (created, last_id)
                {
                    continue;
                }
            }
            tx.conn().execute("UPDATE memberships SET last_read_message_id=?,unread_at=NULL,updated_at=? WHERE id=?",params![last_id,tx.now(),membership])?;
        }
    }
    Ok(())
}
pub fn allowed_domains() -> HashSet<String> {
    static DOMAIN: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(
            r"\A[a-z0-9](?:[a-z0-9-]*[a-z0-9])?(?:\.[a-z0-9](?:[a-z0-9-]*[a-z0-9])?)+\z",
        )
        .unwrap()
    });
    std::env::var("GOOGLE_SIGN_IN_DOMAINS")
        .unwrap_or_default()
        .split(',')
        .map(|s| {
            super::payload::downcase(s.trim_matches(|c: char| c.is_ascii_whitespace() || c == '\0'))
        })
        .filter(|s| DOMAIN.is_match(s))
        .collect()
}
