//! app/controllers/rooms/slash_commands_controller.rb. Registered agent execution uses the WS11 domain dispatcher.
use crate::app::AppCtx;
use crate::concerns::{Before, before_actions, cast_integer, require_current_user};
use crate::controllers::{message_features as features, presenters::page::db_error};
use campfire_db::{
    ChannelThread, Room,
    slash_commands::{self, Context},
};
use campfire_kit::{Ctx, Error, Result, StatusCode};

pub(crate) async fn thread_id(c: &Ctx, room: &Room) -> Result<Option<i64>> {
    let Some(raw) = c.param("thread_id").filter(|p| p.is_present()) else {
        return Ok(None);
    };
    let id = raw
        .to_s()
        .and_then(|s| cast_integer(&s))
        .ok_or(Error::NotFound)?;
    let room_id = room.id;
    c.app()
        .db
        .read(move |conn| {
            let thread = ChannelThread::find(conn, id)?;
            if thread.room_id != room_id {
                return Err(campfire_db::Error::RecordNotFound("ChannelThread"));
            }
            Ok(Some(thread.id))
        })
        .await
        .map_err(db_error)
}
/// App media adapter for built-in posts; kept in the same writer as dispatch.
pub(crate) fn dispatch(
    tx: &mut campfire_db::Tx<'_>,
    context: &Context,
    text: &str,
    storage: std::sync::Arc<campfire_storage::Storage>,
) -> campfire_db::Result<slash_commands::CommandResult> {
    let result = slash_commands::dispatch_in_user_time_zone(tx, context, text)?;
    if let Some(id) = result.message_id {
        let message = campfire_db::Message::find(tx.conn(), id)?;
        crate::messaging::process_message_attachment(tx, storage, &message)?;
    }
    Ok(result)
}
pub async fn create(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = features::room(c).await?;
    features::active_human(c)?;
    c.start_action();
    let thread_id = thread_id(c, &room).await?;
    let context = Context {
        user_id: require_current_user(c)?.id,
        room_id: room.id,
        thread_id,
        huddles_configured: c.app().config.huddles_configured,
    };
    let text = c
        .param("text")
        .map(features::param_string)
        .unwrap_or_default();
    let origin = c.url_for("");
    let storage = c.app().storage.clone();
    let result = c
        .app()
        .db
        .write_scoped(
            move || crate::channels::message_features::slash_origin(&origin),
            move |tx| dispatch(tx, &context, &text, storage),
        )
        .await
        .map_err(db_error)?;
    if let Some(id) = result.message_id {
        let message = c
            .app()
            .db
            .read(move |conn| campfire_db::Message::find(conn, id))
            .await
            .map_err(db_error)?;
        crate::controllers::messages::release_webhooks(c, &message).await;
    }
    let payload = match result.kind.as_str() {
        "posted" => {
            let mut object = serde_json::json!({"status":"posted","message_id":result.message_id});
            if let Some(notice) = result.notice {
                object["notice"] = notice.into();
            }
            object
        }
        "open_url" => serde_json::json!({"status":"open_url","url":result.url}),
        "open_poll" => serde_json::json!({"status":"open_poll"}),
        "start_huddle" => {
            let name =
                crate::controllers::messages::present(c, move |p| p.room_display_name(&room, None))
                    .await?;
            serde_json::json!({"status":"start_huddle","room_id":result.room_id,"room_name":name})
        }
        _ => serde_json::json!({"status":result.kind,"message":result.message}),
    };
    c.json(StatusCode::OK, &payload)
}
