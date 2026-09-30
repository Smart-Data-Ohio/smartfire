//! `MessagePayloadHelper`: request-specific JSON, never a shared fragment-cache value.
use campfire_db::{ChannelThread, Message, MessagePin, SavedItem, Timestamp, User};
use campfire_views::helpers::{AvatarIcon, IconSource};
use campfire_views::messages::support::json_time;
use rusqlite::OptionalExtension;
use serde_json::{Value, json};

use crate::controllers::presenters::{Presenter, Result, avatar_path};

pub(super) fn message(p: &Presenter<'_>, message: &Message, viewer: &User, base: &str) -> Result<Value> {
    let room = campfire_db::Room::find(p.conn, message.room_id)?;
    let mut body = json!({"plain_text": p.plain_text_body(message)?, "html": html(p, message)?});
    if let Some(source) = &message.markdown_source { body["markdown_source"] = source.clone().into(); }
    let mut result = json!({
        "id": message.id, "client_message_id": message.client_message_id,
        "created_at": json_time(message.created_at.jiff()), "updated_at": json_time(message.updated_at.jiff()),
        "body": body, "creator": user(p, &p.user(message.creator_id)?, base)?,
        "room": {"id": message.room_id, "icon_name": room.icon_name}
    });
    if let Some(id) = message.thread_id {
        result["thread_context"] = thread(p, &ChannelThread::find(p.conn, id)?, viewer, base)?;
    }
    if let Some(summary) = ChannelThread::find_by_parent_message(p.conn, message.id)? {
        result["thread_summary"] = thread(p, &summary, viewer, base)?;
    }
    if message.reply_to_message_id.is_some() || message.reply_target_deleted_at.is_some() {
        let source = message.reply_to_message_id.map(|id| Message::find_by_id(p.conn, id)).transpose()?.flatten();
        let mut reply = match &source {
            Some(source) => json!({"id": source.id, "url": permalink(source, base), "deleted": false, "creator": user(p, &p.user(source.creator_id)?, base)?,
                "body": {"plain_text": p.plain_text_body(source)?, "html": html(p, source)?}}),
            None => json!({}),
        };
        reply["deleted"] = source.is_none().into();
        reply["notify_author"] = message.reply_notify_author.into();
        result["reply_to"] = reply;
    }
    if message.forwarded_at.is_some() {
        let mut forwarded = json!({"label": "Forwarded"});
        if let Some(note) = &message.forward_note { forwarded["note"] = note.clone().into(); }
        result["forwarded"] = forwarded;
    }
    result["drive_attachments"] = message.drive_file_ids(p.conn)?.into_iter().map(|id| json!({"file_id": id, "url": format!("https://drive.google.com/open?id={id}")})).collect::<Vec<_>>().into();
    if message.streaming { result["streaming"] = true.into(); }
    result["url"] = permalink(message, base).into();
    Ok(result)
}

/// Nested thread reads suppress only the root's thread-summary field.
pub(crate) fn thread_message(p: &Presenter<'_>, record: &Message, viewer: &User, base: &str) -> Result<Value> {
    let mut payload = message(p, record, viewer, base)?;
    payload.as_object_mut().expect("message payload is an object").remove("thread_summary");
    Ok(payload)
}

