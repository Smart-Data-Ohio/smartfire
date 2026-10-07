//! Human decisions. WS11 owns authorization rechecks and atomic model callbacks.
use crate::{
    app::AppCtx,
    concerns::{self, AuthenticatedBy, Before},
};
use campfire_db::{
    AgentApproval,
    models::{
        agent_approval::ApprovalDecision,
        audit_log::{AuditLog, Context, NewAuditLog, Target},
    },
};
use campfire_kit::{Ctx, Error, Param, Result, StatusCode, format};
use serde_json::json;

pub async fn update(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default().allow_agent_access().allow_bot_access()).await?;
    let actor = concerns::require_current_user(c)?.clone();
    if actor.is_bot()
        || matches!(
            concerns::authenticated_by(c),
            AuthenticatedBy::AgentToken | AuthenticatedBy::BotKey
        )
    {
        return Ok(c.head(StatusCode::NOT_FOUND));
    }
    let Some(id) = c.param_str("id").and_then(concerns::cast_integer) else {
        return Ok(c.head(StatusCode::NOT_FOUND));
    };
    let decision = c
        .params
        .get("decision")
        .and_then(Param::to_s)
        .unwrap_or_default();
    let note = ["decision_note", "note"].into_iter().find_map(|key| {
        c.params
            .get(key)
            .and_then(Param::to_s)
            .filter(|s| !campfire_richtext::ruby::is_blank(s))
    });
    match decide(c, actor, id, &decision, note).await? {
        Decided::NotFound => Ok(c.head(StatusCode::NOT_FOUND)),
        Decided::Refused {
            status,
            message,
            alert,
        } => failure(c, status, &message, &alert),
        Decided::Applied { payload, .. } => match c.respond_to(&[&format::HTML, &format::JSON])? {
            chosen if chosen == &format::JSON => c.json(StatusCode::OK, &payload),
            _ => redirect(
                c,
                &format!("Request {}.", payload["status"].as_str().unwrap()),
                false,
            ),
        },
    }
}

/// What [`decide`] did.
pub enum Decided {
    /// The approval doesn't exist, or `actor` may not decide it (`decidable_by`).
    NotFound,
    /// Refused with `status`: `message` is the JSON `error`, `alert` the HTML flash.
    Refused {
        status: StatusCode,
        message: String,
        alert: String,
    },
    /// Decided: the approval as it now is, and the JSON reply's payload.
    Applied {
        approval: Box<AgentApproval>,
        payload: serde_json::Value,
    },
}

