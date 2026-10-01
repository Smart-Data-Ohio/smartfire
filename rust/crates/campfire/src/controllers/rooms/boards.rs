//! Rails Rooms::BoardsController: board-only scope, explicit memberships, and room audits.

use campfire_db::{Room, RoomType, User};
use campfire_kit::{Ctx, Error, Result, StatusCode};
use campfire_views::rooms::{ClosedFormView, FormRoom};

use super::{
    Scope, ensure_can_administer, ensure_permission_to_create_rooms, existing_user_ids,
    redirect_to_room, render_shared_room, room_icon_param, room_name_param, set_room,
    user_ids_param,
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
    let user_id = require_current_user(c)?.id;
    let grantee_ids = user_ids_param(c);
    // Rooms::Board.create_for(room_params, users: grantees)
    let room = c
        .app()
        .db
        .write(move |tx| {
            let grantees = existing_user_ids(tx.conn(), &grantee_ids)?;
            Room::create_for_with_icon(
                tx,
                RoomType::Board,
                name.as_deref(),
                icon.as_deref(),
                user_id,
                &grantees,
                crate::rich_text::room_icon_resolves,
            )
        })
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
        let github = c.app().db.read(move |conn| {
            let room = Room::find(conn, id)?;
            crate::controllers::presenters::github::subscription_section(conn, &room, &viewer)
        }).await.map_err(db_error)?;
        page::framed_page!(c, status, |ctx| campfire_views::rooms::boards::Edit { ctx, form: &form, github: github.clone() }).await
    } else {
        page::framed_page!(c, status, |ctx| campfire_views::rooms::boards::New { ctx, form: &form }).await
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
    let room = c
        .app()
        .db
        .write(move |tx| {
            let mut room = room;
            room.update_with_icon(
                tx,
                name.as_ref().map(|name| name.as_deref()),
                None,
                icon.as_ref().map(|icon| icon.as_deref()),
                crate::rich_text::room_icon_resolves,
            )?;
            Ok(room)
        })
        .await;
    let room = match room {
        Ok(room) => room,
        Err(campfire_db::Error::RecordInvalid(errors)) => {
            let room = super::form_room(c, Some(draft.0), draft.1, draft.2, errors).await?;
            return render_form(c, room, StatusCode::UNPROCESSABLE_ENTITY).await;
        }
        Err(error) => return Err(db_error(error)),
    };
    // `@room.memberships.revise(granted: grantees, revoked: revokees)`
    let revised = room.clone();
    let changes = c
        .app()
        .db
        .write(move |tx| {
            let before = revised.user_ids(tx.conn())?;
            let granted = existing_user_ids(tx.conn(), &grantee_ids)?;
            let revoked: Vec<i64> = before
                .iter()
                .copied()
                .filter(|id| !grantee_ids.contains(id))
                .collect();
            revised.revise(tx, &granted, &revoked)?;
            let after = revised.user_ids(tx.conn())?;
            let granted: Vec<_> = after
                .into_iter()
                .filter(|id| !before.contains(id))
                .collect();
            if granted.is_empty() && revoked.is_empty() {
                return Ok(None);
            }
            let users = User::where_ids(
                tx.conn(),
                &granted.iter().chain(&revoked).copied().collect::<Vec<_>>(),
            )?;
            let names = |ids: &[i64]| {
                ids.iter()
                    .filter_map(|id| {
                        users
                            .iter()
                            .find(|user| user.id == *id)
                            .map(|user| user.name.clone())
                    })
                    .collect::<Vec<_>>()
            };
            Ok(Some(
                serde_json::json!({"granted":names(&granted),"revoked":names(&revoked)}),
            ))
        })
        .await
        .map_err(db_error)?;
    if let Some(changes) = changes {
        super::audit_room(c, &room, "room.membership.change", changes).await?;
    }
    broadcast_to_members(c, &room, true).await?;
    redirect_to_room(c, room.id)
}

/// `broadcast_create_room` / `broadcast_update_room`: the shared-room partial, rendered once, to
/// every member's own rooms stream.
async fn broadcast_to_members(c: &mut Ctx, room: &Room, update: bool) -> Result<()> {
    // The fork renders these partials through the request's lookup context. With no HTML
    // format available (including JSON and Turbo-only requests), Rails raises MissingTemplate
    // after the domain and audit commits, before publishing any controller row/header.
    let formats = c.formats()?;
    if !formats.contains(&&campfire_kit::format::HTML)
        && !formats.contains(&&campfire_kit::format::ALL)
    {
        return Err(Error::internal(anyhow::anyhow!(
            "Missing partial users/sidebars/rooms/board for requested format"
        )));
    }
    let partials = render_shared_room(c, room).await?;
    let header = if update {
        Some(super::render_shared_header(c, room).await?)
    } else {
        None
    };
    let (broadcasts, room) = (c.app().broadcasts.clone(), room.clone());
    c.app()
        .db
        .read(move |conn| {
            if update {
                broadcasts.closed_room_update(conn, &room, &partials, header.as_deref())
            } else {
                {
                use crate::channels::broadcasts::{Partials, Stream};
                let html = partials.shared_room(&room);
                for user_id in room.user_ids(conn)? { broadcasts.prepend(&Stream::user_rooms(user_id), "board_rooms", &html); }
                Ok(())
            }
            }
        })
        .await
        .map_err(db_error)
}

pub(crate) async fn render_index(c: &mut Ctx, room: Room) -> Result {
    let text = |key: &str| c.params.get(key).and_then(|value| value.to_s()).unwrap_or_default();
    let board_view = text("view") == "board";
    let status = text("status");
    let status = if matches!(status.as_str(), "open"|"done"|"all") {status} else {"open".into()};
    let owner = text("owner");
    let owner = if campfire_richtext::ruby::is_blank(&owner) {"anyone".into()} else {owner};
    let tag = campfire_richtext::ruby::strip(&text("tag")).to_string();
    let number = campfire_db::models::channel_thread::board_page_number(&text("page"));
    let viewer = require_current_user(c)?.clone();
    let listing = crate::controllers::messages::present(c, move |p| crate::controllers::presenters::boards::listing(p,&room,&viewer,board_view,status,owner,tag,number)).await?;
    page::framed_page!(c, StatusCode::OK, |ctx| campfire_views::rooms::boards::Index {ctx, board:&listing}).await
}
