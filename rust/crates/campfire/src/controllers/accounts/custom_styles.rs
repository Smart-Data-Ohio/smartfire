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
    let custom_styles = params.contains_key("custom_styles").then(|| params.get("custom_styles").filter(|p| !p.is_null()).and_then(Param::to_s));
    let audit = crate::controllers::two_factor::audit_context(c)?;
    c.app()
        .db
        .write(move |tx| campfire_db::models::account_mutations::change_custom_styles(tx, &mut account, custom_styles.as_ref().map(|styles| styles.as_deref()), &audit))
        .await
        .map_err(Error::internal)?;
    let location = c.url_for(&campfire_routes::edit_account_custom_styles());
    c.redirect_to_with(&location, Redirect { notice: Some("✓".into()), ..Redirect::default() })
}
