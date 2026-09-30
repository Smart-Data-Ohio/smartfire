//! `Messages::ByBotsController` (reference/app/controllers/messages/by_bots_controller.rb): the
//! bot API under `/rooms/:room_id/:bot_key/messages` (JSON by route default). Bodies are the raw
//! request body (`RawRequestBody`), or a top-level multipart `attachment`.

use campfire_db::{Message, Room, Timeline};
use campfire_kit::{Ctx, Param, Response, Result, StatusCode, format, halt, permit_keys};

use super::{
    MessageParams, attachment_assignment, broadcast_create, broadcast_replace, create_message_with_agent_policy, destroy_message, release_webhooks,
    ensure_can_administer, find_paged_messages, present, set_message, update_message,
};
use crate::app::AppCtx;
use crate::concerns::{self, Before, before_actions, cast_integer, require_current_user};
use crate::controllers::presenters::page::db_error;

fn before() -> Before {
    Before::default().allow_bot_access()
}

pub async fn index(c: &mut Ctx) -> Result {
    before_actions(c, before()).await?;
    deny_bot_reply_token(c)?;
    let room = set_room(c).await?;
    concerns::ensure_agent_capability(c, "read_messages", room.id).await?;
    let messages = find_paged_messages(c, &room).await?;
    set_pagination_headers(c, &room, &messages).await?;
    c.respond_to(&[&format::JSON])?;
    let body = present(c, move |presenter| {
        let messages = messages.iter().map(|m| presenter.agent_message_payload(m)).collect::<campfire_db::Result<Vec<_>>>()?;
        Ok(campfire_views::helpers::to_rails_json(&messages))
    })
    .await?;
    Ok(c.render(StatusCode::OK, &format::JSON, body))
}

pub async fn create(c: &mut Ctx) -> Result {
    before_actions(c, before()).await?;
    let room = set_room(c).await?;
    concerns::ensure_agent_capability(c, "post_messages", room.id).await?;
    ensure_body_or_attachment_present(c)?;
    if room.board() {
        return Ok(c.head(StatusCode::UNPROCESSABLE_ENTITY));
    }
    let input=c.params.get("message").and_then(|p|p.get("client_message_id")).map(Param::to_json);
    let client_id=campfire_db::models::agent_posting::client_ids::ClientId::from_json(input.as_ref());
    let client_message_id=client_id.stored;
    // Posting's replay/budget checks precede attachment validation and staging. The final
    // write repeats the check in its transaction so concurrent posts cannot overrun a cap.
    let (user_id, room_id, replay_id) = (require_current_user(c)?.id, room.id, client_message_id.clone());
    let lookup=client_id.lookup.clone();
    let preflight = c.app().db.write(move |tx| campfire_db::models::agent_posting::prepare_for_user_with_lookup(tx, user_id, room_id, replay_id.as_deref(), &lookup)).await.map_err(db_error)?;
    match preflight {
        Some(campfire_db::models::agent_posting::PostingCheck::Replay(message)) => {
            let location = c.url_for(&campfire_routes::message(message.id));
            return c.head_with_location(StatusCode::CREATED, &location);
        }
        Some(campfire_db::models::agent_posting::PostingCheck::Budget(payload)) => return Ok(c.render(StatusCode::TOO_MANY_REQUESTS, &format::JSON, payload.to_string())),
        _ => {},
    }
    let mut attributes = message_params(c)?;
    attributes.client_message_id = client_message_id;
    attributes.client_message_lookup = client_id.lookup;
    let message = match create_message_with_agent_policy(c, &room, attributes, true).await? {
        campfire_db::models::agent_posting::PostingOutcome::Created(message) => {
            broadcast_create(c, &room, &message).await?;
            release_webhooks(c, &message).await;
            message
        }
        campfire_db::models::agent_posting::PostingOutcome::Replay(message) => message,
        campfire_db::models::agent_posting::PostingOutcome::Budget(payload) => return Ok(c.render(StatusCode::TOO_MANY_REQUESTS, &format::JSON, payload.to_string())),
    };

    let location = c.url_for(&campfire_routes::message(message.id));
    c.head_with_location(StatusCode::CREATED, &location)
}

pub async fn update(c: &mut Ctx) -> Result {
    before_actions(c, before()).await?;
    deny_bot_reply_token(c)?;
    let room = set_room(c).await?;
    let message = set_message(c, &room).await?;
    ensure_can_manage_bot_message(c, &message)?;
    concerns::ensure_agent_capability(c, "post_messages", room.id).await?;
    // MessagesController#update
    let attributes = message_params(c)?;
    let message = update_message(c, message, attributes).await?;
    broadcast_replace(c, &room, &message).await?;
    match c.respond_to(&[&format::HTML, &format::JSON])? {
        f if *f == format::JSON => render_show(c, message).await,
        _ => {
            let url = c.url_for(&campfire_routes::room_message(room.id, message.id));
            c.redirect_to(&url)
        }
    }
}

