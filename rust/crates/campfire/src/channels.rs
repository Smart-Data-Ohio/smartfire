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
use campfire_cable::{Config, EmptyChannel, Server, ServerBuilder};
use campfire_db::Database;
use rails_compat::Secrets;

pub use crate::cable::{Broadcasts, Cable, CableUser, broadcasts, room_gid, user_gid};
pub use connection::SessionAuthenticator;

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
