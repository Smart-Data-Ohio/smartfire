//! ChannelThreadsController, ported in coherent endpoint slices.
#[cfg(test)]
mod tests;
#[cfg(test)]
mod page_tests;
#[cfg(test)]
mod board_read_tests;
#[cfg(test)]
mod board_write_tests;
mod writes;
pub use writes::{create, destroy, new, update};
#[cfg(test)]
pub(crate) mod write_tests;
#[cfg(test)]
mod content_tests;
#[cfg(test)]
mod github_tests;
#[cfg(test)]
mod chrome_tests;
#[cfg(test)]
mod declaration_tests;

use askama::Template;
use campfire_db::{ChannelThread, Message, Room, ThreadInvolvement, ThreadMembership, Timeline, Timestamp};
use campfire_kit::{Ctx, Error, Result, StatusCode, format};
use serde_json::{Value, json};
use crate::app::AppCtx;
use crate::concerns::{self, Before, before_actions, cast_integer, require_current_user};
use crate::controllers::{messages, presenters::page::{self, db_error}};

pub async fn index(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (_, room) = concerns::set_room(c).await?;
    if room.deleted_at.is_some() { return Err(Error::NotFound); }
    let state = c.params.get("state").and_then(|value| value.to_s()).unwrap_or_default();
    let (room_id, board, now) = (room.id, room.board(), Timestamp::from_jiff(c.now()));
    let threads = c.app().db.read(move |conn| {
        let threads = if state == "closed" { ChannelThread::effectively_closed_for_room(conn, room_id, now)? }
            else { ChannelThread::for_room(conn, room_id)? };
        Ok(threads.into_iter().filter(|thread| match state.as_str() {
            "all" | "closed" => true,
            "locked" => thread.locked_at.is_some(),
            "work" | "working" => thread.work() && thread.work_status.as_deref() != Some("done"),
            "done" | "completed" => thread.work_status.as_deref() == Some("done"),
            _ => thread.closed_at.is_none() && thread.locked_at.is_none() && (board || thread.auto_archive_at() > now),
        }).collect::<Vec<_>>())
    }).await.map_err(db_error)?;
    let viewer = require_current_user(c)?.clone();
    let base = c.url_for("");
    if c.format()? == Some(&format::JSON) { c.no_store(); }
    if *c.respond_to(&[&format::HTML, &format::JSON])? == format::JSON {
        let payload = messages::present(c, move |p| Ok(json!({"threads": threads.iter()
            .map(|thread| messages::payload::thread(p, thread, &viewer, &base)).collect::<campfire_db::Result<Vec<_>>>()?}))).await?;
        return render_json(c, StatusCode::OK, &payload);
    }
    let (room_name, rows) = messages::present(c, move |p| {
        // The index displays only these facts. Loading a full per-thread JSON
        // payload here made its query cost grow with each row.
        let owner_ids=threads.iter().filter_map(|thread|thread.work_owner_id).collect::<std::collections::BTreeSet<_>>().into_iter().collect::<Vec<_>>();
        let owners=campfire_db::User::where_ids(p.conn,&owner_ids)?.into_iter().map(|user|(user.id,user)).collect::<std::collections::HashMap<_,_>>();
        let members=p.conn.prepare("SELECT user_id FROM memberships WHERE room_id=?")?.query_map([room.id],|r|r.get::<_,i64>(0))?.collect::<rusqlite::Result<std::collections::HashSet<_>>>()?;
        let eligible_agents=p.conn.prepare("SELECT agents.user_id FROM agents WHERE agents.suspended_at IS NULL AND (NOT EXISTS(SELECT 1 FROM agent_grants WHERE agent_id=agents.id) OR EXISTS(SELECT 1 FROM agent_grants WHERE agent_id=agents.id AND revoked_at IS NULL AND capability='post_messages' AND (room_id IS NULL OR room_id=?)))")?.query_map([room.id],|r|r.get::<_,i64>(0))?.collect::<rusqlite::Result<std::collections::HashSet<_>>>()?;
        let counts=p.conn.prepare("SELECT thread_id,COUNT(*) FROM messages WHERE room_id=? AND thread_id IS NOT NULL GROUP BY thread_id")?.query_map([room.id],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?)))?.collect::<rusqlite::Result<std::collections::HashMap<_,_>>>()?;
        let rows=threads.iter().map(|thread| {
            let owner=thread.work_owner_id.and_then(|id|owners.get(&id));
            let active=owner.is_some_and(|owner|owner.is_active()&&members.contains(&owner.id)&&(!owner.is_bot()||room.deleted_at.is_none()&&eligible_agents.contains(&owner.id)));
            let owner_label=match owner {
                None=>"Unassigned".into(),
                Some(owner) if active=>owner.name.clone(),
                Some(owner)=>format!("Owner unavailable ({})",owner.name),
            };
            campfire_views::channel_threads::ListRow {id:thread.id,name:thread.name.clone(),status:thread.status_in_room(&room,Timestamp::from_jiff(p.now)).name().into(),
                message_count:counts.get(&thread.id).copied().unwrap_or(0),work_label:work_status_label(thread.work_status.as_deref()).map(str::to_string),
                owner_label,agent:owner.is_some_and(|owner|owner.is_bot())}
        }).collect::<Vec<_>>();
        Ok((p.room_display_name(&room, Some(&viewer))?, rows))
    }).await?;
    page::titled_content(c, StatusCode::OK, &format!("Threads in {room_name}"), |ctx| campfire_views::channel_threads::Index { ctx, room_id, room_name: &room_name, threads: &rows }.render()).await
}

