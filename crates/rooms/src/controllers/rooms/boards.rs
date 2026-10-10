//! Rails Rooms::BoardsController: board-only scope, explicit memberships, and room audits.

use campfire_db::{Room, RoomType, User};
use campfire_kit::{Ctx, Result, StatusCode};
use campfire_views::rooms::{ClosedFormView, FormRoom};

use super::{
    Scope, ensure_can_administer, ensure_permission_to_create_rooms, redirect_to_room,
    room_icon_param, room_name_param, set_room, user_ids_param,
};
use crate::app::AppCtx;
use crate::concerns::{self, Before, before_actions, require_current_user};
use crate::controllers::presenters::page::{self, db_error};
use crate::controllers::presenters::user_view;

/// `DEFAULT_ROOM_NAME`
const DEFAULT_ROOM_NAME: &str = "New board";

pub async fn show(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = super::set_room(c, Scope::Boards).await?;
    concerns::remember_last_room_visited(c, room.id);
    redirect_to_room(c, room.id)
}

/// Same deletion as `RoomsController#destroy`: membership scope, the administer gate, then
/// the room's threads, posts and work rows go with the destroy job.
pub async fn destroy(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = set_room(c, Scope::Boards).await?;
    super::ensure_can_delete(c, &room).await?;
    super::destroy_room(c, room).await
}

pub async fn new(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    ensure_permission_to_create_rooms(c).await?;
    let room = super::form_room(
        c,
        None,
        Some(DEFAULT_ROOM_NAME.into()),
        None,
        Default::default(),
    )
    .await?;
    render_form(c, room, StatusCode::OK).await
}

pub async fn create(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    ensure_permission_to_create_rooms(c).await?;
    let name = room_name_param(c)?.flatten();
    let icon = room_icon_param(c)?.flatten();
    let draft = (name.clone(), icon.clone());
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
        Err(campfire_db::Error::RecordInvalid(errors)) => {
            let room = super::form_room(c, None, draft.0, draft.1, errors).await?;
            return render_form(c, room, StatusCode::UNPROCESSABLE_ENTITY).await;
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

pub async fn edit(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = set_room(c, Scope::Boards).await?;
    let form_room = super::form_room(
        c,
        Some(room.id),
        room.name,
        room.icon_name,
        Default::default(),
    )
    .await?;
    render_form(c, form_room, StatusCode::OK).await
}

async fn render_form(c: &mut Ctx, room: FormRoom, status: StatusCode) -> Result {
    let current_user = require_current_user(c)?.clone();
    let secrets = c.app().secrets.clone();
    let room_id = room.id;
    let (selected_users, unselected_users) = c
        .app()
        .db
        .read(move |conn| {
            let selected_ids = if let Some(id) = room_id {
                Room::find(conn, id)?.user_ids(conn)?
            } else {
                Vec::new()
            };
            let (selected, unselected): (Vec<User>, Vec<User>) = User::active_ordered(conn)?
                .into_iter()
                .partition(|user| selected_ids.contains(&user.id));
            let views = |users: Vec<User>| {
                users
                    .iter()
                    .map(|user| user_view(&secrets, user))
                    .collect::<Vec<_>>()
            };
            Ok((views(selected), views(unselected)))
        })
        .await
        .map_err(db_error)?;
    let can_administer = if let Some(id) = room.id {
        let creator_id = c
            .app()
            .db
            .read(move |conn| Ok(Room::find(conn, id)?.creator_id))
            .await
            .map_err(db_error)?;
        current_user.can_administer(Some(creator_id), false)
    } else {
        true
    };
    let form = ClosedFormView {
        room,
        can_administer,
        current_user_id: current_user.id,
        selected_users,
        unselected_users,
    };
    if let Some(id) = form.room.id {
        let viewer = require_current_user(c)?.clone();
        let github = c
            .app()
            .db
            .read(move |conn| {
                let room = Room::find(conn, id)?;
                crate::controllers::presenters::github::subscription_section(conn, &room, &viewer)
            })
            .await
            .map_err(db_error)?;
        page::framed_page!(c, status, |ctx| campfire_views::rooms::boards::Edit {
            ctx,
            form: &form,
            github: github.clone()
        })
        .await
    } else {
        page::framed_page!(c, status, |ctx| campfire_views::rooms::boards::New {
            ctx,
            form: &form
        })
        .await
    }
}

pub async fn update(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = set_room(c, Scope::Boards).await?;
    ensure_can_administer(c, &room)?;
    let name = room_name_param(c)?;
    let icon = room_icon_param(c)?;
    let draft = (
        room.id,
        name.clone().unwrap_or_else(|| room.name.clone()),
        icon.clone().unwrap_or_else(|| room.icon_name.clone()),
    );
    let grantee_ids = user_ids_param(c);
    // Board updates retain their existing STI type.
    let room = super::operations::update(c, room, name, icon, None).await;
    let room = match room {
        Ok(room) => room,
        Err(campfire_db::Error::RecordInvalid(errors)) => {
            let room = super::form_room(c, Some(draft.0), draft.1, draft.2, errors).await?;
            return render_form(c, room, StatusCode::UNPROCESSABLE_ENTITY).await;
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

pub(crate) async fn render_index(c: &mut Ctx, room: Room) -> Result {
    let text = |key: &str| {
        c.params
            .get(key)
            .map(|value| campfire_richtext::ruby::json_value_to_s(&value.to_json()))
            .unwrap_or_default()
    };
    let board_view = text("view") == "board";
    let status = text("status");
    let status = if matches!(status.as_str(), "open" | "done" | "all") {
        status
    } else {
        "open".into()
    };
    let owner = text("owner");
    let owner = if campfire_richtext::ruby::is_blank(&owner) {
        "anyone".into()
    } else {
        owner
    };
    let tag = campfire_richtext::ruby::strip(&text("tag")).to_string();
    let number = campfire_db::models::channel_thread::board_page_number(&text("page"));
    let viewer = require_current_user(c)?.clone();
    let listing = crate::controllers::messages::present(c, move |p| {
        crate::controllers::presenters::boards::listing(
            p,
            &room,
            &viewer,
            crate::controllers::presenters::boards::Filters {
                board_view,
                status,
                owner,
                tag,
                page: number,
            },
        )
    })
    .await?;
    page::framed_page!(c, StatusCode::OK, |ctx| {
        campfire_views::rooms::boards::Index {
            ctx,
            board: &listing,
        }
    })
    .await
}
