//! Member catalog, administrator writes, and private original/still sticker media.

use axum::{
    Router,
    routing::{get, patch},
};
use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_db::{
    CachedStatements, Message,
    models::{
        audit_log::{AuditLog, NewAuditLog, Target},
        sticker::{CAPACITY, MAX_IMAGE_BYTES, Sticker, StickerValues},
    },
};
use campfire_kit::{
    Ctx, Error, ExpiresIn, Kit, Result, SendOptions, StatusCode, action, unparsed_action,
};
use campfire_runtime::{
    concerns::{self, Before},
    context::db_error,
    presenters::attachments::{self, Assignment, Record, Upload},
};
use campfire_storage::{
    Blob,
    branding::{Kind, PreparedEmoji},
};

use crate::{
    admin::{administrator, body, viewer},
    dto::time,
    error::{fail, validation},
};

pub fn routes() -> Router<Kit> {
    Router::new()
        .route("/api/v1/stickers", get(action(index)))
        .route(
            "/api/v1/admin/stickers",
            get(action(admin_index)).post(unparsed_action(create)),
        )
        .route(
            "/api/v1/admin/stickers/{id}",
            patch(unparsed_action(update)).delete(action(destroy)),
        )
        .route("/stickers/{id}", get(action(media)))
}

endpoint!(index => list);
endpoint!(admin_index => admin_list);
endpoint!(create => create_sticker);
endpoint!(update => update_sticker);
endpoint!(destroy => delete_sticker);

fn paths(id: i64, animated: bool) -> (String, String) {
    let image = format!("/stickers/{id}");
    let still = if animated {
        format!("{image}?still=1")
    } else {
        image.clone()
    };
    (image, still)
}

pub(crate) fn message_sticker(
    conn: &campfire_db::Connection,
    message: &Message,
) -> campfire_db::Result<Option<api::MessageSticker>> {
    campfire_runtime::presenters::stickers::message_sticker(
        conn, message, campfire_runtime::presenters::stickers::StickerUrls::Session,
    )
}

async fn list(c: &mut Ctx) -> Result {
    viewer(c).await?;
    catalog(c, StatusCode::OK).await
}

async fn admin_list(c: &mut Ctx) -> Result {
    administrator(c).await?;
    catalog(c, StatusCode::OK).await
}

async fn catalog(c: &mut Ctx, status: StatusCode) -> Result {
    let stickers = c
        .app()
        .db
        .read(|conn| {
            Sticker::list(conn)?
                .into_iter()
                .map(|sticker| {
                    let blob = attachments::attached_blob(conn, "Sticker", sticker.id, "image")?
                        .ok_or(campfire_db::Error::RecordNotFound("sticker image"))?;
                    let animated = campfire_storage::workspace_icon::animated(&blob);
                    let (image_url, still_url) = paths(sticker.id, animated);
                    Ok(api::Sticker {
                        id: sticker.id,
                        name: sticker.name,
                        description: sticker.description,
                        tags: sticker.tags,
                        image_url,
                        still_url,
                        animated,
                        uploader_id: sticker.uploader_id,
                        created_at: time(sticker.created_at),
                        updated_at: time(sticker.updated_at),
                    })
                })
                .collect::<campfire_db::Result<Vec<_>>>()
        })
        .await
        .map_err(Error::internal)?;
    c.json(
        status,
        &api::StickerCatalog {
            stickers,
            capacity: CAPACITY,
        },
    )
}

async fn input(c: &mut Ctx) -> Result<(api::CreateSticker, Assignment<campfire_storage::Staged>)> {
    let multipart = c
        .request
        .header("content-type")
        .is_some_and(|value| value.starts_with("multipart/form-data"));
    let (input, assignment) = if multipart {
        c.parse_spooled_params().await?;
        let input = api::CreateSticker {
            name: c.param_str("name").unwrap_or_default().into(),
            description: c.param_str("description").unwrap_or_default().into(),
            tags: c.param_str("tags").unwrap_or_default().into(),
            signed_id: None,
        };
        let upload = Upload::from_param(c.params.get("image"))
            .ok_or_else(|| fail(c, validation("image", "must be attached")))?;
        if upload.file.size > MAX_IMAGE_BYTES as u64 {
            return Err(fail(c, validation("image", "must be 512 KB or smaller")));
        }
        (input, Assignment::Create(upload))
    } else {
        let input: api::CreateSticker = body(c).await?;
        let signed = input.signed_id.as_deref().unwrap_or_default();
        let id = campfire_storage::paths::verify_signed_blob_id(
            &*c.app().storage.verifier,
            signed,
            c.app().clock.now(),
        )
        .ok_or_else(|| fail(c, validation("image", "isn't an uploaded file")))?;
        let blob = c
            .app()
            .db
            .read(move |conn| Blob::find(conn, id).map_err(attachments::storage_error))
            .await
            .map_err(Error::internal)?
            .ok_or_else(|| fail(c, validation("image", "isn't an uploaded file")))?;
        (input, Assignment::Existing(blob))
    };
    Ok((input, assignment.stage(c.app()).await?))
}

