//! `app/controllers/rooms/inbound_email_addresses_controller.rb`, on WS10's token domain.
use crate::app::AppCtx;
use crate::concerns::{self, Before, before_actions};
use crate::controllers::presenters::page::db_error;
use campfire_kit::{Ctx, Redirect, Result, StatusCode};

pub async fn create(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (_, mut room) = concerns::set_room(c).await?;
    if !room.emailable() {
        return campfire_kit::halt(concerns::head(StatusCode::NOT_FOUND));
    }
    super::ensure_can_administer(c, &room)?;
    let location = c.url_for(&campfire_routes::edit_room(room.id));
    c.app()
        .db
        .write(move |tx| room.regenerate_inbound_email_token(tx))
        .await
        .map_err(db_error)?;
    c.redirect_to_with(
        &location,
        Redirect {
            notice: Some("Room email address rotated.".into()),
            ..Redirect::default()
        },
    )
}
