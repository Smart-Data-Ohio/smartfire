use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{Timestamp, User};

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
    pub topic: Option<String>,
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

/// `GET /api/v1/rooms/:id`: a room the viewer belongs to, with what its header and timeline need
/// before the first page of messages. 404 when the viewer has no membership (`set_room`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoomDetail {
    pub room: Room,
    /// The viewer's own membership.
    pub membership: Membership,
    /// What the header shows: the room's name, or for a direct message its other members'
    /// names (`room_display_name(room, for_user:)`).
    pub display_name: String,
    /// `memberships.count` for the room.
    pub member_count: i64,
    /// `MessagePin::count_for_room`.
    pub pins_count: i64,
    /// Direct messages only: the other members in membership order, or just the viewer for a
    /// note-to-self. Empty for every other kind.
    pub direct_member_ids: Vec<i64>,
    /// Up to 5 members for the header's avatar stack, in membership order (oldest first),
    /// viewer included. Every id here and in `directMemberIds` has its entry in `users`.
    pub member_preview_ids: Vec<i64>,
    /// The people `directMemberIds` and `memberPreviewIds` name.
    pub users: Vec<User>,
    /// Where the unread divider goes (`unread_divider`); `null` when the room is read.
    pub unread: Option<UnreadDivider>,
}

/// The first unread root message and how many root messages follow it, as the classic room page
/// computes them from the membership's `last_read_message_id` and `unread_at`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UnreadDivider {
    /// The divider sits above this message. Load the timeline `around` it when it isn't on the
    /// newest page.
    pub first_unread_message_id: i64,
    /// Root messages from the first unread one to the newest, inclusive.
    pub count: i64,
}
