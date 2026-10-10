//! The `/api/v1` REST endpoints (`campfire_api_types` documents each one). They run the classic
//! pages' before-actions with `Authentication::JsonUnauthorized` (so CSRF is checked on writes,
//! from `X-CSRF-Token`), scope rooms with `set_room`, and post through `messages#create`'s path.

use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_db::models::agent_posting::PostingOutcome;
use campfire_db::{Account, Message, Timeline};
use campfire_kit::{Ctx, Error, Result, StatusCode};
use campfire_messages::controllers::messages::{self as posting, MessageParams};
use campfire_runtime::concerns::{self, Authentication, Before};
use campfire_runtime::presenters::attachments::Assignment;
use campfire_runtime::context::db_error;
use campfire_runtime::presenters::room_shell;
use serde::de::DeserializeOwned;

use crate::dto;
use crate::error::{fail, not_found, validation};

/// The most ids `GET /users` and `GET /presence` look up.
const MAX_IDS: usize = 100;
const MAX_MESSAGE_FILES: usize = campfire_db::message::ATTACHMENTS_PER_MESSAGE;
/// The largest request body read: a message at `SOURCE_LIMIT` characters, four bytes each,
/// escaped, with room to spare.
pub(crate) const BODY_LIMIT: usize = 1 << 20;

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

