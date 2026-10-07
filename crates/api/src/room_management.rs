//! Facts and writes behind the classic room management forms.
use std::collections::BTreeMap;

use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_db::models::audit_log::{AuditLog, Context, NewAuditLog};
use campfire_db::{Account, CachedStatements, Membership, Room, RoomType, User};
use campfire_kit::{Ctx, Error, Param, Result, StatusCode};
use campfire_rooms::controllers::rooms::{
    self, boards, call_channels, closeds, directs, opens, operations,
};
use campfire_views::helpers::IconSource;
use campfire_web::concerns::require_current_user;
use campfire_web::controllers::presenters::{Presenter, accounts, page::db_error};

use crate::{
    dto,
    endpoints::{before_actions, body, now, set_room},
    error::{fail, validation},
};

endpoint!(new => new_form);
endpoint!(edit => edit_form);
endpoint!(create => create_room);
endpoint!(update => update_room);
endpoint!(destroy => destroy_room);
endpoint!(leave => leave_room);

fn room_type(kind: api::RoomKind) -> RoomType {
    match kind {
        api::RoomKind::Open => RoomType::Open,
        api::RoomKind::Closed => RoomType::Closed,
        api::RoomKind::Direct => RoomType::Direct,
        api::RoomKind::Voice => RoomType::Voice,
        api::RoomKind::Stage => RoomType::Stage,
        api::RoomKind::Board => RoomType::Board,
    }
}

fn query_kind(c: &mut Ctx) -> Result<api::RoomKind> {
    match c.param("type").and_then(Param::as_str) {
        Some("open") => Ok(api::RoomKind::Open),
        Some("closed") => Ok(api::RoomKind::Closed),
        Some("direct") => Ok(api::RoomKind::Direct),
        Some("voice") => Ok(api::RoomKind::Voice),
        Some("stage") => Ok(api::RoomKind::Stage),
        Some("board") => Ok(api::RoomKind::Board),
        _ => Err(fail(c, validation("type", "must be a supported room type"))),
    }
}

async fn new_form(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let kind = query_kind(c)?;
    if kind != api::RoomKind::Direct {
        rooms::ensure_permission_to_create_rooms(c).await?;
    }
    let form = form(c, kind, None).await?;
    c.json(StatusCode::OK, &form)
}

async fn edit_form(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (_, room) = set_room(c).await?;
    let form = form(c, dto::room_kind(room.room_type), Some(room)).await?;
    c.json(StatusCode::OK, &form)
}

