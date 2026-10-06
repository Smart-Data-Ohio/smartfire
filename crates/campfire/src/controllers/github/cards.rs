//! `Rooms::Github::PullRequestCardsController`: authorize context before per-viewer GitHub access.
use crate::{
    app::AppCtx,
    concerns::{self, Before, before_actions, cast_integer},
    controllers::presenters::{github, page::db_error, view_context::Layout},
    integrations::github::{blank, pull_requests::viewer_card_context},
};
use campfire_kit::{Ctx, Error, Result};
pub async fn show(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (_, room) = concerns::set_room(c).await?;
    let id = c
        .param_str("id")
        .and_then(cast_integer)
        .ok_or(Error::NotFound)?;
    let message = c.param_str("message_id").filter(|s| !blank(s));
    let thread = c.param_str("thread_id").filter(|s| !blank(s));
    let (message_id, thread_id) = if let Some(message) = message {
        (Some(cast_integer(message).ok_or(Error::NotFound)?), None)
    } else if let Some(thread) = thread {
        (None, Some(cast_integer(thread).ok_or(Error::NotFound)?))
    } else {
        (None, None)
    };
    let context = c
        .app()
        .db
        .read(move |conn| viewer_card_context(conn, room.id, id, message_id, thread_id))
        .await
        .map_err(db_error)?;
    let user_id = concerns::require_current_user(c)?.id;
    let visible = context
        .pull_request
        .visible_to(&c.app().db, &c.app().github_accounts, Some(user_id))
        .await
        .map_err(db_error)?;
    let data = if visible {
        Some(
            c.app()
                .db
                .read(move |conn| {
                    let data = if context.thread.is_some() {
                        github::card_with_files(conn, &context.pull_request, room.id)?
                    } else {
                        github::card(conn, &context.pull_request, room.id)?
                    };
                    Ok((
                        data,
                        context
                            .message
                            .map(|m| campfire_views::github::CardMessage {
                                id: m.id,
                                room_id: m.room_id,
                                thread_id: m.thread_id,
                            }),
                        context.thread.is_some(),
                    ))
                })
                .await
                .map_err(db_error)?,
        )
    } else {
        None
    };
    let frame_id = campfire_views::github::frame_id(id, message_id, thread_id);
    let layout = Layout::load(c).await?;
    let html = layout.render_without_secrets(c, |ctx| {
        Ok(campfire_views::github::viewer_frame(
            ctx,
            &frame_id,
            data.as_ref(),
        ))
    })?;
    Ok(c.html(html))
}
