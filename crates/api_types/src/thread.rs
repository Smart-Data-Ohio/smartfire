//! Channel threads: a conversation hanging off one root message, shown in the right pane.
//! Board posts (threads without a parent in board rooms) are out of scope here.
//!
//! Replies are messages with `threadId` set. They never appear on the room's timeline and never
//! make the room unread; they're published on `thread:<id>` (subscribe while the pane is open),
//! and each reply also updates its parent's indicator on `room:<id>` (`thread.indicator`).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{CreateMessage, MessageDTO, Timestamp, User, WorkDetail, WorkFacts};

/// A thread as every room member sees it (`channel_threads`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Thread {
    pub id: i64,
    pub room_id: i64,
    /// The root message it hangs off; `null` once that message is deleted (the thread stays).
    pub parent_message_id: Option<i64>,
    pub creator_id: i64,
    /// Up to 100 characters; defaults to the parent's first line ("New thread" without one).
    pub name: String,
    pub status: ThreadStatus,
    /// Replies that count (`messages_count`: not system notes, not still streaming).
    pub reply_count: i64,
    /// Bumped by every reply, and by reopening a stale thread (`last_activity_at`).
    pub last_activity_at: Timestamp,
    /// Idle this long, an active thread reads as closed: 60, 1440, 4320 or 10080.
    pub auto_archive_after_minutes: i64,
    pub created_at: Timestamp,
    /// `null` unless the thread is tracked as work (see [`WorkFacts`]).
    pub work: Option<WorkFacts>,
}

/// `ChannelThread#status_in_room`: `locked` when locked; else `closed` when closed or idle past
/// its auto-archive; else `active`. Replying to a closed thread reopens it; a locked one refuses
/// replies (403).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum ThreadStatus {
    Active,
    Closed,
    Locked,
}

/// `thread_memberships.involvement`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum ThreadInvolvement {
    /// Muted: not even mentions notify.
    Nothing,
    /// The default ("not following"): only mentions notify.
    Mentions,
    /// Following: every reply notifies.
    Everything,
}

/// The viewer's place in a thread. Created by posting in it, creating it or joining it; the
/// viewer has none in a thread they never touched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ThreadMembership {
    pub thread_id: i64,
    pub involvement: ThreadInvolvement,
    /// When the newest unread reply was posted; `null` when read. Every member but the author
    /// goes unread on a reply, whatever their involvement. There's no read position, so no
    /// unread count or divider.
    pub unread_at: Option<Timestamp>,
    pub joined_at: Timestamp,
}

/// One row of the thread list: the thread and the viewer's membership in it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ThreadSummary {
    pub thread: Thread,
    /// `null` when the viewer isn't a member.
    pub membership: Option<ThreadMembership>,
}

/// Which threads `GET /api/v1/rooms/:id/threads` lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum ThreadFilter {
    /// The default: neither closed (or stale) nor locked.
    Active,
    Closed,
    Locked,
    All,
}

/// `GET /api/v1/rooms/:id/threads?state=active|closed|locked|all` (`channel_threads#index`):
/// the room's threads, most recently active first (`last_activity_at DESC, id DESC`). No paging.
/// Direct rooms have none.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ThreadList {
    pub threads: Vec<ThreadSummary>,
    /// The threads' creators, once each.
    pub users: Vec<User>,
}

/// What the viewer may do to a thread (`thread_with_facts`' permissions). A moderator is an
/// administrator or the room's creator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ThreadPermissions {
    /// Rename or change auto-archive: a moderator or the thread's creator.
    pub can_rename: bool,
    /// A moderator or the thread's creator, while active.
    pub can_close: bool,
    /// A closed thread: any member of it.
    pub can_reopen: bool,
    /// Moderators.
    pub can_lock: bool,
    pub can_unlock: bool,
    /// `DELETE /api/v1/threads/:id` (`channel_threads#destroy`): moderators. Its replies go with
    /// it; the parent stays, its indicator cleared (`thread.indicator` with `thread: null`), and
    /// `thread.removed` follows. 204; 403 for anyone else.
    pub can_delete: bool,
    /// An untracked thread: start tracking it as work. A moderator or the thread's creator
    /// (`settings_manageable`) who is an active member of the room. `false` once tracked.
    pub can_convert_work: bool,
    /// A tracked thread: edit its result and hand it off. As `canConvertWork`, or its owner.
    pub can_manage_work: bool,
    /// A tracked thread: move its status. As `canManageWork`.
    pub can_update_work_status: bool,
    /// A tracked thread: assign its owner, or stop tracking it. As `canConvertWork`.
    pub can_assign_work: bool,
    /// Offer "stop tracking": `canAssignWork`, except on a board post, which stays tracked.
    pub can_remove_work: bool,
}

