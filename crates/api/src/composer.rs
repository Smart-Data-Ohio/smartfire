//! The S2 composer endpoints (`campfire_api_types`' `composer` and `attachment` modules document
//! each one): direct uploads, autocomplete, slash commands, preview and scheduled messages.
//! Each reuses the classic action's lookup or write, so the classic pages see the same frames.

use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_channels::channels::message_features::{origin as renderer_origin, slash_origin};
use campfire_controllers::controllers::autocompletable::icons;
use campfire_db::{
    ChannelThread, Message, NewScheduledMessage, Room, ScheduledMessage, UserStatusSettings,
    autocomplete_users, command_suggestions, slash_commands,
};
use campfire_kit::{Ctx, Error, Result, StatusCode};
use campfire_messages::controllers::message_features as features;
use campfire_messages::controllers::messages as classic;
use campfire_web::concerns;
use campfire_web::controllers::presenters::page::{self, db_error};

use crate::dto;
use crate::endpoints::{before_actions, body, now, set_room};
use crate::error::{fail, validation};

endpoint!(
    /// `POST /api/v1/uploads`
    upload => create_upload
);
endpoint!(
    /// `GET /api/v1/autocomplete/users`
    autocomplete_users => index_user_suggestions
);
endpoint!(
    /// `GET /api/v1/autocomplete/icons`
    autocomplete_icons => index_icon_suggestions
);
endpoint!(
    /// `GET /api/v1/icons`
    icons => index_icons
);
endpoint!(
    /// `GET /api/v1/rooms/:room_id/slash_commands`
    slash_commands => index_slash_commands
);
endpoint!(
    /// `POST /api/v1/rooms/:room_id/slash_commands`
    run_slash_command => create_slash_command
);
endpoint!(
    /// `POST /api/v1/rooms/:room_id/messages/preview`
    preview => create_preview
);
endpoint!(
    /// `GET /api/v1/scheduled_messages`
    scheduled_messages => index_scheduled
);
endpoint!(
    /// `POST /api/v1/rooms/:room_id/scheduled_messages`
    schedule => create_scheduled
);
endpoint!(
    /// `PATCH /api/v1/scheduled_messages/:id`
    update_scheduled => update_scheduled_message
);
endpoint!(
    /// `DELETE /api/v1/scheduled_messages/:id`
    cancel_scheduled => destroy_scheduled
);
endpoint!(
    /// `POST /api/v1/scheduled_messages/:id/send_now`
    send_scheduled_now => send_now
);

/// The most people `GET /autocomplete/users` suggests (`autocompletable/users`' page size).
const USER_SUGGESTIONS: i64 = 20;
/// `scheduled_messages_controller`'s refusal while the scheduler holds the row.
const BUSY: &str = "That message is sending right now; try again in a moment.";

fn too_long(c: &mut Ctx) -> Error {
    fail(
        c,
        validation(
            "markdownSource",
            "is too long (maximum is 50000 characters)",
        ),
    )
}

/// An optional integer query parameter; present but not an integer is a 404, as the classic
/// actions' `find` makes it.
fn id_query(c: &Ctx, name: &str) -> Result<Option<i64>> {
    match c.param_str(name).filter(|raw| !raw.is_empty()) {
        None => Ok(None),
        Some(raw) => concerns::cast_integer(raw).map(Some).ok_or(Error::NotFound),
    }
}

// --- Uploads --------------------------------------------------------------------------------------

async fn create_upload(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let input: api::CreateUpload = body(c).await?;
    for (field, value) in [
        ("filename", &input.filename),
        ("checksum", &input.checksum),
        ("contentType", &input.content_type),
    ] {
        if value.trim().is_empty() {
            return Err(fail(c, validation(field, "can't be blank")));
        }
    }
    if input.byte_size < 0 {
        return Err(fail(
            c,
            validation("byteSize", "must be greater than or equal to 0"),
        ));
    }
    let content_type = Some(input.content_type);
    let upload = campfire_web::active_storage::create_direct_upload(
        c,
        input.filename,
        input.byte_size,
        input.checksum,
        content_type,
        campfire_storage::Json::object(),
    )
    .await?;
    c.json(
        StatusCode::CREATED,
        &api::DirectUpload {
            signed_id: upload.signed_id,
            upload_url: upload.path,
        },
    )
}

// --- Autocomplete ---------------------------------------------------------------------------------

/// `@[Name]` for a name only one person in scope has and the token can carry.
fn mention_token(name: &str, unique: bool) -> Option<String> {
    (unique && !name.contains(['[', ']', '\n', '\r'])).then(|| format!("@[{name}]"))
}

