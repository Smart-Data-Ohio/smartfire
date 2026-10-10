//! Agents::ContextBuilder. Message presentation stays with the HTTP/UI adapter.
use super::{agent_access, agent_delivery::ruby_i64, agent_service::ServiceResult};
use crate::sql::exists;
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
    build_batched(
        conn,
        agent_id,
        message_id,
        thread_id,
        false,
        limit,
        now,
        |records| records.iter().map(&mut presenter).collect(),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_batched(
    conn: &Connection,
    agent_id: i64,
    message_id: Option<i64>,
    thread_id: Option<i64>,
    invalid_thread_constraint: bool,
    limit: Option<&Value>,
    now: Timestamp,
    mut presenter: impl FnMut(&[Message]) -> Result<Vec<Value>>,
) -> Result<ServiceResult> {
    build_batched_with_users(
        conn,
        agent_id,
        message_id,
        thread_id,
        invalid_thread_constraint,
        limit,
        now,
        |records| {
            let users = User::where_ids(
                conn,
                &records.iter().map(|m| m.creator_id).collect::<Vec<_>>(),
            )?
            .into_iter()
            .map(|u| (u.id, u))
            .collect();
            Ok((presenter(records)?, users))
        },
    )
}

/// The HTTP adapter shares its page-scoped author records with context serialization.
#[allow(clippy::too_many_arguments)]
pub fn build_batched_with_users(
    conn: &Connection,
    agent_id: i64,
    message_id: Option<i64>,
    thread_id: Option<i64>,
    invalid_thread_constraint: bool,
    limit: Option<&Value>,
    now: Timestamp,
    mut presenter: impl FnMut(&[Message]) -> Result<(Vec<Value>, HashMap<i64, User>)>,
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
    if message_id.is_some() && invalid_thread_constraint {
        return Err(crate::Error::Other(
            "thread_id does not support to_i".into(),
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
    let window = super::agent_reading::message_window(
        conn,
        room.id,
        thread.as_ref().map(|t| t.id),
        message.as_ref().map(|m| m.id),
        None,
        true,
        limit,
    )?;
    let root = thread
        .as_ref()
        .and_then(|t| t.parent_message_id)
        .map(|id| Message::find_by_id(conn, id))
        .transpose()?
        .flatten();
    let records = message
        .iter()
        .chain(root.iter())
        .chain(window.iter())
        .cloned()
        .collect::<Vec<_>>();
    let (mut values, users) = presenter(&records)?;
    let mut authors = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for m in window.iter().chain(message.iter()).chain(root.iter()) {
        if seen.insert(m.creator_id)
            && let Some(user) = users.get(&m.creator_id)
        {
            authors.push(
                json!({"id":user.id,"name":user.display_name(),"agent":user.is_bot(),"human":!user.is_bot()}),
            );
        }
    }
    for value in &mut values {
        if let Some(creator) = value.get_mut("creator").and_then(Value::as_object_mut)
            && let Some(user) = creator
                .get("id")
                .and_then(Value::as_i64)
                .and_then(|id| users.get(&id))
        {
            creator.insert("agent".into(), user.is_bot().into());
            creator.insert("human".into(), (!user.is_bot()).into());
        }
    }
    let mut values = values.into_iter();
    let trigger = if message.is_some() {
        values.next().unwrap_or(Value::Null)
    } else {
        Value::Null
    };
    let root = if root.is_some() {
        values.next().unwrap_or(Value::Null)
    } else {
        Value::Null
    };
    let messages = values.collect::<Vec<_>>();
    let thread=thread.as_ref().map(|t|Ok::<_,crate::Error>(json!({"id":t.id,"name":t.name,"status":t.status_in_room(&room,now).name(),"room_id":t.room_id,"parent_message_id":t.parent_message_id}))).transpose()?;
    Ok(ServiceResult::ok(
        json!({"message":trigger,"thread":thread,"root_message":root,"messages":messages,"authors":authors,"room":{"id":room.id,"name":room.name,"purpose":null}}),
        200,
    ))
}
