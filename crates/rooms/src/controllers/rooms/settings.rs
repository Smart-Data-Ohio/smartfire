//! `GET /rooms/:room_id/settings`. The route is declared and the classic controller is not.
//! Someone using the SPA is sent to `/app/r/:id/settings` by the coexistence redirect. Everyone
//! else lands on that room type's edit form.

use campfire_db::RoomType;
use campfire_kit::{Ctx, Result};

use super::{Scope, set_room};
use crate::concerns::{Before, before_actions};

pub async fn show(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = set_room(c, Scope::All).await?;
    let path = match room.room_type {
        RoomType::Open => campfire_routes::edit_rooms_open(room.id),
        RoomType::Closed => campfire_routes::edit_rooms_closed(room.id),
        RoomType::Direct => campfire_routes::edit_rooms_direct(room.id),
        RoomType::Voice => campfire_routes::edit_rooms_voice(room.id),
        RoomType::Stage => campfire_routes::edit_rooms_stage(room.id),
        RoomType::Board => campfire_routes::edit_rooms_board(room.id),
    };
    c.redirect_to(&c.url_for(&path))
}
