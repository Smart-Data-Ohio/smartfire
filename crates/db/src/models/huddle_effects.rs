//! Render descriptions for `HuddleGrant#broadcast_voice_presence`. No HTML in the domain.
use crate::{Broadcast, Event, Tx};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Presence {
    pub room_id: i64,
}
impl Broadcast for Presence {
    const KIND: &'static str = "HuddleGrant#broadcast_voice_presence";
}
pub fn broadcast_presence(tx: &mut Tx<'_>, room_id: i64) {
    tx.emit_after_commit(Event::broadcast(&Presence { room_id }));
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChanged {
    pub room_id: i64,
}
impl Broadcast for StreamChanged {
    const KIND: &'static str = "Stream#broadcast_stream_changed";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStopped {
    pub room_id: i64,
    pub user_id: i64,
}
impl Broadcast for StreamStopped {
    const KIND: &'static str = "Stream#broadcast_stream_stopped_event";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageRoster {
    pub room_id: i64,
}
impl Broadcast for StageRoster {
    const KIND: &'static str = "Stage#broadcast_roster";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleEvent {
    pub room_id: i64,
    pub membership_id: i64,
}
impl Broadcast for RoleEvent {
    const KIND: &'static str = "Stage#broadcast_role_event_to_member";
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageEndedNote { pub message_id:i64 }
impl Broadcast for StageEndedNote { const KIND:&'static str="Stage#post_stage_ended_note"; }
