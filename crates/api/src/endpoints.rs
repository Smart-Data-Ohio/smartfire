//! The `/api/v1` REST endpoints (`campfire_api_types` documents each one). They run the classic
//! pages' before-actions with `Authentication::JsonUnauthorized` (so CSRF is checked on writes,
//! from `X-CSRF-Token`), scope rooms with `set_room`, and post through `messages#create`'s path.

use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_db::{Account, Message, Timeline};
use campfire_kit::{Ctx, Error, Result, StatusCode};
use campfire_web::concerns::{self, Authentication, Before};
use campfire_web::controllers::presenters::page::db_error;
use campfire_messages::controllers::messages::{self as posting, MessageParams};
use serde::de::DeserializeOwned;

use crate::dto;
use crate::error::{fail, not_found, prepare, respond, validation};

/// The most ids `GET /users` and `GET /presence` look up.
const MAX_IDS: usize = 100;
/// The largest request body read: a message at `SOURCE_LIMIT` characters, four bytes each,
/// escaped, with room to spare.
const BODY_LIMIT: usize = 1 << 20;

macro_rules! endpoint {
    ($(#[$doc:meta])* $name:ident => $body:ident) => {
        $(#[$doc])*
        pub async fn $name(c: &mut Ctx) -> Result {
            prepare(c);
            let result = $body(c).await;
            respond(c, result)
        }
    };
}

endpoint!(
    /// `GET /api/v1/me`
    me => show_me
);
endpoint!(
    /// `GET /api/v1/sidebar`
    sidebar => show_sidebar
);
endpoint!(
    /// `GET /api/v1/rooms/:room_id`
    room => show_room
);
endpoint!(
    /// `GET /api/v1/rooms/:room_id/messages`
    messages => index_messages
);
endpoint!(
    /// `POST /api/v1/rooms/:room_id/messages`
    create_message => post_message
);
endpoint!(
    /// `POST /api/v1/rooms/:room_id/read`
    mark_read => create_read
);
endpoint!(
    /// `DELETE /api/v1/rooms/:room_id/read`
    mark_unread => destroy_read
);
endpoint!(
    /// `GET /api/v1/users`
    users => index_users
);
endpoint!(
    /// `GET /api/v1/presence`
    presence => index_presence
);

async fn before_actions(c: &mut Ctx) -> Result<()> {
    concerns::before_actions(
        c,
        Before {
            authentication: Authentication::JsonUnauthorized,
            ..Before::default()
        },
    )
    .await
}

/// `set_room`, of an alive room (`RoomScoped` with `Room.alive`).
async fn set_room(c: &mut Ctx) -> Result<(campfire_db::Membership, campfire_db::Room)> {
    let (membership, room) = concerns::set_room(c).await?;
    if room.deleted_at.is_some() {
        return Err(Error::NotFound);
    }
    Ok((membership, room))
}

/// The JSON body as `T`; anything else is a 422.
async fn body<T: DeserializeOwned>(c: &mut Ctx) -> Result<T> {
    let bytes = c.read_body(BODY_LIMIT).await;
    serde_json::from_slice(&bytes).map_err(|error| {
        fail(
            c,
            api::ApiError::Validation {
                message: format!("The request body isn't valid: {error}"),
                fields: Default::default(),
            },
        )
    })
}

fn now(c: &Ctx) -> campfire_db::Timestamp {
    c.app().db.env().now()
}

async fn show_me(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let viewer = concerns::require_current_user(c)?.clone();
    let last_room_id = concerns::last_room_visited(c).await?.map(|room| room.id);
    let (secrets, now) = (c.app().secrets.clone(), now(c));
    let me = c
        .app()
        .db
        .read(move |conn| dto::me(conn, &secrets, &viewer, last_room_id, now))
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &me)
}

async fn show_sidebar(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let viewer = concerns::require_current_user(c)?.clone();
    let (secrets, now) = (c.app().secrets.clone(), now(c));
    let sidebar = c
        .app()
        .db
        .read(move |conn| {
            // `Current.user.administrator? || !Current.account.settings.restrict_room_creation_to_administrators?`
            let restricted = Account::first(conn)?.is_some_and(|account| {
                account
                    .settings()
                    .restrict_room_creation_to_administrators()
            });
            dto::sidebar(
                conn,
                &secrets,
                &viewer,
                viewer.is_administrator() || !restricted,
                now,
            )
        })
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &sidebar)
}

async fn show_room(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (membership, room) = set_room(c).await?;
    let viewer = concerns::require_current_user(c)?.clone();
    let (secrets, now) = (c.app().secrets.clone(), now(c));
    let detail = c
        .app()
        .db
        .read(move |conn| dto::room_detail(conn, &secrets, &viewer, &room, &membership, now))
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &detail)
}

enum Cursor {
    Newest,
    Before(i64),
    After(i64),
    Around(i64),
}

