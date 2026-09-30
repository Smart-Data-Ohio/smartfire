//! `Accounts::Bots::KeysController` (reference/app/controllers/accounts/bots/keys_controller.rb).

use campfire_kit::{Ctx, Error, Result};

use crate::app::AppCtx;
use crate::concerns::{self, Before};

/// `User.active_bots.find(params[:bot_id]).reset_bot_key`
pub async fn update(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    concerns::ensure_can_administer(c)?;
    concerns::sudo::require_sudo_mode(c)?;
    let mut bot = super::find_active_bot(c, "bot_id").await?;
    c.app().db.write(move |tx| bot.reset_bot_key(tx)).await.map_err(Error::internal)?;
    let location = c.url_for(&campfire_routes::account_bots());
    c.redirect_to(&location)
}
