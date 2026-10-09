//! `translation_button` takes a classic [`crate::ViewContext`] for its globe image.
//! The popup markup and the translation table live in `campfire_view_kit`.

pub use campfire_view_kit::helpers::translations::translations_for;

use super::assets::image_tag;
use super::html::Html;
use super::tag::attrs;
use crate::ViewContext;

/// `translation_button(key)`.
pub fn translation_button(ctx: &ViewContext, key: &str) -> Html {
    let globe = image_tag(
        ctx,
        "globe.svg",
        attrs().size(20).aria_hidden().class("color-icon"),
    );
    campfire_view_kit::helpers::translations::translation_popup(&globe.0, key)
}