async fn form(c: &Ctx, kind: api::RoomKind, room: Option<Room>) -> Result<api::RoomForm> {
    let viewer = require_current_user(c)?.clone();
    let secrets = c.app().secrets.clone();
    let now = now(c);
    c.app()
        .db
        .read(move |conn| {
            let restricted = Account::first(conn)?
                .is_some_and(|a| a.settings().restrict_room_creation_to_administrators());
            let can_create = viewer.is_administrator() || !restricted;
            let allowed_types = if can_create {
                vec![
                    api::RoomKind::Open,
                    api::RoomKind::Closed,
                    api::RoomKind::Direct,
                    api::RoomKind::Voice,
                    api::RoomKind::Stage,
                    api::RoomKind::Board,
                ]
            } else {
                vec![api::RoomKind::Direct]
            };
            let active = User::active_ordered(conn)?;
            let member_ids = room
                .as_ref()
                .map(|r| r.user_ids(conn))
                .transpose()?
                .unwrap_or_default();
            let group_capable = room
                .as_ref()
                .map(|r| r.direct_group_capable(conn))
                .transpose()?
                .unwrap_or(false);
            let can_administer = room
                .as_ref()
                .is_none_or(|r| viewer.can_administer(Some(r.creator_id), false));
            let can_submit = if kind == api::RoomKind::Direct {
                room.is_none() || group_capable
            } else {
                can_administer
            };
            let can_delete = room.is_some()
                && if kind == api::RoomKind::Direct {
                    !group_capable || viewer.is_administrator()
                } else {
                    can_administer
                };
            let can_leave = room.is_some() && kind == api::RoomKind::Direct;
            let conversion_types = if room.is_some()
                && can_administer
                && matches!(kind, api::RoomKind::Open | api::RoomKind::Closed)
            {
                vec![api::RoomKind::Open, api::RoomKind::Closed]
            } else {
                Vec::new()
            };
            let mut candidate_ids: Vec<i64> = active
                .iter()
                .filter(|u| {
                    kind != api::RoomKind::Direct
                        || if room.is_some() {
                            !member_ids.contains(&u.id)
                        } else {
                            u.id != viewer.id
                        }
                })
                .map(|u| u.id)
                .collect();
            if room.is_none() && kind == api::RoomKind::Direct {
                let stars = viewer.starred_ids_among(conn, &candidate_ids)?;
                candidate_ids.sort_by_key(|id| !stars.contains(id));
            }
            let user_ids = if kind == api::RoomKind::Open {
                active.iter().map(|u| u.id).collect()
            } else if room.is_some() {
                active
                    .iter()
                    .filter(|u| member_ids.contains(&u.id))
                    .map(|u| u.id)
                    .collect()
            } else if kind == api::RoomKind::Direct {
                Vec::new()
            } else {
                vec![viewer.id]
            };
            let display_member_ids = if kind == api::RoomKind::Direct && room.is_some() {
                member_ids
                    .iter()
                    .copied()
                    .filter(|id| member_ids.len() == 1 || *id != viewer.id)
                    .collect()
            } else {
                Vec::new()
            };
            let (name, icon_name, display_name) = match &room {
                Some(room) => (
                    room.name.clone(),
                    room.icon_name.clone(),
                    accounts::room_display_name(conn, room, &viewer)?,
                ),
                None => {
                    let name = match kind {
                        api::RoomKind::Voice => "New voice channel",
                        api::RoomKind::Stage => "New stage channel",
                        api::RoomKind::Board => "New board",
                        api::RoomKind::Direct => "",
                        _ => "New room",
                    };
                    (
                        (kind != api::RoomKind::Direct).then(|| name.to_string()),
                        None,
                        name.to_string(),
                    )
                }
            };
            let stage_roles = if kind == api::RoomKind::Stage {
                if let Some(room) = &room {
                    Membership::for_room(conn, room.id)?
                        .into_iter()
                        .filter_map(|m| {
                            m.stage_role.map(|role| api::RoomFormStageRole {
                                user_id: m.user_id,
                                role: match role {
                                    campfire_db::StageRole::Host => api::StageRole::Host,
                                    campfire_db::StageRole::Speaker => api::StageRole::Speaker,
                                    campfire_db::StageRole::Listener => api::StageRole::Listener,
                                },
                            })
                        })
                        .collect()
                } else {
                    vec![api::RoomFormStageRole {
                        user_id: viewer.id,
                        role: api::StageRole::Host,
                    }]
                }
            } else {
                Vec::new()
            };
            let users = dto::users(
                conn,
                &secrets,
                candidate_ids
                    .iter()
                    .chain(&member_ids)
                    .chain(&user_ids)
                    .copied(),
                now,
            )?;
            Ok(api::RoomForm {
                kind,
                room_id: room.as_ref().map(|r| r.id),
                name,
                icon_name,
                display_name,
                user_ids,
                member_ids,
                candidate_ids,
                display_member_ids,
                users,
                allowed_types,
                conversion_types,
                can_submit,
                can_delete,
                can_leave,
                group_capable,
                default_involvement: if kind == api::RoomKind::Direct {
                    api::Involvement::Everything
                } else {
                    api::Involvement::Mentions
                },
                stage_roles,
            })
        })
        .await
        .map_err(db_error)
}

fn create_fields(
    input: api::CreateRoom,
) -> (api::RoomKind, Option<String>, Option<String>, Vec<i64>) {
    match input {
        api::CreateRoom::Open {
            name, icon_name, ..
        } => (api::RoomKind::Open, name, icon_name, Vec::new()),
        api::CreateRoom::Closed {
            name,
            icon_name,
            user_ids,
            ..
        } => (api::RoomKind::Closed, name, icon_name, user_ids),
        api::CreateRoom::Voice {
            name,
            icon_name,
            user_ids,
            ..
        } => (api::RoomKind::Voice, name, icon_name, user_ids),
        api::CreateRoom::Stage {
            name,
            icon_name,
            user_ids,
            ..
        } => (api::RoomKind::Stage, name, icon_name, user_ids),
        api::CreateRoom::Board {
            name,
            icon_name,
            user_ids,
            ..
        } => (api::RoomKind::Board, name, icon_name, user_ids),
    }
}

type UpdateFields = (
    api::RoomKind,
    Option<Option<String>>,
    Option<Option<String>>,
    Vec<i64>,
);

