use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{RoomDetail, SidebarRow, Timestamp};

/// `GET /api/v1/rooms/:id/preview`: an alive open room the viewer may join
/// (`RoomsController#show`'s join preview). Rooms have no topic column; the classic page shows
/// the name and nothing else. The body has no messages. 404 for every other room, the same
/// refusal `GET /api/v1/rooms/:id` gives a nonmember.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OpenRoomPreview {
    pub id: i64,
    /// `rooms.name`, or `""` when the room is unnamed (the classic heading is `#` plus this).
    pub name: String,
}

/// `POST /api/v1/rooms/:id/join` (`rooms#join`, `Membership::join_open`). Idempotent: a viewer who
/// already belongs gets the same membership and no second broadcast. A hidden membership
/// (`invisible`, the only way to leave an open room) comes back at the room's default involvement
/// and publishes the change, as a created membership publishes `sidebar.row.upserted`. `row` is
/// the joiner's sidebar row, null only if the membership was hidden again before the reply was
/// read. 404 unless the room is an alive open room.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoomJoin {
    pub detail: RoomDetail,
    pub row: Option<SidebarRow>,
}

/// `GET /api/v1/rooms/browse?q=&cursor=`: the alive open rooms, the only rooms anyone may join.
/// Closed, direct, voice, stage, board and deleted rooms never appear, not even by name. `q`
/// matches the name or topic case-insensitively; name matches come first (exact, then prefix,
/// then anywhere), then topic-only matches, each by latest activity. No `q` lists every open room
/// by latest activity. 50 per page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ChannelDirectory {
    pub channels: Vec<DirectoryChannel>,
    /// Pass back as `cursor` for the next page; null on the last page.
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DirectoryChannel {
    pub room_id: i64,
    /// `rooms.name`, or `""` when the room is unnamed.
    pub name: String,
    pub topic: Option<String>,
    /// Active people with a membership, hidden or not, as the room's members pane lists them.
    pub member_count: i64,
    /// `rooms.updated_at`, which every new message touches.
    pub last_active_at: Timestamp,
    /// The viewer has a membership that isn't hidden, so the room shows in their sidebar.
    pub joined: bool,
}
