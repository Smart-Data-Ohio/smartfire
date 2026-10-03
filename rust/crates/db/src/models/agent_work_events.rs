//! ChannelThread's agent assignment, handoff and deleted-work ledger callbacks.
//! Work mutation/authorization belongs to WS12; call these on its fresh locked row.
use super::agent_delivery::{AgentEvent, EventWebhookJob, NewEvent, WORK_TYPES};
use super::{agent_payloads, bot_webhook_fanout};
use crate::sql::{exists, query_one};
use crate::{Agent, ChannelThread, Result, Tx, User};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

fn owner_agent(tx: &Tx<'_>, owner: Option<i64>) -> Result<Option<i64>> {
    let Some(owner) = owner else {
        return Ok(None);
    };
    query_one(
        tx.conn(),
        "SELECT agents.id FROM agents JOIN users ON users.id=agents.user_id WHERE users.id=? AND users.role=2",
        [owner],
        |r| r.get(0),
    )
}

/// Ledger rows survive loss of access. The optional webhook owes no private data
/// once current membership/read permission has gone away.
pub fn enqueue_webhook(tx: &mut Tx<'_>, event: &AgentEvent) -> Result<()> {
    queue_webhook(tx, event).map(|_| ())
}
fn queue_webhook(tx: &mut Tx<'_>, event: &AgentEvent) -> Result<Option<crate::Timestamp>> {
    if !WORK_TYPES.contains(&event.event_type.as_str()) || event.webhook_status != "none" {
        return Ok(None);
    }
    let Some(room_id) = event.room_id else {
        return Ok(None);
    };
    if !webhook_eligible(tx, event.agent_id, room_id)? {
        return Ok(None);
    }
    let next_attempt_at = tx.now();
    let changed=tx.conn().execute("UPDATE agent_events SET webhook_status='pending',webhook_next_attempt_at=? WHERE id=? AND webhook_status='none'",params![next_attempt_at,event.id])?;
    if changed != 0 {
        tx.emit_after_commit(crate::Event::job(&EventWebhookJob {
            event_id: event.id,
            attempt: Some(event.webhook_attempts),
        }));
    }
    Ok((changed != 0).then_some(next_attempt_at))
}
fn webhook_eligible(tx: &Tx<'_>, agent_id: i64, room_id: i64) -> Result<bool> {
    // One current statement, retaining suspended/user/deleted-room/revoked-grant
    // checks. No permission snapshot crosses the writer transaction.
    exists(tx.conn(),"SELECT 1 FROM agents a JOIN users u ON u.id=a.user_id
        LEFT JOIN rooms r ON r.id=? WHERE a.id=? AND a.suspended_at IS NULL AND u.status=0
        AND (r.id IS NULL OR r.deleted_at IS NULL)
        AND EXISTS(SELECT 1 FROM memberships m WHERE m.user_id=a.user_id AND m.room_id=?)
        AND EXISTS(SELECT 1 FROM webhooks w WHERE w.user_id=a.user_id)
        AND (NOT EXISTS(SELECT 1 FROM agent_grants g WHERE g.agent_id=a.id)
          OR EXISTS(SELECT 1 FROM agent_grants g WHERE g.agent_id=a.id AND g.capability='read_messages'
            AND g.revoked_at IS NULL AND (g.room_id IS NULL OR g.room_id=?)))",
        params![room_id,agent_id,room_id,room_id])
}

fn actor_name(tx: &Tx<'_>, actor_id: Option<i64>) -> Result<Option<String>> {
    actor_id
        .map(|id| User::find_by_id(tx.conn(), id))
        .transpose()
        .map(|actor| actor.flatten().map(|u| u.name))
}

