//! `image_tag` and asset resolution (`AssetTagHelper`, `AssetUrlHelper`).

use super::html::Html;
use super::tag::{Attrs, Value, legacy_tag};
use crate::ViewContext;

/// `asset_path(source)`: URLs and absolute paths pass through; logical asset paths are digested.
pub fn asset_path(ctx: &ViewContext, source: &str) -> String {
    let is_url = source.starts_with('/')
        || source.starts_with("data:")
        || source.starts_with("cid:")
        || source.split_once("://").is_some_and(|(scheme, _)| {
            !scheme.is_empty() && scheme.chars().all(|c| c.is_ascii_alphabetic() || c == '-')
        });
    if is_url { source.to_string() } else { ctx.asset(source) }
}

/// `image_tag(source, options)`: the options in order, then `src`, then `width`/`height` from
/// `size:` ("20" or "20x30").
pub fn image_tag(ctx: &ViewContext, source: impl std::fmt::Display, options: Attrs) -> Html {
    let mut options = options;
    let size = options.remove("size");
    options.set("src", Some(asset_path(ctx, &source.to_string()).into()));
    if let Some(size) = size {
        let size = match size {
            Value::Text(text) | Value::Safe(text) => text,
            Value::Bool(flag) => flag.to_string(),
        };
        let (width, height) = match size.split_once('x') {
            Some((width, height)) => (width.to_string(), height.to_string()),
            None => (size.clone(), size),
        };
        options.set("width", Some(width.into()));
        options.set("height", Some(height.into()));
    }
    legacy_tag("img", &options)
}

/// The standalone auth stylesheet, deliberately outside the classic stylesheet set.
pub fn auth_stylesheet_tag() -> Html {
    super::raw(format!("<link rel=\"stylesheet\" href=\"{}\">", campfire_assets::stylesheet_path("auth")))
}

/// A blocking same-origin script restores appearance before the page paints.
pub fn auth_script_tag() -> Html {
    super::raw(format!("<script src=\"{}\"></script>", campfire_assets::javascript_path("auth")))
}

/// The `id` of the auth layout's rejection message (its alert flash).
pub const AUTH_ALERT_ID: &str = "auth-alert";

/// A field the auth page's rejection is about: described by the alert and marked invalid while
/// one shows; unchanged otherwise.
pub fn rejected_field(ctx: &ViewContext, attrs: Attrs) -> Attrs {
    if ctx.flash_alert().is_some() {
        attrs.attr("aria-describedby", AUTH_ALERT_ID).attr("aria-invalid", "true")
    } else {
        attrs
    }
}
