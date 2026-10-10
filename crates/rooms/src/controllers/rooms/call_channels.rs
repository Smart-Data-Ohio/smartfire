//! `Rooms::VoicesController` / `Rooms::StagesController`. Membership and call
//! callbacks remain in the existing domain; Stage edits use one immediate transaction.
use super::{
    Scope, ensure_can_administer, ensure_permission_to_create_rooms, existing_user_ids,
    redirect_to_room, room_name_param, set_room, user_ids_param,
};
use crate::controllers::presenters::{
    Presenter,
    page::db_error,
};
use crate::{
    app::AppCtx,
    concerns::{Before, before_actions, require_current_user},
};
use campfire_db::models::audit_log::{AuditLog, Context, NewAuditLog};
use campfire_db::{CachedStatements, Membership, Room, RoomType, StageRole, User};
use campfire_kit::{Ctx, Result, StatusCode};
use campfire_presentation::helpers::IconSource;
use serde_json::json;


type CreateOutcome = std::result::Result<Room, (Option<String>, Option<String>)>;
type UpdateOutcome = std::result::Result<Room, (Room, Vec<String>)>;

fn stage(c: &Ctx) -> bool {
    c.current::<crate::concerns::MatchedRoute>()
        .unwrap()
        .endpoint
        .starts_with("rooms/stages#")
}
fn kind(c: &Ctx) -> RoomType {
    if stage(c) {
        RoomType::Stage
    } else {
        RoomType::Voice
    }
}
fn scope(c: &Ctx) -> Scope {
    if stage(c) {
        Scope::Stages
    } else {
        Scope::Voices
    }
}
fn audit_context(c: &Ctx) -> Result<Context> {
    Ok(Context {
        actor: Some(require_current_user(c)?.into()),
        ip_address: Some(c.request.remote_ip()?.to_string()),
        user_agent: c.request.user_agent().map(str::to_string),
    })
}
fn icon_param(c: &Ctx) -> Result<Option<Option<String>>> {
    let attrs = c
        .params
        .require("room")?
        .permit(&campfire_kit::permit_keys(&["icon_name"]));
    Ok(attrs.get("icon_name").map(|p| {
        let value = p.to_s().unwrap_or_default();
        let value = campfire_richtext::ruby::strip(&value).trim_matches(':');
        let value = campfire_richtext::ruby::strip(value).to_lowercase();
        (!value.is_empty()).then_some(value)
    }))
}
fn valid_icon(conn: &campfire_db::Connection, app: &crate::app::App, icon: Option<&str>) -> bool {
    icon.is_none_or(|name| {
        Presenter::new(conn, app, None)
            .resolve_avatar_icon(name)
            .is_some()
    })
}

pub async fn create(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    ensure_permission_to_create_rooms(c).await?;
    let name = room_name_param(c)?.flatten();
    let icon = icon_param(c)?.flatten();
    let grantees = user_ids_param(c);
    let kind = kind(c);
    let result = create_room(c, kind, name, icon, grantees).await?;
    let room = match result {
        Ok(room) => room,
        Err((_name, _icon_name)) => {
            return Ok(c.head(StatusCode::UNPROCESSABLE_ENTITY));
        }
    };
    broadcast(c, &room, false).await?;
    redirect_to_room(c, room.id)
}


