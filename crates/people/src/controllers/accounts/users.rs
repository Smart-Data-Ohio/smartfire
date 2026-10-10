//! `Accounts::UsersController` (reference/app/controllers/accounts/users_controller.rb): the
//! people list's next pages, role changes and removal.

pub mod two_factor_resets;

use campfire_db::{Role, User};
use campfire_kit::{Ctx, Error, Result};

use crate::app::AppCtx;
use crate::concerns::{self, Before, cast_integer};

/// `@user.update(role: params.require(:user)[:role].presence_in(%w[ member administrator ]) || "member")`
pub async fn update(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    concerns::ensure_can_administer(c)?;
    let mut user = set_user(c).await?;
    concerns::sudo::require_sudo_mode(c)?;
    let role = match c.params.require("user")?.get("role").and_then(|role| role.as_str()) {
        Some("administrator") => Role::Administrator,
        _ => Role::Member,
    };
    let audit = crate::controllers::two_factor::audit_context(c)?;
    let saved = c.app()
        .db
        .write(move |tx| {
            // Rails validates every persisted preference before saving a new role.
            campfire_db::models::user::profile_settings::update(tx, user.id, Default::default())?;
            crate::authentication::update_role(tx, &mut user, role, &audit)
        })
        .await;
    match saved {
        // Accounts::UsersController redirects after an unsuccessful non-bang update.
        Ok(()) | Err(campfire_db::Error::RecordInvalid(_)) => (),
        Err(error) => return Err(Error::internal(error)),
    }
    redirect_to_edit_account(c)
}

/// `@user.deactivate`
pub async fn destroy(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    concerns::ensure_can_administer(c)?;
    let mut user = set_user(c).await?;
    concerns::sudo::require_sudo_mode(c)?;
    let audit = crate::controllers::two_factor::audit_context(c)?;
    crate::integrations::google::calendar::stop_remote(c.app(), user.id).await.map_err(Error::internal)?;
    c.app()
        .db
        .write(move |tx| crate::authentication::deactivate_user(tx, &mut user, &audit))
        .await
        .map_err(Error::internal)?;
    redirect_to_edit_account(c)
}

/// `User.active.find(params[:user_id] || params[:id])`
async fn set_user(c: &Ctx) -> Result<User> {
    let id = c.param_str("user_id").or_else(|| c.param_str("id")).and_then(cast_integer).ok_or(Error::NotFound)?;
    c.app()
        .db
        .read(move |conn| match User::find_active(conn, id) {
            Ok(user) => Ok(Some(user)),
            Err(campfire_db::Error::RecordNotFound(_)) => Ok(None),
            Err(error) => Err(error),
        })
        .await
        .map_err(Error::internal)?
        .ok_or(Error::NotFound)
}

fn redirect_to_edit_account(c: &mut Ctx) -> Result {
    let location = c.url_for(&campfire_routes::edit_account());
    c.redirect_to(&location)
}
