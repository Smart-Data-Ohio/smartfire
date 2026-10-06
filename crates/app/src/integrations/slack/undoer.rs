//! `app/models/slack_import/undoer.rb`: resumable deletion in reverse dependency order.
use campfire_db::models::slack_import::{IssueLevel, SlackImport};
use campfire_db::{ChannelThread, Database, Message, Result, Room, Tx, User};
use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};

use super::runner::{Outcome, array, integer, string, truthy};

pub const BATCH_SIZE: usize = 200;
const STEPS: [&str; 7] = [
    "leaves",
    "messages",
    "threads",
    "memberships",
    "rooms",
    "users",
    "records",
];

pub struct Undoer {
    pub db: Database,
    pub id: i64,
    pub lease: String,
}
#[derive(Clone)]
struct Record {
    id: i64,
    kind: String,
    record_id: i64,
    created: bool,
}
struct Undo<'a> {
    run: &'a SlackImport,
    state: Value,
    lease: &'a str,
}

impl Undoer {
    pub async fn step(self) -> anyhow::Result<Outcome> {
        Ok(self
            .db
            .write(move |tx| {
                let Some(run) = SlackImport::find(tx.conn(), self.id)? else {
                    return Ok(Outcome::Stopped);
                };
                if run.status != "undoing" || run.state["step_lease_token"] != self.lease {
                    return Ok(Outcome::Stopped);
                }
                let mut state = json!({"phase":"undo","undo_step":"leaves","undo_cursor":0,
                "undo_rooms_decided":false,"undo_messages_decided":false,
                "undo_kept_room_ids":[],"undo_kept_thread_ids":[],"undo_kept_message_ids":[]});
                state
                    .as_object_mut()
                    .expect("state")
                    .extend(run.state.as_object().cloned().unwrap_or_default());
                let mut undo = Undo {
                    run: &run,
                    state,
                    lease: &self.lease,
                };
                undo.step(tx)
            })
            .await?)
    }
}
impl Undo<'_> {
    fn step(&mut self, tx: &mut Tx<'_>) -> Result<Outcome> {
        let step = string(&self.state["undo_step"]);
        if step == "records" || !STEPS.contains(&step.as_str()) {
            return self.finish(tx);
        }
        if step == "messages" {
            self.decide_messages(tx)?;
        }
        if step == "memberships" {
            self.decide_rooms(tx)?;
        }
        let kinds: &[&str] = match step.as_str() {
            "leaves" => &["reaction", "pin", "thread_membership"],
            "messages" => &["message"],
            "threads" => &["thread"],
            "memberships" => &["membership"],
            "rooms" => &["conversation"],
            _ => &["user"],
        };
        let batch = self.batch(tx, kinds)?;
        if batch.is_empty() {
            self.state["undo_step"] =
                json!(STEPS[STEPS.iter().position(|s| *s == step).expect("step") + 1]);
            self.state["undo_cursor"] = json!(0);
        } else {
            if step == "messages" {
                self.keep_parents(tx, &batch)?;
            }
            // Rails find_each destroys model rows in primary-key order, independently of
            // the mapping cursor order used to select this batch.
            let mut ordered = batch.clone();
            ordered.sort_by_key(|r| r.record_id);
            for record in ordered {
                match step.as_str() {
                    "leaves" => {
                        let table = match record.kind.as_str() {
                            "reaction" => "boosts",
                            "pin" => "message_pins",
                            _ => "thread_memberships",
                        };
                        tx.conn().execute(
                            &format!("DELETE FROM {table} WHERE id=?"),
                            [record.record_id],
                        )?;
                    }
                    "messages" => {
                        if !self.kept("message", record.record_id) {
                            tx.conn().execute(
                                "DELETE FROM message_pins WHERE message_id=?",
                                [record.record_id],
                            )?;
                            if let Some(message) = Message::find_by_id(tx.conn(), record.record_id)?
                            {
                                self.destroy(
                                    tx,
                                    "message",
                                    message.id,
                                    || format!("Could not remove imported message {}", message.id),
                                    |tx| message.destroy_imported(tx),
                                )?;
                            }
                        }
                    }
                    "threads" => {
                        if let Some(thread) =
                            ChannelThread::find_by_id(tx.conn(), record.record_id)?
                        {
                            if self.kept("thread", thread.id) || self.thread_stays(tx, thread.id)? {
                                self.keep_thread(tx, &thread)?;
                            } else {
                                self.destroy(
                                    tx,
                                    "thread",
                                    thread.id,
                                    || format!("Could not remove imported thread {}", thread.id),
                                    |tx| thread.destroy_imported(tx),
                                )?;
                            }
                        }
                    }
                    "memberships" => {
                        let room = tx
                            .conn()
                            .query_row(
                                "SELECT room_id FROM memberships WHERE id=?",
                                [record.record_id],
                                |r| r.get::<_, i64>(0),
                            )
                            .optional()?;
                        if !room.is_some_and(|room| self.kept("room", room)) {
                            tx.conn().execute(
                                "DELETE FROM memberships WHERE id=?",
                                [record.record_id],
                            )?;
                        }
                    }
                    "rooms" => {
                        if record.created
                            && let Some(room) = Room::find_by_id(tx.conn(), record.record_id)?
                                .filter(|r| !r.deleted())
                            && !self.kept("room", room.id)
                        {
                            if self.room_stays(tx, room.id)? {
                                self.keep_room(tx, &room)?;
                            } else {
                                self.destroy(
                                    tx,
                                    "room",
                                    room.id,
                                    || format!("Could not remove imported room {}", room.id),
                                    |tx| room.destroy(tx),
                                )?;
                            }
                        }
                    }
                    "users" => {
                        if record.created
                            && let Some(user) = User::find_by_id(tx.conn(), record.record_id)?
                        {
                            let claimed: bool = tx.conn().query_row("SELECT EXISTS(SELECT 1 FROM sessions WHERE user_id=?1) OR EXISTS(SELECT 1 FROM google_identities WHERE user_id=?1) OR EXISTS(SELECT 1 FROM messages WHERE creator_id=?1)", [user.id], |r| r.get(0))?;
                            if !claimed
                                && user
                                    .password_digest
                                    .as_deref()
                                    .is_none_or(|s| s.trim().is_empty())
                            {
                                self.destroy(
                                    tx,
                                    "user",
                                    user.id,
                                    || format!("Could not remove placeholder user {}", user.id),
                                    |tx| user.destroy_for_slack_undo(tx),
                                )?;
                            }
                        }
                    }
                    _ => (),
                }
            }
            self.state["undo_cursor"] = json!(batch.last().expect("batch").id);
        }
        self.save(tx)?;
        Ok(Outcome::Continue)
    }
    fn batch(&self, tx: &Tx<'_>, kinds: &[&str]) -> Result<Vec<Record>> {
        let sql = format!(
            "SELECT id,slack_kind,record_id,created_record FROM slack_import_records WHERE slack_import_id=? AND id>? AND slack_kind IN ({}) ORDER BY id LIMIT {BATCH_SIZE}",
            vec!["?"; kinds.len()].join(",")
        );
        let mut values = vec![
            rusqlite::types::Value::Integer(self.run.id),
            rusqlite::types::Value::Integer(integer(&self.state["undo_cursor"])),
        ];
        values.extend(
            kinds
                .iter()
                .map(|kind| rusqlite::types::Value::Text((*kind).into())),
        );
        let mut q = tx.conn().prepare(&sql)?;
        Ok(
            q.query_map(rusqlite::params_from_iter(values), record_from_row)?
                .collect::<rusqlite::Result<_>>()?,
        )
    }
    fn records(&self, tx: &Tx<'_>) -> Result<Vec<Record>> {
        let mut q = tx.conn().prepare("SELECT id,slack_kind,record_id,created_record FROM slack_import_records WHERE slack_import_id=? ORDER BY id")?;
        Ok(q.query_map([self.run.id], |r| {
            Ok(Record {
                id: r.get(0)?,
                kind: r.get(1)?,
                record_id: r.get(2)?,
                created: r.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?)
    }
    fn kept(&self, kind: &str, id: i64) -> bool {
        array(&self.state[format!("undo_kept_{kind}_ids")]).contains(&json!(id))
    }
    fn mark(&mut self, kind: &str, id: i64) -> bool {
        if self.kept(kind, id) {
            return false;
        }
        self.state[format!("undo_kept_{kind}_ids")]
            .as_array_mut()
            .expect("kept ids")
            .push(json!(id));
        true
    }
    fn keep_message(&mut self, tx: &mut Tx<'_>, id: i64, what: &str) -> Result<()> {
        if self.mark("message", id) {
            SlackImport::record_issue(
                tx,
                self.run.id,
                IssueLevel::Warning,
                Some(&format!("message:{id}")),
                &format!("Message {id} kept: it holds {what} this import did not create"),
            )?;
        }
        Ok(())
    }
    fn keep_thread(&mut self, tx: &mut Tx<'_>, thread: &ChannelThread) -> Result<()> {
        if self.mark("thread", thread.id) {
            if let Some(id) = thread.parent_message_id {
                self.mark("message", id);
            }
            SlackImport::record_issue(
                tx,
                self.run.id,
                IssueLevel::Warning,
                Some(&format!("thread:{}", thread.id)),
                &format!(
                    "Thread {} kept: it holds messages this import did not create",
                    thread.id
                ),
            )?;
        }
        Ok(())
    }
    fn keep_room(&mut self, tx: &mut Tx<'_>, room: &Room) -> Result<()> {
        if self.mark("room", room.id) {
            SlackImport::record_issue(
                tx,
                self.run.id,
                IssueLevel::Warning,
                Some(&format!("room:{}", room.id)),
                &format!(
                    "Room {} kept: it holds content this import did not create",
                    room.name.clone().unwrap_or_else(|| room.id.to_string())
                ),
            )?;
        }
        Ok(())
    }
    fn decide_messages(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        if truthy(&self.state["undo_messages_decided"]) {
            return Ok(());
        }
        for (sql, what) in [
            ("SELECT message_id FROM polls", "a poll"),
            ("SELECT message_id FROM saved_items", "a saved item"),
            (
                "SELECT message_id FROM message_pins WHERE id NOT IN (SELECT record_id FROM slack_import_records WHERE slack_import_id=?1 AND slack_kind='pin')",
                "a pin",
            ),
            (
                "SELECT reply_to_message_id FROM scheduled_messages WHERE sent_at IS NULL AND dropped_at IS NULL",
                "a pending scheduled reply",
            ),
        ] {
            let sql = format!("SELECT message_id FROM ({sql}) AS content(message_id)");
            // SQLite doesn't accept derived column lists; alias the scheduled column directly.
            let sql = sql
                .replace(" AS content(message_id)", " AS content")
                .replace(
                    "SELECT reply_to_message_id FROM",
                    "SELECT reply_to_message_id AS message_id FROM",
                );
            let mut q = tx.conn().prepare(&format!("{sql} WHERE message_id IN (SELECT record_id FROM slack_import_records WHERE slack_import_id=?1 AND slack_kind='message')"))?;
            let ids = q
                .query_map([self.run.id], |r| r.get::<_, i64>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            drop(q);
            for id in ids {
                self.keep_message(tx, id, what)?;
            }
        }
        self.state["undo_messages_decided"] = json!(true);
        Ok(())
    }
    fn keep_parents(&mut self, tx: &mut Tx<'_>, batch: &[Record]) -> Result<()> {
        let mut q = tx.conn().prepare(&format!("SELECT id,parent_message_id FROM channel_threads WHERE parent_message_id IN ({}) ORDER BY id", vec!["?"; batch.len()].join(",")))?;
        let rows = q
            .query_map(
                rusqlite::params_from_iter(batch.iter().map(|r| r.record_id)),
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Option<i64>>(1)?)),
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(q);
        for (id, parent) in rows {
            if let Some(parent) = parent
                && batch.iter().any(|r| r.record_id == parent)
            {
                if self.owns(tx, "thread", id)? {
                    if self.thread_stays(tx, id)? {
                        self.keep_thread(tx, &ChannelThread::find(tx.conn(), id)?)?;
                    }
                } else {
                    self.keep_message(tx, parent, "a thread")?;
                }
            }
        }
        Ok(())
    }
    fn owns(&self, tx: &Tx<'_>, kind: &str, id: i64) -> Result<bool> {
        Ok(tx.conn().query_row("SELECT EXISTS(SELECT 1 FROM slack_import_records WHERE slack_import_id=? AND slack_kind=? AND record_id=?)", params![self.run.id,kind,id], |r|r.get(0))?)
    }
    fn thread_stays(&self, tx: &Tx<'_>, id: i64) -> Result<bool> {
        let mut q = tx
            .conn()
            .prepare("SELECT id FROM messages WHERE thread_id=?")?;
        let ids = q
            .query_map([id], |r| r.get::<_, i64>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(q);
        for id in ids {
            if self.kept("message", id) || !self.owns(tx, "message", id)? {
                return Ok(true);
            }
        }
        Ok(tx.conn().query_row("SELECT EXISTS(SELECT 1 FROM scheduled_messages WHERE thread_id=? AND sent_at IS NULL AND dropped_at IS NULL)", [id], |r|r.get(0))?)
    }
    fn decide_rooms(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        if truthy(&self.state["undo_rooms_decided"]) {
            return Ok(());
        }
        for (index, record) in self
            .records(tx)?
            .iter()
            .filter(|r| r.kind == "conversation" && r.created)
            .enumerate()
        {
            if index % 25 == 0 {
                SlackImport::refresh_step_lease(tx, self.run.id, self.lease)?;
            }
            if let Some(room) =
                Room::find_by_id(tx.conn(), record.record_id)?.filter(|r| !r.deleted())
                && self.room_stays(tx, room.id)?
            {
                self.keep_room(tx, &room)?;
            }
        }
        self.state["undo_rooms_decided"] = json!(true);
        Ok(())
    }
    fn room_stays(&self, tx: &Tx<'_>, id: i64) -> Result<bool> {
        for (table, kind) in [
            ("messages", "message"),
            ("message_pins", "pin"),
            ("channel_threads", "thread"),
        ] {
            if tx.conn().query_row(&format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE room_id=?1 AND id NOT IN (SELECT record_id FROM slack_import_records WHERE slack_import_id=?2 AND slack_kind=?3))"), params![id,self.run.id,kind], |r|r.get::<_,bool>(0))? { return Ok(true); }
        }
        for table in [
            "events",
            "scheduled_messages",
            "github_repository_subscriptions",
            "board_tag_assignments",
            "board_sla_rules",
            "board_sla_nudges",
            "board_stale_digests",
            "agent_slash_commands",
        ] {
            if tx.conn().query_row(
                &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE room_id=?)"),
                [id],
                |r| r.get::<_, bool>(0),
            )? {
                return Ok(true);
            }
        }
        for table in ["polls", "saved_items"] {
            if tx.conn().query_row(&format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE message_id IN (SELECT id FROM messages WHERE room_id=?))"), [id], |r|r.get::<_,bool>(0))? { return Ok(true); }
        }
        Ok(false)
    }
    fn destroy(
        &self,
        tx: &mut Tx<'_>,
        kind: &str,
        id: i64,
        prefix: impl FnOnce() -> String,
        operation: impl FnOnce(&mut Tx<'_>) -> Result<()>,
    ) -> Result<()> {
        if let Err(error) = tx.savepoint(operation) {
            SlackImport::record_issue(
                tx,
                self.run.id,
                IssueLevel::Error,
                Some(&format!("{kind}:{id}")),
                &format!("{}: {error}", prefix()),
            )?;
        }
        Ok(())
    }
    fn save(&self, tx: &mut Tx<'_>) -> Result<()> {
        tx.conn().execute(
            "UPDATE slack_imports SET state=?,heartbeat_at=?,updated_at=? WHERE id=?",
            params![self.state.to_string(), tx.now(), tx.now(), self.run.id],
        )?;
        SlackImport::refresh_step_lease(tx, self.run.id, self.lease)?;
        Ok(())
    }
    fn finish(&self, tx: &mut Tx<'_>) -> Result<Outcome> {
        tx.conn().execute("DELETE FROM slack_import_records WHERE slack_import_id=?1 AND NOT (
            (slack_kind='conversation' AND record_id IN (SELECT value FROM json_each(?2))) OR
            (slack_kind='membership' AND record_id IN (SELECT id FROM memberships WHERE room_id IN (SELECT value FROM json_each(?2)))) OR
            (slack_kind='thread' AND record_id IN (SELECT value FROM json_each(?3))) OR
            (slack_kind='message' AND record_id IN (SELECT value FROM json_each(?4))) OR
            (slack_kind='user' AND created_record=1 AND record_id IN (SELECT id FROM users)))",
            params![self.run.id,self.state["undo_kept_room_ids"].to_string(),self.state["undo_kept_thread_ids"].to_string(),self.state["undo_kept_message_ids"].to_string()])?;
        let count: i64 = tx.conn().query_row(
            "SELECT count(*) FROM slack_import_issues WHERE slack_import_id=?",
            [self.run.id],
            |r| r.get(0),
        )?;
        let mut stats = self.run.stats.clone();
        stats["phase"] = json!("done");
        stats["current"] = Value::Null;
        stats["issues_count"] = json!(count);
        tx.conn().execute("UPDATE slack_imports SET state=?,stats=?,status='undone',finished_at=?,heartbeat_at=?,updated_at=? WHERE id=?",params![json!({"phase":"done"}).to_string(),stats.to_string(),tx.now(),tx.now(),tx.now(),self.run.id])?;
        SlackImport::kick_next_queued(tx)?;
        Ok(Outcome::Done)
    }
}

fn record_from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Record> {
    Ok(Record {
        id: r.get(0)?,
        kind: r.get(1)?,
        record_id: r.get(2)?,
        created: r.get(3)?,
    })
}
