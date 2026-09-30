//! WS8bm2 composition adapter over WS14e's published event partial.
use crate::{ViewContext, helpers as h};
use askama::Template;
pub fn cards(ctx: &ViewContext, message: &crate::messages::MessageView) -> h::Html {
    let Some(events) = &message.components.provider_events else {
        return crate::messages::event_cards(message);
    };
    let entries = crate::events::card_entries(events, &message.id.to_string(), &ctx.time_zone);
    h::raw(
        crate::events::Cards {
            message_key: &message.client_message_id,
            entries: &entries,
        }
        .render()
        .expect("event cards render"),
    )
}
