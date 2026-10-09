//! The S2 thread endpoints (`campfire_api_types::thread` documents each one). Each runs the
//! classic `channel_threads` / `channel_thread_messages` action's write and frames, then
//! publishes the thread's sync event.

use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_db::models::agent_posting::PostingOutcome;
use campfire_db::models::channel_thread::ThreadStatus;
use campfire_db::{
    ChannelThread, Membership, Message, Room, ThreadInvolvement, ThreadMembership, Timeline, User,
};
use campfire_kit::{Ctx, Error, Result, StatusCode};
use campfire_messages::controllers::messages::{self as posting, MessageParams, ThreadOutcome};
use campfire_web::concerns;
use campfire_web::controllers::presenters::attachments::Assignment;
use campfire_web::controllers::presenters::page::db_error;

use crate::dto;
use crate::endpoints::{before_actions, blob_exists, body, message_page, now, set_room};
use crate::error::{fail, not_found, validation};

endpoint!(
    /// `GET /api/v1/rooms/:room_id/threads`
    threads => index_threads
);
endpoint!(
    /// `POST /api/v1/rooms/:room_id/threads`
    create => create_thread
);
endpoint!(
    /// `GET /api/v1/threads/:thread_id`
    thread => show_thread
);
endpoint!(
    /// `PATCH /api/v1/threads/:thread_id`
    update => update_thread
);
endpoint!(
    /// `DELETE /api/v1/threads/:thread_id`
    destroy => destroy_thread
);
endpoint!(
    /// `GET /api/v1/threads/:thread_id/messages`
    messages => index_replies
);
endpoint!(
    /// `POST /api/v1/threads/:thread_id/messages`
    reply => create_reply
);
endpoint!(
    /// `POST /api/v1/threads/:thread_id/join`
    join => create_join
);
endpoint!(
    /// `DELETE /api/v1/threads/:thread_id/join`
    leave => destroy_join
);
endpoint!(
    /// `POST /api/v1/threads/:thread_id/read`
    read => create_read
);

/// A refused thread change (`channel_threads#update`'s `FORBIDDEN_UPDATE`).
pub(crate) const FORBIDDEN_UPDATE: &str = "You can't make that change to this thread";

/// The thread `:thread_id` names, with its room: a thread in an alive room the viewer belongs
/// to. Anything else is a 404, as the classic nested routes' `set_room` makes it.
pub(crate) async fn scope(c: &mut Ctx) -> Result<(ChannelThread, Room, User)> {
    let viewer = concerns::require_current_user(c)?.clone();
    let Some(id) = c.param_str("thread_id").and_then(concerns::cast_integer) else {
        return Err(fail(c, not_found()));
    };
    let user_id = viewer.id;
    let found = c
        .app()
        .db
        .read(move |conn| {
            let Some(thread) = ChannelThread::find_by_id(conn, id)? else {
                return Ok(None);
            };
            let room = Room::find(conn, thread.room_id)?;
            if room.deleted_at.is_some()
                || Membership::find_by_room_and_user(conn, room.id, user_id)?.is_none()
            {
                return Ok(None);
            }
            Ok(Some((thread, room)))
        })
        .await
        .map_err(db_error)?;
    match found {
        Some((thread, room)) => Ok((thread, room, viewer)),
        None => Err(fail(c, not_found())),
    }
}

fn forbidden(c: &mut Ctx, message: &str) -> Error {
    fail(
        c,
        api::ApiError::Forbidden {
            message: message.into(),
        },
    )
}

/// The database error `error` carries, if it's one.
fn db_cause(error: &Error) -> Option<&campfire_db::Error> {
    match error {
        Error::Internal(error) => error.downcast_ref::<campfire_db::Error>(),
        _ => None,
    }
}

fn is_locked(error: &Error) -> bool {
    matches!(
        db_cause(error),
        Some(campfire_db::Error::Other(text)) if text == campfire_db::channel_thread::LOCKED_MESSAGE
    )
}

pub(crate) async fn detail(c: &Ctx, viewer: User, thread_id: i64) -> Result<api::ThreadDetail> {
    let (app, now) = (c.app().clone(), now(c));
    let (detail, fetches) = c
        .app()
        .db
        .read(move |conn| {
            let thread = ChannelThread::find(conn, thread_id)?;
            let room = Room::find(conn, thread.room_id)?;
            dto::thread_detail(conn, &app, &viewer, &thread, &room, now)
        })
        .await
        .map_err(db_error)?;
    fetches.request(c.app()).await;
    Ok(detail)
}

