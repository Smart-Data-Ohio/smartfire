//! Shared value cache retained until its remaining callers move in PR 8.

pub mod fragment_cache;
pub mod helpers {
    pub use campfire_view_kit::helpers::request_forgery;
}
