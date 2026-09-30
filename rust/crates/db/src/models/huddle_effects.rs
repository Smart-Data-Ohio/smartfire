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
