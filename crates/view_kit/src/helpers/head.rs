//! Document tags both shells print: the title, the auth assets, and the turbo reload marker.

use super::html::{Html, raw};
use super::tag::{attrs, builder_tag, content_tag_text};

/// `page_title_tag`: `@page_title || "Smartfire"`.
pub fn page_title_tag(page_title: Option<&str>) -> Html {
    content_tag_text("title", attrs(), page_title.unwrap_or("Smartfire"))
}

/// The standalone auth stylesheet, deliberately outside the classic stylesheet set.
pub fn auth_stylesheet_tag() -> Html {
    raw(format!(
        "<link rel=\"stylesheet\" href=\"{}\">",
        campfire_assets::stylesheet_path("auth")
    ))
}

/// A blocking same-origin script restores appearance before the page paints.
pub fn auth_script_tag() -> Html {
    raw(format!(
        "<script src=\"{}\"></script>",
        campfire_assets::javascript_path("auth")
    ))
}

/// The `id` of the auth layout's rejection message (its alert flash).
pub const AUTH_ALERT_ID: &str = "auth-alert";

/// `turbo_page_requires_reload_tag`, which `turbo_page_requires_reload` provides to `:head`.
pub fn turbo_page_requires_reload_tag() -> Html {
    builder_tag(
        "meta",
        attrs()
            .name("turbo-visit-control")
            .attr("content", "reload"),
    )
}
