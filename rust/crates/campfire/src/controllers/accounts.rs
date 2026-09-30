//! `AccountsController` (reference/app/controllers/accounts_controller.rb): account settings.

pub mod bots;
pub mod integrations_health;
pub mod custom_styles;
pub mod join_codes;
pub mod logos;
pub mod users;

use campfire_db::Account;
use campfire_kit::{Ctx, Error, Param, Redirect, Result, StatusCode, format};
use campfire_kit::params::Permit;
use campfire_views::accounts;

use super::presenters::attachments::{self, Assignment, Record};
use super::presenters::pagination::Page;
use super::presenters;
use crate::app::AppCtx;
use crate::controllers::presenters::page::framed_page;
use crate::concerns::{self, Before, current_user};

/// `set_page_and_extract_portion_from users, per_page: 500`
const PER_PAGE: &[i64] = &[500];

/// Everyone, administrators first; the page only decides whether a next-page loader follows.
pub async fn edit(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let account = current_account(c).await?;
    c.respond_to(&[&format::HTML])?;
    let can_administer = current_user(c).is_some_and(|user| user.can_administer(None, false));
    let users = c.app().db.read(move |conn| presenters::accounts::account_users(conn, can_administer)).await.map_err(Error::internal)?;
    let page = Page::new(c.param_str("page"), users.len() as i64, PER_PAGE);

    let secrets = c.app().secrets.clone();
    let (administrators, members): (Vec<_>, Vec<_>) =
        c.app().db.read(move |conn| users.iter().map(|user| presenters::account_user_summary(conn,&secrets,user)).collect::<campfire_db::Result<Vec<_>>>()).await.map_err(Error::internal)?.into_iter().partition(|user| user.administrator());
    let next_page = (!page.is_last()).then(|| page.next_param().to_string());
    let restrict_room_creation_to_administrators = account.settings().restrict_room_creation_to_administrators();
    framed_page!(c, StatusCode::OK, |ctx| accounts::Edit {
        ctx,
        account_id: account.id,
        join_code: account.join_code.clone(),
        restrict_room_creation_to_administrators,
        administrators: administrators.clone(),
        members: members.clone(),
        next_page: next_page.clone(),
    })
    .await
}

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
    let audit = super::two_factor::audit_context(c)?;

    let pending = c
        .app()
        .db
        .write(move |tx| {
            let before = account.clone();
            let before_logo = attachments::attached_blob(tx.conn(), "Account", account.id, "logo")?.is_some();
            let settings: Option<Vec<(&str, &str)>> = settings.as_ref().map(|s| s.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect());
            account.update(tx, name.as_deref(), None, settings.as_deref())?;
            let pending = attachments::assign(tx, Record::account(account.id), "logo", logo)?;
            let after_logo = attachments::attached_blob(tx.conn(), "Account", account.id, "logo")?.is_some();
            crate::account_security::settings_changed(tx, &before, &account, before_logo, after_logo, &audit)?;
            Ok(pending)
        })
        .await
        .map_err(Error::internal)?;
    attachments::analyze_later(c.app(), pending);

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