async fn index_user_suggestions(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let viewer = concerns::require_current_user(c)?.id;
    let room_id = id_query(c, "roomId")?;
    let query = c
        .param_str("query")
        .map(str::to_string)
        .filter(|query| !query.is_empty());
    let (secrets, now) = (c.app().secrets.clone(), now(c));
    let suggestions = c
        .app()
        .db
        .read(move |conn| {
            let room = room_id
                .map(|id| Room::find_for_user(conn, viewer, id))
                .transpose()?;
            if room_id.is_some() && room.as_ref().is_some_and(Option::is_none) {
                return Ok(None);
            }
            let room = room.flatten().map(|room| room.id);
            let (users, unique) =
                autocomplete_users::page(conn, room, query.as_deref(), 0, USER_SUGGESTIONS)?;
            let settings = UserStatusSettings::for_ids(
                conn,
                &users.iter().map(|user| user.id).collect::<Vec<_>>(),
            )?;
            Ok(Some(
                users
                    .iter()
                    .filter_map(|user| {
                        let settings = settings.get(&user.id)?;
                        Some(api::UserSuggestion {
                            user: dto::user(settings, &secrets, now),
                            mention_token: mention_token(&user.name, unique.contains(&user.name)),
                        })
                    })
                    .collect::<Vec<_>>(),
            ))
        })
        .await
        .map_err(db_error)?
        .ok_or(Error::NotFound)?;
    c.json(StatusCode::OK, &api::UserSuggestionList { suggestions })
}

fn icon(suggestion: icons::Suggestion) -> api::Icon {
    api::Icon {
        name: suggestion.name,
        title: suggestion.title,
        kind: match suggestion.kind {
            "brand" => api::IconKind::Brand,
            "custom" => api::IconKind::Custom,
            _ => api::IconKind::Emoji,
        },
        character: suggestion.character,
        image_url: suggestion.image,
    }
}

async fn index_icon_suggestions(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let query = c.param_str("query").unwrap_or_default().to_string();
    let found = c
        .app()
        .db
        .read(move |conn| icons::suggestions(conn, &query, false))
        .await
        .map_err(db_error)?;
    c.json(
        StatusCode::OK,
        &api::IconList {
            icons: found.into_iter().map(icon).collect(),
        },
    )
}

async fn index_icons(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let found = c.app().db.read(icons::catalog).await.map_err(db_error)?;
    c.json(
        StatusCode::OK,
        &api::IconList {
            icons: found.into_iter().map(icon).collect(),
        },
    )
}

// --- Slash commands -------------------------------------------------------------------------------

/// `rooms/slash_commands#thread_id`: a thread of `room`, or a 404.
async fn room_thread(c: &Ctx, room: &Room, thread_id: Option<i64>) -> Result<Option<i64>> {
    let Some(id) = thread_id else {
        return Ok(None);
    };
    let room_id = room.id;
    c.app()
        .db
        .read(move |conn| match ChannelThread::find_by_id(conn, id)? {
            Some(thread) if thread.room_id == room_id => Ok(Some(thread.id)),
            _ => Err(campfire_db::Error::RecordNotFound("ChannelThread")),
        })
        .await
        .map_err(db_error)
}

async fn index_slash_commands(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (_, room) = set_room(c).await?;
    let thread_id = id_query(c, "threadId")?;
    let in_thread = room_thread(c, &room, thread_id).await?.is_some();
    let room_id = room.id;
    let found = c
        .app()
        .db
        .read(move |conn| command_suggestions::for_room(conn, room_id, in_thread, None))
        .await
        .map_err(db_error)?;
    let commands = found
        .into_iter()
        .map(|command| api::SlashCommand {
            name: command.name,
            description: command.description,
            arg_hint: command.arg_hint,
            takes_arguments: command.takes_arguments,
            agent_name: command.agent,
        })
        .collect();
    c.json(StatusCode::OK, &api::SlashCommandList { commands })
}

