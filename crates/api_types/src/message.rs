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

/// `GET /api/v1/rooms/:id/messages`: one page of the room's root timeline (thread replies are
/// left out), oldest first, at most `Message::PAGE_SIZE` (40) messages; `around` returns up to 40
/// on each side of the anchor plus the anchor itself.
///
/// Query parameters, at most one of them: `before=<message id>` (`page_before`),
/// `after=<message id>` (`page_after`), `around=<message id>` (`page_around`); none gives the
/// newest page (`last_page`). An id that isn't on this room's root timeline is a 404.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MessagePage {
    pub messages: Vec<MessageDTO>,
    /// Every creator of a message on the page, once each, so the page renders without another
    /// request.
    pub users: Vec<crate::User>,
    /// The id to pass as `before` for the next older page: the oldest message here when an older
    /// one exists (`exists_before`), else `null` (the start of the room).
    pub before: Option<i64>,
    /// The id to pass as `after` for the next newer page: the newest message here when a newer
    /// one exists (`exists_after`), else `null` (the page reaches the present, and live events
    /// carry on from here).
    pub after: Option<i64>,
}

/// `POST /api/v1/rooms/:id/messages`: post to the room's root timeline (`messages#create`).
///
/// Idempotent on `clientMessageId` (`Message::find_duplicate`): posting the same id again
/// returns the message already created, with 200 instead of 201. The response body is the
/// [`MessageDTO`], the same one the `message.created` event carries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateMessage {
    /// Chosen by the client (a UUID v7); the pending row is matched to the created message by
    /// it, whichever of the response and the `message.created` event arrives first.
    pub client_message_id: String,
    /// The Markdown source, rendered by the server's pipeline into `bodyHtml`. Up to
    /// `Message::SOURCE_LIMIT` (50 000) characters.
    pub markdown_source: String,
    /// The message this one replies to, on the same timeline; `null` for none.
    pub reply_to_message_id: Option<i64>,
    /// Whether the replied-to author is notified; `null` keeps the default (true).
    pub reply_notify_author: Option<bool>,
}
