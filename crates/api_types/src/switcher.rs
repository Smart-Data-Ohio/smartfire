//! The quick switcher (⌘/Ctrl+K).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::User;

/// `GET /api/v1/switcher` (`switchers#show`): everything the switcher searches, in one
/// response; the client ranks as the person types (fuzzy subsequence, recents first when the
/// query is empty).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Switcher {
    /// The viewer's rooms (memberships that aren't invisible, rooms not deleted), by
    /// `LOWER(rooms.name)`.
    pub rooms: Vec<SwitcherRoom>,
    /// Active humans but the viewer, by name.
    pub people: Vec<SwitcherPerson>,
    /// The 15 most recently active threads in the viewer's rooms.
    pub threads: Vec<SwitcherThread>,
    /// The people `people` names.
    pub users: Vec<User>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SwitcherRoom {
    pub room_id: i64,
    /// The room's name; a direct room's other members' full names joined with "and".
    pub name: String,
    pub kind: SwitcherRoomKind,
    pub icon_name: Option<String>,
    pub unread: bool,
    pub muted: bool,
    pub favorite: bool,
}

/// The switcher's room kinds: `group` is a direct room with more than 2 members.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum SwitcherRoomKind {
    Channel,
    Dm,
    Group,
    Voice,
    Stage,
    Board,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SwitcherPerson {
    pub user_id: i64,
    /// The one-to-one room with them, if there is one; else picking them creates it
    /// (`POST /api/v1/directs`).
    pub direct_room_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SwitcherThread {
    pub thread_id: i64,
    pub name: String,
    pub room_id: i64,
    /// `rooms.name`; `null` for a direct room.
    pub room_name: Option<String>,
}
