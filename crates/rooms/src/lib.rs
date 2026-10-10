//! The room controllers of the Campfire server: `RoomsController` and its subclasses (opens,
//! closeds, directs, members, boards, events, calls, huddles and the rest), plus the sidebar's
//! `RoomCategoriesController`. The crates above route to them (plans/crate-split-plan.md).
//! Modules keep their paths from the campfire crate, so `controllers::rooms` lives at
//! `campfire_rooms::controllers::rooms`.

pub mod controllers {
    pub mod room_categories;
    pub mod rooms;

    // The layers below, under the paths this code names them by.
    pub(crate) use campfire_messages::controllers::{message_features, messages};
    pub(crate) use campfire_runtime::presenters;
}

// The app, web and channels layers, under the paths this code used inside the campfire crate.
use campfire_app::app;
use campfire_channels::channels;
use campfire_runtime::{concerns, messaging, rich_text};
