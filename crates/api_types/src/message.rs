use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Timestamp;

/// A message on a room's timeline or in a thread. Viewer-independent: whether the viewer may
/// edit it or is mentioned is worked out on the client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MessageDTO {
    pub id: i64,
    pub room_id: i64,
    /// `null` on the room's root timeline.
    pub thread_id: Option<i64>,
    pub creator_id: i64,
    /// The sender's id for the message, which makes posting idempotent.
    pub client_message_id: String,
    /// The rendered body from the server's sanitizer pipeline.
    pub body_html: String,
    /// The Markdown the body was written in, for editing; `null` for rich-text-only bodies.
    pub markdown_source: Option<String>,
    /// A quiet timeline note: shown, but never unread or notified.
    pub system_note: bool,
    /// An `/me`-style action.
    pub action: bool,
    /// Still being written by an agent.
    pub streaming: bool,
    pub embeds_suppressed: bool,
    pub reply_to_message_id: Option<i64>,
    pub forwarded_from_message_id: Option<i64>,
    pub edited_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// The `message.removed` event: enough to drop the message from any cached page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MessageRemoved {
    pub id: i64,
    pub room_id: i64,
    pub thread_id: Option<i64>,
}
