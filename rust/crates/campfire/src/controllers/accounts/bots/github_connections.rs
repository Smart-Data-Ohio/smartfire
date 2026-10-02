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
    Agent,
    models::audit_log::{AuditLog, NewAuditLog},
};
use campfire_kit::{Ctx, Error, Result};

pub async fn create(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    concerns::ensure_can_administer(c)?;
    let bot = super::find_active_bot(c, "bot_id").await?;
    concerns::sudo::require_sudo_mode(c)?;
    let token = c
        .params
        .get("access_token")
        .map(super::input_casts::token_string)
        .unwrap_or_default();
    let token =
        token.trim_matches(|ch| matches!(ch, '\0' | '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' '));
    if campfire_richtext::ruby::is_blank(token) {
        return redirect(c, bot.id, "Paste a token to connect GitHub.", false);
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
                ErrorKind::Unauthorized => redirect(
                    c,
                    bot.id,
                    "GitHub rejected that token. Check it and try again.",
                    false,
                ),
                ErrorKind::Refused | ErrorKind::Other => {
                    redirect(c, bot.id, "Could not reach GitHub. Try again.", false)
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
    redirect(c, bot_id, &notice, true)
}

pub async fn destroy(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    concerns::ensure_can_administer(c)?;
    let bot = super::find_active_bot(c, "bot_id").await?;
    concerns::sudo::require_sudo_mode(c)?;
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
    redirect(c, bot_id, "GitHub disconnected.", true)
}

fn redirect(c: &mut Ctx, bot_id: i64, message: &str, notice: bool) -> Result {
    if notice {
        c.flash().set_notice(message);
    } else {
        c.flash().set_alert(message);
    }
    c.redirect_to(&c.url_for(&campfire_routes::edit_account_bot(bot_id)))
}

/// Preserve numeric JSON lexemes solely for this controller's token `.to_s`.
/// All other routes and attributes retain the kit's ordinary parameter parser.
pub(crate) fn json_body_params(
    method: &campfire_kit::Method,
    path: &str,
    raw: &[u8],
) -> std::result::Result<campfire_kit::ParamMap, campfire_kit::params::ParamError> {
    use campfire_kit::params::{self, ParamError};
    use campfire_kit::{Param, ParamMap};
    let is_token = crate::controllers::recognize(method, path)
        .ok()
        .flatten()
        .is_some_and(|(route, _)| route.endpoint == "accounts/bots/github_connections#create");
    if !is_token {
        return params::from_json_body(raw);
    }
    let value: Box<serde_json::value::RawValue> =
        serde_json::from_slice(raw).map_err(|_| ParamError::Parse)?;
    if !value.get().starts_with('{') {
        return params::from_json_body(raw);
    }
    let fields: indexmap::IndexMap<String, Box<serde_json::value::RawValue>> =
        serde_json::from_str(value.get()).map_err(|_| ParamError::Parse)?;
    let mut result = ParamMap::new();
    for (key, value) in fields {
        let param = if key == "access_token" {
            Param::Str(
                super::input_casts::json_token_string(&value).map_err(|_| ParamError::Parse)?,
            )
        } else {
            Param::from_json(serde_json::from_str(value.get()).map_err(|_| ParamError::Parse)?)
        };
        result.insert(key, param);
    }
    Ok(result)
}

/// Other routes keep the already-parsed parameters without decoding them twice.
pub(crate) fn scoped_json_body_params(
    method: &campfire_kit::Method,
    path: &str,
    raw: &[u8],
) -> Option<std::result::Result<campfire_kit::ParamMap, campfire_kit::params::ParamError>> {
    if *method != campfire_kit::Method::POST {
        return None;
    }
    let is_token = crate::controllers::recognize(method, path)
        .ok()
        .flatten()
        .is_some_and(|(route, _)| route.endpoint == "accounts/bots/github_connections#create");
    is_token.then(|| json_body_params(method, path, raw))
}
