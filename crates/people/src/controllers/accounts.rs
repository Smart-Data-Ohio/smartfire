//! `AccountsController` (reference/app/controllers/accounts_controller.rb): account settings.

pub mod audit_logs;
pub mod banners;
pub mod icons;
pub mod bots;
pub mod integrations_health;
pub mod custom_styles;
pub mod join_codes;
pub mod logos;
pub mod users;

use campfire_db::Account;
use campfire_kit::{Ctx, Error, Param, Redirect, Result};
use campfire_kit::params::Permit;

use super::presenters::attachments::{self, Assignment, Record};
use super::presenters;
use crate::app::AppCtx;
use crate::concerns::{self, Before};

/// `@account.update!(params.require(:account).permit(:name, :logo, settings: {}))`
pub async fn update(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    concerns::ensure_can_administer(c)?;
    let mut account = current_account(c).await?;

    let params = c.params.require("account")?.permit(&[Permit::from("name"), Permit::from("logo"), Permit::AnyHash("settings".into())]);
    let name = params.get("name").and_then(Param::to_s);
    let settings: Option<Vec<(String, String)>> = params.get("settings").and_then(Param::as_hash).map(|settings| {
        settings.iter().map(|(key, value)| (key.clone(), value.to_s().unwrap_or_else(|| campfire_richtext::ruby::json_value_inspect(&value.to_json())))).collect()
    });
    let logo = Assignment::from_params(&params, "logo")?.stage(c.app()).await?;
    let secrets = c.app().secrets.clone();
    let audit = super::two_factor::audit_context(c)?;

    let (before, account, before_logo, after_logo) = c
        .app()
        .db
        .write(move |tx| {
            let before = account.clone();
            let before_logo = attachments::attached_blob(tx.conn(), "Account", account.id, "logo")?.is_some();
            let settings: Option<Vec<(&str, &str)>> = settings.as_ref().map(|s| s.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect());
            account.update(tx, name.as_deref(), None, settings.as_deref())?;
            attachments::assign(tx, Record::account(account.id, &secrets), "logo", logo)?;
            let after_logo = attachments::attached_blob(tx.conn(), "Account", account.id, "logo")?.is_some();
            Ok((before, account, before_logo, after_logo))
        })
        .await
        .map_err(Error::internal)?;

    // Rails record_settings_changes runs only after update! and its after_commit callbacks
    // return. In particular, failed NullAnalyzer metadata leaves the logo saved and no audit.
    c.app().db.write(move |tx| {
        crate::account_security::settings_changed(tx, &before, &account, before_logo, after_logo, None, &audit)
    }).await.map_err(Error::internal)?;

    presenters::workspace_branding::publish(c.app()).await;
    let location = c.url_for(&campfire_routes::edit_account());
    c.redirect_to_with(&location, Redirect { notice: Some("✓".into()), ..Redirect::default() })
}

/// `Current.account` where the reference dereferences it (a nil account raises NoMethodError).
pub async fn current_account(c: &Ctx) -> Result<Account> {
    c.app()
        .db
        .read(Account::first)
        .await
        .map_err(Error::internal)?
        .ok_or_else(|| Error::internal(anyhow::anyhow!("no account")))
}
