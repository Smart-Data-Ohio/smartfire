//! Message-list shell and uncached, per-viewer notices from our Rails room view.
use super::RoomView;
use crate::{ViewContext, helpers as h, messages::UserView};
use askama::Template;

#[derive(Clone, Debug, Default, serde::Deserialize, PartialEq)]
#[serde(default)]
pub struct State {
    pub unread_message_id: Option<i64>,
    pub unread_count: i64,
    pub unread_index: Option<usize>,
    pub scroll_to_divider: bool,
    pub jump_url: Option<String>,
    pub notices: Vec<Notice>,
}
#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub struct Notice {
    pub id: i64,
    pub name: String,
    pub until_date: Option<String>,
    pub note: Option<String>,
}
#[derive(Template)]
#[template(path = "messages/_template.html")]
pub struct Pending<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub user: &'a UserView,
}
#[derive(Template)]
#[template(path = "rooms/shell/_ooo.html")]
pub struct Notices<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub notices: &'a [Notice],
}
#[derive(Template)]
#[template(path = "rooms/show/_invitation.html")]
pub struct Invitation<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub join_code: &'a str,
}
mod filters {
    pub use crate::helpers::filters::*;
}

pub fn area(ctx: &ViewContext, room: &RoomView, scroll: bool, body: &str) -> h::Html {
    h::content_tag("div",h::attrs().id("message-area").class("message-area").attr("contents",true)
        .data("controller","messages presence drop-target")
        .data("action","turbo:before-stream-render@document->messages#beforeStreamRender keydown.up@document->messages#editMyLastMessage dragenter->drop-target#dragenter dragover->drop-target#dragover drop->drop-target#drop visibilitychange@document->presence#visibilityChanged")
        .data("messages_first_of_day_class","message--first-of-day")
        .data("messages_formatted_class","message--formatted")
        .data("messages_me_class","message--me")
        .data("messages_mentioned_class","message--mentioned")
        .data("messages_threaded_class","message--threaded")
        .data("messages_page_url_value",ctx.url(&h::routes::room_messages(room.id)))
        .attr_opt("data-messages-scroll-to-divider-value",scroll.then_some("true")),body)
}
pub fn list(
    ctx: &ViewContext,
    room: &RoomView,
    updated_at: jiff::Timestamp,
    body: &str,
) -> h::Html {
    h::content_tag("div",h::attrs().id(room.dom_id("messages")).class("messages")
        .attr("role","log").aria("live","polite").aria("relevant","additions")
        .data("controller","maintain-scroll refresh-room message-list")
        .data("action","turbo:before-stream-render@document->maintain-scroll#beforeStreamRender visibilitychange@document->refresh-room#visibilityChanged online@window->refresh-room#online")
        .data("messages_target","messages")
        .data("refresh_room_loaded_at_value",crate::messages::support::epoch_ms(updated_at))
        .data("refresh_room_url_value",ctx.url(&h::routes::room_refresh(room.id))),body)
}
pub fn preloads(ctx: &ViewContext) -> String {
    [
        "messages",
        "maintain_scroll",
        "reply",
        "composer",
        "markdown_editor",
        "typing_notifications",
        "local_time",
        "presence",
        "message_list",
        "header_overflow",
        "attach_menu",
    ]
    .iter()
    .map(|name| {
        h::builder_tag(
            "link",
            h::attrs()
                .attr("rel", "modulepreload")
                .attr(
                    "href",
                    ctx.asset(&format!("controllers/{name}_controller.js")),
                )
                .attr_opt("nonce", h::request_forgery::csp_nonce()),
        )
        .0
    })
    .collect::<Vec<_>>()
    .join("\n")
}
pub fn unread(count: impl std::borrow::Borrow<i64>) -> String {
    let count = count.borrow();
    format!(
        "<div id=\"unread-divider\" class=\"unread-divider\" data-unread-count=\"{count}\">\n  <span class=\"unread-divider__label\" aria-label=\"{count} new messages\">New messages</span>\n</div>\n"
    )
}
pub fn jump(ctx: &ViewContext, url: Option<&str>) -> h::Html {
    let label = format!(
        "{}<span>Jump to unread</span>",
        h::image_tag(ctx, "arrow-up.svg", h::attrs().aria_hidden().size(20)).0
    );
    let attrs = h::attrs()
        .id("jump-to-unread")
        .class("message-area__jump-to-unread btn");
    match url {
        Some(url) => h::link_to(url, attrs, &label),
        None => h::content_tag(
            "button",
            attrs
                .data("action", "messages#jumpToUnread")
                .attr("hidden", true),
            &label,
        ),
    }
}
