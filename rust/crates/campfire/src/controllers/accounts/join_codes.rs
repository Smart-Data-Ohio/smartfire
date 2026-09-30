//! `Accounts::JoinCodesController` (reference/app/controllers/accounts/join_codes_controller.rb).

use campfire_kit::{Ctx, Error, Result};

use crate::app::AppCtx;
use crate::concerns::{self, Before};

/// `Current.account.reset_join_code`
pub async fn create(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    concerns::ensure_can_administer(c)?;
    concerns::sudo::require_sudo_mode(c)?;
    let mut account = super::current_account(c).await?;
    let audit = crate::controllers::two_factor::audit_context(c)?;
    c.app().db.write(move |tx| crate::account_security::reset_join_code(tx, &mut account, &audit)).await.map_err(Error::internal)?;
    let location = c.url_for(&campfire_routes::edit_account());
    c.redirect_to(&location)
}
