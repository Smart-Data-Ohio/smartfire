use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    AgentStep, Attachment, Boost, MessageCard, Poll, Reaction, ThreadIndicator, Timestamp,
};

/// A message on a room's timeline or in a thread. Viewer-independent: whether the viewer may
/// edit it or is mentioned is worked out on the client.
///
/// Permissions the client derives (`messages#ensure_can_edit` / `ensure_can_delete`):
/// - **edit**: the viewer is the creator, it isn't a `systemNote`, and (for a reply) its thread
///   isn't locked;
/// - **delete**: it isn't a `systemNote`, and the viewer is the creator or an administrator;
/// - **pin, save, react, forward**: any human member of the room (bots never use this API).
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
    /// The classic built-in `/play` presentation; attachments take precedence.
    pub sound: Option<MessageSound>,
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
    /// The reply's source was deleted; keep its tombstone when the foreign key becomes null.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub reply_target_deleted_at: Option<Timestamp>,
    /// The original of a forward; `null` for an original or once the source is deleted. Where it
    /// came from is viewer-relative (the viewer may not see that room), so it isn't here: the
    /// "Forwarded from" header reads `GET /api/v1/messages/:id` on this id, whose 404 means the
    /// viewer can't see the source and the header says only "Forwarded", as the classic card's
    /// hidden "View original" link does (`message_forwards#forward_source`).
    pub forwarded_from_message_id: Option<i64>,
    /// When this message was forwarded here (`messages.forwarded_at`); `null` for an original.
    /// Stays set after the source is deleted, when `forwardedFromMessageId` goes `null`, so the
    /// "Forwarded" label survives.
    pub forwarded_at: Option<Timestamp>,
    /// The forwarder's note (`messages.forward_note`, plain text, up to 50 000 characters),
    /// shown above the forwarded body; `null` for none or an original.
    pub forward_note: Option<String>,
    pub edited_at: Option<Timestamp>,
    /// The one attached file (`has_one_attached :attachment`); `null` for none.
    pub attachment: Option<Attachment>,
    /// Files from the `attachments` slot, plus any legacy `attachment`, in attachment-id
    /// order. Omitted for legacy single-file messages to preserve their JSON shape.
    /// `attachment` remains the first file for clients that don't read this list yet.
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<Attachment>>,
    /// Emoji and icon reactions grouped by content, in order of first reaction
    /// (`boosts ORDER BY created_at`). Empty when there are none.
    pub reactions: Vec<Reaction>,
    /// Free-text boosts (the `boosts` rows that aren't reactions), oldest first.
    pub boosts: Vec<Boost>,
    /// Pinned in its room (`message_pins` has a row for it).
    pub pinned: bool,
    /// Root messages with a thread only: the reply indicator. `null` on replies and on root
    /// messages nobody has replied to in a thread.
    pub thread: Option<ThreadIndicator>,
    /// The poll this message asks, if it's a poll's question (`polls.message_id`); `null`
    /// otherwise. Kept current by `poll.updated` (by `asOf`, not `updatedAt`). A `null` here
    /// never clears a poll the client already holds (see the **Ordering** note in
    /// [`crate::MessageCard`]'s module).
    pub poll: Option<Poll>,
    /// The cards under the body (events, link previews), in the classic slot order; empty when
    /// there are none or the server doesn't fill that kind yet (see [`crate::MessageCard`]).
    /// Kept current by `message.cards`.
    pub cards: Vec<MessageCard>,
    /// When the server read `cards`. A `message.cards` with an earlier `asOf` is stale, and so
    /// is this message's `cards` when the client already holds a later one (see the **Ordering**
    /// note in [`crate::MessageCard`]'s module). `poll` carries its own `asOf`.
    pub cards_as_of: Timestamp,
    /// An agent's progress steps on this message, in `(position, id)` order; empty for people's
    /// messages and agents that report none. Kept current by `agent.steps`, merged per step by
    /// its own `updatedAt` (see [`AgentStep`]), since a step change doesn't bump this message's.
    pub steps: Vec<AgentStep>,
    pub created_at: Timestamp,
    /// Bumped by edits, embed suppression, streaming growth, reactions, boosts, pins and thread
    /// replies (`touch`), so a later `updatedAt` always holds the newer copy.
    pub updated_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MessageSound {
    pub name: String,
    pub url: String,
    pub presentation: SoundPresentation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum SoundPresentation {
    Text {
        text: String,
    },
    Image {
        url: String,
        width: u32,
        height: u32,
    },
}

/// The `message.removed` event: enough to drop the message from any cached page.
///
/// Published after `DELETE /api/v1/messages/:id` (`messages#destroy`, or
/// `channel_thread_messages#destroy` for a reply), which answers 204. Allowed for the creator or
/// an administrator, never for a system note. It's a hard delete: replies to it lose their
/// `replyToMessageId`, its pins, saves, reactions and attachment go, and a thread it started
/// stays (with no parent).
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
/// on each side of the anchor plus the anchor itself. `GET /api/v1/threads/:id/messages` pages a
/// thread's replies the same way (`Timeline::Thread`), with ids from that thread as anchors.
///
/// Query parameters, at most one of them: `before=<message id>` (`page_before`),
/// `after=<message id>` (`page_after`), `around=<message id>` (`page_around`); none gives the
/// newest page (`last_page`). An id that isn't on this room's root timeline is a 404.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MessagePage {
    pub messages: Vec<MessageDTO>,
    /// Every creator of a message on the page and every replier its thread indicators name,
    /// once each, so the page renders without another request.
    pub users: Vec<crate::User>,
    /// The id to pass as `before` for the next older page: the oldest message here when an older
    /// one exists (`exists_before`), else `null` (the start of the room).
    pub before: Option<i64>,
    /// The id to pass as `after` for the next newer page: the newest message here when a newer
    /// one exists (`exists_after`), else `null` (the page reaches the present, and live events
    /// carry on from here).
    pub after: Option<i64>,
    /// The viewer's saved items among the page's messages (`saved_items` for this user and
    /// these message ids), so the Save action shows its state without another request.
    pub saved: Vec<crate::SavedMark>,
}

