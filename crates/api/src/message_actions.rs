//! The S2 message actions: edit, delete, reactions and boosts, pins, saved items and forwards
//! (`campfire_api_types`' `message`, `reaction` and `actions` modules document each one). Each
//! runs the classic action's write and broadcasts, so the classic pages see the same frames.

use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_channels::channels::message_features::origin as renderer_origin;
use campfire_db::models::forwarder::{self, Destination};
use campfire_db::{Boost, ChannelThread, Message, MessagePin, Room, SavedItem};
use campfire_kit::{Ctx, Error, Result, StatusCode};
use campfire_messages::controllers::messages::{self as classic, boosts};
use campfire_web::concerns;
use campfire_web::controllers::messages::rendered;
use campfire_web::controllers::presenters::page::{self, db_error};
use campfire_web::messaging::ForwarderCopier;

use crate::dto;
use crate::endpoints::{before_actions, body, now, set_room};
use crate::error::{fail, validation};

endpoint!(
    /// `PATCH /api/v1/messages/:message_id`
    update => update_message
);
endpoint!(
    /// `DELETE /api/v1/messages/:message_id`
    destroy => destroy_message
);
endpoint!(
    /// `GET /api/v1/messages/:message_id/source`
    source => show_source
);
endpoint!(
    /// `POST /api/v1/messages/:message_id/boosts`
    create_boost => post_boost
);
endpoint!(
    /// `DELETE /api/v1/messages/:message_id/boosts/:boost_id`
    destroy_boost => delete_boost
);
endpoint!(
    /// `POST /api/v1/messages/:message_id/pin`
    pin => create_pin
);
endpoint!(
    /// `DELETE /api/v1/messages/:message_id/pin`
    unpin => delete_pin
);
endpoint!(
    /// `GET /api/v1/rooms/:room_id/pins`
    pins => index_pins
);
endpoint!(
    /// `POST /api/v1/saved`
    save => create_saved
);
endpoint!(
    /// `DELETE /api/v1/saved/:saved_id`
    unsave => delete_saved
);
endpoint!(
    /// `GET /api/v1/forward_destinations`
    forward_destinations => index_forward_destinations
);
endpoint!(
    /// `POST /api/v1/messages/:message_id/forwards`
    forward => create_forwards
);

/// The `:name` path parameter as an id; anything else is a 404.
fn id_param(c: &Ctx, name: &str) -> Result<i64> {
    c.param_str(name)
        .and_then(concerns::cast_integer)
        .ok_or(Error::NotFound)
}

/// `Current.user.reachable_messages.find(params[:message_id])`, with its room: a message in an
/// alive room the viewer belongs to.
async fn reachable(c: &mut Ctx) -> Result<(Message, Room)> {
    let user_id = concerns::require_current_user(c)?.id;
    let id = id_param(c, "message_id")?;
    c.app()
        .db
        .read(move |conn| {
            let message = Message::find_reachable(conn, user_id, id)?;
            let room = Room::find(conn, message.room_id)?;
            Ok((message, room))
        })
        .await
        .map_err(db_error)
}

/// A reply in a locked thread can't be edited (`ChannelThread::LockedError`).
async fn ensure_unlocked(c: &mut Ctx, message: &Message) -> Result<()> {
    let Some(thread_id) = message.thread_id else {
        return Ok(());
    };
    let locked = c
        .app()
        .db
        .read(move |conn| Ok(ChannelThread::find(conn, thread_id)?.locked_at.is_some()))
        .await
        .map_err(db_error)?;
    if locked {
        return Err(locked_error(c));
    }
    Ok(())
}

fn locked_error(c: &mut Ctx) -> Error {
    fail(
        c,
        api::ApiError::Forbidden {
            message: campfire_db::channel_thread::LOCKED_MESSAGE.into(),
        },
    )
}

async fn render_message(c: &Ctx, message: Message) -> Result<api::MessageDTO> {
    let app = c.app().clone();
    c.app()
        .db
        .read(move |conn| dto::message(conn, &app, &message))
        .await
        .map_err(db_error)
}

async fn update_message(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (message, room) = reachable(c).await?;
    classic::ensure_can_edit(c, &message)?;
    ensure_unlocked(c, &message).await?;
    let input: api::UpdateMessage = body(c).await?;
    if input.markdown_source.chars().count() > campfire_db::message::SOURCE_LIMIT {
        return Err(fail(
            c,
            validation(
                "markdownSource",
                "is too long (maximum is 50000 characters)",
            ),
        ));
    }
    let thread_id = message.thread_id;
    let updated = match classic::update_markdown_source(
        c,
        thread_id,
        message,
        input.markdown_source,
    )
    .await
    {
        Ok(message) => message,
        Err(Error::Internal(error))
            if matches!(
                error.downcast_ref::<campfire_db::Error>(),
                Some(campfire_db::Error::Other(text)) if text == campfire_db::channel_thread::LOCKED_MESSAGE
            ) =>
        {
            return Err(locked_error(c));
        }
        Err(error) => return Err(error),
    };
    // `messages#update` / `channel_thread_messages#update`'s edit frames.
    if thread_id.is_some() {
        rendered::broadcast_thread_edit(c, &room, &updated, false).await?;
    } else {
        rendered::broadcast_edit(c, &room, &updated, false).await?;
    }
    let dto = render_message(c, updated).await?;
    c.json(StatusCode::OK, &dto)
}

