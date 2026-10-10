//! Accounts::Bots::CredentialsController; issuance/revocation use WS11's model.
use crate::{
    app::AppCtx,
    concerns::{self, Before},
};
use campfire_db::models::audit_log::{AuditLog, NewAuditLog, Target};
use campfire_db::{AgentCredential, User};
use campfire_kit::{Ctx, Error, Param, Result, StatusCode, permit_keys};

pub async fn create(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let bot = super::find_active_bot(c, "bot_id").await?;
    concerns::ensure_can_administer(c)?;
    concerns::sudo::require_sudo_mode(c).await?;
    let agent = super::ensure_agent(c, &bot).await?;
    let params = c
        .params
        .require("agent_credential")?
        .permit(&permit_keys(&["name", "expires_at"]));
    let name = params.get("name").and_then(Param::to_s).unwrap_or_default();
    let zone = super::viewer_zone(c).await?;
    let raw_expires_at = params.get("expires_at");
    let expires_at = super::input_casts::datetime(
        raw_expires_at,
        &zone,
        campfire_db::Timestamp::from_jiff(c.now()),
    )
    .map_err(Error::internal)?;
    let non_time = matches!(raw_expires_at, Some(Param::Number(_) | Param::Bool(true)));
    let raw_expiry = match raw_expires_at {
        Some(Param::Number(n)) if n.is_i64() => n.as_i64().map(rusqlite::types::Value::Integer),
        Some(Param::Number(n)) => n.as_f64().map(rusqlite::types::Value::Real),
        Some(Param::Bool(true)) => Some(rusqlite::types::Value::Integer(1)),
        _ => None,
    };
    let expiry = match raw_expiry {
        Some(raw) => Expiry::Raw(raw),
        None => Expiry::At(expires_at),
    };
    let result = issue(c, &bot, agent.id, name, expiry).await?;
    match result {
        Ok(secret) => {
            c.no_store();
            c.json(StatusCode::CREATED, &serde_json::json!({"secret": secret}))
        }
        Err(campfire_db::Error::RecordInvalid(_)) if non_time => {
            // Rails retains a numeric/true value after validation fails, then the
            // datetime_local_field calls strftime on it. The unsaved form raises.
            Err(Error::internal(anyhow::anyhow!(
                "non-time expires_at has no strftime"
            )))
        }
        Err(campfire_db::Error::RecordInvalid(_errors)) => Ok(c.head(StatusCode::UNPROCESSABLE_ENTITY)),
        Err(error) => Err(Error::internal(error)),
    }
}

/// A new credential's expiry: the time the form's value casts to, or a raw non-time value
/// (a number, `true`) stored as given, as Rails does.
pub enum Expiry {
    At(Option<campfire_db::Timestamp>),
    Raw(rusqlite::types::Value),
}

/// `create`'s writes once the gates passed and the agent exists: the credential (its secret
/// drawn), then the audit. The inner result is the save's: the secret, shown once, or
/// `RecordInvalid` for the form.
pub async fn issue(
    c: &Ctx,
    bot: &User,
    agent_id: i64,
    name: String,
    expiry: Expiry,
) -> Result<campfire_db::Result<String>> {
    let actor = concerns::require_current_user(c)?.id;
    let context = super::audit_context(c)?;
    let bot_name = bot.name.clone();
    Ok(async {
        let (secret, audit) = c
            .app()
            .db
            .write(move |tx| {
                let (credential, secret) = match expiry {
                    Expiry::Raw(raw) => AgentCredential::create_with_raw_expiry_secret(tx, agent_id, &name, actor, raw)?,
                    Expiry::At(expires_at) => {
                        let (credential, secret) = AgentCredential::create_with_secret(tx, agent_id, &name, actor, expires_at)?;
                        (campfire_db::models::agent_credential::IssuedCredential {
                            id: credential.id, name: credential.name, token_last_four: credential.token_last_four,
                        }, secret)
                    }
                };
                let audit = NewAuditLog {
                    action: "agent.credential.create".into(),
                    target: Some(target(credential.id, &credential.name, &bot_name)),
                    changes: Some(serde_json::json!({"name":credential.name,"last_four":credential.token_last_four})),
                    ..Default::default()
                };
                Ok((secret, audit))
            })
            .await?;
        // Rails commits create_with_secret! before the independent audit insert.
        c.app().db.write(move |tx| AuditLog::record(tx, audit, &context)).await?;
        Ok::<_, campfire_db::Error>(secret)
    }.await)
}

pub async fn destroy(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let bot = super::find_active_bot(c, "bot_id").await?;
    super::ensure_can_manage_bot(c, &bot).await?;
    concerns::sudo::require_sudo_mode(c).await?;
    let agent = super::ensure_agent(c, &bot).await?;
    let id = c
        .param_str("id")
        .and_then(crate::concerns::cast_integer)
        .ok_or(Error::NotFound)?;
    if !revoke(c, &bot, agent.id, id).await? {
        return Err(Error::NotFound);
    }
    c.redirect_to(&c.url_for(&campfire_routes::account_bot_credentials(bot.id)))
}

/// `destroy`'s writes once the gates passed and the agent exists: credential `id` revoked
/// (once), then the audit. `false` when the agent has no such credential.
pub async fn revoke(c: &Ctx, bot: &User, agent_id: i64, id: i64) -> Result<bool> {
    let context = super::audit_context(c)?;
    let bot_name = bot.name.clone();
    let audit = c
        .app()
        .db
        .write(move |tx| {
            let Some(mut credential) =
                AgentCredential::find(tx.conn(), id)?.filter(|c| c.agent_id == agent_id)
            else {
                return Ok(None);
            };
            if credential.revoked_at.is_none() {
                credential.revoke(tx)?;
                return Ok(Some(Some(NewAuditLog {
                    action: "agent.credential.revoke".into(),
                    target: Some(target(id, &credential.name, &bot_name)),
                    changes: Some(serde_json::json!({"name":credential.name})),
                    ..Default::default()
                })));
            }
            Ok(Some(None))
        })
        .await
        .map_err(Error::internal)?;
    let Some(audit) = audit else {
        return Ok(false);
    };
    if let Some(audit) = audit {
        c.app()
            .db
            .write(move |tx| AuditLog::record(tx, audit, &context))
            .await
            .map_err(Error::internal)?;
    }
    Ok(true)
}
fn target(id: i64, name: &str, bot: &str) -> Target {
    Target {
        record_type: "AgentCredential".into(),
        id,
        label: Some(format!("{name} ({bot})")),
    }
}
