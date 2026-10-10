//! Board reads and creation, using the classic model queries and atomic board-post writer.

use campfire_api_types as api;
use campfire_app::app::{AppCtx, AppState};
use campfire_db::models::channel_thread::{BOARD_POSTS_PER_PAGE, board_page_number};
use campfire_db::{
    ChannelThread, Connection, Membership, Message, NewChannelThread, NewMessage, Room, User,
};
use campfire_kit::{Ctx, Result, StatusCode};
use campfire_messages::controllers::messages::claim_attachment_blob;
use campfire_web::controllers::presenters::attachments::{self, Assignment};
use campfire_web::controllers::presenters::page::db_error;
use rusqlite::OptionalExtension;

use crate::agents::human;
use crate::dto;
use crate::endpoints::{before_actions, blob_exists, body, grouped_signed_ids, now, require_grouped_uploads, set_room};
use crate::error::{fail, not_found, record_invalid, validation};

endpoint!(
    /// `GET /api/v1/rooms/:room_id/board`
    index => list_board
);
endpoint!(
    /// `GET /api/v1/rooms/:room_id/posts/new`
    new => new_post
);
endpoint!(
    /// `POST /api/v1/rooms/:room_id/posts`
    create => create_post
);

async fn scope(c: &mut Ctx) -> Result<(Room, User)> {
    before_actions(c).await?;
    let Some(viewer) = human(c)?.filter(User::is_active) else {
        return Err(fail(c, not_found()));
    };
    let (_, room) = set_room(c).await?;
    if !room.board() {
        return Err(fail(c, not_found()));
    }
    Ok((room, viewer))
}

fn owner_filter(value: &str) -> String {
    if matches!(value, "me" | "agents")
        || (!value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()))
    {
        value.into()
    } else {
        "anyone".into()
    }
}

fn digest(
    conn: &Connection,
    app: &AppState,
    room_id: i64,
) -> campfire_db::Result<Option<api::BoardDigest>> {
    // Classic shows the latest claim only if its note exists; a newer unposted claim hides
    // the preceding digest, rather than displaying an older day as current.
    let latest = conn.query_row(
        "SELECT digest_on, message_id FROM board_stale_digests WHERE room_id=? ORDER BY digest_on DESC LIMIT 1",
        [room_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?)),
    ).optional()?;
    let Some((date, Some(id))) = latest else {
        return Ok(None);
    };
    Message::find_by_id(conn, id)?
        .map(|message| {
            Ok(api::BoardDigest {
                date,
                text: message.plain_text_body(conn, app.db.env().rich_text.as_ref())?,
            })
        })
        .transpose()
}

async fn list_board(c: &mut Ctx) -> Result {
    let (room, viewer) = scope(c).await?;
    let (status, stored_status) = match c.param_str("status") {
        Some("all") => (api::BoardStatusFilter::All, "all"),
        Some("done") => (api::BoardStatusFilter::Done, "done"),
        _ => (api::BoardStatusFilter::Open, "open"),
    };
    let owner = owner_filter(c.param_str("owner").unwrap_or_default());
    let tag = rails_compat::unicode::downcase(campfire_richtext::ruby::strip(
        c.param_str("tag").unwrap_or_default(),
    ));
    let page = board_page_number(c.param_str("page").unwrap_or_default());
    let (app, now) = (c.app().clone(), now(c));
    let listing = c
        .app()
        .db
        .read(move |conn| {
            let mut posts = ChannelThread::board_posts_for(
                conn,
                room.id,
                stored_status,
                &owner,
                &tag,
                Some(viewer.id),
                page,
            )?;
            let size = (page * BOARD_POSTS_PER_PAGE) as usize;
            let has_more = posts.len() > size;
            posts.truncate(size);
            let member_ids = room.user_ids(conn)?;
            let owner_options: Vec<api::BoardOwnerOption> = User::active_ordered(conn)?
                .into_iter()
                .filter(|user| member_ids.contains(&user.id))
                .map(|user| api::BoardOwnerOption {
                    user_id: user.id,
                    agent: user.is_bot(),
                })
                .collect();
            let users = dto::users(
                conn,
                &app.secrets,
                posts
                    .iter()
                    .map(|post| post.creator_id)
                    .chain(owner_options.iter().map(|option| option.user_id)),
                now,
            )?;
            Ok(api::BoardListing {
                room_id: room.id,
                status,
                owner,
                tag,
                page,
                posts: dto::thread_summaries(conn, &app.secrets, viewer.id, &room, &posts, now)?,
                has_more,
                any_posts: ChannelThread::board_has_posts(conn, room.id)?,
                owner_options,
                tag_counts: ChannelThread::board_tag_counts(conn, room.id)?
                    .into_iter()
                    .map(|(name, count)| api::BoardTagCount { name, count })
                    .collect(),
                digest: digest(conn, &app, room.id)?,
                can_administer: viewer.can_administer(Some(room.creator_id), false),
                users,
            })
        })
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &listing)
}

