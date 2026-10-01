//! Agents::EventPolling. Request-specific message presentation is supplied by the
//! caller, keeping REST and MCP on the same domain without depending on HTML.
use super::agent_delivery::{AgentEvent, WORK_TYPES, ruby_i64};
use super::agent_payloads::{RepositoryAccess, compact, json_time};
use crate::sql::{query_all, query_one};
use crate::{ChannelThread, Connection, Message, Result, Timestamp};
use serde_json::{Value, json};
use std::collections::HashSet;

pub fn poll(
    conn: &Connection,
    agent_id: i64,
    since: Option<&Value>,
    limit: Option<&Value>,
    now: Timestamp,
    access: &RepositoryAccess,
    mut presenter: impl FnMut(&Message) -> Result<Value>,
) -> Result<Value> {
    let since = since.map_or(0, ruby_i64);
    let limit = limit.filter(|v| !blank(v)).map(ruby_i64);
    let events = super::agent_event_access::readable_page(conn, agent_id, since, limit)?;
    let next_since = events.last().map_or(since, |e| e.id);
    let context = PollContext::load(conn, agent_id, now, access)?;
    let mut payloads = vec![];
    for event in events {
        if let Some(payload) = context.payload(&event, &mut presenter)? {
            payloads.push(payload);
        }
    }
    if !access.valid(conn)? {
        return poll(conn,agent_id,Some(&json!(since)),limit.as_ref().map(|value|json!(value)).as_ref(),now,&RepositoryAccess::default(),presenter);
    }
    Ok(json!({"events":payloads,"next_since":next_since}))
}

fn blank(value: &Value) -> bool {
    match value {
        Value::Null | Value::Bool(false) => true,
        Value::String(s) => campfire_richtext::ruby::is_blank(s),
        Value::Array(a) => a.is_empty(),
        Value::Object(m) => m.is_empty(),
        _ => false,
    }
}

