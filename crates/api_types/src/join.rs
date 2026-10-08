use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{RoomDetail, SidebarRow};

/// `GET /api/v1/rooms/:id/preview`: an alive open room the viewer may join
/// (`RoomsController#show`'s join preview). Rooms have no topic column; the classic page shows
/// the name. `memberCount` is `memberships.count`. The body has no messages. 404 for every other
/// room, the same refusal `GET /api/v1/rooms/:id` gives a nonmember.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OpenRoomPreview {
    pub id: i64,
    /// `rooms.name`, or `""` when the room is unnamed (the classic heading is `#` plus this).
    pub name: String,
    pub member_count: i64,
}

/// `POST /api/v1/rooms/:id/join` (`rooms#join`, `Membership::join_open`). Idempotent: a viewer who
/// already belongs gets the same membership and no second broadcast. `row` is the joiner's
/// sidebar row, also published as `sidebar.row.upserted` when the membership was created.
/// 404 unless the room is an alive open room.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoomJoin {
    pub detail: RoomDetail,
    pub row: SidebarRow,
}
