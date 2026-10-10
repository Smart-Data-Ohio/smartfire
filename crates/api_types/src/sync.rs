//! The `/api/v1/sync` WebSocket protocol: JSON text frames tagged by `t`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    ActivityItemChanged, ActivityItemRemoved, AgentStatusChanged, AgentStepsChanged,
    ApprovalUpdated, BoardAutomationsChanged, CustomStyles, EventsChanged, HuddleNotice, HuddlePresence, HuddleRing, HuddleRoleChanged,
    MessageCards, MessageDTO, MessageReactions, MessageRemoved, PinState, PollBallot, PollUpdated,
    RoomCategory, RoomCategoryRemoved, RoomRead, RoomUnread, SavedChanged, ScheduledMessage,
    ScheduledMessageRemoved, SidebarRow, SidebarRowRemoved, StageState, StageStreamStopped, Thread,
    ThreadIndicatorChanged, ThreadRead, ThreadRemoved, ThreadUnread, UserPresence,
};

/// A frame the client sends.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "t", rename_all = "lowercase", rename_all_fields = "camelCase")]
#[ts(export)]
pub enum ClientFrame {
    /// The first frame on every connection. `resume` is the last point the client saw, `null`
    /// on a fresh start; `topics` are the conversations it has open (the user topic is implicit).
    Hello {
        v: u32,
        resume: Option<ResumePoint>,
        topics: Vec<String>,
    },
    Sub {
        topics: Vec<String>,
    },
    Unsub {
        topics: Vec<String>,
    },
    /// Typing started or stopped in `conv`: `room:<id>` in a room's composer, `thread:<id>` in a
    /// thread's.
    Typing {
        conv: String,
        on: bool,
    },
    /// The client is looking at `room` (presence, replacing `PresenceChannel`'s `present`).
    Present {
        room: i64,
    },
    Absent {
        room: i64,
    },
    /// Heartbeat, every 25 s while connected: extends the connection's workspace presence lease
    /// (`WorkspacePresenceChannel#heartbeat`). `active` is whether the person used this tab
    /// since the last heartbeat; 10 minutes without an active one reads as `idle`.
    Hb {
        active: bool,
    },
}

/// Where a reconnecting client left off: the server's boot epoch and the last sequence seen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ResumePoint {
    pub epoch: String,
    pub seq: i64,
}

/// A frame the server sends.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "t", rename_all = "lowercase", rename_all_fields = "camelCase")]
#[ts(export)]
pub enum ServerFrame {
    /// The reply to `hello`. `resumed` is false when the client must refetch what it shows.
    Welcome {
        epoch: String,
        /// The client's starting cursor. On resume this stays at its last acknowledged event.
        seq: i64,
        resumed: bool,
        /// Events through this ring head are reconnect replay, not live arrivals.
        replay_through: i64,
    },
    /// Events in sequence order, flushed every 25 ms or 64 events.
    Batch { events: Vec<SyncEvent> },
    /// The server can't replay these topics (epoch changed, the replay ring rolled over, or
    /// events were skipped for this person while they had no socket open): refetch them over
    /// REST.
    Resync { topics: Vec<String>, reason: String },
    /// The server is closing the socket; reconnect if `reconnect`.
    Bye { reconnect: bool, reason: String },
    /// Keep-alive: sent after 15 s in which the connection carried nothing else. A client that
    /// hears nothing at all for 30 s treats the socket as dead and reconnects.
    Ping,
}

/// One event in a `batch`: `{"seq", "topic", "type", "data"}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SyncEvent {
    /// The hub's global publication sequence.
    pub seq: i64,
    /// `user`, `room:<id>` or `thread:<id>`.
    pub topic: String,
    #[serde(flatten)]
    pub payload: SyncPayload,
}