async fn index_messages(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (_, room) = set_room(c).await?;
    let mut given = Vec::new();
    for key in ["before", "after", "around"] {
        if let Some(value) = c.param_str(key) {
            given.push((key, value.to_string()));
        }
    }
    let cursor = match given.as_slice() {
        [] => Cursor::Newest,
        [(key, value)] => {
            let Some(id) = concerns::cast_integer(value) else {
                return Err(fail(c, not_found()));
            };
            match *key {
                "before" => Cursor::Before(id),
                "after" => Cursor::After(id),
                _ => Cursor::Around(id),
            }
        }
        _ => {
            return Err(fail(
                c,
                validation("before", "can't be given with after or around"),
            ));
        }
    };
    let (app, now, room_id) = (c.app().clone(), now(c), room.id);
    let page = c
        .app()
        .db
        .read(move |conn| {
            let timeline = Timeline::Room(room_id);
            let anchor = |id| Message::find_in(conn, timeline, id);
            let messages = match cursor {
                Cursor::Newest => Message::last_page(conn, timeline)?,
                Cursor::Before(id) => Message::page_before(conn, timeline, &anchor(id)?)?,
                Cursor::After(id) => Message::page_after(conn, timeline, &anchor(id)?)?,
                Cursor::Around(id) => Message::page_around(conn, timeline, &anchor(id)?)?,
            };
            let before = match messages.first() {
                Some(oldest) if Message::exists_before(conn, timeline, oldest)? => Some(oldest.id),
                _ => None,
            };
            let after = match messages.last() {
                Some(newest) if Message::exists_after(conn, timeline, newest)? => Some(newest.id),
                _ => None,
            };
            Ok(api::MessagePage {
                users: dto::users(
                    conn,
                    &app.secrets,
                    messages.iter().map(|message| message.creator_id),
                    now,
                )?,
                messages: dto::messages(conn, &app, &messages)?,
                before,
                after,
            })
        })
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &page)
}

async fn post_message(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (_, room) = set_room(c).await?;
    let input: api::CreateMessage = body(c).await?;
    let client_message_id = input.client_message_id.trim().to_string();
    if client_message_id.is_empty() {
        return Err(fail(c, validation("clientMessageId", "can't be blank")));
    }
    if input.markdown_source.chars().count() > campfire_db::message::SOURCE_LIMIT {
        return Err(fail(
            c,
            validation(
                "markdownSource",
                "is too long (maximum is 50000 characters)",
            ),
        ));
    }
    let (room_id, creator_id) = (room.id, concerns::require_current_user(c)?.id);
    // `Message.find_duplicate`: a retry gets the message the first attempt created.
    let lookup = client_message_id.clone();
    let duplicate = c
        .app()
        .db
        .read(move |conn| Message::find_duplicate(conn, room_id, creator_id, &lookup))
        .await
        .map_err(db_error)?;
    let (message, status) = match duplicate {
        Some(message) => (message, StatusCode::OK),
        None => {
            if let Some(reply_to) = input.reply_to_message_id {
                let found = c
                    .app()
                    .db
                    .read(move |conn| {
                        match Message::find_in(conn, Timeline::Room(room_id), reply_to) {
                            Ok(_) => Ok(true),
                            Err(campfire_db::Error::RecordNotFound(_)) => Ok(false),
                            Err(error) => Err(error),
                        }
                    })
                    .await
                    .map_err(db_error)?;
                if !found {
                    return Err(fail(
                        c,
                        validation("replyToMessageId", "isn't a message on this timeline"),
                    ));
                }
            }
            let attributes = MessageParams {
                markdown_source: Some(input.markdown_source),
                client_message_id: Some(client_message_id),
                reply_to_message_id: input.reply_to_message_id,
                reply_notify_author: input.reply_notify_author,
                ..MessageParams::default()
            };
            let message = posting::create_message_into(c, &room, None, attributes).await?;
            posting::broadcast_create(c, &room, &message).await?;
            posting::release_webhooks(c, &message).await;
            (message, StatusCode::CREATED)
        }
    };
    let app = c.app().clone();
    let dto = c
        .app()
        .db
        .read(move |conn| dto::message(conn, &app, &message))
        .await
        .map_err(db_error)?;
    c.json(status, &dto)
}

async fn create_read(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (mut membership, room) = set_room(c).await?;
    c.app()
        .db
        .write(move |tx| membership.read(tx))
        .await
        .map_err(db_error)?;
    let user_id = concerns::require_current_user(c)?.id;
    campfire_app::cable::broadcasts::read_room(&c.app().cable, user_id, room.id);
    c.json(
        StatusCode::OK,
        &api::ReadState {
            room_id: room.id,
            unread: false,
            first_unread_message_id: None,
        },
    )
}

async fn destroy_read(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (mut membership, room) = set_room(c).await?;
    let api::MarkUnread { message_id } = body(c).await?;
    let room_id = room.id;
    c.app()
        .db
        .write(move |tx| {
            let message = Message::find_in(tx.conn(), Timeline::Room(room_id), message_id)?;
            membership.mark_unread_before(tx, &message)
        })
        .await
        .map_err(db_error)?;
    let user_id = concerns::require_current_user(c)?.id;
    c.app().broadcasts.mark_room_unread(user_id, room_id);
    c.json(
        StatusCode::OK,
        &api::ReadState {
            room_id,
            unread: true,
            first_unread_message_id: Some(message_id),
        },
    )
}

/// `ids=1,2,3`: the first [`MAX_IDS`] integers named; anything else is skipped.
fn ids(c: &Ctx) -> Vec<i64> {
    c.param_str("ids")
        .unwrap_or_default()
        .split(',')
        .filter_map(|id| id.trim().parse::<i64>().ok())
        .take(MAX_IDS)
        .collect()
}

async fn index_users(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (ids, secrets, now) = (ids(c), c.app().secrets.clone(), now(c));
    let users = c
        .app()
        .db
        .read(move |conn| dto::users(conn, &secrets, ids, now))
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &api::UserList { users })
}

async fn index_presence(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (ids, now) = (ids(c), now(c));
    let presences = c
        .app()
        .db
        .read(move |conn| dto::presences(conn, &ids, now))
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &api::PresenceList { presences })
}
