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
/// scheduled. The scheduler checks every 30 s and posts it as the viewer, like a typed message;
/// if they lost access by then it's dropped instead.
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
    /// Set once posted; `null` while pending.
    pub sent_at: Option<Timestamp>,
    /// Set when it was dropped (access lost); `null` otherwise.
    pub dropped_at: Option<Timestamp>,
    pub created_at: Timestamp,
}

/// `POST /api/v1/rooms/:id/scheduled_messages` (201 with the [`ScheduledMessage`]). Active
/// humans only.
///
/// 422 when the text is blank or over 50 000 characters, `sendAt` isn't in the future, the
/// thread isn't in this room, or the reply target isn't in the same conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateScheduledMessage {
    pub markdown_source: String,
    pub send_at: Timestamp,
    pub thread_id: Option<i64>,
    pub reply_to_message_id: Option<i64>,
}

/// `PATCH /api/v1/scheduled_messages/:id`: change a pending one's text or time (200 with the
/// [`ScheduledMessage`]). `DELETE` cancels it (204); `POST …/:id/send_now` posts it at once
/// (200 with it sent, or 202 while it's still sending). A message being sent right now is a 409
/// for all three; one that isn't the viewer's or isn't pending, a 404.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateScheduledMessage {
    pub markdown_source: String,
    pub send_at: Timestamp,
}

/// `GET /api/v1/scheduled_messages?roomId=`: the viewer's pending scheduled messages, soonest
/// first, in one room or (without `roomId`) everywhere. New JSON: the classic list is HTML.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ScheduledMessageList {
    pub scheduled_messages: Vec<ScheduledMessage>,
}
