//! The signed-in member's session list and scoped revocation.
use crate::app::AppCtx;
use crate::concerns::{self, Before, current_session, require_current_user};
use campfire_db::Session;
use campfire_kit::{Ctx, Error, Redirect, Result, StatusCode};

async fn before(c: &mut Ctx) -> Result<()> {
    concerns::before_actions(c, Before::default()).await?;
    c.set_header("cache-control", "no-store");
    c.set_header("pragma", "no-cache");
    Ok(())
}
pub async fn destroy(c: &mut Ctx) -> Result {
    before(c).await?;
    let user = require_current_user(c)?.clone();
    let id = concerns::ruby_to_i(c.param_str("id").unwrap_or(""));
    let user_id = user.id;
    let session = c
        .app()
        .db
        .read(move |conn| {
            Ok(Session::for_user(conn, user_id)?
                .into_iter()
                .find(|s| s.id == id))
        })
        .await
        .map_err(Error::internal)?
        .ok_or(Error::Status(StatusCode::NOT_FOUND))?;
    if current_session(c).is_some_and(|s| s.id == id) {
        crate::controllers::sessions::remove_push_subscription(c).await?;
        concerns::terminate_current_session(c).await?;
        return c.redirect_to(&c.url_for("/"));
    }
    let context = crate::controllers::two_factor::audit_context(c)?;
    c.app()
        .db
        .write(move |tx| crate::authentication::revoke_session(tx, &user, &session, &context))
        .await
        .map_err(Error::internal)?;
    redirect(c, "Signed out that session.".into())
}
pub async fn revoke_others(c: &mut Ctx) -> Result {
    before(c).await?;
    let user = require_current_user(c)?.clone();
    let current = current_session(c).expect("authenticated session").id;
    let context = crate::controllers::two_factor::audit_context(c)?;
    let count = c
        .app()
        .db
        .write(move |tx| crate::authentication::revoke_other_sessions(tx, &user, current, &context))
        .await
        .map_err(Error::internal)?;
    let notice = match count {
        0 => "No other sessions to sign out.".into(),
        1 => "Signed out 1 other session.".into(),
        _ => format!("Signed out {count} other sessions."),
    };
    redirect(c, notice)
}
fn redirect(c: &mut Ctx, notice: String) -> Result {
    c.redirect_to_with(
        &c.url_for("/users/me/sessions"),
        Redirect {
            notice: Some(notice),
            ..Default::default()
        },
    )
}
