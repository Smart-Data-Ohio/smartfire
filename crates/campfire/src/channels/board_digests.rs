//! Digest notes share rendering reads, while retaining one ordinary Turbo append per note.
use crate::app::App;
use crate::controllers::presenters::{Presenter, page};
use askama::Template;
use campfire_cable::turbo::{Action, Target};
use campfire_db::{Message, Room, models::board_automations::DigestNotes};
use std::collections::HashMap;

// Bound root-note lists below the supported 64-variable SQLite limit. Associations
// can exceed this batch size; their loaders bind one JSON list.
const BATCH_SIZE: usize = 32;

fn render_batch(app: &App, ids: &[i64]) -> campfire_db::Result<Vec<(i64, Room, String)>> {
    app.db.read_blocking(|conn| {
        let messages = Message::for_ids(conn, ids)?;
        let presenter = Presenter::new(conn, app, None).preload_broadcast(&messages)?;
        let messages: HashMap<_, _> = messages
            .iter()
            .map(|message| (message.id, message))
            .collect();
        let mut rendered = Vec::new();
        for id in ids {
            let Some(message) = messages.get(id) else {
                continue;
            };
            let result: campfire_db::Result<_> = (|| {
                let room = presenter
                    .search_preloads
                    .as_ref()
                    .and_then(|data| data.records.rooms.get(&message.room_id))
                    .cloned()
                    .ok_or(campfire_db::Error::RecordNotFound("Room"))?;
                let view = presenter.message(message)?;
                let html = page::render_detached_at(
                    app,
                    None,
                    page::default_renderer_base_url(app),
                    |ctx| {
                        campfire_views::messages::MessagePartial {
                            ctx,
                            message: &view,
                        }
                        .render()
                        .map_err(|error| campfire_db::Error::Other(error.to_string()))
                    },
                )?;
                Ok((*id, room, html))
            })();
            match result {
                Ok(note) => rendered.push(note),
                Err(error) => tracing::warn!(message_id=id,%error,"digest broadcast failed"),
            }
        }
        Ok(rendered)
    })
}

pub(crate) fn render(
    app: &App,
    notes: &DigestNotes,
) -> campfire_db::Result<Vec<(i64, Room, String)>> {
    let mut rendered = Vec::new();
    for ids in notes.message_ids.chunks(BATCH_SIZE) {
        match render_batch(app, ids) {
            Ok(batch) => rendered.extend(batch),
            Err(error) => {
                // A failed shared preload must not discard earlier batches or healthy
                // notes in this batch. Only the exceptional path falls back per note.
                tracing::warn!(%error,"digest preload failed; rendering notes individually");
                for id in ids {
                    match render_batch(app, std::slice::from_ref(id)) {
                        Ok(note) => rendered.extend(note),
                        Err(error) => {
                            tracing::warn!(message_id=id,%error,"digest broadcast failed")
                        }
                    }
                }
            }
        }
    }
    Ok(rendered)
}

pub(crate) fn deliver(
    cable: &super::Cable,
    app: &App,
    notes: &DigestNotes,
) -> anyhow::Result<Vec<i64>> {
    let mut published = Vec::new();
    for (id, room, html) in render(app, notes)? {
        if campfire_views::helpers::request_forgery::has_token_slots(&html) {
            tracing::warn!(
                message_id = id,
                "refusing unresolved CSRF token slots in a digest note"
            );
            continue;
        }
        let stream = campfire_db::broadcasts::room_messages(&room)
            .iter()
            .map(|stream| stream.to_param())
            .collect::<Vec<_>>();
        let stream = stream.iter().map(String::as_str).collect::<Vec<_>>();
        cable.broadcast_action_to(
            &stream,
            Action::Append,
            Target::Target(&campfire_db::broadcasts::room_dom_id(
                &room,
                Some("messages"),
            )),
            Some(&html),
            &[],
        );
        published.push(id);
    }
    Ok(published)
}
