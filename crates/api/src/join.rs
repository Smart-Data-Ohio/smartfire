//! Nonmember preview and join for an alive open room (`rooms#show`'s join preview and `rooms#join`).

use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_kit::{Ctx, Error, Result, StatusCode};
use campfire_rooms::controllers::rooms::{find_joinable_open_room, join_open_room};
use campfire_web::concerns::{self, require_current_user};
use campfire_web::controllers::presenters::page::db_error;

use crate::dto;
use crate::endpoints::{before_actions, now};

endpoint!(
    /// `GET /api/v1/rooms/:room_id/preview`
    preview => show_preview
);
endpoint!(
    /// `POST /api/v1/rooms/:room_id/join`
    join => join_room
);

fn room_id(c: &Ctx) -> Option<i64> {
    c.param_str("room_id").and_then(concerns::cast_integer)
}

async fn show_preview(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let Some(id) = room_id(c) else {
        return Err(Error::NotFound);
    };
    let Some(room) = find_joinable_open_room(c, id).await? else {
        return Err(Error::NotFound);
    };
    c.json(
        StatusCode::OK,
        &api::OpenRoomPreview {
            id: room.id,
            name: room.name.unwrap_or_default(),
        },
    )
}

async fn join_room(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let Some(id) = room_id(c) else {
        return Err(Error::NotFound);
    };
    let Some((room, membership)) = join_open_room(c, id).await? else {
        return Err(Error::NotFound);
    };
    let viewer = require_current_user(c)?.clone();
    let (secrets, now) = (c.app().secrets.clone(), now(c));
    let joined = c
        .app()
        .db
        .read_snapshot(move |conn| {
            let detail = dto::room_detail(conn, &secrets, &viewer, &room, &membership, now)?;
            // `None` for an invisible involvement: that membership has no sidebar row.
            // Classic still redirects into the room, so the detail is the whole answer.
            let row = dto::sidebar_row(conn, &room, &membership)?;
            Ok(api::RoomJoin { detail, row })
        })
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &joined)
}