/// `rooms/slash_commands#create`: the same dispatch in the same writer, with the posted message's
/// webhooks released after it.
async fn create_slash_command(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (_, room) = set_room(c).await?;
    features::active_human(c)?;
    let input: api::RunSlashCommand = body(c).await?;
    let thread_id = room_thread(c, &room, input.thread_id).await?;
    let context = slash_commands::Context {
        user_id: concerns::require_current_user(c)?.id,
        room_id: room.id,
        thread_id,
        huddles_configured: c.app().config.huddles_configured,
    };
    let origin = c.url_for("");
    let zone = features::user_zone(c).await?;
    let storage = c.app().storage.clone();
    let text = input.text;
    let result = c
        .app()
        .db
        .write_scoped(
            move || (slash_origin(&origin), page::enter_time_zone(zone)),
            move |tx| {
                campfire_rooms::controllers::rooms::slash_commands::dispatch(
                    tx, &context, &text, storage,
                )
            },
        )
        .await
        .map_err(db_error)?;
    if let Some(id) = result.message_id {
        let message = c
            .app()
            .db
            .read(move |conn| Message::find(conn, id))
            .await
            .map_err(db_error)?;
        classic::release_webhooks(c, &message).await;
    }
    let outcome = match result.kind.as_str() {
        "posted" => match result.message_id {
            Some(message_id) => api::SlashCommandResult::Posted {
                message_id,
                notice: result.notice,
            },
            None => api::SlashCommandResult::Error {
                message: result.message.unwrap_or_default(),
            },
        },
        "open_url" => api::SlashCommandResult::OpenUrl {
            url: result.url.unwrap_or_default(),
        },
        "open_poll" => api::SlashCommandResult::OpenPoll,
        "start_huddle" => {
            let room_id = result.room_id.unwrap_or(room.id);
            let room_name =
                classic::present(c, move |presenter| presenter.room_display_name(&room, None))
                    .await?;
            api::SlashCommandResult::StartHuddle { room_id, room_name }
        }
        "ephemeral" => api::SlashCommandResult::Ephemeral {
            message: result.message.unwrap_or_default(),
        },
        _ => api::SlashCommandResult::Error {
            message: result.message.unwrap_or_default(),
        },
    };
    c.json(StatusCode::OK, &outcome)
}

// --- Preview --------------------------------------------------------------------------------------

async fn create_preview(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (_, room) = set_room(c).await?;
    let input: api::PreviewMessage = body(c).await?;
    if input.markdown_source.chars().count() > campfire_db::message::SOURCE_LIMIT {
        return Err(too_long(c));
    }
    let html = classic::render_preview(c, room.id, input.markdown_source).await?;
    c.json(
        StatusCode::OK,
        &api::MessagePreview {
            body_html: dto::inline_mentions(&html),
        },
    )
}

// --- Scheduled messages ---------------------------------------------------------------------------

/// The page size of `GET /scheduled_messages`.
const SCHEDULED_PAGE: i64 = 50;

/// A row as the wire carries it. `sendable` is [`ScheduledMessage::sendable_ids`]'s answer for
/// a pending row; `sent_message_id` names the message only while it still exists.
fn scheduled(
    row: &ScheduledMessage,
    sendable: bool,
    sent_message_exists: bool,
    now: campfire_db::Timestamp,
) -> api::ScheduledMessage {
    let state = if row.sent() {
        api::ScheduledMessageState::Sent
    } else if row.dropped() {
        api::ScheduledMessageState::Dropped
    } else if row.claimed(now) {
        api::ScheduledMessageState::Sending
    } else {
        api::ScheduledMessageState::Pending
    };
    api::ScheduledMessage {
        id: row.id,
        room_id: row.room_id,
        thread_id: row.thread_id,
        reply_to_message_id: row.reply_to_message_id,
        markdown_source: row.markdown_source.clone(),
        send_at: dto::time(row.send_at),
        state,
        sendable: row.pending() && sendable,
        sent_at: row.sent_at.map(dto::time),
        sent_message_id: row.sent_message_id.filter(|_| sent_message_exists),
        dropped_at: row.dropped_at.map(dto::time),
        drop_reason: row
            .drop_reason
            .clone()
            .filter(|reason| row.dropped() && !reason.trim().is_empty()),
        created_at: dto::time(row.created_at),
    }
}

/// Rows as the wire carries them, with their sendability and sent messages read in one go.
fn scheduled_rows(
    conn: &campfire_db::Connection,
    rows: &[ScheduledMessage],
    now: campfire_db::Timestamp,
) -> campfire_db::Result<Vec<api::ScheduledMessage>> {
    let pending = rows
        .iter()
        .filter(|row| row.pending())
        .cloned()
        .collect::<Vec<_>>();
    let sendable = ScheduledMessage::sendable_ids(conn, &pending)?;
    rows.iter()
        .map(|row| {
            let exists = match row.sent_message_id {
                Some(id) => Message::find_by_id(conn, id)?.is_some(),
                None => false,
            };
            Ok(scheduled(row, sendable.contains(&row.id), exists, now))
        })
        .collect()
}

