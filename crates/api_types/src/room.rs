use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Timestamp;

/// A conversation: channel, direct message, voice or stage room, or board.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Room {
    pub id: i64,
    pub kind: RoomKind,
    /// `null` for direct messages, which are named after their members.
    pub name: Option<String>,
    pub icon_name: Option<String>,
    pub creator_id: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// `rooms.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum RoomKind {
    Open,
    Closed,
    Direct,
    Voice,
    Stage,
    Board,
}

/// One person's place in a room: their notification level, read position and sidebar placement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Membership {
    pub id: i64,
    pub room_id: i64,
    pub user_id: i64,
    pub involvement: Involvement,
    /// When the room last became unread; `null` when read.
    pub unread_at: Option<Timestamp>,
    /// The newest root message seen; the unread divider starts after it.
    pub last_read_message_id: Option<i64>,
    pub room_category_id: Option<i64>,
    /// Position among favourites; `null` when not a favourite.
    pub favorite_position: Option<i64>,
    /// Stage rooms only.
    pub stage_role: Option<StageRole>,
}

/// `memberships.involvement` (a missing value reads as `mentions`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Involvement {
    Invisible,
    Nothing,
    Muted,
    Mentions,
    Everything,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum StageRole {
    Listener,
    Speaker,
    Host,
}
