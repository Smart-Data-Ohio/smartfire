//! The activity inbox: the viewer's mentions, replies, followed threads, work, events,
//! reminders, agent requests and security notices (`activity_items`).
//!
//! Ports `activity_items#index/open/read/handled/unread_count`
//! (`crates/campfire/src/controllers/activity_items.rs`, from
//! `app/controllers/activity_items_controller.rb`) and the `ActivityChannel` broadcast
//! (`crates/channels/src/channels/activity.rs`, from `app/channels/activity_channel.rb`).
//!
//! Who sees an item (`ActivityItem::accessible_to`, `crates/db/src/models/activity_item/access.sql`):
//! only its owner, and only while they're an active human. Items sourced on a message, saved
//! item, work event, SLA nudge, huddle or event also need the owner to be a member of the
//! source's room now; agent approvals and budget notices need the owner to own the agent or be
//! an administrator; scheduled messages, sign-ins and two-step lockouts must be the owner's own.
//! An item the viewer can't see is a 404 everywhere and never counted.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{Timestamp, User};

/// `activity_items.event_type` (`ActivityItem::EVENT_TYPES`, `crates/db/src/models/activity_item.rs`).
/// The label is what the classic inbox shows (`presenters/activity.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ActivityEventType {
    /// "Mention": the viewer was @-mentioned.
    Mention,
    /// "Reply": someone replied to the viewer's message.
    Reply,
    /// "Followed thread": a reply in a thread the viewer follows, or a new post on a board they
    /// open. Grouped: one unhandled item per thread is re-pointed at the newest reply.
    ThreadActivity,
    /// "Keyword alert": a message matched one of the viewer's notification keywords.
    KeywordAlert,
    /// "Work update": a work thread's status or owner changed. Grouped like `thread_activity`.
    WorkUpdate,
    /// "Work assignment": the viewer was made a work thread's owner.
    WorkAssignment,
    /// "SLA breach": a board post sat in a status past its SLA.
    WorkSla,
    /// "Incoming huddle".
    HuddleStarted,
    /// "Missed huddle".
    HuddleMissed,
    /// "Event invitation".
    EventInvitation,
    /// "Event update".
    EventUpdate,
    /// "Event cancelled".
    EventCancelled,
    /// "Event reminder".
    EventReminder,
    /// "Review requested": a GitHub pull request review was requested of the viewer.
    PrReviewRequest,
    /// "Approval request": an agent the viewer owns (or any, for an administrator) is waiting
    /// for approval.
    AgentApprovalRequest,
    /// "Budget exceeded": an agent hit a daily cap.
    AgentBudgetExceeded,
    /// "Reminder": a saved message's reminder came due.
    MessageReminder,
    /// "Scheduled message not sent": one of the viewer's scheduled messages was dropped.
    ScheduledMessageDropped,
    /// "Sign-in lockout": two-step sign-in was locked after failed attempts.
    TwoFactorLockout,
    /// "New sign-in": the account signed in from a new session.
    NewSignIn,
}

/// What an item points at (`activity_items.source_type`: the eleven source tables
/// `ActivityItem` validation knows, `crates/db/src/models/activity_item.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ActivitySourceType {
    Message,
    SavedItem,
    WorkThreadEvent,
    BoardSlaNudge,
    HuddleGrant,
    Event,
    AgentApproval,
    AgentBudgetNotice,
    ScheduledMessage,
    TwoFactorCredential,
    Session,
}

/// An item's state, derived from its timestamps (`ActivityItem#state`): `handled` when
/// `handledAt` is set, else `read` when `readAt` is set, else `unread`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum ActivityState {
    Unread,
    Read,
    Handled,
}

/// The inbox's type tabs (`campfire_views::activity::TYPES`; the event types each holds are in
/// `ActivityItem::query_accessible`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum ActivityTab {
    /// "All": every type, and the only tab that shows `scheduled_message_dropped`.
    All,
    /// "Mentions and replies": `mention`, `reply`, `keyword_alert`.
    Mentions,
    /// "Threads and work": `thread_activity`, `work_update`, `work_assignment`, `work_sla`.
    Threads,
    /// "Events": `event_invitation`, `event_update`, `event_cancelled`, `event_reminder`.
    Events,
    /// "Agents": `agent_approval_request`, `agent_budget_exceeded`.
    Agents,
    /// "GitHub": `pr_review_request`.
    Github,
    /// "Huddles": `huddle_started`, `huddle_missed`.
    Huddles,
    /// "Reminders": `message_reminder`.
    Reminders,
    /// "Security": `new_sign_in`, `two_factor_lockout`.
    Security,
}

/// One inbox entry (`activity_items`, unique per owner and source).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ActivityItem {
    pub id: i64,
    pub event_type: ActivityEventType,
    pub state: ActivityState,
    pub read_at: Option<Timestamp>,
    pub handled_at: Option<Timestamp>,
    /// When the item was first recorded.
    pub created_at: Timestamp,
    /// The inbox's sort key: bumped when a grouped thread or work item is re-pointed at newer
    /// activity, or an old item is re-armed (`refresh_unread`). New on the wire: the classic
    /// JSON sorts by it but leaves it out.
    pub updated_at: Timestamp,
    /// What it's about; `null` when the source is gone.
    pub source: Option<ActivitySource>,
}

