//! `app/controllers/rooms/categories_controller.rb`.
use crate::app::AppCtx;
use crate::concerns::{self, Before, before_actions, cast_integer};
use crate::controllers::{presenters::page::db_error, room_categories};
use campfire_db::RoomType;
use campfire_kit::{Ctx, Error, Result, StatusCode};

pub async fn update(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (mut membership, room) = concerns::set_room(c).await?;
    if !matches!(room.room_type, RoomType::Open | RoomType::Closed) {
        return Ok(c.head(StatusCode::UNPROCESSABLE_ENTITY));
    }
    let category = match c
        .param("room_category_id")
        .filter(|value| value.is_present())
    {
        Some(value) => {
            let id = value
                .to_s()
                .as_deref()
                .and_then(cast_integer)
                .ok_or(Error::NotFound)?;
            Some(room_categories::find_for_user(c, id).await?.id)
        }
        None => None,
    };
    c.app()
        .db
        .write(move |tx| membership.update_category(tx, category))
        .await
        .map_err(db_error)?;
    Ok(c.head(StatusCode::OK))
}
