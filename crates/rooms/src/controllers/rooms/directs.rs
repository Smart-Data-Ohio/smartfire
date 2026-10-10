//! `app/controllers/rooms/directs_controller.rb`: active capped selection and group writes.
//! Group notes and directory events come from WS8a; templates consume per-viewer facts.

use campfire_db::{Room, User};
use campfire_kit::{Ctx, Param, Redirect, Result, StatusCode};

use super::{Scope, audit_room, destroy_room, redirect_to_room, set_room};
use crate::app::AppCtx;
use crate::concerns::{Before, before_actions, require_current_user};
use crate::controllers::presenters::page::db_error;

pub async fn create(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let user_id = require_current_user(c)?.id;
    let ids = selected_user_ids(c, Some(user_id));
    let result = c
        .app()
        .db
        .write(move |tx| {
            let users = active_user_ids(tx.conn(), &ids)?;
            if users.len() > campfire_db::models::direct_room::MAX_MEMBERS {
                return Ok(None);
            }
            let created = Room::find_direct_for(tx.conn(), &users)?.is_none();
            let room = Room::find_or_create_direct_for(tx, &users, user_id)?;
            Ok(Some((room, created)))
        })
        .await
        .map_err(db_error)?;
    let Some((room, created)) = result else {
        let location = c.url_for(&campfire_routes::new_rooms_direct());
        return c.redirect_to_with(
            &location,
            Redirect {
                alert: Some(capacity_alert()),
                ..Redirect::default()
            },
        );
    };
    if created {
        audit_room(
            c,
            &room,
            "room.create",
            serde_json::json!({"name":room.name}),
        )
        .await?;
        broadcast_create_room(c, &room).await?;
    }
    if c.param("start_huddle").is_some_and(Param::is_present) {
        c.redirect_to(&c.url_for(&format!("{}?huddle=start", campfire_routes::room(room.id))))
    } else {
        redirect_to_room(c, room.id)
    }
}

pub async fn update(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = set_room(c, Scope::Directs).await?;
    let user = require_current_user(c)?.id;
    let permitted = c
        .params
        .require("room")?
        .permit(&campfire_kit::permit_keys(&["name", "icon_name"]));
    let name = permitted
        .get("name")
        .and_then(Param::to_s)
        .unwrap_or_default();
    let mut updated = room.clone();
    match c
        .app()
        .db
        .write(move |tx| updated.rename_direct(tx, &name, user))
        .await
    {
        Ok(()) => redirect_edit(c, room.id, Some("Group renamed.".into()), None),
        Err(campfire_db::Error::Other(kind)) if kind == "NotAGroup" => redirect_edit(
            c,
            room.id,
            None,
            Some("Only group direct messages can be renamed.".into()),
        ),
        Err(campfire_db::Error::RecordInvalid(_errors)) => { Ok(c.head(StatusCode::UNPROCESSABLE_ENTITY)) }
        Err(error) => Err(db_error(error)),
    }
}

pub async fn add_members(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = set_room(c, Scope::Directs).await?;
    let ids = selected_user_ids(c, None);
    let user = require_current_user(c)?.id;
    let updated = room.clone();
    let result = c
        .app()
        .db
        .write(move |tx| {
            let users = active_user_ids(tx.conn(), &ids)?;
            let added = updated.add_direct_members(tx, &users, user)?;
            let names = added
                .iter()
                .map(|id| User::find(tx.conn(), *id).map(|u| u.name))
                .collect::<campfire_db::Result<Vec<_>>>()?;
            Ok(names)
        })
        .await;
    match result {
        Ok(names) if names.is_empty() => redirect_edit(
            c,
            room.id,
            None,
            Some("Select at least one new member to add.".into()),
        ),
        Ok(names) => {
            audit_room(
                c,
                &room,
                "room.membership.change",
                serde_json::json!({"granted":names}),
            )
            .await?;
            redirect_edit(
                c,
                room.id,
                Some(format!("Added {} to the group.", sentence(&names))),
                None,
            )
        }
        Err(campfire_db::Error::Other(kind)) if kind == "NotAGroup" => redirect_edit(
            c,
            room.id,
            None,
            Some("Only group direct messages can add members.".into()),
        ),
        Err(campfire_db::Error::Other(kind)) if kind == "OverCapacity" => {
            redirect_edit(c, room.id, None, Some(capacity_alert()))
        }
        Err(error) => Err(db_error(error)),
    }
}

pub async fn leave(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = set_room(c, Scope::Directs).await?;
    super::leave_room(c, room).await
}

// Array(params.fetch(:user_ids, [])).first(MAX_MEMBERS), capped before any user query.
fn selected_user_ids(c: &Ctx, actor: Option<i64>) -> Vec<i64> {
    let mut values = match c.param("user_ids") {
        Some(Param::Array(values)) => values
            .iter()
            .take(campfire_db::models::direct_room::MAX_MEMBERS)
            .cloned()
            .collect::<Vec<_>>(),
        Some(Param::Null) | None => Vec::new(),
        Some(value) => vec![value.clone()],
    };
    // Including Current.user happens before User.where, so a nested sole operand
    // becomes an IN operand instead of being recursively unwrapped on creation.
    if let Some(actor) = actor {
        values.push(Param::Number(actor.into()));
    }
    super::user_ids_from_param(&Param::Array(values))
}
fn active_user_ids(conn: &campfire_db::Connection, ids: &[i64]) -> campfire_db::Result<Vec<i64>> {
    Ok(User::where_ids(conn, ids)?
        .into_iter()
        .filter(User::is_active)
        .map(|u| u.id)
        .collect())
}
fn capacity_alert() -> String {
    format!(
        "Group direct messages hold at most {} people.",
        campfire_db::models::direct_room::MAX_MEMBERS
    )
}
fn redirect_edit(c: &mut Ctx, id: i64, notice: Option<String>, alert: Option<String>) -> Result {
    c.redirect_to_with(
        &c.url_for(&campfire_routes::edit_rooms_direct(id)),
        Redirect {
            notice,
            alert,
            ..Redirect::default()
        },
    )
}
fn sentence(names: &[String]) -> String {
    match names.len() {
        0 => String::new(),
        1 => names[0].clone(),
        2 => format!("{} and {}", names[0], names[1]),
        n => format!("{}, and {}", names[..n - 1].join(", "), names[n - 1]),
    }
}

pub async fn destroy(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = set_room(c, Scope::Directs).await?;
    ensure_can_delete(c, &room).await?;
    destroy_room(c, room).await
}

pub async fn ensure_can_delete(c: &Ctx, room: &Room) -> Result<()> {
    let id = room.id;
    let group = c
        .app()
        .db
        .read(move |conn| Room::find(conn, id)?.direct_group_capable(conn))
        .await
        .map_err(db_error)?;
    if group && !require_current_user(c)?.is_administrator() {
        return campfire_kit::halt(crate::concerns::head(StatusCode::FORBIDDEN));
    }
    Ok(())
}

/// `broadcast_create_room`: `users/sidebars/rooms/_direct` for each membership, to its user.
pub async fn broadcast_create_room(c: &Ctx, room: &Room) -> Result<()> {
    c.app().broadcasts.direct_room_create(room);
    Ok(())
}
