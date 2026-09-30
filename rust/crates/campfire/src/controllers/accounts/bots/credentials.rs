//! Accounts::Bots::CredentialsController; issuance/revocation use WS11's model.
use crate::controllers::presenters::page::framed_page;
use crate::{
    app::AppCtx,
    concerns::{self, Before},
    controllers::presenters,
};
use campfire_db::models::audit_log::{AuditLog, NewAuditLog, Target};
use campfire_db::{AgentCredential, User};
use campfire_kit::{Ctx, Error, Param, Result, StatusCode, format, permit_keys};
use campfire_views::accounts::bot_access::{CredentialCreated, CredentialForm};

pub async fn index(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let bot = super::find_active_bot(c, "bot_id").await?;
    super::ensure_can_manage_bot(c, &bot).await?;
    let agent = super::ensure_agent(c, &bot).await?;
    render_index(c, &bot, agent.id, CredentialForm::default(), StatusCode::OK).await
}
pub async fn create(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let bot = super::find_active_bot(c, "bot_id").await?;
    concerns::ensure_can_administer(c)?;
    concerns::require_sudo_mode(c)?;
    let agent = super::ensure_agent(c, &bot).await?;
    let params = c
        .params
        .require("agent_credential")?
        .permit(&permit_keys(&["name", "expires_at"]));
    let name = params.get("name").and_then(Param::to_s).unwrap_or_default();
    let zone = super::viewer_zone(c).await?;
    let expires_at = super::parse_datetime(params.get("expires_at"), &zone);
    let actor = concerns::require_current_user(c)?.id;
    let context = super::audit_context(c)?;
    let form_name = name.clone();
    let bot_name = bot.name.clone();
    let result=c.app().db.write(move |tx| {
        let (credential,secret)=AgentCredential::create_with_secret(tx,agent.id,&name,actor,expires_at)?;
        AuditLog::record(tx,NewAuditLog{action:"agent.credential.create".into(),target:Some(target(credential.id,&credential.name,&bot_name)),changes:Some(serde_json::json!({"name":credential.name,"last_four":credential.token_last_four})),..Default::default()},&context)?;
        Ok(secret)
    }).await;
    match result {
        Ok(secret) => {
            let bot_id = bot.id;
            framed_page!(c, StatusCode::CREATED, |ctx| CredentialCreated {
                ctx,
                bot_id,
                secret: secret.clone()
            })
            .await
        }
        Err(campfire_db::Error::RecordInvalid(errors)) => {
            render_index(
                c,
                &bot,
                agent.id,
                CredentialForm {
                    name: Some(form_name),
                    expires_at: expires_at.map(|t| zone.format(t.jiff(), "%Y-%m-%dT%H:%M:%S")),
                    errors: Some(super::error_sentence(&errors)),
                    error_fields: errors.0.iter().map(|(f, _)| f.to_string()).collect(),
                },
                StatusCode::UNPROCESSABLE_ENTITY,
            )
            .await
        }
        Err(error) => Err(Error::internal(error)),
    }
}
pub async fn destroy(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let bot = super::find_active_bot(c, "bot_id").await?;
    super::ensure_can_manage_bot(c, &bot).await?;
    concerns::require_sudo_mode(c)?;
    let agent = super::ensure_agent(c, &bot).await?;
    let id = c
        .param_str("id")
        .and_then(crate::concerns::cast_integer)
        .ok_or(Error::NotFound)?;
    let context = super::audit_context(c)?;
    let bot_name = bot.name.clone();
    let found = c
        .app()
        .db
        .write(move |tx| {
            let Some(mut credential) =
                AgentCredential::find(tx.conn(), id)?.filter(|c| c.agent_id == agent.id)
            else {
                return Ok(false);
            };
            if credential.revoked_at.is_none() {
                credential.revoke(tx)?;
                AuditLog::record(
                    tx,
                    NewAuditLog {
                        action: "agent.credential.revoke".into(),
                        target: Some(target(id, &credential.name, &bot_name)),
                        changes: Some(serde_json::json!({"name":credential.name})),
                        ..Default::default()
                    },
                    &context,
                )?;
            }
            Ok(true)
        })
        .await
        .map_err(Error::internal)?;
    if !found {
        return Err(Error::NotFound);
    }
    c.redirect_to(&c.url_for(&campfire_routes::account_bot_credentials(bot.id)))
}
fn target(id: i64, name: &str, bot: &str) -> Target {
    Target {
        record_type: "AgentCredential".into(),
        id,
        label: Some(format!("{name} ({bot})")),
    }
}
async fn render_index(
    c: &mut Ctx,
    bot: &User,
    agent_id: i64,
    form: CredentialForm,
    status: StatusCode,
) -> Result {
    c.respond_to(&[&format::HTML])?;
    let credentials = c
        .app()
        .db
        .read(move |conn| presenters::accounts::bot_access::credentials(conn, agent_id))
        .await
        .map_err(Error::internal)?;
    let (bot_id, bot_name, now) = (bot.id, bot.name.clone(), c.now());
    framed_page!(c, status, |ctx| {
        campfire_views::accounts::bot_access::Credentials {
            ctx,
            bot_id,
            bot_name: bot_name.clone(),
            credentials: credentials.clone(),
            credential: form.clone(),
            now,
        }
    })
    .await
}
