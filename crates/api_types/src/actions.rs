//! Message actions beyond editing: pins, saved items and forwards.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{MessageDTO, Timestamp, User};

/// The reply to `POST` (201) and `DELETE` (200) `/api/v1/messages/:id/pin`
/// (`messages/pins#create` / `#destroy`), and the `message.pinned` event on `room:<id>` (the JSON
/// twin of the pin badge, `pins_count` and `pins_list` replaces).
///
/// Any human member of the message's room may pin or unpin any message, replies included.
/// Pinning a pinned message and unpinning an unpinned one succeed unchanged. A room holds at most
/// 50 pins (`MessagePin::MAX_PER_ROOM`): one more is a 422.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PinState {
    pub message_id: i64,
    pub room_id: i64,
    pub pinned: bool,
    /// The room's pin count afterwards (`MessagePin::count_for_room`), for the header.
    pub pin_count: i64,
}

/// `GET /api/v1/rooms/:id/pins`: the room's pins, newest first (`created_at DESC, id DESC`),
/// at most 50, with the pinned messages themselves so the pane renders them in full.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PinList {
    pub pins: Vec<Pin>,
    /// The pinned messages, one per pin, in no particular order.
    pub messages: Vec<MessageDTO>,
    /// The pinners and the messages' creators, once each.
    pub users: Vec<User>,
}

/// One `message_pins` row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Pin {
    pub message_id: i64,
    pub pinner_id: i64,
    pub pinned_at: Timestamp,
}

/// `saved_items.status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SavedStatus {
    InProgress,
    Done,
}

/// One of the viewer's saved items (`saved_items`, unique per person and message). The reply
/// to `POST /api/v1/saved`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SavedItem {
    /// For `DELETE /api/v1/saved/:id` (unsave, 204).
    pub id: i64,
    pub message_id: i64,
    pub status: SavedStatus,
    /// When to remind the viewer; `null` for no reminder.
    pub remind_at: Option<Timestamp>,
    /// When the reminder went out; `null` until it does.
    pub reminded_at: Option<Timestamp>,
    pub created_at: Timestamp,
}

/// `POST /api/v1/saved`: save a message for later (`saved_items#create`). Any message in a room
/// the viewer belongs to. Saving a saved message only updates `remindAt` (it doesn't toggle).
/// Answers the [`SavedItem`] (201) and publishes `saved.changed` to the viewer's other tabs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SaveMessage {
    pub message_id: i64,
    /// Must be in the future (422 otherwise); `null` for no reminder.
    pub remind_at: Option<Timestamp>,
}

/// Which of a page's messages the viewer saved: carried by [`crate::MessagePage`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SavedMark {
    pub message_id: i64,
    pub saved_item_id: i64,
}

/// The `saved.changed` event on the viewer's `user` topic: they saved or unsaved a message in
/// another tab. New: the classic app has no broadcast for saved items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SavedChanged {
    pub message_id: i64,
    /// The saved item, or `null` when it was unsaved.
    pub saved_item_id: Option<i64>,
}

/// `GET /api/v1/forward_destinations`: where the viewer may forward to
/// (`message_forwards#destinations`): their rooms except boards, sorted by name, each with its
/// open threads. Direct rooms have no threads; locked threads are left out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ForwardDestinationList {
    pub destinations: Vec<ForwardDestination>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ForwardDestination {
    pub room_id: i64,
    /// The name the viewer knows the room by (a direct message's is relative to them).
    pub name: String,
    pub direct: bool,
    pub threads: Vec<ForwardThread>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ForwardThread {
    pub id: i64,
    pub name: String,
    pub status: crate::ThreadStatus,
}

/// `POST /api/v1/messages/:id/forwards`: forward a message the viewer can see
/// (`message_forwards#create`). Each destination gets a new message with a frozen copy of the
/// body, the attachment and the note, published as `message.created` there.
///
/// 422 when: not 1 to 5 destinations, a room isn't the viewer's or is a board, a thread doesn't
/// belong to its room, is in a direct room or is locked, a destination repeats, or the note is
/// over 50 000 characters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateForwards {
    /// Shown above the forwarded body; blank or `null` for none.
    pub note: Option<String>,
    pub destinations: Vec<ForwardTarget>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ForwardTarget {
    pub room_id: i64,
    /// A thread in that room, or `null` for its root timeline.
    pub thread_id: Option<i64>,
}

/// The reply to `POST /api/v1/messages/:id/forwards` (201): one per destination, in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ForwardResult {
    pub forwards: Vec<MessageDTO>,
}