pub(crate) fn actions(p: &Presenter<'_>, message: &Message, viewer: &User, base: &str) -> Result<Value> {
    let saved = SavedItem::find_by_user_and_message(p.conn, viewer.id, message.id)?;
    let summary = ChannelThread::find_by_parent_message(p.conn, message.id)?;
    let locked = message.thread_id.map(|id| ChannelThread::find(p.conn, id)).transpose()?.is_some_and(|thread| thread.locked_at.is_some());
    let author = message.creator_id == viewer.id;
    let path = match message.thread_id {
        Some(id) => format!("/rooms/{}/threads/{id}/messages/{}", message.room_id, message.id),
        None => format!("/rooms/{}/messages/{}", message.room_id, message.id),
    };
    let edit_source = p.editable_markdown_source(message)?;
    let boosts = campfire_db::Boost::for_message(p.conn, message.id)?;
    let mut reactions = serde_json::Map::new();
    for (character, title) in campfire_views::messages::REACTIONS {
        let users = boosts.iter().filter(|boost| boost.content == *character).map(|boost| boost.booster_id).collect::<std::collections::HashSet<_>>();
        reactions.insert((*character).into(), json!({"title": title, "count": users.len(), "active": users.contains(&viewer.id)}));
    }
    let mut result = json!({
        "can_edit": !message.system_note && author && !locked,
        "can_delete": !message.system_note && (author || viewer.is_administrator()),
        "can_remove_embeds": !message.system_note && author && !message.embeds_suppressed && !locked && renderable_embeds(p, message)?,
        "suppress_embeds_url": format!("{base}{path}/embed_suppression"),
        "edit_source": edit_source, "edit_format": if message.markdown() {"markdown"} else {"rich_text"},
        "copy_text": p.plain_text_body(message)?
    });
    if let Some(summary) = summary {
        result["thread_url"] = format!("{base}/rooms/{}/threads/{}", summary.room_id, summary.id).into();
        result["thread_summary"] = thread(p, &summary, viewer, base)?;
    }
    result["forward_url"] = format!("{base}{path}/forwards.json").into();
    result["forward_destinations_url"] = format!("{base}{path}/forwards/destinations.json").into();
    result["reactions"] = reactions.into();
    result["pinned"] = MessagePin::pinned(p.conn, message.id)?.into();
    result["pin_url"] = format!("{base}/messages/{}/pin.json", message.id).into();
    result["saved"] = saved.is_some().into();
    result["save_url"] = format!("{base}/saved.json").into();
    if let Some(saved) = saved { result["saved_item_url"] = format!("{base}/saved/{}.json", saved.id).into(); }
    Ok(result)
}

fn html(p: &Presenter<'_>, message: &Message) -> Result<String> {
    if message.markdown() {
        let resolver = p.resolver();
        crate::rich_text::markdown_presentation(p.conn, &message.body_html(p.conn)?.unwrap_or_default(), &resolver.render_context(p.request_host.clone()))
            .map_err(campfire_db::Error::Other)
    } else { p.rendered_body_html(message) }
}

fn permalink(message: &Message, base: &str) -> String {
    format!("{base}{}", campfire_db::message_pin::message_path(message))
}

fn user(p: &Presenter<'_>, user: &User, base: &str) -> Result<Value> {
    let icon: Option<String> = p.conn.query_row("SELECT icon_name FROM users WHERE id = ?", [user.id], |row| row.get(0))?;
    let icon_url = icon.as_deref().and_then(|name| p.resolve_avatar_icon(name)).and_then(|icon| match icon {
        AvatarIcon::Image {url, ..} => Some(url), _ => None,
    });
    Ok(json!({"id": user.id, "name": user.name, "role": user.role.name(),
        "avatar_url": format!("{base}{}", avatar_path(p.secrets, user)), "icon_name": icon, "icon_avatar_url": icon_url}))
}

