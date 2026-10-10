//! Agents::DirectMessages. Typed results let REST and MCP supply their presenter.
use super::audit_log::{AuditLog, NewAuditLog};
use super::{
    agent_access,
    agent_posting::{DriveInput, PostResult},
    agent_service::ServiceResult,
};
use crate::broadcasts::{Broadcast};
use crate::sql::exists;
use crate::{Agent, Message, NewMessage, Result, Room, Tx, User};
use rusqlite::params;
use serde_json::json;

#[derive(Debug)]
pub enum DirectMessageResult {
    Posted {
        room: Box<Room>,
        message: Box<Message>,
    },
    Denied(ServiceResult),
}

pub fn allowed(tx: &Tx<'_>, agent: &Agent, target_id: i64) -> Result<bool> {
    if agent.owner_id == Some(target_id)
        || agent_access::capability_for_agent(tx.conn(), agent.id, "dm_anyone", None)?
    {
        return Ok(true);
    }
    // Even old/suppressed inbound message events establish this relationship.
    exists(
        tx.conn(),
        "SELECT 1 FROM agent_events WHERE agent_id=? AND actor_id=? AND event_type IN ('mention','reply','direct_message')",
        params![agent.id, target_id],
    )
}

pub fn open_and_post(
    tx: &mut Tx<'_>,
    agent_id: i64,
    target_id: i64,
    mut attributes: NewMessage,
    drive: DriveInput,
    audit: &super::audit_log::Context,
) -> Result<DirectMessageResult> {
    let fail =
        |error: &str, status| DirectMessageResult::Denied(ServiceResult::fail(error, status));
    let Some(target) = User::find_by_id(tx.conn(), target_id)? else {
        return Ok(fail("User not found", 404));
    };
    if target.is_bot() {
        return Ok(fail("Cannot open a DM with a bot", 422));
    }
    if !target.is_active() {
        return Ok(fail("Cannot open a DM with an inactive account", 422));
    }
    let agent = Agent::find(tx.conn(), agent_id)?.ok_or(crate::Error::RecordNotFound("Agent"))?;
    if !agent_access::has_capability_anywhere(tx.conn(), agent.id, "post_messages")? {
        return Ok(fail("Forbidden: agent lacks post_messages capability", 403));
    }
    if !allowed(tx, &agent, target.id)? {
        return Ok(fail(
            "Forbidden: agent may only DM its owner or humans who messaged it without the dm_anyone capability",
            403,
        ));
    }
    let members = [agent.user_id, target.id];
    let existing = Room::find_direct_for(tx.conn(), &members)?;
    let new_room = existing.is_none();
    let room = if let Some(room) = existing {
        room
    } else {
        Room::find_or_create_direct_for(tx, &members, agent.user_id)?
    };
    if !new_room
        && !agent_access::capability_for_agent(tx.conn(), agent.id, "post_messages", Some(room.id))?
    {
        return Ok(fail("Forbidden: agent lacks post_messages capability", 403));
    }
    if new_room {
        for membership in room.memberships(tx.conn())? {
            tx.emit_after_commit(crate::Event::broadcast(&Broadcast::MembershipChanged { membership_id: membership.id }));
        }
        let user = User::find(tx.conn(), agent.user_id)?;
        AuditLog::record(
            tx,
            NewAuditLog {
                action: "room.create".into(),
                actor: Some((&user).into()),
                actor_label: Some(format!("Agent {}", user.name)),
                target: Some((&room).into()),
                changes: Some(json!({"name":room.name})),
                ..Default::default()
            },
            audit,
        )?;
    }
    attributes.room_id = room.id;
    attributes.thread_id = None;
    match super::agent_posting::post_service(tx, agent.id, attributes, drive) {
        Ok(PostResult::Posted(message)) => Ok(DirectMessageResult::Posted {
            room: Box::new(room),
            message,
        }),
        Ok(PostResult::Denied(denial)) => Ok(DirectMessageResult::Denied(denial)),
        Err(crate::Error::RecordNotFound(_)) => Ok(fail("Reply target not found", 404)),
        Err(error) => Err(error),
    }
}