fn update_fields(input: api::UpdateRoom) -> UpdateFields {
    match input {
        api::UpdateRoom::Open { name, icon_name } => {
            (api::RoomKind::Open, name, icon_name, Vec::new())
        }
        api::UpdateRoom::Closed {
            name,
            icon_name,
            user_ids,
        } => (api::RoomKind::Closed, name, icon_name, user_ids),
        api::UpdateRoom::Voice {
            name,
            icon_name,
            user_ids,
        } => (api::RoomKind::Voice, name, icon_name, user_ids),
        api::UpdateRoom::Stage {
            name,
            icon_name,
            user_ids,
        } => (api::RoomKind::Stage, name, icon_name, user_ids),
        api::UpdateRoom::Board {
            name,
            icon_name,
            user_ids,
        } => (api::RoomKind::Board, name, icon_name, user_ids),
    }
}

// Call forms use String#empty? after normalization rather than Active Support presence.
fn call_icon(icon: Option<String>) -> Option<String> {
    let value = icon.unwrap_or_default();
    let value = campfire_richtext::ruby::strip(&value).trim_matches(':');
    let value = campfire_richtext::ruby::strip(value).to_lowercase();
    (!value.is_empty()).then_some(value)
}

fn call_invalid(c: &mut Ctx, messages: Vec<String>) -> Error {
    let field = if messages
        .iter()
        .all(|m| m == "Icon name is not a known icon")
    {
        "iconName"
    } else {
        "userIds"
    };
    let fields = BTreeMap::from([(
        field.into(),
        messages
            .iter()
            .map(|m| m.strip_prefix("Icon name ").unwrap_or(m).to_string())
            .collect(),
    )]);
    fail(
        c,
        api::ApiError::Validation {
            message: messages.join(", "),
            fields,
        },
    )
}

async fn create_room(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    rooms::ensure_permission_to_create_rooms(c).await?;
    let input: api::CreateRoom = body(c).await?;
    let key = input.client_room_id().trim().to_string();
    if key.is_empty() {
        return Err(fail(c, validation("clientRoomId", "can't be blank")));
    }
    let creator = require_current_user(c)?.id;
    let lookup = key.clone();
    let duplicate = c
        .app()
        .db
        .read(move |conn| Room::find_by_creation_key(conn, creator, &lookup))
        .await
        .map_err(db_error)?;
    if let Some(room) = duplicate {
        return c.json(StatusCode::OK, &mutation(c, room).await?);
    }
    #[cfg(feature = "test-support")]
    crate::test_hooks::after_duplicate_check(&key).await;
    let (kind, name, icon, ids) = create_fields(input);
    let app = c.app().clone();
    let audit = Context {
        actor: Some(require_current_user(c)?.into()),
        ip_address: Some(c.request.remote_ip()?.to_string()),
        user_agent: c.request.user_agent().map(str::to_string),
    };
    let lookup = key.clone();
    let created = c
        .app()
        .db
        .write(move |tx| {
            // The immediate write transaction repeats the lookup before any grants or callbacks.
            if let Some(room) = Room::find_by_creation_key(tx.conn(), creator, &key)? {
                return Ok((room, false));
            }
            let grantees = if kind == api::RoomKind::Open {
                vec![creator]
            } else {
                User::where_ids(tx.conn(), &ids)?
                    .into_iter()
                    .map(|user| user.id)
                    .collect()
            };
            let room = if matches!(kind, api::RoomKind::Voice | api::RoomKind::Stage) {
                let icon = call_icon(icon);
                if icon.as_deref().is_some_and(|name| {
                    Presenter::new(tx.conn(), &app, None)
                        .resolve_avatar_icon(name)
                        .is_none()
                }) {
                    let mut errors = campfire_db::Errors::default();
                    errors.add("icon_name", "is not a known icon");
                    return Err(campfire_db::Error::RecordInvalid(errors));
                }
                let mut room =
                    Room::create_for(tx, room_type(kind), name.as_deref(), creator, &grantees)?;
                if icon.is_some() {
                    tx.conn().execute_cached(
                        "UPDATE rooms SET icon_name=? WHERE id=?",
                        rusqlite::params![icon, room.id],
                    )?;
                    room.reload(tx.conn())?;
                }
                room
            } else {
                Room::create_for_with_icon(
                    tx,
                    room_type(kind),
                    name.as_deref(),
                    icon.as_deref(),
                    creator,
                    &grantees,
                    campfire_web::rich_text::room_icon_resolves,
                )?
            };
            tx.conn().execute_cached(
                "UPDATE rooms SET client_room_id=? WHERE id=?",
                rusqlite::params![key, room.id],
            )?;
            AuditLog::record(
                tx,
                NewAuditLog {
                    action: "room.create".into(),
                    target: Some((&room).into()),
                    changes: Some(serde_json::json!({"name":room.name})),
                    ..Default::default()
                },
                &audit,
            )?;
            Ok((room, true))
        })
        .await;
    let (room, created) = match created {
        // A unique-index conflict rolls back the whole write, including grants and callbacks.
        Err(error) if error.is_record_not_unique() => {
            let duplicate = c
                .app()
                .db
                .read(move |conn| Room::find_by_creation_key(conn, creator, &lookup))
                .await
                .map_err(db_error)?;
            match duplicate {
                Some(room) => (room, false),
                None => return Err(db_error(error)),
            }
        }
        result => result.map_err(db_error)?,
    };
    if created {
        broadcast(c, &room, false).await?;
    }
    let result = mutation(c, room).await?;
    c.json(
        if created {
            StatusCode::CREATED
        } else {
            StatusCode::OK
        },
        &result,
    )
}

