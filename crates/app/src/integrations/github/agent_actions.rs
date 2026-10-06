//! Approved GitHub writes (`app/jobs/github/perform_agent_action_job.rb`).
//! Rechecks precede an atomic claim; no database transaction spans an HTTP request.
use super::{
    accounts::{Account, Accounts},
    actions::Action,
    client::{ErrorKind, PullRequestKey, ruby_string, ruby_to_i},
};
use crate::integrations::action_claims::{self, EventWebhookJob};
use campfire_db::models::audit_log::{Actor, AuditLog, Context, NewAuditLog, Target};
use campfire_db::{Connection, Database, Event, Result, Tx, User};
use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};

#[derive(Clone)]
struct Approval {
    id: i64,
    agent_id: i64,
    room_id: Option<i64>,
    actor_id: Option<i64>,
    action: String,
    summary: String,
    payload: Option<String>,
    status: String,
    account_id: Option<i64>,
    login: Option<String>,
    agent_user_id: i64,
    owner_id: Option<i64>,
    active: bool,
}
impl Approval {
    fn find(conn: &Connection, id: i64) -> Result<Option<Self>> {
        Ok(conn.query_row(
            "SELECT a.id,a.agent_id,r.id,a.decided_by_id,a.action,a.summary,a.payload,a.status,a.github_account_id,a.github_login,g.user_id,g.owner_id,g.suspended_at IS NULL AND u.status=0
             FROM agent_approvals a JOIN agents g ON g.id=a.agent_id
             LEFT JOIN users u ON u.id=g.user_id LEFT JOIN rooms r ON r.id=a.room_id WHERE a.id=?",
            [id],
            |row| Ok(Self {
                id: row.get(0)?, agent_id: row.get(1)?, room_id: row.get(2)?,
                actor_id: row.get(3)?, action: row.get(4)?, summary: row.get(5)?,
                payload: row.get(6)?, status: row.get(7)?, account_id: row.get(8)?,
                login: row.get(9)?, agent_user_id: row.get(10)?, owner_id: row.get(11)?,
                active: row.get::<_, Option<bool>>(12)?.unwrap_or(false),
            }),
        ).optional()?)
    }

    fn already_executed(&self, conn: &Connection) -> Result<bool> {
        Ok(conn.query_row("SELECT EXISTS(SELECT 1 FROM agent_events WHERE agent_id=? AND event_type='github_action_completed' AND (agent_approval_id=? OR CAST(json_extract(metadata,'$.approval_id') AS INTEGER)=?))",params![self.agent_id,self.id,self.id],|r|r.get(0))?)
    }
    fn premature_failure(&self, conn: &Connection) -> Result<Option<&'static str>> {
        if self.status != "approved" {
            return Ok(Some("Approval is no longer approved"));
        }
        if !self.active {
            return Ok(Some("Agent is suspended or deactivated"));
        }
        let deleted: Option<bool> = conn
            .query_row(
                "SELECT deleted_at IS NOT NULL FROM rooms WHERE id=?",
                [self.room_id],
                |r| r.get(0),
            )
            .optional()?;
        let Some(deleted) = deleted else {
            return Ok(Some("Room no longer exists"));
        };
        let member: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM memberships WHERE user_id=? AND room_id=?)",
            params![self.agent_user_id, self.room_id],
            |r| r.get(0),
        )?;
        if !member {
            return Ok(Some("Agent is no longer a member of the room"));
        }
        let grant:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM agent_grants WHERE agent_id=? AND capability='external_action' AND revoked_at IS NULL AND (room_id=? OR room_id IS NULL))",params![self.agent_id,self.room_id],|r|r.get(0))?;
        Ok((deleted || !grant).then_some("Agent no longer has the external_action capability"))
    }
    fn actor(&self, conn: &Connection) -> Result<Option<Actor>> {
        self.actor_id
            .map(|id| match User::find(conn, id) {
                Ok(user) => Ok(Some(Actor::from(&user))),
                Err(campfire_db::Error::RecordNotFound(_)) => Ok(None),
                Err(e) => Err(e),
            })
            .transpose()
            .map(Option::flatten)
    }
}

