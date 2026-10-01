//! `Users::BansController` (reference/app/controllers/users/bans_controller.rb).

use campfire_kit::{Ctx, Error, Result, StatusCode};

use super::find_user;
use crate::app::AppCtx;
use crate::concerns::{self, Before};

/// `before_action :ensure_can_administer, :set_user`; `@user.ban`
pub async fn create(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    concerns::ensure_can_administer(c)?;
    let mut user = find_user(c, "user_id").await?;
    concerns::sudo::require_sudo_mode(c)?;
    let id = user.id;
    let audit = crate::controllers::two_factor::audit_context(c)?;
    c.app()
        .db
        .write(move |tx| {
            campfire_db::models::user::profile_settings::update(tx, user.id, Default::default())?;
            crate::authentication::set_user_banned(tx, &mut user, true, &audit)
        })
        .await
        .map_err(save_error)?;
    redirect_to_user(c, id)
}

/// `@user.unban`
pub async fn destroy(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    concerns::ensure_can_administer(c)?;
    let mut user = find_user(c, "user_id").await?;
    concerns::sudo::require_sudo_mode(c)?;
    let id = user.id;
    let audit = crate::controllers::two_factor::audit_context(c)?;
    c.app()
        .db
        .write(move |tx| {
            campfire_db::models::user::profile_settings::update(tx, user.id, Default::default())?;
            crate::authentication::set_user_banned(tx, &mut user, false, &audit)
        })
        .await
        .map_err(save_error)?;
    redirect_to_user(c, id)
}

/// `redirect_to @user`
fn redirect_to_user(c: &mut Ctx, id: i64) -> Result {
    let location = c.url_for(&campfire_routes::user(id));
    c.redirect_to(&location)
}

fn save_error(error: campfire_db::Error) -> Error {
    match error {
        campfire_db::Error::RecordInvalid(_) => Error::Status(StatusCode::UNPROCESSABLE_ENTITY),
        error => Error::internal(error),
    }
}
