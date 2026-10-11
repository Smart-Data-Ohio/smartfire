//! `Sessions::TransfersController` (reference/app/controllers/sessions/transfers_controller.rb):
//! sign in on another device with a user's transfer link.

use campfire_db::User;
use campfire_kit::{Ctx, Error, Result, StatusCode, format};
use campfire_retained::sessions;

use crate::app::AppCtx;
use crate::concerns::Before;
use crate::controllers::auth::{self, ResponseMode};
use crate::controllers::presenters;
use crate::controllers::presenters::page::retained_page;

/// `allow_unauthenticated_access`: an auto-submitting form that PUTs back to this URL.
pub async fn show(c: &mut Ctx) -> Result {
    auth::before_actions(
        c,
        "sessions/transfers#show",
        Before::default().allow_unauthenticated_access(),
        ResponseMode::Html,
    )
    .await?;
    c.respond_to(&[&format::HTML])?;
    // `url_for({})`: this request's own path.
    let action = c.request.path().to_string();
    retained_page!(c, StatusCode::OK, |ctx| sessions::TransferShow {
        ctx,
        action: action.clone()
    })
    .await
}

pub async fn update(c: &mut Ctx) -> Result {
    update_response(c, ResponseMode::Html).await
}

pub async fn update_json(c: &mut Ctx) -> Result {
    let result = update_response(c, ResponseMode::Json).await;
    auth::complete(c, result)
}

async fn update_response(c: &mut Ctx, mode: ResponseMode) -> Result {
    auth::before_actions(
        c,
        "sessions/transfers#update",
        Before::default().allow_unauthenticated_access(),
        mode,
    )
    .await?;
    let transfer_id = c.param_str("id").unwrap_or_default().to_string();
    let user_id =
        presenters::accounts::user_id_from_transfer_id(&c.app().secrets, &transfer_id, c.now());
    // `User.active.find_by_transfer_id(params[:id])`
    let user = match user_id {
        Some(id) => c
            .app()
            .db
            .read(move |conn| Ok(User::find_by_id(conn, id)?.filter(User::is_active)))
            .await
            .map_err(Error::internal)?,
        None => None,
    };
    match user {
        Some(user) => {
            crate::controllers::two_factor::begin_session_response(c, user, "transfer", mode).await
        }
        None => {
            super::record_sign_in_failure(c, "transfer", String::new()).await?;
            if mode == ResponseMode::Json {
                return auth::field_error(
                    c,
                    StatusCode::BAD_REQUEST,
                    "base",
                    "This sign-in link is invalid or expired.",
                );
            }
            Ok(c.head(StatusCode::BAD_REQUEST))
        }
    }
}