/// `agent_approvals#update` after its before-actions, for a human `actor`: every server-side
/// check, the decision (with its inbox and ledger callbacks), then the audit entry. The single-page
/// app's `PATCH /api/v1/agent_approvals/:id` decides through here too.
pub async fn decide(
    c: &Ctx,
    actor: campfire_db::User,
    id: i64,
    decision: &str,
    note: Option<String>,
) -> Result<Decided> {
    let decision = decision.to_owned();
    let for_lookup = actor.clone();
    let Some(approval) = c
        .app()
        .db
        .read(move |conn| {
            let Some(approval) = AgentApproval::find(conn, id)? else {
                return Ok(None);
            };
            Ok(approval
                .decidable_by(conn, &for_lookup)?
                .then_some(approval))
        })
        .await
        .map_err(Error::internal)?
    else {
        return Ok(Decided::NotFound);
    };
    let refused = |status: StatusCode, message: &str, alert: &str| {
        Ok(Decided::Refused {
            status,
            message: message.to_owned(),
            alert: alert.to_owned(),
        })
    };
    if !["approved", "denied"].contains(&decision.as_str()) {
        return refused(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Decision must be approved or denied",
            "Choose Approve or Deny.",
        );
    }
    if decision == "approved"
        && (approval.github_action() || approval.fizzy_action())
        && !actor.is_administrator()
    {
        let service = if approval.fizzy_action() {
            "Fizzy"
        } else {
            "GitHub"
        };
        let message = format!("Only an administrator can approve {service} write actions");
        return refused(StatusCode::FORBIDDEN, &message, &format!("{message}."));
    }
    if decision == "approved" && (approval.github_action() || approval.fizzy_action()) {
        let agent_id = approval.agent_id;
        let agent = c
            .app()
            .db
            .read(move |conn| campfire_db::Agent::find(conn, agent_id))
            .await
            .map_err(Error::internal)?
            .ok_or(Error::NotFound)?;
        let current = if approval.github_action() {
            c.app()
                .github_accounts
                .agent_identity(agent.owner_id, agent.user_id)
                .await
                .map_err(Error::internal)?
                .is_some_and(|account| {
                    approval.github_identity_matches(account.id, &account.github_login)
                })
        } else {
            c.app()
                .db
                .read(move |conn| {
                    agent
                        .owner_id
                        .map(|owner| {
                            crate::integrations::fizzy::accounts::Account::for_user(conn, owner)
                        })
                        .transpose()
                        .map(Option::flatten)
                })
                .await
                .map_err(Error::internal)?
                .is_some_and(|account| {
                    account
                        .fizzy_user_id
                        .as_deref()
                        .is_some_and(|user| approval.fizzy_identity_matches(account.id, user))
                })
        };
        if !current {
            let message = if approval.github_action() {
                "The agent's GitHub account changed since this was requested; deny it and ask the agent to request again"
            } else {
                "The agent owner's Fizzy account changed since this was requested; deny it and ask the agent to request again"
            };
            return refused(
                StatusCode::UNPROCESSABLE_ENTITY,
                message,
                &format!("{message}."),
            );
        }
    }
    let note = note.filter(|s| !campfire_richtext::ruby::is_blank(s));
    let context = Context {
        actor: Some((&actor).into()),
        ip_address: Some(c.request.remote_ip()?.to_string()),
        user_agent: c.request.user_agent().map(str::to_owned),
    };
    let (decision_result, approval, payload, audit) = c.app().db.write(move |tx| {
        let mut approval = approval;
        let result = approval.decide_authorized(tx, &decision, &actor, note.as_deref())?;
        let audit = if result == ApprovalDecision::Applied {
            let mut changes = json!({"decision": campfire_db::models::audit_log::pair(json!("pending"), json!(approval.status))});
            if let Some(note) = &approval.decision_note { changes["note"] = json!(note); }
            Some(NewAuditLog {
                action: "agent.approval.decide".into(),
                target: Some(Target {record_type:"AgentApproval".into(), id:approval.id, label:Some(format!("{} approval #{}",approval.action,approval.id))}),
                changes:Some(changes), ..Default::default()
            })
        } else { None };
        let mut payload = approval.payload(tx.conn(), tx.now())?;
        payload.as_object_mut().unwrap().retain(|k,_| ["id", "status", "decided_by", "decided_by_id", "decided_at", "decision_note", "note"].contains(&k.as_str()));
        Ok((result,approval,payload,audit))
    }).await.map_err(Error::internal)?;
    // Rails commits decide! (including inbox/ledger callbacks) before the
    // independent audit insert. An audit failure returns 500 with that decision
    // preserved; it must not roll the owner mutation back.
    if let Some(audit) = audit {
        c.app()
            .db
            .write(move |tx| AuditLog::record(tx, audit, &context))
            .await
            .map_err(Error::internal)?;
    }
    match decision_result {
        ApprovalDecision::Forbidden => Err(Error::NotFound),
        ApprovalDecision::Invalid(errors) => {
            let message = campfire_views::helpers::to_sentence(&errors.full_messages(), " and ");
            let message = if message.is_empty() {
                "Request cannot be decided"
            } else {
                &message
            };
            refused(StatusCode::UNPROCESSABLE_ENTITY, message, message)
        }
        ApprovalDecision::Applied => Ok(Decided::Applied {
            approval: Box::new(approval),
            payload,
        }),
    }
}
fn failure(c: &mut Ctx, status: StatusCode, json_error: &str, alert: &str) -> Result {
    match c.respond_to(&[&format::HTML, &format::JSON])? {
        chosen if chosen == &format::JSON => c.json(status, &json!({"error":json_error})),
        _ => redirect(c, alert, true),
    }
}
fn redirect(c: &mut Ctx, message: &str, alert: bool) -> Result {
    if alert {
        c.flash().set_alert(message);
    } else {
        c.flash().set_notice(message);
    }
    let mut response = c.redirect_back_or_to("/activity")?;
    response.status = StatusCode::SEE_OTHER;
    Ok(response)
}
