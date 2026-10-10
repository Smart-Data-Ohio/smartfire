//! Shared view helpers for the classic templates and the retained pages.
//!
//! This crate has no templates and no layouts. Deleting the classic stylesheet set, import map,
//! or `campfire_views` templates does not require editing the pages that stay server-rendered.

pub mod flash;
pub mod helpers;
pub mod sudo;

pub use context::{HelpContact, AccountSummary, CurrentUser, NotificationSounds, Platform, UserPreferences};

mod context;

/// `AllowBrowser::VERSIONS`, minus the browsers it blocks outright (`ie: false`).
pub const ALLOW_BROWSER_VERSIONS: [(&str, &str); 4] = [
    ("safari", "17.2"),
    ("chrome", "120"),
    ("firefox", "121"),
    ("opera", "104"),
];
