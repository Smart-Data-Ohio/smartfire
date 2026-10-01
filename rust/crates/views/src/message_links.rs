//! Session-independent quote children of the shared message renderer.
use crate::{ViewContext, helpers as h};
use askama::Template;

pub fn lazy(reference_id: i64, room_id: i64) -> String {
    h::content_tag(
        "turbo-frame",
        h::attrs()
            .attr("loading", "lazy")
            .class("message-link-frame")
            .id(format!(
                "message_link_card_message_reference_{reference_id}"
            ))
            .attr(
                "src",
                format!("/rooms/{room_id}/message_links/{reference_id}"),
            ),
        "",
    )
    .0
}
#[derive(Template)]
#[template(path = "messages/message_links/_card.html")]
pub struct Card<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub author: &'a str,
    pub room_label: &'a str,
    pub created_at: jiff::Timestamp,
    pub plain_text: &'a str,
    pub path: &'a str,
}
impl Card<'_> {
    /// The ERB child partial retains its final newline inside the cards container.
    pub fn html(&self) -> String {
        format!("{}\n", self.render().expect("quote card renders"))
    }

    fn time(&self) -> h::Html {
        h::local_datetime_tag(
            &crate::time::Zone::utc(),
            self.created_at,
            "time",
            h::attrs().class("message-quote__time"),
            "",
        )
    }
    fn excerpt(&self) -> String {
        h::truncate(self.plain_text, 200, "...")
    }
    fn jump(&self) -> h::Html {
        h::link_to(
            &self.ctx.url(self.path),
            h::attrs()
                .class("message-quote__jump")
                .data("turbo_frame", "_top"),
            "Jump to message",
        )
    }
}
