//! The message controllers of the Campfire server: `MessagesController` with its boosts, pins and
//! bot endpoints, and the HTTP seams the message features share (finding the room and the
//! reachable message, redirects, time parsing). The crates above route to them
//! (plans/crate-split-plan.md). Modules keep their paths from the campfire crate, so
//! `controllers::messages` lives at `campfire_messages::controllers::messages`.

pub mod controllers {
    pub mod message_features;
    pub mod messages;

    // The web layer's presenters, under the path this code names them by.
    pub(crate) use campfire_web::controllers::presenters;
}

// The app, web and channels layers, under the paths this code used inside the campfire crate.
use campfire_app::{app, queue};
use campfire_runtime::{concerns, messaging, rich_text};
