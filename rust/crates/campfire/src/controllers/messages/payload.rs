//! `MessagePayloadHelper`: request-specific JSON, never a shared fragment-cache value.
use campfire_db::{ChannelThread, Message, MessagePin, SavedItem, Timestamp, User};
use campfire_views::helpers::{AvatarIcon, IconSource};
use campfire_views::messages::support::json_time;
use serde_json::{Value, json};

use crate::controllers::presenters::{Presenter, Result, avatar_path};

pub(crate) fn message(p: &Presenter<'_>, message: &Message, viewer: &User, base: &str) -> Result<Value> {
    let room = payload_room(p, message.room_id)?;
    let mut body = json!({"plain_text": p.plain_text_body(message)?, "html": html(p, message)?});
    if let Some(source) = &message.markdown_source { body["markdown_source"] = source.clone().into(); }
    let mut result = json!({
        "id": message.id, "client_message_id": message.client_message_id,
        "created_at": json_time(message.created_at.jiff()), "updated_at": json_time(message.updated_at.jiff()),
        "body": body, "creator": user(p, &p.user(message.creator_id)?, base)?,
        "room": {"id": message.room_id, "icon_name": room.icon_name}
    });
    if let Some(id) = message.thread_id {
        let record = match p.search_preloads.as_ref().and_then(|d|d.threads.as_ref()) {
            Some(data)=>data.threads.get(&id).cloned().ok_or(campfire_db::Error::RecordNotFound("ChannelThread"))?,
            None=>ChannelThread::find(p.conn,id)?,
        };
        result["thread_context"] = thread(p, &record, viewer, base)?;
    }
    let summary = match p.search_preloads.as_ref().and_then(|d|d.threads.as_ref()) {
        Some(data)=>data.by_parent.get(&message.id).and_then(|id|data.threads.get(id)).cloned(),
        None=>ChannelThread::find_by_parent_message(p.conn,message.id)?,
    };
    if let Some(summary) = summary {
        result["thread_summary"] = thread(p, &summary, viewer, base)?;
    }
    if message.reply_to_message_id.is_some() || message.reply_target_deleted_at.is_some() {
        let source = match &p.search_preloads {
            Some(data)=>message.reply_to_message_id.and_then(|id|data.records.sources.get(&id)).cloned(),
            None=>message.reply_to_message_id.map(|id|Message::find_by_id(p.conn,id)).transpose()?.flatten(),
        };
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
    let files=match &p.search_preloads {
        Some(data)=>data.records.drive_files.get(&message.id).cloned().unwrap_or_default(),
        None=>message.drive_file_ids(p.conn)?,
    };
    result["drive_attachments"] = files.into_iter().map(|id| json!({"file_id": id, "url": format!("https://drive.google.com/open?id={id}")})).collect::<Vec<_>>().into();
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

fn payload_room(p:&Presenter<'_>,id:i64) -> Result<campfire_db::Room> {
    match &p.search_preloads {
        Some(data)=>data.records.rooms.get(&id).cloned().ok_or(campfire_db::Error::RecordNotFound("Room")),
        None=>campfire_db::Room::find(p.conn,id),
    }
}

fn html(p: &Presenter<'_>, message: &Message) -> Result<String> {
    if message.markdown() {
        let resolver = p.resolver();
        let body = p.stored_body(message)?.unwrap_or_default();
        let context = resolver.render_context(p.request_host.clone());
        match &p.search_preloads {
            Some(data) => campfire_richtext::markdown::presentation(&body, &context, &data.icons, None)
                .map_err(|error| campfire_db::Error::Other(error.to_string())),
            None => crate::rich_text::markdown_presentation(p.conn, &body, &context)
                .map_err(campfire_db::Error::Other),
        }
    } else { p.editable_body(message) }
}

fn permalink(message: &Message, base: &str) -> String {
    format!("{base}{}", campfire_db::message_pin::message_path(message))
}

pub(crate) fn user(p: &Presenter<'_>, user: &User, base: &str) -> Result<Value> {
    let icon = match &p.search_preloads {
        Some(data)=>data.users.get(&user.id).and_then(|u|u.icon_name.clone()),
        None=>p.conn.query_row("SELECT icon_name FROM users WHERE id = ?", [user.id], |row|row.get::<_,Option<String>>(0))?,
    };
    let icon_url = icon.as_deref().and_then(|name| p.resolve_avatar_icon(name)).and_then(|icon| match icon {
        AvatarIcon::Image {url, brand, ..} => Some(if brand { campfire_assets::asset_path(&url) } else { url }), _ => None,
    });
    Ok(json!({"id": user.id, "name": user.name, "role": user.role.name(),
        "avatar_url": format!("{base}{}", avatar_path(p.secrets, user)), "icon_name": icon, "icon_avatar_url": icon_url}))
}

pub(crate) fn thread(p: &Presenter<'_>, thread: &ChannelThread, viewer: &User, base: &str) -> Result<Value> {
    let facts=p.search_preloads.as_ref().and_then(|d|d.threads.as_ref());
    let member=match facts {Some(d)=>d.members.get(&thread.id).cloned(),None=>thread.membership_for(p.conn,viewer.id)?};
    let room=payload_room(p,thread.room_id)?;
    let settings=thread.settings_manageable_in_room(&room,viewer);
    let lifecycle=thread.manageable_in_room(&room,viewer);
    let viewer_member=match facts {Some(d)=>d.room_members.contains(&(room.id,viewer.id)),None=>campfire_db::Membership::find_by_room_and_user(p.conn,room.id,viewer.id)?.is_some()};
    let work_assignment=thread.work_assignment_manageable_in_room(&room,viewer,viewer_member);
    let work_manageable=thread.work_manageable_in_room(&room,viewer,viewer_member);
    let owner=thread.work_owner_id.map(|id|p.user(id)).transpose()?;
    let owner_active=match &owner {
        Some(owner) if owner.is_active()=> {
            let member=match facts {Some(d)=>d.room_members.contains(&(room.id,owner.id)),None=>campfire_db::Membership::find_by_room_and_user(p.conn,room.id,owner.id)?.is_some()};
            member && if owner.is_bot(){match facts {Some(d)=>d.posting.contains(&(owner.id,room.id)),None=>agent_may_post(p,owner.id,&room)?}}else{true}
        }
        _=>false,
    };
    let owner = owner.map(|owner| -> Result<Value> {
        let mut value = user(p, &owner, base)?;
        value["active"] = owner.is_active().into(); value["human"] = (!owner.is_bot()).into(); value["agent"] = owner.is_bot().into();
        Ok(value)
    }).transpose()?;
    let member_count = match facts {Some(d)=>d.member_counts.get(&thread.id).copied().unwrap_or(0),None=>p.conn.query_row("SELECT count(*) FROM thread_memberships WHERE thread_id = ?",[thread.id],|r|r.get::<_,i64>(0))?};
    let message_count=match facts {Some(d)=>d.message_counts.get(&thread.id).copied().unwrap_or(0),None=>thread.message_count(p.conn)?};
    Ok(json!({
        "id": thread.id, "name": thread.name, "status": thread.status_in_room(&room, Timestamp::from_jiff(p.now)).name(),
        "room_id": thread.room_id, "parent_message_id": thread.parent_message_id,
        "last_activity_at": json_time(thread.last_activity_at.jiff()),
        "closed_at": thread.closed_at.map(|time| json_time(time.jiff())), "locked_at": thread.locked_at.map(|time| json_time(time.jiff())),
        "auto_archive_after_minutes": thread.auto_archive_after_minutes, "work": thread.work(), "work_status": thread.work_status,
        "work_owner_id": thread.work_owner_id, "work_owner": owner, "work_owner_active": owner_active,
        "work_history": Value::Null, "work_owner_options": Value::Null,
        "joined": member.is_some(), "unread": member.as_ref().map(|member| member.unread()),
        "involvement": member.as_ref().map(|member| member.involvement.name()),
        "message_count": message_count, "member_count": member_count,
        "creator": user(p, &p.user(thread.creator_id)?, base)?, "url": format!("{base}/rooms/{}/threads/{}", thread.room_id, thread.id),
        "permalink_url": format!("{base}/rooms/{}?thread={}", thread.room_id, thread.id),
        "permissions": {"can_rename": settings, "can_close": settings, "can_reopen": if thread.locked_at.is_some() {lifecycle} else {member.is_some()},
            "can_lock": lifecycle, "can_unlock": lifecycle, "can_delete": lifecycle, "can_convert_work": !thread.work() && work_assignment,
            "can_manage_work": thread.work() && work_manageable, "can_update_work_status": thread.work() && work_manageable,
            "can_assign_work": thread.work() && work_assignment, "can_remove_work": thread.work() && work_assignment && !room.board()}
    }))
}

/// ChannelThreadsController#show asks for these two additional read-only facts. Keep the
/// ordinary thread/message payload entry points unchanged for the other feature workers.
pub(crate) fn thread_details(p: &Presenter<'_>, record: &ChannelThread, viewer: &User, base: &str) -> Result<Value> {
    let mut value = thread(p, record, viewer, base)?;
    let room = record.room(p.conn)?;
    let (humans, agents) = ChannelThread::work_owner_candidates_for(p.conn, room.id)?;
    let choices = humans.into_iter().chain(agents).map(|owner| -> Result<Value> {
        let mut entry = user(p, &owner, base)?;
        entry["active"] = true.into(); entry["human"] = (!owner.is_bot()).into(); entry["agent"] = owner.is_bot().into();
        if owner.is_bot() {
            let agent = campfire_db::Agent::for_user(p.conn, owner.id)?.ok_or(campfire_db::Error::RecordNotFound("Agent"))?;
            entry["provider"] = json!(agent.provider); entry["description"] = json!(agent.description);
            entry.as_object_mut().expect("user payload").retain(|_, value| !value.is_null());
        }
        Ok(entry)
    }).collect::<Result<Vec<Value>>>()?;
    value["work_owner_options"] = choices.into();
    let mut query = p.conn.prepare("SELECT id, event_type, created_at, actor_id, from_status, to_status, from_owner_id, from_owner_name, to_owner_id, to_owner_name, metadata FROM work_thread_events WHERE channel_thread_id = ? ORDER BY created_at DESC, id DESC")?;
    let events = query.query_map([record.id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?,
        row.get::<_, Timestamp>(2)?, row.get::<_, Option<i64>>(3)?, row.get::<_, Option<String>>(4)?, row.get::<_, Option<String>>(5)?,
        row.get::<_, Option<i64>>(6)?, row.get::<_, Option<String>>(7)?, row.get::<_, Option<i64>>(8)?, row.get::<_, Option<String>>(9)?, row.get::<_, Option<String>>(10)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let history = events.into_iter().map(|(id, kind, time, actor, before, after, from_id, from_name, to_id, to_name, metadata)| {
        let mut event = json!({"id": id, "event_type": kind, "created_at": json_time(time.jiff()),
            "actor": actor.map(|id| p.user(id).and_then(|user_record| user(p, &user_record, base))).transpose()?,
            "before": {"status": before, "owner": owner_state(from_id, from_name)},
            "after": {"status": after, "owner": owner_state(to_id, to_name)}});
        event.as_object_mut().expect("event payload").retain(|_, value| !value.is_null());
        if let Some(note) = metadata.as_deref().and_then(|text| serde_json::from_str::<Value>(text).ok())
            .and_then(|value| value["note"].as_str().filter(|note| !note.trim().is_empty()).map(str::to_string)) {
            event["note"] = note.into();
        }
        Ok(event)
    }).collect::<Result<Vec<Value>>>()?;
    value["work_history"] = history.into();
    Ok(value)
}

fn owner_state(id: Option<i64>, name: Option<String>) -> Value {
    if id.is_none() && name.as_deref().is_none_or(str::is_empty) { return Value::Null; }
    let mut owner = json!({"id": id, "name": name});
    owner.as_object_mut().expect("owner state").retain(|_, value| !value.is_null());
    owner
}

fn agent_may_post(p: &Presenter<'_>, user: i64, room: &campfire_db::Room) -> Result<bool> {
    if room.deleted_at.is_some() { return Ok(false) }
    let Some(agent) = campfire_db::Agent::for_user(p.conn, user)? else { return Ok(false) };
    Ok(agent.active(p.conn)? && agent.can(p.conn, "post_messages", Some(room.id))?)
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

#[cfg(test)]
mod unicode_tests {
    use super::*;
    use crate::controllers::presenters::test_support::{DAVID, KEVIN, TestApp};
    use campfire_db::{Agent, NewAgent, NewChannelThread, Room, RoomType};

    #[tokio::test]
    async fn unicode_parity_thread_payload_sorts_human_and_agent_owner_options() {
        let app = TestApp::boot_frozen()
            .await
            .expect("default seed required")
            .without_job_runner()
            .await;
        let thread = app
            .booted
            .app
            .db
            .write(|tx| {
                tx.conn()
                    .execute("UPDATE users SET name='ΟΣ' WHERE id=?", [DAVID])?;
                tx.conn()
                    .execute("UPDATE users SET name='οςa' WHERE id=?", [KEVIN])?;
                let mut members = vec![DAVID, KEVIN];
                for name in ["ΟΣ", "οςa"] {
                    let user = User::create_integration_bot(tx, name)?;
                    Agent::create(
                        tx,
                        NewAgent {
                            user_id: user.id,
                            owner_id: Some(DAVID),
                            ..Default::default()
                        },
                    )?;
                    members.push(user.id);
                }
                let room = Room::create_for(tx, RoomType::Closed, Some("Casing"), DAVID, &members)?;
                ChannelThread::create(
                    tx,
                    NewChannelThread {
                        room_id: room.id,
                        creator_id: DAVID,
                        name: Some("Casing".into()),
                        ..Default::default()
                    },
                )
            })
            .await
            .unwrap();
        let state = app.booted.app.clone();
        let presenter_state = state.clone();
        let payload = state
            .db
            .read(move |c| {
                let p = Presenter::new(c, &presenter_state, None);
                thread_details(&p, &thread, &User::find(c, DAVID)?, "http://campfire.test")
            })
            .await
            .unwrap();
        let names = payload["work_owner_options"]
            .as_array()
            .unwrap()
            .iter()
            .map(|owner| owner["name"].clone())
            .collect::<Vec<_>>();
        let oracle: Value = serde_json::from_str(include_str!(
            "../../../../../vectors/unicode_casing_parity.json"
        ))
        .unwrap();
        let mut expected = oracle["sigma_names"].as_array().unwrap().clone();
        expected.extend(expected.clone());
        assert_eq!(names, expected);
        assert!(payload["work_owner_options"][0]["human"].as_bool().unwrap());
        assert!(payload["work_owner_options"][2]["agent"].as_bool().unwrap());
    }
}
