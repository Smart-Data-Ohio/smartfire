//! `Rooms::RefreshesController` (reference/app/controllers/rooms/refreshes_controller.rb): what
//! changed in a room since the client last loaded it.

use askama::Template;
use campfire_db::{Message, Timeline, Timestamp};
use campfire_kit::{Ctx, Error, Result, StatusCode, format};
use campfire_views::rooms::{RefreshShow, RefreshView};

use crate::app::AppCtx;
use crate::concerns::{self, Before, before_actions, ruby_to_i};
use crate::controllers::presenters::page::{self, db_error};
use crate::controllers::presenters::{Presenter, room_kind};

pub async fn show(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (_, room) = concerns::set_room(c).await?;
    let last_updated_at = set_last_updated_at(c)?;

    let app = c.app().clone();
    let request_host = Some(c.request.host());
    let viewer_id = concerns::require_current_user(c)?.id;
    let refresh = c
        .app()
        .db
        .read(move |conn| {
            let new_messages = Message::page_created_since(conn, Timeline::Room(room.id), last_updated_at)?;
            let new_ids: Vec<i64> = new_messages.iter().map(|message| message.id).collect();
            let updated_messages = Message::page_updated_since(conn, Timeline::Room(room.id), last_updated_at, &new_ids)?;
            let pins_changed = room.pins_changed_at.is_some_and(|stamp| stamp > last_updated_at);
            if new_messages.is_empty() && updated_messages.is_empty() && !pins_changed { return Ok(None); }
            let mut presenter = Presenter::new(conn, &app, request_host);
            presenter.use_viewer_zone(viewer_id)?;
            let refresh = campfire_views::fragment_cache::with(&app.fragment_cache, || {
                Ok::<_, campfire_db::Error>(RefreshView {
                    room_id: room.id,
                    room_kind: room_kind(room.room_type),
                    new_messages: presenter.messages(&new_messages)?,
                    updated_messages: presenter.messages(&updated_messages)?,
                    pins: pins_changed.then(|| super::pins::list(conn,&app,&room)).transpose()?,
                })
            })?;
            Ok(Some((refresh, presenter.take_render_refreshes())))
        })
        .await
        .map_err(db_error)?;
    let Some((refresh, refreshes)) = refresh else { return Ok(c.head(StatusCode::NO_CONTENT)); };
    c.respond_to(&[&format::TURBO_STREAM])?;
    crate::controllers::presenters::refresh_after_render(&c.app().db, refreshes).await;
    page::bare(c, StatusCode::OK, &format::TURBO_STREAM, |ctx| RefreshShow { ctx, refresh: &refresh }.render()).await
}

/// `Time.at(0, params[:since].to_i, :millisecond)`
fn set_last_updated_at(c: &Ctx) -> Result<Timestamp> {
    let since = match c.param("since") {
        None => 0,
        Some(param) => match param.as_str() {
            Some(value) => ruby_to_i(value),
            None if param.is_null() => 0,
            // `to_i` isn't defined for a hash or an array.
            None => return Err(Error::internal(anyhow::anyhow!("undefined method 'to_i'"))),
        },
    };
    // Outside the representable range (a crafted `since`), the nearest end of it.
    let since = jiff::Timestamp::from_microsecond(since.saturating_mul(1000))
        .unwrap_or(if since < 0 { jiff::Timestamp::MIN } else { jiff::Timestamp::MAX });
    Ok(Timestamp::from_jiff(since))
}