/// The thing an item is about, as the classic inbox row shows it
/// (`presenters::activity::payload_with_sources` for the ids and body, `campfire_views::activity::Item`
/// for the title and timestamp).
///
/// Unlike the classic JSON, which leaves `source` null for SLA nudges, scheduled messages and
/// two-step lockouts (following Rails' JSON), this is filled for every type the HTML inbox
/// renders, with the same title and body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ActivitySource {
    pub source_type: ActivitySourceType,
    pub source_id: i64,
    /// The room it happened in; `null` for agent budget notices, sign-ins and lockouts (and an
    /// approval outside a room).
    pub room_id: Option<i64>,
    /// The thread, for messages in a thread, work events and SLA nudges.
    pub thread_id: Option<i64>,
    /// The message to open at, for `message` and `saved_item` sources (a saved item's message).
    pub message_id: Option<i64>,
    /// `event` sources only: the calendar event.
    pub event_id: Option<i64>,
    /// Who's behind it: the message's creator, the event's organizer, the huddle's caller, the
    /// work event's actor, the agent's user, or the scheduled message's owner. `null` for
    /// sign-ins, lockouts and a work event without an actor.
    pub creator_id: Option<i64>,
    /// The row's heading: the viewer-relative room name, then ` · ` and the thread name or the
    /// event title where there is one; `"{bot} · daily {label} budget"`; `"Two-step sign-in"`;
    /// `"Account security"`.
    pub title: String,
    /// Plain text, at most 500 characters (497 and `...`): the message, or a sentence such as
    /// "Ada started a huddle", "Status: Planned → In progress", "You asked to be reminded about
    /// this message: …" or "Your scheduled message was not sent (…): …".
    pub body: String,
    /// The timestamp the row shows: the source's own creation time, not the item's.
    pub occurred_at: Timestamp,
    /// Agent approvals: `pending`, `approved`, `denied`, `cancelled` or `expired`
    /// (`AgentApproval#effective_status`). Agent budget notices: the cap, `messages`,
    /// `board_posts` or `external_actions`. `null` otherwise.
    pub status: Option<String>,
    /// The classic page `open` leads to (`presenters::activity` destination), for sources the SPA
    /// has no screen for yet, e.g. `/agents/3/approvals` or `/users/me/sessions`. The SPA routes
    /// the others from the ids above.
    pub path: String,
}

/// `GET /api/v1/activity?status=&type=&before=`: one page of the inbox
/// (`activity_items#index`).
///
/// - `status`: `unread` (default), `read` or `handled`; an unknown value reads as the default.
/// - `type`: an [`ActivityTab`], default `all`; an unknown value reads as `all`.
/// - `before`: the previous page's `nextCursor`. Keyset paging on `(updatedAt, id)`, newest first;
///   a cursor that isn't an item the viewer can see is ignored.
///
/// At most 100 items a page. Listing first settles the viewer's overdue huddle invitations and
/// agent approvals, as the classic page does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ActivityList {
    pub items: Vec<ActivityItem>,
    /// Every `creatorId` on the page, once each.
    pub users: Vec<User>,
    /// The viewer's unread items across every type (the badge), as `unreadCount` below.
    pub unread_count: i64,
    /// Pass as `before` for the next page; `null` when this is the last. Set only when a newer
    /// row exists past this page (the server reads 101), unlike the classic `next_cursor`,
    /// which is set on any full page.
    pub next_cursor: Option<i64>,
}

/// `GET /api/v1/activity/unread_count` (`activity_items#unread_count`, `no-store`): the badge.
/// Unread means neither read nor handled; every type counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ActivityUnreadCount {
    pub unread_count: i64,
}

/// `PATCH /api/v1/activity/:id`: move an item between states, replacing the classic
/// `PATCH /activity/:id/read?state=` and `PATCH /activity/:id/handled?state=` pair. Answers
/// [`ActivityItemChanged`] and publishes it as `activity.item` to the viewer's other tabs.
///
/// - `read`: from `unread`, sets `readAt` (`mark_read`); from `handled`, clears `handledAt` and
///   keeps `readAt` (`mark_unhandled`).
/// - `unread`: clears both `readAt` and `handledAt` (`mark_unread`).
/// - `handled`: sets `handledAt`, and `readAt` if it was unset (`mark_handled`).
///
/// Setting the state an item already has succeeds unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateActivityItem {
    pub state: ActivityState,
}

/// The reply to `PATCH /api/v1/activity/:id` and `POST /api/v1/activity/:id/open` (which marks
/// it read, never handled, before the SPA navigates to the source: `activity_items#open`), and
/// the `activity.item` event on the owner's `user` topic.
///
/// The event is the JSON twin of `ActivityChannel`'s `{activityItemId}` frame
/// (`ActivityItem::broadcast_item_for_user`): an item was recorded, re-armed, revived by newer
/// thread activity, read, unread, handled or unhandled, or its approval was settled or expired.
/// It carries the item itself so the client needn't refetch. A grouped item re-pointed at a newer
/// reply without a state change isn't published, as in the classic app. Huddle invitation
/// frames on the same channel belong to the huddle slice's `huddle.*` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ActivityItemChanged {
    pub item: ActivityItem,
    /// The owner's unread count afterwards.
    pub unread_count: i64,
}

/// The `activity.removed` event on the owner's `user` topic: an item went with its source
/// (unsaving a message deletes its reminder items, cancelling a scheduled message its drop
/// item). New: the classic inbox only drops these on reload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ActivityItemRemoved {
    pub id: i64,
    pub unread_count: i64,
}
