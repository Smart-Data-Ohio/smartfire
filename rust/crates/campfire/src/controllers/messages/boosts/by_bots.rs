//! `Messages::Boosts::ByBotsController` (reference/app/controllers/messages/boosts/by_bots_controller.rb):
//! bots boost with the raw request body as the content.

use campfire_db::{Boost, Message, Room};
use campfire_kit::{Ctx, Error, Result, StatusCode, format, halt};

use super::{broadcast_create, destroy_boost, set_boost};
use crate::app::AppCtx;
use crate::concerns::{self, Before, before_actions, cast_integer, require_current_user};
use crate::controllers::messages::by_bots::{deny_bot_reply_token, is_blank, raw_request_body};
use crate::controllers::messages::present;
use crate::controllers::presenters::page::db_error;

fn before() -> Before {
    Before::default().allow_bot_access()
}

pub async fn create(c: &mut Ctx) -> Result {
    before_actions(c, before()).await?;
    let message = set_message(c).await?;
    deny_bot_reply_token(c)?;
    concerns::ensure_agent_capability(c, "react", message.room_id).await?;
    // ensure_content_present
    let content = raw_request_body(c);
    if is_blank(&content) {
        return halt(concerns::head(StatusCode::UNPROCESSABLE_ENTITY));
    }
    let (message_id,user_id)=(message.id,require_current_user(c)?.id);
    let boost = match c.app().db.write(move|tx|Boost::create(tx,message_id,user_id,&content)).await {
        Ok(boost)=>boost,
        Err(campfire_db::Error::RecordInvalid(errors))=>return Ok(c.render(StatusCode::UNPROCESSABLE_ENTITY,&format::JSON,serde_json::json!({"errors":errors.full_messages()}).to_string())),
        Err(error)=>return Err(db_error(error)),
    };
    broadcast_create(c, &message, &boost).await?;

    // render :show, status: :created
    c.respond_to(&[&format::JSON])?;
    let base_url = c.url_for("");
    let body = present(c, move |presenter| {
        let mut payload=serde_json::to_value(presenter.boost_json(&boost,&message,&base_url)?).map_err(|e|campfire_db::Error::Other(e.to_string()))?;
        payload["booster"]=presenter.user_payload(boost.booster_id)?;
        Ok(campfire_views::helpers::to_rails_json(&payload))
    }).await?;
    Ok(c.render(StatusCode::CREATED, &format::JSON, body))
}

pub async fn destroy(c: &mut Ctx) -> Result {
    before_actions(c, before()).await?;
    let message = set_message(c).await?;
    // set_boost, with `rescue ActiveRecord::RecordNotFound` → head :not_found
    let boost = match set_boost(c, &message).await {
        Ok(boost) => boost,
        Err(Error::NotFound) => return Ok(concerns::head(StatusCode::NOT_FOUND)),
        Err(error) => return Err(error),
    };
    deny_bot_reply_token(c)?;
    concerns::ensure_agent_capability(c, "react", message.room_id).await?;
    destroy_boost(c, &message, boost).await?;
    Ok(c.head(StatusCode::NO_CONTENT))
}

/// The room among `Current.user.rooms`, then its message; `head :not_found` without one.
async fn set_message(c: &mut Ctx) -> Result<Message> {
    let user_id = require_current_user(c)?.id;
    let room_id = c.param_str("room_id").and_then(cast_integer);
    let message_id = c.param_str("message_id").and_then(cast_integer);
    let message = c
        .app()
        .db
        .read(move |conn| {
            let Some(room) = room_id.map(|id| Room::find_for_user(conn, user_id, id)).transpose()?.flatten() else {
                return Ok(None);
            };
            match message_id {
                Some(id) => Ok(Message::find_by_id(conn, id)?.filter(|message| message.room_id == room.id)),
                None => Ok(None),
            }
        })
        .await
        .map_err(db_error)?;
    match message {
        Some(message) => Ok(message),
        None => halt(concerns::head(StatusCode::NOT_FOUND)),
    }
}
