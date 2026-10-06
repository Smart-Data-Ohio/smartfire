//! Administrator recovery, excluding self, bots, inactive and unenrolled targets.
use crate::app::AppCtx;
use crate::concerns::{self, Before, require_current_user};
use campfire_db::User;
use campfire_kit::{Ctx, Error, Redirect, Result, StatusCode};
pub async fn create(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    concerns::ensure_can_administer(c)?;
    let id = concerns::ruby_to_i(c.param_str("user_id").unwrap_or(""));
    let user = c
        .app()
        .db
        .read(move |conn| Ok(User::find_by_id(conn, id)?.filter(|u| u.is_active() && !u.is_bot())))
        .await
        .map_err(Error::internal)?
        .ok_or(Error::Status(StatusCode::NOT_FOUND))?;
    if user.id == require_current_user(c)?.id {
        return redirect(c,Some("Reset someone else's two-step sign-in from here. To change your own, use Disable on your profile.".into()),None);
    }
    let user_id = user.id;
    let enabled = c
        .app()
        .db
        .read(move |conn| User::find(conn, user_id)?.two_factor_enabled(conn))
        .await
        .map_err(Error::internal)?;
    if !enabled {
        return redirect(
            c,
            Some(format!(
                "{} doesn't have two-step sign-in enabled.",
                user.name
            )),
            None,
        );
    }
    let name = user.name.clone();
    let context = crate::controllers::two_factor::audit_context(c)?;
    c.app()
        .db
        .write(move |tx| crate::authentication::reset_two_factor(tx, &user, &context))
        .await
        .map_err(Error::internal)?;
    redirect(
        c,
        None,
        Some(format!(
            "Two-step sign-in reset for {name}. They will set it up again at next sign-in."
        )),
    )
}
fn redirect(c: &mut Ctx, alert: Option<String>, notice: Option<String>) -> Result {
    c.redirect_to_with(
        &c.url_for("/account/edit"),
        Redirect {
            alert,
            notice,
            ..Default::default()
        },
    )
}