async fn new_post(c: &mut Ctx) -> Result {
    let (room, _) = scope(c).await?;
    let (secrets, now) = (c.app().secrets.clone(), now(c));
    let form = c
        .app()
        .db
        .read(move |conn| {
            let owner_candidates = crate::work::owner_candidates(conn, room.id)?;
            Ok(api::BoardPostForm {
                users: dto::users(
                    conn,
                    &secrets,
                    owner_candidates.iter().map(|candidate| candidate.user_id),
                    now,
                )?,
                owner_candidates,
                tag_suggestions: ChannelThread::board_tag_counts(conn, room.id)?
                    .into_iter()
                    .map(|(name, _)| name)
                    .collect(),
            })
        })
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &form)
}

/// The receipt kind for a board post made with a `clientPostId`.
/// The longest `clientPostId` honoured.
const CLIENT_POST_ID_LIMIT: usize = 255;

/// A client id is a board creation receipt only when it names this creator's board opener.
fn duplicate(
    conn: &Connection,
    room_id: i64,
    creator_id: i64,
    client_id: &str,
) -> campfire_db::Result<Option<ChannelThread>> {
    let Some(message) = Message::find_duplicate(conn, room_id, creator_id, client_id)? else {
        return Ok(None);
    };
    if message.board_post_opener
        && let Some(id) = message.thread_id
    {
        return ChannelThread::find(conn, id).map(Some);
    }
    let mut errors = campfire_db::Errors::default();
    errors.add(
        "message",
        "clientMessageId is already used by another message",
    );
    Err(campfire_db::Error::RecordInvalid(errors))
}

