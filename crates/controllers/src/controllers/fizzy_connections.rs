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
    let token =
        campfire_richtext::ruby::strip(c.param_str("access_token").unwrap_or("")).to_owned();
    if token.is_empty() {
        return redirect(c, false, "Paste a token to connect Fizzy.");
    }
    let identity = match Client::new(Network::system(), token.clone(), &client::api_base_url())
        .identity()
        .await
    {
        Ok(identity) => identity,
        Err(error) => {
            return redirect(
                c,
                false,
                if error.kind == ErrorKind::Unauthorized {
                    "Fizzy rejected that token. Check it and try again."
                } else {
                    "Could not reach Fizzy. Try again."
                },
            );
        }
    };
    let Some(account) = identity["accounts"]
        .as_array()
        .and_then(|a| a.first())
        .filter(|a| !super::fizzy_message_cards::blank(&a["slug"]))
    else {
        return redirect(c, false, "That token has no Fizzy account to use.");
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
    redirect(c, true, &notice)
}

pub async fn destroy(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    concerns::require_sudo_mode(c)?;
    let user = require_current_user(c)?.clone();
    let audit = audit_context(c, &user)?;
    c.app().db.write(move |tx| {
        let existing = Account::for_user(tx.conn(), user.id)?;
        Account::disconnect(tx, user.id)?;
        if let Some(existing) = existing {
            AuditLog::record(tx, NewAuditLog { action: "fizzy.account.disconnect".into(), target: Some((&user).into()), changes: Some(json!({"fizzy_user_name":existing.fizzy_user_name,"fizzy_account_name":existing.account_name})), ..Default::default() }, &audit)?;
        }
        Ok(())
    }).await.map_err(db_error)?;
    redirect(c, true, "Fizzy disconnected.")
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
