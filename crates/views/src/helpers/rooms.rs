//! The parts of `RoomsHelper` and `Rooms::InvolvementsHelper` the sidebar and the profile's
//! memberships use; the rest of the rooms helpers are in `crate::rooms`.

use super::assets::image_tag;
use super::forms::button_to;
use super::html::Html;
use super::links::link_to;
use super::tag::{Attrs, attrs, content_tag_text};
use super::turbo::dom_id;
use super::url::{Param, with_query};
use crate::ViewContext;

pub fn first_paint_controller_preloads(ctx: &ViewContext) -> Html {
    super::Safe(
        FIRST_PAINT_CONTROLLERS
            .iter()
            .map(|name| {
                super::builder_tag(
                    "link",
                    super::attrs()
                        .attr("rel", "modulepreload")
                        .attr(
                            "href",
                            ctx.asset(&format!("controllers/{name}_controller.js")),
                        )
                        .attr_opt("nonce", super::request_forgery::csp_nonce().as_deref()),
                )
                .0
            })
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

/// `link_to_room(room, **attributes) { content }`. `options` is the attribute hash in Ruby
/// order, `data-*` entries included where the `data:` key was.
pub fn link_to_room(room_id: i64, options: Attrs, content: &str) -> Html {
    let defaults = attrs()
        .data("rooms_list_target", "room")
        .data("room_id", room_id)
        .data("badge_dot_target", "unread")
        .data("sorted_list_target", "item");
    link_to(
        &campfire_routes::room(room_id),
        options.with_default_data(defaults),
        content,
    )
}

/// `button_to_change_involvement(room, involvement)`.
pub fn button_to_change_involvement<'r>(
    ctx: &ViewContext,
    room: impl std::borrow::Borrow<InvolvementRoom<'r>>,
    involvement: &str,
) -> Html {
    let room = room.borrow();
    let label_id = dom_id(room.param_key, room.id, Some("involvement_label"));
    let url = with_query(
        &campfire_routes::room_involvement(room.id),
        vec![(
            "involvement",
            Param::One(next_involvement(room.direct, involvement).to_string()),
        )],
    );
    let content = format!(
        "{}{}",
        image_tag(
            ctx,
            format!("notification-bell-{involvement}.svg"),
            attrs().aria_hidden().size(20)
        )
        .0,
        content_tag_text(
            "span",
            attrs().class("for-screen-reader").id(label_id.as_str()),
            humanize_involvement(involvement)
        )
        .0
    );
    let options = attrs()
        .method("put")
        .role("checkbox")
        .aria("checked", true)
        .aria("labelledby", label_id.as_str())
        .tabindex(0)
        .class(format!("btn {involvement}"));
    button_to(&url, options, &content)
}
pub use campfire_presentation::helpers::rooms::*;
