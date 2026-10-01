//! `app/controllers/users/tours_controller.rb`: skipping and finishing share this stamp.
use crate::app::AppCtx;
use crate::concerns::{self, Before};
use campfire_db::User;
use campfire_kit::{Ctx, Error, Result, StatusCode};

pub async fn update(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let user_id = concerns::require_current_user(c)?.id;
    c.app()
        .db
        .write(move |tx| User::complete_tour(tx, user_id))
        .await
        .map_err(Error::internal)?;
    Ok(c.head(StatusCode::NO_CONTENT))
}