pub(crate) fn thread(p: &Presenter<'_>, thread: &ChannelThread, viewer: &User, base: &str) -> Result<Value> {
    let member = thread.membership_for(p.conn, viewer.id)?;
    let room = thread.room(p.conn)?;
    let settings = thread.settings_manageable_by(p.conn, viewer)?;
    let lifecycle = thread.manageable_by(p.conn, viewer)?;
    let accessible = viewer.is_active() && !viewer.is_bot() && campfire_db::Membership::find_by_room_and_user(p.conn, room.id, viewer.id)?.is_some();
    let work_assignment = accessible && settings;
    let work_manageable = accessible && (settings || thread.work_owner_id == Some(viewer.id));
    let owner = thread.work_owner_id.map(|id| p.user(id)).transpose()?;
    let owner_active = match &owner {
        Some(owner) if owner.is_active() && campfire_db::Membership::find_by_room_and_user(p.conn, room.id, owner.id)?.is_some() => {
            if owner.is_bot() { agent_may_post(p, owner.id, &room)? } else { true }
        }
        _ => false,
    };
    let owner = owner.map(|owner| -> Result<Value> {
        let mut value = user(p, &owner, base)?;
        value["active"] = owner.is_active().into(); value["human"] = (!owner.is_bot()).into(); value["agent"] = owner.is_bot().into();
        Ok(value)
    }).transpose()?;
    let member_count: i64 = p.conn.query_row("SELECT count(*) FROM thread_memberships WHERE thread_id = ?", [thread.id], |row| row.get(0))?;
    Ok(json!({
        "id": thread.id, "name": thread.name, "status": thread.status(p.conn, Timestamp::from_jiff(p.now))?.name(),
        "room_id": thread.room_id, "parent_message_id": thread.parent_message_id,
        "last_activity_at": json_time(thread.last_activity_at.jiff()),
        "closed_at": thread.closed_at.map(|time| json_time(time.jiff())), "locked_at": thread.locked_at.map(|time| json_time(time.jiff())),
        "auto_archive_after_minutes": thread.auto_archive_after_minutes, "work": thread.work(), "work_status": thread.work_status,
        "work_owner_id": thread.work_owner_id, "work_owner": owner, "work_owner_active": owner_active,
        "work_history": Value::Null, "work_owner_options": Value::Null,
        "joined": member.is_some(), "unread": member.as_ref().map(|member| member.unread()),
        "involvement": member.as_ref().map(|member| member.involvement.name()),
        "message_count": thread.message_count(p.conn)?, "member_count": member_count,
        "creator": user(p, &p.user(thread.creator_id)?, base)?, "url": format!("{base}/rooms/{}/threads/{}", thread.room_id, thread.id),
        "permalink_url": format!("{base}/rooms/{}?thread={}", thread.room_id, thread.id),
        "permissions": {"can_rename": settings, "can_close": settings, "can_reopen": if thread.locked_at.is_some() {lifecycle} else {member.is_some()},
            "can_lock": lifecycle, "can_unlock": lifecycle, "can_delete": lifecycle, "can_convert_work": !thread.work() && work_assignment,
            "can_manage_work": thread.work() && work_manageable, "can_update_work_status": thread.work() && work_manageable,
            "can_assign_work": thread.work() && work_assignment, "can_remove_work": thread.work() && work_assignment && !room.board()}
    }))
}

fn agent_may_post(p: &Presenter<'_>, user: i64, room: &campfire_db::Room) -> Result<bool> {
    if room.deleted_at.is_some() { return Ok(false) }
    let id: Option<i64> = p.conn.query_row("SELECT id FROM agents WHERE user_id = ? AND suspended_at IS NULL LIMIT 1", [user], |row| row.get(0)).optional()?;
    let Some(id) = id else { return Ok(false) };
    Ok(p.conn.query_row("SELECT NOT EXISTS(SELECT 1 FROM agent_grants WHERE agent_id = ?1) OR EXISTS(SELECT 1 FROM agent_grants WHERE agent_id = ?1 AND revoked_at IS NULL AND capability = 'post_messages' AND (room_id IS NULL OR room_id = ?2))", (id, room.id), |row| row.get(0))?)
}

fn renderable_embeds(p: &Presenter<'_>, message: &Message) -> Result<bool> {
    // LinkEmbed#usable? / #linkedin?: use the stored reference rows, never fetch on a metadata read.
    let mut query = p.conn.prepare("SELECT title, description, normalized_url FROM link_embeds JOIN link_embed_references ON link_embed_references.link_embed_id = link_embeds.id WHERE link_embed_references.message_id = ?")?;
    for row in query.query_map([message.id], |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, Option<String>>(1)?, row.get::<_, String>(2)?)))? {
        let (title, description, url) = row?;
        if title.iter().chain(description.iter()).any(|text| !text.chars().all(char::is_whitespace)) || linkedin_url(&url) { return Ok(true) }
    }
    Ok(false)
}

fn linkedin_url(url: &str) -> bool {
    use std::sync::LazyLock;
    static PATTERN: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r#"https?://(?:www\.)?linkedin\.com/(?:posts/[^/?#\s<>"'()\]]+|feed/update/urn:li:(?:activity|share|ugcPost):[0-9]+(?:$|[/?#\s<>"'()\].,;:!?}]))"#).unwrap());
    PATTERN.is_match(url)
}
