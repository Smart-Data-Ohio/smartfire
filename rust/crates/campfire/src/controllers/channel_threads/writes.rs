//! Channel creation and ordinary thread lifecycle. Work/board writes await WS12's domain API.
use super::*;
use campfire_db::{Membership, NewChannelThread, NewMessage, ThreadStatus};
use campfire_kit::{Param, ParamMap, permit_keys};
use crate::controllers::presenters::attachments::Assignment;

const FORBIDDEN_UPDATE: &str = "WS8bm thread update forbidden";

fn source(c: &Ctx) -> Result<&ParamMap> {
    match c.params.get("thread").filter(|value| value.is_present()) {
        Some(value) => value.as_hash().ok_or_else(|| Error::internal(anyhow::anyhow!("thread parameters do not support permit"))),
        None => Ok(&c.params),
    }
}

fn permitted(c: &Ctx, keys: &[&str]) -> Result<ParamMap> {
    Ok(Param::Hash(source(c)?.clone()).permit(&permit_keys(keys)))
}

fn archive_minutes(value: &Param) -> Result<i64> {
    if matches!(value, Param::Bool(_)) { return Err(Error::internal(anyhow::anyhow!("boolean has no to_i"))); }
    Ok(value.to_s().as_deref().and_then(cast_integer).unwrap_or(0))
}

async fn alive_room(c: &mut Ctx) -> Result<Room> {
    let (_, room) = concerns::set_room(c).await?;
    if room.deleted_at.is_some() { return Err(Error::NotFound); }
    Ok(room)
}

pub async fn new(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = alive_room(c).await?;
    if !room.board() { return Ok(c.head(StatusCode::NOT_FOUND)); }
    let first_message = match c.params.get("thread").filter(|value| !value.is_null()) {
        Some(value) => value.as_hash().ok_or_else(|| Error::internal(anyhow::anyhow!("thread does not support dig")))?
            .get("first_message").map(|value| campfire_richtext::ruby::json_value_to_s(&value.to_json())),
        None => None,
    };
    let viewer = require_current_user(c)?.clone();
    let mut post = messages::present(c, move |p| crate::controllers::presenters::board_posts::new_post(p, &room, &viewer)).await?;
    post.first_message = first_message;
    page::framed_page!(c, StatusCode::OK, |ctx| campfire_views::channel_threads::board::New { ctx, post: &post }).await
}

pub async fn create(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = alive_room(c).await?;
    if room.direct() { return render_error(c, StatusCode::FORBIDDEN, "Direct rooms cannot contain channel threads"); }
    if room.board() { return crate::controllers::not_yet_ported(c).await; }
    match create_channel(c, room).await {
        Err(Error::NotFound) => Ok(c.head(StatusCode::NOT_FOUND)),
        Err(error) => write_error(c, error),
        result => result,
    }
}

