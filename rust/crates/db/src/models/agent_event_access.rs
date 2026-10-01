//! Event selection and acknowledgment shared by the future REST and MCP callers.
//!
//! Selection deliberately matches AgentEvent.readable_by, including its lack of an
//! activity check. EventPolling drops unreadable payloads later, while still advancing
//! its cursor across the selected rows. Endpoint authentication remains in the caller.
use super::agent_delivery::AgentEvent;
use crate::sql::{exists, query_all, query_one};
use crate::{Connection, Result, Tx};
use rusqlite::params;

const DELIVERABLE: [&str; 10] = [
    "mention",
    "direct_message",
    "reply",
    "approval_decided",
    "github_action_completed",
    "fizzy_action_completed",
    "work_assigned",
    "work_unassigned",
    "work_handed_off",
    "slash_command",
];

/// Filter access in SQL before applying the page limit. A revoked row cannot hide a
/// newer readable event, and multiple matching grants cannot duplicate a row.
pub fn readable_page(
    conn: &Connection,
    agent_id: i64,
    since: i64,
    limit: Option<i64>,
) -> Result<Vec<AgentEvent>> {
    let Some(user_id) = query_one(
        conn,
        "SELECT user_id FROM agents WHERE id=?",
        [agent_id],
        |r| r.get::<_, i64>(0),
    )?
    else {
        return Ok(vec![]);
    };
    let legacy = !exists(
        conn,
        "SELECT 1 FROM agent_grants WHERE agent_id=?",
        [agent_id],
    )?;
    query_all(
        conn,
        "SELECT e.* FROM agent_events e LEFT JOIN messages m ON m.id=e.message_id
         WHERE e.agent_id=?1 AND e.id>?2 AND e.outcome IN ('pending','delivered','acknowledged') AND (
           (e.event_type IN ('mention','direct_message','reply') AND m.id IS NOT NULL
            AND EXISTS (SELECT 1 FROM memberships WHERE user_id=?3 AND room_id=m.room_id)
            AND (?4 OR EXISTS (SELECT 1 FROM agent_grants g WHERE g.agent_id=?1
                 AND g.capability='read_messages' AND g.revoked_at IS NULL
                 AND (g.room_id=m.room_id OR g.room_id IS NULL))))
           OR e.event_type IN ('approval_decided','github_action_completed','fizzy_action_completed')
           OR (e.event_type IN ('work_assigned','work_unassigned','work_handed_off','slash_command')
            AND EXISTS (SELECT 1 FROM memberships WHERE user_id=?3 AND room_id=e.room_id)
            AND (?4 OR EXISTS (SELECT 1 FROM agent_grants g WHERE g.agent_id=?1
                 AND g.capability='read_messages' AND g.revoked_at IS NULL
                 AND (g.room_id=e.room_id OR g.room_id IS NULL))))
         ) ORDER BY e.id LIMIT ?5",
        params![agent_id, since, user_id, legacy, limit.unwrap_or(50).clamp(1, 100)],
        AgentEvent::from_row,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Acknowledgment {
    Acknowledged { id: i64 },
    NotFound,
    Forbidden,
}

/// Idempotent polling acknowledgment. Work/slash/decision rows use the anywhere
/// capability gate, even when their room is no longer accessible; message rows also
/// require the live message's membership and the event room's capability. Polling
/// outcome changes never touch the separately claimed webhook state.
pub fn acknowledge(tx: &Tx<'_>, agent_id: i64, id: i64) -> Result<Acknowledgment> {
    let Some(event) = AgentEvent::find(tx.conn(), id)?.filter(|e| {
        e.agent_id == agent_id
            && DELIVERABLE.contains(&e.event_type.as_str())
            && e.outcome.as_deref().is_some_and(|o| o != "suppressed")
    }) else {
        return Ok(Acknowledgment::NotFound);
    };
    let allowed = if super::agent_delivery::MESSAGE_TYPES.contains(&event.event_type.as_str()) {
        let membership = exists(
            tx.conn(),
            "SELECT 1 FROM messages m JOIN memberships mm ON mm.room_id=m.room_id
             JOIN agents a ON a.user_id=mm.user_id WHERE m.id=? AND a.id=?",
            params![event.message_id, agent_id],
        )?;
        if !membership {
            return Ok(Acknowledgment::NotFound);
        }
        // event.room returns nil for a missing optional association in Rails.
        let room = match event.room_id {
            Some(id) if exists(tx.conn(), "SELECT 1 FROM rooms WHERE id=?", [id])? => Some(id),
            _ => None,
        };
        super::agent_access::capability_for_agent(tx.conn(), agent_id, "read_messages", room)?
    } else {
        super::agent_access::has_capability_anywhere(tx.conn(), agent_id, "read_messages")?
    };
    if !allowed {
        return Ok(Acknowledgment::Forbidden);
    }
    if event.outcome.as_deref() != Some("acknowledged") {
        tx.conn().execute(
            "UPDATE agent_events SET outcome='acknowledged' WHERE id=?",
            [id],
        )?;
    }
    Ok(Acknowledgment::Acknowledged { id })
}
