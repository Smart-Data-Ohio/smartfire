//! The cable server's vocabulary, shared by everything that broadcasts: the connection's user
//! ([`CableUser`]), the GlobalIDs and stream names streams are built from, and the typed
//! broadcasts ([`broadcasts`]). The channels themselves are `crate::channels`.

pub mod broadcasts;
pub mod sync;

use campfire_cable::{Identified, Server};
use rails_compat::global_id::GlobalId;

pub use broadcasts::{Broadcasts, Partials};

/// The cable server, identified by `current_user`.
pub type Cable = Server<CableUser>;

/// `identified_by :current_user`: the user as loaded when the connection opened, and the
/// connection's `current_session` (by id: channels that need it read it fresh).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CableUser {
    pub id: i64,
    pub name: String,
    pub role: campfire_db::Role,
    pub status: campfire_db::Status,
    pub session_id: i64,
}

impl CableUser {
    pub fn bot(&self) -> bool {
        self.role == campfire_db::Role::Bot
    }

    /// `ActivityItem.active_human?(user)`: `user&.active? && !user.bot?`.
    pub fn active_human(&self) -> bool {
        self.status == campfire_db::Status::Active && !self.bot()
    }
}

impl Identified for CableUser {
    /// `connection_gid`: the user's GlobalID, which `remote_connections.where(current_user:)`
    /// matches.
    fn connection_identifier(&self) -> String {
        user_gid(self.id).to_string()
    }
}

pub fn user_gid(user_id: i64) -> GlobalId {
    GlobalId::new("User", user_id)
}

/// A room's GlobalID names its STI class (`gid://campfire/Rooms::Open/1`).
pub fn room_gid(room: &campfire_db::Room) -> GlobalId {
    GlobalId::new(room.room_type.class_name(), room.id)
}

/// A thread's GlobalID (`gid://campfire/ChannelThread/1`).
pub fn thread_gid(thread_id: i64) -> GlobalId {
    GlobalId::new("ChannelThread", thread_id)
}

/// `ReadRoomsChannel`'s stream: the user's own stream of rooms read in another window.
pub fn read_rooms_stream_name(user_id: i64) -> String {
    format!("user_{user_id}_reads")
}
