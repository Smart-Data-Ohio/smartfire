//! Event#broadcast_event_card_updates: ordered, bounded, viewer-neutral callbacks.
use super::broadcasts::{Stream, message_dom_id};
use crate::{
    app::App,
    controllers::presenters::events,
    integrations::message_batches::{self, Reference},
};
use campfire_cable::turbo::Action;
use campfire_db::Room;
use std::collections::{BTreeSet, HashMap};
pub fn publish(app: &App, event_id: i64) -> anyhow::Result<()> {
    let app = app.clone();
    app.db.clone().read_blocking(move |conn| {
        let mut after = None;
        loop {
            let messages = message_batches::next(conn, Reference::CalendarEvent(event_id), after)?;
            if messages.is_empty() {
                break;
            }
            let ids: Vec<_> = messages
                .iter()
                .map(|m| m.room_id)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            let rooms: HashMap<_, _> = Room::for_ids(conn, &ids)?
                .into_iter()
                .map(|r| (r.id, r))
                .collect();
            let cards = events::cards_for_messages(conn, &messages)?;
            for message in &messages {
                let room = rooms
                    .get(&message.room_id)
                    .ok_or(campfire_db::Error::RecordNotFound("Room"))?;
                app.broadcasts.turbo(
                    &Stream::conversation(room, message),
                    Action::Replace,
                    &message_dom_id(message, Some("event_cards")),
                    Some(&cards[&message.id]),
                    true,
                );
            }
            app.broadcasts.sync_message_cards(conn, &messages);
            if messages.len() < message_batches::SIZE {
                break;
            }
            after = messages.last().map(|m| m.id);
        }
        Ok(())
    })?;
    Ok(())
}
