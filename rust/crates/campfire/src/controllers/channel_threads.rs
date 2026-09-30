//! ChannelThreadsController, ported in coherent endpoint slices.
#[cfg(test)]
mod tests;

use campfire_db::{ChannelThread, Room, ThreadInvolvement, ThreadMembership};
use campfire_kit::{Ctx, Error, Result, StatusCode, format};
use serde_json::{Value, json};
use crate::app::AppCtx;
use crate::concerns::{self, Before, before_actions, cast_integer, require_current_user};
use crate::controllers::{messages, presenters::page::db_error};

/// An alive parent-room membership is sufficient to join, read or leave its thread. A thread
/// membership is required only by read; browsing a thread never silently joins it.
async fn scope(c: &mut Ctx) -> Result<(Room, ChannelThread)> {
    let (_, room) = concerns::set_room(c).await?;
    if room.deleted_at.is_some() { return Err(Error::NotFound); }
    let id = c.param_str("id").and_then(cast_integer).ok_or(Error::NotFound)?;
    let room_id = room.id;
    let thread = c.app().db.read(move |conn| {
        let thread = ChannelThread::find(conn, id)?;
        if thread.room_id != room_id { return Err(campfire_db::Error::RecordNotFound("ChannelThread")); }
        Ok(thread)
    }).await.map_err(db_error)?;
    Ok((room, thread))
}

pub async fn join(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (room, thread) = scope(c).await?;
    c.no_store();
    let involvement = match c.params.get("involvement") {
        None => None,
        Some(value) => match value.to_s().as_deref().and_then(ThreadInvolvement::from_name) {
            Some(value) => Some(value),
            None => return render_error(c, StatusCode::UNPROCESSABLE_ENTITY, "Involvement must be one of nothing, mentions, or everything"),
        },
    };
    let (thread_id, user_id) = (thread.id, require_current_user(c)?.id);
    let member = match c.app().db.write(move |tx| {
        let mut member = ThreadMembership::join(tx, thread_id, user_id)?;
        if let Some(involvement) = involvement { member.update_involvement(tx, involvement)?; }
        Ok(member)
    }).await {
        Ok(member) => member,
        Err(campfire_db::Error::RecordNotFound(_)) => return render_error(c, StatusCode::UNPROCESSABLE_ENTITY, "Thread is inaccessible"),
        Err(error) => return Err(db_error(error)),
    };
    if *c.respond_to(&[&format::HTML, &format::JSON])? == format::HTML {
        return c.redirect_to(&c.url_for(&format!("/rooms/{}/threads/{}", room.id, thread.id)));
    }
    render_membership(c, thread, member).await
}

pub async fn leave(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (room, thread) = scope(c).await?;
    let (thread_id, user_id) = (thread.id, require_current_user(c)?.id);
    c.app().db.write(move |tx| {
        // ThreadMembership has no destroy callbacks; this is the scoped find_by(user)&.destroy!.
        tx.conn().execute("DELETE FROM thread_memberships WHERE thread_id = ? AND user_id = ?", (thread_id, user_id))?;
        Ok(())
    }).await.map_err(db_error)?;
    c.expires_now();
    if c.format()? == Some(&format::HTML) {
        c.redirect_to(&c.url_for(&format!("/rooms/{}/threads/{}", room.id, thread.id)))
    } else { Ok(c.head(StatusCode::NO_CONTENT)) }
}

pub async fn read(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (_, thread) = scope(c).await?;
    c.no_store();
    let (thread_id, user_id) = (thread.id, require_current_user(c)?.id);
    let member = c.app().db.write(move |tx| {
        let Some(mut member) = ThreadMembership::find_by_thread_and_user(tx.conn(), thread_id, user_id)? else { return Ok(None) };
        member.read(tx)?;
        Ok(Some(member))
    }).await.map_err(db_error)?;
    let Some(member) = member else { return render_error(c, StatusCode::NOT_FOUND, "Join the thread before marking it read") };
    render_membership(c, thread, member).await
}

async fn render_membership(c: &mut Ctx, thread: ChannelThread, member: ThreadMembership) -> Result {
    let viewer = require_current_user(c)?.clone();
    let base = c.url_for("");
    let payload = messages::present(c, move |p| Ok(json!({
        "thread": messages::payload::thread(p, &thread, &viewer, &base)?,
        "membership": {"id": member.id, "user_id": member.user_id, "involvement": member.involvement.name(),
            "unread_at": member.unread_at.map(|time| campfire_views::messages::support::json_time(time.jiff())),
            "joined_at": campfire_views::messages::support::json_time(member.joined_at.jiff())}
    }))).await?;
    render_json(c, StatusCode::OK, &payload)
}

fn render_error(c: &mut Ctx, status: StatusCode, message: &str) -> Result {
    if c.format()? == Some(&format::JSON) { render_json(c, status, &json!({"error": message})) }
    else { Ok(c.head(status)) }
}

fn render_json(c: &mut Ctx, status: StatusCode, payload: &Value) -> Result {
    Ok(c.render(status, &format::JSON, serde_json::to_string(payload).map_err(Error::internal)?))
}
