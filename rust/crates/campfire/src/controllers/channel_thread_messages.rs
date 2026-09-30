//! Nested message endpoints (ChannelThreadMessagesController).
#[cfg(test)]
mod tests;

use askama::Template;
use campfire_db::{ChannelThread, Message, Room, Timeline};
use campfire_kit::{Ctx, Error, Result, StatusCode, format};
use serde_json::{Value, json};
use crate::app::AppCtx;
use crate::concerns::{self, Before, before_actions, cast_integer, require_current_user};
use crate::controllers::{messages, presenters::page::{self, db_error}};

async fn scope(c: &mut Ctx) -> Result<(Room, ChannelThread)> {
    let (_, room) = concerns::set_room(c).await?;
    if room.deleted_at.is_some() { return Err(Error::NotFound); }
    let id = c.param_str("thread_id").and_then(cast_integer).ok_or(Error::NotFound)?;
    let room_id = room.id;
    let thread = c.app().db.read(move |conn| {
        let thread = ChannelThread::find(conn, id)?;
        if thread.room_id != room_id { return Err(campfire_db::Error::RecordNotFound("ChannelThread")); }
        Ok(thread)
    }).await.map_err(db_error)?;
    Ok((room, thread))
}

async fn set_message(c: &Ctx, thread: &ChannelThread) -> Result<Message> {
    let id = c.param_str("id").and_then(cast_integer).ok_or(Error::NotFound)?;
    let thread_id = thread.id;
    c.app().db.read(move |conn| Message::find_in(conn, Timeline::Thread(thread_id), id)).await.map_err(db_error)
}

pub async fn index(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (_, thread) = scope(c).await?;
    let timeline = Timeline::Thread(thread.id);
    let before = c.params.get("before").filter(|value| value.is_present()).cloned();
    let after = c.params.get("after").filter(|value| value.is_present()).cloned();
    let records = c.app().db.read(move |conn| match (before, after) {
        (Some(value), _) => Message::page_before(conn, timeline, &messages::paging_anchor(conn, timeline, &value)?),
        (None, Some(value)) => Message::page_after(conn, timeline, &messages::paging_anchor(conn, timeline, &value)?),
        _ => Message::last_page(conn, timeline),
    }).await.map_err(db_error)?;
    if c.format()? == Some(&format::JSON) { c.no_store(); }
    if *c.respond_to(&[&format::HTML, &format::JSON])? == format::JSON {
        let viewer = require_current_user(c)?.clone();
        let base = c.url_for("");
        let payload = messages::present(c, move |p| {
            let records = records.iter().map(|message| messages::payload::thread_message(p, message, &viewer, &base)).collect::<campfire_db::Result<Vec<_>>>()?;
            Ok(json!({"messages": records}))
        }).await?;
        return render_json(c, &payload);
    }
    let items = messages::present(c, move |p| p.messages(&records)).await?;
    let response = page::content(c, StatusCode::OK, |ctx| campfire_views::messages::Index { ctx, messages: &items }.render()).await?;
    let fragments = campfire_views::messages::MessageItem::cached_fragments(&c.app().fragment_cache, &items, &c.url_for(""));
    Ok(response.with_cached_fragments(fragments))
}

pub async fn show(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (room, thread) = scope(c).await?;
    let message = set_message(c, &thread).await?;
    if c.format()? == Some(&format::JSON) { c.no_store(); }
    if *c.respond_to(&[&format::HTML, &format::JSON])? == format::HTML {
        return c.redirect_to(&c.url_for(&format!("/rooms/{}?message_id={}&thread={}", room.id, message.id, thread.id)));
    }
    let viewer = require_current_user(c)?.clone();
    let base = c.url_for("");
    let payload = messages::present(c, move |p| messages::payload::thread_message(p, &message, &viewer, &base)).await?;
    render_json(c, &payload)
}

pub async fn actions(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (_, thread) = scope(c).await?;
    let message = set_message(c, &thread).await?;
    c.no_store();
    let viewer = require_current_user(c)?.clone();
    let base = c.url_for("");
    let payload = messages::present(c, move |p| Ok(json!({"actions": messages::payload::actions(p, &message, &viewer, &base)?}))).await?;
    render_json(c, &payload)
}

fn render_json(c: &mut Ctx, payload: &Value) -> Result {
    Ok(c.render(StatusCode::OK, &format::JSON, serde_json::to_string(payload).map_err(Error::internal)?))
}