/// What happened, tagged by `type` with its body in `data`. The catalogue grows with the JSON
/// twins of the broadcasts in `crates/cable/BROADCASTS.md`, one domain at a time.
///
/// The client ignores (and logs) a `type` it doesn't know, so the server can deploy new events
/// first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type", content = "data")]
#[ts(export)]
pub enum SyncPayload {
    /// On everyone's `user` topic: the workspace name or images changed.
    #[serde(rename = "workspace.updated")]
    WorkspaceUpdated(WorkspaceBranding),
    /// On everyone's `user` topic: workspace CSS was saved or cleared.
    #[serde(rename = "workspace.styles.updated")]
    WorkspaceStylesUpdated(CustomStyles),
    /// On `room:<id>` (or `thread:<id>` for a reply): a message was posted. Carries the same
    /// [`MessageDTO`] the `POST` returns, so `clientMessageId` reconciles a pending send.
    #[serde(rename = "message.created")]
    MessageCreated(MessageDTO),
    /// On the message's conversation topic: an edit, embed suppression, or a streaming agent
    /// message growing or finishing. Applied only if `updatedAt` is newer than the copy held.
    #[serde(rename = "message.updated")]
    MessageUpdated(MessageDTO),
    /// On the message's conversation topic: the message was deleted.
    #[serde(rename = "message.removed")]
    MessageRemoved(MessageRemoved),
    /// On `room:<id>` or `thread:<id>`: someone started or stopped typing there. The client
    /// drops a typist after 6 s without a refresh.
    #[serde(rename = "typing")]
    Typing(Typing),
    /// On the member's `user` topic: the room became unread for them.
    #[serde(rename = "room.unread")]
    RoomUnread(RoomUnread),
    /// On the person's `user` topic: they read the room elsewhere.
    #[serde(rename = "room.read")]
    RoomRead(RoomRead),
    /// On the person's `user` topic: a row of their sidebar appeared or changed (a room was
    /// created, joined, renamed, re-iconed, favourited, categorized, or its involvement
    /// changed; a direct message's members changed). Replaces the row with this `roomId`.
    #[serde(rename = "sidebar.row.upserted")]
    SidebarRowUpserted(SidebarRow),
    /// On the person's `user` topic: a row left their sidebar.
    #[serde(rename = "sidebar.row.removed")]
    SidebarRowRemoved(SidebarRowRemoved),
    /// On every signed-in person's `user` topic: someone's workspace presence or status line
    /// changed (connected, went idle, disconnected, turned do-not-disturb on or off).
    #[serde(rename = "presence")]
    Presence(UserPresence),
    /// On the message's conversation topic: its reactions or boosts changed.
    #[serde(rename = "message.reactions")]
    MessageReactions(MessageReactions),
    /// On `room:<id>`: a message was pinned or unpinned (`pinned` says which). Sets the
    /// message's `pinned` and the room's pin count.
    #[serde(rename = "message.pinned")]
    MessagePinned(PinState),
    /// On `room:<id>`: a root message's reply indicator changed.
    #[serde(rename = "thread.indicator")]
    ThreadIndicator(ThreadIndicatorChanged),
    /// On `room:<id>` only: a thread was started there (a new thread has no followers yet but
    /// its creator, whose tab has the `POST` response). New: the classic app shows new threads
    /// only through the parent's indicator. A board post publishes it too, however it was created
    /// (the JSON twin of the classic board-row prepend).
    #[serde(rename = "thread.created")]
    ThreadCreated(Thread),
    /// On `room:<id>` and `thread:<id>`: a thread was renamed, closed, reopened, locked or
    /// unlocked, or its reply count or last activity moved, or its work changed (status, owner,
    /// result, run URL, links, a handoff, a board post's tags; see [`crate::WorkFacts`]). New, as
    /// `thread.created`; on a board it is the JSON twin of the board-row replace, so a board
    /// post's reply also publishes it (its `replyCount` and `lastActivityAt` moved).
    #[serde(rename = "thread.updated")]
    ThreadUpdated(Thread),
    /// On `room:<id>` and `thread:<id>`: a thread was deleted. Connections following
    /// `thread:<id>` stop following it.
    #[serde(rename = "thread.removed")]
    ThreadRemoved(ThreadRemoved),
    /// On `room:<id>`: a board's tag rules or SLA timers changed (see
    /// [`BoardAutomationsChanged`]). New.
    #[serde(rename = "board.automations.changed")]
    BoardAutomationsChanged(BoardAutomationsChanged),
    /// On a member's `user` topic: a thread went unread for them, or needs refreshing.
    #[serde(rename = "thread.unread")]
    ThreadUnread(ThreadUnread),
    /// On the person's `user` topic: they read a thread elsewhere.
    #[serde(rename = "thread.read")]
    ThreadRead(ThreadRead),
    /// On the person's `user` topic: they saved or unsaved a message elsewhere.
    #[serde(rename = "saved.changed")]
    SavedChanged(SavedChanged),
    /// On the owner's `user` topic: an activity item was recorded or changed state. The JSON twin
    /// of `ActivityChannel`'s `{activityItemId}` frame, carrying the item.
    #[serde(rename = "activity.item")]
    ActivityItem(ActivityItemChanged),
    /// On the owner's `user` topic: an activity item was deleted with its source. New.
    #[serde(rename = "activity.removed")]
    ActivityRemoved(ActivityItemRemoved),
    /// On the author's `user` topic: a scheduled message was created, edited, sent or dropped.
    /// New: the classic app has no scheduled-message broadcast.
    #[serde(rename = "scheduled.changed")]
    ScheduledChanged(ScheduledMessage),
    /// On the author's `user` topic: a scheduled message was cancelled. New.
    #[serde(rename = "scheduled.removed")]
    ScheduledRemoved(ScheduledMessageRemoved),
    /// On the owner's `user` topic: a sidebar category was created, renamed, folded or moved.
    /// New: the classic app has no category broadcast.
    #[serde(rename = "sidebar.category.upserted")]
    SidebarCategoryUpserted(RoomCategory),
    /// On the owner's `user` topic: a sidebar category was deleted (its rooms arrive first as
    /// `sidebar.row.upserted` with no category). New.
    #[serde(rename = "sidebar.category.removed")]
    SidebarCategoryRemoved(RoomCategoryRemoved),
    /// On the question's conversation topic: a poll's votes changed or it closed. The JSON twin
    /// of the classic `card_poll_<id>` replace (`Poll#broadcast_card`).
    #[serde(rename = "poll.updated")]
    PollUpdated(PollUpdated),
    /// On the voter's `user` topic: their own ballot changed (see [`PollBallot`]). New.
    #[serde(rename = "poll.ballot")]
    PollBallot(PollBallot),
    /// On the message's conversation topic: its cards changed (see [`MessageCards`]).
    #[serde(rename = "message.cards")]
    MessageCards(MessageCards),
    /// On `room:<id>`: an event there was scheduled, edited, cancelled or removed (see
    /// [`EventsChanged`]). New.
    #[serde(rename = "events.changed")]
    EventsChanged(EventsChanged),
    /// On every active human's `user` topic: an agent's status, note, suspension or working
    /// presence changed (see [`AgentStatusChanged`]).
    #[serde(rename = "agent.status")]
    AgentStatus(AgentStatusChanged),
    /// On the parent's conversation topic: an agent added or updated a step (see
    /// [`AgentStepsChanged`]).
    #[serde(rename = "agent.steps")]
    AgentSteps(AgentStepsChanged),
    /// On each decider's `user` topic: an approval request was decided, cancelled or expired
    /// (see [`ApprovalUpdated`]). New.
    #[serde(rename = "approval.updated")]
    ApprovalUpdated(ApprovalUpdated),
    /// On every member's `user` topic: who's in the room's call changed, or a stage stream
    /// started or stopped. Latest per room in a batch.
    #[serde(rename = "huddle.presence")]
    HuddlePresence(HuddlePresence),
    /// On the member's `user` topic: their stage role or server mute changed and their grant was
    /// revoked; rejoin for a fresh token.
    #[serde(rename = "huddle.role")]
    HuddleRole(HuddleRoleChanged),
    /// On the viewer's `user` topic: someone joined or left a call in one of their rooms, or it
    /// ended.
    #[serde(rename = "huddle.notice")]
    HuddleNotice(HuddleNotice),
    /// On the recipient's `user` topic: an incoming call rings, or stops ringing.
    #[serde(rename = "huddle.ring")]
    HuddleRing(HuddleRing),
    /// On `room:<id>` of a stage: its roster or live stream changed. Latest per room in a batch.
    #[serde(rename = "stage.updated")]
    StageUpdated(StageState),
    /// On the presenter's `user` topic: someone else ended their stream.
    #[serde(rename = "stage.stream.stopped")]
    StageStreamStopped(StageStreamStopped),
}

/// Workspace images as the SPA uses them; animated sources have a PNG still URL too.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorkspaceBranding {
    pub name: String,
    pub logo_url: Option<String>,
    pub logo_still_url: Option<String>,
    pub banner_url: Option<String>,
    pub banner_still_url: Option<String>,
}

/// Someone started or stopped typing in the event's topic. Never echoed to the typist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Typing {
    pub user_id: i64,
    /// `false` when they sent, cleared the composer or left.
    pub on: bool,
}