struct PollContext<'a> {
    conn: &'a Connection,
    agent_id: i64,
    owner_id: Option<i64>,
    now: Timestamp,
    access: &'a RepositoryAccess,
    active: bool,
    legacy: bool,
    members: HashSet<i64>,
    grants: HashSet<Option<i64>>,
}
impl<'a> PollContext<'a> {
    fn load(
        conn: &'a Connection,
        agent_id: i64,
        now: Timestamp,
        access: &'a RepositoryAccess,
    ) -> Result<Self> {
        let (user,owner_id,active,legacy):(i64,Option<i64>,bool,bool)=conn.query_row(
            "SELECT a.user_id,a.owner_id,(a.suspended_at IS NULL AND u.status=0),NOT EXISTS(SELECT 1 FROM agent_grants WHERE agent_id=a.id) FROM agents a JOIN users u ON u.id=a.user_id WHERE a.id=?",[agent_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))?;
        let members = query_all(
            conn,
            "SELECT room_id FROM memberships WHERE user_id=?",
            [user],
            |r| r.get(0),
        )?
        .into_iter()
        .collect();
        let grants=query_all(conn,"SELECT room_id FROM agent_grants WHERE agent_id=? AND capability='read_messages' AND revoked_at IS NULL",[agent_id],|r|r.get(0))?.into_iter().collect();
        Ok(Self {
            conn,
            agent_id,
            owner_id,
            now,
            access,
            active,
            legacy,
            members,
            grants,
        })
    }
    fn readable(&self, room: &Value) -> bool {
        let Some(id) = room.get("id").and_then(Value::as_i64) else {
            return false;
        };
        // Rails' preloaded poll check deliberately does not inspect deleted_at.
        self.active
            && self.members.contains(&id)
            && (self.legacy || self.grants.contains(&None) || self.grants.contains(&Some(id)))
    }
    fn common(&self, e: &AgentEvent) -> Result<Value> {
        let room = e
            .room_id
            .map(|id| {
                query_one(
                    self.conn,
                    "SELECT id,name FROM rooms WHERE id=?",
                    [id],
                    |r| Ok(json!({"id":r.get::<_,i64>(0)?,"name":r.get::<_,Option<String>>(1)?})),
                )
            })
            .transpose()?
            .flatten();
        let actor = e
            .actor_id
            .map(|id| {
                query_one(
                    self.conn,
                    "SELECT id,name FROM users WHERE id=?",
                    [id],
                    |r| Ok(json!({"id":r.get::<_,i64>(0)?,"name":r.get::<_,String>(1)?})),
                )
            })
            .transpose()?
            .flatten();
        Ok(compact(
            json!({"id":e.id,"event_type":e.event_type,"outcome":e.outcome,"created_at":json_time(e.created_at),"room":room,"actor":actor}),
        ))
    }
    fn payload(
        &self,
        e: &AgentEvent,
        presenter: &mut impl FnMut(&Message) -> Result<Value>,
    ) -> Result<Option<Value>> {
        let mut payload = self.common(e)?;
        if ["github_action_completed", "fizzy_action_completed"].contains(&e.event_type.as_str()) {
            payload[if e.event_type == "github_action_completed" {
                "github_action"
            } else {
                "fizzy_action"
            }] = compact(
                json!({"approval_id":e.metadata.get("approval_id"),"action":e.metadata.get("action"),"status":e.metadata.get("status"),"url":e.metadata.get("url"),"message":e.metadata.get("message")}),
            );
        } else if e.event_type == "approval_decided"
            || (e.message_id.is_none()
                && e.metadata
                    .get("approval_id")
                    .is_some_and(|v| !v.is_null() && *v != Value::Bool(false)))
        {
            let Some(approval) = self.approval(e)? else {
                return Ok(None);
            };
            payload["approval"] = approval;
        } else if WORK_TYPES.contains(&e.event_type.as_str()) {
            if !self.readable(&payload["room"]) {
                return Ok(None);
            };
            let thread = e
                .metadata
                .get("thread_id")
                .map(ruby_i64)
                .map(|id| ChannelThread::find_by_id(self.conn, id))
                .transpose()?
                .flatten();
            let work = if let Some(thread) = thread {
                let mut work = super::agent_payloads::work_payload(
                    self.conn,
                    &thread,
                    self.owner_id,
                    &self.access.in_event(e.id),
                )?;
                work["thread_id"] = json!(thread.id);
                work["status"] = json!(thread.work_status);
                work["assigned_by"] = e
                    .metadata
                    .get("assigned_by")
                    .cloned()
                    .unwrap_or(Value::Null);
                work
            } else {
                if e.event_type != "work_unassigned" {
                    return Ok(None);
                };
                let Some(snapshot) = e.metadata.get("work_snapshot").filter(|v| v.is_object())
                else {
                    return Ok(None);
                };
                payload["thread_deleted"] = json!(true);
                snapshot.clone()
            };
            payload["work"] = work;
            if e.event_type == "work_handed_off"
                && let Some(handoff) = e.metadata.get("handoff").and_then(Value::as_object)
            {
                let mut sliced = serde_json::Map::new();
                for key in [
                    "id",
                    "summary",
                    "links",
                    "open_questions",
                    "sender_name",
                    "receiver_agent_id",
                ] {
                    if let Some(value) = handoff.get(key) {
                        sliced.insert(key.into(), value.clone());
                    }
                }
                payload["handoff"] = Value::Object(sliced);
            }
        } else if e.event_type == "slash_command" {
            if !self.readable(&payload["room"]) {
                return Ok(None);
            };
            if let Some(id) = e.metadata.get("thread_id").filter(|v| !v.is_null()) {
                payload["thread_id"] = id.clone();
            }
            payload["command"] =
                json!({"name":e.metadata.get("command"),"arguments":e.metadata.get("arguments")});
        } else {
            if !self.readable(&payload["room"]) {
                return Ok(None);
            };
            let Some(message) = e
                .message_id
                .map(|id| Message::find_by_id(self.conn, id))
                .transpose()?
                .flatten()
            else {
                return Ok(None);
            };
            payload["hop"] = json!(e.hop());
            payload["message"] = presenter(&message)?;
            payload["pull_request"] = super::agent_payloads::pull_request_for_message(
                self.conn,
                &message,
                self.owner_id,
                &self.access.in_event(e.id),
            )?;
        }
        Ok(Some(payload))
    }
    fn approval(&self, e: &AgentEvent) -> Result<Option<Value>> {
        let Some(id) = e.metadata.get("approval_id").map(ruby_i64) else {
            return Ok(None);
        };
        query_one(
            self.conn,
            "SELECT a.*,u.name AS decider_name FROM agent_approvals a LEFT JOIN users u ON u.id=a.decided_by_id WHERE a.id=? AND a.agent_id=?",
            rusqlite::params![id, self.agent_id],
            |r| {
                let expires = r.get::<_, Option<Timestamp>>("expires_at")?;
                let status = r.get::<_, String>("status")?;
                let status = if status == "pending" && expires.is_some_and(|t| t <= self.now) {
                    "expired"
                } else {
                    &status
                };
                let decider = r
                    .get::<_, Option<String>>("decider_name")?
                    .map(Value::String)
                    .or_else(|| e.metadata.get("decided_by").cloned());
                let note = r
                    .get::<_, Option<String>>("decision_note")?
                    .map(Value::String)
                    .or_else(|| e.metadata.get("note").cloned());
                Ok(compact(
                    json!({"id":id,"approval_id":id,"action":r.get::<_,String>("action")?,"summary":r.get::<_,String>("summary")?,"status":status,"decided_by":decider,"note":note,"expires_at":expires.map(json_time),"room_id":r.get::<_,Option<i64>>("room_id")?}),
                ))
            },
        )
    }
}
