//! Render WS8bm2's domain descriptions on the committing thread, preserving callback order.
use crate::app::App;
use crate::controllers::presenters::{Presenter, page};
use askama::Template;
use campfire_db::broadcasts::{Broadcast, Partial, TurboAction};
use std::cell::{Cell, RefCell};

thread_local! { pub static SLASH: Cell<bool> = const { Cell::new(false) }; }
thread_local! { pub static ORIGIN: RefCell<Option<String>> = const { RefCell::new(None) }; }

/// The writer closure carries the request's renderer origin. A guard restores the previous
/// origin on errors and panics; jobs use Rails' configured default URL origin.
pub struct OriginGuard(Option<String>);
impl Drop for OriginGuard {
    fn drop(&mut self) {
        ORIGIN.with(|origin| origin.replace(self.0.take()));
    }
}
pub fn origin(origin: &str) -> OriginGuard {
    OriginGuard(ORIGIN.with(|value| value.replace(Some(origin.to_owned()))))
}

pub struct SlashGuard {
    _origin: OriginGuard,
    previous: bool,
}
impl Drop for SlashGuard {
    fn drop(&mut self) {
        SLASH.with(|flag| flag.set(self.previous));
    }
}
pub fn slash_origin(base: &str) -> SlashGuard {
    SlashGuard {
        _origin: origin(base),
        previous: SLASH.with(|flag| flag.replace(true)),
    }
}

pub(crate) fn deliver(
    cable: &super::Cable,
    app: &App,
    broadcast: &Broadcast,
) -> anyhow::Result<bool> {
    let Broadcast::Turbo(frame) = broadcast else {
        return Ok(false);
    };
    let Some(partial) = &frame.partial else {
        return Ok(false);
    };
    // The general message/threads/quotes adapter remains owned by WS8b-m. The quiet pin note and
    // scheduled sends are consumed here, using that worker's unchanged message presenter/partial.
    if let Partial::Message { message_id } = partial {
        let note = app
            .db
            .read_blocking(|conn| campfire_db::Message::find(conn, *message_id))?;
        let pin_note = note.system_note
            && note
                .markdown_source
                .as_deref()
                .is_some_and(|source| source.starts_with("pinned a message: [jump to message]("));
        let scheduled = app.db.read_blocking(|conn| {
            Ok(conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM scheduled_messages WHERE sent_message_id = ?)",
                [note.id],
                |row| row.get::<_, bool>(0),
            )?)
        })?;
        if !pin_note && !scheduled && !SLASH.with(|flag| flag.get()) {
            return Ok(false);
        }
    } else if !matches!(
        partial,
        Partial::Poll { .. }
            | Partial::QuoteCards { .. }
            | Partial::PinBadge { .. }
            | Partial::PinsCount { .. }
            | Partial::PinsList { .. }
    ) {
        return Ok(false);
    }
    let origin = ORIGIN
        .with(|value| value.borrow().clone())
        .unwrap_or_else(|| page::default_renderer_base_url(app).into());
    let zone = page::renderer_time_zone();
    let html = app.db.read_blocking(|conn| {
        let _zone = page::enter_time_zone(zone.clone());
        let account = campfire_db::Account::first(conn)?;
        let presenter = Presenter::new(conn, app, None);
        let rendered = page::render_detached_at(
            app,
            account.as_ref(),
            &origin,
            |ctx| -> campfire_db::Result<String> {
                let rendered = match partial {
                    // WS8bm2 owning-partial seam, including request-free refresh jobs.
                    Partial::QuoteCards { message_id } => {
                        let view = presenter.message(&campfire_db::Message::find(conn, *message_id)?)?;
                        campfire_views::message_links::cards(ctx, &view).0
                    }
                    Partial::Poll { poll_id } => {
                        campfire_views::messages::parts::poll(
                            ctx,
                            &crate::controllers::presenters::message_parts::poll_view(
                                conn, app, *poll_id, None,
                            )?,
                        )
                        .0
                    }
                    Partial::PinBadge { message_id } => {
                        let message = campfire_db::Message::find(conn, *message_id)?;
                        let message = campfire_views::pins::Badge::new(
                            message.client_message_id,
                            campfire_db::MessagePin::pinned(conn, *message_id)?,
                        );
                        campfire_views::pins::BadgePartial {
                            ctx,
                            message: &message,
                        }
                        .render()
                        .map_err(|error| campfire_db::Error::Other(error.to_string()))?
                    }
                    Partial::PinsCount { room_id } => {
                        let room = campfire_db::Room::find(conn, *room_id)?;
                        campfire_views::pins::CountPartial {
                            room_id: room.id,
                            room_param_key: campfire_db::broadcasts::room_param_key(room.room_type),
                            count: campfire_db::MessagePin::count_for_room(conn, *room_id)?,
                        }
                        .render()
                        .map_err(|error| campfire_db::Error::Other(error.to_string()))?
                    }
                    Partial::PinsList { room_id } => {
                        let room = campfire_db::Room::find(conn, *room_id)?;
                        let list = crate::controllers::presenters::pins::list(conn, app, &room)?;
                        campfire_views::pins::ListPartial { ctx, list: &list }
                            .render()
                            .map_err(|error| campfire_db::Error::Other(error.to_string()))?
                    }
                    Partial::Message { message_id } => {
                        let message =
                            presenter.message(&campfire_db::Message::find(conn, *message_id)?)?;
                        campfire_views::messages::message(ctx, &message)
                    }
                    _ => unreachable!("filtered above"),
                };
                Ok(rendered)
            },
        )?;
        Ok(rendered)
    })?;
    let action = match frame.action {
        TurboAction::Append => campfire_cable::turbo::Action::Append,
        TurboAction::Prepend => campfire_cable::turbo::Action::Prepend,
        TurboAction::Replace => campfire_cable::turbo::Action::Replace,
        TurboAction::Update => campfire_cable::turbo::Action::Update,
        TurboAction::Remove => campfire_cable::turbo::Action::Remove,
    };
    let attributes = if frame.maintain_scroll {
        vec![("maintain_scroll", Some("true"))]
    } else {
        vec![]
    };
    let html = campfire_cable::turbo::action_tag(
        action,
        campfire_cable::turbo::Target::Target(&frame.target),
        Some(&html),
        &attributes,
    );
    cable.broadcast_stream_to(&[&broadcast.stream_name()], &html);
    // The JSON twins: a quiet note or scheduled/slash post consumed here is `message.created`
    // (or `message.updated`) as in `messaging`, and a pin badge is `message.pinned`.
    if cable.sync_wanted() {
        let twinned = app.db.read_blocking(|conn| {
            match partial {
                Partial::Message { message_id } if matches!(frame.action, TurboAction::Append | TurboAction::Replace) => {
                    if let Some(message) = campfire_db::Message::find_by_id(conn, *message_id)? {
                        app.broadcasts.sync_message(conn, &message, frame.action == TurboAction::Append);
                    }
                }
                Partial::PinBadge { message_id } => crate::cable::sync::message_pinned(cable, conn, *message_id),
                _ => {}
            }
            Ok(())
        });
        if let Err(error) = twinned {
            tracing::warn!(%error, "sync: message feature twin not read");
        }
    }
    Ok(true)
}