pub async fn perform(db: &Database, accounts: &Accounts, approval_id: i64) -> Result<()> {
    let state = db
        .read(move |conn| {
            let Some(approval) =
                Approval::find(conn, approval_id)?.filter(|a| a.action.starts_with("github."))
            else {
                return Ok(None);
            };
            if approval.already_executed(conn)? {
                return Ok(None);
            }
            let reason = approval.premature_failure(conn)?;
            Ok(Some((approval, reason)))
        })
        .await?;
    let Some((approval, reason)) = state else {
        return Ok(());
    };
    if let Some(reason) = reason {
        return record_failure(db, approval, reason).await;
    }
    let payload = approval
        .payload
        .as_deref()
        .and_then(|s| serde_json::from_str::<Value>(s).ok())
        .filter(Value::is_object);
    let room_id = approval.room_id;
    let action = db.read(move |conn| {
        let Some(payload) = payload else { return Ok(None); };
        // Active Record casts the id before lookup, including numeric string prefixes.
        let id = ruby_to_i(&ruby_string(&payload["pull_request_id"]));
        let key = conn.query_row(
            "SELECT owner,repo,number FROM github_pull_requests WHERE id=? AND EXISTS(
                SELECT 1 FROM github_pull_request_threads WHERE github_pull_request_id=github_pull_requests.id AND room_id=?)",
            params![id,room_id],
            |row| Ok(PullRequestKey { owner: row.get(0)?, repo: row.get(1)?, number: row.get(2)? }),
        ).optional()?;
        Ok(key.map(|key| Action::from_payload(id, key, &payload)))
    }).await?;
    let Some(action) = action else {
        return record_failure(
            db,
            approval,
            "The pull request is no longer discussed in this room",
        )
        .await;
    };
    if !action.errors().is_empty() {
        return record_failure(db, approval, "Approval payload is invalid").await;
    }
    if action.action_name() != approval.action
        || action.summary().as_deref() != Some(&approval.summary)
    {
        return record_failure(db, approval, "Approval summary does not match its payload").await;
    }
    let account = accounts
        .agent_identity(approval.owner_id, approval.agent_user_id)
        .await?;
    let Some(account) = account else {
        return record_failure(db, approval, "Agent has no usable GitHub account").await;
    };
    if !accounts.usable(account.id).await? {
        return record_failure(db, approval, "Agent has no usable GitHub account").await;
    }
    if approval.account_id != Some(account.id)
        || approval.login.as_deref().is_none_or(|login| {
            super::blank(login)
                || !caseless::default_caseless_match_str(login, &account.github_login)
        })
    {
        return record_failure(
            db,
            approval,
            "The agent's GitHub account changed since this was approved",
        )
        .await;
    }
    let Some(token) = accounts.access_token_for_use(account.id).await? else {
        return record_failure(db, approval, "Agent has no usable GitHub account").await;
    };
    let client = accounts.write_client(token);
    let claimed = db
        .write({
            let approval = approval.clone();
            move |tx| {
                let mut approval = approval;
                let actor = approval.actor(tx.conn())?;
                approval.actor_id = actor.as_ref().map(|actor| actor.id);
                Ok(insert_event(tx, &approval, "running", None, None)?.map(|id| (id, actor)))
            }
        })
        .await?;
    let Some((event_id, actor)) = claimed else {
        return Ok(());
    };
    let (status, message, url) = match action.perform(&client).await {
        Ok(response) => (
            "completed",
            None,
            response
                .as_object()
                .and_then(|o| o.get("html_url"))
                .filter(|v| !v.is_null())
                .cloned(),
        ),
        Err(error) if error.kind == ErrorKind::Unauthorized => {
            db.write(move |tx| {
                // Rails can save its cached account after another connection deleted the row.
                if Account::find(tx.conn(), account.id)?.is_some() {
                    Account::mark_disconnected(
                        tx,
                        account.id,
                        "GitHub rejected the linked token (401)",
                    )?;
                }
                Ok(())
            })
            .await?;
            (
                "failed",
                Some("GitHub rejected the agent's linked token (401)".to_owned()),
                None,
            )
        }
        Err(error) => ("failed", Some(error.message), None),
    };
    db.write(move |tx| {
        let metadata = metadata(&approval, status, message.as_deref(), url);
        if action_claims::rewrite_running(tx, event_id, metadata.clone(), message)? {
            record_audit(tx, &approval, actor, metadata);
        }
        Ok(())
    })
    .await
}
fn metadata(approval: &Approval, status: &str, message: Option<&str>, url: Option<Value>) -> Value {
    let mut data = json!({"approval_id":approval.id,"action":approval.action,"status":status});
    if let Some(message) = message {
        data["message"] = message.into();
    }
    if let Some(url) = url {
        data["url"] = url;
    }
    data
}
fn insert_event(
    tx: &mut Tx<'_>,
    approval: &Approval,
    status: &str,
    message: Option<&str>,
    url: Option<Value>,
) -> Result<Option<i64>> {
    let pending = status != "running"
        && tx.conn().query_row(
            "SELECT EXISTS(SELECT 1 FROM webhooks WHERE user_id=?)",
            [approval.agent_user_id],
            |r| r.get::<_, bool>(0),
        )?;
    let result=tx.conn().execute("INSERT INTO agent_events (agent_id,agent_approval_id,room_id,actor_id,event_type,outcome,detail,metadata,webhook_status,webhook_next_attempt_at,created_at) VALUES (?,?,?,?,'github_action_completed','delivered',?,?,?,?,?)",params![approval.agent_id,approval.id,approval.room_id,approval.actor_id,message,metadata(approval,status,message,url).to_string(),if pending {"pending"}else{"none"},pending.then(||tx.now()),tx.now()]);
    if let Err(error) = result {
        let error = campfire_db::Error::from(error);
        if error.is_record_not_unique() {
            return Ok(None);
        }
        return Err(error);
    }
    let id = tx.conn().last_insert_rowid();
    if pending {
        tx.emit_after_commit(Event::job(&EventWebhookJob {
            event_id: id,
            attempt: Some(0),
        }));
    }
    Ok(Some(id))
}
async fn record_failure(db: &Database, approval: Approval, message: &'static str) -> Result<()> {
    db.write(move |tx| {
        let mut approval = approval;
        let actor = approval.actor(tx.conn())?;
        approval.actor_id = actor.as_ref().map(|actor| actor.id);
        if insert_event(tx, &approval, "failed", Some(message), None)?.is_some() {
            record_audit(
                tx,
                &approval,
                actor,
                metadata(&approval, "failed", Some(message), None),
            );
        }
        Ok(())
    })
    .await
}
fn record_audit(tx: &Tx<'_>, approval: &Approval, actor: Option<Actor>, mut changes: Value) {
    changes.as_object_mut().unwrap().remove("approval_id");
    let input = NewAuditLog {
        action: action_claims::GITHUB.audit_action.into(),
        actor,
        target: Some(Target {
            record_type: "AgentApproval".into(),
            id: approval.id,
            label: Some(format!("{} approval #{}", approval.action, approval.id)),
        }),
        changes: Some(changes),
        ..Default::default()
    };
    if let Err(error) = AuditLog::record(tx, input, &Context::default()) {
        tracing::error!(%error,approval_id=approval.id,"GitHub execution audit failed");
    }
}