pub async fn show(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (room, thread) = scope(c).await?;
    let thread_id = thread.id;
    let records = c.app().db.read(move |conn| Message::last_page(conn, Timeline::Thread(thread_id))).await.map_err(db_error)?;
    if c.format()? == Some(&format::JSON) { c.no_store(); }
    if *c.respond_to(&[&format::HTML, &format::JSON])? == format::JSON {
        let viewer = require_current_user(c)?.clone();
        let base = c.url_for("");
        let payload = messages::present(c, move |p| {
            let parent = thread.parent_message_id.map(|id| Message::find(p.conn, id)).transpose()?;
            Ok(json!({"thread": messages::payload::thread_details(p, &thread, &viewer, &base)?,
                "parent_message": parent.as_ref().map(|message| messages::payload::message(p, message, &viewer, &base)).transpose()?,
                "messages": records.iter().map(|message| messages::payload::thread_message(p, message, &viewer, &base)).collect::<campfire_db::Result<Vec<_>>>()?}))
        }).await?;
        return render_json(c, StatusCode::OK, &payload);
    }
    if room.board() {
        let viewer = require_current_user(c)?.clone();
        let picker = c.app().config.google_picker.is_some();
        let post = messages::present(c, move |p| crate::controllers::presenters::board_posts::post(p, &room, &thread, &viewer, &records, picker)).await?;
        return page::framed_page!(c, StatusCode::OK, |ctx| campfire_views::channel_threads::board::Show { ctx, post: &post }).await;
    }
    if thread.work() { return Ok(c.head(StatusCode::NOT_IMPLEMENTED)); }
    render_standalone(c, thread, records, StatusCode::OK).await
}

