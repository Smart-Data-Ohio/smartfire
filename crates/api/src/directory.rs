//! The S2 directory endpoints (`campfire_api_types`' `direct`, `panes` and `switcher` modules
//! document each one): direct messages, a room's members and files, stars and the quick
//! switcher. Each reuses the classic action's read or write and its broadcasts.

use std::collections::BTreeSet;

use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_db::models::direct_room::MAX_MEMBERS;
use campfire_db::models::room_members;
use campfire_db::models::workspace_presence_lease::Presence as LeasePresence;
use campfire_db::{Membership, Room, User, UserStar};
use campfire_kit::{Ctx, Error, Result, StatusCode};
use campfire_rooms::controllers::rooms::{audit_room, directs};
use campfire_web::concerns;
use campfire_web::controllers::presenters::page::db_error;

use crate::dto;
use crate::endpoints::{before_actions, body, now, set_room};
use crate::error::{fail, validation};

endpoint!(
    /// `GET /api/v1/directs/candidates`
    direct_candidates => index_direct_candidates
);
endpoint!(
    /// `POST /api/v1/directs`
    create_direct => post_direct
);
endpoint!(
    /// `POST /api/v1/directs/:room_id/members`
    add_direct_members => post_direct_members
);
endpoint!(
    /// `PATCH /api/v1/directs/:room_id`
    rename_direct => patch_direct
);
endpoint!(
    /// `GET /api/v1/rooms/:room_id/members`
    members => index_members
);
endpoint!(
    /// `PUT /api/v1/users/:user_id/star`
    star => put_star
);
endpoint!(
    /// `DELETE /api/v1/users/:user_id/star`
    unstar => delete_star
);
endpoint!(
    /// `GET /api/v1/rooms/:room_id/files`
    files => index_files
);
endpoint!(
    /// `GET /api/v1/switcher`
    switcher => show_switcher
);

// --- Direct messages ------------------------------------------------------------------------------

async fn index_direct_candidates(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let viewer = concerns::require_current_user(c)?.clone();
    let (secrets, now) = (c.app().secrets.clone(), now(c));
    let list = c
        .app()
        .db
        .read(move |conn| {
            let people: Vec<User> = User::active_ordered(conn)?
                .into_iter()
                .filter(|user| user.id != viewer.id)
                .collect();
            let ids: Vec<i64> = people.iter().map(|user| user.id).collect();
            let starred = viewer.starred_ids_among(conn, &ids)?;
            let agents: BTreeSet<i64> = campfire_db::Agent::for_users(conn, &ids)?
                .into_iter()
                .map(|agent| agent.user_id)
                .collect();
            let mut candidates: Vec<api::DirectCandidate> = people
                .iter()
                .map(|user| api::DirectCandidate {
                    user_id: user.id,
                    agent: agents.contains(&user.id),
                    starred: starred.contains(&user.id),
                })
                .collect();
            // `rooms/directs#new`: a stable partition keeps `User.ordered` within each half.
            candidates.sort_by_key(|candidate| !candidate.starred);
            Ok(api::DirectCandidateList {
                candidates,
                users: dto::users(conn, &secrets, ids, now)?,
            })
        })
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &list)
}

fn capacity(c: &mut Ctx) -> Error {
    fail(
        c,
        validation(
            "userIds",
            &format!(
                "would make more than {MAX_MEMBERS} people (a group holds at most {MAX_MEMBERS})"
            ),
        ),
    )
}

/// `rooms/directs#create`: the first [`MAX_MEMBERS`] ids and the viewer, active people only.
async fn post_direct(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let viewer = concerns::require_current_user(c)?.id;
    let input: api::CreateDirect = body(c).await?;
    let mut ids: Vec<i64> = input.user_ids.into_iter().take(MAX_MEMBERS).collect();
    ids.push(viewer);
    let result = c
        .app()
        .db
        .write(move |tx| {
            let users: Vec<i64> = User::where_ids(tx.conn(), &ids)?
                .into_iter()
                .filter(User::is_active)
                .map(|user| user.id)
                .collect();
            if users.len() > MAX_MEMBERS {
                return Ok(None);
            }
            let created = Room::find_direct_for(tx.conn(), &users)?.is_none();
            let room = Room::find_or_create_direct_for(tx, &users, viewer)?;
            Ok(Some((room, created)))
        })
        .await
        .map_err(db_error)?;
    let Some((room, created)) = result else {
        return Err(capacity(c));
    };
    if created {
        audit_room(
            c,
            &room,
            "room.create",
            serde_json::json!({"name": room.name}),
        )
        .await?;
        directs::broadcast_create_room(c, &room).await?;
    }
    let revision = crate::endpoints::now(c);
    let row = c
        .app()
        .db
        .read(move |conn| {
            let membership = Membership::find_by_room_and_user(conn, room.id, viewer)?
                .ok_or(campfire_db::Error::RecordNotFound("Membership"))?;
            dto::sidebar_row(conn, &room, &membership, revision)
        })
        .await
        .map_err(db_error)?
        .ok_or(Error::NotFound)?;
    let status = if created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    c.json(status, &row)
}

