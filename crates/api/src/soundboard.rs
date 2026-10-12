//! The voice soundboard catalog, using the same signed uploads and attachments as custom emoji.

use axum::{
    Router,
    routing::{get, patch},
};
use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_db::CachedStatements;
use campfire_db::models::audit_log::{AuditLog, NewAuditLog, Target};
use campfire_db::models::soundboard::{
    AudioFacts, CAPACITY, MAX_AUDIO_BYTES, SoundboardSound, SoundboardValues,
};
use campfire_kit::{Ctx, Error, Kit, Result, StatusCode, action, unparsed_action};
use campfire_people::controllers::two_factor::audit_context;
use campfire_runtime::context::db_error;
use campfire_runtime::presenters::attachments::{self, Assignment, Record};
use campfire_storage::{Blob, Json};

use crate::admin::{administrator, body, viewer};
use crate::dto::time;
use crate::error::{fail, validation};

pub fn routes() -> Router<Kit> {
    Router::new()
        .route("/api/v1/soundboard/sounds", get(action(index)))
        .route(
            "/api/v1/admin/soundboard/sounds",
            axum::routing::post(unparsed_action(create)),
        )
        .route(
            "/api/v1/admin/soundboard/sounds/{id}",
            patch(unparsed_action(update)).delete(action(destroy)),
        )
}

endpoint!(index => list_sounds);
endpoint!(create => create_sound);
endpoint!(update => update_sound);
endpoint!(destroy => delete_sound);

async fn list_sounds(c: &mut Ctx) -> Result {
    viewer(c).await?;
    reply_catalog(c, StatusCode::OK).await
}

async fn reply_catalog(c: &mut Ctx, status: StatusCode) -> Result {
    let storage = c.app().storage.clone();
    let sounds = c
        .app()
        .db
        .read(move |conn| {
            SoundboardSound::list(conn)?
                .into_iter()
                .map(|sound| {
                    let blob =
                        attachments::attached_blob(conn, "SoundboardSound", sound.id, "audio")?
                            .ok_or(campfire_db::Error::RecordNotFound("soundboard audio"))?;
                    Ok(api::SoundboardSound {
                        id: sound.id,
                        name: sound.name,
                        emoji: sound.emoji,
                        volume: sound.volume,
                        uploader_id: sound.uploader_id,
                        audio_url: campfire_storage::paths::blob_proxy_path(
                            &*storage.verifier,
                            &blob,
                            Some("inline"),
                        ),
                        created_at: time(sound.created_at),
                        updated_at: time(sound.updated_at),
                    })
                })
                .collect::<campfire_db::Result<Vec<_>>>()
        })
        .await
        .map_err(Error::internal)?;
    c.json(
        status,
        &api::SoundboardCatalog {
            sounds,
            capacity: CAPACITY,
        },
    )
}

async fn uploaded_audio(c: &mut Ctx, signed_id: &str) -> Result<Blob> {
    let id = campfire_storage::paths::verify_signed_blob_id(
        &*c.app().storage.verifier,
        signed_id,
        c.app().clock.now(),
    )
    .ok_or_else(|| fail(c, validation("audio", "isn't an uploaded file")))?;
    let blob = c
        .app()
        .db
        .read(move |conn| Blob::find(conn, id).map_err(attachments::storage_error))
        .await
        .map_err(Error::internal)?
        .ok_or_else(|| fail(c, validation("audio", "isn't an uploaded file")))?;
    if !(1..=MAX_AUDIO_BYTES).contains(&blob.byte_size) {
        return Err(fail(
            c,
            validation("audio", "must be between 1 byte and 512 KB"),
        ));
    }
    let storage = c.app().storage.clone();
    let prepared = tokio::task::spawn_blocking(move || {
        let path = storage.path_for(&blob);
        let size = std::fs::metadata(&path)?.len();
        if size != blob.byte_size as u64 {
            return Ok(None);
        }
        // Soundboard formats come only from stored bytes, even for preidentified uploads.
        let detected = campfire_storage::marcel::identify(&std::fs::read(&path)?, None, None);
        let detected_type = audio_type(&detected);
        if !(AudioFacts {
            content_type: detected_type.to_owned(),
            byte_size: blob.byte_size,
        })
        .errors()
        .is_empty()
        {
            return Ok(None);
        }
        let probe = campfire_storage::analyze::probe(&path)?;
        let content_type = match probe
            .get("format")
            .and_then(|format| format.get("format_name"))
            .and_then(Json::as_str)
        {
            Some("mp3") => "audio/mpeg",
            Some("ogg") => "audio/ogg",
            Some("wav") => "audio/wav",
            Some("matroska,webm") => "audio/webm",
            _ => return Ok(None),
        };
        let Some(Json::Array(streams)) = probe.get("streams") else {
            return Ok(None);
        };
        let has_stream = |kind| {
            streams
                .iter()
                .any(|stream| stream.get("codec_type").and_then(Json::as_str) == Some(kind))
        };
        if content_type != detected_type
            || !has_stream("audio")
            || (content_type == "audio/webm" && has_stream("video"))
        {
            return Ok(None);
        }
        let mut blob = blob;
        blob.content_type = Some(content_type.to_owned());
        blob.metadata.set("identified", Json::Bool(true));
        Ok::<_, campfire_storage::Error>(Some(blob))
    })
    .await
    .map_err(Error::internal)?
    .map_err(|_| fail(c, validation("audio", "couldn't be read as an audio file")))?;
    prepared.ok_or_else(|| {
        fail(
            c,
            validation(
                "audio",
                "must be a readable MP3, Ogg, WAV or audio-only WebM file",
            ),
        )
    })
}

