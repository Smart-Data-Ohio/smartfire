//! Users::DndAllowancesController: authenticated, current-user-scoped DND exceptions.

use crate::app::AppCtx;
use crate::concerns::{self, Before, cast_integer};
use campfire_db::{DndAllowedUser, User};
use campfire_kit::{Ctx, Error, Redirect, Result, StatusCode};

pub async fn create(c: &mut Ctx) -> Result {
    change(c, true).await
}
pub async fn destroy(c: &mut Ctx) -> Result {
    change(c, false).await
}

async fn change(c: &mut Ctx, create: bool) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let owner = concerns::require_current_user(c)?.id;
    let target = c
        .param_str("user_id")
        .and_then(cast_integer)
        .ok_or(Error::NotFound)?;
    let person = c
        .app()
        .db
        .read(move |conn| User::find_by_id(conn, target))
        .await
        .map_err(Error::internal)?
        .filter(|u| u.is_active() && !u.is_bot())
        .ok_or(Error::NotFound)?;
    if person.id == owner {
        return Ok(c.head(StatusCode::UNPROCESSABLE_ENTITY));
    }
    let result = c
        .app()
        .db
        .write(move |tx| {
            if create {
                DndAllowedUser::find_or_create(tx, owner, target)?;
            } else {
                DndAllowedUser::remove(tx, owner, target)?;
            }
            Ok(())
        })
        .await;
    if let Err(error) = result
        && !error.is_record_not_unique()
    {
        return Err(Error::internal(error));
    }
    let location = c.url_for(&campfire_routes::user(target));
    c.redirect_to_with(
        &location,
        Redirect {
            notice: Some("✓".into()),
            ..Redirect::default()
        },
    )
}
