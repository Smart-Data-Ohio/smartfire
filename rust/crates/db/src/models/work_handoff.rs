//! `app/models/work_handoff.rb`: the normalized context package and shared receiver policy.
use crate::sql::{query_all, query_one};
use crate::{Agent, ChannelThread, Errors, Membership, Result, Timestamp, Tx, User};
use campfire_richtext::ruby::{is_blank, json_value_to_s, strip};
use rusqlite::{Connection, Row, params};
use serde_json::{Value, json};

pub const SUMMARY_LIMIT: usize = 2_000;
pub const COLLECTION_LIMIT: usize = 10;
pub const ENTRY_LIMIT: usize = 500;

#[derive(Debug, Clone)]
pub struct WorkHandoff {
    pub id: i64,
    pub channel_thread_id: i64,
    pub sender_id: i64,
    pub receiver_agent_id: i64,
    pub summary: String,
    pub links: Vec<String>,
    pub open_questions: Vec<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
#[derive(Debug, Clone)]
pub struct NewWorkHandoff {
    pub channel_thread_id: i64,
    pub sender_id: i64,
    pub receiver_agent_id: i64,
    pub summary: String,
    pub links: Value,
    pub open_questions: Value,
}
#[derive(Debug, Clone, Default)]
pub struct HandoffPackage {
    pub summary: String,
    pub links: Value,
    pub open_questions: Value,
}
impl WorkHandoff {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        let links: Value = row.get("links")?;
        let questions: Value = row.get("open_questions")?;
        Ok(Self {
            id: row.get("id")?,
            channel_thread_id: row.get("channel_thread_id")?,
            sender_id: row.get("sender_id")?,
            receiver_agent_id: row.get("receiver_agent_id")?,
            summary: row.get("summary")?,
            links: normalize_list(&links),
            open_questions: normalize_list(&questions),
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
    pub fn find(conn: &Connection, id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM work_handoffs WHERE id=?",
            [id],
            Self::from_row,
        )
    }
    pub fn for_thread(conn: &Connection, thread_id: i64) -> Result<Vec<Self>> {
        query_all(
            conn,
            "SELECT * FROM work_handoffs WHERE channel_thread_id=? ORDER BY created_at DESC,id DESC",
            [thread_id],
            Self::from_row,
        )
    }
    pub fn create(tx: &mut Tx<'_>, attributes: NewWorkHandoff) -> Result<Self> {
        let mut handoff = Self {
            id: 0,
            channel_thread_id: attributes.channel_thread_id,
            sender_id: attributes.sender_id,
            receiver_agent_id: attributes.receiver_agent_id,
            summary: attributes.summary,
            links: normalize_list(&attributes.links),
            open_questions: normalize_list(&attributes.open_questions),
            created_at: tx.now(),
            updated_at: tx.now(),
        };
        handoff.validate(tx.conn())?.into_result()?;
        handoff.id = tx.conn().query_row("INSERT INTO work_handoffs(channel_thread_id,sender_id,receiver_agent_id,summary,links,open_questions,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?) RETURNING id",
            params![handoff.channel_thread_id,handoff.sender_id,handoff.receiver_agent_id,handoff.summary,json!(handoff.links),json!(handoff.open_questions),tx.now(),tx.now()], |r|r.get(0))?;
        Ok(handoff)
    }
    pub fn validate(&self, conn: &Connection) -> Result<Errors> {
        let mut errors = Errors::default();
        if ChannelThread::find_by_id(conn, self.channel_thread_id)?.is_none() {
            errors.add("channel_thread", "must exist");
        }
        if User::find_by_id(conn, self.sender_id)?.is_none() {
            errors.add("sender", "must exist");
        }
        if Agent::find(conn, self.receiver_agent_id)?.is_none() {
            errors.add("receiver_agent", "must exist");
        }
        if is_blank(&self.summary) {
            errors.add("summary", "can't be blank");
        }
        if self.summary.chars().count() > SUMMARY_LIMIT {
            errors.add("summary", "is too long (maximum is 2000 characters)");
        }
        if self.links.len() > COLLECTION_LIMIT {
            errors.add("links", "are limited to 10 per handoff");
        }
        for link in &self.links {
            if link.chars().count() > ENTRY_LIMIT {
                errors.add("links", "must be at most 500 characters each");
                break;
            } else if !link
                .get(..7)
                .is_some_and(|s| s.eq_ignore_ascii_case("http://"))
                && !link
                    .get(..8)
                    .is_some_and(|s| s.eq_ignore_ascii_case("https://"))
            {
                errors.add("links", "must be http(s) URLs");
                break;
            }
        }
        if self.open_questions.len() > COLLECTION_LIMIT {
            errors.add("open_questions", "are limited to 10 per handoff");
        }
        for question in &self.open_questions {
            if question.chars().count() > ENTRY_LIMIT {
                errors.add("open_questions", "must be at most 500 characters each");
                break;
            }
        }
        Ok(errors)
    }
    pub fn receiver_error(
        conn: &Connection,
        thread: &ChannelThread,
        receiver: Option<&Agent>,
    ) -> Result<Option<&'static str>> {
        let Some(agent) = receiver else {
            return Ok(Some(
                "Receiver must be an active agent member of this room with permission to post",
            ));
        };
        if !agent.active(conn)?
            || Membership::find_by_room_and_user(conn, thread.room_id, agent.user_id)?.is_none()
            || !agent.can(conn, "post_messages", Some(thread.room_id))?
        {
            return Ok(Some(
                "Receiver must be an active agent member of this room with permission to post",
            ));
        }
        if !agent.can(conn, "manage_threads", Some(thread.room_id))? {
            return Ok(Some(
                "Receiver must hold the manage_threads capability in this room",
            ));
        }
        if !agent.can(conn, "read_messages", Some(thread.room_id))? {
            return Ok(Some(
                "Receiver must hold the read_messages capability in this room",
            ));
        }
        if thread.work_owner_id == Some(agent.user_id) {
            return Ok(Some("Receiver is already the owner of this work"));
        }
        Ok(None)
    }
    pub fn receivers_for(conn: &Connection, thread: &ChannelThread) -> Result<Vec<(String, i64)>> {
        let ids = query_all(conn, "SELECT agents.id FROM agents WHERE user_id IN (SELECT user_id FROM memberships WHERE room_id=?)", [thread.room_id], |row| row.get::<_, i64>(0))?;
        let mut receivers = Vec::new();
        for id in ids {
            let agent = Agent::find(conn, id)?.ok_or(crate::Error::RecordNotFound("Agent"))?;
            if Self::receiver_error(conn, thread, Some(&agent))?.is_none() {
                receivers.push((User::find(conn, agent.user_id)?.name, id));
            }
        }
        receivers.sort_by_key(|(name, _)| rails_compat::unicode::downcase(name));
        Ok(receivers)
    }
    pub fn payload(&self, conn: &Connection) -> Result<Value> {
        let sender = User::find_by_id(conn, self.sender_id)?;
        Ok(
            json!({"id":self.id,"summary":self.summary,"links":self.links,"open_questions":self.open_questions,
            "sender_name":sender.map(|u|u.name),"receiver_agent_id":self.receiver_agent_id}),
        )
    }
}

/// Ruby `Array(value)`, newline splitting, String#strip, blank rejection and stable uniq.
pub fn normalize_list(value: &Value) -> Vec<String> {
    let values = match value {
        Value::Null => Vec::new(),
        Value::Array(items) => items.clone(),
        Value::Object(map) => map.iter().map(|(key, value)| json!([key, value])).collect(),
        _ => vec![value.clone()],
    };
    let mut result = Vec::new();
    for value in values {
        for item in json_value_to_s(&value).split(['\r', '\n']) {
            let text = strip(item);
            if !is_blank(text) && !result.iter().any(|item| item == text) {
                result.push(text.to_owned());
            }
        }
    }
    result
}
