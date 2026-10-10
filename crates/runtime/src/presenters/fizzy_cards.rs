use campfire_app::app::App;

pub fn broadcast_updates(app: &App, card_id: i64) -> anyhow::Result<()> {
    let app = app.clone();
    app.db.clone().read_blocking(move |conn| {
        use crate::integrations::message_batches::{self, Reference};
        let mut after = None;
        loop {
            let messages = message_batches::next(conn, Reference::FizzyCard(card_id), after)?;
            app.broadcasts.sync_message_cards(conn, &messages);
            if messages.len() < message_batches::SIZE {
                break;
            }
            after = messages.last().map(|message| message.id);
        }
        Ok(())
    })?;
    Ok(())
}
