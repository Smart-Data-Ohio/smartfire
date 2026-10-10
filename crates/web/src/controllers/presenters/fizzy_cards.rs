//! HTML adapter only. Shared containers contain no viewer data; payload frames are private.
use crate::{
    app::App,
    cable::broadcasts::{Stream, message_dom_id},
    integrations::fizzy::cards::Card,
};
use campfire_db::{Message, Room};

/// Render the same owner frame body from already loaded shared card rows.
pub fn frames_from_cards(message: &Message, cards: &[Card]) -> Vec<String> {
    cards.iter().map(|card| {
        let src = campfire_routes::ROOM_FIZZY_CARD.path_with(&[&message.room_id, &card.id], None, &[("message_id", Some(&message.id.to_string()))]);
        format!("\n  <turbo-frame loading=\"lazy\" class=\"fizzy-card-frame\" id=\"card_for_message_{}_fizzy_card_{}\" src=\"{}\"></turbo-frame>\n",message.id,card.id,campfire_views::helpers::escape(&src))
    }).collect()
}
pub fn broadcast_updates(app: &App, card_id: i64) -> anyhow::Result<()> {
    let app2 = app.clone();
    app.db.read_blocking(move |conn| {
        use crate::integrations::message_batches::{self, Reference};
        let mut after = None;
        loop {
            let messages = message_batches::next(conn, Reference::FizzyCard(card_id), after)?;
            let ids: Vec<_> = messages.iter().map(|m| m.id).collect();
            let room_ids: Vec<_> = messages
                .iter()
                .map(|m| m.room_id)
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();
            let rooms: std::collections::HashMap<_, _> = Room::for_ids(conn, &room_ids)?
                .into_iter()
                .map(|r| (r.id, r))
                .collect();
            let cards = Card::for_messages(conn, &ids)?;
            for message in &messages {
                let room = rooms
                    .get(&message.room_id)
                    .ok_or(campfire_db::Error::RecordNotFound("Room"))?;
                let html = container_from_cards(
                    message,
                    cards
                        .get(&message.id)
                        .map(Vec::as_slice)
                        .unwrap_or_default(),
                );
                app2.broadcasts.turbo(
                    &Stream::conversation(room, message),
                    campfire_cable::turbo::Action::Replace,
                    &message_dom_id(message, Some("fizzy_cards")),
                    Some(&html),
                    true,
                );
            }
            app2.broadcasts.sync_message_cards(conn, &messages);
            if messages.len() < message_batches::SIZE {
                break;
            }
            after = messages.last().map(|m| m.id);
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
        campfire_presentation::helpers::escape(&message_dom_id(message, Some("fizzy_cards"))),
        frames_from_cards(message, cards).concat()
    )
}
