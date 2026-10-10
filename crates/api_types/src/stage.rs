//! Stage rooms: a huddle with hosts, speakers and an audience, raised hands, and one live screen
//! stream at a time (`Stage`, `Rooms::Stage::RolesController`, `HandsController`,
//! `StreamsController`, `Stream`).
//!
//! Every endpoint here 404s unless the room is a stage the viewer belongs to.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{StageRole, Timestamp, User};

/// A stage member as the roster shows them (`presenters::calls::stage_model`). Every membership
/// of a stage room has a role (listener when they join).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StageMember {
    pub membership_id: i64,
    pub user_id: i64,
    pub role: StageRole,
    /// Listeners only: when they raised their hand; the queue is ordered by it.
    pub hand_raised_at: Option<Timestamp>,
    /// A host or administrator muted them.
    pub server_muted: bool,
}

/// `stream_qualities`: the resolution and frame rate a presenter picks when going live.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum StreamQuality {
    #[serde(rename = "720p15")]
    P720Fps15,
    /// The default choice.
    #[serde(rename = "1080p15")]
    P1080Fps15,
    #[serde(rename = "1080p30")]
    P1080Fps30,
    /// Motion-first: games and video at full HD and 60 frames a second.
    #[serde(rename = "1080p60")]
    P1080Fps60,
}

/// The stage's live stream (`Stream.live_for_room`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StageStream {
    pub id: i64,
    pub membership_id: i64,
    pub user_id: i64,
    /// The presenter's current LiveKit identity (their newest live grant), so viewers can put
    /// that participant's screen share in theater mode; `null` if they have no live grant.
    pub identity: Option<String>,
    pub quality: StreamQuality,
    pub started_at: Timestamp,
}

/// A stage's roster and stream, the same for every viewer: each client works out what it may do
/// from its own membership (hosts and administrators manage; see the endpoints below).
///
/// Also the `stage.updated` event on `room:<id>`: the JSON twin of `Stage#broadcast_roster`,
/// `Stage#broadcast_panel_to_member` and the panel half of `Stream#broadcast_stream_changed`.
/// Sent when a role, hand, mute or the stream changes; a batch keeps only the latest per room.
/// The client keeps the room's topic while it's in the room's call, so the dock follows it from
/// other rooms too.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StageState {
    pub room_id: i64,
    /// Every member, in membership order. The roster groups them by role: hosts and speakers by
    /// name, listeners with raised hands first (oldest hand first), then by name.
    pub members: Vec<StageMember>,
    /// `null` when nobody's streaming.
    pub live: Option<StageStream>,
}

/// `GET /api/v1/rooms/:id/stage`: the stage panel's data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StageDetail {
    pub stage: StageState,
    /// Every member's directory entry.
    pub users: Vec<User>,
}

/// `PATCH /api/v1/rooms/:id/stage/members/:membershipId` (`rooms/stage/roles#update`): a host or
/// administrator moves a member between hosts, speakers and the audience. Answers the updated
/// [`StageState`].
///
/// 403 `Forbidden` unless the viewer is a host or administrator, and for a non-administrator
/// changing an administrator's role ("Only administrators can change an administrator's stage
/// role"); 422 `Validation` for demoting the last host ("can't demote the last host"); 404 for a
/// membership that isn't in the room. Any role change lowers the member's hand; moving someone to
/// the audience ends their stream; moving between the audience and the speakers or hosts revokes
/// their grant (`huddle.role`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ChangeStageRole {
    pub role: StageRole,
}

/// `POST /api/v1/rooms/:id/stage/hand` raises the viewer's hand (listeners only, else 422
/// `Validation` "Only listeners can raise a hand"; more than 10 a minute is 429 `RateLimited`).
/// `DELETE /api/v1/rooms/:id/stage/hand` lowers it, or with `?membershipId=` a host or
/// administrator lowers someone else's (403 otherwise: "Only hosts can lower another member's
/// hand"). Both answer the updated [`StageState`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LowerHand {
    /// `null` for the viewer's own hand.
    pub membership_id: Option<i64>,
}

/// `POST /api/v1/rooms/:id/stage/stream` (`rooms/stage/streams#create`): go live. Answers the new
/// [`StageStream`]. The client captures the screen first (inside the click), then calls this,
/// then publishes; if publishing fails it stops the stream again.
///
/// 403 `Forbidden` with the reason: "Only hosts and speakers can go live", "The stage needs a host
/// to go live", "Muted members cannot go live", "Join the stage before going live" (no grant seen
/// in the last 20 s); 409 `Conflict` "<name> is already live".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StartStageStream {
    pub quality: StreamQuality,
}

/// `DELETE /api/v1/rooms/:id/stage/stream?streamId=` (`rooms/stage/streams#destroy`): the
/// presenter, a host or an administrator ends the live stream (403 for anyone else: "Only the
/// presenter or a host can stop the stream"). With `streamId`, only that stream: a stale stop
/// can't end a newer one. Answers 204; sent with `keepalive` when the page is closing.
///
/// When someone other than the presenter stops it, the presenter hears `stage.stream.stopped`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StopStageStream {
    pub stream_id: Option<i64>,
}

/// The `stage.stream.stopped` event on the presenter's `user` topic: the JSON twin of
/// `Stream#broadcast_stream_stopped_event`. A host or administrator ended their stream, so the
/// client stops sharing. (Being muted or moved to the audience also ends it, but those revoke
/// the grant and disconnect the client instead.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StageStreamStopped {
    pub room_id: i64,
}
