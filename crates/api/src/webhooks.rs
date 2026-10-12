//! Room-scoped incoming webhooks. Delivery uses the managed-bot message save and broadcasts.
use axum::{
    Router,
    routing::{get, post},
};
use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_db::{
    Attachment, Message, Room, User,
    models::channel_webhook::{ChannelWebhook, WebhookMessage},
};
use campfire_kit::{Ctx, Error, Kit, Param, Result, StatusCode, action, unparsed_action};
use campfire_messages::controllers::messages::{self, AttachmentPolicy, MessageParams};
use campfire_runtime::{
    concerns,
    context::db_error,
    presenters::attachments::{Assignment, Upload},
};

use crate::{
    dto,
    endpoints::{before_actions, body, set_room},
    error::{fail, validation},
};

pub fn routes() -> Router<Kit> {
    Router::new()
        .route("/api/webhooks/{id}/{token}", post(unparsed_action(execute)))
        .route(
            "/api/v1/rooms/{room_id}/webhooks",
            get(action(index)).post(unparsed_action(create)),
        )
        .route(
            "/api/v1/rooms/{room_id}/webhooks/{id}",
            axum::routing::patch(unparsed_action(update)).delete(action(destroy)),
        )
        .route(
            "/api/v1/rooms/{room_id}/webhooks/{id}/regenerate_token",
            post(action(regenerate)),
        )
        .route(
            "/api/v1/rooms/{room_id}/webhooks/{id}/test",
            post(action(test)),
        )
}

endpoint!(index => list_webhooks);
endpoint!(create => create_webhook);
endpoint!(update => update_webhook);
endpoint!(destroy => delete_webhook);
endpoint!(regenerate => regenerate_webhook);
endpoint!(test => test_webhook);
endpoint!(execute => execute_webhook);

fn id(c: &Ctx) -> Result<i64> {
    c.param_str("id")
        .and_then(concerns::cast_integer)
        .ok_or(Error::NotFound)
}

async fn managed_room(c: &mut Ctx) -> Result<Room> {
    before_actions(c).await?;
    let (_, room) = set_room(c).await?;
    campfire_rooms::controllers::rooms::ensure_can_administer(c, &room)?;
    Ok(room)
}

async fn managed_webhook(c: &mut Ctx) -> Result<ChannelWebhook> {
    let room = managed_room(c).await?;
    let id = id(c)?;
    c.app()
        .db
        .read(move |conn| ChannelWebhook::find(conn, id))
        .await
        .map_err(db_error)?
        .filter(|hook| hook.room_id == room.id)
        .ok_or(Error::NotFound)
}

fn avatar_url(
    conn: &campfire_db::Connection,
    app: &campfire_app::app::AppState,
    attachment: Option<Attachment>,
) -> campfire_db::Result<Option<String>> {
    attachment
        .map(|attachment| {
            let blob = campfire_storage::Blob::find(conn, attachment.blob_id)
                .map_err(campfire_runtime::presenters::attachments::storage_error)?
                .ok_or(campfire_db::Error::RecordNotFound("Blob"))?;
            Ok(campfire_storage::paths::blob_redirect_path(
                &*app.storage.verifier,
                &blob,
                Some("inline"),
            ))
        })
        .transpose()
}

pub(crate) struct MessagePayload {
    pub embeds: Vec<api::WebhookEmbed>,
    pub attribution: api::MessageWebhook,
}

pub(crate) fn message_payloads(
    conn: &campfire_db::Connection,
    app: &campfire_app::app::AppState,
    ids: &[i64],
) -> campfire_db::Result<std::collections::BTreeMap<i64, MessagePayload>> {
    ChannelWebhook::message_payloads(conn, ids)?
        .into_iter()
        .map(|(id, payload)| {
            let avatar_url = avatar_url(
                conn,
                app,
                Attachment::find_for(conn, "Message", id, "webhook_avatar")?,
            )?;
            Ok((
                id,
                MessagePayload {
                    embeds: serde_json::from_value(payload.embeds)
                        .map_err(|error| campfire_db::Error::Other(error.to_string()))?,
                    attribution: api::MessageWebhook {
                        name: payload.name,
                        username: payload.username,
                        avatar_url,
                    },
                },
            ))
        })
        .collect()
}

