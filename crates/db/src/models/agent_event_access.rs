//! Event selection and acknowledgment shared by the future REST and MCP callers.
//!
//! Selection deliberately matches AgentEvent.readable_by, including its lack of an
//! activity check. EventPolling drops unreadable payloads later, while still advancing
//! its cursor across the selected rows. Endpoint authentication remains in the caller.
use super::agent_delivery::AgentEvent;
use crate::sql::{exists, query_all, query_one};
use crate::{Connection, Result, Tx};
use rusqlite::params;

use super::agent_delivery::DELIVERABLE_TYPES;

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
        &format!("SELECT e.* FROM agent_events e LEFT JOIN messages m ON m.id=e.message_id
         WHERE e.agent_id=?1 AND e.id>?2 AND e.outcome IN ('pending','delivered','acknowledged') AND ({thread_access}) AND (
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
            thread_access = thread_readable_sql("e", "m", "?3")),
        params![agent_id, since, user_id, legacy, limit.unwrap_or(50).clamp(1, 100)],
        AgentEvent::from_row,
    )
}

/// Current thread access for both the live message and a recorded work-event context.
/// Public events retain Rails' room lifecycle rules; private events require live access.
/// Deleted public snapshots remain readable; deleted private snapshots do not.
/// SQL expressions are fixed by callers, never supplied by a request.
pub fn thread_readable_sql(event: &str, message: &str, user: &str) -> String {
    let thread = format!("CAST(json_extract({event}.metadata, '$.thread_id') AS INTEGER)");
    let message_access = format!("EXISTS (SELECT 1 FROM channel_threads WHERE id={message}.thread_id AND visibility='public') OR {}", super::channel_thread::readable_by_sql(&format!("{message}.thread_id"), user));
    let event_access = format!("EXISTS (SELECT 1 FROM channel_threads WHERE id={thread} AND visibility='public') OR {}", super::channel_thread::readable_by_sql(&thread, user));
    format!("({message}.thread_id IS NULL OR {message_access}) AND ({event_access} OR (COALESCE(json_extract({event}.metadata, '$.thread_visibility'),'public')!='private' AND (json_extract({event}.metadata,'$.thread_id') IS NULL OR NOT EXISTS (SELECT 1 FROM channel_threads WHERE id={thread}))))")
}

/// Management history filters both identities before its 50+1 keyset window.
pub fn history_page_for_viewer(
    conn: &Connection,
    agent_id: i64,
    agent_user_id: i64,
    viewer_id: i64,
    outcome: Option<&str>,
    before: Option<(crate::Timestamp, i64)>,
) -> Result<Vec<AgentEvent>> {
    let (at, id) = before.unzip();
    query_all(conn, &format!(
        "SELECT e.* FROM agent_events e LEFT JOIN messages m ON m.id=e.message_id WHERE e.agent_id=?1 AND (?2 IS NULL OR e.outcome=?2) AND (?3 IS NULL OR e.created_at<?3 OR (e.created_at=?3 AND e.id<?4)) AND ({agent_access}) AND ({viewer_access}) ORDER BY e.created_at DESC,e.id DESC LIMIT 51",
        agent_access = thread_readable_sql("e", "m", "?5"),
        viewer_access = thread_readable_sql("e", "m", "?6"),
    ), params![agent_id, outcome, at, id, agent_user_id, viewer_id], AgentEvent::from_row)
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
            && DELIVERABLE_TYPES.contains(&e.event_type.as_str())
            && e.outcome.as_deref().is_some_and(|o| o != "suppressed")
    }) else {
        return Ok(Acknowledgment::NotFound);
    };
    let agent_user: i64 = tx.conn().query_row("SELECT user_id FROM agents WHERE id=?", [agent_id], |row| row.get(0))?;
    if !exists(tx.conn(), &format!(
        "SELECT 1 FROM agent_events e LEFT JOIN messages m ON m.id=e.message_id WHERE e.id=?1 AND ({})",
        thread_readable_sql("e", "m", "?2"),
    ), params![event.id, agent_user])? {
        return Ok(Acknowledgment::NotFound);
    }
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
