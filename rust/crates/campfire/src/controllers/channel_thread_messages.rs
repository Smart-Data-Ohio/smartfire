//! Nested message endpoints (ChannelThreadMessagesController).
#[cfg(test)]
mod tests;
#[cfg(test)]
pub(crate) mod write_tests;

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

pub async fn create(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (room, thread) = scope(c).await?;
    match create_action(c, room, thread).await { Err(error) => render_write_error(c, error), result => result }
}

async fn create_action(c: &mut Ctx, room: Room, thread: ChannelThread) -> Result {
    let client_id = c.params.get("message").and_then(|message| message.get("client_message_id")).filter(|id| id.is_present()).and_then(|id| id.to_s());
    let (room_id, creator_id) = (room.id, require_current_user(c)?.id);
    let duplicate = if let Some(client_id) = client_id {
        c.app().db.read(move |conn| Message::find_duplicate(conn, room_id, creator_id, &client_id)).await.map_err(db_error)?
    } else { None };
    let message = if let Some(message) = duplicate { message }
        else {
            let attributes = messages::human_message_params(c, None).await?;
            let message = messages::create_message_into(c, &room, Some(thread.clone()), attributes).await?;
            messages::broadcast_create(c, &room, &message).await?;
            message
        };
    if c.format()? == Some(&format::JSON) { c.no_store(); }
    match c.respond_to(&[&format::HTML, &format::JSON, &format::TURBO_STREAM])? {
        f if *f == format::HTML => c.redirect_to(&c.url_for(&format!("/rooms/{}/threads/{}", room.id, thread.id))),
        f if *f == format::JSON => {
            let viewer = require_current_user(c)?.clone();
            let base = c.url_for("");
            let payload = messages::present(c, move |p| messages::payload::thread_message(p, &message, &viewer, &base)).await?;
            Ok(c.render(StatusCode::CREATED, &format::JSON, serde_json::to_string(&payload).map_err(Error::internal)?))
        },
        _ => {
            let view = messages::present(c, move |p| p.message(&message)).await?;
            page::bare(c, StatusCode::OK, &format::TURBO_STREAM, |ctx| campfire_views::messages::ThreadCreateStream { ctx, message: &view, thread_id: thread.id }.render()).await
        }
    }
}

pub async fn update(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (room, thread) = scope(c).await?;
    let message = set_message(c, &thread).await?;
    messages::ensure_can_edit(c, &message)?;
    if thread.locked_at.is_some() { return Ok(concerns::head(StatusCode::FORBIDDEN)); }
    let message = match messages::update_human_message(c, None, Some(thread.id), message).await {
        Ok(message) => message, Err(error) => return render_write_error(c, error),
    };
    let drive_given = c.params.get("message").and_then(|params| params.get("drive_file_ids")).is_some();
    messages::rendered::broadcast_thread_edit(c, &room, &message, drive_given).await?;
    if c.format()? == Some(&format::JSON) { c.no_store(); }
    if *c.respond_to(&[&format::HTML, &format::JSON])? == format::HTML {
        return c.redirect_to(&c.url_for(&format!("/rooms/{}/threads/{}/messages/{}", room.id, thread.id, message.id)));
    }
    let viewer = require_current_user(c)?.clone();
    let base = c.url_for("");
    let payload = messages::present(c, move |p| messages::payload::thread_message(p, &message, &viewer, &base)).await?;
    render_json(c, &payload)
}

pub async fn destroy(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (room, thread) = scope(c).await?;
    let message = set_message(c, &thread).await?;
    messages::ensure_can_delete(c, &message)?;
    messages::destroy_message(c, &room, &message).await?;
    messages::rendered::broadcast_thread_refresh(c.app(), room.id, thread.id).await?;
    if c.format()? == Some(&format::HTML) { c.redirect_to(&c.url_for(&format!("/rooms/{}/threads/{}", room.id, thread.id))) }
    else { Ok(c.head(StatusCode::NO_CONTENT)) }
}

fn render_write_error(c: &mut Ctx, error: Error) -> Result {
    let Error::Internal(internal) = error else { return Err(error) };
    let (status, text) = match internal.downcast_ref::<campfire_db::Error>() {
        Some(campfire_db::Error::RecordInvalid(errors)) => (StatusCode::UNPROCESSABLE_ENTITY, campfire_views::helpers::to_sentence(&errors.full_messages(), " and ")),
        Some(campfire_db::Error::Other(message)) if message == campfire_db::channel_thread::LOCKED_MESSAGE => (StatusCode::FORBIDDEN, message.clone()),
        _ => return Err(Error::Internal(internal)),
    };
    if c.format()? == Some(&format::JSON) {
        Ok(c.render(status, &format::JSON, serde_json::to_string(&json!({"error": text})).map_err(Error::internal)?))
    } else { Ok(c.head(status)) }
}