fn record(
    tx: &mut Tx<'_>,
    thread: &ChannelThread,
    agent_id: i64,
    kind: &str,
    actor: (Option<i64>, Option<&str>),
    hop_chain: (i64, &str),
    extra: Value,
) -> Result<AgentEvent> {
    let (actor_id, actor_name) = actor;
    let (hop, chain) = hop_chain;
    let mut metadata = json!({"thread_id":thread.id,"title":thread.name,"work_status":thread.work_status,"assigned_by":actor_name,"hop":hop});
    if let Some(extra) = extra.as_object() {
        metadata
            .as_object_mut()
            .expect("object")
            .extend(extra.clone());
    }
    let suppressed = hop >= bot_webhook_fanout::HOP_LIMIT;
    let mut event = AgentEvent::create(
        tx,
        NewEvent {
            agent_id,
            room_id: Some(thread.room_id),
            actor_id,
            event_type: if suppressed {
                "delivery_suppressed_hop_limit"
            } else {
                kind
            }
            .into(),
            outcome: Some(
                if suppressed {
                    "suppressed"
                } else {
                    "delivered"
                }
                .into(),
            ),
            chain_id: Some(chain.into()),
            metadata,
            hop,
            detail: suppressed.then(|| format!("Hop limit reached (hop {hop})")),
            ..Default::default()
        },
    )?;
    if let Some(next_attempt_at) = queue_webhook(tx, &event)? {
        event.webhook_status = "pending".into();
        event.webhook_next_attempt_at = Some(next_attempt_at);
    }
    Ok(event)
}

pub fn record_owner_change(
    tx: &mut Tx<'_>,
    thread: &ChannelThread,
    from_owner_id: Option<i64>,
    to_owner_id: Option<i64>,
    actor_id: Option<i64>,
) -> Result<Vec<AgentEvent>> {
    if from_owner_id == to_owner_id {
        return Ok(Vec::new());
    }
    let (hop, chain) = bot_webhook_fanout::hop_and_chain_for_actor(tx, actor_id)?;
    let chain = chain.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let mut assignments = Vec::new();
    for (owner, kind) in [
        (from_owner_id, "work_unassigned"),
        (to_owner_id, "work_assigned"),
    ] {
        if let Some(agent) = owner_agent(tx, owner)? {
            assignments.push((agent, kind));
        }
    }
    if assignments.is_empty() {
        return Ok(Vec::new());
    }
    let actor_name = actor_name(tx, actor_id)?;
    let mut events = Vec::new();
    for (agent, kind) in assignments {
        events.push(record(
            tx,
            thread,
            agent,
            kind,
            (actor_id, actor_name.as_deref()),
            (hop, &chain),
            Value::Null,
        )?);
    }
    Ok(events)
}

pub fn record_handoff(
    tx: &mut Tx<'_>,
    thread: &ChannelThread,
    from_owner_id: Option<i64>,
    receiver_agent_id: i64,
    sender_id: Option<i64>,
    handoff_payload: Value,
) -> Result<Vec<AgentEvent>> {
    let (hop, chain) = bot_webhook_fanout::hop_and_chain_for_actor(tx, sender_id)?;
    let chain = chain.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let actor_name = actor_name(tx, sender_id)?;
    let mut events = Vec::new();
    if let Some(previous) = owner_agent(tx, from_owner_id)?
        && previous != receiver_agent_id
    {
        events.push(record(
            tx,
            thread,
            previous,
            "work_unassigned",
            (sender_id, actor_name.as_deref()),
            (hop, &chain),
            Value::Null,
        )?);
    }
    events.push(record(
        tx,
        thread,
        receiver_agent_id,
        "work_handed_off",
        (sender_id, actor_name.as_deref()),
        (hop, &chain),
        json!({"handoff":handoff_payload}),
    )?);
    Ok(events)
}

pub(crate) struct DeletedWork {
    agent_id: i64,
    snapshot: Value,
}

/// The durable webhook starts with the captured deletion, not a reserved ledger
/// ID. Its arguments are bound to the published event in the ledger's write.
#[derive(Clone, Serialize, Deserialize)]
pub struct DeletedWorkWebhookJob {
    deleted_work: NewEvent,
}
impl crate::Job for DeletedWorkWebhookJob {
    const CLASS: &'static str = "Agent::EventWebhookJob";
}

