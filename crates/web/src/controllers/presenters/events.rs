//! Preload event card facts once, with no request or viewer state in the provider.
use campfire_db::{Connection, Message, Result};

pub fn cards(conn: &Connection, message_id: i64) -> Result<String> {
    let message = Message::find(conn, message_id)?;
    Ok(cards_for_messages(conn, std::slice::from_ref(&message))?
        .remove(&message_id)
        .expect("requested message rendered"))
}

pub fn cards_for_messages(
    conn: &Connection,
    messages: &[Message],
) -> Result<std::collections::HashMap<i64, String>> {
    use askama::Template;
    let cards = for_messages(conn, messages)?;
    let zone = super::page::renderer_time_zone();
    messages
        .iter()
        .map(|message| {
            let events = cards
                .get(&message.id)
                .map(Vec::as_slice)
                .unwrap_or_default();
            let entries =
                campfire_views::events::card_entries(events, &message.id.to_string(), &zone);
            let html = campfire_views::events::Cards {
                message_key: &message.client_message_id,
                entries: &entries,
            }
            .render()
            .map_err(|error| campfire_db::Error::Other(error.to_string()))?;
            Ok((message.id, html))
        })
        .collect()
}

pub use campfire_runtime::presenters::events::*;
