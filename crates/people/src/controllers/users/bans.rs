//! `Users::BansController` (reference/app/controllers/users/bans_controller.rb).

use campfire_kit::{Ctx, Error, Result, StatusCode};

use super::find_user;
use crate::app::AppCtx;
use crate::concerns::{self, Before};

/// `before_action :ensure_can_administer, :set_user`; `@user.ban`
pub async fn create(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    concerns::ensure_can_administer(c)?;
    let user = find_user(c, "user_id").await?;
    concerns::sudo::require_sudo_mode(c)?;
    let id = user.id;
    set_banned(c, user, true).await.map_err(save_error)?;
    redirect_to_user(c, id)
}

/// `@user.unban`
pub async fn destroy(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    concerns::ensure_can_administer(c)?;
    let user = find_user(c, "user_id").await?;
    concerns::sudo::require_sudo_mode(c)?;
    let id = user.id;
    set_banned(c, user, false).await.map_err(save_error)?;
    redirect_to_user(c, id)
}

/// The transaction shared by classic bans and their JSON twins. Preserve `RecordInvalid` for
/// the API's `Validation` envelope; classic handlers map it to their original bare 422.
pub async fn set_banned(c: &Ctx, mut user: campfire_db::User, banned: bool) -> Result<()> {
    let audit = crate::controllers::two_factor::audit_context(c)?;
    c.app()
        .db
        .write(move |tx| {
            campfire_db::models::user::profile_settings::update(tx, user.id, Default::default())?;
            crate::authentication::set_user_banned(tx, &mut user, banned, &audit)
        })
        .await
        .map_err(Error::internal)
}

/// `redirect_to @user`
fn redirect_to_user(c: &mut Ctx, id: i64) -> Result {
    let location = c.url_for(&campfire_routes::user(id));
    c.redirect_to(&location)
}

fn save_error(error: Error) -> Error {
    if let Error::Internal(source) = &error
        && matches!(
            source.downcast_ref::<campfire_db::Error>(),
            Some(campfire_db::Error::RecordInvalid(_))
        )
    {
        return Error::Status(StatusCode::UNPROCESSABLE_ENTITY);
    }
    error
}
