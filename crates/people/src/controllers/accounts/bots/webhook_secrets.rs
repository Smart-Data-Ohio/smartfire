//! Rails secret rotation, authorized here and written by WS11's domain.
use crate::{
    app::AppCtx,
    concerns::{self, Before},
};
use campfire_db::{
    Agent, User, Webhook,
    models::audit_log::{AuditLog, NewAuditLog, Target},
};
use campfire_kit::{Ctx, Error, Result};
enum Rotation {
    NoWebhook,
    Reset(Target),
}
pub async fn create(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let bot = super::find_active_bot(c, "bot_id").await?;
    super::ensure_can_manage_bot(c, &bot).await?;
    concerns::sudo::require_sudo_mode(c)?;
    let id = bot.id;
    if reset_signing_secret(c, bot).await? {
        c.flash()
            .set_notice("Signing secret reset. Update the receiving service with the new secret.");
    } else {
        c.flash()
            .set_alert("Set a webhook URL before generating a signing secret.");
    }
    c.redirect_to(&c.url_for(&campfire_routes::edit_account_bot(id)))
}

/// `create`'s writes once the gates passed: a new signing secret (the agent's, else the legacy
/// webhook's), then the audit. `false` when there's nothing to sign (no agent, no webhook).
pub async fn reset_signing_secret(c: &Ctx, bot: User) -> Result<bool> {
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
            Ok(Rotation::Reset(target))
        })
        .await
        .map_err(Error::internal)?;
    match result {
        Rotation::NoWebhook => Ok(false),
        Rotation::Reset(target) => {
            // Both Rails reset methods commit before the independent audit insert.
            c.app()
                .db
                .write(move |tx| {
                    AuditLog::record(
                        tx,
                        NewAuditLog {
                            action: "agent.webhook_secret.reset".into(),
                            target: Some(target),
                            ..Default::default()
                        },
                        &context,
                    )
                    .map(|_| ())
                })
                .await
                .map_err(Error::internal)?;
            Ok(true)
        }
    }
}
