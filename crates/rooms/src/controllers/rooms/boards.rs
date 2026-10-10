//! Rails Rooms::BoardsController: board-only scope, explicit memberships, and room audits.

use campfire_db::{Room, RoomType};
use campfire_kit::{Ctx, Result, StatusCode};

use super::{
    Scope, ensure_can_administer, ensure_permission_to_create_rooms, redirect_to_room,
    room_icon_param, room_name_param, set_room, user_ids_param,
};
use crate::app::AppCtx;
use crate::concerns::{Before, before_actions, require_current_user};
use crate::controllers::presenters::page::db_error;

/// Same deletion as `RoomsController#destroy`: membership scope, the administer gate, then
/// the room's threads, posts and work rows go with the destroy job.
pub async fn destroy(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = set_room(c, Scope::Boards).await?;
    super::ensure_can_delete(c, &room).await?;
    super::destroy_room(c, room).await
}

pub async fn create(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    ensure_permission_to_create_rooms(c).await?;
    let name = room_name_param(c)?.flatten();
    let icon = room_icon_param(c)?.flatten();
    let grantee_ids = user_ids_param(c);
    // Rooms::Board.create_for(room_params, users: grantees)
    let room = super::operations::create(
        c,
        RoomType::Board,
        name,
        icon,
        require_current_user(c)?.id,
        grantee_ids,
    )
    .await;
    let room = match room {
        Ok(room) => room,
        Err(campfire_db::Error::RecordInvalid(_errors)) => {
            return Ok(c.head(StatusCode::UNPROCESSABLE_ENTITY));
        }
        Err(error) => return Err(db_error(error)),
    };
    super::audit_room(
        c,
        &room,
        "room.create",
        serde_json::json!({"name":room.name}),
    )
    .await?;
    broadcast_to_members(c, &room, false).await?;
    redirect_to_room(c, room.id)
}

pub async fn update(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = set_room(c, Scope::Boards).await?;
    ensure_can_administer(c, &room)?;
    let name = room_name_param(c)?;
    let icon = room_icon_param(c)?;
    let grantee_ids = user_ids_param(c);
    // Board updates retain their existing STI type.
    let room = super::operations::update(c, room, name, icon, None).await;
    let room = match room {
        Ok(room) => room,
        Err(campfire_db::Error::RecordInvalid(_errors)) => {
            return Ok(c.head(StatusCode::UNPROCESSABLE_ENTITY));
        }
        Err(error) => return Err(db_error(error)),
    };
    // `@room.memberships.revise(granted: grantees, revoked: revokees)`
    super::operations::revise_members(c, &room, grantee_ids).await?;
    broadcast_to_members(c, &room, true).await?;
    redirect_to_room(c, room.id)
}

/// `broadcast_create_room` / `broadcast_update_room`: the shared-room partial, rendered once, to
/// every member's own rooms stream.
async fn broadcast_to_members(c: &mut Ctx, room: &Room, update: bool) -> Result<()> {
    broadcast(c, room, update).await
}

pub async fn broadcast(c: &Ctx, room: &Room, update: bool) -> Result<()> {
    let (broadcasts, room) = (c.app().broadcasts.clone(), room.clone());
    c.app().db.read(move |conn| {
        if update { broadcasts.closed_room_update(conn, &room) }
        else { broadcasts.closed_room_create(conn, &room) }
    }).await.map_err(db_error)
}
