//! `Accounts::Bots::GithubConnectionsController`: administrator and sudo gates;
//! WS15g validates, encrypts, relinks, claims the login and revokes remote grants.
use crate::{
    app::AppCtx,
    concerns::{self, Before},
    integrations::github::{
        accounts::{Account, AccountInput},
        client::ErrorKind,
    },
};
use campfire_db::{
    Agent, User,
    models::audit_log::{AuditLog, NewAuditLog},
};
use campfire_kit::{Ctx, Error, Result};

pub async fn create(c: &mut Ctx) -> Result {
    // Rails ParamsWrapper runs before sudo stores the JSON request for replay.
    c.wrap_parameters("github_connection", None);
    concerns::before_actions(c, Before::default()).await?;
    concerns::ensure_can_administer(c)?;
    let bot = super::find_active_bot(c, "bot_id").await?;
    concerns::sudo::require_sudo_mode(c).await?;
    let token = c
        .params
        .get("access_token")
        .map(super::input_casts::token_string)
        .unwrap_or_default();
    let id = bot.id;
    let (message, notice) = connect(c, bot, &token).await?;
    redirect(c, id, &message, notice)
}

/// `create`'s work once the gates passed: GitHub names the token's login, the account is
/// relinked to the bot, then the audit. Answers the flash: its message, and whether it's a
/// notice (else an alert).
pub async fn connect(c: &Ctx, bot: User, token: &str) -> Result<(String, bool)> {
    let refusal = |message: &str| Ok((message.to_string(), false));
    let token =
        token.trim_matches(|ch| matches!(ch, '\0' | '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' '));
    if campfire_richtext::ruby::is_blank(token) {
        return refusal("Paste a token to connect GitHub.");
    }
    let login = match c
        .app()
        .github_accounts
        .write_client(token.to_owned())
        .authenticated_login()
        .await
    {
        Ok(login) => login,
        Err(error) => {
            return match error.kind {
                ErrorKind::Unauthorized => {
                    refusal("GitHub rejected that token. Check it and try again.")
                }
                ErrorKind::Refused | ErrorKind::Other => {
                    refusal("Could not reach GitHub. Try again.")
                }
                _ => Err(Error::internal(error)),
            };
        }
    };
    let token = token.to_owned();
    let crypto = c.app().ar_encryption.clone();
    let context = super::audit_context(c)?;
    let notice = format!("GitHub connected as {login}.");
    let bot_id = bot.id;
    let audit = c
        .app()
        .db
        .write(move |tx| {
            Account::relink(
                tx,
                &crypto,
                &AccountInput {
                    user_id: bot_id,
                    github_login: &login,
                    access_token: &token,
                    refresh_token: None,
                    token_expires_at: None,
                    token_source: "pat",
                },
            )?;
            let agent = Agent::for_user(tx.conn(), bot_id)?;
            Ok(NewAuditLog {
                action: "agent.github.connect".into(),
                target: Some(super::audit_target(&bot, agent.as_ref())),
                changes: Some(serde_json::json!({"github_login":login})),
                ..Default::default()
            })
        })
        .await
        .map_err(Error::internal)?;
    // Rails account.save! commits before the independent AuditLog.record!.
    c.app()
        .db
        .write(move |tx| AuditLog::record(tx, audit, &context).map(|_| ()))
        .await
        .map_err(Error::internal)?;
    Ok((notice, true))
}

pub async fn destroy(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    concerns::ensure_can_administer(c)?;
    let bot = super::find_active_bot(c, "bot_id").await?;
    concerns::sudo::require_sudo_mode(c).await?;
    let bot_id = bot.id;
    disconnect(c, bot).await?;
    redirect(c, bot_id, "GitHub disconnected.", true)
}

/// `destroy`'s work once the gates passed: the remote grant revoked, the account deleted, then
/// the audit. Nothing happens for a bot without an account.
pub async fn disconnect(c: &Ctx, bot: User) -> Result<()> {
    let bot_id = bot.id;
    let account = c
        .app()
        .db
        .read(move |conn| Account::for_user(conn, bot_id))
        .await
        .map_err(Error::internal)?;
    if let Some(account) = account {
        c.app()
            .github_accounts
            .revoke_remote_token(account.id)
            .await
            .map_err(Error::internal)?;
        let context = super::audit_context(c)?;
        let audit = c
            .app()
            .db
            .write(move |tx| {
                // FLAGGED WS15g destroy seam: GithubConnectedAccount has no destroy
                // callbacks or dependents. Replace this one-row delete with its owner
                // API when supplied; revocation and every other write use that owner.
                tx.conn().execute(
                    "DELETE FROM github_connected_accounts WHERE id=?",
                    [account.id],
                )?;
                let agent = Agent::for_user(tx.conn(), bot_id)?;
                Ok(NewAuditLog {
                    action: "agent.github.disconnect".into(),
                    target: Some(super::audit_target(&bot, agent.as_ref())),
                    changes: Some(serde_json::json!({"github_login":account.github_login})),
                    ..Default::default()
                })
            })
            .await
            .map_err(Error::internal)?;
        // Rails account.destroy! also commits before the audit insert.
        c.app()
            .db
            .write(move |tx| AuditLog::record(tx, audit, &context).map(|_| ()))
            .await
            .map_err(Error::internal)?;
    }
    Ok(())
}

fn redirect(c: &mut Ctx, bot_id: i64, message: &str, notice: bool) -> Result {
    if notice {
        c.flash().set_notice(message);
    } else {
        c.flash().set_alert(message);
    }
    c.redirect_to(&c.url_for(&campfire_routes::edit_account_bot(bot_id)))
}