pub async fn create_room(
    c: &Ctx,
    kind: RoomType,
    name: Option<String>,
    icon: Option<String>,
    grantees: Vec<i64>,
) -> Result<CreateOutcome> {
    let creator = require_current_user(c)?.id;
    let app = c.app().clone();
    let audit = audit_context(c)?;
    c.app()
        .db
        .write(move |tx| {
            if !valid_icon(tx.conn(), &app, icon.as_deref()) {
                return Ok(Err((name, icon)));
            }
            let ids = existing_user_ids(tx.conn(), &grantees)?;
            let mut room = Room::create_for(tx, kind, name.as_deref(), creator, &ids)?;
            if icon.is_some() {
                tx.conn().execute_cached(
                    "UPDATE rooms SET icon_name=? WHERE id=?",
                    rusqlite::params![icon, room.id],
                )?;
                room.reload(tx.conn())?;
            }
            AuditLog::record(
                tx,
                NewAuditLog {
                    action: "room.create".into(),
                    target: Some((&room).into()),
                    changes: Some(json!({"name":room.name})),
                    ..Default::default()
                },
                &audit,
            )?;
            Ok(Ok(room))
        })
        .await
        .map_err(db_error)
}

pub async fn update(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = set_room(c, scope(c)).await?;
    ensure_can_administer(c, &room)?;
    let name = room_name_param(c)?;
    let icon = icon_param(c)?;
    let ids = user_ids_param(c);
    // Rails maps a present blank id to zero for its host guard, while its
    // single-NULL `where.not(id:)` selects no revokees. A missing list is empty.
    let values = c.param("user_ids").and_then(campfire_kit::Param::as_array);
    let has_remaining_ids = values.is_some_and(|v| !v.is_empty()) || !ids.is_empty();
    let blank_only = values.is_some_and(|v| v.len() == 1 && v[0].as_str() == Some(""));
    let result = update_room(c, room, name, icon, ids, has_remaining_ids, blank_only).await?;
    match result {
        Ok(room) => {
            broadcast(c, &room, true).await?;
            redirect_to_room(c, room.id)
        }
        Err((_room, _errors)) => Ok(c.head(StatusCode::UNPROCESSABLE_ENTITY)),
    }
}

pub async fn update_room(
    c: &Ctx,
    room: Room,
    name: Option<Option<String>>,
    icon: Option<Option<String>>,
    ids: Vec<i64>,
    has_remaining_ids: bool,
    blank_only: bool,
) -> Result<UpdateOutcome> {
    update_room_with_topic(
        c,
        room,
        super::operations::RoomChanges {
            name,
            icon,
            topic: None,
        },
        ids,
        has_remaining_ids,
        blank_only,
    )
    .await
}

pub async fn update_room_with_topic(
    c: &Ctx,
    room: Room,
    changes: super::operations::RoomChanges,
    ids: Vec<i64>,
    has_remaining_ids: bool,
    blank_only: bool,
) -> Result<UpdateOutcome> {
    let super::operations::RoomChanges { name, icon, topic } = changes;
    let app = c.app().clone();
    let audit = audit_context(c)?;
    c.app().db.write(move |tx| {
        let mut room=Room::find(tx.conn(),room.id)?;
        // Re-read the hosts under the same immediate transaction as the revision.
        if room.stage() && has_remaining_ids {
            let hosts:Vec<_>=Membership::for_room(tx.conn(),room.id)?.into_iter().filter(|m|m.stage_role==Some(StageRole::Host)).collect();
            if !hosts.iter().any(|m|ids.contains(&m.user_id)) && let Some(removed)=hosts.iter().find(|m|!ids.contains(&m.user_id)) {
                let user=User::find(tx.conn(),removed.user_id)?;
                return Ok(Err((room,vec![format!("Promote another host before removing {}",user.name)])));
            }
        }
        let preview_icon=icon.as_ref().unwrap_or(&room.icon_name);
        if preview_icon != &room.icon_name && !valid_icon(tx.conn(),&app,preview_icon.as_deref()) {
            if let Some(name)=name {room.name=name;}
            room.icon_name=preview_icon.clone();
            return Ok(Err((room,vec!["Icon name is not a known icon".into()])));
        }
        room.update(tx,name.as_ref().map(|n|n.as_deref()),None)?;
        if let Some(icon)=icon && icon != room.icon_name {tx.conn().execute_cached("UPDATE rooms SET icon_name=?,updated_at=? WHERE id=?",rusqlite::params![icon,tx.now(),room.id])?;room.reload(tx.conn())?;}
        if let Some(topic)=topic { room.update_topic(tx,topic.as_deref())?; }
        let before=room.user_ids(tx.conn())?;
        let granted=existing_user_ids(tx.conn(),&ids)?;
        let revoked:Vec<_>=before.iter().filter(|id|!blank_only && !ids.contains(id)).copied().collect();
        room.revise(tx,&granted,&revoked)?;
        if room.stage() {tx.conn().execute_cached("UPDATE memberships SET stage_role='listener' WHERE room_id=? AND stage_role IS NULL",[room.id])?;}
        let after=room.user_ids(tx.conn())?;
        let added:Vec<_>=after.iter().filter(|id|!before.contains(id)).copied().collect();
        let removed:Vec<_>=before.iter().filter(|id|!after.contains(id)).copied().collect();
        if !added.is_empty() || !removed.is_empty() {
            let names=|ids:&[i64]| -> campfire_db::Result<Vec<String>> {Ok(User::where_ids(tx.conn(),ids)?.into_iter().map(|u|u.name).collect())};
            AuditLog::record(tx,NewAuditLog{action:"room.membership.change".into(),target:Some((&room).into()),changes:Some(json!({"granted":names(&added)?,"revoked":names(&removed)?})),..Default::default()},&audit)?;
        }
        Ok(Ok(room))
    }).await.map_err(db_error)
}