/// The direct room `:room_id` names, among the viewer's (`Current.user.rooms.directs`).
async fn direct_room(c: &mut Ctx) -> Result<Room> {
    let (_, room) = set_room(c).await?;
    if !room.direct() {
        return Err(Error::NotFound);
    }
    Ok(room)
}

/// A refused direct room change, as `rooms/directs`' alerts word it.
fn not_a_group(c: &mut Ctx, field: &str, action: &str) -> Error {
    fail(
        c,
        validation(
            field,
            &format!("can't be changed: only group direct messages can {action}"),
        ),
    )
}

async fn room_detail(c: &Ctx, room_id: i64) -> Result<api::RoomDetail> {
    let viewer = concerns::require_current_user(c)?.clone();
    let (secrets, now) = (c.app().secrets.clone(), now(c));
    c.app()
        .db
        .read(move |conn| {
            let room = Room::find(conn, room_id)?;
            let membership = Membership::find_by_room_and_user(conn, room_id, viewer.id)?
                .ok_or(campfire_db::Error::RecordNotFound("Membership"))?;
            dto::room_detail(conn, &secrets, &viewer, &room, &membership, now)
        })
        .await
        .map_err(db_error)
}

/// `rooms/directs#add_members`.
async fn post_direct_members(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let room = direct_room(c).await?;
    let input: api::AddDirectMembers = body(c).await?;
    let ids: Vec<i64> = input.user_ids.into_iter().take(MAX_MEMBERS).collect();
    let actor = concerns::require_current_user(c)?.id;
    let updated = room.clone();
    let result = c
        .app()
        .db
        .write(move |tx| {
            let users: Vec<i64> = User::where_ids(tx.conn(), &ids)?
                .into_iter()
                .filter(User::is_active)
                .map(|user| user.id)
                .collect();
            let added = updated.add_direct_members(tx, &users, actor)?;
            added
                .iter()
                .map(|id| User::find(tx.conn(), *id).map(|user| user.name))
                .collect::<campfire_db::Result<Vec<_>>>()
        })
        .await;
    match result {
        // Backend note 6: nobody new is the model's no-op, answered with the room as it is (no
        // audit, no event).
        Ok(names) if names.is_empty() => {
            let detail = room_detail(c, room.id).await?;
            c.json(StatusCode::OK, &detail)
        }
        Ok(names) => {
            audit_room(
                c,
                &room,
                "room.membership.change",
                serde_json::json!({"granted": names}),
            )
            .await?;
            let detail = room_detail(c, room.id).await?;
            c.json(StatusCode::OK, &detail)
        }
        Err(campfire_db::Error::Other(kind)) if kind == "NotAGroup" => {
            Err(not_a_group(c, "userIds", "add members"))
        }
        Err(campfire_db::Error::Other(kind)) if kind == "OverCapacity" => Err(capacity(c)),
        Err(error) => Err(db_error(error)),
    }
}

/// `rooms/directs#update`.
async fn patch_direct(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let room = direct_room(c).await?;
    let input: api::RenameDirect = body(c).await?;
    let name = input.name.unwrap_or_default();
    let actor = concerns::require_current_user(c)?.id;
    let mut updated = room.clone();
    match c
        .app()
        .db
        .write(move |tx| updated.rename_direct(tx, &name, actor))
        .await
    {
        Ok(()) => {
            let detail = room_detail(c, room.id).await?;
            c.json(StatusCode::OK, &detail)
        }
        Err(campfire_db::Error::Other(kind)) if kind == "NotAGroup" => {
            Err(not_a_group(c, "name", "be renamed"))
        }
        Err(error) => Err(db_error(error)),
    }
}

// --- Members and stars ----------------------------------------------------------------------------

/// `rooms/members#index`, as the contract types it.
async fn index_members(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (_, room) = set_room(c).await?;
    let viewer = concerns::require_current_user(c)?.id;
    let (secrets, now) = (c.app().secrets.clone(), now(c));
    let list = c
        .app()
        .db
        .read(move |conn| {
            let found = room_members::for_room(conn, room.id, viewer, now)?;
            let extras = dto::UserExtras::load(conn, found.iter().map(|member| &member.user))?;
            let mut members = Vec::with_capacity(found.len());
            let mut users = Vec::with_capacity(found.len());
            for member in found {
                let (presence, status_text) = match &member.agent {
                    Some(agent) => {
                        let live = agent.suspended_at.is_none() && agent.last_seen_at.is_some();
                        let status = agent
                            .working_presence_text(now)
                            .or_else(|| {
                                agent
                                    .status_note
                                    .as_deref()
                                    .filter(|note| !campfire_richtext::ruby::is_blank(note))
                            })
                            .map(str::to_owned)
                            .unwrap_or_else(|| {
                                let mut letters = agent.status.chars();
                                letters
                                    .next()
                                    .map(|first| {
                                        first.to_uppercase().to_string() + letters.as_str()
                                    })
                                    .unwrap_or_default()
                            });
                        (
                            if live {
                                api::Presence::Online
                            } else {
                                api::Presence::Offline
                            },
                            Some(status),
                        )
                    }
                    None => (
                        match member.settings.effective_presence(member.lease) {
                            LeasePresence::Online => api::Presence::Online,
                            LeasePresence::Idle => api::Presence::Idle,
                            LeasePresence::Dnd => api::Presence::Dnd,
                            LeasePresence::Offline => api::Presence::Offline,
                        },
                        member.settings.status_text_display(now),
                    ),
                };
                members.push(api::Member {
                    user_id: member.user.id,
                    presence,
                    status_text,
                    starred: member.starred,
                });
                users.push(dto::user(&member.settings, &secrets, now, &extras));
            }
            Ok(api::MemberList { members, users })
        })
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &list)
}

