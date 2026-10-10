//! Viewer-authorized quote frame and viewer-neutral card facts supplied by controllers.
use crate::{ViewContext, helpers as h};
use askama::Template;

pub fn cards(ctx: &ViewContext, message: &crate::messages::MessageView) -> h::Html {
    let Some(references) = &message.components.quote_references else {
        return crate::messages::cards(message, "message_link_cards", "message-link-cards", 0,
            &message.components.message_link_cards);
    };
    let mut bodies = Vec::new();
    for reference in references {
        let content = if let Some(card) = &reference.card {
            CardPartial { ctx, card }.render().expect("quote renders")
        } else {
            h::turbo_frame_tag(&format!("message_link_card_message_reference_{}", reference.id),
                Some(&format!("/rooms/{}/message_links/{}", message.room_id, reference.id)),
                None, h::attrs().attr("loading", "lazy").class("message-link-frame"), "").0
        };
        bodies.push(format!("\n    {content}\n"));
    }
    crate::messages::cards(message, "message_link_cards", "message-link-cards", 0, &bodies)
}
pub trait CardRendering {
    fn datetime(&self, ctx: &ViewContext) -> h::Html;

    fn html(&self, ctx: &ViewContext) -> String;
}

impl CardRendering for Card {
    fn datetime(&self, ctx: &ViewContext) -> h::Html {
        crate::time::local_datetime_tag(
            &ctx.time_zone,
            self.created_at,
            "time",
            h::attrs().class("message-quote__time"),
            "",
        )
    }
 fn html(&self,ctx:&ViewContext) -> String {CardPartial{ctx,card:self}.render().expect("quote renders")}

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
pub fn lazy(id: i64, room_id: i64) -> String {
    h::turbo_frame_tag(&format!("message_link_card_message_reference_{id}"),Some(&format!("/rooms/{room_id}/message_links/{id}")),None,h::attrs().attr("loading","lazy").class("message-link-frame"),"").0
}

pub use campfire_presentation::message_links::*;
