//! Document tags for retained pages: the title and auth assets.

use super::html::{Html, raw};
use super::tag::{attrs, content_tag_text};

/// `page_title_tag`: `@page_title || "Smartfire"`.
pub fn page_title_tag(page_title: Option<&str>) -> Html {
    content_tag_text("title", attrs(), page_title.unwrap_or("Smartfire"))
}

/// The standalone auth stylesheet.
pub fn auth_stylesheet_tag() -> Html {
    raw(format!(
        "<link rel=\"stylesheet\" href=\"{}\">",
        campfire_static_assets::stylesheet_path("auth")
    ))
}

/// A blocking same-origin script restores appearance before the page paints.
pub fn auth_script_tag() -> Html {
    raw(format!(
        "<script src=\"{}\"></script>",
        campfire_static_assets::javascript_path("auth")
    ))
}

/// Popup dismissal for the unsupported-browser page. Separate from [`auth_script_tag`]: `auth.js`
/// uses syntax the browsers that page is shown to cannot parse, and one parse error drops it.
pub fn unsupported_script_tag() -> Html {
    raw(format!(
        "<script src=\"{}\"></script>",
        campfire_static_assets::javascript_path("unsupported")
    ))
}

/// The `id` of the auth layout's rejection message (its alert flash).
pub const AUTH_ALERT_ID: &str = "auth-alert";
