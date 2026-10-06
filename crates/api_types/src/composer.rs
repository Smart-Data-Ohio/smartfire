//! What the composer asks for: autocomplete, slash commands, preview and scheduled sends.
//!
//! There's no room autocomplete endpoint: `#` completes from the sidebar the client already
//! holds, and inserts an in-app Markdown link (`[#name](/rooms/12)`; links starting with `/`
//! open in place). The server has no room-reference syntax to target.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{Timestamp, User};

/// `GET /api/v1/autocomplete/users?roomId=&query=`: people to mention
/// (`autocompletable/users`). With `roomId` (the viewer must be a member, else 404), that room's
/// members; without, the whole workspace. Active users only, bots included, `query` matching
/// anywhere in the name, by `LOWER(name)`, at most 20.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UserSuggestionList {
    pub suggestions: Vec<UserSuggestion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UserSuggestion {
    pub user: User,
    /// What to insert: `@[Exact Name]`, the only mention syntax (resolved when the message
    /// renders against exactly one active room member of that name). `null` when the name
    /// isn't unique in scope or contains `[`, `]` or a line break: the row shows disabled
    /// ("Duplicate name — type as plain text").
    pub mention_token: Option<String>,
}

/// Where an icon comes from (`Icons::lookup` order: brand, then custom, then emoji).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum IconKind {
    /// One of the built-in brand logos (`vendor/icons.yml`).
    Brand,
    /// A workspace icon an administrator uploaded (`workspace_icons`).
    Custom,
    /// A gemoji alias.
    Emoji,
}

/// Something `:name:` expands to in a message, and a reaction can be.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Icon {
    /// Insert as `:name:` (lowercase `[a-z0-9_]`).
    pub name: String,
    /// The human name: the icon's title, or the emoji alias with spaces, capitalised.
    pub title: String,
    pub kind: IconKind,
    /// Emoji only: the character.
    pub character: Option<String>,
    /// Brand and custom only: the image (`/icons/:name` for custom).
    pub image_url: Option<String>,
}

/// `GET /api/v1/autocomplete/icons?query=` (`autocompletable/icons`): at most 8, ranked exact,
/// then prefix, then substring, non-emoji before emoji, then by name; empty for an empty query.
/// `GET /api/v1/icons`: every brand and workspace icon (no emoji), by kind then name, for the
/// picker's custom tab.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct IconList {
    pub icons: Vec<Icon>,
}

/// `GET /api/v1/rooms/:id/slash_commands?threadId=` (`autocompletable/slash_commands`): the
/// built-ins in registry order (in a thread, `/poll` is left out), then the room's agent
/// commands by name. The client filters as the person types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SlashCommandList {
    pub commands: Vec<SlashCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SlashCommand {
    /// Without the slash, e.g. `remind`.
    pub name: String,
    /// One line; an agent command without one reads "Custom command".
    pub description: String,
    /// E.g. `<when> <text>`; empty for none and for agent commands.
    pub arg_hint: String,
    /// When `false`, picking it runs it at once.
    pub takes_arguments: bool,
    /// The agent that registered it; `null` for a built-in.
    pub agent_name: Option<String>,
}

/// `POST /api/v1/rooms/:id/slash_commands`: run a command (`rooms/slash_commands#create`).
/// Active human members only (403 otherwise). Always 200 with a [`SlashCommandResult`].
///
/// The client's routing, as the classic composer's: `//text` posts `/text` as a message; a known
/// command comes here; an unknown `/word` is posted as an ordinary message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RunSlashCommand {
    /// The whole line, starting with `/`.
    pub text: String,
    /// Run in this thread of the room; `null` for the root timeline.
    pub thread_id: Option<i64>,
}

/// What a command did, tagged by `status`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "status",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum SlashCommandResult {
    /// It posted a message (`/shrug`, `/me`, `/remind`, …), published as `message.created`.
    /// `notice` is a line to toast, e.g. "Reminder set for …".
    Posted {
        message_id: i64,
        notice: Option<String>,
    },
    /// A reply only the viewer sees (`/status`, `/dnd`, `/ooo`, an agent command's
    /// "Sent to …"): toast it.
    Ephemeral { message: String },
    /// It failed; `message` says why (and lists the commands for an unknown one).
    Error { message: String },
    /// Open this URL (an `/event` form).
    OpenUrl { url: String },
    /// Open the poll builder.
    OpenPoll,
    /// Start a huddle in this room.
    StartHuddle { room_id: i64, room_name: String },
}

/// `POST /api/v1/rooms/:id/messages/preview`: render Markdown without posting
/// (`messages#preview`), resolving mentions against the room's members.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PreviewMessage {
    /// Up to 50 000 characters (422 beyond).
    pub markdown_source: String,
}

/// The reply to `POST /api/v1/rooms/:id/messages/preview`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MessagePreview {
    /// Sanitized, as `MessageDTO.bodyHtml` would be.
    pub body_html: String,
}

