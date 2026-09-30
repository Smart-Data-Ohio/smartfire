//! Author-only embed removal, scoped through reachable conversations.
use crate::{
    app::AppCtx,
    concerns::{self, Before, before_actions, cast_integer, require_current_user},
    controllers::presenters::{link_embeds, page::db_error},
};
use campfire_db::{Message, Timeline};
use campfire_kit::{Ctx, Error, Result, StatusCode, format};

pub async fn create(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let user_id = require_current_user(c)?.id;
    let id = c.param_str("message_id").and_then(cast_integer).ok_or(Error::NotFound)?;
    let room = if c.param_str("room_id").is_some() {
        Some(concerns::set_room(c).await?.1)
    } else {
        None
    };
    let thread = c
        .param_str("thread_id")
        .map(|value| cast_integer(value).ok_or(Error::NotFound))
        .transpose()?;
    let app = c.app().clone();
    let app2 = app.clone();
    let message=app.db.write(move |tx| {
        let mut message=if let Some(room)=room {
            let timeline=if let Some(thread)=thread {
                let belongs:bool=tx.conn().query_row("SELECT EXISTS(SELECT 1 FROM channel_threads WHERE id=?1 AND room_id=?2)",rusqlite::params![thread,room.id],|row| row.get(0))?;
                if !belongs { return Err(campfire_db::Error::RecordNotFound("ChannelThread")); }
                Timeline::Thread(thread)
            } else { Timeline::Room(room.id) };
            Message::find_in(tx.conn(),timeline,id)?
        } else { Message::find_reachable(tx.conn(),user_id,id)? };
        let locked=message.thread_id.map(|thread| tx.conn().query_row("SELECT locked_at IS NOT NULL FROM channel_threads WHERE id=?",[thread],|row| row.get::<_,bool>(0))).transpose()?.unwrap_or(false);
        if message.creator_id!=user_id || message.system_note || locked { return Ok(None); }
        message.suppress_embeds(tx)?;
        Ok(Some(message))
    }).await.map_err(db_error)?;
    let Some(message) = message else {
        return Ok(c.head(StatusCode::FORBIDDEN));
    };
    let permalink = if let Some(thread) = message.thread_id {
        campfire_routes::ROOM.path_with(
            &[&message.room_id],
            None,
            &[("thread", Some(&thread.to_string())), ("message_id", Some(&message.id.to_string()))],
        )
    } else {
        campfire_routes::room_at_message(message.room_id, message.id)
    };
    let targets = [
        crate::channels::broadcasts::message_dom_id(&message, Some("linkedin_cards")),
        crate::channels::broadcasts::message_dom_id(&message, Some("link_embed_cards")),
    ];
    let (linkedin, generic) = app
        .db
        .read(move |conn| {
            link_embeds::broadcast_message(&app2, conn, &message, true)?;
            link_embeds::broadcast_message(&app2, conn, &message, false)?;
            Ok((
                link_embeds::container(&app2, conn, &message, true)?,
                link_embeds::container(&app2, conn, &message, false)?,
            ))
        })
        .await
        .map_err(db_error)?;
    c.no_store();
    let accepted = c.respond_to(&[&format::TURBO_STREAM, &format::JSON, &format::HTML])?;
    if accepted == &format::JSON {
        Ok(c.render_as(StatusCode::OK, campfire_kit::response::JSON_UTF8, "{\"embeds_suppressed\":true}"))
    } else if accepted == &format::TURBO_STREAM {
        let html = [(&targets[0], linkedin), (&targets[1], generic)]
            .map(|(target, html)| {
                campfire_cable::turbo::action_tag(
                    campfire_cable::turbo::Action::Replace,
                    campfire_cable::turbo::Target::Target(target),
                    Some(&html),
                    &[],
                )
            })
            .concat();
        Ok(c.render(StatusCode::OK, &format::TURBO_STREAM, html))
    } else {
        c.redirect_to(&c.url_for(&permalink))
    }
}

#[cfg(test)]
mod tests;
