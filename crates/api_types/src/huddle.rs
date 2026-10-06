//! Huddles: the LiveKit calls any room can hold, and the voice rooms built around them
//! (`Rooms::HuddlesController`, `Users::HuddlePresenceController`, `Rooms::CallModerationController`,
//! `HuddleGrant`, `Huddle::JoinNotifier`).
//!
//! The media itself goes between the browser and LiveKit; the server only issues tokens
//! (`HuddleGrant::issue` and the LiveKit JWT, which the authorization gateway checks exactly as
//! for the classic app), records who's in the call, and moderates. Someone is in a call while one
//! of their grants is unrevoked and was seen by the gateway in the last 20 seconds
//! (`HuddleGrant::IN_CALL_WINDOW`).
//!
//! `GET /api/v1/huddles` and the `GET`, `POST` and `leave` endpoints under
//! `/api/v1/rooms/:id/huddle` check, before anything else (`Rooms::HuddlesController`'s
//! before-actions): 503 `Unavailable` ("Huddles are not configured") when the server has no
//! LiveKit settings, 403 `Forbidden` for bots and inactive users, and 404 `NotFound` for a room
//! the viewer doesn't belong to or that was deleted. Moderation and the stage endpoints check
//! only the room, as the classic ones do. Every `/api/v1` answer is `Cache-Control: no-store`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{StageRole, User};

/// Someone in a room's call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HuddleParticipant {
    pub user_id: i64,
    /// Their membership in the room: what moderation and the stage roster address.
    pub membership_id: i64,
    /// The LiveKit identities of their live grants, oldest first: one per tab or device that's in
    /// the call (`campfire-participant-<hex>`). The client maps LiveKit participants back to
    /// people with these.
    pub identities: Vec<String>,
    /// A host or administrator muted them (`memberships.server_muted_at`): their token can't
    /// publish until they're unmuted.
    pub server_muted: bool,
}

/// Who's in one room's call. A row of `GET /api/v1/huddles`, part of
/// `GET /api/v1/rooms/:id/huddle`, and the `huddle.presence` event's data.
///
/// The event is the JSON twin of `HuddleGrant#broadcast_voice_presence` (the sidebar and header
/// avatar stacks) and of the sidebar live dot in `Stream#broadcast_stream_changed`. It goes to
/// every member's `user` topic whenever someone joins, leaves, is disconnected, or their grant is
/// reissued or revoked, and when a stage stream starts or stops; a batch keeps only the latest
/// per room. An empty `participants` means the call ended.
///
/// Nothing is published when someone drops out of the 20-second window without leaving (a closed
/// laptop): like the classic app, the client polls `GET /api/v1/huddles` every 15 s while the tab
/// is visible to catch those.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HuddlePresence {
    pub room_id: i64,
    /// One entry per person, ordered by name (case-insensitively), as the avatar stacks show them.
    pub participants: Vec<HuddleParticipant>,
    /// Stage rooms: someone is streaming (the sidebar's live dot). Always `false` elsewhere.
    pub live: bool,
}

/// `GET /api/v1/huddles` (`users/huddle_presence#show`): the calls going on in the viewer's
/// rooms, in no particular order. Rooms with nobody in the call are left out. A 503 `Unavailable`
/// tells the client to hide every huddle control.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HuddlePresenceList {
    pub rooms: Vec<HuddlePresence>,
    /// Every participant's directory entry.
    pub users: Vec<User>,
}

/// `GET /api/v1/rooms/:id/huddle`: one room's call, before joining and while in it. It replaces
/// both `rooms/huddles#show` and `#participants`; the client also calls it every 45 s while
/// connected, and when LiveKit reports a reconnect or the tab becomes visible, to end the call at
/// once if access was lost (401, 403 or 404).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HuddleDetail {
    /// The room's name as the viewer sees it: for a direct message, its other members' names
    /// (`room_display_name`).
    pub room_name: String,
    pub presence: HuddlePresence,
    /// Every participant's directory entry.
    pub users: Vec<User>,
}

/// `POST /api/v1/rooms/:id/huddle` (`rooms/huddles#create`): join. Issues a grant for this
/// session and returns the LiveKit token for it, exactly as the classic endpoint does. A grant
/// the room no longer allows is a 404.
///
/// The token's `canPublish` follows `livekit::can_publish`: false while server-muted, and on a
/// stage for listeners. A role change or mute revokes the grant, which disconnects the client;
/// the `huddle.role` event tells it to join again for a fresh token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HuddleCredentials {
    /// The LiveKit server's public WebSocket URL.
    pub url: String,
    /// The LiveKit access token (a JWT).
    pub token: String,
    /// This grant's LiveKit identity.
    pub identity: String,
    pub grant_id: i64,
    pub room_id: i64,
    /// As in [`HuddleDetail::room_name`].
    pub room_name: String,
    /// What the token allows, so the client knows without decoding it: listeners skip the
    /// microphone check and start with publishing off. New on the wire.
    pub can_publish: bool,
}

