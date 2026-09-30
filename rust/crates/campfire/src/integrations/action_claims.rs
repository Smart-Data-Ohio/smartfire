//! Shared GitHub/Fizzy execution-claim outcomes and stuck-claim recovery.
use campfire_db::{Database, Event, Timestamp, Tx};
use campfire_jobs::claims::ConditionalUpdate;
use rusqlite::{OptionalExtension, params, types::Value as SqlValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const STUCK_CLAIM_AFTER: jiff::SignedDuration = jiff::SignedDuration::from_mins(15);
pub const SWEEP_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);
#[derive(Clone, Copy)]
pub struct Integration {
    pub event_type: &'static str,
    pub audit_action: &'static str,
    pub timeout_message: &'static str,
}
pub const GITHUB: Integration = Integration {
    event_type: "github_action_completed",
    audit_action: "agent.github_action.execute",
    timeout_message: "GitHub action execution timed out",
};
#[allow(dead_code, reason = "WS15e reuses the shared helper for Fizzy")]
pub const FIZZY: Integration = Integration {
    event_type: "fizzy_action_completed",
    audit_action: "agent.fizzy_action.execute",
    timeout_message: "Fizzy action execution timed out",
};

/// WS11's Agent::EventWebhookJob contract; actual delivery belongs to its agent runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventWebhookJob {
    pub event_id: i64,
    pub attempt: Option<i64>,
}
impl campfire_db::Job for EventWebhookJob {
    const CLASS: &'static str = "Agent::EventWebhookJob";
}

/// Rails' strict age cutoff and per-row recovery. A malformed/deleted row or a failed
/// transaction cannot prevent the remaining claims from being recovered.
pub async fn recover_stuck_claims(
    db: &Database,
    integration: Integration,
    now: Timestamp,
) -> usize {
    let candidates = db.read(move |conn| {
        let mut statement = conn.prepare_cached("SELECT id FROM agent_events WHERE event_type = ? AND created_at < ? AND json_extract(metadata, '$.status') = 'running' ORDER BY id")?;
        Ok(statement.query_map(params![integration.event_type, now.ago(STUCK_CLAIM_AFTER)], |row| row.get::<_, i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?)
    }).await;
    let candidates = match candidates {
        Ok(ids) => ids,
        Err(error) => {
            tracing::error!(%error, event_type = integration.event_type, "Stuck integration claim lookup failed");
            return 0;
        }
    };
    let mut recovered = 0;
    for event_id in candidates {
        let result = db
            .write(move |tx| {
                let stored = tx
                    .conn()
                    .query_row(
                        "SELECT metadata FROM agent_events WHERE id = ?",
                        [event_id],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()?;
                let Some(Value::Object(mut metadata)) =
                    stored.and_then(|text| serde_json::from_str(&text).ok())
                else {
                    return Ok(false);
                };
                if metadata.get("status").and_then(Value::as_str) != Some("running") {
                    return Ok(false);
                }
                metadata.insert("status".into(), Value::String("failed".into()));
                metadata.insert(
                    "message".into(),
                    Value::String(integration.timeout_message.into()),
                );
                if !rewrite_running(
                    tx,
                    event_id,
                    Value::Object(metadata),
                    Some(integration.timeout_message.into()),
                )? {
                    return Ok(false);
                }
                // Rails rescues audit errors separately: an audit outage must not suppress delivery.
                if let Err(error) = record_execution_audit(tx, event_id, integration) {
                    tracing::error!(%error, event_id, "Stuck integration claim audit failed");
                }
                Ok(true)
            })
            .await;
        match result {
            Ok(true) => {
                recovered += 1;
                tracing::error!(
                    event_id,
                    event_type = integration.event_type,
                    "Stuck integration claim marked failed"
                );
            }
            Ok(false) => {}
            Err(error) => {
                tracing::error!(%error, event_id, "Stuck integration claim recovery failed")
            }
        }
    }
    recovered
}

/// Both the worker and sweeper use the same first-writer-wins condition. Durable
/// webhook enqueue and the winning rewrite commit together; losing writers emit nothing.
pub fn rewrite_running(
    tx: &mut Tx<'_>,
    event_id: i64,
    metadata: Value,
    detail: Option<String>,
) -> campfire_db::Result<bool> {
    let state = tx.conn().query_row("SELECT webhook_attempts, EXISTS (SELECT 1 FROM agents JOIN webhooks ON webhooks.user_id = agents.user_id WHERE agents.id = agent_events.agent_id) FROM agent_events WHERE id = ?", [event_id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, bool>(1)?))).optional()?;
    let Some((attempt, pending_webhook)) = state else {
        return Ok(false);
    };
    let won = ConditionalUpdate::table("agent_events")
        .set("detail", detail.map_or(SqlValue::Null, SqlValue::Text))
        .set("metadata", metadata.to_string())
        .set(
            "webhook_status",
            if pending_webhook { "pending" } else { "none" }.to_owned(),
        )
        .set(
            "webhook_next_attempt_at",
            if pending_webhook {
                SqlValue::Text(tx.now().to_db())
            } else {
                SqlValue::Null
            },
        )
        .id(event_id)
        .condition(
            "json_extract(agent_events.metadata, '$.status') = 'running'",
            [],
        )
        .claim(tx.conn())?;
    if won && pending_webhook {
        tx.emit_after_commit(Event::job(&EventWebhookJob {
            event_id,
            attempt: Some(attempt),
        }));
    }
    Ok(won)
}

pub fn record_execution_audit(
    tx: &Tx<'_>,
    event_id: i64,
    integration: Integration,
) -> campfire_db::Result<()> {
    // Prefer the additive approval column; metadata covers historical rows. A missing
    // approval means there is no audit target, exactly as Rails' find_by guard.
    let source = tx.conn().query_row("SELECT agent_approvals.action, agent_events.metadata FROM agent_events JOIN agent_approvals ON agent_approvals.id = COALESCE(agent_events.agent_approval_id, json_extract(agent_events.metadata, '$.approval_id')) WHERE agent_events.id = ?", [event_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).optional()?;
    let Some((action, stored)) = source else {
        return Ok(());
    };
    let metadata: Value = serde_json::from_str(&stored)
        .map_err(|error| campfire_db::Error::Other(error.to_string()))?;
    let mut details = serde_json::Map::new();
    details.insert("action".into(), Value::String(action));
    for key in ["status", "url", "message"] {
        if let Some(value) = metadata.get(key).filter(|value| !value.is_null()) {
            details.insert(key.into(), value.clone());
        }
    }
    tx.conn().execute("INSERT INTO audit_logs (action, actor_id, actor_label, target_type, target_id, target_label, details, created_at, updated_at) SELECT ?1, users.id, CASE WHEN users.id IS NOT NULL THEN users.name || ' <' || COALESCE(users.email_address, '') || '>' END, 'AgentApproval', agent_approvals.id, agent_approvals.action || ' approval #' || agent_approvals.id, ?2, ?3, ?3 FROM agent_events JOIN agent_approvals ON agent_approvals.id = COALESCE(agent_events.agent_approval_id, json_extract(agent_events.metadata, '$.approval_id')) LEFT JOIN users ON users.id = agent_events.actor_id WHERE agent_events.id = ?4", params![integration.audit_action, Value::Object(details).to_string(), tx.now(), event_id])?;
    Ok(())
}

#[cfg(test)]
mod tests;
