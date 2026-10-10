//! Digest publication IDs acknowledge successfully serialized JSON notes, including when idle.
use crate::app::App;
use campfire_db::{Message, models::board_automations::DigestNotes};

const BATCH_SIZE: usize = 32;

pub fn deliver(_cable: &super::Cable, app: &App, notes: &DigestNotes) -> anyhow::Result<Vec<i64>> {
    let mut published = Vec::new();
    for ids in notes.message_ids.chunks(BATCH_SIZE) {
        let batch = app.db.read_blocking(|conn| {
            let messages = Message::for_ids(conn, ids)?.into_iter().map(|message| (message.id, message)).collect::<std::collections::HashMap<_, _>>();
            let ordered = ids.iter().filter_map(|id| messages.get(id).cloned()).collect::<Vec<_>>();
            app.broadcasts.sync_digest_messages(conn, &ordered)
                .ok_or_else(|| campfire_db::Error::Other("digest JSON batch could not be serialized".into()))
        });
        match batch {
            Ok(ids) => published.extend(ids),
            Err(error) => {
                tracing::warn!(%error, "digest batch failed; retrying each note");
                for id in ids {
                    if app.db.read_blocking(|conn| {
                        let message = Message::find(conn, *id)?;
                        Ok(app.broadcasts.sync_digest_message(conn, &message))
                    }).unwrap_or(false) { published.push(*id); }
                }
            }
        }
    }
    Ok(published)
}