async fn view(c: &Ctx, hook: ChannelWebhook) -> Result<api::ChannelWebhook> {
    let app = c.app().clone();
    c.app()
        .db
        .read(move |conn| {
            Ok(api::ChannelWebhook {
                id: hook.id,
                room_id: hook.room_id,
                creator_id: hook.creator_id,
                name: hook.name,
                avatar_url: avatar_url(
                    conn,
                    &app,
                    Attachment::find_for(conn, "ChannelWebhook", hook.id, "avatar")?,
                )?,
                last_used_at: hook.last_used_at.map(dto::time),
                created_at: dto::time(hook.created_at),
                updated_at: dto::time(hook.updated_at),
            })
        })
        .await
        .map_err(db_error)
}

async fn credential(
    c: &mut Ctx,
    hook: ChannelWebhook,
    token: String,
    status: StatusCode,
) -> Result {
    let url = c.url_for(&format!("/api/webhooks/{}/{token}", hook.id));
    let webhook = view(c, hook).await?;
    c.json(
        status,
        &api::ChannelWebhookCredential {
            webhook,
            token,
            url,
        },
    )
}

async fn list_webhooks(c: &mut Ctx) -> Result {
    let room = managed_room(c).await?;
    let hooks = c
        .app()
        .db
        .read(move |conn| ChannelWebhook::list_for_room(conn, room.id))
        .await
        .map_err(db_error)?;
    let mut webhooks = Vec::with_capacity(hooks.len());
    for hook in hooks {
        webhooks.push(view(c, hook).await?);
    }
    c.json(StatusCode::OK, &api::ChannelWebhookList { webhooks })
}

async fn avatar(
    c: &mut Ctx,
    signed_id: Option<Option<String>>,
) -> Result<Option<Assignment<campfire_storage::Staged>>> {
    let Some(signed_id) = signed_id else {
        return Ok(None);
    };
    let assignment = match signed_id {
        None => Assignment::Delete,
        Some(signed_id) => {
            if campfire_storage::paths::verify_signed_blob_id(
                &*c.app().storage.verifier,
                &signed_id,
                c.app().clock.now(),
            )
            .is_none()
            {
                return Err(fail(
                    c,
                    validation("avatarSignedId", "isn't an uploaded file"),
                ));
            }
            Assignment::Signed(signed_id)
        }
    };
    Ok(Some(messages::stage_attachment(c, assignment).await?))
}

fn save_avatar(
    tx: &mut campfire_db::Tx<'_>,
    hook_id: i64,
    assignment: Option<Assignment<campfire_storage::Staged>>,
) -> campfire_db::Result<()> {
    if let Some(assignment) = assignment {
        let blob = messages::attachment_blob(
            tx,
            assignment,
            AttachmentPolicy::SignedBlob { actor_id: None },
            "avatarSignedId",
        )?;
        if let Some(blob) = &blob {
            if !blob.is_image() {
                let mut errors = campfire_db::Errors::default();
                errors.add("avatarSignedId", "must be an image");
                errors.into_result()?;
            }
            campfire_runtime::presenters::attachments::enqueue_analysis(tx, blob);
        }
        ChannelWebhook::set_avatar(tx, hook_id, blob.map(|b| b.id))?;
    }
    Ok(())
}

async fn create_webhook(c: &mut Ctx) -> Result {
    let room = managed_room(c).await?;
    let creator = concerns::require_current_user(c)?.id;
    let input: api::CreateChannelWebhook = body(c).await?;
    let avatar = avatar(c, input.avatar_signed_id.map(Some)).await?;
    let (hook, token) = c
        .app()
        .db
        .write(move |tx| {
            let (hook, token) = ChannelWebhook::create(tx, room.id, creator, &input.name)?;
            save_avatar(tx, hook.id, avatar)?;
            Ok((
                ChannelWebhook::find(tx.conn(), hook.id)?
                    .ok_or(campfire_db::Error::RecordNotFound("ChannelWebhook"))?,
                token,
            ))
        })
        .await
        .map_err(db_error)?;
    credential(c, hook, token, StatusCode::CREATED).await
}