pub async fn destroy(c: &mut Ctx) -> Result {
    before_actions(c, before()).await?;
    deny_bot_reply_token(c)?;
    let room = set_room(c).await?;
    let message = set_message(c, &room).await?;
    ensure_can_manage_bot_message(c, &message)?;
    concerns::ensure_agent_capability(c, "post_messages", room.id).await?;
    destroy_message(c, &room, &message).await?;
    Ok(c.head(StatusCode::NO_CONTENT))
}

/// `deny_bot_reply_token`: a short-lived webhook reply token authenticates create only.
pub(crate) fn deny_bot_reply_token(c: &Ctx) -> Result<()> {
    if concerns::authenticated_by(c) == concerns::AuthenticatedBy::BotReply {
        return halt(concerns::head(StatusCode::FORBIDDEN));
    }
    Ok(())
}

fn ensure_can_manage_bot_message(c: &mut Ctx, message: &Message) -> Result<()> {
    if message.system_note {
        return halt(concerns::head(StatusCode::FORBIDDEN));
    }
    ensure_can_administer(c, message)
}

/// `set_room`: `Current.user.rooms.find_by(id: params[:room_id])`, else `head :not_found`.
async fn set_room(c: &mut Ctx) -> Result<Room> {
    let user_id = require_current_user(c)?.id;
    let room = match c.param_str("room_id").and_then(cast_integer) {
        Some(id) => c.app().db.read(move |conn| Room::find_for_user(conn, user_id, id)).await.map_err(db_error)?,
        None => None,
    };
    match room {
        Some(room) => Ok(room),
        None => halt(concerns::head(StatusCode::NOT_FOUND)),
    }
}

/// `head :unprocessable_content if params[:attachment].blank? && raw_request_body.blank?`
fn ensure_body_or_attachment_present(c: &mut Ctx) -> Result<()> {
    let attachment_blank = c.params.get("attachment").is_none_or(Param::is_blank);
    if attachment_blank && is_blank(&raw_request_body(c)) {
        return halt(concerns::head(StatusCode::UNPROCESSABLE_ENTITY));
    }
    Ok(())
}

/// `params[:attachment] ? params.permit(:attachment) : { body: raw_request_body }`
fn message_params(c: &Ctx) -> Result<MessageParams> {
    if c.params.get("attachment").is_some_and(|p| !p.is_null()) {
        let permitted = c.params.permit(&permit_keys(&["attachment"]));
        Ok(MessageParams { clear_markdown_source: true, attachment: attachment_assignment(&permitted)?, ..MessageParams::default() })
    } else {
        Ok(MessageParams { clear_markdown_source: true, body: Some(raw_request_body(c)), ..MessageParams::default() })
    }
}

/// `RawRequestBody#raw_request_body`: the whole body, as UTF-8.
pub(crate) fn raw_request_body(c: &Ctx) -> String {
    String::from_utf8_lossy(c.request.raw_post()).into_owned()
}

/// `String#blank?`: empty or only whitespace.
pub(crate) fn is_blank(value: &str) -> bool {
    value.chars().all(char::is_whitespace)
}

/// `X-Total-Count`, and a `Link` to the next page when there is one.
async fn set_pagination_headers(c: &mut Ctx, room: &Room, messages: &[Message]) -> Result<()> {
    let after = c.params.get("after").is_some_and(Param::is_present);
    let (room_id, first, last) = (room.id, messages.first().cloned(), messages.last().cloned());
    let (count, next_page) = c
        .app()
        .db
        .read(move |conn| {
            let count = Message::count_roots_in_room(conn, room_id)?;
            let next_page = match (first, last) {
                (Some(_), Some(last)) if after => {
                    Message::exists_after(conn, Timeline::Room(room_id), &last)?.then_some(("after", last.id))
                }
                (Some(first), Some(_)) => Message::exists_before(conn, Timeline::Room(room_id), &first)?.then_some(("before", first.id)),
                _ => None,
            };
            Ok((count, next_page))
        })
        .await
        .map_err(db_error)?;
    c.set_header("x-total-count", &count.to_string());
    if let Some((key, id)) = next_page {
        let bot_key = c.param_str("bot_key").unwrap_or_default().to_string();
        let url = c.url_for(&format!("{}?{key}={id}", campfire_routes::room_bot_messages(room_id, bot_key)));
        c.set_header("link", &format!("<{url}>; rel=\"next\""));
    }
    Ok(())
}

/// `render :show` (`messages/by_bots/show.json.jbuilder`).
async fn render_show(c: &mut Ctx, message: Message) -> Result<Response> {
    let body = present(c, move |presenter| Ok(presenter.agent_message_payload(&message)?.to_string())).await?;
    Ok(c.render(StatusCode::OK, &format::JSON, body))
}
