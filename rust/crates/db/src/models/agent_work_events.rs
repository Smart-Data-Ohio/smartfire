//! ChannelThread's agent assignment, handoff and deleted-work ledger callbacks.
//! Work mutation/authorization belongs to WS12; call these on its fresh locked row.
use super::agent_delivery::{AgentEvent, EventWebhookJob, NewEvent, WORK_TYPES};
use super::{agent_access, agent_payloads, bot_webhook_fanout};
use crate::sql::{exists, query_one};
use crate::{Agent, ChannelThread, Result, Tx, User};
use rusqlite::params;
use serde_json::{Value, json};

fn owner_agent(tx: &Tx<'_>, owner: Option<i64>) -> Result<Option<i64>> {
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
    if !WORK_TYPES.contains(&event.event_type.as_str()) || event.webhook_status != "none" {
        return Ok(());
    }
    let Some(room_id) = event.room_id else {
        return Ok(());
    };
    let Some(agent) = Agent::find(tx.conn(), event.agent_id)? else {
        return Ok(());
    };
    if !webhook_eligible(tx, agent.user_id, agent.id, room_id)? {
        return Ok(());
    }
    let changed=tx.conn().execute("UPDATE agent_events SET webhook_status='pending',webhook_next_attempt_at=? WHERE id=? AND webhook_status='none'",params![tx.now(),event.id])?;
    if changed != 0 {
        tx.emit_after_commit(crate::Event::job(&EventWebhookJob {
            event_id: event.id,
            attempt: Some(event.webhook_attempts),
        }));
    }
    Ok(())
}

fn webhook_eligible(tx: &Tx<'_>, user_id: i64, agent_id: i64, room_id: i64) -> Result<bool> {
    Ok(exists(tx.conn(), "SELECT 1 FROM memberships WHERE user_id=? AND room_id=?", params![user_id,room_id])?
        && agent_access::capability_for_agent(tx.conn(),agent_id,"read_messages",Some(room_id))?
        && exists(tx.conn(),"SELECT 1 FROM webhooks WHERE user_id=?",[user_id])?)
}

fn record(
    tx: &mut Tx<'_>,
    thread: &ChannelThread,
    agent_id: i64,
    kind: &str,
    actor_id: Option<i64>,
    hop_chain: (i64, &str),
    extra: Value,
) -> Result<AgentEvent> {
    let (hop, chain) = hop_chain;
    let actor = actor_id
        .map(|id| User::find_by_id(tx.conn(), id))
        .transpose()?
        .flatten();
    let mut metadata = json!({"thread_id":thread.id,"title":thread.name,"work_status":thread.work_status,"assigned_by":actor.map(|u|u.name),"hop":hop});
    if let Some(extra) = extra.as_object() {
        metadata
            .as_object_mut()
            .expect("object")
            .extend(extra.clone());
    }
    let suppressed = hop >= bot_webhook_fanout::HOP_LIMIT;
    let event = AgentEvent::create(
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
    enqueue_webhook(tx, &event)?;
    Ok(AgentEvent::find(tx.conn(), event.id)?.expect("created event"))
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
    let mut events = Vec::new();
    for (owner, kind) in [
        (from_owner_id, "work_unassigned"),
        (to_owner_id, "work_assigned"),
    ] {
        if let Some(agent) = owner_agent(tx, owner)? {
            events.push(record(
                tx,
                thread,
                agent,
                kind,
                actor_id,
                (hop, &chain),
                Value::Null,
            )?);
        }
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
    let mut events = Vec::new();
    if let Some(previous) = owner_agent(tx, from_owner_id)?
        && previous != receiver_agent_id
    {
        events.push(record(
            tx,
            thread,
            previous,
            "work_unassigned",
            sender_id,
            (hop, &chain),
            Value::Null,
        )?);
    }
    events.push(record(
        tx,
        thread,
        receiver_agent_id,
        "work_handed_off",
        sender_id,
        (hop, &chain),
        json!({"handoff":handoff_payload}),
    )?);
    Ok(events)
}

pub(crate) struct DeletedWork {
    agent_id: i64,
    snapshot: Value,
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
        let deliverable = agent.map(|agent| webhook_eligible(tx, agent.user_id, agent.id, thread.room_id)).transpose()?.unwrap_or(false);
        let reserved = if deliverable {Some(AgentEvent::reserve_id(tx)?)} else {None};
        // The durable job shares deletion's transaction; a rejected enqueue
        // still rolls back deletion. The ledger itself belongs after commit.
        if let Some(event_id)=reserved {
            tx.emit_after_commit(crate::Event::job(&EventWebhookJob {event_id,attempt:Some(0)}));
        }
        let event=NewEvent {
            agent_id:deleted.agent_id, room_id:Some(thread.room_id), actor_id,
            event_type:"work_unassigned".into(),outcome:Some("delivered".into()),
            chain_id:Some(uuid::Uuid::new_v4().to_string()),
            metadata:json!({"thread_id":thread.id,"title":thread.name,"work_status":thread.work_status,"assigned_by":deleted.snapshot["assigned_by"],"hop":0,"work_snapshot":deleted.snapshot}),
            ..Default::default()
        };
        tx.after_commit(move|tx|crate::database::run_write(tx.conn(),tx.env(),move|tx| {
            let event=AgentEvent::create_captured(tx,reserved,event)?;
            if reserved.is_some() {
                tx.conn().execute("UPDATE agent_events SET webhook_status='pending',webhook_next_attempt_at=? WHERE id=?",params![tx.now(),event.id])?;
            } else {enqueue_webhook(tx,&event)?;}
            Ok(())
        }));
    }
    Ok(())
}