async fn update_webhook(c: &mut Ctx) -> Result {
    let hook = managed_webhook(c).await?;
    let input: api::UpdateChannelWebhook = body(c).await?;
    let avatar = avatar(c, input.avatar_signed_id).await?;
    let hook = c
        .app()
        .db
        .write(move |tx| {
            if let Some(name) = input.name {
                ChannelWebhook::rename(tx, hook.id, &name)?;
            }
            save_avatar(tx, hook.id, avatar)?;
            ChannelWebhook::find(tx.conn(), hook.id)?
                .ok_or(campfire_db::Error::RecordNotFound("ChannelWebhook"))
        })
        .await
        .map_err(db_error)?;
    let hook = view(c, hook).await?;
    c.json(StatusCode::OK, &hook)
}

async fn delete_webhook(c: &mut Ctx) -> Result {
    let hook = managed_webhook(c).await?;
    c.app()
        .db
        .write(move |tx| ChannelWebhook::delete(tx, hook.id))
        .await
        .map_err(db_error)?;
    Ok(c.head(StatusCode::NO_CONTENT))
}

async fn regenerate_webhook(c: &mut Ctx) -> Result {
    let hook = managed_webhook(c).await?;
    let (hook, token) = c
        .app()
        .db
        .write(move |tx| ChannelWebhook::regenerate(tx, hook.id))
        .await
        .map_err(db_error)?;
    credential(c, hook, token, StatusCode::OK).await
}

async fn test_webhook(c: &mut Ctx) -> Result {
    let hook = managed_webhook(c).await?;
    let message = deliver(
        c,
        hook,
        None,
        api::ExecuteWebhook {
            content: Some("This is a test webhook message.".into()),
            username: None,
            embeds: Vec::new(),
        },
        Vec::new(),
    )
    .await?;
    render_message(c, message).await
}

async fn execute_webhook(c: &mut Ctx) -> Result {
    let id = id(c)?;
    let digest = ChannelWebhook::digest(c.param_str("token").ok_or(Error::NotFound)?);
    let lookup = digest.clone();
    let hook = c
        .app()
        .db
        .read(move |conn| {
            let Some(hook) =
                ChannelWebhook::find_by_token_digest(conn, &lookup)?.filter(|hook| hook.id == id)
            else {
                return Ok(None);
            };
            Ok((!Room::find(conn, hook.room_id)?.deleted()).then_some(hook))
        })
        .await
        .map_err(db_error)?
        .ok_or(Error::NotFound)?;
    let now = c.clock().now();
    let key = format!("channel_webhook/{}/{}", hook.id, now.as_second() / 60);
    if c.kit()
        .rate_limits()
        .increment(&key, jiff::SignedDuration::from_secs(60), now)
        > 30
    {
        return Err(fail(
            c,
            api::ApiError::RateLimited {
                message: "Slow down and try again".into(),
                retry_after: u32::try_from(60 - now.as_second().rem_euclid(60)).unwrap_or(60),
            },
        ));
    }
    let multipart = c
        .request
        .headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|value| value.starts_with("multipart/form-data"));
    let (mut input, files) = if multipart {
        c.parse_spooled_params().await?;
        let payload = c.param_str("payload_json").unwrap_or("{}");
        let input = serde_json::from_str(payload).map_err(|_| {
            fail(
                c,
                validation("payload_json", "must contain a valid webhook payload"),
            )
        })?;
        let mut files = Vec::new();
        if let Some(param) = c.params.get("files").cloned() {
            let parts: Vec<&Param> = match &param {
                Param::Hash(map) => map.iter().map(|(_, p)| p).collect(),
                Param::Array(array) => array.iter().collect(),
                _ => vec![&param],
            };
            for part in parts {
                let upload = Upload::from_param(Some(part))
                    .ok_or_else(|| fail(c, validation("files", "must be uploaded files")))?;
                files.push(Assignment::Create(upload));
            }
        }
        if files.len() > 10 {
            return Err(fail(
                c,
                validation("files", "must contain at most 10 files"),
            ));
        }
        (input, files)
    } else {
        (body::<api::ExecuteWebhook>(c).await?, Vec::new())
    };
    validate(c, &mut input, files.len())?;
    let wait = c.query_params.get("wait").and_then(Param::as_str) == Some("true");
    let message = deliver(c, hook, Some(digest), input, files).await?;
    if wait {
        render_message(c, message).await
    } else {
        Ok(c.head(StatusCode::NO_CONTENT))
    }
}

