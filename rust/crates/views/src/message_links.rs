//! Viewer-authorized quote frame and viewer-neutral card facts supplied by controllers.
use crate::{ViewContext, helpers as h};
use askama::Template;
pub struct Card {
    pub author: String,
    pub room_label: String,
    pub excerpt: String,
    pub created_at: jiff::Timestamp,
    pub message_path: String,
}
impl Card {
    pub fn datetime(&self, ctx: &ViewContext) -> h::Html {
        crate::time::local_datetime_tag(
            &ctx.time_zone,
            self.created_at,
            "time",
            h::attrs().class("message-quote__time"),
            "",
        )
    }
}
#[derive(Template)]
#[template(path = "messages/message_links/_card.html")]
pub struct CardPartial<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub card: &'a Card,
}
#[derive(Template)]
#[template(path = "rooms/message_links/show.html")]
pub struct Frame<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub reference_id: i64,
    pub card: Option<&'a Card>,
}
impl Frame<'_> {
    pub fn html(&self) -> h::Html {
        let content = if let Some(card) = self.card {
            CardPartial {
                ctx: self.ctx,
                card,
            }
            .render()
            .expect("quote card renders")
        } else {
            "<span class=\"message-quote-private\">Message in a private room</span>\n".into()
        };
        h::turbo_frame_tag(
            &format!("message_link_card_message_reference_{}", self.reference_id),
            None,
            None,
            h::attrs().class("message-link-frame"),
            &format!("\n    {content}\n"),
        )
    }
}
