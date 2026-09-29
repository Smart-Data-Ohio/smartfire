//! turbo-rails helpers: `turbo_frame_tag`, `turbo_stream_from`, `turbo_page_requires_reload`.

use super::html::Html;
use super::tag::{Attrs, attrs, builder_tag, content_tag};

/// `turbo_frame_tag(id, src:, target:, **attributes) { content }`: the attributes first, then
/// `id`, `src` and `target` (nil ones dropped).
pub fn turbo_frame_tag(id: &str, src: Option<&str>, target: Option<&str>, attributes: Attrs, content: &str) -> Html {
    let options = attributes.id(id).attr_opt("src", src).attr_opt("target", target);
    content_tag("turbo-frame", &options, content)
}

/// `turbo_stream_from(*streamables)`. The signed stream name comes from the caller
/// (`Turbo::StreamsChannel.signed_stream_name`).
pub fn turbo_stream_from(signed_stream_name: &str) -> Html {
    builder_tag(
        "turbo-cable-stream-source",
        attrs().attr("channel", "Turbo::StreamsChannel").attr("signed-stream-name", signed_stream_name),
    )
}

/// `turbo_page_requires_reload_tag`, which `turbo_page_requires_reload` provides to `:head`.
pub fn turbo_page_requires_reload_tag() -> Html {
    builder_tag("meta", attrs().name("turbo-visit-control").attr("content", "reload"))
}

/// `dom_id(record, prefix)`: "prefix_model_id".
pub fn dom_id(model: &str, id: impl std::fmt::Display, prefix: Option<&str>) -> String {
    match prefix {
        Some(prefix) => format!("{prefix}_{model}_{id}"),
        None => format!("{model}_{id}"),
    }
}
