//! `Accounts::Bots::KeysController` (app/controllers/accounts/bots/keys_controller.rb).

use campfire_db::models::audit_log::{AuditLog, Context, NewAuditLog, Target};
use campfire_kit::{Ctx, Error, Result, StatusCode};
use campfire_views::accounts;

use crate::app::AppCtx;
use crate::concerns::{self, Before, require_current_user};
use crate::controllers::presenters::page::framed_page;

/// Reset requires sudo, audits the rotation, and displays the key once without caching it.
pub async fn update(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    concerns::ensure_can_administer(c)?;
    concerns::sudo::require_sudo_mode(c)?;
    let mut bot = super::find_active_bot(c, "bot_id").await?;
    let name = bot.name.clone();
    let context = Context {
        actor: Some(require_current_user(c)?.into()),
        ip_address: Some(c.request.remote_ip()?.to_string()),
        user_agent: c.request.user_agent().map(str::to_string),
    };
    let (key, audit) = c
        .app()
        .db
        .write(move |tx| {
            let key = bot.reset_bot_key(tx)?;
            // Rails targets the Agent when one exists, otherwise the legacy bot.
            let agent_id =
                tx.conn()
                    .query_row("SELECT id FROM agents WHERE user_id=?", [bot.id], |r| {
                        r.get::<_, i64>(0)
                    });
            let target = match agent_id {
                Ok(id) => Target {
                    record_type: "Agent".into(),
                    id,
                    label: Some(format!("Agent {}", bot.name)),
                },
                Err(rusqlite::Error::QueryReturnedNoRows) => Target::from(&bot),
                Err(error) => return Err(error.into()),
            };
            Ok((
                key,
                NewAuditLog {
                    action: "agent.credential.reset".into(),
                    target: Some(target),
                    ..Default::default()
                },
            ))
        })
        .await
        .map_err(Error::internal)?;
    // Rails reset_bot_key commits before AuditLog.record!; an audit error cannot restore the old key.
    c.app()
        .db
        .write(move |tx| AuditLog::record(tx, audit, &context).map(|_| ()))
        .await
        .map_err(Error::internal)?;
    c.set_header("cache-control", "no-store");
    c.set_header("pragma", "no-cache");
    framed_page!(c, StatusCode::OK, |ctx| accounts::BotKey {
        ctx,
        bot_name: &name,
        bot_key: &key
    })
    .await
}
