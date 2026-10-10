//! HTML helpers for retained server pages.
//!
//! This crate has no templates or layouts. Retained pages use their own assets and shells.

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
