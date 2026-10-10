//! Shared server runtime, request support and template-free database presenters.
pub mod active_storage;
pub mod authentication;
pub mod concerns;
pub mod context;
pub mod mail;
pub mod messaging;
pub mod presenters;
pub mod request_context;
pub mod rich_text;
pub mod controllers {
    pub use crate::presenters;
}
use campfire_app::{cable, icons, integrations, queue, ruby, security, state};
mod app {
    #[cfg(test)]
    pub(crate) use crate::asset_goldens;
    pub(crate) use campfire_app::app::*;
}
#[cfg(test)]
#[path = "../../../test-support/asset_goldens.rs"]
mod asset_goldens;