async fn show_source(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (message, _) = reachable(c).await?;
    classic::ensure_can_edit(c, &message)?;
    ensure_unlocked(c, &message).await?;
    let message_id = message.id;
    let markdown_source = classic::present(c, move |presenter| {
        presenter.editable_markdown_source(&message)
    })
    .await?;
    c.json(
        StatusCode::OK,
        &api::MessageSource {
            message_id,
            markdown_source,
        },
    )
}

async fn destroy_message(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (message, room) = reachable(c).await?;
    classic::ensure_can_delete(c, &message)?;
    classic::destroy_message(c, &room, &message).await?;
    // `channel_thread_messages#destroy` refreshes the thread's unread rows.
    if let Some(thread_id) = message.thread_id {
        rendered::broadcast_thread_refresh(c.app(), room.id, thread_id).await?;
    }
    Ok(c.head(StatusCode::NO_CONTENT))
}

/// The classic boost form's `maxlength`.
const BOOST_LIMIT: usize = 16;

/// `Boost::SHORTCODE_CONTENT_PATTERN`.
static SHORTCODE: std::sync::LazyLock<regex::Regex> =
    std::sync::LazyLock::new(|| regex::Regex::new(r"\A:[a-z0-9_]+:\z").expect("pattern"));

async fn reactions_reply(c: &mut Ctx, message_id: i64) -> Result {
    let app = c.app().clone();
    let reactions = c
        .app()
        .db
        .read(move |conn| dto::message_reactions(conn, &app, &Message::find(conn, message_id)?))
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &reactions)
}

async fn post_boost(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (message, _) = reachable(c).await?;
    let input: api::CreateBoost = body(c).await?;
    // The classic form redirects past an invalid boost; the API says why.
    let trimmed = input.content.trim();
    if trimmed.is_empty() {
        return Err(fail(c, validation("content", "can't be blank")));
    }
    // A `:shortcode:` from the icon picker resolves to its icon, so only free text is held to
    // the classic form's 16 characters.
    if !SHORTCODE.is_match(trimmed) && trimmed.chars().count() > BOOST_LIMIT {
        return Err(fail(
            c,
            validation("content", "is too long (maximum is 16 characters)"),
        ));
    }
    // `messages/boosts#create`: a reaction toggles, anything else is a new boost.
    let (message_id, booster_id, app) = (
        message.id,
        concerns::require_current_user(c)?.id,
        c.app().clone(),
    );
    let content = trimmed.to_owned();
    c.app()
        .db
        .write(move |tx| {
            let presenter =
                campfire_web::controllers::presenters::Presenter::new(tx.conn(), &app, None);
            let content =
                campfire_views::messages::reactions::resolve_content(&content, &presenter);
            let reaction =
                campfire_views::messages::reactions::resolve(&content, &presenter).is_some();
            Boost::toggle_reaction(tx, message_id, booster_id, &content, reaction)
        })
        .await
        .map_err(db_error)?;
    boosts::broadcast_reactions(c, &message).await?;
    reactions_reply(c, message_id).await
}

async fn delete_boost(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (message, _) = reachable(c).await?;
    let boost_id = id_param(c, "boost_id")?;
    let (message_id, user_id) = (message.id, concerns::require_current_user(c)?.id);
    // `@message.boosts.find_by!(id: params[:id], booster: Current.user)`
    let boost = c
        .app()
        .db
        .read(move |conn| Boost::find_by_message_and_booster(conn, message_id, boost_id, user_id))
        .await
        .map_err(db_error)?;
    c.app()
        .db
        .write(move |tx| boost.destroy(tx))
        .await
        .map_err(db_error)?;
    boosts::broadcast_reactions(c, &message).await?;
    reactions_reply(c, message_id).await
}

async fn pin_reply(c: &mut Ctx, message: Message, status: StatusCode) -> Result {
    let state = c
        .app()
        .db
        .read(move |conn| dto::pin_state(conn, &message))
        .await
        .map_err(db_error)?;
    c.json(status, &state)
}

async fn create_pin(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (message, _) = reachable(c).await?;
    let pinner = concerns::require_current_user(c)?.id;
    let origin = page::renderer_base_url(c);
    let pinning = message.clone();
    let pinned = c
        .app()
        .db
        .write_scoped(
            move || renderer_origin(&origin),
            move |tx| MessagePin::pin(tx, &pinning, pinner),
        )
        .await
        .map_err(db_error)?;
    if let Err(cap) = pinned {
        return Err(fail(
            c,
            api::ApiError::Validation {
                message: cap.0,
                fields: Default::default(),
            },
        ));
    }
    pin_reply(c, message, StatusCode::CREATED).await
}

