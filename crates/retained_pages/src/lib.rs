//! Pages that stay server-rendered after the classic UI goes: sign-in, join, first run,
//! sudo, two-factor, the public pages, and the unsupported-browser rejection.
//!
//! Their shell, templates and request context live here so deleting the classic layout,
//! import map and stylesheet set does not require editing these pages. Token and font
//! files stay in `frontend/src` and `crates/assets/auth`; [`auth_build`] (compiled by the
//! assets build) turns them into the same `auth.css` / `auth.js` the digest pipeline serves.

pub mod first_runs;
pub mod helpers;
pub mod layouts;
pub mod public_pages;
pub mod sessions;
pub mod sudos;
pub mod two_factor;
pub mod users;

use campfire_views::{AccountSummary, CurrentUser, Platform};

/// Service-worker bits the auth shell prints. Nothing here names an import map or the
/// classic stylesheet set.
#[derive(Clone, Debug, Default)]
pub struct Chrome {
    pub service_worker_auto_register: bool,
    pub service_worker_url: Option<String>,
}

/// What a retained page actually reads. Callers fill it from the request; rendering it
/// does not build an import map or the classic stylesheet tags.
pub struct Context<'a> {
    pub current_user: Option<CurrentUser>,
    pub account: AccountSummary,
    pub flash_notice: Option<String>,
    pub flash_alert: Option<String>,
    pub custom_styles: Option<String>,
    pub platform: Platform,
    pub app_version: String,
    pub base_url: String,
    pub asset_path: &'a dyn Fn(&str) -> String,
    pub chrome: Chrome,
}

impl Context<'_> {
    /// Marks the lazy flash as read, the same way the classic context does, so a rendered
    /// notice or alert is swept at the end of the request.
    pub fn flash_notice(&self) -> Option<&String> {
        campfire_views::flash::read();
        self.flash_notice.as_ref()
    }

    pub fn flash_alert(&self) -> Option<&String> {
        campfire_views::flash::read();
        self.flash_alert.as_ref()
    }

    pub fn asset(&self, logical_path: &str) -> String {
        (self.asset_path)(logical_path)
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }
}
