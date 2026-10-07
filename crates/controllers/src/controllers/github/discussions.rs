//! Discuss transport over the transactional thread domain.
use crate::{
    app::AppCtx,
    concerns::{self, Before, cast_integer},
    controllers::presenters::page::db_error,
};
use campfire_kit::{Ctx, Error, Redirect, Result, StatusCode, format};
pub async fn create(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let (_, room) = concerns::set_room(c).await?;
    let user_id = concerns::require_current_user(c)?.id;
    let id = c
        .params.get("pull_request_id").and_then(campfire_kit::Param::to_s).as_deref()
        .and_then(cast_integer)
        .ok_or(Error::NotFound)?;
    let parent = c
        .params.get("message_id").and_then(campfire_kit::Param::to_s).as_deref()
        .and_then(cast_integer)
        .ok_or(Error::NotFound)?;
    let room_id = room.id;
    match c
        .app()
        .db
        .write(move |tx| {
            use crate::integrations::github::threads::{PullRequestThread, discuss};
            // Whether this write starts the thread, for the SPA's `thread.created`.
            let existed = PullRequestThread::for_room_pr(tx.conn(), room_id, id)?.is_some();
            discuss(tx, room_id, user_id, id, parent).map(|mapping| (mapping, !existed))
        })
        .await
    {
        Ok((mapping, created)) => {
            if created {
                c.app().broadcasts.thread_created(mapping.channel_thread_id);
            }
            c.redirect_to_with(
                &campfire_routes::room_thread(room_id, mapping.channel_thread_id),
                Redirect {
                    status: Some(StatusCode::SEE_OTHER),
                    ..Default::default()
                },
            )
        }
        Err(campfire_db::Error::RecordInvalid(errors)) => {
            if c.format()? == Some(&format::JSON) {
                c.json(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    &serde_json::json!({"error":super::subscriptions::sentence(&errors)}),
                )
            } else {
                Ok(concerns::head(StatusCode::UNPROCESSABLE_ENTITY))
            }
        }
        Err(error) => Err(db_error(error)),
    }
}