// Normalize Marcel's MIME aliases before comparing them with ffprobe's container.
fn audio_type(content_type: &str) -> &str {
    match content_type {
        "audio/x-wav" | "audio/vnd.wav" => "audio/wav",
        "application/ogg" | "audio/opus" | "audio/vorbis" | "audio/speex" => "audio/ogg",
        "video/webm" => "audio/webm",
        content_type => content_type,
    }
}

fn audit(
    tx: &campfire_db::Tx<'_>,
    sound: &SoundboardSound,
    action: &str,
    context: &campfire_db::models::audit_log::Context,
) -> campfire_db::Result<()> {
    AuditLog::record(
        tx,
        NewAuditLog {
            action: action.into(),
            target: Some(Target {
                record_type: "SoundboardSound".into(),
                id: sound.id,
                label: Some(sound.name.clone()),
            }),
            ..Default::default()
        },
        context,
    )?;
    Ok(())
}

async fn create_sound(c: &mut Ctx) -> Result {
    let user = administrator(c).await?;
    let input: api::CreateSoundboardSound = body(c).await?;
    let blob = uploaded_audio(c, &input.signed_id).await?;
    let context = audit_context(c)?;
    c.app()
        .db
        .write(move |tx| {
            let facts = AudioFacts {
                content_type: blob.content_type().to_owned(),
                byte_size: blob.byte_size,
            };
            let sound = SoundboardSound::create(
                tx,
                user.id,
                SoundboardValues {
                    name: input.name,
                    emoji: input.emoji,
                    volume: input.volume.unwrap_or(1.0),
                },
                &facts,
            )?;
            // The shared attachment saver skips updates for blobs already marked identified.
            tx.conn().execute_cached(
                "UPDATE active_storage_blobs SET content_type=? WHERE id=?",
                rusqlite::params![blob.content_type, blob.id],
            )?;
            attachments::assign(
                tx,
                Record::soundboard_sound(sound.id),
                "audio",
                Assignment::Existing(blob),
            )?;
            audit(tx, &sound, "soundboard_sound.create", &context)
        })
        .await
        .map_err(Error::internal)?;
    reply_catalog(c, StatusCode::CREATED).await
}

fn record_id(c: &Ctx) -> Result<i64> {
    c.param_str("id")
        .and_then(|id| id.parse().ok())
        .ok_or(Error::NotFound)
}

async fn update_sound(c: &mut Ctx) -> Result {
    administrator(c).await?;
    let id = record_id(c)?;
    let input: api::UpdateSoundboardSound = body(c).await?;
    let context = audit_context(c)?;
    c.app()
        .db
        .write(move |tx| {
            let sound = SoundboardSound::find(tx.conn(), id)?.update(
                tx,
                SoundboardValues {
                    name: input.name,
                    emoji: input.emoji,
                    volume: input.volume,
                },
            )?;
            audit(tx, &sound, "soundboard_sound.update", &context)
        })
        .await
        .map_err(db_error)?;
    reply_catalog(c, StatusCode::OK).await
}

async fn delete_sound(c: &mut Ctx) -> Result {
    administrator(c).await?;
    let id = record_id(c)?;
    let context = audit_context(c)?;
    c.app()
        .db
        .write(move |tx| {
            let sound = SoundboardSound::find(tx.conn(), id)?;
            attachments::destroy(tx, Record::soundboard_sound(id), "audio")?;
            sound.destroy(tx)?;
            audit(tx, &sound, "soundboard_sound.destroy", &context)
        })
        .await
        .map_err(db_error)?;
    reply_catalog(c, StatusCode::OK).await
}
