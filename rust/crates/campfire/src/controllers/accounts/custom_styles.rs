//! `Accounts::CustomStylesController` (reference/app/controllers/accounts/custom_styles_controller.rb).

use campfire_kit::{Ctx, Error, Param, Redirect, Result, StatusCode, format, permit_keys};
use campfire_views::accounts;

use crate::app::AppCtx;
use crate::controllers::presenters::page::framed_page;
use crate::concerns::{self, Before};

/// `before_action :ensure_can_administer, :set_account`
pub async fn edit(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    concerns::ensure_can_administer(c)?;
    let account = super::current_account(c).await?;
    c.respond_to(&[&format::HTML])?;
    let custom_styles = account.custom_styles;
    framed_page!(c, StatusCode::OK, |ctx| accounts::CustomStylesEdit { ctx, custom_styles: custom_styles.clone() }).await
}

/// `@account.update!(params.require(:account).permit(:custom_styles))`
pub async fn update(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    concerns::ensure_can_administer(c)?;
    let mut account = super::current_account(c).await?;
    concerns::sudo::require_sudo_mode(c)?;
    let params = c.params.require("account")?.permit(&permit_keys(&["custom_styles"]));
    // ActiveModel::Type::String retains nil and casts booleans to "t"/"f".
    let custom_styles = params.get("custom_styles").map(|value| match value {
        Param::Null => None,
        Param::Bool(value) => Some(if *value { "t" } else { "f" }.into()),
        value => value.to_s(),
    });
    let audit = crate::controllers::two_factor::audit_context(c)?;
    let (before, account) = c.app()
        .db
        .write(move |tx| {
            let before = account.clone();
            account.update(tx, None, custom_styles.as_ref().map(|styles| styles.as_deref()), None)?;
            Ok((before, account))
        })
        .await
        .map_err(Error::internal)?;
    // Rails update! and all save callbacks return before auditing the changed styles.
    c.app().db.write(move |tx| crate::account_security::styles_changed(tx, &before, &account, &audit)).await.map_err(Error::internal)?;
    let location = c.url_for(&campfire_routes::edit_account_custom_styles());
    c.redirect_to_with(&location, Redirect { notice: Some("✓".into()), ..Redirect::default() })
}