async fn create_post(c: &mut Ctx) -> Result {
    let (room, viewer) = scope(c).await?;
    let input: api::CreateBoardPost = body(c).await?;
    let (room_id, creator_id) = (room.id, viewer.id);
    let message = input.message.filter(|message| {
        !campfire_richtext::ruby::is_blank(campfire_richtext::ruby::strip(&message.markdown_source))
            || message
                .attachment_signed_id
                .as_ref()
                .is_some_and(|id| !id.is_empty())
            || message
                .attachment_signed_ids
                .as_ref()
                .is_some_and(|ids| !ids.is_empty())
            || message
                .drive_file_ids
                .as_ref()
                .is_some_and(|ids| !ids.is_empty())
    });
    let client_id = message
        .as_ref()
        .map(|message| message.client_message_id.trim().to_string());
    let post_key = input
        .client_post_id
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map(str::to_string);
    if post_key
        .as_ref()
        .is_some_and(|key| key.chars().count() > CLIENT_POST_ID_LIMIT)
    {
        return Err(fail(
            c,
            validation("clientPostId", "is too long (maximum is 255 characters)"),
        ));
    }
    let lookup = client_id.clone();
    let replay = c
        .app()
        .db
        .read(move |conn| {
            lookup
                .as_deref()
                .map(|id| duplicate(conn, room_id, creator_id, id))
                .transpose()
                .map(Option::flatten)
        })
        .await;
    let replay = match replay {
        Ok(replay) => replay,
        Err(campfire_db::Error::RecordInvalid(errors)) => {
            return Err(fail(c, record_invalid(&errors, &[])));
        }
        Err(error) => return Err(db_error(error)),
    };
    if let Some(thread) = replay {
        let detail = crate::threads::detail(c, viewer, thread.id).await?;
        return c.json(StatusCode::OK, &detail);
    }
    if client_id.as_deref() == Some("") {
        return Err(fail(
            c,
            validation("message", "clientMessageId can't be blank"),
        ));
    }
    if message.as_ref().is_some_and(|message| {
        message.markdown_source.chars().count() > campfire_db::message::SOURCE_LIMIT
    }) {
        return Err(fail(
            c,
            validation("message", "is too long (maximum is 50000 characters)"),
        ));
    }
    // A new post has no messages yet, so its opener cannot reply on that conversation.
    if message
        .as_ref()
        .is_some_and(|message| message.reply_to_message_id.is_some())
    {
        return Err(fail(
            c,
            validation(
                "message",
                "replyToMessageId isn't a message on this timeline",
            ),
        ));
    }
    let signed_id = message
        .as_ref()
        .and_then(|message| message.attachment_signed_id.clone())
        .filter(|id| !id.is_empty());
    let signed_ids = grouped_signed_ids(c, signed_id.as_deref(), message.as_ref()
        .and_then(|message| message.attachment_signed_ids.clone()).unwrap_or_default())?;
    require_grouped_uploads(c, &signed_ids).await?;
    if let Some(id) = &signed_id
        && !blob_exists(c, id).await?
    {
        return Err(fail(
            c,
            validation("message", "attachmentSignedId isn't a finished upload"),
        ));
    }
    let attachment = match signed_id {
        Some(id) => Assignment::Signed(id).stage(c.app()).await?,
        None => Assignment::Unchanged,
    };
    let files = attachments::stage_many(c.app(), signed_ids.into_iter().map(Assignment::Signed).collect()).await?;
    let message = if let Some(mut message) = message {
        message.drive_file_ids = Some(crate::drive::require_drive_file_ids(
            c,
            message.drive_file_ids.as_deref().unwrap_or(&[]),
        )?);
        Some(message)
    } else {
        None
    };
    let outcome = c
        .app()
        .db
        .write(move |tx| {
            let room = Room::find(tx.conn(), room_id)?;
            if !room.board()
                || room.deleted_at.is_some()
                || Membership::find_by_room_and_user(tx.conn(), room_id, creator_id)?.is_none()
                || !User::find(tx.conn(), creator_id)?.is_active()
            {
                return Err(campfire_db::Error::RecordNotFound("Room"));
            }
            if let Some(id) = client_id.as_deref()
                && let Some(thread) = duplicate(tx.conn(), room_id, creator_id, id)?
            {
                return Ok((thread.id, false));
            }
            if let Some(key) = post_key.as_deref()
                && let Some(thread) =
                    ChannelThread::find_by_client_post_id(tx.conn(), room_id, creator_id, key)?
            {
                return Ok((thread.id, false));
            }
            let blob = claim_attachment_blob(tx, attachment, creator_id, "attachment_signed_id")?;
            let mut blobs = Vec::with_capacity(files.len() + 1);
            for file in files {
                if let Some(blob) = claim_attachment_blob(tx, file, creator_id, "attachment_signed_ids")? { blobs.push(blob); }
            }
            let message = message.map(|message| NewMessage {
                markdown_source: Some(message.markdown_source),
                client_message_id: client_id,
                attachment_blob_id: blob.as_ref().map(|blob| blob.id),
                attachment_blob_ids: blobs.iter().map(|blob| blob.id).collect(),
                reply_to_message_id: message.reply_to_message_id,
                reply_notify_author: message.reply_notify_author,
                drive_file_ids: message.drive_file_ids.unwrap_or_default(),
                ..Default::default()
            });
            let (thread, opener) = ChannelThread::create_board_post_with_message(
                tx,
                NewChannelThread {
                    room_id,
                    creator_id,
                    name: Some(input.name),
                    work_status: Some(crate::work::stored_status(input.status).to_string()),
                    work_owner_id: input.owner_id,
                    tag_names: Some(input.tags),
                    client_post_id: post_key,
                    ..Default::default()
                },
                message,
            )?;
            if let Some(opener) = &opener {
                blobs.extend(blob);
                for blob in &blobs { attachments::enqueue_analysis(tx, blob); }
                campfire_db::models::message_attachment_processing::schedule_message(tx, opener)?;
            }
            Ok((thread.id, true))
        })
        .await;
    let (id, created) = match outcome {
        Ok(outcome) => outcome,
        Err(campfire_db::Error::RecordInvalid(errors)) => {
            return Err(fail(
                c,
                record_invalid(
                    &errors,
                    &[
                        ("work_owner", "ownerId"),
                        ("work_status", "status"),
                        ("markdown_source", "message"),
                        ("reply_to_message", "message"),
                    ],
                ),
            ));
        }
        Err(error) => return Err(db_error(error)),
    };
    let detail = crate::threads::detail(c, viewer, id).await?;
    c.json(
        if created {
            StatusCode::CREATED
        } else {
            StatusCode::OK
        },
        &detail,
    )
}