async fn prepare(
    c: &mut Ctx,
    assignment: &Assignment<campfire_storage::Staged>,
) -> Result<PreparedEmoji> {
    let (key, filename, size) = match assignment {
        Assignment::Create(staged) => (
            staged.blob().key.clone(),
            staged.blob().filename.clone(),
            staged.blob().byte_size,
        ),
        Assignment::Existing(blob) => (blob.key.clone(), blob.filename.clone(), blob.byte_size),
        _ => return Err(fail(c, validation("image", "must be attached"))),
    };
    let storage = c.app().storage.clone();
    let prepared = campfire_runtime::active_storage::process_branding_with_deadline(
        std::time::Duration::from_secs(10),
        move |cancel| {
            Ok(campfire_storage::sticker::prepare(
                &storage, &key, &filename, size, &cancel,
            ))
        },
    )
    .await?;
    prepared.map_err(|message| fail(c, validation("image", &message)))
}

fn audit(
    tx: &campfire_db::Tx<'_>,
    sticker: &Sticker,
    action: &str,
    context: &campfire_db::models::audit_log::Context,
) -> campfire_db::Result<()> {
    AuditLog::record(
        tx,
        NewAuditLog {
            action: action.into(),
            target: Some(Target {
                record_type: "Sticker".into(),
                id: sticker.id,
                label: Some(sticker.name.clone()),
            }),
            ..Default::default()
        },
        context,
    )?;
    Ok(())
}

async fn create_sticker(c: &mut Ctx) -> Result {
    let user = administrator(c).await?;
    let (input, assignment) = input(c).await?;
    let prepared = prepare(c, &assignment).await?;
    let storage = c.app().storage.clone();
    let context = campfire_people::controllers::two_factor::audit_context(c)?;
    c.app()
        .db
        .write(move |tx| {
            let sticker = Sticker::create(
                tx,
                user.id,
                StickerValues {
                    name: input.name,
                    description: input.description,
                    tags: input.tags,
                },
            )?;
            let mut blob = match assignment {
                Assignment::Create(staged) => {
                    let blob = staged
                        .insert(tx.conn(), tx.now().jiff())
                        .map_err(attachments::storage_error)?;
                    campfire_runtime::active_storage::keep_after_commit(tx, staged);
                    blob
                }
                Assignment::Existing(blob) => blob,
                _ => return Err(campfire_db::Error::RecordNotFound("sticker image")),
            };
            blob.content_type = Some(prepared.content_type.into());
            blob.metadata.merge(&prepared.metadata);
            tx.conn().execute_cached(
                "UPDATE active_storage_blobs SET content_type=?,metadata=? WHERE id=?",
                rusqlite::params![blob.content_type, blob.metadata.encode(), blob.id],
            )?;
            if let Some((variation, still)) = prepared.still {
                if storage
                    .record_variant(tx.conn(), &blob, &variation, &still, tx.now().jiff())
                    .map_err(attachments::storage_error)?
                    .is_some()
                {
                    campfire_runtime::active_storage::keep_after_commit(tx, still);
                } else {
                    storage
                        .existing_variant_file(tx.conn(), &blob, &variation)
                        .map_err(attachments::storage_error)?
                        .ok_or(campfire_db::Error::RecordNotFound("sticker still"))?;
                }
            }
            attachments::assign(
                tx,
                Record::sticker(sticker.id),
                "image",
                Assignment::Existing(blob),
            )?;
            audit(tx, &sticker, "sticker.create", &context)
        })
        .await
        .map_err(db_error)?;
    catalog(c, StatusCode::CREATED).await
}

fn id(c: &Ctx) -> Result<i64> {
    c.param_str("id")
        .and_then(|id| id.parse().ok())
        .ok_or(Error::NotFound)
}