async fn create_channel(c: &mut Ctx, room: Room) -> Result {
    let attributes = permitted(c, &["name", "auto_archive_after_minutes"])?;
    let parent = source(c)?.get("parent_message_id").filter(|value| value.is_present()).cloned();
    let room_id = room.id;
    let parent_id = if let Some(value) = parent {
        Some(c.app().db.read(move |conn| messages::paging_anchor(conn, Timeline::Room(room_id), &value)).await.map_err(db_error)?.id)
    } else { None };
    let raw_initial = c.params.get("message").or_else(|| source(c).ok()?.get("message")).filter(|value| value.is_present());
    let initial = raw_initial.map(|value| value.permit(&permit_keys(&["body", "attachment", "markdown_source", "client_message_id", "reply_to_message_id", "reply_notify_author", "forward_note"]))).unwrap_or_default();
    let creator = require_current_user(c)?.id;
    let client_id = initial.get("client_message_id").filter(|value| value.is_present()).and_then(messages::string_column);
    let duplicate = if let Some(client_id) = client_id {
        c.app().db.read(move |conn| {
            let duplicate = Message::find_duplicate(conn, room_id, creator, &client_id)?;
            duplicate.and_then(|message| message.thread_id).map(|id| ChannelThread::find(conn, id)).transpose()
        }).await.map_err(db_error)?
    } else { None };
    let thread = if let Some(thread) = duplicate { thread } else {
        let mut body = initial.get("body").and_then(messages::string_column);
        let markdown = initial.get("markdown_source").and_then(messages::string_column);
        if markdown.is_some() { body = None; }
        let body = match body { Some(body) => Some(messages::canonicalize_body(c.app(), body, Some(c.request.host())).await?), None => None };
        let staged = match messages::attachment_assignment(&initial)? {
            Some(Assignment::Create(upload)) => Some(upload.stage(c.app()).await?),
            Some(Assignment::Invalid) => return Err(Error::internal(anyhow::anyhow!("Could not find or build blob: expected attachable"))),
            _ => None,
        };
        let name = attributes.get("name").and_then(messages::string_column);
        let minutes = attributes.get("auto_archive_after_minutes").map(archive_minutes).transpose()?;
        let reply = initial.get("reply_to_message_id").filter(|value| value.is_present()).and_then(|value| value.to_s()).as_deref().and_then(cast_integer);
        let notify = initial.get("reply_notify_author").map(|value| {
            if value.is_null() || value.as_str() == Some("") { None }
            else { Some(!matches!(value.to_s().as_deref(), Some("false" | "FALSE" | "f" | "F" | "0" | "off" | "OFF"))) }
        });
        let (thread, blob) = c.app().db.write(move |tx| {
            let room = Room::find(tx.conn(), room_id)?;
            if room.deleted_at.is_some() || Membership::find_by_room_and_user(tx.conn(), room_id, creator)?.is_none() {
                return Err(campfire_db::Error::RecordNotFound("Membership"));
            }
            let mut thread = ChannelThread::create(tx, NewChannelThread { room_id, creator_id: creator, parent_message_id: parent_id,
                name, auto_archive_after_minutes: minutes, ..Default::default() })?;
            ThreadMembership::join(tx, thread.id, creator)?;
            let mut saved_blob = None;
            if !initial.is_empty() {
                if notify == Some(None) { return Err(campfire_db::Error::Other("reply_notify_author violates NOT NULL".into())); }
                let blob = staged.map(|staged| messages::save_staged(tx, staged)).transpose()?;
                thread.post_message(tx, creator, NewMessage { body, markdown_source: markdown,
                    client_message_id: initial.get("client_message_id").and_then(messages::string_column),
                    attachment_blob_id: blob.as_ref().map(|blob| blob.id), reply_to_message_id: reply, reply_notify_author: notify.flatten(),
                    forward_note: initial.get("forward_note").and_then(messages::string_column), ..Default::default() })?;
                saved_blob = blob;
            }
            Ok((thread, saved_blob))
        }).await.map_err(db_error)?;
        if let Some(blob) = blob { messages::process_attachment(c.app(), blob).await?; }
        thread
    };
    if *c.respond_to(&[&format::HTML, &format::JSON])? == format::HTML {
        return c.redirect_to(&c.url_for(&format!("/rooms/{room_id}/threads/{}", thread.id)));
    }
    let viewer = require_current_user(c)?.clone();
    let base = c.url_for("");
    let payload = messages::present(c, move |p| {
        let parent = thread.parent_message_id.map(|id| Message::find(p.conn, id)).transpose()?;
        Ok(json!({"thread": messages::payload::thread(p, &thread, &viewer, &base)?,
            "parent_message": parent.as_ref().map(|message| messages::payload::message(p, message, &viewer, &base)).transpose()?}))
    }).await?;
    render_json(c, StatusCode::CREATED, &payload)
}