async fn put_star(c: &mut Ctx) -> Result {
    change_star(c, true).await
}

async fn delete_star(c: &mut Ctx) -> Result {
    change_star(c, false).await
}

/// `users/stars#create` and `#destroy`: the viewer's own private preference.
async fn change_star(c: &mut Ctx, starred: bool) -> Result {
    before_actions(c).await?;
    let Some(target_id) = c.param_str("user_id").and_then(concerns::cast_integer) else {
        return Err(Error::NotFound);
    };
    let exists = c
        .app()
        .db
        .read(move |conn| Ok(User::find_by_id(conn, target_id)?.is_some()))
        .await
        .map_err(db_error)?;
    if !exists {
        return Err(Error::NotFound);
    }
    let viewer = concerns::require_current_user(c)?.clone();
    if viewer.id == target_id {
        return Err(fail(c, validation("userId", "can't be yourself")));
    }
    if !viewer.is_active() || viewer.is_bot() {
        return Err(fail(
            c,
            api::ApiError::Forbidden {
                message: "Only people can star someone".into(),
            },
        ));
    }
    let viewer_id = viewer.id;
    let result = c
        .app()
        .db
        .write(move |tx| {
            if starred {
                UserStar::find_or_create(tx, viewer_id, target_id)?;
            } else {
                UserStar::remove(tx, viewer_id, target_id)?;
            }
            Ok(())
        })
        .await;
    // A star racing another for the same person is already there.
    if let Err(error) = result
        && !(starred
            && (error.is_record_not_unique()
                || matches!(error, campfire_db::Error::RecordInvalid(_))))
    {
        return Err(db_error(error));
    }
    c.json(
        StatusCode::OK,
        &api::StarState {
            user_id: target_id,
            starred,
        },
    )
}

// --- Files ----------------------------------------------------------------------------------------

/// `rooms/files#index`'s uploads, a page at a time.
async fn index_files(c: &mut Ctx) -> Result {
    use campfire_db::models::room_files;
    before_actions(c).await?;
    let (_, room) = set_room(c).await?;
    let file_type = room_files::file_type(c.param_str("type").unwrap_or_default()).to_string();
    let filename =
        campfire_richtext::ruby::strip(c.param_str("filename").unwrap_or_default()).to_string();
    let page = room_files::page(c.param_str("page").unwrap_or_default());
    let (app, now) = (c.app().clone(), now(c));
    let list = c
        .app()
        .db
        .read(move |conn| dto::room_files(conn, &app, room.id, &file_type, &filename, page, now))
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &list)
}

// --- Switcher -------------------------------------------------------------------------------------

/// `switchers#show`'s read model, as the contract types it.
async fn show_switcher(c: &mut Ctx) -> Result {
    use campfire_web::controllers::presenters::switcher;
    before_actions(c).await?;
    let viewer = concerns::require_current_user(c)?.clone();
    let base_url = c.url_for("");
    let (secrets, now) = (c.app().secrets.clone(), now(c));
    let payload = c
        .app()
        .db
        .read(move |conn| {
            let loaded = switcher::load(conn, &secrets, &viewer, &base_url)?;
            let people: Vec<api::SwitcherPerson> = loaded
                .people
                .iter()
                .map(|person| api::SwitcherPerson {
                    user_id: person.id,
                    direct_room_id: person.dm_room_id,
                })
                .collect();
            let users = dto::users(conn, &secrets, people.iter().map(|p| p.user_id), now)?;
            Ok(api::Switcher {
                rooms: loaded
                    .rooms
                    .into_iter()
                    .map(|room| api::SwitcherRoom {
                        room_id: room.id,
                        name: room.name.unwrap_or_default(),
                        kind: match room.kind {
                            "dm" => api::SwitcherRoomKind::Dm,
                            "group" => api::SwitcherRoomKind::Group,
                            "voice" => api::SwitcherRoomKind::Voice,
                            "stage" => api::SwitcherRoomKind::Stage,
                            "board" => api::SwitcherRoomKind::Board,
                            _ => api::SwitcherRoomKind::Channel,
                        },
                        icon_name: room.icon_name,
                        unread: room.unread,
                        muted: room.muted,
                        favorite: room.favorite,
                    })
                    .collect(),
                people,
                threads: loaded
                    .threads
                    .into_iter()
                    .map(|thread| api::SwitcherThread {
                        thread_id: thread.id,
                        name: thread.name,
                        room_id: thread.room_id,
                        room_name: thread.room_name,
                    })
                    .collect(),
                users,
            })
        })
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &payload)
}
