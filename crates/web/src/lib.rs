//! The web layer of the Campfire server: what every controller shares. The presenters that map
//! rows into view models, the request concerns and authentication, Active Storage's serving
//! side, inbound mail, attachment processing, rich text, and the message renderer the cable
//! broadcasts. The crates above build channels and controllers on it (plans/crate-split-plan.md).
//! Modules keep their paths from the campfire crate, so `controllers::presenters` lives at
//! `campfire_web::controllers::presenters`.

pub mod active_storage;
pub mod authentication;
pub mod concerns;
pub mod mail;
pub mod messaging;
pub mod rich_text;

pub mod controllers {
    pub mod presenters;
    pub mod messages {
        pub mod rendered;
    }
}

// The app layer, under the paths this code used inside the campfire crate.
use campfire_app::{cable, icons, integrations, queue, ruby, security, state};

mod app {
    pub(crate) use campfire_app::app::*;
    #[cfg(test)]
    pub(crate) use crate::asset_goldens;
}

// The shared page-parity comparison (`rich_text`'s tests read it as `app::asset_goldens`).
#[cfg(test)]
#[path = "../../../test-support/asset_goldens.rs"]
mod asset_goldens;
