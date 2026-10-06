//! The `/api/v1/sync` WebSocket protocol: JSON text frames tagged by `t`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{MessageDTO, MessageRemoved};

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
    /// Heartbeat.
    Hb,
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type", content = "data")]
#[ts(export)]
pub enum SyncPayload {
    #[serde(rename = "message.created")]
    MessageCreated(MessageDTO),
    #[serde(rename = "message.updated")]
    MessageUpdated(MessageDTO),
    #[serde(rename = "message.removed")]
    MessageRemoved(MessageRemoved),
    #[serde(rename = "typing")]
    Typing(Typing),
}

/// Someone started or stopped typing in the event's topic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Typing {
    pub user_id: i64,
    pub on: bool,
}