pub async fn update(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (room, thread) = scope(c).await?;
    let attributes = permitted(c, &["name", "auto_archive_after_minutes", "status", "work_status", "work_owner_id", "tags", "result_markdown"])?;
    let work_pending = ["work_status", "work_owner_id", "result_markdown"].iter().any(|key| attributes.contains_key(key));
    let actor = require_current_user(c)?.clone();
    let name = attributes.get("name").and_then(messages::string_column);
    let minutes = attributes.get("auto_archive_after_minutes").map(archive_minutes).transpose()?;
    let tags = attributes.get("tags").map(|value| value.to_s().unwrap_or_default().split(',').map(str::to_string).collect::<Vec<_>>());
    let status = attributes.get("status").and_then(messages::string_column);
    let metadata_given = attributes.contains_key("name") || minutes.is_some() || tags.is_some();
    let (thread_id, room_id) = (thread.id, room.id);
    let mut attempted = thread.clone();
    if let Some(name) = &name { attempted.name = name.clone(); }
    if let Some(minutes) = minutes { attempted.auto_archive_after_minutes = minutes; }
    let result = c.app().db.write(move |tx| {
        let mut thread = ChannelThread::find(tx.conn(), thread_id)?;
        if Membership::find_by_room_and_user(tx.conn(), room_id, actor.id)?.is_none() {
            return Err(campfire_db::Error::RecordNotFound("Membership"));
        }
        let settings = thread.settings_manageable_by(tx.conn(), &actor)?;
        let moderator = thread.manageable_by(tx.conn(), &actor)?;
        let allowed = (!metadata_given || settings) && match status.as_deref() {
            None => true,
            Some("closed") => settings,
            Some("locked") => moderator,
            Some("active") if thread.locked_at.is_some() => moderator,
            Some("active") if thread.status(tx.conn(), tx.now())? == ThreadStatus::Closed => thread.membership_for(tx.conn(), actor.id)?.is_some(),
            Some("active") => true,
            _ => false,
        };
        if !allowed { return Err(campfire_db::Error::Other(FORBIDDEN_UPDATE.into())); }
        if room.board() || work_pending { return Err(campfire_db::Error::Other("WS8bm pending work domain".into())); }
        thread.update_metadata(tx, name.as_deref(), minutes, tags.as_deref())?;
        match status.as_deref() {
            Some("closed") => thread.close(tx)?, Some("locked") => thread.lock_conversation(tx)?,
            Some("active") if thread.locked_at.is_some() => thread.unlock_conversation(tx)?,
            Some("active") => thread.reopen(tx)?, _ => {},
        }
        Ok(thread)
    }).await;
    let thread = match result {
        Ok(thread) => thread,
        Err(campfire_db::Error::RecordNotFound(_)) => return forbidden_update(c, &thread),
        Err(campfire_db::Error::Other(message)) if message == FORBIDDEN_UPDATE => return forbidden_update(c, &thread),
        Err(campfire_db::Error::Other(message)) if message == "WS8bm pending work domain" => return crate::controllers::not_yet_ported(c).await,
        Err(campfire_db::Error::RecordInvalid(errors)) if c.format()? == Some(&format::HTML) => {
            let records = c.app().db.read(move |conn| Message::last_page(conn, Timeline::Thread(thread_id))).await.map_err(db_error)?;
            let _ = errors; // Ordinary Rails show has no post-error slot; attempted values still render.
            return render_standalone(c, attempted, records, StatusCode::UNPROCESSABLE_ENTITY).await;
        }
        Err(error) => return write_error(c, Error::internal(error)),
    };
    if *c.respond_to(&[&format::HTML, &format::JSON])? == format::HTML {
        return c.redirect_to(&c.url_for(&format!("/rooms/{room_id}/threads/{thread_id}")));
    }
    let viewer = require_current_user(c)?.clone();
    let base = c.url_for("");
    let payload = messages::present(c, move |p| Ok(json!({"thread": messages::payload::thread_details(p, &thread, &viewer, &base)?}))).await?;
    render_json(c, StatusCode::OK, &payload)
}

fn forbidden_update(c: &mut Ctx, thread: &ChannelThread) -> Result {
    if c.format()? == Some(&format::HTML) {
        c.flash().set("alert", "You are not allowed to change this post.");
        c.redirect_to(&c.url_for(&format!("/rooms/{}/threads/{}", thread.room_id, thread.id)))
    } else { Ok(c.head(StatusCode::FORBIDDEN)) }
}

pub async fn destroy(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (room, thread) = scope(c).await?;
    let actor = require_current_user(c)?.clone();
    let id = thread.id;
    let removed = c.app().db.write(move |tx| {
        let thread = ChannelThread::find(tx.conn(), id)?;
        if Room::find(tx.conn(), thread.room_id)?.deleted_at.is_some() || Membership::find_by_room_and_user(tx.conn(), thread.room_id, actor.id)?.is_none() {
            return Err(campfire_db::Error::RecordNotFound("Membership"));
        }
        if !thread.manageable_by(tx.conn(), &actor)? { return Ok(false); }
        if thread.work() { return Err(campfire_db::Error::Other("WS8bm pending work domain".into())); }
        thread.destroy(tx)?;
        Ok(true)
    }).await;
    match removed {
        Ok(false) => return Ok(concerns::head(StatusCode::FORBIDDEN)),
        Err(campfire_db::Error::Other(message)) if message == "WS8bm pending work domain" => return crate::controllers::not_yet_ported(c).await,
        Err(error) => return Err(db_error(error)), _ => {},
    }
    if c.format()? == Some(&format::HTML) { c.redirect_to(&c.url_for(&format!("/rooms/{}", room.id))) }
    else { Ok(concerns::head(StatusCode::NO_CONTENT)) }
}

fn write_error(c: &mut Ctx, error: Error) -> Result {
    let Error::Internal(error) = error else { return Err(error); };
    let Some(error) = error.downcast_ref::<campfire_db::Error>() else { return Err(Error::Internal(error)); };
    match error {
        campfire_db::Error::RecordInvalid(errors) => render_error(c, StatusCode::UNPROCESSABLE_ENTITY,
            &campfire_views::helpers::to_sentence(&errors.full_messages(), " and ")),
        error if error.is_record_not_unique() => render_error(c, StatusCode::CONFLICT, "A thread already exists for that message"),
        _ => Err(Error::internal(anyhow::anyhow!("{error}"))),
    }
}
