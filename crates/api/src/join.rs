//! Nonmember preview and join for an alive open room (`rooms#show`'s join preview and `rooms#join`),
//! and the directory of every such room.

use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_db::models::room_directory::{self, DirectoryKey};
use campfire_db::{Involvement, Membership, Room, RoomType};
use campfire_kit::{Ctx, Error, Result, StatusCode};
use campfire_rooms::controllers::rooms::{find_joinable_open_room, involvements, join_open_room};
use campfire_runtime::concerns::{self, require_current_user};
use campfire_runtime::context::db_error;

use crate::dto;
use crate::endpoints::{before_actions, now};
use crate::error::{fail, validation};

const PAGE: usize = 50;
const QUERY_LIMIT: usize = 100;

endpoint!(
    /// `GET /api/v1/rooms/browse`
    browse => browse_rooms
);

endpoint!(
    /// `GET /api/v1/rooms/:room_id/preview`
    preview => show_preview
);
endpoint!(
    /// `POST /api/v1/rooms/:room_id/join`
    join => join_room
);

fn room_id(c: &Ctx) -> Option<i64> {
    c.param_str("room_id").and_then(concerns::cast_integer)
}

async fn show_preview(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let Some(id) = room_id(c) else {
        return Err(Error::NotFound);
    };
    let Some(room) = find_joinable_open_room(c, id).await? else {
        return Err(Error::NotFound);
    };
    c.json(
        StatusCode::OK,
        &api::OpenRoomPreview {
            id: room.id,
            name: room.name.unwrap_or_default(),
        },
    )
}

async fn join_room(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let Some(id) = room_id(c) else {
        return Err(Error::NotFound);
    };
    let Some((room, membership)) = join_open_room(c, id).await? else {
        return Err(Error::NotFound);
    };
    // Hiding is the only way to leave an open room, so joining one brings it back to the sidebar.
    if !membership
        .involvement
        .is_some_and(|involvement| involvement != Involvement::Invisible)
    {
        let default = Involvement::from_name(room.default_involvement());
        involvements::change(c, &room, membership.id, default).await?;
    }
    let viewer = require_current_user(c)?.clone();
    let (secrets, now) = (c.app().secrets.clone(), now(c));
    let joined = c
        .app()
        .db
        .read_snapshot(move |conn| {
            // The room and the membership are read again here, in the snapshot that renders
            // them: a hide or leave committed since the join must not leave a visible row. With
            // no joinable room or no membership any more, the answer is the 404 a failed join
            // gives.
            let Some(room) = Room::find_by_id(conn, id)?.filter(joinable) else {
                return Ok(None);
            };
            let Some(membership) = Membership::find_by_room_and_user(conn, id, viewer.id)? else {
                return Ok(None);
            };
            let detail = dto::room_detail(conn, &secrets, &viewer, &room, &membership, now)?;
            // `None` only when the membership was hidden again since the join.
            let row = dto::sidebar_row(conn, &room, &membership, now)?;
            Ok(Some(api::RoomJoin { detail, row }))
        })
        .await
        .map_err(db_error)?
        .ok_or(Error::NotFound)?;
    c.json(StatusCode::OK, &joined)
}

async fn browse_rooms(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let viewer = require_current_user(c)?.id;
    let query: String = c
        .param_str("q")
        .unwrap_or_default()
        .trim()
        .chars()
        .take(QUERY_LIMIT)
        .collect();
    let after = match c.param_str("cursor").filter(|raw| !raw.is_empty()) {
        None => None,
        Some(raw) => match crate::cursor::decode_ranked(raw) {
            Some((rank, updated_at, id)) => Some(DirectoryKey {
                rank,
                updated_at,
                id,
            }),
            None => return Err(fail(c, validation("cursor", "is invalid"))),
        },
    };
    let mut rooms = c
        .app()
        .db
        .read(move |conn| {
            room_directory::open_rooms(conn, viewer, query.trim_end(), after, PAGE + 1)
        })
        .await
        .map_err(db_error)?;
    let more = rooms.len() > PAGE;
    rooms.truncate(PAGE);
    let next_cursor = rooms
        .last()
        .filter(|_| more)
        .map(|room| crate::cursor::encode_ranked(room.key.rank, room.key.updated_at, room.key.id));
    let channels = rooms
        .into_iter()
        .map(|room| api::DirectoryChannel {
            room_id: room.key.id,
            name: room.name.unwrap_or_default(),
            topic: room.topic,
            member_count: room.member_count,
            last_active_at: dto::time(room.key.updated_at),
            joined: room.joined,
        })
        .collect();
    c.json(
        StatusCode::OK,
        &api::ChannelDirectory {
            channels,
            next_cursor,
        },
    )
}

/// What `find_joinable_open_room` accepts: an alive open room.
fn joinable(room: &Room) -> bool {
    room.deleted_at.is_none() && room.room_type == RoomType::Open
}
