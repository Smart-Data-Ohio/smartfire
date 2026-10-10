//! Classic page, fragment and broadcast adapters over the shared server runtime.
//! Service re-exports retain the paths used by the controller crates.

pub use campfire_runtime::active_storage;
pub use campfire_runtime::authentication;
pub use campfire_runtime::concerns;
pub mod mail;
pub use campfire_runtime::messaging;
pub use campfire_runtime::rich_text;

pub mod controllers {
    pub mod presenters;
    pub mod messages {
        pub mod rendered;
    }
}

// The app layer, under the paths this code used inside the campfire crate.
use campfire_app::{cable, integrations};

mod app {
    pub(crate) use campfire_app::app::*;
}
