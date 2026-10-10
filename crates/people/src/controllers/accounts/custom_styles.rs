//! `Accounts::CustomStylesController` (reference/app/controllers/accounts/custom_styles_controller.rb).

use campfire_kit::{Ctx, Error, Param, Redirect, Result, permit_keys};

use crate::app::AppCtx;
use crate::concerns::{self, Before};

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
    let css = account.custom_styles.clone();
    c.app().db.write(move |tx| crate::account_security::styles_changed(tx, &before, &account, &audit)).await.map_err(Error::internal)?;
    campfire_app::cable::sync::workspace_styles_updated(&c.app().cable, css);
    let location = c.url_for(&campfire_routes::edit_account_custom_styles());
    c.redirect_to_with(&location, Redirect { notice: Some("✓".into()), ..Redirect::default() })
}
