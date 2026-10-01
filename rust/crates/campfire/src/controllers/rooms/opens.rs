//! `Rooms::OpensController` (reference/app/controllers/rooms/opens_controller.rb). `index` is
//! RoomsController's; `destroy` is RoomsController's without `set_room`
//! (`super::destroy_without_room`).

use campfire_db::{Room, RoomType, User};
use campfire_kit::{Ctx, Result, StatusCode};
use campfire_views::rooms::{FormRoom, OpenFormView, OpensEdit, OpensNew};

use super::{
    Scope, ensure_can_administer, ensure_permission_to_create_rooms, redirect_to_room,
    render_shared_room, room_icon_param, room_name_param, set_room,
};
use crate::app::AppCtx;
use crate::concerns::{self, Before, before_actions, require_current_user};
use crate::controllers::presenters::page::{self, db_error};
use crate::controllers::presenters::user_view;

/// `DEFAULT_ROOM_NAME`
const DEFAULT_ROOM_NAME: &str = "New room";

pub async fn show(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (room, _) = super::set_room_for_show(c, Scope::WithoutDirects).await?;
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
    // Rooms::Open.create_for(room_params, users: Current.user)
    let room = c
        .app()
        .db
        .write(move |tx| {
            Room::create_for_with_icon(
                tx,
                RoomType::Open,
                name.as_deref(),
                icon.as_deref(),
                user_id,
                &[user_id],
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
    let partials = render_shared_room(c, &room).await?;
    c.app().broadcasts.open_room_create(&room, &partials);
    redirect_to_room(c, room.id)
}

pub async fn edit(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let mut room = set_room(c, Scope::WithoutDirects).await?;
    room.room_type = RoomType::Open; // force_room_type
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

pub async fn update(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = set_room(c, Scope::WithoutDirects).await?;
    ensure_can_administer(c, &room)?;
    let name = room_name_param(c)?;
    let icon = room_icon_param(c)?;
    let draft = (
        room.id,
        name.clone().unwrap_or_else(|| room.name.clone()),
        icon.clone().unwrap_or_else(|| room.icon_name.clone()),
    );
    // force_room_type, then `@room.update! room_params` saves the name and the new type.
    let room = c
        .app()
        .db
        .write(move |tx| {
            let mut room = room;
            room.update_with_icon(
                tx,
                name.as_ref().map(|name| name.as_deref()),
                Some(RoomType::Open),
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
    let partials = render_shared_room(c, &room).await?;
    let header = super::render_shared_header(c, &room).await?;
    c.app()
        .broadcasts
        .open_room_update(&room, &partials, Some(&header));
    redirect_to_room(c, room.id)
}

async fn render_form(c: &mut Ctx, room: FormRoom, status: StatusCode) -> Result {
    let can_administer = if let Some(id) = room.id {
        let creator_id = c
            .app()
            .db
            .read(move |conn| Ok(Room::find(conn, id)?.creator_id))
            .await
            .map_err(db_error)?;
        require_current_user(c)?.can_administer(Some(creator_id), false)
    } else {
        true
    };
    let form = OpenFormView {
        room,
        can_administer,
        users: active_users(c).await?,
    };
    if form.room.id.is_some() {
        let id = form.room.id.unwrap();
        let viewer = require_current_user(c)?.clone();
        let github = c.app().db.read(move |conn| {
            let room = Room::find(conn, id)?;
            crate::controllers::presenters::github::subscription_section(conn, &room, &viewer)
        }).await.map_err(db_error)?;
        page::framed_page!(c, status, |ctx| OpensEdit { ctx, form: &form, github: github.clone() }).await
    } else {
        page::framed_page!(c, status, |ctx| OpensNew { ctx, form: &form }).await
    }
}

/// `User.active.ordered`
pub(super) async fn active_users(c: &Ctx) -> Result<Vec<campfire_views::messages::UserView>> {
    let secrets = c.app().secrets.clone();
    c.app()
        .db
        .read(move |conn| {
            Ok(User::active_ordered(conn)?
                .iter()
                .map(|user| user_view(&secrets, user))
                .collect())
        })
        .await
        .map_err(db_error)
}
