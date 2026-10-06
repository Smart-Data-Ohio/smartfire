//! The `/api/v1/sync` WebSocket protocol: JSON text frames tagged by `t`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    MessageDTO, MessageRemoved, RoomRead, RoomUnread, SidebarRow, SidebarRowRemoved, UserPresence,
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
    /// Typing started or stopped in `conv` (a topic such as `room:12`).
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
        seq: i64,
        resumed: bool,
    },
    /// Events in sequence order, flushed every 25 ms or 64 events.
    Batch { events: Vec<SyncEvent> },
    /// The server can't replay these topics (epoch changed or the replay ring rolled over):
    /// refetch them over REST.
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