/// `GET /api/v1/messages/:id`: one message, from a room the viewer belongs to (or a thread in
/// one), wherever it sits on its timeline (`Message::find_reachable`, as `message_forwards` and
/// pin links look messages up). Anything else is a 404. For a link to a message outside the
/// loaded window (a thread's root, a forward's source) without reloading the room around it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MessageRead {
    pub message: MessageDTO,
    /// Its creator, and the repliers its thread indicator names.
    pub users: Vec<crate::User>,
    /// The room and thread it's in, named for the viewer.
    pub conversation: crate::ConversationName,
    /// Whether the viewer saved it (`saved_items`); `null` when they haven't.
    pub saved: Option<crate::SavedMark>,
}

/// `POST /api/v1/rooms/:id/messages`: post to the room's root timeline (`messages#create`).
/// `POST /api/v1/threads/:id/messages` takes the same body and posts a reply to a thread
/// (`channel_thread_messages#create`), published as `message.created` on `thread:<id>`. A locked
/// thread refuses every reply, moderators' too (`ChannelThread::LockedError`: 403, "This thread is
/// locked"); a moderator unlocks it first with `PATCH /api/v1/threads/:id` `status: active`. A thread's first reply goes through
/// `POST /api/v1/rooms/:id/threads` ([`crate::CreateThread`]) instead.
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
    /// `Message::SOURCE_LIMIT` (50 000) characters. May be empty only with an attachment.
    #[serde(default)]
    pub markdown_source: String,
    /// The message this one replies to, on the same timeline; `null` for none.
    pub reply_to_message_id: Option<i64>,
    /// Whether the replied-to author is notified; `null` keeps the default (true).
    pub reply_notify_author: Option<bool>,
    /// A finished direct upload's `signedId` ([`crate::DirectUpload`]), attached as the
    /// message's one legacy file (`message[attachment]` given a signed blob id); `null` for none.
    pub attachment_signed_id: Option<String>,
    /// Finished direct uploads to attach as one message, in this order. At most ten, with
    /// no repeats; cannot be combined with `attachmentSignedId`. Omitted on the legacy path.
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachment_signed_ids: Option<Vec<String>>,
    /// Google Drive file ids to pin (`message[drive_file_ids][]`). Left out when there are none.
    /// At most ten; an invalid id is a 422, as the classic composer is. A message may be empty
    /// of Markdown when it carries one of these or a file.
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drive_file_ids: Option<Vec<String>>,
}

/// `PATCH /api/v1/messages/:id`: edit a message (`messages#update`, or
/// `channel_thread_messages#update` for a reply). The creator only, never a system note; a reply
/// in a locked thread is a 403. Answers the updated [`MessageDTO`] and publishes
/// `message.updated`. `editedAt` changes only when the text does. Drive ids named in
/// `removeDriveFileIds` are dropped from the message (`message[drive_file_ids][]` on the classic
/// edit form is the set that remains; this is the inverse). The creator only, same as the text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateMessage {
    /// The new Markdown, up to 50 000 characters; blank only when the message has an
    /// attachment or a Drive file left. Always sent: the classic update without it turns the
    /// message into rich text.
    pub markdown_source: String,
    /// Drive file ids to remove. Left out when the edit doesn't touch attachments. An invalid
    /// id is a 422 (`includes an invalid file id`), as the classic edit is.
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remove_drive_file_ids: Option<Vec<String>>,
}

/// `GET /api/v1/messages/:id/source`: what the edit box starts from
/// (`editable_markdown_source`): the message's Markdown, or its legacy rich text converted to
/// Markdown. Same permission as editing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MessageSource {
    pub message_id: i64,
    pub markdown_source: String,
}
