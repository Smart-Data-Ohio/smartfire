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
pub mod board_digests;
mod github_notifier;
mod github_cards;
mod event_cards;
mod connection;
pub mod huddle_notice;
pub mod huddle_effects;
mod presence;
mod read_rooms;
pub mod revocation;
mod room;
pub mod sink;
pub mod threads;
mod typing_notifications;
pub mod unread_threads;
mod unread_rooms;
mod workspace_presence;

use campfire_cable::{Config, EmptyChannel, Server, ServerBuilder};
use campfire_db::Database;

pub use crate::cable::{Broadcasts, Cable, CableUser, broadcasts, room_gid, user_gid};
pub use connection::SessionAuthenticator;

/// What the cable server needs from the app.
#[derive(Clone)]
pub struct Deps {
    pub db: Database,
    pub crypto: campfire_kit::SharedCrypto,
    pub clock: campfire_kit::SharedClock,
    /// `config.x.admin_session_idle_timeout`
    pub admin_session_idle_timeout: jiff::SignedDuration,
}

/// The cable server with `ApplicationCable::Connection` and every channel registered. Mount it
/// with `cable.router("/cable")`.
pub fn server(deps: Deps, config: Config) -> Cable {
    let authenticator = SessionAuthenticator::new(deps.db.clone(), deps.crypto.clone(), deps.clock.clone(), deps.admin_session_idle_timeout);
    register(Server::builder(config, authenticator), &deps).build()
}

/// Registers the retained JSON channels under their protocol names.
pub fn register(builder: ServerBuilder<CableUser>, deps: &Deps) -> ServerBuilder<CableUser> {
    let db = &deps.db;
    let (presence_db, room_db, typing_db, workspace_db) = (db.clone(), db.clone(), db.clone(), db.clone());
    let idle_timeout = deps.admin_session_idle_timeout;
    builder
        .channel("ActivityChannel", || activity::ActivityChannel)
        .channel("AgentsChannel", || agents::AgentsChannel)
        .channel("ApplicationCable::Channel", || EmptyChannel)
        .channel("HeartbeatChannel", || EmptyChannel)
        .channel("HuddleNoticeChannel", || huddle_notice::HuddleNoticeChannel)
        .channel("PresenceChannel", move || presence::PresenceChannel::new(presence_db.clone()))
        .channel("ReadRoomsChannel", || read_rooms::ReadRoomsChannel)
        .channel("RoomChannel", move || room::RoomChannel::new(room_db.clone()))
        .channel("TypingNotificationsChannel", move || typing_notifications::TypingNotificationsChannel::new(typing_db.clone()))
        .channel("UnreadRoomsChannel", || unread_rooms::UnreadRoomsChannel)
        .channel("UnreadThreadsChannel", || unread_threads::UnreadThreadsChannel)
        .channel("WorkspacePresenceChannel", move || workspace_presence::WorkspacePresenceChannel::new(workspace_db.clone(), idle_timeout))

}
