//! HTML adapter only. Shared containers contain no viewer data; payload frames are private.
use crate::{
    app::App,
    integrations::fizzy::cards::Card,
};
use campfire_db::Message;

/// Render the same owner frame body from already loaded shared card rows.
pub fn frames_from_cards(message: &Message, cards: &[Card]) -> Vec<String> {
    cards.iter().map(|card| {
        let src = campfire_routes::ROOM_FIZZY_CARD.path_with(&[&message.room_id, &card.id], None, &[("message_id", Some(&message.id.to_string()))]);
        format!("\n  <turbo-frame loading=\"lazy\" class=\"fizzy-card-frame\" id=\"card_for_message_{}_fizzy_card_{}\" src=\"{}\"></turbo-frame>\n",message.id,card.id,campfire_views::helpers::escape(&src))
    }).collect()
}
pub fn broadcast_updates(app: &App, card_id: i64) -> anyhow::Result<()> {
    let app = app.clone();
    app.db.clone().read_blocking(move |conn| {
        use crate::integrations::message_batches::{self, Reference};
        let mut after = None;
        loop {
            let messages = message_batches::next(conn, Reference::FizzyCard(card_id), after)?;
            app.broadcasts.sync_message_cards(conn, &messages);
            if messages.len() < message_batches::SIZE { break; }
            after = messages.last().map(|message| message.id);
        }
        Ok(())
    })?;
    Ok(())
}

#[cfg(any(test, feature = "test-support"))]
pub fn container(conn: &campfire_db::Connection, message: &Message) -> campfire_db::Result<String> {
    Ok(container_from_cards(
        message,
        &Card::for_message(conn, message.id)?,
    ))
}
fn container_from_cards(message: &Message, cards: &[Card]) -> String {
    format!(
        "<div id=\"{}\" class=\"fizzy-cards\">{}</div>\n",
        campfire_presentation::helpers::escape(&format!("fizzy_cards_message_{}", message.client_message_id)),
        frames_from_cards(message, cards).concat()
    )
}
