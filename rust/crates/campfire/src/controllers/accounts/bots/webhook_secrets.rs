//! Rails secret rotation, authorized here and written by WS11's domain.
use crate::{
    app::AppCtx,
    concerns::{self, Before},
};
use campfire_db::{
    Agent, Webhook,
    models::audit_log::{AuditLog, NewAuditLog, Target},
};
use campfire_kit::{Ctx, Error, Result};
enum Rotation {
    NoWebhook,
    Reset,
}
pub async fn create(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let bot = super::find_active_bot(c, "bot_id").await?;
    super::ensure_can_manage_bot(c, &bot).await?;
    concerns::sudo::require_sudo_mode(c)?;
    let id = bot.id;
    let crypto = c.app().ar_encryption.clone();
    let context = super::audit_context(c)?;
    let result = c
        .app()
        .db
        .write(move |tx| {
            let target = if let Some(agent) = Agent::for_user(tx.conn(), id)? {
                campfire_db::models::agent_access::reset_webhook_signing_secret(
                    tx, &crypto, agent.id,
                )?;
                super::audit_target(&bot, Some(&agent))
            } else {
                let Some(mut webhook) = Webhook::find_by_user(tx.conn(), id)? else {
                    return Ok(Rotation::NoWebhook);
                };
                webhook.reset_signing_secret(tx, &crypto)?;
                Target::from(&bot)
            };
            AuditLog::record(
                tx,
                NewAuditLog {
                    action: "agent.webhook_secret.reset".into(),
                    target: Some(target),
                    ..Default::default()
                },
                &context,
            )?;
            Ok(Rotation::Reset)
        })
        .await
        .map_err(Error::internal)?;
    match result {
        Rotation::NoWebhook => {
            c.flash()
                .set_alert("Set a webhook URL before generating a signing secret.");
            c.redirect_to(&c.url_for(&campfire_routes::edit_account_bot(id)))
        }
        Rotation::Reset => {
            c.flash().set_notice(
                "Signing secret reset. Update the receiving service with the new secret.",
            );
            c.redirect_to(&c.url_for(&campfire_routes::edit_account_bot(id)))
        }
    }
}
