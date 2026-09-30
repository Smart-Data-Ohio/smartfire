//! Agent suspension and kill switch. Quiet stream rendering runs only after commit.
use super::audit_log::{AuditLog, Context, NewAuditLog, Target};
use crate::sql::query_all;
use crate::{Agent, AgentApproval, AgentChanges, Message, Result, Tx};
use serde_json::json;

fn target(tx: &Tx<'_>, agent: &Agent) -> Result<Target> {
    Ok(Target {
        record_type: "Agent".into(),
        id: agent.id,
        label: Some(format!(
            "Agent {}",
            crate::User::find(tx.conn(), agent.user_id)?.name
        )),
    })
}
pub fn suspend(tx: &mut Tx<'_>, agent_id: i64, audit: &Context) -> Result<()> {
    let mut agent =
        Agent::find(tx.conn(), agent_id)?.ok_or(crate::Error::RecordNotFound("Agent"))?;
    if agent.suspended() {
        return Ok(());
    }
    agent.update(
        tx,
        AgentChanges {
            suspended_at: Some(Some(tx.now())),
            ..Default::default()
        },
    )?;
    let user_id = agent.user_id;
    tx.after_commit(move |tx| {
        for id in query_all(
            tx.conn(),
            "SELECT id FROM messages WHERE creator_id=? AND streaming=1 ORDER BY id",
            [user_id],
            |r| r.get::<_, i64>(0),
        )? {
            let result =
                Message::find(tx.conn(), id).and_then(|mut m| m.finalize_stream_quietly(tx));
            if let Err(error) = result {
                tracing::error!(message_id=id,%error,"quiet stream finalize failed");
            }
        }
        Ok(())
    });
    AuditLog::record(
        tx,
        NewAuditLog {
            action: "agent.suspend".into(),
            target: Some(target(tx, &agent)?),
            ..Default::default()
        },
        audit,
    )?;
    Ok(())
}
pub fn suspend_owned(tx: &mut Tx<'_>, owner_id: i64, audit: &Context) -> Result<()> {
    for id in query_all(
        tx.conn(),
        "SELECT id FROM agents WHERE owner_id=? ORDER BY id",
        [owner_id],
        |r| r.get::<_, i64>(0),
    )? {
        suspend(tx, id, audit)?;
    }
    Ok(())
}
pub fn kill_switch(tx: &mut Tx<'_>, agent_id: i64, audit: &Context) -> Result<usize> {
    suspend(tx, agent_id, audit)?;
    let mut cancelled = 0;
    for id in query_all(
        tx.conn(),
        "SELECT id FROM agent_approvals WHERE agent_id=? AND status='pending' ORDER BY id",
        [agent_id],
        |r| r.get::<_, i64>(0),
    )? {
        let mut approval = AgentApproval::find(tx.conn(), id)?.expect("selected approval");
        approval.expire_if_due(tx)?;
        if approval.status == "pending" {
            approval.cancel_by_agent(tx)?.into_result()?;
            cancelled += 1;
        }
    }
    let mut agent = Agent::find(tx.conn(), agent_id)?.expect("suspended agent");
    agent.clear_working_presence(tx)?;
    AuditLog::record(
        tx,
        NewAuditLog {
            action: "agent.kill_switch".into(),
            target: Some(target(tx, &agent)?),
            changes: Some(json!({"pending_approvals_cancelled":cancelled})),
            ..Default::default()
        },
        audit,
    )?;
    Ok(cancelled)
}