/// Only after_destroy_commit publishes the ledger, as Rails does. SQLite
/// allocates the ID at insertion; the durable runner never recreates a missing row.
pub fn publish_deleted_webhook(
    tx: &mut Tx<'_>,
    job: DeletedWorkWebhookJob,
) -> Result<EventWebhookJob> {
    let captured = job.deleted_work;
    let chain = captured.chain_id.clone();
    let agent_id = captured.agent_id;
    let event = query_one(
        tx.conn(),
        "SELECT * FROM agent_events WHERE agent_id=? AND chain_id=? AND event_type='work_unassigned'",
        params![agent_id, chain],
        AgentEvent::from_row,
    )?;
    let event = match event {
        Some(event) => event,
        None => {
            let event = AgentEvent::create_captured(tx, captured)?;
            if let (Some(agent), Some(room)) =
                (Agent::find(tx.conn(), event.agent_id)?, event.room_id)
                && webhook_eligible(tx, agent.id, room)?
            {
                tx.conn().execute("UPDATE agent_events SET webhook_status='pending',webhook_next_attempt_at=? WHERE id=?",params![tx.now(),event.id])?;
            }
            event
        }
    };
    let bound = EventWebhookJob {
        event_id: event.id,
        attempt: Some(event.webhook_attempts),
    };
    // Replacing the intent commits with the ledger, so retries and later removal
    // use the ordinary missing-event/attempt guards rather than republishing it.
    tx.conn().execute("UPDATE background_jobs SET arguments=?,updated_at=? WHERE job_class='Agent::EventWebhookJob' AND json_extract(arguments,'$.deleted_work.chain_id')=? AND json_extract(arguments,'$.deleted_work.agent_id')=?",
        params![json!(bound),tx.now(),chain,agent_id])?;
    Ok(bound)
}
pub(crate) fn capture_deleted(
    tx: &Tx<'_>,
    thread: &ChannelThread,
    actor_id: Option<i64>,
) -> Result<Option<DeletedWork>> {
    let Some(agent_id) = owner_agent(tx, thread.work_owner_id)? else {
        return Ok(None);
    };
    // Rails captures without an Agent collaborator: private PR data is redacted.
    let mut snapshot = agent_payloads::work_payload(tx.conn(), thread, None, &Default::default())?;
    snapshot["thread_id"] = json!(thread.id);
    snapshot["status"] = json!(thread.work_status);
    snapshot["assigned_by"] = json!(
        actor_id
            .map(|id| User::find_by_id(tx.conn(), id))
            .transpose()?
            .flatten()
            .map(|u| u.name)
    );
    Ok(Some(DeletedWork { agent_id, snapshot }))
}
pub(crate) fn record_deleted(
    tx: &mut Tx<'_>,
    thread: &ChannelThread,
    actor_id: Option<i64>,
    deleted: Option<DeletedWork>,
) -> Result<()> {
    if let Some(deleted) = deleted {
        let agent = Agent::find(tx.conn(), deleted.agent_id)?;
        let deliverable = agent
            .map(|agent| webhook_eligible(tx, agent.id, thread.room_id))
            .transpose()?
            .unwrap_or(false);
        let event = NewEvent {
            agent_id: deleted.agent_id,
            room_id: Some(thread.room_id),
            actor_id,
            event_type: "work_unassigned".into(),
            outcome: Some("delivered".into()),
            chain_id: Some(uuid::Uuid::new_v4().to_string()),
            metadata: json!({"thread_id":thread.id,"title":thread.name,"work_status":thread.work_status,"assigned_by":deleted.snapshot["assigned_by"],"hop":0,"work_snapshot":deleted.snapshot}),
            ..Default::default()
        };
        // The durable job shares deletion's transaction; a rejected enqueue
        // still rolls back deletion. Only insertion publishes a polling ID.
        let job = DeletedWorkWebhookJob {
            deleted_work: event.clone(),
        };
        if deliverable {
            tx.emit_after_commit(crate::Event::job(&job));
        }
        tx.after_commit_record("channel_threads", thread.id, move |tx| {
            crate::database::run_write(tx.conn(), tx.env(), move |tx| {
                if deliverable {
                    publish_deleted_webhook(tx, job)?;
                } else {
                    let event = AgentEvent::create_captured(tx, event)?;
                    enqueue_webhook(tx, &event)?;
                }
                Ok(())
            })
        });
    }
    Ok(())
}
