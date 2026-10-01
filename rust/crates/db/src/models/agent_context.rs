//! Agents::ContextBuilder. Message presentation stays with the HTTP/UI adapter.
use super::{agent_access, agent_delivery::ruby_i64, agent_service::ServiceResult};
use crate::sql::{exists, query_all};
use crate::{Agent, ChannelThread, Connection, Message, Result, Room, Timestamp, User};
use rusqlite::params;
use serde_json::{Value, json};
use std::collections::HashMap;

pub fn build(
    conn: &Connection,
    agent_id: i64,
    message_id: Option<i64>,
    thread_id: Option<i64>,
    limit: Option<&Value>,
    now: Timestamp,
    mut presenter: impl FnMut(&Message) -> Result<Value>,
) -> Result<ServiceResult> {
    if message_id.is_none() && thread_id.is_none() {
        return Ok(ServiceResult::fail(
            "message_id or thread_id is required",
            422,
        ));
    }
    let mut limit = limit.map_or(30, ruby_i64);
    if limit < 1 {
        limit = 30;
    }
    limit = limit.min(100);
    let message = message_id
        .map(|id| Message::find_by_id(conn, id))
        .transpose()?
        .flatten();
    let thread = if message_id.is_some() {
        message
            .as_ref()
            .and_then(|m| m.thread_id)
            .map(|id| ChannelThread::find_by_id(conn, id))
            .transpose()?
            .flatten()
    } else {
        thread_id
            .map(|id| ChannelThread::find_by_id(conn, id))
            .transpose()?
            .flatten()
    };
    let missing = if message_id.is_some() {
        "Message not found"
    } else {
        "Thread not found"
    };
    let room_id = thread
        .as_ref()
        .map(|t| t.room_id)
        .or_else(|| message.as_ref().map(|m| m.room_id));
    let Some(room_id) = room_id else {
        return Ok(ServiceResult::fail(missing, 404));
    };
    let Some(agent) = Agent::find(conn, agent_id)? else {
        return Ok(ServiceResult::fail(missing, 404));
    };
    let room = Room::find_by_id(conn, room_id)?.filter(|r| r.deleted_at.is_none());
    let Some(room) = room else {
        return Ok(ServiceResult::fail(missing, 404));
    };
    if !exists(
        conn,
        "SELECT 1 FROM memberships WHERE room_id=? AND user_id=?",
        params![room.id, agent.user_id],
    )? {
        return Ok(ServiceResult::fail(missing, 404));
    }
    if !agent_access::capability_for_agent(conn, agent.id, "read_messages", Some(room.id))? {
        return Ok(ServiceResult::fail(
            "Forbidden: agent lacks read_messages capability",
            403,
        ));
    }
    if let (Some(message), Some(thread_id)) = (&message, thread_id)
        && message.thread_id.unwrap_or(0) != thread_id
    {
        return Ok(ServiceResult::fail(
            "Message is not in the given thread",
            422,
        ));
    }
    let ids = query_all(
        conn,
        "SELECT id FROM messages WHERE room_id=? AND ((? IS NULL AND thread_id IS NULL) OR thread_id=?) AND (? IS NULL OR id<=?) ORDER BY id DESC LIMIT ?",
        params![
            room.id,
            thread.as_ref().map(|t| t.id),
            thread.as_ref().map(|t| t.id),
            message_id,
            message_id,
            limit
        ],
        |r| r.get::<_, i64>(0),
    )?;
    let window = ids
        .into_iter()
        .rev()
        .map(|id| Message::find(conn, id))
        .collect::<Result<Vec<_>>>()?;
    let root = thread
        .as_ref()
        .and_then(|t| t.parent_message_id)
        .map(|id| Message::find_by_id(conn, id))
        .transpose()?
        .flatten();
    let mut authors = Vec::new();
    let mut flags = HashMap::new();
    for m in window.iter().chain(message.iter()).chain(root.iter()) {
        if let std::collections::hash_map::Entry::Vacant(entry) = flags.entry(m.creator_id)
            && let Some(user) = User::find_by_id(conn, m.creator_id)?
        {
            let f = json!({"agent":user.is_bot(),"human":!user.is_bot()});
            entry.insert(f);
            authors.push(
                json!({"id":user.id,"name":user.name,"agent":user.is_bot(),"human":!user.is_bot()}),
            );
        }
    }
    let mut present = |message: &Message| -> Result<Value> {
        let mut value = presenter(message)?;
        if let Some(creator) = value.get_mut("creator").and_then(Value::as_object_mut)
            && let Some(id) = creator.get("id").and_then(Value::as_i64)
            && let Some(f) = flags.get(&id).and_then(Value::as_object)
        {
            creator.extend(f.clone());
        }
        Ok(value)
    };
    let trigger = message
        .as_ref()
        .map(&mut present)
        .transpose()?
        .unwrap_or(Value::Null);
    let root = root
        .as_ref()
        .map(&mut present)
        .transpose()?
        .unwrap_or(Value::Null);
    let messages = window
        .iter()
        .map(&mut present)
        .collect::<Result<Vec<_>>>()?;
    let thread=thread.as_ref().map(|t|Ok::<_,crate::Error>(json!({"id":t.id,"name":t.name,"status":t.status(conn,now)?.name(),"room_id":t.room_id,"parent_message_id":t.parent_message_id}))).transpose()?;
    Ok(ServiceResult::ok(
        json!({"message":trigger,"thread":thread,"root_message":root,"messages":messages,"authors":authors,"room":{"id":room.id,"name":room.name,"purpose":null}}),
        200,
    ))
}