/// `POST /api/v1/rooms/:id/huddle/moderation` (`rooms/call_moderation#mute`, `#unmute` and
/// `#disconnect`): in a voice or stage room only (404 elsewhere), an administrator, or a stage's
/// host, mutes, unmutes or disconnects a member. Answers 204.
///
/// Denied with 403 `Forbidden` when the viewer may not moderate, or the target is an
/// administrator and the viewer isn't ("Only administrators can moderate an administrator");
/// 422 `Validation` for one's own session ("You cannot moderate your own call session"), except an
/// administrator unmuting themselves; 404 for a membership that isn't in the room.
///
/// Muting revokes the target's grants and ends their stream; disconnecting revokes their grants
/// and ends their stream. The target hears `huddle.role`; everyone else sees `huddle.presence` and,
/// on a stage, `stage.updated`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ModerateHuddle {
    pub membership_id: i64,
    pub action: HuddleModeration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum HuddleModeration {
    Mute,
    Unmute,
    Disconnect,
}

/// The `huddle.role` event on the member's own `user` topic: the JSON twin of
/// `Stage#broadcast_role_event_to_member`. Their stage role moved between listener and
/// speaker or host, or they were muted or unmuted by a host or administrator (in a voice room
/// too). Their grant was revoked, so a client in this room's call joins again for a token with the
/// new permissions, and says "A host muted you" when `serverMuted` turned on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HuddleRoleChanged {
    pub room_id: i64,
    /// Stage rooms only; `null` elsewhere.
    pub stage_role: Option<StageRole>,
    pub server_muted: bool,
}

/// The `huddle.notice` event on the viewer's `user` topic: the JSON twin of `Huddle::JoinNotifier`'s
/// `user_<id>_huddle_notices` frames. Someone joined or left a call in one of the viewer's rooms,
/// or the call ended. Shown as a toast when the viewer is in that call, a banner otherwise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "kind",
    rename_all = "lowercase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum HuddleNotice {
    /// `huddle_joined`.
    Joined {
        room_id: i64,
        /// As the viewer sees it; a direct message names its other members.
        room_name: String,
        user_id: i64,
        user_name: String,
        /// The viewer is in this call.
        in_call: bool,
        /// They were in the call moments ago (a reconnect or a reissued grant): the client
        /// cancels the pending "left" notice instead of showing "joined".
        rejoin: bool,
    },
    /// `huddle_left`: sent to those still in the call.
    Left {
        room_id: i64,
        room_name: String,
        user_id: i64,
        user_name: String,
    },
    /// `huddle_ended`: the last person left. Clears the room's banner.
    Ended { room_id: i64 },
}

/// The `huddle.ring` event on the recipient's `user` topic: the JSON twin of the
/// `{activityItemId, huddleInvitation}` frames on `user_<id>_activity`. A call started in a room
/// that rings them (direct messages, and rooms whose involvement isn't `nothing` or `invisible`),
/// or it ended or was answered elsewhere.
///
/// The client rings (440/480 Hz every 3 s, for at most 45 s) unless `silent`. Joining navigates to
/// the room and joins; it also marks the activity item handled, and dismissing marks it read
/// (`PATCH /api/v1/activity/:id`), when there's an item. The inbox entry itself follows
/// `activity.item`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HuddleRing {
    /// The `huddle_started` or `huddle_missed` inbox item; `null` for a ring sent before its item
    /// exists (as the call starts) and for the end-of-call frame that follows such a ring.
    pub activity_item_id: Option<i64>,
    pub event: HuddleRingEvent,
    pub state: HuddleRingState,
    pub room_id: i64,
    /// As the recipient sees it.
    pub room_name: String,
    pub caller_name: String,
    /// Don't play a sound: do-not-disturb, quiet hours or the recipient's sound settings
    /// (`kind=huddle` sound policy). Always `false` when `event` is `ended`.
    pub silent: bool,
}

/// What a [`HuddleRing`] says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum HuddleRingEvent {
    /// `huddle_started`: ring while `state` is `unread`; stop when it's `read` or `handled`
    /// (answered or dismissed in another tab).
    Started,
    /// `huddle_ended`: stop ringing; show "caller left" for 5 s if it was ringing.
    Ended,
}

/// The inbox item's state when the ring was sent (`ActivityItem#state`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum HuddleRingState {
    Unread,
    Read,
    Handled,
}
