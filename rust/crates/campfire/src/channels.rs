//! Action Cable: `reference/app/channels`, the broadcasts the app makes (`Broadcasts`), and
//! revocation (`revocation`). `app.rs` builds the cable server with these channels and routes the
//! models' events to `Broadcasts` and `revocation`.
//!
//! Every channel matches its Ruby class: identifier, streams, payloads, and callback order
//! (`on_subscribe` runs after `subscribed`, before the confirmation). Ruby makes every public
//! method an action, including a publicly redefined `subscribed`, so those are performable here
//! too.

pub mod activity;
pub mod agents;
pub mod broadcasts;
pub(crate) mod board_digests;
pub(crate) mod message_features;
mod github_notifier;
mod github_cards;
mod event_cards;
mod connection;
pub mod huddle_notice;
pub(crate) mod huddle_effects;
#[cfg(test)]
mod huddle_effects_tests;
mod presence;
mod read_rooms;
pub mod revocation;
mod room;
mod room_messages;
mod rooms_directory;
pub mod sink;
mod room_composition;
pub mod threads;
mod typing_notifications;
pub mod unread_threads;
mod unread_rooms;
mod workspace_presence;

#[cfg(test)]
pub(crate) mod tests;

use std::sync::Arc;

use campfire_cable::turbo::{STREAMS_CHANNEL, StreamsChannel};
use campfire_cable::{Config, EmptyChannel, Identified, Server, ServerBuilder};
use campfire_db::Database;
use rails_compat::Secrets;
use rails_compat::global_id::GlobalId;

pub use broadcasts::{Broadcasts, Partials};
pub use connection::SessionAuthenticator;

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

/// What the cable server needs from the app.
#[derive(Clone)]
pub struct Deps {
    pub db: Database,
    pub secrets: Arc<Secrets>,
    pub crypto: campfire_kit::SharedCrypto,
    pub clock: campfire_kit::SharedClock,
    /// `config.x.admin_session_idle_timeout`
    pub admin_session_idle_timeout: jiff::SignedDuration,
}

/// The cable server with `ApplicationCable::Connection` and every channel registered. Mount it
/// with `cable.router("/cable")`.
pub fn server(deps: Deps, config: Config) -> Cable {
    let authenticator = SessionAuthenticator::new(deps.db.clone(), deps.crypto.clone(), deps.clock.clone(), deps.admin_session_idle_timeout);
    register(Server::builder(config, authenticator), &deps, StreamsChannel::new(deps.secrets.clone())).build()
}

/// Registers the channels under their Ruby class names. `streams` verifies signed stream names
/// (`Turbo.signed_stream_verifier`); `RoomMessagesChannel` verifies with it too, and
/// `Turbo::StreamsChannel` gets it with `RoomStreamsAreAuthorized` prepended.
pub fn register(builder: ServerBuilder<CableUser>, deps: &Deps, streams: StreamsChannel) -> ServerBuilder<CableUser> {
    let db = &deps.db;
    let (presence_db, room_db, messages_db, typing_db, workspace_db) = (db.clone(), db.clone(), db.clone(), db.clone(), db.clone());
    let idle_timeout = deps.admin_session_idle_timeout;
    let stock = streams.clone().guarded_by(room_messages::guarded_stream);
    builder
        .channel("ActivityChannel", || activity::ActivityChannel)
        .channel("AgentsChannel", || agents::AgentsChannel)
        .channel("ApplicationCable::Channel", || EmptyChannel)
        .channel("HeartbeatChannel", || EmptyChannel)
        .channel("HuddleNoticeChannel", || huddle_notice::HuddleNoticeChannel)
        .channel("PresenceChannel", move || presence::PresenceChannel::new(presence_db.clone()))
        .channel("ReadRoomsChannel", || read_rooms::ReadRoomsChannel)
        .channel("RoomChannel", move || room::RoomChannel::new(room_db.clone()))
        .channel("RoomMessagesChannel", move || room_messages::RoomMessagesChannel::new(messages_db.clone(), streams.clone()))
        .channel("TypingNotificationsChannel", move || typing_notifications::TypingNotificationsChannel::new(typing_db.clone()))
        .channel("UnreadRoomsChannel", || unread_rooms::UnreadRoomsChannel)
        .channel("UnreadThreadsChannel", || unread_threads::UnreadThreadsChannel)
        .channel("WorkspacePresenceChannel", move || workspace_presence::WorkspacePresenceChannel::new(workspace_db.clone(), idle_timeout))
        .channel(STREAMS_CHANNEL, move || stock.clone())
}

pub fn user_gid(user_id: i64) -> GlobalId {
    GlobalId::new("User", user_id)
}

/// A room's GlobalID names its STI class (`gid://campfire/Rooms::Open/1`).
pub fn room_gid(room: &campfire_db::Room) -> GlobalId {
    GlobalId::new(room.room_type.class_name(), room.id)
}