async fn index_threads(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (_, room) = set_room(c).await?;
    let filter = match c.param_str("state").unwrap_or("active") {
        "active" | "" => api::ThreadFilter::Active,
        "closed" => api::ThreadFilter::Closed,
        "locked" => api::ThreadFilter::Locked,
        "all" => api::ThreadFilter::All,
        _ => {
            return Err(fail(
                c,
                validation("state", "must be one of active, closed, locked or all"),
            ));
        }
    };
    let viewer_id = concerns::require_current_user(c)?.id;
    let (secrets, now) = (c.app().secrets.clone(), now(c));
    let list = c
        .app()
        .db
        .read(move |conn| dto::thread_list(conn, &secrets, viewer_id, &room, filter, now))
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &list)
}

async fn show_thread(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (thread, _, viewer) = scope(c).await?;
    let detail = detail(c, viewer, thread.id).await?;
    c.json(StatusCode::OK, &detail)
}

async fn index_replies(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (thread, _, _) = scope(c).await?;
    message_page(c, Timeline::Thread(thread.id)).await
}

/// The reply fields `CreateMessage` shares between a first reply and a later one, checked as
/// `POST /rooms/:id/messages` checks them. `timeline` is where `replyToMessageId` must be.
async fn message_params(
    c: &mut Ctx,
    input: api::CreateMessage,
    timeline: Timeline,
) -> Result<MessageParams> {
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
    if signed_id.is_none() && input.markdown_source.trim().is_empty() {
        return Err(fail(c, validation("markdownSource", "can't be blank")));
    }
    if let Some(reply_to) = input.reply_to_message_id {
        let found = c
            .app()
            .db
            .read(
                move |conn| match Message::find_in(conn, timeline, reply_to) {
                    Ok(_) => Ok(true),
                    Err(campfire_db::Error::RecordNotFound(_)) => Ok(false),
                    Err(error) => Err(error),
                },
            )
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
    Ok(MessageParams {
        markdown_source: Some(input.markdown_source).filter(|source| !source.trim().is_empty()),
        attachment: signed_id.map(Assignment::Signed),
        client_message_id: Some(client_message_id),
        reply_to_message_id: input.reply_to_message_id,
        reply_notify_author: input.reply_notify_author,
        ..MessageParams::default()
    })
}

async fn create_thread(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (_, room) = set_room(c).await?;
    if room.direct() {
        return Err(forbidden(c, "Direct rooms cannot contain channel threads"));
    }
    // Boards use the post endpoint, which accepts work fields and an optional brief.
    if room.board() {
        return Err(forbidden(c, "Board rooms take posts, not threads"));
    }
    let input: api::CreateThread = body(c).await?;
    if let Some(name) = &input.name
        && name.chars().count() > 100
    {
        return Err(fail(
            c,
            validation("name", "is too long (maximum is 100 characters)"),
        ));
    }
    // `paging_anchor(Timeline::Room)`: the parent is on the room's root timeline.
    let (room_id, parent_id) = (room.id, input.parent_message_id);
    let parent = c
        .app()
        .db
        .read(
            move |conn| match Message::find_in(conn, Timeline::Room(room_id), parent_id) {
                Ok(parent) => Ok(Some(parent)),
                Err(campfire_db::Error::RecordNotFound(_)) => Ok(None),
                Err(error) => Err(error),
            },
        )
        .await
        .map_err(db_error)?;
    if parent.is_none() {
        return Err(fail(c, not_found()));
    }
    let attributes = message_params(c, input.message, Timeline::Room(room_id)).await?;
    let name = input.name.filter(|name| !name.trim().is_empty());
    let outcome = match posting::create_or_find_thread(c, &room, parent_id, name, attributes).await
    {
        Ok(outcome) => outcome,
        Err(error) if db_cause(&error).is_some_and(campfire_db::Error::is_record_not_unique) => {
            return Err(fail(
                c,
                api::ApiError::Conflict {
                    message: "A thread already exists for that message".into(),
                },
            ));
        }
        Err(error) => return Err(error),
    };
    let (thread, message, status) = match outcome {
        ThreadOutcome::Created(thread, message) => {
            c.app().broadcasts.thread_created(thread.id);
            (thread, message, StatusCode::CREATED)
        }
        ThreadOutcome::Replay(thread, message) => (thread, message, StatusCode::OK),
        ThreadOutcome::ClientMessageIdTaken => {
            return Err(fail(
                c,
                validation(
                    "clientMessageId",
                    "is already used by a message outside a thread",
                ),
            ));
        }
    };
    let viewer = concerns::require_current_user(c)?.clone();
    let detail = detail(c, viewer, thread.id).await?;
    let app = c.app().clone();
    let message = c
        .app()
        .db
        .read(move |conn| dto::message(conn, &app, &message))
        .await
        .map_err(db_error)?;
    c.json(status, &api::ThreadCreated { detail, message })
}

async fn update_thread(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (thread, room, viewer) = scope(c).await?;
    let input: api::UpdateThread = body(c).await?;
    if let Some(name) = &input.name {
        if name.trim().is_empty() {
            return Err(fail(c, validation("name", "can't be blank")));
        }
        if name.chars().count() > 100 {
            return Err(fail(
                c,
                validation("name", "is too long (maximum is 100 characters)"),
            ));
        }
    }
    let (thread_id, board) = (thread.id, room.board());
    // An empty body changes nothing, so nothing is published.
    let changes = input.name.is_some() || input.status.is_some();
    let actor = viewer.clone();
    let result = c
        .app()
        .db
        .write(move |tx| {
            let mut thread = ChannelThread::find(tx.conn(), thread_id)?;
            // `channel_threads#update`'s permissions, read in its write.
            let settings = thread.settings_manageable_by(tx.conn(), &actor)?;
            let moderator = thread.manageable_by(tx.conn(), &actor)?;
            // A board post's name is work metadata: its owner may change it too.
            let rename = if board {
                thread.work_manageable_by(tx.conn(), &actor)?
            } else {
                settings
            };
            let allowed = (input.name.is_none() || rename)
                && match input.status {
                    None => true,
                    Some(api::ThreadStatus::Closed) => {
                        if board {
                            moderator
                        } else {
                            settings
                        }
                    }
                    Some(api::ThreadStatus::Locked) => moderator,
                    Some(api::ThreadStatus::Active) if thread.locked_at.is_some() => moderator,
                    Some(api::ThreadStatus::Active)
                        if thread.status(tx.conn(), tx.now())? == ThreadStatus::Closed =>
                    {
                        thread.membership_for(tx.conn(), actor.id)?.is_some()
                    }
                    Some(api::ThreadStatus::Active) => true,
                };
            if !allowed {
                return Err(campfire_db::Error::Other(FORBIDDEN_UPDATE.into()));
            }
            if input.name.is_some() {
                thread.update_metadata(tx, input.name.as_deref(), None, None)?;
            }
            match input.status {
                Some(api::ThreadStatus::Closed) => thread.close(tx)?,
                Some(api::ThreadStatus::Locked) => thread.lock_conversation(tx)?,
                Some(api::ThreadStatus::Active) if thread.locked_at.is_some() => {
                    thread.unlock_conversation(tx)?
                }
                Some(api::ThreadStatus::Active) => thread.reopen(tx)?,
                None => {}
            }
            Ok(campfire_db::models::channel_thread::ThreadWorkChange::pending(tx, thread_id))
        })
        .await;
    let model_published = match result {
        Ok(model_published) => model_published,
        Err(campfire_db::Error::Other(text)) if text == FORBIDDEN_UPDATE => {
            return Err(forbidden(c, FORBIDDEN_UPDATE));
        }
        Err(error) => return Err(db_error(error)),
    };
    if changes && !model_published {
        c.app().broadcasts.thread_updated(thread_id);
    }
    let detail = detail(c, viewer, thread_id).await?;
    c.json(StatusCode::OK, &detail)
}

async fn destroy_thread(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (thread, room, viewer) = scope(c).await?;
    let thread_id = thread.id;
    let removed = c
        .app()
        .db
        .write(move |tx| {
            let thread = ChannelThread::find(tx.conn(), thread_id)?;
            if !thread.manageable_by(tx.conn(), &viewer)? {
                return Ok(false);
            }
            thread.destroy_by(tx, Some(viewer.id))?;
            Ok(true)
        })
        .await
        .map_err(db_error)?;
    if !removed {
        return Err(forbidden(c, "Only moderators can delete a thread"));
    }
    c.app().broadcasts.thread_removed(thread_id, room.id);
    Ok(c.head(StatusCode::NO_CONTENT))
}

async fn create_reply(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (thread, room, _) = scope(c).await?;
    let input: api::CreateMessage = body(c).await?;
    let attributes = message_params(c, input, Timeline::Thread(thread.id)).await?;
    // `channel_thread_messages#create`: a retry gets the reply its first attempt posted.
    let (message, status) = match posting::create_or_find_reply(c, &room, thread, attributes).await
    {
        Ok(PostingOutcome::Created(message)) => {
            posting::broadcast_create(c, &room, &message).await?;
            (message, StatusCode::CREATED)
        }
        Ok(PostingOutcome::Replay(message)) => (message, StatusCode::OK),
        Ok(PostingOutcome::Budget(_)) => unreachable!("human posting does not run agent policy"),
        Err(error) if is_locked(&error) => {
            return Err(forbidden(c, campfire_db::channel_thread::LOCKED_MESSAGE));
        }
        Err(error) => return Err(error),
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

async fn create_join(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (thread, _, viewer) = scope(c).await?;
    // The body is optional: none joins with the default involvement.
    let bytes = c.read_body(crate::endpoints::BODY_LIMIT).await;
    let api::JoinThread { involvement } = if bytes.iter().all(u8::is_ascii_whitespace) {
        api::JoinThread { involvement: None }
    } else {
        serde_json::from_slice(&bytes).map_err(|error| {
            fail(
                c,
                api::ApiError::Validation {
                    message: format!("The request body isn't valid: {error}"),
                    fields: Default::default(),
                },
            )
        })?
    };
    let involvement = involvement.map(|involvement| match involvement {
        api::ThreadInvolvement::Nothing => ThreadInvolvement::Nothing,
        api::ThreadInvolvement::Mentions => ThreadInvolvement::Mentions,
        api::ThreadInvolvement::Everything => ThreadInvolvement::Everything,
    });
    let (thread_id, user_id) = (thread.id, viewer.id);
    let membership = c
        .app()
        .db
        .write(move |tx| {
            let mut membership = ThreadMembership::join(tx, thread_id, user_id)?;
            if let Some(involvement) = involvement {
                membership.update_involvement(tx, involvement)?;
            }
            Ok(membership)
        })
        .await;
    // `channel_threads#join`: the thread went between the lookup and the write.
    let membership = match membership {
        Ok(membership) => membership,
        Err(campfire_db::Error::RecordNotFound(_)) => {
            return Err(fail(
                c,
                api::ApiError::Validation {
                    message: "Thread is inaccessible".into(),
                    fields: Default::default(),
                },
            ));
        }
        Err(error) => return Err(db_error(error)),
    };
    c.json(
        StatusCode::OK,
        &api::ThreadMembershipState {
            membership: dto::thread_membership(&membership),
        },
    )
}

async fn destroy_join(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (thread, _, viewer) = scope(c).await?;
    let (thread_id, room_id, user_id) = (thread.id, thread.room_id, viewer.id);
    c.app()
        .db
        .write(move |tx| {
            // `channel_threads#leave`: the scoped `find_by(user)&.destroy!`, with no callbacks.
            tx.conn().execute(
                "DELETE FROM thread_memberships WHERE thread_id = ? AND user_id = ?",
                (thread_id, user_id),
            )?;
            Ok(())
        })
        .await
        .map_err(db_error)?;
    // Its pings no longer count once the thread is left.
    c.app().broadcasts.sync_read_row(user_id, room_id);
    Ok(c.head(StatusCode::NO_CONTENT))
}

async fn create_read(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (thread, room, viewer) = scope(c).await?;
    let (thread_id, user_id) = (thread.id, viewer.id);
    let membership = c
        .app()
        .db
        .write(move |tx| {
            let Some(mut membership) =
                ThreadMembership::find_by_thread_and_user(tx.conn(), thread_id, user_id)?
            else {
                return Ok(None);
            };
            membership.read(tx)?;
            Ok(Some(membership))
        })
        .await
        .map_err(db_error)?;
    let Some(membership) = membership else {
        return Err(fail(
            c,
            api::ApiError::NotFound {
                message: "Join the thread before marking it read".into(),
            },
        ));
    };
    c.app().broadcasts.thread_read(user_id, thread_id, room.id);
    c.json(
        StatusCode::OK,
        &api::ThreadMembershipState {
            membership: dto::thread_membership(&membership),
        },
    )
}