pub async fn content(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (room, thread) = scope(c).await?;
    if !room.board() && thread.work() {
        // Ordinary work conversation controls and history remain a WS12 seam.
        // Board posts already have their Rails-compatible conversation pane.
        return Ok(c.head(StatusCode::NOT_IMPLEMENTED));
    }
    let id = thread.id;
    let anchor = c.params.get("message_id").filter(|value| value.is_present()).cloned();
    let (records, anchor) = c.app().db.read(move |conn| {
        let anchor = anchor.as_ref().map(|value| messages::paging_anchor(conn, Timeline::Thread(id), value)).transpose()?;
        let records = if let Some(anchor) = &anchor { Message::page_around(conn, Timeline::Thread(id), anchor)? } else { Message::last_page(conn, Timeline::Thread(id))? };
        Ok((records, anchor.map(|message| message.id)))
    }).await.map_err(db_error)?;
    // Rails' explicit partial render has no JSON template and raises MissingTemplate.
    if c.format()? == Some(&format::JSON) { return Err(Error::internal(anyhow::anyhow!("Missing thread conversation JSON partial"))); }
    c.no_store();
    let viewer = require_current_user(c)?.clone();
    let updated_at = room.updated_at.jiff();
    let picker_available = c.app().config.google_picker.is_some();
    let (messages, user, steps, composer) = messages::present(c, move |p| Ok((p.messages(&records)?, p.user_view(viewer.id)?, p.thread_steps(id)?,
        p.composer_facts(&room, &viewer, Some(&thread), p.composer_drive_flow(&viewer, picker_available && !viewer.is_bot())?)?))).await?;
    c.set_header("x-thread-content-at-latest", if anchor.is_none() {"true"} else {"false"});
    page::bare(c, StatusCode::OK, &format::HTML, |ctx| {
        let scheduled_control = render_thread_schedule_control(ctx, composer.room_id, id)?;
        campfire_views::channel_threads::Conversation {ctx, thread_id: id,
            room_updated_at: updated_at, anchor, messages: &messages, user: &user, steps: &steps, composer: &composer, scheduled_control: &scheduled_control}.render()
    }).await
}

/// Stable M2 provider seam: room and thread scope must both reach the schedule child.
fn render_thread_schedule_control(ctx: &campfire_views::ViewContext<'_>, room_id: i64, thread_id: i64) -> askama::Result<campfire_views::helpers::Html> {
    Ok(campfire_views::helpers::raw(campfire_views::scheduled_messages::ComposerButton {
        ctx, room_id, thread_id: Some(thread_id),
    }.render()?))
}

async fn render_standalone(c: &mut Ctx, thread: ChannelThread, records: Vec<Message>, response_status: StatusCode) -> Result {
    let name = thread.name.clone();
    let (parent, items, count, status, pull_request_header) = messages::present(c, move |p| {
        let parent = thread.parent_message_id.map(|id| Message::find(p.conn, id)).transpose()?.as_ref().map(|message| p.message_item(message)).transpose()?;
        Ok((parent, p.messages(&records)?, thread.message_count(p.conn)?, thread.status(p.conn, Timestamp::from_jiff(p.now))?.name(),
            render_thread_pull_request_header(p, &thread)?))
    }).await?;
    // Work/board sections remain WS12 seams. The standalone PR header uses WS15g. The ordinary standalone
    // thread uses the same stable collection entry point as the room's message list.
    page::titled_content(c, response_status, &name, |ctx| campfire_views::channel_threads::Show { ctx,
        name: &name, status, count, pull_request_header: &pull_request_header, parent: parent.as_ref(), messages: &items }.render()).await
}

/// Named WS15g integration call site, after authorizing the parent room and scoped thread.
fn render_thread_pull_request_header(p: &crate::controllers::presenters::Presenter<'_>, thread: &ChannelThread) -> campfire_db::Result<campfire_views::helpers::Html> {
    p.github_thread_header(thread)
}

fn work_status_label(status: Option<&str>) -> Option<&'static str> {
    match status { Some("planned") => Some("Planned"), Some("in_progress") => Some("In progress"),
        Some("blocked") => Some("Blocked"), Some("done") => Some("Done"), _ => None }
}

/// An alive parent-room membership scopes both the nested reads and membership actions. A
/// thread membership is required only by read; browsing a thread never silently joins it.
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
    match c.respond_to(&[&format::HTML, &format::JSON]) {
        Ok(chosen) if *chosen == format::JSON => render_json(c, status, &json!({"error": message})),
        Ok(_) | Err(Error::UnknownFormat) => Ok(c.head(status)),
        Err(error) => Err(error),
    }
}

fn render_json(c: &mut Ctx, status: StatusCode, payload: &Value) -> Result {
    Ok(c.render(status, &format::JSON, serde_json::to_string(payload).map_err(Error::internal)?))
}