/// One row as the wire carries it, read afresh.
async fn scheduled_reply(c: &Ctx, row: ScheduledMessage) -> Result<api::ScheduledMessage> {
    let now = now(c);
    c.app()
        .db
        .read(move |conn| {
            scheduled_rows(conn, std::slice::from_ref(&row), now).map(|mut rows| rows.remove(0))
        })
        .await
        .map_err(db_error)
}

/// `GET /scheduled_messages`' `before`: the base64url `"<send_at>|<id>"` of the previous page's
/// last row.
fn encode_cursor(row: &ScheduledMessage) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(format!(
        "{}|{}",
        row.send_at.to_db(),
        row.id
    ))
}

fn decode_cursor(raw: &str) -> Option<(campfire_db::Timestamp, i64)> {
    use base64::Engine as _;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(raw)
        .ok()?;
    let text = String::from_utf8(bytes).ok()?;
    let (at, id) = text.rsplit_once('|')?;
    Some((campfire_db::Timestamp::parse_db(at)?, id.parse().ok()?))
}

/// `sendAt` as a timestamp; anything else is a 422.
fn send_at(c: &mut Ctx, raw: &str) -> Result<campfire_db::Timestamp> {
    raw.parse::<jiff::Timestamp>()
        .map(campfire_db::Timestamp::from_jiff)
        .map_err(|_| fail(c, validation("sendAt", "is invalid")))
}

/// `scheduled_messages_controller`'s `prepare`: active humans only.
async fn prepare_scheduled(c: &mut Ctx) -> Result<i64> {
    before_actions(c).await?;
    features::active_human(c)?;
    Ok(concerns::require_current_user(c)?.id)
}

/// `scheduled_messages_controller#pending`: the viewer's own pending row `:id`, or a 404.
async fn pending(c: &mut Ctx, user_id: i64) -> Result<ScheduledMessage> {
    let id = c
        .param_str("id")
        .and_then(concerns::cast_integer)
        .ok_or(Error::NotFound)?;
    c.app()
        .db
        .read(move |conn| match ScheduledMessage::find_by_id(conn, id)? {
            Some(row) if row.user_id == user_id && row.pending() => Ok(row),
            _ => Err(campfire_db::Error::RecordNotFound("ScheduledMessage")),
        })
        .await
        .map_err(db_error)
}

async fn index_scheduled(c: &mut Ctx) -> Result {
    let user_id = prepare_scheduled(c).await?;
    let viewer = concerns::require_current_user(c)?.clone();
    let room_id = id_query(c, "roomId")?;
    let past = c.param_str("status") == Some("past");
    let after = match c.param_str("before").filter(|raw| !raw.is_empty()) {
        None => None,
        Some(raw) => match decode_cursor(raw) {
            Some(key) => Some(key),
            None => return Err(fail(c, validation("before", "is invalid"))),
        },
    };
    let now = now(c);
    let list = c
        .app()
        .db
        .read(move |conn| {
            let mut rows = ScheduledMessage::owned_page(
                conn,
                user_id,
                past,
                room_id,
                after,
                SCHEDULED_PAGE + 1,
            )?;
            let more = rows.len() as i64 > SCHEDULED_PAGE;
            rows.truncate(SCHEDULED_PAGE as usize);
            let next_cursor = rows.last().filter(|_| more).map(encode_cursor);
            let conversations = dto::conversation_names(
                conn,
                &viewer,
                rows.iter().map(|row| (row.room_id, row.thread_id)),
            )?;
            Ok(api::ScheduledMessageList {
                scheduled_messages: scheduled_rows(conn, &rows, now)?,
                conversations,
                next_cursor,
            })
        })
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &list)
}

