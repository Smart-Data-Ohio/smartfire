//! `Fizzy::ConnectionsController`: sudo-protected, owner-scoped PAT linking.
use crate::{
    app::AppCtx,
    concerns::{self, Before, before_actions, require_current_user},
    controllers::presenters::page::db_error,
    integrations::{
        fizzy::{
            accounts::{Account, Input},
            client::{self, Client, ErrorKind},
        },
        net::Network,
    },
};
use campfire_db::{
    User,
    audit_log::{AuditLog, Context, NewAuditLog},
};
use campfire_kit::{Ctx, Result};
use rails_compat::ar_encryption::ArEncryption;
use serde_json::json;

pub async fn create(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    concerns::require_sudo_mode(c)?;
    let user = require_current_user(c)?.clone();
    let token = c.param_str("access_token").unwrap_or("").to_owned();
    match connect_token(c, user, &token).await? {
        Ok(notice) => redirect(c, true, &notice),
        Err(alert) => redirect(c, false, &alert),
    }
}

/// The PAT action after authentication and sudo: the classic notice or alert.
pub async fn connect_token(
    c: &Ctx,
    user: User,
    token: &str,
) -> Result<std::result::Result<String, String>> {
    let token = campfire_richtext::ruby::strip(token).to_owned();
    if token.is_empty() {
        return Ok(Err("Paste a token to connect Fizzy.".into()));
    }
    let identity = match Client::new(Network::system(), token.clone(), &client::api_base_url())
        .identity()
        .await
    {
        Ok(identity) => identity,
        Err(error) => {
            let alert = if error.kind == ErrorKind::Unauthorized {
                "Fizzy rejected that token. Check it and try again."
            } else {
                "Could not reach Fizzy. Try again."
            };
            return Ok(Err(alert.into()));
        }
    };
    let Some(account) = identity["accounts"]
        .as_array()
        .and_then(|a| a.first())
        .filter(|a| !super::fizzy_message_cards::blank(&a["slug"]))
    else {
        return Ok(Err("That token has no Fizzy account to use.".into()));
    };
    let account = account.clone();
    let crypto = ArEncryption::new(&c.app().secrets);
    let audit = audit_context(c, &user)?;
    let notice = c.app().db.write(move |tx| {
        let slug = account["slug"].as_str().unwrap_or("");
        let linked = Account::relink(tx, &crypto, &Input { user_id: user.id, account_id: slug.strip_prefix('/').unwrap_or(slug), account_name: account["name"].as_str(), fizzy_user_id: account["user"]["id"].as_str(), fizzy_user_name: account["user"]["name"].as_str(), token: &token })?;
        AuditLog::record(tx, NewAuditLog { action: "fizzy.account.connect".into(), target: Some((&user).into()), changes: Some(json!({"fizzy_user_name":linked.fizzy_user_name,"fizzy_account_name":linked.account_name})), ..Default::default() }, &audit)?;
        Ok(format!("Fizzy connected as {} ({}).", linked.fizzy_user_name.as_deref().unwrap_or(""), linked.account_name.as_deref().unwrap_or("")))
    }).await.map_err(db_error)?;
    Ok(Ok(notice))
}

pub async fn destroy(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    concerns::require_sudo_mode(c)?;
    let user = require_current_user(c)?.clone();
    let notice = disconnect_user(c, user).await?;
    redirect(c, true, &notice)
}

/// The disconnect action after authentication and sudo, including cache cleanup and its audit.
pub async fn disconnect_user(c: &Ctx, user: User) -> Result<String> {
    let audit = audit_context(c, &user)?;
    c.app().db.write(move |tx| {
        let existing = Account::for_user(tx.conn(), user.id)?;
        Account::disconnect(tx, user.id)?;
        if let Some(existing) = existing {
            AuditLog::record(tx, NewAuditLog { action: "fizzy.account.disconnect".into(), target: Some((&user).into()), changes: Some(json!({"fizzy_user_name":existing.fizzy_user_name,"fizzy_account_name":existing.account_name})), ..Default::default() }, &audit)?;
        }
        Ok(())
    }).await.map_err(db_error)?;
    Ok("Fizzy disconnected.".into())
}

pub(crate) fn redirect(c: &mut Ctx, notice: bool, message: &str) -> Result {
    c.flash()
        .set(if notice { "notice" } else { "alert" }, message);
    c.redirect_to(&c.url_for("/users/me/profile"))
}
fn audit_context(c: &Ctx, user: &User) -> Result<Context> {
    Ok(Context {
        actor: Some(user.into()),
        ip_address: Some(c.request.remote_ip()?.to_string()),
        user_agent: c.request.header("user-agent").map(str::to_owned),
    })
}