pub(crate) async fn before_actions(c: &mut Ctx) -> Result<()> {
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
pub(crate) async fn set_room(c: &mut Ctx) -> Result<(campfire_db::Membership, campfire_db::Room)> {
    let (membership, room) = concerns::set_room(c).await?;
    if room.deleted_at.is_some() {
        return Err(Error::NotFound);
    }
    Ok((membership, room))
}

/// The JSON body as `T`; anything else is a 422.
pub(crate) async fn body<T: DeserializeOwned>(c: &mut Ctx) -> Result<T> {
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

pub(crate) fn now(c: &Ctx) -> campfire_db::Timestamp {
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
        .read_snapshot(move |conn| {
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
    message_page(c, Timeline::Room(room.id)).await
}

/// A [`api::MessagePage`] of `timeline`, by the `before`, `after` or `around` cursor.
pub(crate) async fn message_page(c: &mut Ctx, timeline: Timeline) -> Result {
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
    let (app, now) = (c.app().clone(), now(c));
    let viewer_id = concerns::require_current_user(c)?.id;
    let (page, fetches) = c
        .app()
        .db
        .read(move |conn| {
            let anchor = |id| Message::find_in(conn, timeline, id);
            // A `before`/`after` cursor whose message was deleted since pages from its id, so
            // paging doesn't stall on it. Messages are deleted outright, so a cursor that isn't
            // on this timeline but still exists is another room's or thread's, or the room's
            // root timeline's: that's a 404, as is an `around` anchor that's gone.
            let gone = |error: &campfire_db::Error, id: i64| -> campfire_db::Result<bool> {
                if !matches!(error, campfire_db::Error::RecordNotFound(_)) {
                    return Ok(false);
                }
                let elsewhere: bool = conn.query_row(
                    r#"SELECT EXISTS (SELECT 1 FROM "messages" WHERE "messages"."id" = ?)"#,
                    [id],
                    |row| row.get(0),
                )?;
                Ok(!elsewhere)
            };
            let messages = match cursor {
                Cursor::Newest => Message::last_page(conn, timeline)?,
                Cursor::Before(id) => match anchor(id) {
                    Ok(message) => Message::page_before(conn, timeline, &message)?,
                    Err(error) if gone(&error, id)? => Message::page_before_id(conn, timeline, id)?,
                    Err(error) => return Err(error),
                },
                Cursor::After(id) => match anchor(id) {
                    Ok(message) => Message::page_after(conn, timeline, &message)?,
                    Err(error) if gone(&error, id)? => Message::page_after_id(conn, timeline, id)?,
                    Err(error) => return Err(error),
                },
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
            let (dtos, fetches) = dto::messages_and_fetches(conn, &app, &messages)?;
            // The authors, and the repliers the thread indicators name, so their avatars need
            // no `GET /users`.
            let people = dtos.iter().flat_map(|message| {
                let repliers = message
                    .thread
                    .iter()
                    .flat_map(|thread| thread.replier_ids.iter());
                std::iter::once(message.creator_id).chain(repliers.copied())
            });
            let page = api::MessagePage {
                users: dto::users(conn, &app.secrets, people.collect::<Vec<_>>(), now)?,
                messages: dtos,
                saved: dto::saved(conn, viewer_id, &messages)?,
                before,
                after,
            };
            Ok((page, fetches))
        })
        .await
        .map_err(db_error)?;
    // The card fetches the page asked for, as the classic timeline requests them.
    fetches.request(c.app()).await;
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
    let signed_id = input
        .attachment_signed_id
        .filter(|signed_id| !signed_id.is_empty());
    let signed_ids = grouped_signed_ids(
        c,
        signed_id.as_deref(),
        input.attachment_signed_ids.unwrap_or_default(),
    )?;
    let drive_file_ids =
        crate::drive::require_drive_file_ids(c, input.drive_file_ids.as_deref().unwrap_or(&[]))?;
    if signed_id.is_none()
        && signed_ids.is_empty()
        && drive_file_ids.is_empty()
        && input.markdown_source.trim().is_empty()
    {
        return Err(fail(c, validation("markdownSource", "can't be blank")));
    }
    let (room_id, creator_id) = (room.id, concerns::require_current_user(c)?.id);
    // `Message.find_duplicate`: a retry gets the message the first attempt created. This read
    // answers a later retry without validating it again; the create checks again in its write
    // transaction, for retries racing each other.
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
            #[cfg(feature = "test-support")]
            crate::test_hooks::after_duplicate_check(&client_message_id).await;
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
            if let Some(signed_id) = &signed_id
                && !blob_exists(c, signed_id).await?
            {
                return Err(fail(
                    c,
                    validation("attachmentSignedId", "isn't a finished upload"),
                ));
            }
            require_grouped_uploads(c, &signed_ids).await?;
            let markdown_source =
                Some(input.markdown_source).filter(|source| !source.trim().is_empty());
            let attributes = MessageParams {
                markdown_source,
                // `message[attachment]` given a direct upload's signed blob id.
                attachment: signed_id.map(Assignment::Signed),
                attachments: signed_ids.into_iter().map(Assignment::Signed).collect(),
                attachment_policy: posting::AttachmentPolicy::OwnedUpload { uploader_id: creator_id },
                client_message_id: Some(client_message_id),
                reply_to_message_id: input.reply_to_message_id,
                reply_notify_author: input.reply_notify_author,
                drive_file_ids,
                ..MessageParams::default()
            };
            match posting::create_or_find_message(c, &room, attributes).await? {
                PostingOutcome::Created(message) => {
                    posting::broadcast_create(c, &room, &message).await?;
                    posting::release_webhooks(c, &message).await;
                    (message, StatusCode::CREATED)
                }
                PostingOutcome::Replay(message) => (message, StatusCode::OK),
                PostingOutcome::Budget(_) => {
                    unreachable!("human posting does not run agent policy")
                }
            }
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

pub(crate) fn grouped_signed_ids(
    c: &mut Ctx,
    single: Option<&str>,
    ids: Vec<String>,
) -> Result<Vec<String>> {
    if ids.len() > MAX_MESSAGE_FILES {
        return Err(fail(
            c,
            validation("attachmentSignedIds", "has too many files (maximum is 10)"),
        ));
    }
    if single.is_some() && !ids.is_empty() {
        return Err(fail(
            c,
            validation(
                "attachmentSignedIds",
                "cannot be combined with attachmentSignedId",
            ),
        ));
    }
    Ok(ids)
}

pub(crate) async fn require_grouped_uploads(c: &mut Ctx, signed_ids: &[String]) -> Result<()> {
    if signed_ids.is_empty() {
        return Ok(());
    }
    let app = c.app();
    let mut ids = Vec::with_capacity(signed_ids.len());
    for signed_id in signed_ids {
        let Some(id) = campfire_storage::paths::verify_signed_blob_id(
            &*app.storage.verifier,
            signed_id,
            app.clock.now(),
        ) else {
            return Err(fail(
                c,
                validation("attachmentSignedIds", "includes an invalid upload"),
            ));
        };
        if ids.contains(&id) {
            return Err(fail(
                c,
                validation("attachmentSignedIds", "includes a duplicate file"),
            ));
        }
        ids.push(id);
    }
    let storage = app.storage.clone();
    let finished = app
        .db
        .read(move |conn| {
            let blobs = campfire_storage::Blob::find_many(conn, &ids)
                .map_err(campfire_web::controllers::presenters::storage_error)?;
            Ok(ids.iter().all(|id| {
                blobs
                    .get(id)
                    .is_some_and(|blob| storage.service.exist(&blob.key))
            }))
        })
        .await
        .map_err(db_error)?;
    if !finished {
        return Err(fail(
            c,
            validation(
                "attachmentSignedIds",
                "includes an upload that isn't finished",
            ),
        ));
    }
    Ok(())
}

/// The blob a direct upload's signed id names exists (`ActiveStorage::Blob.find_signed`).
pub(crate) async fn blob_exists(c: &Ctx, signed_id: &str) -> Result<bool> {
    let app = c.app();
    let now = app.clock.now();
    let Some(id) =
        campfire_storage::paths::verify_signed_blob_id(&*app.storage.verifier, signed_id, now)
    else {
        return Ok(false);
    };
    app.db
        .read(move |conn| {
            Ok(conn.query_row(
                r#"SELECT EXISTS(SELECT 1 FROM "active_storage_blobs" WHERE "active_storage_blobs"."id" = ?)"#,
                [id],
                |row| row.get(0),
            )?)
        })
        .await
        .map_err(db_error)
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
    c.app().broadcasts.sync_read_row(user_id, room.id);
    c.json(
        StatusCode::OK,
        &api::ReadState {
            room_id: room.id,
            unread: false,
            first_unread_message_id: None,
            unread_count: 0,
        },
    )
}

async fn destroy_read(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (mut membership, room) = set_room(c).await?;
    let api::MarkUnread { message_id } = body(c).await?;
    let room_id = room.id;
    let unread_count = c
        .app()
        .db
        .write(move |tx| {
            let message = Message::find_in(tx.conn(), Timeline::Room(room_id), message_id)?;
            membership.mark_unread_before(tx, &message)?;
            // The count the sidebar row shows from here on.
            Ok(room_shell::first_unread(tx.conn(), &membership)?.map_or(0, |(_, count)| count))
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
            unread_count,
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