async fn delete_pin(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (message, _) = reachable(c).await?;
    let origin = page::renderer_base_url(c);
    let message_id = message.id;
    c.app()
        .db
        .write_scoped(
            move || renderer_origin(&origin),
            move |tx| {
                if let Some(pin) = MessagePin::find_by_message(tx.conn(), message_id)? {
                    pin.unpin(tx)?;
                }
                Ok(())
            },
        )
        .await
        .map_err(db_error)?;
    pin_reply(c, message, StatusCode::OK).await
}

async fn index_pins(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (_, room) = set_room(c).await?;
    let (app, now) = (c.app().clone(), now(c));
    let list = c
        .app()
        .db
        .read(move |conn| dto::pin_list(conn, &app, room.id, now))
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &list)
}

async fn create_saved(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let input: api::SaveMessage = body(c).await?;
    let user_id = concerns::require_current_user(c)?.id;
    let remind_at = match input.remind_at.as_deref() {
        None => None,
        Some(raw) => match raw.parse::<jiff::Timestamp>() {
            Ok(time) => Some(campfire_db::Timestamp::from_jiff(time)),
            Err(_) => return Err(fail(c, validation("remindAt", "is invalid"))),
        },
    };
    let message_id = input.message_id;
    let item = c
        .app()
        .db
        .write(move |tx| {
            // Any message in an alive room the viewer belongs to (`reachable_messages`).
            Message::find_reachable(tx.conn(), user_id, message_id)?;
            SavedItem::save_for(tx, user_id, message_id, remind_at)
        })
        .await
        .map_err(db_error)?;
    let item = dto::saved_item(&item);
    campfire_app::cable::sync::saved_changed(
        &c.app().cable,
        user_id,
        message_id,
        Some(item.clone()),
    );
    c.json(StatusCode::CREATED, &item)
}

async fn delete_saved(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let user_id = concerns::require_current_user(c)?.id;
    let id = id_param(c, "saved_id")?;
    let message_id = c
        .app()
        .db
        .write(move |tx| {
            let item = SavedItem::accessible_to(tx.conn(), user_id)?
                .into_iter()
                .find(|item| item.id == id)
                .ok_or(campfire_db::Error::RecordNotFound("SavedItem"))?;
            item.destroy(tx)?;
            Ok(item.message_id)
        })
        .await
        .map_err(db_error)?;
    campfire_app::cable::sync::saved_changed(&c.app().cable, user_id, message_id, None);
    Ok(c.head(StatusCode::NO_CONTENT))
}

async fn index_forward_destinations(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let viewer = concerns::require_current_user(c)?.clone();
    let now = now(c);
    let list = c
        .app()
        .db
        .read(move |conn| dto::forward_destinations(conn, &viewer, now))
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &list)
}

async fn create_forwards(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (original, _) = reachable(c).await?;
    let input: api::CreateForwards = body(c).await?;
    let destinations: Vec<Destination> = input
        .destinations
        .iter()
        .map(|target| Destination {
            room_id: Some(target.room_id),
            thread_id: target.thread_id,
        })
        .collect();
    let note = input
        .note
        .filter(|note| !campfire_views::helpers::is_blank(note));
    let copier = ForwarderCopier::new(c.app().storage.clone());
    let creator = concerns::require_current_user(c)?.id;
    let results = c
        .app()
        .db
        .write(move |tx| {
            // The source is checked again alongside the destination writes.
            Message::find_reachable(tx.conn(), creator, original.id)?;
            forwarder::forward(
                tx,
                &original,
                &destinations,
                note.as_deref(),
                creator,
                &copier,
            )
        })
        .await;
    let refused = |message: String| api::ApiError::Validation {
        message,
        fields: Default::default(),
    };
    let results = match results {
        Ok(Ok(results)) => results,
        Ok(Err(refusal)) => return Err(fail(c, refused(refusal.to_string()))),
        Err(campfire_db::Error::Other(message))
            if ["You cannot forward to that room", "That thread is locked"]
                .contains(&message.as_str()) =>
        {
            return Err(fail(c, refused(message)));
        }
        Err(error) => return Err(db_error(error)),
    };
    let mut forwards = Vec::with_capacity(results.len());
    for result in results {
        let id = result.message.id;
        let record = c
            .app()
            .db
            .read(move |conn| Message::find(conn, id))
            .await
            .map_err(db_error)?;
        classic::broadcast_create(c, &result.room, &record).await?;
        forwards.push(record);
    }
    let app = c.app().clone();
    let forwards = c
        .app()
        .db
        .read(move |conn| dto::messages(conn, &app, &forwards))
        .await
        .map_err(db_error)?;
    c.json(StatusCode::CREATED, &api::ForwardResult { forwards })
}
