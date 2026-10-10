//! `Rooms::OpensController` (reference/app/controllers/rooms/opens_controller.rb). `index` is
//! RoomsController's; `destroy` is RoomsController's without `set_room`
//! (`super::destroy_without_room`).

use campfire_db::{Room, RoomType};
use campfire_kit::{Ctx, Result, StatusCode};

use super::{
    Scope, ensure_can_administer, ensure_permission_to_create_rooms, redirect_to_room,
    room_icon_param, room_name_param, set_room,
};
use crate::app::AppCtx;
use crate::concerns::{Before, before_actions, require_current_user};
use crate::controllers::presenters::page::db_error;

pub async fn create(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    ensure_permission_to_create_rooms(c).await?;
    let name = room_name_param(c)?.flatten();
    let icon = room_icon_param(c)?.flatten();
    // Rooms::Open.create_for(room_params, users: Current.user)
    let room = super::operations::create(
        c,
        RoomType::Open,
        name,
        icon,
        require_current_user(c)?.id,
        Vec::new(),
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
    broadcast(c, &room, false).await?;
    redirect_to_room(c, room.id)
}

pub async fn update(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = set_room(c, Scope::WithoutDirects).await?;
    ensure_can_administer(c, &room)?;
    let name = room_name_param(c)?;
    let icon = room_icon_param(c)?;
    // force_room_type, then `@room.update! room_params` saves the name and the new type.
    let room = super::operations::update(c, room, name, icon, Some(RoomType::Open)).await;
    let room = match room {
        Ok(room) => room,
        Err(campfire_db::Error::RecordInvalid(_errors)) => {
            return Ok(c.head(StatusCode::UNPROCESSABLE_ENTITY));
        }
        Err(error) => return Err(db_error(error)),
    };
    broadcast(c, &room, true).await?;
    redirect_to_room(c, room.id)
}

pub async fn broadcast(c: &Ctx, room: &Room, update: bool) -> Result<()> {
    if update { c.app().broadcasts.open_room_update(room); }
    else { c.app().broadcasts.open_room_create(room); }
    Ok(())
}
