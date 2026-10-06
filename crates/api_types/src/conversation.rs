//! Labels for the conversations a cross-room list refers to.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::RoomKind;

/// What a row in a cross-room list (saved items, scheduled messages, search results) calls the
/// room and thread it belongs to, as the classic pages do: `Room::display_names_for(rooms,
/// viewer)` for the room (a direct message is named after its other members) and
/// `channel_threads.name` for the thread. Lists carry one per distinct `(roomId, threadId)`
/// pair their rows name, so a row renders without the sidebar (which leaves out `invisible`
/// rooms).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ConversationName {
    pub room_id: i64,
    /// `null` for the room's root timeline.
    pub thread_id: Option<i64>,
    pub room_kind: RoomKind,
    /// The viewer-relative room name.
    pub room_name: String,
    /// The room's icon (`rooms.icon_name`), shown beside search hits; `null` for none.
    pub room_icon_name: Option<String>,
    /// `null` when `threadId` is.
    pub thread_name: Option<String>,
}