pub async fn broadcast(c: &Ctx, room: &Room, _update: bool) -> Result<()> {
    let (app, room) = (c.app().clone(), room.clone());
    c.app().db.read(move |conn| {
        for membership in room.memberships(conn)? {
            app.broadcasts.sync_membership_row(membership.id);
        }
        Ok(())
    }).await.map_err(db_error)
}

/// Generic `/rooms/:id` deletion of a voice/stage channel delegates to WS8a's
/// existing room-deletion seam so call grants and streams end synchronously.
pub async fn destroy(c: &mut Ctx, room: Room) -> Result {
    destroy_operation(c, &room).await?;
    let json_response = *c
        .respond_to(&[&campfire_kit::format::HTML, &campfire_kit::format::JSON])?
        == campfire_kit::format::JSON;
    if json_response {
        c.json(StatusCode::OK, &json!({"deleted":true,"room_id":room.id}))
    } else {
        let root = c.url_for(&campfire_routes::root());
        let notice = room
            .name
            .filter(|n| !campfire_richtext::ruby::is_blank(n))
            .map(|n| format!("Deleted #{n}"));
        c.redirect_to_with(
            &root,
            campfire_kit::Redirect {
                notice,
                ..Default::default()
            },
        )
    }
}

pub async fn destroy_operation(c: &Ctx, room: &Room) -> Result<()> {
    let audit = audit_context(c)?;
    let config = campfire_db::models::room_delete::HuddleConfig {
        api_secret: c.app().config.huddle.api_secret.clone(),
        admin_configured: c.app().config.huddle.admin_configured(),
    };
    let deleted = room.clone();
    c.app()
        .db
        .write(move |tx| campfire_db::models::room_delete::begin_destroy(tx, &deleted, &config))
        .await
        .map_err(db_error)?;
    // RoomsController#destroy audits after its marking transaction commits.
    // An unavailable audit sink must not resurrect a deleted call room.
    let deleted = room.clone();
    c.app()
        .db
        .write(move |tx| {
            AuditLog::record(
                tx,
                NewAuditLog {
                    action: "room.destroy".into(),
                    target: Some((&deleted).into()),
                    target_label: deleted.name.clone(),
                    changes: Some(json!({"name":deleted.name})),
                    ..Default::default()
                },
                &audit,
            )?;
            Ok(())
        })
        .await
        .map_err(db_error)?;
    c.app().broadcasts.room_remove(room);
    Ok(())
}
