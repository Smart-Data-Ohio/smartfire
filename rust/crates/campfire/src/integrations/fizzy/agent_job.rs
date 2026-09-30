//! `Fizzy::PerformAgentActionJob`: an at-most-once external write, never retried.
use super::{
    accounts::{Account, REJECTED_TOKEN_REASON},
    agent_action::Action,
    client::{self, Client, ErrorKind},
};
use crate::{
    app::App,
    integrations::{
        action_claims::{self, FIZZY},
        net::Network,
    },
};
use campfire_db::{Event, Job, Tx};
use campfire_jobs::{Execution, JobKind, JobResult, Outcome, RetryPolicy};
use rails_compat::ar_encryption::ArEncryption;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformJob {
    pub approval_id: i64,
}
impl Job for PerformJob {
    const CLASS: &'static str = "Fizzy::PerformAgentActionJob";
}
impl JobKind for PerformJob {
    fn retry_policy() -> RetryPolicy {
        RetryPolicy::no_retries()
    }
}
pub async fn perform(app: App, job: PerformJob, _: Execution) -> JobResult {
    execute(
        &app,
        &Network::system(),
        &client::api_base_url(),
        job.approval_id,
    )
    .await
    .map_err(crate::jobs::discard_missing)?;
    Ok(Outcome::Done)
}
/// WS11 calls inside its approval-decision transaction, after writing the human decision.
/// Its authenticated REST/MCP adapters consume Action; this hook performs no external HTTP.
#[allow(
    dead_code,
    reason = "WS11 owns the approval-decision controller/model, not yet merged"
)]
pub fn enqueue_approved(tx: &mut Tx<'_>, approval_id: i64) -> campfire_db::Result<()> {
    let approved:bool=tx.conn().query_row("SELECT EXISTS(SELECT 1 FROM agent_approvals WHERE id=? AND status='approved' AND action LIKE 'fizzy.%')",[approval_id],|r|r.get(0))?;
    if approved {
        tx.emit_after_commit(Event::job(&PerformJob { approval_id }));
    }
    Ok(())
}
struct Approval {
    id: i64,
    agent: i64,
    actor: Option<i64>,
    action: String,
    status: String,
    summary: String,
    payload: Option<String>,
    owner: Option<i64>,
    account: Option<i64>,
    fizzy_user: Option<String>,
    active: bool,
    grant: bool,
}
fn approval(tx: &Tx<'_>, id: i64) -> campfire_db::Result<Option<Approval>> {
    Ok(tx.conn().query_row("SELECT a.id,a.agent_id,a.decided_by_id,a.action,a.status,a.summary,a.payload,g.owner_id,a.fizzy_connected_account_id,a.fizzy_user_id,(g.suspended_at IS NULL AND u.status=0),EXISTS(SELECT 1 FROM agent_grants WHERE agent_id=g.id AND capability='external_action' AND room_id IS NULL AND revoked_at IS NULL) FROM agent_approvals a JOIN agents g ON g.id=a.agent_id JOIN users u ON u.id=g.user_id WHERE a.id=?",[id],|r|Ok(Approval{id:r.get(0)?,agent:r.get(1)?,actor:r.get(2)?,action:r.get(3)?,status:r.get(4)?,summary:r.get(5)?,payload:r.get(6)?,owner:r.get(7)?,account:r.get(8)?,fizzy_user:r.get(9)?,active:r.get(10)?,grant:r.get(11)?})).optional()?)
}
fn claim(tx: &Tx<'_>, a: &Approval) -> campfire_db::Result<Option<i64>> {
    let metadata = json!({"approval_id":a.id,"action":a.action,"status":"running"}).to_string();
    let n=tx.conn().execute("INSERT INTO agent_events (agent_id,agent_approval_id,actor_id,event_type,outcome,webhook_status,metadata,created_at) SELECT ?1,?2,?3,'fizzy_action_completed','delivered','none',?4,?5 WHERE NOT EXISTS(SELECT 1 FROM agent_events WHERE agent_id=?1 AND event_type='fizzy_action_completed' AND (agent_approval_id=?2 OR CAST(json_extract(metadata,'$.approval_id') AS INTEGER)=?2))",params![a.agent,a.id,a.actor,metadata,tx.now()])?;
    Ok((n == 1).then(|| tx.conn().last_insert_rowid()))
}
fn finish(
    tx: &mut Tx<'_>,
    event: i64,
    status: &str,
    message: Option<&str>,
    url: Option<Value>,
) -> campfire_db::Result<()> {
    let stored: String = tx.conn().query_row(
        "SELECT metadata FROM agent_events WHERE id=?",
        [event],
        |r| r.get(0),
    )?;
    let mut metadata: Value =
        serde_json::from_str(&stored).map_err(|e| campfire_db::Error::Other(e.to_string()))?;
    // Worker finishes have exactly these compact keys, while the sweep preserves arbitrary extras.
    metadata
        .as_object_mut()
        .unwrap()
        .retain(|k, _| ["approval_id", "action"].contains(&k.as_str()));
    metadata["status"] = json!(status);
    if let Some(message) = message {
        metadata["message"] = json!(message);
    }
    if let Some(url) = url.filter(|v| !v.is_null()) {
        metadata["url"] = url;
    }
    if action_claims::rewrite_running(tx, event, metadata, message.map(str::to_owned))?
        && let Err(error) = action_claims::record_execution_audit(tx, event, FIZZY)
    {
        tracing::error!(%error,event,"Fizzy execution audit failed");
    }
    Ok(())
}
struct Prepared {
    event: i64,
    account: Account,
    token: String,
    action: Action,
}
fn prepare(
    tx: &mut Tx<'_>,
    id: i64,
    crypto: &ArEncryption,
) -> campfire_db::Result<Option<Prepared>> {
    let Some(a) = approval(tx, id)? else {
        return Ok(None);
    };
    if !a.action.starts_with("fizzy.") {
        return Ok(None);
    }
    // Writer serialization plus both legacy and indexed identity checks prevent any duplicate HTTP.
    let Some(event) = claim(tx, &a)? else {
        return Ok(None);
    };
    let fail = |tx: &mut Tx<'_>, message: &str| {
        finish(tx, event, "failed", Some(message), None)?;
        Ok(None)
    };
    if a.status != "approved" {
        return fail(tx, "Approval is no longer approved");
    }
    if !a.active {
        return fail(tx, "Agent is suspended or deactivated");
    }
    if !a.grant {
        return fail(tx, "Agent no longer has the external_action capability");
    }
    let Some(action) = a.payload.as_deref().and_then(Action::from_stored) else {
        return fail(tx, "Approval payload is invalid");
    };
    if !action.errors().is_empty() {
        return fail(tx, "Approval payload is invalid");
    }
    if action.action_name() != a.action || action.summary().as_deref() != Some(&a.summary) {
        return fail(tx, "Approval summary does not match its payload");
    }
    let account = a
        .owner
        .map(|user| Account::for_user(tx.conn(), user))
        .transpose()?
        .flatten();
    let Some(account) = account else {
        return fail(tx, "Agent owner has no usable Fizzy account");
    };
    let Some(token) = account.usable_token(tx, crypto)? else {
        return fail(tx, "Agent owner has no usable Fizzy account");
    };
    if a.account != Some(account.id)
        || a.fizzy_user
            .as_deref()
            .is_none_or(campfire_richtext::ruby::is_blank)
        || a.fizzy_user != account.fizzy_user_id
    {
        return fail(
            tx,
            "The agent owner's Fizzy account changed since this was approved",
        );
    }
    Ok(Some(Prepared {
        event,
        account,
        token,
        action,
    }))
}
pub async fn execute(
    app: &App,
    network: &Network,
    base: &str,
    approval_id: i64,
) -> campfire_db::Result<()> {
    let crypto = ArEncryption::new(&app.secrets);
    let input = app
        .db
        .write(move |tx| prepare(tx, approval_id, &crypto))
        .await?;
    let Some(Prepared {
        event,
        account,
        token,
        action,
    }) = input
    else {
        return Ok(());
    };
    let client = Client::new(network.clone(), token, base);
    let response = action.perform(&client).await;
    let mut disconnect = false;
    let (status, message, url) = match response {
        Ok(value) => (
            "completed",
            None,
            value.as_object().and_then(|v| v.get("url")).cloned(),
        ),
        Err(error) if error.kind == ErrorKind::Unauthorized => {
            let message = match client.identity().await {
                Ok(_) => {
                    "The agent owner's Fizzy token is read-only; card writes need a Read + Write token"
                }
                Err(probe) if probe.kind == ErrorKind::Unauthorized => {
                    disconnect = true;
                    "Fizzy rejected the agent owner's linked token (401)"
                }
                Err(_) => "Could not verify the agent owner's Fizzy token; try again",
            };
            ("failed", Some(message.to_owned()), None)
        }
        Err(error) => ("failed", Some(error.message), None),
    };
    app.db
        .write(move |tx| {
            if disconnect {
                account.mark_disconnected(tx, REJECTED_TOKEN_REASON)?;
            }
            finish(tx, event, status, message.as_deref(), url)
        })
        .await
}
#[cfg(test)]
mod tests;
