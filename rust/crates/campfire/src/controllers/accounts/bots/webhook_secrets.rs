//! Secret rotation adapter. WS11's Agent secret operation is still unavailable.
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
    AgentPending,
    NoWebhook,
    Reset,
}
pub async fn create(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let bot = super::find_active_bot(c, "bot_id").await?;
    super::ensure_can_manage_bot(c, &bot).await?;
    concerns::require_sudo_mode(c)?;
    let id = bot.id;
    let crypto = c.app().ar_encryption.clone();
    let context = super::audit_context(c)?;
    let result = c
        .app()
        .db
        .write(move |tx| {
            if Agent::for_user(tx.conn(), id)?.is_some() {
                // Bind WS11's reset_webhook_signing_secret domain operation here
                // when supplied. Do not rotate an unrelated legacy webhook secret.
                return Ok(Rotation::AgentPending);
            }
            let Some(mut webhook) = Webhook::find_by_user(tx.conn(), id)? else {
                return Ok(Rotation::NoWebhook);
            };
            webhook.reset_signing_secret(tx, &crypto)?;
            AuditLog::record(
                tx,
                NewAuditLog {
                    action: "agent.webhook_secret.reset".into(),
                    target: Some(Target::from(&bot)),
                    ..Default::default()
                },
                &context,
            )?;
            Ok(Rotation::Reset)
        })
        .await
        .map_err(Error::internal)?;
    match result {
        Rotation::AgentPending => crate::controllers::not_yet_ported(c).await,
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