async fn create_scheduled(c: &mut Ctx) -> Result {
    let user_id = prepare_scheduled(c).await?;
    let (_, room) = set_room(c).await?;
    let input: api::CreateScheduledMessage = body(c).await?;
    if input.markdown_source.chars().count() > campfire_db::message::SOURCE_LIMIT {
        return Err(too_long(c));
    }
    let send_at = send_at(c, &input.send_at)?;
    let (room_id, reply_id) = (room.id, input.reply_to_message_id);
    // A thread that isn't this room's is a 404, as `rooms/slash_commands#thread_id` makes it.
    let thread_id = room_thread(c, &room, input.thread_id).await?;
    // The model checks a reply target that exists; one that doesn't is refused here.
    let reply_found = match reply_id {
        Some(id) => c
            .app()
            .db
            .read(move |conn| Message::find_by_id(conn, id))
            .await
            .map_err(db_error)?
            .is_some(),
        None => true,
    };
    if !reply_found {
        return Err(fail(
            c,
            validation("replyToMessageId", "must be in the same conversation"),
        ));
    }
    let markdown_source = input.markdown_source;
    let row = c
        .app()
        .db
        .write(move |tx| {
            ScheduledMessage::create(
                tx,
                NewScheduledMessage {
                    user_id,
                    room_id,
                    thread_id,
                    reply_to_message_id: reply_id,
                    markdown_source,
                    send_at,
                },
            )
        })
        .await
        .map_err(db_error)?;
    let row = scheduled_reply(c, row).await?;
    c.json(StatusCode::CREATED, &row)
}

enum Change {
    Busy,
    Saved(Box<ScheduledMessage>),
}

async fn update_scheduled_message(c: &mut Ctx) -> Result {
    let user_id = prepare_scheduled(c).await?;
    let initial = pending(c, user_id).await?;
    let input: api::UpdateScheduledMessage = body(c).await?;
    // A field left out keeps its value.
    if input
        .markdown_source
        .as_ref()
        .is_some_and(|source| source.chars().count() > campfire_db::message::SOURCE_LIMIT)
    {
        return Err(too_long(c));
    }
    let send_at = match input.send_at.as_deref() {
        Some(raw) => Some(send_at(c, raw)?),
        None => None,
    };
    let source = input.markdown_source;
    let change = c
        .app()
        .db
        .write(move |tx| {
            let mut row = ScheduledMessage::find(tx.conn(), initial.id)?;
            if !row.pending() || row.claimed(tx.now()) {
                return Ok(Change::Busy);
            }
            let source = source.unwrap_or_else(|| row.markdown_source.clone());
            let send_at = send_at.unwrap_or(row.send_at);
            row.update(tx, &source, send_at)?;
            Ok(Change::Saved(Box::new(row)))
        })
        .await
        .map_err(db_error)?;
    match change {
        Change::Busy => Err(busy(c)),
        Change::Saved(row) => {
            let row = scheduled_reply(c, *row).await?;
            c.json(StatusCode::OK, &row)
        }
    }
}

fn busy(c: &mut Ctx) -> Error {
    fail(
        c,
        api::ApiError::Conflict {
            message: BUSY.into(),
        },
    )
}

async fn destroy_scheduled(c: &mut Ctx) -> Result {
    let user_id = prepare_scheduled(c).await?;
    let initial = pending(c, user_id).await?;
    let cancelled = c
        .app()
        .db
        .write(move |tx| {
            let row = ScheduledMessage::find(tx.conn(), initial.id)?;
            if !row.pending() || row.claimed(tx.now()) {
                return Ok(false);
            }
            row.destroy(tx)?;
            Ok(true)
        })
        .await
        .map_err(db_error)?;
    if !cancelled {
        return Err(busy(c));
    }
    Ok(c.head(StatusCode::NO_CONTENT))
}

/// `scheduled_messages#send_now`: the scheduler's dispatch, at once, in the same writer.
async fn send_now(c: &mut Ctx) -> Result {
    let user_id = prepare_scheduled(c).await?;
    let initial = pending(c, user_id).await?;
    let origin = page::renderer_base_url(c);
    let zone = features::user_zone(c).await?;
    let (sent, row) = c
        .app()
        .db
        .write_scoped(
            move || (renderer_origin(&origin), page::enter_time_zone(zone)),
            move |tx| {
                let sent = ScheduledMessage::dispatch(tx, initial.id, tx.now(), true)?;
                Ok((sent, ScheduledMessage::find(tx.conn(), initial.id)?))
            },
        )
        .await
        .map_err(db_error)?;
    if !sent && row.dropped() {
        let message = row
            .drop_reason
            .as_deref()
            .filter(|reason| !reason.is_empty())
            .map(|reason| format!("The scheduled message was not sent ({reason})."))
            .unwrap_or_else(|| {
                "You no longer have access to that room, so the message was not sent.".into()
            });
        return Err(fail(
            c,
            api::ApiError::Validation {
                message,
                fields: Default::default(),
            },
        ));
    }
    let status = if sent {
        StatusCode::OK
    } else {
        StatusCode::ACCEPTED
    };
    let row = scheduled_reply(c, row).await?;
    c.json(status, &row)
}