/// `GET /api/v1/threads/:id` (`channel_threads#show`): the pane's header. The replies come from
/// `GET /api/v1/threads/:id/messages` (a [`crate::MessagePage`]). 404 unless the viewer belongs
/// to its room.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ThreadDetail {
    pub thread: Thread,
    pub membership: Option<ThreadMembership>,
    /// `null` once deleted.
    pub parent_message: Option<MessageDTO>,
    pub permissions: ThreadPermissions,
    /// The work section, for a tracked thread; `null` otherwise.
    pub work: Option<WorkDetail>,
    /// The thread's creator and the parent's, once each; for a tracked thread also the result's
    /// editor, every history actor, owner candidate and handoff receiver. (The owner is whole
    /// on [`crate::WorkFacts::owner`].)
    pub users: Vec<User>,
}

/// `POST /api/v1/rooms/:id/threads`: start a thread on a root message with its first reply
/// (`channel_threads#create`). The creator joins it.
///
/// Idempotent on `message.clientMessageId`: a retry returns the thread already made (200 instead
/// of 201). A message that already has a thread is a 409 (open that one instead); a direct or
/// board room is a 403 (a board takes posts, which come with S6), and a parent off the room's
/// root timeline is a 404. A `clientMessageId` that an earlier message outside a thread already
/// used is a 422 on `clientMessageId`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateThread {
    pub parent_message_id: i64,
    /// `null` takes the default from the parent's first line.
    pub name: Option<String>,
    pub message: CreateMessage,
}

/// The reply to `POST /api/v1/rooms/:id/threads`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ThreadCreated {
    pub detail: ThreadDetail,
    /// The first reply, to reconcile the pending row by `clientMessageId`.
    pub message: MessageDTO,
}

/// `PATCH /api/v1/threads/:id` (`channel_threads#update`): rename, close, reopen, lock or
/// unlock, as [`ThreadPermissions`] allow (403 otherwise). `null` leaves a field alone. Answers
/// the [`ThreadDetail`] and publishes `thread.updated`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateThread {
    /// 1 to 100 characters.
    pub name: Option<String>,
    /// `closed` closes, `locked` locks, `active` reopens or unlocks.
    pub status: Option<ThreadStatus>,
}

/// `POST /api/v1/threads/:id/join`: join or change involvement (`channel_threads#join`);
/// following is `everything`, unfollowing `mentions`. `DELETE /api/v1/threads/:id/join` leaves
/// (204). Both are the viewer's alone (no other tab is told).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct JoinThread {
    /// `null` joins with the default (`mentions`) or keeps the current one.
    pub involvement: Option<ThreadInvolvement>,
}

/// The reply to `POST /api/v1/threads/:id/join` and `POST /api/v1/threads/:id/read` (mark read;
/// 404 for a non-member), which also publishes `thread.read` to the viewer's other tabs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ThreadMembershipState {
    pub membership: ThreadMembership,
}

/// A root message's reply indicator ("3 replies · 2m ago" with avatars), on
/// `MessageDTO.thread` and in `thread.indicator`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ThreadIndicator {
    pub thread_id: i64,
    /// `messages_count`, as the classic indicator shows it. 0 hides the indicator.
    pub reply_count: i64,
    /// The thread's `last_activity_at`.
    pub last_reply_at: Timestamp,
    /// Up to 3 distinct authors of the newest replies, newest first, for the avatar stack.
    /// New: the classic indicator shows only the count.
    pub replier_ids: Vec<i64>,
}

/// The `thread.indicator` event on `room:<id>`: a reply was posted, deleted or finished
/// streaming, or the thread was deleted (the JSON twin of the `thread_indicator_message_*`
/// replace). Replaces the parent message's `thread`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ThreadIndicatorChanged {
    pub room_id: i64,
    pub parent_message_id: i64,
    /// `null` when the thread was deleted.
    pub thread: Option<ThreadIndicator>,
}

/// The `thread.removed` event on `room:<id>` and `thread:<id>`: a moderator deleted the thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ThreadRemoved {
    pub thread_id: i64,
    pub room_id: i64,
}

/// The `thread.unread` event on a member's `user` topic (the JSON twin of
/// `user_<id>_unread_threads`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ThreadUnread {
    pub thread_id: i64,
    pub room_id: i64,
    /// `false`: a reply made the thread unread for this member. `true`: sent to every room
    /// member when a reply or the parent was deleted; refresh the thread's row without marking
    /// it unread.
    pub refresh_only: bool,
}

/// The `thread.read` event on the person's `user` topic: they read the thread in another tab.
/// New: the classic app doesn't broadcast thread reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ThreadRead {
    pub thread_id: i64,
    pub room_id: i64,
}
