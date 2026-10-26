//! Live event authorization shared by polling, acknowledgment and every delivery attempt.
use super::agent_delivery::AgentEvent;
use crate::sql::{exists, query_all};
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
    query_all(
        conn,
        &format!(
            "SELECT e.* FROM agent_events e LEFT JOIN messages m ON m.id=e.message_id WHERE e.agent_id=?1 AND e.id>?2 AND e.outcome IN ('pending','delivered','acknowledged') AND ({}) ORDER BY e.id LIMIT ?3",
            authorized_sql("e", "m")
        ),
        params![agent_id, since, limit.unwrap_or(50).clamp(1, 100)],
        AgentEvent::from_row,
    )
}

/// Event-to-room/capability resolution is identical for every transport and enqueue.
pub fn authorized_sql(event: &str, message: &str) -> String {
    let approval = format!(
        "COALESCE({event}.agent_approval_id,CAST(json_extract({event}.metadata,'$.approval_id') AS INTEGER))"
    );
    let completion = format!(
        "{event}.event_type IN ('approval_decided','github_action_completed','fizzy_action_completed')"
    );
    let room = format!(
        "CASE WHEN {completion} THEN (SELECT event_approval.room_id FROM agent_approvals event_approval WHERE event_approval.id={approval} AND event_approval.agent_id={event}.agent_id) WHEN {event}.event_type IN ('mention','direct_message','reply') THEN {message}.room_id ELSE {event}.room_id END"
    );
    let cap = format!("CASE WHEN {completion} THEN 'external_action' ELSE 'read_messages' END");
    let user = format!("(SELECT user_id FROM agents WHERE id={event}.agent_id)");
    let capability =
        super::agent_access::event_capability_sql(&format!("{event}.agent_id"), &room, &cap);
    let thread = thread_readable_sql(event, message, &user);
    format!("({capability}) AND ({thread}) AND (({room}) IS NULL OR EXISTS (SELECT 1 FROM memberships event_member WHERE event_member.room_id=({room}) AND event_member.user_id={user})) AND (
        ({event}.event_type IN ('mention','direct_message','reply') AND {message}.id IS NOT NULL)
        OR ({event}.event_type IN ('work_assigned','work_unassigned','work_handed_off','slash_command') AND {event}.room_id IS NOT NULL)
        OR ({completion} AND EXISTS (SELECT 1 FROM agent_approvals event_approval WHERE event_approval.id={approval} AND event_approval.agent_id={event}.agent_id))
    )")
}

pub fn delivery_authorized(conn: &Connection, event_id: i64) -> Result<bool> {
    exists(
        conn,
        &format!(
            "SELECT 1 FROM agent_events e LEFT JOIN messages m ON m.id=e.message_id WHERE e.id=? AND ({})",
            authorized_sql("e", "m")
        ),
        [event_id],
    )
}

/// Current thread access for both the live message and a recorded work-event context.
/// Deleted public thread snapshots retain their management audit context; private ones do not.
/// Delivery's room and capability checks belong to authorized_sql.
/// SQL expressions are fixed by callers, never supplied by a request.
pub fn thread_readable_sql(event: &str, message: &str, user: &str) -> String {
    let thread = format!("CAST(json_extract({event}.metadata, '$.thread_id') AS INTEGER)");
    let message_access = format!(
        "EXISTS (SELECT 1 FROM channel_threads WHERE id={message}.thread_id AND visibility='public') OR {}",
        super::channel_thread::readable_by_sql(&format!("{message}.thread_id"), user)
    );
    let event_access = format!(
        "EXISTS (SELECT 1 FROM channel_threads WHERE id={thread} AND visibility='public') OR {}",
        super::channel_thread::readable_by_sql(&thread, user)
    );
    format!(
        "({message}.thread_id IS NULL OR {message_access}) AND ({event_access} OR (COALESCE(json_extract({event}.metadata, '$.thread_visibility'),'public')!='private' AND (json_extract({event}.metadata,'$.thread_id') IS NULL OR NOT EXISTS (SELECT 1 FROM channel_threads WHERE id={thread}))))"
    )
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
    query_all(
        conn,
        &format!(
            "SELECT e.* FROM agent_events e LEFT JOIN messages m ON m.id=e.message_id WHERE e.agent_id=?1 AND (?2 IS NULL OR e.outcome=?2) AND (?3 IS NULL OR e.created_at<?3 OR (e.created_at=?3 AND e.id<?4)) AND ({agent_access}) AND ({viewer_access}) ORDER BY e.created_at DESC,e.id DESC LIMIT 51",
            agent_access = thread_readable_sql("e", "m", "?5"),
            viewer_access = thread_readable_sql("e", "m", "?6"),
        ),
        params![agent_id, outcome, at, id, agent_user_id, viewer_id],
        AgentEvent::from_row,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Acknowledgment {
    Acknowledged { id: i64 },
    NotFound,
    Forbidden,
}

/// Idempotent polling acknowledgment uses the same live selection as delivery.
pub fn acknowledge(tx: &Tx<'_>, agent_id: i64, id: i64) -> Result<Acknowledgment> {
    let Some(event) = AgentEvent::find(tx.conn(), id)?.filter(|e| {
        e.agent_id == agent_id
            && DELIVERABLE_TYPES.contains(&e.event_type.as_str())
            && matches!(
                e.outcome.as_deref(),
                Some("pending" | "delivered" | "acknowledged")
            )
    }) else {
        return Ok(Acknowledgment::NotFound);
    };
    if !delivery_authorized(tx.conn(), event.id)? {
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
