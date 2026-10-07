use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The reply to `POST /api/v1/rooms/:id/read` (mark the room read up to its newest root message,
/// `rooms/reads#create`) and `DELETE /api/v1/rooms/:id/read` with a [`MarkUnread`] body (mark it
/// unread from a message on, `rooms/reads#destroy`). Both also notify the person's other tabs
/// with `room.read` / `room.unread` on their `user` topic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ReadState {
    pub room_id: i64,
    pub unread: bool,
    /// The message the room is unread from; `null` after marking it read.
    pub first_unread_message_id: Option<i64>,
    /// Root messages from `firstUnreadMessageId` to the newest, as `SidebarRow.unreadCount`
    /// counts them, for the sidebar badge; 0 after marking it read.
    pub unread_count: i64,
}

/// The body of `DELETE /api/v1/rooms/:id/read`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MarkUnread {
    /// A message on the room's root timeline; it and everything after it become unread.
    pub message_id: i64,
}

/// The `room.unread` event on a member's `user` topic: the JSON twin of
/// `user_<id>_unreads {roomId}`. Sent to every member a new root message makes the room unread
/// for (muted members only when mentioned, `unread_user_ids`), and to the person's other tabs
/// when they mark a room unread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoomUnread {
    pub room_id: i64,
    /// The new root message that made it unread; `null` when the person marked the room unread
    /// themselves (the client then refetches the row's counts).
    pub message_id: Option<i64>,
    /// The message mentions this member (`mentionees`), so the mention count goes up too.
    pub mentioned: bool,
}

/// The `room.read` event on a person's `user` topic: they read the room in another tab
/// (the JSON twin of `user_<id>_reads {room_id}`). Clears its unread and mention counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoomRead {
    pub room_id: i64,
}
