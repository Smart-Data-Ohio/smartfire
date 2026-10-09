//! turbo-rails helpers: `turbo_frame_tag`, `turbo_stream_from`, `turbo_page_requires_reload`.

use super::html::Html;
use super::tag::{Attrs, attrs, builder_tag, content_tag};

/// `turbo_frame_tag(id, src:, target:, **attributes) { content }`: the attributes first, then
/// `id`, `src` and `target` (nil ones dropped).
pub fn turbo_frame_tag(
    id: &str,
    src: Option<&str>,
    target: Option<&str>,
    attributes: Attrs,
    content: &str,
) -> Html {
    let options = attributes
        .id(id)
        .attr_opt("src", src)
        .attr_opt("target", target);
    content_tag("turbo-frame", &options, content)
}

/// `turbo_stream_from(*streamables)`. The signed stream name comes from the caller
/// (`Turbo::StreamsChannel.signed_stream_name`).
pub fn turbo_stream_from(signed_stream_name: &str) -> Html {
    builder_tag(
        "turbo-cable-stream-source",
        attrs()
            .attr("channel", "Turbo::StreamsChannel")
            .attr("signed-stream-name", signed_stream_name),
    )
}

/// `turbo_stream_from(*streamables)`, signing the names with the app's secret
/// ([`crate::ViewContext::signed_stream_name`]). A record is its [`gid_param`], a symbol itself:
/// `turbo_stream_from Current.user, :rooms` is `&[&gid_param("User", id), "rooms"]`.
pub fn turbo_stream_from_streamables(ctx: &crate::ViewContext, streamables: &[&str]) -> Html {
    turbo_stream_from(&(ctx.signed_stream_name)(streamables))
}

/// `record.to_gid_param`: `gid://campfire/<Model>/<id>` in unpadded URL-safe Base64
/// (`GlobalID#to_param`).
pub fn gid_param(model: &str, id: impl std::fmt::Display) -> String {
    let encoded =
        super::application::base64_url::urlsafe_encode64(&format!("gid://campfire/{model}/{id}"));
    encoded.trim_end_matches('=').to_string()
}

pub use campfire_view_kit::helpers::head::turbo_page_requires_reload_tag;

/// `dom_id(record, prefix)`: "prefix_model_id".
pub fn dom_id(model: &str, id: impl std::fmt::Display, prefix: Option<&str>) -> String {
    match prefix {
        Some(prefix) => format!("{prefix}_{model}_{id}"),
        None => format!("{model}_{id}"),
    }
}