async fn update_sticker(c: &mut Ctx) -> Result {
    administrator(c).await?;
    let id = id(c)?;
    let input: api::UpdateSticker = body(c).await?;
    let context = campfire_people::controllers::two_factor::audit_context(c)?;
    c.app()
        .db
        .write(move |tx| {
            let sticker = Sticker::find(tx.conn(), id)?.update(
                tx,
                StickerValues {
                    name: input.name,
                    description: input.description,
                    tags: input.tags,
                },
            )?;
            audit(tx, &sticker, "sticker.update", &context)
        })
        .await
        .map_err(db_error)?;
    catalog(c, StatusCode::OK).await
}

async fn delete_sticker(c: &mut Ctx) -> Result {
    administrator(c).await?;
    let id = id(c)?;
    let context = campfire_people::controllers::two_factor::audit_context(c)?;
    c.app()
        .db
        .write(move |tx| {
            let sticker = Sticker::find(tx.conn(), id)?;
            attachments::destroy(tx, Record::sticker(id), "image")?;
            sticker.destroy(tx)?;
            audit(tx, &sticker, "sticker.destroy", &context)
        })
        .await
        .map_err(db_error)?;
    catalog(c, StatusCode::OK).await
}

async fn media(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default().allow_unauthenticated_access()).await?;
    if !concerns::restore_authentication(c).await? {
        return Ok(c.head(StatusCode::NOT_FOUND));
    }
    let id = id(c)?;
    let mut blob = c
        .app()
        .db
        .read(move |conn| {
            Sticker::find(conn, id)?;
            attachments::attached_blob(conn, "Sticker", id, "image")
        })
        .await
        .map_err(db_error)?
        .ok_or(Error::NotFound)?;
    if c.param_str("still") == Some("1") && campfire_storage::workspace_icon::animated(&blob) {
        let storage = c.app().storage.clone();
        blob = c
            .app()
            .db
            .read(move |conn| {
                storage
                    .existing_variant_file(conn, &blob, &Kind::Sticker.still_variation())
                    .map_err(attachments::storage_error)
            })
            .await
            .map_err(db_error)?
            .ok_or(Error::NotFound)?;
    }
    c.expires_in(3600, ExpiresIn::default());
    c.set_header(
        "etag",
        &format!("\"{}\"", blob.checksum.as_deref().unwrap_or_default()),
    );
    c.set_header("x-content-type-options", "nosniff");
    if c.is_fresh() {
        return Ok(c.head(StatusCode::NOT_MODIFIED));
    }
    c.send_file(
        c.app().storage.service.path_for(&blob.key),
        SendOptions::inline(blob.content_type()),
    )
    .map_err(|_| Error::NotFound)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sticker_deleted_after_message_load_serializes_the_retained_name() {
        let mut conn = campfire_db::Connection::open_in_memory().unwrap();
        campfire_db::schema::prepare(&mut conn, "production", &campfire_db::SystemClock).unwrap();
        conn.execute_batch(
            "INSERT INTO users(id,name,created_at,updated_at) VALUES(1,'David','2026-10-12 00:00:00','2026-10-12 00:00:00');
             INSERT INTO rooms(id,type,creator_id,created_at,updated_at) VALUES(1,'Rooms::Open',1,'2026-10-12 00:00:00','2026-10-12 00:00:00');
             INSERT INTO stickers(id,name,name_key,uploader_id,created_at,updated_at) VALUES(1,'Hello','hello',1,'2026-10-12 00:00:00','2026-10-12 00:00:00');
             INSERT INTO messages(id,room_id,creator_id,client_message_id,sticker_id,sticker_name,created_at,updated_at) VALUES(1,1,1,'stale-sticker',1,'Hello','2026-10-12 00:00:00','2026-10-12 00:00:00');",
        ).unwrap();
        let loaded = Message::find(&conn, 1).unwrap();
        assert_eq!(loaded.sticker_id, Some(1));
        conn.execute("DELETE FROM stickers WHERE id=1", []).unwrap();
        let reloaded = Message::find(&conn, 1).unwrap();
        assert_eq!(reloaded.sticker_id, None);
        let placeholder = Some(api::MessageSticker::Deleted {
            name: "Hello".into(),
        });
        assert_eq!(message_sticker(&conn, &loaded).unwrap(), placeholder);
        assert_eq!(message_sticker(&conn, &reloaded).unwrap(), placeholder);
    }
}
