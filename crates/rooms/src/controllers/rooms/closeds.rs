//! `Rooms::ClosedsController` (reference/app/controllers/rooms/closeds_controller.rb). `index` is
//! RoomsController's; `destroy` is RoomsController's without `set_room`
//! (`super::destroy_without_room`).

use campfire_db::{Room, RoomType};
use campfire_kit::{Ctx, Result, StatusCode};

use super::{
    Scope, ensure_can_administer, ensure_permission_to_create_rooms, redirect_to_room,
    room_icon_param, room_name_param, set_room, user_ids_param,
};
use crate::app::AppCtx;
use crate::concerns::{Before, before_actions, require_current_user};
use crate::controllers::presenters::page::db_error;

pub async fn create(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    ensure_permission_to_create_rooms(c).await?;
    let name = room_name_param(c)?.flatten();
    let icon = room_icon_param(c)?.flatten();
    let grantee_ids = user_ids_param(c);
    // Rooms::Closed.create_for(room_params, users: grantees)
    let room = super::operations::create(
        c,
        RoomType::Closed,
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
    let room = set_room(c, Scope::WithoutDirects).await?;
    ensure_can_administer(c, &room)?;
    let name = room_name_param(c)?;
    let icon = room_icon_param(c)?;
    let grantee_ids = user_ids_param(c);
    // force_room_type, then `@room.update! room_params`
    let room = super::operations::update(c, room, name, icon, Some(RoomType::Closed)).await;
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