async fn update_room(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (_, room) = set_room(c).await?;
    if room.direct() {
        return Err(Error::NotFound);
    }
    rooms::ensure_can_administer(c, &room)?;
    let input = body(c).await?;
    let (kind, name, icon, ids) = update_fields(input);
    let target = room_type(kind);
    if room.room_type != target
        && !(matches!(room.room_type, RoomType::Open | RoomType::Closed)
            && matches!(target, RoomType::Open | RoomType::Closed))
    {
        return Err(fail(
            c,
            validation("type", "cannot convert this room to that type"),
        ));
    }
    let room = if matches!(kind, api::RoomKind::Voice | api::RoomKind::Stage) {
        let has_remaining_ids = !ids.is_empty();
        match call_channels::update_room(
            c,
            room,
            name,
            icon.map(call_icon),
            ids,
            has_remaining_ids,
            false,
        )
        .await?
        {
            Ok(room) => room,
            Err((_, messages)) => return Err(call_invalid(c, messages)),
        }
    } else {
        let target = (kind != api::RoomKind::Board).then_some(target);
        let room = operations::update(c, room, name, icon, target)
            .await
            .map_err(db_error)?;
        if kind != api::RoomKind::Open {
            operations::revise_members(c, &room, ids).await?;
        }
        room
    };
    broadcast(c, &room, true).await?;
    let result = mutation(c, room).await?;
    c.json(StatusCode::OK, &result)
}

async fn broadcast(c: &Ctx, room: &Room, update: bool) -> Result<()> {
    match room.room_type {
        RoomType::Open => opens::broadcast(c, room, update).await,
        RoomType::Closed => closeds::broadcast(c, room, update).await,
        RoomType::Board => boards::broadcast(c, room, update).await,
        RoomType::Voice | RoomType::Stage => call_channels::broadcast(c, room, update).await,
        RoomType::Direct => unreachable!("direct management uses its existing API"),
    }
}

async fn mutation(c: &Ctx, room: Room) -> Result<api::RoomMutation> {
    let viewer = require_current_user(c)?.clone();
    let secrets = c.app().secrets.clone();
    let now = now(c);
    c.app()
        .db
        .read(move |conn| {
            let membership = Membership::find_by_room_and_user(conn, room.id, viewer.id)?;
            let (detail, row) = match membership {
                Some(membership) if !room.deleted() => (
                    Some(dto::room_detail(
                        conn,
                        &secrets,
                        &viewer,
                        &room,
                        &membership,
                        now,
                    )?),
                    dto::sidebar_row(conn, &room, &membership)?,
                ),
                _ => (None, None),
            };
            Ok(api::RoomMutation {
                room: dto::room(&room),
                detail,
                row,
            })
        })
        .await
        .map_err(db_error)
}

async fn destroy_room(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (_, room) = set_room(c).await?;
    if room.direct() {
        directs::ensure_can_delete(c, &room).await?;
    } else {
        rooms::ensure_can_administer(c, &room)?;
    }
    rooms::destroy_operation(c, &room).await?;
    c.json(
        StatusCode::OK,
        &api::RoomRemoved {
            room_id: room.id,
            deleted: true,
        },
    )
}

async fn leave_room(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (_, room) = set_room(c).await?;
    if !room.direct() {
        return Err(Error::NotFound);
    }
    let deleted = rooms::leave_operation(c, &room).await?;
    c.json(
        StatusCode::OK,
        &api::RoomLeft {
            room_id: room.id,
            deleted,
        },
    )
}