/// A message to be sent later (`scheduled_messages`). Text only: attachments can't be
/// scheduled. The scheduler checks every 30 s and posts it as the viewer, like a typed message
/// (`ScheduledMessage::dispatch`, from `app/models/scheduled_message/dispatcher.rb`). If it
/// can't be posted by then it's dropped instead, with a `scheduled_message_dropped` activity
/// item.
///
/// Times are UTC like every other timestamp here; the classic JSON renders them in the viewer's
/// zone with an offset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ScheduledMessage {
    pub id: i64,
    pub room_id: i64,
    pub thread_id: Option<i64>,
    pub reply_to_message_id: Option<i64>,
    pub markdown_source: String,
    pub send_at: Timestamp,
    /// Derived from the timestamps, as the model does: there's no status column.
    pub state: ScheduledMessageState,
    /// Pending ones only: whether it can still be posted (`ScheduledMessage::sendable_ids`: the
    /// author is an active human and a member of the room, which isn't deleted, and the thread
    /// is in the room). A pending one that isn't sendable is "stranded": it will be dropped when
    /// due unless access comes back. `false` once sent or dropped.
    pub sendable: bool,
    /// Set once posted; `null` otherwise.
    pub sent_at: Option<Timestamp>,
    /// The message it became, for "View message"; `null` until sent, or once that message is
    /// deleted.
    pub sent_message_id: Option<i64>,
    /// Set when it was dropped; `null` otherwise.
    pub dropped_at: Option<Timestamp>,
    /// Why it was dropped (`scheduled_messages.drop_reason`): `"its room was deleted"`,
    /// `"its thread was deleted"`, or the message's validation errors joined. `null` when it
    /// wasn't dropped, or was dropped because the author lost access to the room (the classic
    /// page's "Not sent (channel access lost)").
    pub drop_reason: Option<String>,
    pub created_at: Timestamp,
}

/// A scheduled message's state (`ScheduledMessage#pending?/sent?/dropped?/claimed?`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum ScheduledMessageState {
    /// Waiting for its time.
    Pending,
    /// Pending, but a runner claimed it in the last 5 minutes (`STALE_CLAIM_AFTER`) and is
    /// posting it: until that finishes, editing or cancelling it is refused (409) and sending it
    /// now answers 202 with it unchanged.
    Sending,
    Sent,
    Dropped,
}

/// `POST /api/v1/rooms/:id/scheduled_messages` (201 with the [`ScheduledMessage`];
/// `scheduled_messages#create`). Active humans only (403 otherwise).
///
/// A thread that isn't in this room is a 404. 422 when the text is blank or over 50 000
/// characters, `sendAt` isn't in the future, or the reply target isn't in the same conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateScheduledMessage {
    pub markdown_source: String,
    pub send_at: Timestamp,
    pub thread_id: Option<i64>,
    pub reply_to_message_id: Option<i64>,
}

/// `PATCH /api/v1/scheduled_messages/:id`: change a pending one's text, time or both
/// (`scheduled_messages#update`; 200 with the [`ScheduledMessage`]). A field left out (or
/// `null`) keeps its value. `sendAt` must be in the future when it changes (422).
///
/// The other actions on one:
/// - `DELETE /api/v1/scheduled_messages/:id` cancels it (204; `#destroy`), deleting its activity
///   items.
/// - `POST /api/v1/scheduled_messages/:id/send_now` posts it at once (`#send_now`): 200 with it
///   sent; 202 with it still pending when another runner holds it or its thread is locked (it
///   isn't sent, and stays scheduled); 422 when it was dropped instead: an
///   `ApiError::Validation` whose `message` is the drop reason (the message is now `dropped`,
///   and `scheduled.changed` says so).
///
/// Editing or cancelling one that's `sending` is a 409 ("That message is sending right now; try
/// again in a moment."). Any of the three on one that isn't the viewer's or isn't pending is a
/// 404.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateScheduledMessage {
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub markdown_source: Option<String>,
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub send_at: Option<Timestamp>,
}

/// Which of the viewer's scheduled messages to list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum ScheduledMessageFilter {
    /// Not yet sent or dropped, soonest first (`send_at ASC, id ASC`): the classic page's
    /// "Upcoming" and "Stranded" sections, told apart by `sendable`. The default.
    Pending,
    /// Sent or dropped, most recent first (`send_at DESC, id DESC`): the "Past" section.
    Past,
}

/// `GET /api/v1/scheduled_messages?status=&roomId=&before=`: the viewer's scheduled messages
/// (`scheduled_messages#index`, `ScheduledMessage::owned_by`). Active humans only (403
/// otherwise, as for creating one).
///
/// - `status`: a [`ScheduledMessageFilter`], default `pending`; an unknown value reads as the
///   default.
/// - `roomId`: only this room's (the composer's list). New: the classic page lists every room.
/// - `before`: the previous page's `nextCursor`. Keyset paging in the filter's order, 50 a page.
///   A cursor that doesn't decode is a 422 (`ApiError::Validation` on `before`). New: the
///   classic page lists everything at once.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ScheduledMessageList {
    pub scheduled_messages: Vec<ScheduledMessage>,
    /// The rooms and threads they're in.
    pub conversations: Vec<crate::ConversationName>,
    /// Pass as `before` for the next page; `null` on the last (set only when another row
    /// exists past this page).
    ///
    /// Opaque to the client: it encodes the last row's `(sendAt, id)`, and the next page holds
    /// the rows strictly after that key in the filter's order. So it stays valid when that
    /// message is edited to another time, sent, dropped or cancelled. A message rescheduled
    /// behind the cursor after the client paged past it won't appear on later pages; the client
    /// learns of it from `scheduled.changed`.
    pub next_cursor: Option<String>,
}

/// The `scheduled.removed` event on the author's `user` topic: they cancelled a scheduled
/// message in another tab. New, like `scheduled.changed` (which carries the
/// [`ScheduledMessage`] whenever one is created, edited, sent or dropped): the classic app has
/// no broadcast for scheduled messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ScheduledMessageRemoved {
    pub id: i64,
    pub room_id: i64,
}