fn validate(c: &mut Ctx, input: &mut api::ExecuteWebhook, files: usize) -> Result<()> {
    let invalid = |c: &mut Ctx, field, reason| Err(fail(c, validation(field, reason)));
    if input
        .content
        .as_ref()
        .is_some_and(|s| s.chars().count() > campfire_db::models::message::SOURCE_LIMIT)
    {
        return invalid(c, "content", "is too long");
    }
    if let Some(name) = &mut input.username {
        *name = name.trim().to_string();
        if name.is_empty() || name.chars().count() > 80 {
            return invalid(c, "username", "must have 1 to 80 characters");
        }
    }
    if input.embeds.len() > 10 {
        return invalid(c, "embeds", "must contain at most 10 embeds");
    }
    let mut total = 0;
    for embed in &mut input.embeds {
        let mut text = Vec::new();
        if let Some(title) = &mut embed.title {
            text.push((title, 256));
        }
        if let Some(description) = &mut embed.description {
            text.push((description, 4096));
        }
        if let Some(footer) = &mut embed.footer {
            text.push((&mut footer.text, 2048));
        }
        if let Some(fields) = &mut embed.fields {
            if fields.len() > 25 {
                return invalid(c, "embeds", "must contain at most 25 fields per embed");
            }
            for field in fields {
                text.push((&mut field.name, 256));
                text.push((&mut field.value, 1024));
            }
        }
        for (value, max) in text {
            *value = value.trim().to_string();
            let length = value.chars().count();
            if length == 0 || length > max {
                return invalid(c, "embeds", "contains an empty or oversized text field");
            }
            total += length;
        }
        if embed.color.is_some_and(|color| color > 0xffffff) {
            return invalid(c, "embeds", "color must be between 0 and 16777215");
        }
        if let Some(url) = &embed.url {
            let valid = campfire_richtext::uri::parse(url)
                .ok()
                .is_some_and(|uri| uri.is_http() && uri.host.is_some());
            if !valid {
                return invalid(c, "embeds", "URL must be an absolute HTTP or HTTPS link");
            }
        }
        if embed
            .timestamp
            .as_ref()
            .is_some_and(|stamp| stamp.parse::<jiff::Timestamp>().is_err())
        {
            return invalid(c, "embeds", "timestamp must be an RFC 3339 instant");
        }
        if embed.title.is_none()
            && embed.description.is_none()
            && embed.fields.as_ref().is_none_or(Vec::is_empty)
            && embed.footer.is_none()
        {
            return invalid(c, "embeds", "must contain text");
        }
    }
    if total > 6000 {
        return invalid(
            c,
            "embeds",
            "must contain at most 6000 text characters in total",
        );
    }
    if input.content.as_ref().is_none_or(|s| s.trim().is_empty())
        && input.embeds.is_empty()
        && files == 0
    {
        return invalid(c, "content", "requires content, embeds or files");
    }
    Ok(())
}

async fn deliver(
    c: &mut Ctx,
    hook: ChannelWebhook,
    token_digest: Option<String>,
    input: api::ExecuteWebhook,
    files: Vec<Assignment>,
) -> Result<Message> {
    let (room, author) = c
        .app()
        .db
        .read(move |conn| {
            Ok((
                Room::find(conn, hook.room_id)?,
                User::find(conn, hook.author_id)?,
            ))
        })
        .await
        .map_err(db_error)?;
    if room.board() {
        return Err(fail(
            c,
            validation("room", "cannot accept webhook messages"),
        ));
    }
    c.set_current(concerns::CurrentUser(author));
    let parameters = MessageParams {
        markdown_source: Some(input.content.unwrap_or_default()),
        attachments: files,
        webhook: Some(WebhookMessage::Post {
            webhook_id: hook.id,
            token_digest,
            username: input.username,
            embeds: serde_json::to_value(input.embeds).map_err(Error::internal)?,
        }),
        ..Default::default()
    };
    let message = messages::create_message_into(c, &room, None, parameters).await?;
    messages::broadcast_create(c, &room, &message).await?;
    messages::release_webhooks(c, &message).await;
    Ok(message)
}

async fn render_message(c: &mut Ctx, message: Message) -> Result {
    let app = c.app().clone();
    let message = c
        .app()
        .db
        .read(move |conn| dto::message(conn, &app, &message))
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &message)
}
