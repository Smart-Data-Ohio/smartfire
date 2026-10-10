//! The Active Storage endpoints (`activestorage/config/routes.rb`) over `campfire_storage`, as the
//! engine's controllers serve them, plus `ActiveStorage::Blob#purge` for the purge job.
//!
//! Poll option media additionally requires membership in its room and uses private, uncached
//! responses, including stills and disk redirects. Other downloads stay public behind signed
//! URLs; the disk `PUT` and direct uploads require a
//! Campfire session (`reference/config/initializers/active_storage_authentication.rb`). The disk
//! service's `show` gets `Cache-Control: max-age=3600, public`
//! (`reference/config/initializers/active_storage.rb`). These controllers inherit from
//! `ActiveStorage::BaseController` (`protect_from_forgery with: :exception`), not
//! `ApplicationController`, so none of Campfire's concerns run.

use std::sync::{Arc, LazyLock};

use campfire_db::CachedStatements;
use campfire_kit::{Ctx, Error, ExpiresIn, Freshness, Response, Result, SendOptions, StatusCode, halt, http::header};
use campfire_storage::file_server::{self, BodyPart};
use campfire_storage::{Blob, Filename, Json, Staged, Storage, Variation, content_types, disk, paths};
use rusqlite::params;
use tokio::sync::Semaphore;

use crate::app::{App, AppCtx};
use crate::concerns::{find_session_by_cookie, head};

/// `ActiveStorage.service_urls_expire_in`
const SERVICE_URLS_EXPIRE_IN: i64 = 5 * 60;
/// `http_cache_forever`: `expires_in 100.years`.
const HUNDRED_YEARS: u64 = 3_155_695_200;
/// The most image and video jobs (variants, previews, analysis) that run at once.
const MAX_MEDIA_JOBS: usize = 4;
static MEDIA_PERMITS: LazyLock<Arc<Semaphore>> = LazyLock::new(|| Arc::new(Semaphore::new(media_capacity())));

// Codec calls may not observe cancellation. Keep branding from occupying attachment slots.
const MAX_BRANDING_JOBS: usize = 1;
static BRANDING_PERMITS: LazyLock<Arc<Semaphore>> =
    LazyLock::new(|| Arc::new(Semaphore::new(MAX_BRANDING_JOBS)));

fn media_capacity() -> usize {
    std::thread::available_parallelism()
        .map_or(2, |n| n.get())
        .clamp(1, MAX_MEDIA_JOBS)
}

#[cfg(feature = "test-support")]
pub fn media_permits() -> (usize, usize) {
    (MEDIA_PERMITS.available_permits(), media_capacity())
}

#[cfg(feature = "test-support")]
pub fn branding_permits() -> (usize, usize) {
    (BRANDING_PERMITS.available_permits(), MAX_BRANDING_JOBS)
}

#[cfg(feature = "test-support")]
pub mod test_hooks {
    use std::sync::Mutex;

    type Hook = Box<dyn FnOnce() + Send>;

    static BETWEEN_ANALYSIS_READS: Mutex<Vec<(i64, Hook)>> = Mutex::new(Vec::new());

    /// Run `hook` once on the reader, after analysis loads blob `blob_id` and before it decides
    /// whether the blob is branding.
    pub fn between_analysis_reads(blob_id: i64, hook: impl FnOnce() + Send + 'static) {
        BETWEEN_ANALYSIS_READS.lock().unwrap().push((blob_id, Box::new(hook)));
    }

    pub(super) fn reached_analysis_classification(blob_id: i64) {
        let hook = {
            let mut hooks = BETWEEN_ANALYSIS_READS.lock().unwrap();
            let found = hooks.iter().position(|(id, _)| *id == blob_id);
            found.map(|index| hooks.swap_remove(index).1)
        };

        if let Some(hook) = hook {
            hook();
        }
    }
}

// --- Blobs -----------------------------------------------------------------------------------------

/// `ActiveStorage::Blobs::RedirectController#show`
pub async fn blobs_redirect(c: &mut Ctx) -> Result {
    c.verify_authenticity_token()?;
    let blob = set_blob(c).await?;
    c.expires_in(SERVICE_URLS_EXPIRE_IN as u64, ExpiresIn::default());
    let disposition = c.param_str("disposition").map(str::to_string);
    let url = blob_url(c, &blob, disposition.as_deref());
    let restricted = matches!(blob.metadata.get("poll_media"), Some(Json::Bool(true)));
    let response = c.redirect_to_with(&url, campfire_kit::Redirect { allow_other_host: true, ..Default::default() })?;
    Ok(if restricted { response.header(header::CACHE_CONTROL, "private, no-store") } else { response })
}

/// `ActiveStorage::Blobs::ProxyController#show`
pub async fn blobs_proxy(c: &mut Ctx) -> Result {
    c.verify_authenticity_token()?;
    let blob = set_blob(c).await?;
    let disposition = c.param_str("disposition").map(str::to_string);
    if let Some(range) = c.request.header("range").filter(|r| !r.trim().is_empty()).map(str::to_string) {
        let response = send_blob_byte_range_data(c, &blob, &range)?;
        return Ok(if matches!(blob.metadata.get("poll_media"), Some(Json::Bool(true))) { response.header(header::CACHE_CONTROL, "private, no-store") } else { response });
    }
    let restricted = matches!(blob.metadata.get("poll_media"), Some(Json::Bool(true)));
    if !restricted && let Some(not_modified) = http_cache_forever(c) {
        return Ok(not_modified);
    }
    let response = send_blob_stream(c, &blob, disposition.as_deref())?.header(header::ACCEPT_RANGES, "bytes");
    Ok(if restricted { response.header(header::CACHE_CONTROL, "private, no-store") } else { response })
}

// --- Representations -------------------------------------------------------------------------------

/// `ActiveStorage::Representations::RedirectController#show`
pub async fn representations_redirect(c: &mut Ctx) -> Result {
    c.verify_authenticity_token()?;
    let blob = set_blob(c).await?;
    let restricted = matches!(blob.metadata.get("poll_media"), Some(Json::Bool(true)));
    let image = set_representation(c, blob).await?;
    c.expires_in(SERVICE_URLS_EXPIRE_IN as u64, ExpiresIn::default());
    let disposition = c.param_str("disposition").map(str::to_string);
    let url = blob_url(c, &image, disposition.as_deref());
    let response = c.redirect_to_with(&url, campfire_kit::Redirect { allow_other_host: true, ..Default::default() })?;
    Ok(if restricted { response.header(header::CACHE_CONTROL, "private, no-store") } else { response })
}

/// `ActiveStorage::Representations::ProxyController#show`
pub async fn representations_proxy(c: &mut Ctx) -> Result {
    c.verify_authenticity_token()?;
    let blob = set_blob(c).await?;
    let restricted = matches!(blob.metadata.get("poll_media"), Some(Json::Bool(true)));
    let image = set_representation(c, blob).await?;
    if !restricted && let Some(not_modified) = http_cache_forever(c) {
        return Ok(not_modified);
    }
    let disposition = c.param_str("disposition").map(str::to_string);
    let response = send_blob_stream(c, &image, disposition.as_deref())?;
    Ok(if restricted { response.header(header::CACHE_CONTROL, "private, no-store") } else { response })
}

/// `ActiveStorage::SetBlob#set_blob`: `Blob.find_signed!(params[:signed_blob_id] || params[:signed_id])`.
/// A bad signature is `head :not_found`; a valid one for a missing blob is `RecordNotFound`.
async fn set_blob(c: &mut Ctx) -> Result<Blob> {
    let signed_id = c.param_str("signed_blob_id").or_else(|| c.param_str("signed_id")).unwrap_or("").to_string();
    let storage = c.app().storage.clone();
    let Some(blob_id) = paths::verify_signed_blob_id(&*storage.verifier, &signed_id, c.now()) else {
        return halt(head(StatusCode::NOT_FOUND));
    };
    let mut blob = c.app()
        .db
        .read(move |conn| Blob::find(conn, blob_id).map_err(storage_error))
        .await
        .map_err(Error::internal)?
        .ok_or(Error::NotFound)?;
    if authorize_poll_media(c, blob_id).await? {
        blob.metadata.set("poll_media", Json::Bool(true));
    }
    Ok(blob)
}

/// Follow still/variant images back to their original, including redirected disk URLs.
async fn authorize_poll_media(c: &Ctx, blob_id: i64) -> Result<bool> {
    let (restricted, rooms) = c.app().db.read(move |conn| {
        let mut roots = conn.prepare_cached(
            "WITH RECURSIVE roots(id) AS (
                VALUES (?1)
                UNION SELECT v.blob_id FROM active_storage_attachments a
                    JOIN active_storage_variant_records v ON v.id=a.record_id
                    JOIN roots r ON r.id=a.blob_id
                    WHERE a.record_type='ActiveStorage::VariantRecord' AND a.name='image'
             ) SELECT b.id, json_extract(b.metadata, '$.poll_media') = 1 FROM roots r JOIN active_storage_blobs b ON b.id=r.id"
        )?;
        let blobs = roots.query_map([blob_id], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Option<bool>>(1)?.unwrap_or(false))))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let mut restricted = false;
        let mut rooms = Vec::new();
        for (id, marked) in blobs {
            restricted |= marked;
            let mut statement = conn.prepare_cached(
                "SELECT m.room_id FROM active_storage_attachments a
                 JOIN poll_options o ON o.id=a.record_id JOIN polls p ON p.id=o.poll_id
                 JOIN messages m ON m.id=p.message_id
                 WHERE a.blob_id=? AND a.record_type='PollOption' AND a.name='media'"
            )?;
            let found = statement.query_map([id], |r| r.get::<_, i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
            restricted |= !found.is_empty();
            rooms.extend(found);
            if marked {
                let mut statement = conn.prepare_cached("SELECT m.room_id FROM active_storage_attachments a JOIN messages m ON m.id=a.record_id WHERE a.blob_id=? AND a.record_type='Message'")?;
                rooms.extend(statement.query_map([id], |r| r.get::<_, i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?);
            }
        }
        Ok((restricted, rooms))
    }).await.map_err(Error::internal)?;
    if !restricted { return Ok(false); }
    let session = find_session_by_cookie(c).await?.ok_or(Error::NotFound)?;
    let user_id = session.user_id;
    let verified = session.two_factor_verified();
    let allowed = c.app().db.read(move |conn| {
        let user = campfire_db::User::find(conn, user_id)?;
        if !user.is_active() || (!verified && user.requires_two_factor()) { return Ok(false); }
        for room in rooms {
            if conn.query_row("SELECT EXISTS(SELECT 1 FROM memberships WHERE room_id=? AND user_id=?)", params![room, user_id], |r| r.get::<_, bool>(0))? {
                return Ok(true);
            }
        }
        Ok(false)
    }).await.map_err(Error::internal)?;
    if !allowed { return Err(Error::NotFound); }
    Ok(true)
}

/// `set_representation`: `@blob.representation(params[:variation_key]).processed`. A bad
/// variation key is `head :not_found`. Returns the blob that represents it (the variant's or
/// preview's image).
async fn set_representation(c: &mut Ctx, blob: Blob) -> Result<Blob> {
    let storage = c.app().storage.clone();
    let key = c.param_str("variation_key").unwrap_or("").to_string();
    let variation = match Variation::decode(&*storage.verifier, &key, c.now()) {
        Ok(variation) => variation,
        Err(campfire_storage::Error::InvalidSignature) => return halt(head(StatusCode::NOT_FOUND)),
        Err(error) => return Err(Error::internal(error)),
    };
    processed_representation(c.app(), blob, variation).await
}

/// `blob.representation(variation).processed`, reusing an existing variant or preview.
pub async fn processed_representation(app: &App, blob: Blob, variation: Variation) -> Result<Blob> {
    if blob.is_previewable() {
        processed_preview(app, blob, variation).await
    } else if blob.is_variable() {
        let variation = app.storage.variation_for(&blob, &variation).map_err(Error::internal)?;
        processed_variant(app, blob, variation).await
    } else {
        Err(Error::internal(campfire_storage::Error::Unrepresentable(blob.content_type().to_string())))
    }
}

/// `blob.preview(transformations).processed`: the preview image itself for empty
/// transformations, otherwise its processed variant.
pub async fn processed_preview(app: &App, blob: Blob, transformations: Variation) -> Result<Blob> {
    let image = preview_image(app, blob).await?;
    if transformations.is_empty() {
        return Ok(image);
    }
    let variation = app.storage.variation_for(&image, &transformations).map_err(Error::internal)?;
    processed_variant(app, image, variation).await
}

/// `VariantWithRecord#processed` for an already-defaulted variation: the existing variant, or
/// one transformed off the writer and then recorded.
async fn processed_variant(app: &App, blob: Blob, variation: Variation) -> Result<Blob> {
    processed_variant_with(app, blob, variation, |storage, blob, variation| storage.transform_variant(blob, variation)).await
}

pub async fn processed_variant_with(
    app: &App,
    blob: Blob,
    variation: Variation,
    transform: impl FnOnce(&Storage, &Blob, &Variation) -> campfire_storage::Result<Staged>
    + Send
    + 'static,
) -> Result<Blob> {
    processed_variant_using(
        app,
        blob,
        variation,
        true,
        |storage, blob, variation| async move {
            process_media(move || transform(&storage, &blob, &variation)).await
        },
    )
    .await
}

pub async fn processed_branding_variant_with_deadline(
    app: &App,
    blob: Blob,
    variation: Variation,
    timeout: std::time::Duration,
    transform: impl FnOnce(
        &Storage,
        &Blob,
        &Variation,
        &campfire_storage::vips::Cancellation,
    ) -> campfire_storage::Result<Staged>
    + Send
    + 'static,
) -> Result<Blob> {
    processed_variant_using(
        app,
        blob,
        variation,
        false,
        |storage, blob, variation| async move {
            process_branding_with_deadline(timeout, move |cancel| {
                transform(&storage, &blob, &variation, &cancel)
            })
            .await
        },
    )
    .await
}

async fn processed_variant_using<F: std::future::Future<Output = Result<Staged>>>(
    app: &App,
    blob: Blob,
    variation: Variation,
    defer_analysis: bool,
    work: impl FnOnce(Arc<Storage>, Blob, Variation) -> F,
) -> Result<Blob> {
    let marker = rails_compat::blob_branding::Marker::new(&app.secrets);
    let storage = app.storage.clone();
    let (source, digested) = (blob.clone(), variation.clone());
    // Rails' processed? checks the record, not the file. The serving endpoints
    // handle a missing final file; a valid derivative needs no intermediate file.
    let (existing, branding) = app
        .db
        .read(move |conn| {
            let existing = storage
                .existing_variant(conn, &source, &digested)
                .map_err(storage_error)?;
            let branding = !defer_analysis || branding_blob(conn, marker, &source)?;
            Ok((existing, branding))
        })
        .await
        .map_err(Error::internal)?;
    if let Some(mut image) = existing {
        if branding && !verified_branding_mark(marker, &image) {
            image = app
                .db
                .write(move |tx| {
                    let mut image = Blob::find(tx.conn(), image.id)
                        .map_err(storage_db_error)?
                        .ok_or(campfire_db::Error::RecordNotFound("ActiveStorage::Blob"))?;
                    mark_branding_blob(tx.conn(), marker, &mut image)?;
                    Ok(image)
                })
                .await
                .map_err(Error::internal)?;
        }
        return Ok(image);
    }

    let storage = app.storage.clone();
    let (source, digested) = (blob.clone(), variation.clone());
    let image = work(storage, source, digested).await?;
    // Branding transforms supply dimensions on their bounded path; retain them and skip
    // the attachment analyzer's after-commit callback.
    let image = if defer_analysis {
        image.defer_analysis()
    } else {
        image
    };

    let storage = app.storage.clone();
    app.db
        .write(move |tx| {
            let conn = tx.conn();
            let source = Blob::find(conn, blob.id)
                .map_err(storage_db_error)?
                .ok_or(campfire_db::Error::RecordNotFound("ActiveStorage::Blob"))?;
            let branding = !defer_analysis || branding_blob(conn, marker, &source)?;
            match storage.record_variant(conn, &blob, &variation, &image, tx.now().jiff()).map_err(storage_error)? {
                Some(mut recorded) => {
                    if branding {
                        mark_branding_blob(conn, marker, &mut recorded)?;
                    }
                    if defer_analysis {
                        crate::controllers::presenters::attachments::enqueue_analysis(tx, &recorded);
                    }
                    keep_after_commit(tx, image);
                    Ok(recorded)
                }
                // Another request recorded it first; ours is dropped (and its file deleted).
                None => {
                    let mut image = storage
                        .existing_variant(conn, &blob, &variation)
                        .map_err(storage_error)?
                        .ok_or(campfire_db::Error::RecordNotFound(
                            "ActiveStorage::VariantRecord",
                        ))?;
                    if branding {
                        mark_branding_blob(conn, marker, &mut image)?;
                    }
                    Ok(image)
                }
            }
        })
        .await
        .map_err(Error::internal)
}

/// `blob.preview_image`, drawing it with ffmpeg off the writer when it's missing.
async fn preview_image(app: &App, blob: Blob) -> Result<Blob> {
    let storage = app.storage.clone();
    let source = blob.clone();
    let existing = app.db.read(move |conn| storage.existing_preview_image(conn, &source).map_err(storage_error)).await;
    if let Some(image) = existing.map_err(Error::internal)? {
        return Ok(image);
    }

    let storage = app.storage.clone();
    let source = blob.clone();
    let image = process_media(move || storage.draw_preview_image(&source)).await?.defer_analysis();

    let storage = app.storage.clone();
    app.db
        .write(move |tx| {
            let conn = tx.conn();
            match storage.record_preview_image(conn, &blob, &image, tx.now().jiff()).map_err(storage_error)? {
                Some(recorded) => {
                    crate::controllers::presenters::attachments::enqueue_analysis(tx, &recorded);
                    keep_after_commit(tx, image);
                    Ok(recorded)
                }
                None => storage.existing_preview_image(conn, &blob).map_err(storage_error)?.ok_or(campfire_db::Error::RecordNotFound("ActiveStorage::Blob")),
            }
        })
        .await
        .map_err(Error::internal)
}

/// Analyze an attached blob off the writer, then update metadata and touch its records.
/// Re-delivery after success (or deletion) has no side effects.
pub async fn analyze(app: &App, blob_id: i64) -> anyhow::Result<Option<Blob>> {
    analyze_with(app, blob_id, true).await
}

/// Message#process_attachment calls Blob#analyze explicitly, including its save/touch
/// callbacks on an already analyzed blob. Durable job retries keep their no-op boundary.
pub(crate) async fn analyze_explicit(app: &App, blob_id: i64) -> anyhow::Result<Option<Blob>> {
    analyze_with(app, blob_id, false).await
}

async fn analyze_with(
    app: &App,
    blob_id: i64,
    skip_analyzed: bool,
) -> anyhow::Result<Option<Blob>> {
    let marker = rails_compat::blob_branding::Marker::new(&app.secrets);
    let classified = app
        .db
        .read(move |conn| {
            let blob = Blob::find(conn, blob_id).map_err(storage_db_error)?;
            #[cfg(feature = "test-support")]
            test_hooks::reached_analysis_classification(blob_id);
            Ok(match blob {
                Some(blob) if skip_analyzed && blob.is_analyzed() => Some((blob, false)),
                // A row purged since `find` stops analysis, as Rails discards a purged blob's job:
                // its file may not be deleted yet, so nothing may open it.
                Some(blob) => {
                    branding_classification(conn, marker, &blob)?.map(|branding| (blob, branding))
                }
                None => None,
            })
        })
        .await?;
    let Some((blob, branding)) = classified else { return Ok(None) };
    if skip_analyzed && blob.is_analyzed() {
        return Ok(Some(blob));
    }
    let storage = app.storage.clone();
    let mut source = blob.clone();
    source.metadata = Json::object();
    let metadata = if branding {
        let timeout = campfire_storage::branding::processing_timeout(&source);
        process_branding_work_with_deadline(timeout, move |cancel| {
            campfire_storage::branding::analyzed_metadata(&storage, &source, &cancel)
        })
        .await?
    } else {
        process_blocking_work(MEDIA_PERMITS.clone(), move || {
            storage.analyzed_metadata(&source)
        })
        .await?
    };
    app.db
        .write(move |tx| {
            // Another delivery or synchronous message processing may have finished meanwhile.
            let Some(mut blob) = Blob::find(tx.conn(), blob_id).map_err(storage_db_error)? else {
                return Ok(None);
            };
            if skip_analyzed && blob.is_analyzed() {
                return Ok(Some(blob));
            }
            blob.metadata.merge(&metadata);
            blob.update_metadata(tx.conn(), blob.metadata.clone())
                .map_err(storage_db_error)?;
            touch_attachment_records(tx, blob.id)?;
            Ok(Some(blob))
        })
        .await
        .map_err(Into::into)
}

fn branding_blob(
    conn: &campfire_db::Connection,
    marker: rails_compat::blob_branding::Marker,
    blob: &Blob,
) -> campfire_db::Result<bool> {
    Ok(branding_classification(conn, marker, blob)?.unwrap_or(false))
}

/// Whether `blob` is workspace branding, or `None` once its row is gone (purged).
fn branding_classification(
    conn: &campfire_db::Connection,
    marker: rails_compat::blob_branding::Marker,
    blob: &Blob,
) -> campfire_db::Result<Option<bool>> {
    // One statement reads one snapshot. A replacement or removal marks the tree and deletes the
    // Account association in one write transaction, so this sees the association or the mark,
    // never neither; `blob` may have been loaded before that commit and lack the mark.
    // Untagged legacy variants and previews still belong to the Account through their source
    // tree: a variant's image climbs to its variant record's blob, a preview to the blob it
    // previews.
    let current = conn.query_row_cached(
        "WITH RECURSIVE sources(id) AS (
            VALUES (?1)
            UNION
            SELECT variants.blob_id FROM active_storage_attachments images
            JOIN active_storage_variant_records variants ON variants.id = images.record_id
            JOIN sources ON sources.id = images.blob_id
            WHERE images.record_type = 'ActiveStorage::VariantRecord' AND images.name = 'image'
            UNION
            SELECT previews.record_id FROM active_storage_attachments previews
            JOIN sources ON sources.id = previews.blob_id
            WHERE previews.record_type = 'ActiveStorage::Blob' AND previews.name = 'preview_image'
         ) SELECT blobs.metadata, EXISTS (
            SELECT 1 FROM active_storage_attachments attachments JOIN sources ON sources.id = attachments.blob_id
            WHERE attachments.record_type = 'Account' AND attachments.name IN ('logo', 'banner')
         ) FROM active_storage_blobs blobs WHERE blobs.id = ?1",
        [blob.id],
        |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, bool>(1)?)),
    );
    let (metadata, attached) = match current {
        Ok(current) => current,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let metadata = metadata.and_then(|text| Json::parse(&text).ok());
    Ok(Some(
        attached || metadata.is_some_and(|metadata| mark_verifies(marker, &blob.key, &metadata)),
    ))
}

fn verified_branding_mark(marker: rails_compat::blob_branding::Marker, blob: &Blob) -> bool {
    mark_verifies(marker, &blob.key, &blob.metadata)
}

fn mark_verifies(marker: rails_compat::blob_branding::Marker, key: &str, metadata: &Json) -> bool {
    metadata
        .get(campfire_storage::branding::MARK_KEY)
        .and_then(Json::as_str)
        .is_some_and(|mark| marker.verifies(key, mark))
}

pub fn mark_branding_blob(
    conn: &campfire_db::Connection,
    marker: rails_compat::blob_branding::Marker,
    blob: &mut Blob,
) -> campfire_db::Result<()> {
    blob.metadata.set(
        campfire_storage::branding::MARK_KEY,
        Json::String(marker.sign(&blob.key)),
    );
    blob.update_metadata(conn, blob.metadata.clone())
        .map_err(storage_db_error)
}

/// Persist provenance before removing any associations that queued analysis may rely on.
pub fn mark_branding_tree(
    conn: &campfire_db::Connection,
    marker: rails_compat::blob_branding::Marker,
    blob_id: i64,
) -> campfire_db::Result<()> {
    let ids = query_ids(conn,
        "WITH RECURSIVE images(id) AS (
            VALUES (?1)
            UNION
            SELECT attachments.blob_id FROM active_storage_attachments attachments
            JOIN active_storage_variant_records variants ON variants.id = attachments.record_id
            JOIN images ON images.id = variants.blob_id
            WHERE attachments.record_type = 'ActiveStorage::VariantRecord' AND attachments.name = 'image'
            UNION
            SELECT attachments.blob_id FROM active_storage_attachments attachments
            JOIN images ON images.id = attachments.record_id
            WHERE attachments.record_type = 'ActiveStorage::Blob' AND attachments.name = 'preview_image'
         ) SELECT id FROM images", blob_id)?;
    for id in ids {
        if let Some(mut blob) = Blob::find(conn, id).map_err(storage_db_error)? {
            mark_branding_blob(conn, marker, &mut blob)?;
        }
    }
    Ok(())
}

/// Active Storage's blob-save callback touches every attached record; messages touch rooms too.
pub(crate) fn touch_attachment_records(
    tx: &mut campfire_db::Tx<'_>,
    blob_id: i64,
) -> campfire_db::Result<()> {
    for (record_type, record_id) in
        campfire_storage::blob::attachment_records(tx.conn(), blob_id).map_err(storage_db_error)?
    {
        match record_type.as_str() {
            "Message" => campfire_db::Message::find(tx.conn(), record_id)?.touch(tx)?,
            "User" => crate::controllers::presenters::accounts::touch(
                tx.conn(),
                "users",
                record_id,
                tx.now(),
            )?,
            "Account" | "WorkspaceIcon" => {
                let table = match record_type.as_str() {
                    "Account" => "accounts",
                    _ => "workspace_icons",
                };
                tx.conn().execute_cached(
                    &format!("UPDATE {table} SET updated_at = ?1 WHERE id = ?2"),
                    rusqlite::params![tx.now().to_string(), record_id],
                )?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn storage_db_error(error: campfire_storage::Error) -> campfire_db::Error {
    campfire_db::Error::Other(error.to_string())
}

/// Uploads a file to storage for a blob whose row the caller saves next (see [`keep_after_commit`]).
pub async fn stage_file(app: &App, path: std::path::PathBuf, filename: Filename, content_type: Option<String>) -> Result<Staged> {
    let storage = app.storage.clone();
    tokio::task::spawn_blocking(move || storage.stage_file(&path, filename, content_type.as_deref()))
        .await
        .map_err(Error::internal)?
        .map_err(Error::internal)
}

/// Keeps a staged file once the write saving its row commits; a rollback drops it instead,
/// which deletes the file. Ownership is finalized before fallible model callbacks.
pub fn keep_after_commit(tx: &mut campfire_db::Tx<'_>, staged: Staged) {
    tx.on_commit_success(move || { staged.keep(); });
}

/// Runs libvips, ffmpeg or ffprobe work on the blocking pool, a few jobs at a time: each can take
/// a lot of memory and CPU (libvips threads its own work), and uploads shouldn't queue behind
/// more of them than the machine can run at once.
pub async fn process_media<T: Send + 'static>(
    work: impl FnOnce() -> campfire_storage::Result<T> + Send + 'static,
) -> Result<T> {
    process_blocking_work(MEDIA_PERMITS.clone(), work)
        .await
        .map_err(Error::internal)
}

/// The branding deadline includes waiting for its own slot. Cancellation is cooperative;
/// timed out or disconnected work retains that slot until the blocking task actually ends.
pub async fn process_branding_with_deadline<T: Send + 'static>(
    timeout: std::time::Duration,
    work: impl FnOnce(campfire_storage::vips::Cancellation) -> campfire_storage::Result<T>
    + Send
    + 'static,
) -> Result<T> {
    process_branding_work_with_deadline(timeout, work)
        .await
        .map_err(Error::internal)
}

async fn process_branding_work_with_deadline<T: Send + 'static>(
    timeout: std::time::Duration,
    work: impl FnOnce(campfire_storage::vips::Cancellation) -> campfire_storage::Result<T>
    + Send
    + 'static,
) -> anyhow::Result<T> {
    struct CancelOnDrop(campfire_storage::vips::Cancellation);
    impl Drop for CancelOnDrop {
        fn drop(&mut self) {
            self.0.cancel();
        }
    }
    let cancel = campfire_storage::vips::Cancellation::new(timeout);
    let _guard = CancelOnDrop(cancel.clone());
    tokio::time::timeout(
        timeout,
        process_blocking_work(BRANDING_PERMITS.clone(), move || {
            cancel.check()?;
            let result = work(cancel.clone());
            // A failed still can degrade to static, but cancellation must reject the upload.
            cancel.check()?;
            result
        }),
    )
    .await?
}

async fn process_blocking_work<T: Send + 'static>(
    permits: Arc<Semaphore>,
    work: impl FnOnce() -> campfire_storage::Result<T> + Send + 'static,
) -> anyhow::Result<T> {
    // The permit goes with the work: a request that gives up (a timeout, a closed connection)
    // doesn't stop the blocking task, so it mustn't free the slot either.
    let permit = permits.acquire_owned().await?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        work()
    })
    .await?
    .map_err(Into::into)
}

/// `blob.url(disposition:)` on the disk service: a signed `/rails/active_storage/disk/...` URL
/// on this request's host that expires in `service_urls_expire_in`.
fn blob_url(c: &Ctx, blob: &Blob, disposition: Option<&str>) -> String {
    let storage = &c.app().storage;
    let content_type = content_types::for_serving(blob.content_type());
    let disposition = content_types::forced_disposition(blob.content_type()).or(disposition).unwrap_or("inline");
    let expires_at = c.now() + jiff::SignedDuration::from_secs(SERVICE_URLS_EXPIRE_IN);
    let path = storage.service.url_path(
        &*storage.verifier,
        &blob.key,
        Some(expires_at),
        &blob.filename,
        Some(content_type),
        disposition,
    );
    c.url_for(&path)
}

/// `http_cache_forever(public: true)`: cache for 100 years, ETag on the full path, and a fixed
/// Last-Modified. `Some(304)` when the client's copy is fresh.
fn http_cache_forever(c: &mut Ctx) -> Option<Response> {
    c.expires_in(HUNDRED_YEARS, ExpiresIn { public: true, immutable: true, ..ExpiresIn::default() });
    let last_modified: jiff::Timestamp = "2011-01-01T00:00:00Z".parse().expect("valid timestamp");
    c.fresh_when(Freshness {
        etag: Some(c.request.fullpath()),
        last_modified: Some(last_modified),
        public: true,
        ..Freshness::default()
    })
}

/// `send_blob_stream(blob, disposition:)`: the whole file, inline unless the type is forced to
/// download.
fn send_blob_stream(c: &mut Ctx, blob: &Blob, disposition: Option<&str>) -> Result {
    let storage = c.app().storage.clone();
    let path = storage.path_for(blob);
    let disposition = content_types::forced_disposition(blob.content_type()).or(disposition).unwrap_or("inline");
    let mut response = if !path.is_file() {
        // `rescue ActiveStorage::FileNotFoundError`: expires_now, head :not_found.
        // send_stream sets image headers before download raises in Rails.
        c.expires_now();
        c.send_data(bytes::Bytes::new(), SendOptions {
            filename: Some(blob.filename.sanitized()),
            content_type: Some(content_types::for_serving(blob.content_type()).to_string()),
            disposition: Some(disposition.to_string()),
            status: StatusCode::NOT_FOUND,
            ..SendOptions::default()
        })
    } else {
        c.send_file(
            &path,
            SendOptions {
                filename: Some(blob.filename.sanitized()),
                content_type: Some(content_types::for_serving(blob.content_type()).to_string()),
                disposition: Some(disposition.to_string()),
                ..SendOptions::default()
            },
        )?
    };
    // Rails send_stream omits the send_file/send_data transfer encoding, on
    // success and on its handled FileNotFoundError. Range responses use send_data.
    response.headers.remove("content-transfer-encoding");
    Ok(response)
}

/// `send_blob_byte_range_data(blob, range_header)`
fn send_blob_byte_range_data(c: &mut Ctx, blob: &Blob, range: &str) -> Result {
    let storage = c.app().storage.clone();
    let size = blob.byte_size.max(0) as u64;
    let ranges = match file_server::byte_ranges(Some(range), size) {
        Some(ranges) if !ranges.is_empty() => ranges,
        _ => return Ok(c.head(StatusCode::RANGE_NOT_SATISFIABLE)),
    };
    let path = storage.path_for(blob);
    if !path.is_file() {
        return Err(storage_error_to_kit(campfire_storage::Error::FileNotFound));
    }
    let content_type_for_serving = content_types::for_serving(blob.content_type()).to_string();
    let (content_type, parts, content_range) = if let [(start, end)] = ranges[..] {
        (content_type_for_serving, vec![BodyPart::File { path, start, end }], Some(format!("bytes {start}-{end}/{size}")))
    } else {
        let boundary = random_hex(16);
        let mut parts = Vec::new();
        for &(start, end) in &ranges {
            let heading = format!("\r\n--{boundary}\r\nContent-Type: {content_type_for_serving}\r\nContent-Range: bytes {start}-{end}/{size}\r\n\r\n");
            parts.push(BodyPart::Bytes(heading.into_bytes()));
            parts.push(BodyPart::File { path: path.clone(), start, end });
        }
        parts.push(BodyPart::Bytes(format!("\r\n--{boundary}--\r\n").into_bytes()));
        (format!("multipart/byteranges; boundary={boundary}"), parts, None)
    };
    let disposition = content_types::forced_disposition(blob.content_type()).unwrap_or("inline");
    let mut response = c.send_data(
        bytes::Bytes::new(),
        SendOptions {
            filename: Some(blob.filename.sanitized()),
            content_type: Some(content_type),
            disposition: Some(disposition.to_string()),
            status: StatusCode::PARTIAL_CONTENT,
            ..SendOptions::default()
        },
    );
    let length = parts_len(&parts);
    response.body = parts_body(parts);
    if matches!(response.body, campfire_kit::Body::Stream(_)) {
        response = response.header(header::CONTENT_LENGTH, &length.to_string());
    }
    if let Some(content_range) = content_range {
        response = response.header(header::CONTENT_RANGE, &content_range);
    }
    Ok(response.header(header::ACCEPT_RANGES, "bytes"))
}

/// The body for byte ranges of files and the bytes between them: a single range is sent as a
/// file body and several are streamed, so neither is read into memory up front.
fn parts_body(parts: Vec<BodyPart>) -> campfire_kit::Body {
    match <[BodyPart; 1]>::try_from(parts) {
        Ok([BodyPart::File { path, start, end }]) => {
            campfire_kit::Body::File(campfire_kit::response::FileBody { path, offset: start, len: end - start + 1 })
        }
        Ok([BodyPart::Bytes(bytes)]) => campfire_kit::Body::Bytes(bytes.into()),
        Err(parts) if parts.is_empty() => campfire_kit::Body::Empty,
        Err(parts) => campfire_kit::Body::Stream(axum::body::Body::from_stream(stream_parts(parts))),
    }
}

fn parts_len(parts: &[BodyPart]) -> u64 {
    parts
        .iter()
        .map(|part| match part {
            BodyPart::Bytes(bytes) => bytes.len() as u64,
            BodyPart::File { start, end, .. } => end - start + 1,
        })
        .sum()
}

/// Reads each part in turn, a chunk at a time.
fn stream_parts(parts: Vec<BodyPart>) -> impl futures_util::Stream<Item = std::io::Result<bytes::Bytes>> + Send + 'static {
    use tokio::io::{AsyncReadExt, AsyncSeekExt};
    const CHUNK: usize = 64 * 1024;
    let state = (parts.into_iter(), None::<tokio::io::Take<tokio::fs::File>>);
    futures_util::stream::try_unfold(state, |(mut parts, mut reading)| async move {
        loop {
            if let Some(reader) = reading.as_mut() {
                let mut chunk = vec![0; CHUNK];
                let read = reader.read(&mut chunk).await?;
                if read > 0 {
                    chunk.truncate(read);
                    return Ok(Some((bytes::Bytes::from(chunk), (parts, reading))));
                }
            }
            match parts.next() {
                None => return Ok(None),
                Some(BodyPart::Bytes(bytes)) => return Ok(Some((bytes::Bytes::from(bytes), (parts, None)))),
                Some(BodyPart::File { path, start, end }) => {
                    let mut file = tokio::fs::File::open(&path).await?;
                    file.seek(std::io::SeekFrom::Start(start)).await?;
                    reading = Some(file.take(end - start + 1));
                }
            }
        }
    })
}

// --- Disk service ----------------------------------------------------------------------------------

/// `ActiveStorage::DiskController#show`, plus the initializer's `after_action` cache header.
pub async fn disk_show(c: &mut Ctx) -> Result {
    let encoded = c.param_str("encoded_key").unwrap_or("");
    let Some(key) = disk::decode_verified_key(&*c.app().storage.verifier, encoded, c.now()) else { return Ok(c.head(StatusCode::NOT_FOUND)); };
    let blob_id = c.app().db.read(move |conn| {
        use rusqlite::OptionalExtension;
        Ok(conn.query_row("SELECT id FROM active_storage_blobs WHERE key=?", [key.key], |r| r.get::<_, i64>(0)).optional()?)
    }).await.map_err(Error::internal)?;
    let restricted = match blob_id {
        Some(id) => authorize_poll_media(c, id).await?,
        None => false,
    };
    let response = disk_serve(c)?;
    Ok(response.header(header::CACHE_CONTROL, if restricted { "private, no-store" } else { "max-age=3600, public" }))
}

fn disk_serve(c: &mut Ctx) -> Result {
    let storage = c.app().storage.clone();
    let encoded_key = c.param_str("encoded_key").unwrap_or("").to_string();
    let Some(key) = disk::decode_verified_key(&*storage.verifier, &encoded_key, c.now()) else {
        return Ok(c.head(StatusCode::NOT_FOUND));
    };
    let request = file_server::Request {
        method: c.request.method.as_str(),
        range: c.request.header("range"),
        if_modified_since: c.request.header("if-modified-since"),
    };
    let served = match file_server::serve_file(&request, &storage.service.path_for(&key.key), key.content_type.as_deref(), Some(&key.disposition)) {
        Ok(served) => served,
        Err(campfire_storage::Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(c.head(StatusCode::NOT_FOUND));
        }
        Err(error) => return Err(Error::internal(error)),
    };
    let mut response = Response::new(StatusCode::from_u16(served.status).map_err(Error::internal)?);
    for (name, value) in &served.headers {
        response = response.header(name.as_str(), value);
    }
    // `served.headers` carries the Content-Length of every part together.
    response.body = parts_body(served.body);
    Ok(response)
}

/// `ActiveStorage::DiskController#update` (the direct-upload PUT), behind
/// `require_active_storage_authentication`.
pub async fn disk_update(c: &mut Ctx) -> Result {
    require_active_storage_authentication(c).await?;
    let storage = c.app().storage.clone();
    let encoded_token = c.param_str("encoded_token").unwrap_or("").to_string();
    let Some(token) = disk::decode_verified_token(&*storage.verifier, &encoded_token, c.now())
    else {
        return Ok(c.head(StatusCode::NOT_FOUND));
    };
    let Ok(limit) = usize::try_from(token.content_length) else {
        return Ok(c.head(StatusCode::UNPROCESSABLE_ENTITY));
    };
    c.spool_body(limit).await?;
    c.parse_spooled_params().await?;
    let mut body = c.take_body_file().await.map_err(Error::internal)?;
    if !acceptable_content(c, &token, body.metadata().map_err(Error::internal)?.len()) {
        return Ok(c.head(StatusCode::UNPROCESSABLE_ENTITY));
    }
    // Parameter parsing may consume form bytes. Store the entire verified upload.
    std::io::Seek::rewind(&mut body).map_err(Error::internal)?;
    let (key, checksum) = (token.key.clone(), token.checksum.clone());
    let uploaded =
        tokio::task::spawn_blocking(move || storage.service.upload(&key, body, Some(&checksum)))
            .await
            .map_err(Error::internal)?;
    match uploaded {
        Ok(()) => Ok(c.head(StatusCode::NO_CONTENT)),
        Err(campfire_storage::Error::Integrity) => Ok(c.head(StatusCode::UNPROCESSABLE_ENTITY)),
        Err(error) => Err(Error::internal(error)),
    }
}

/// MIME type, received length, and any declared HTTP length must match the signed declaration.
fn acceptable_content(c: &Ctx, token: &disk::DiskToken, body_length: u64) -> bool {
    let media_type = c.request.media_type();
    let content_length = if c.request.header("transfer-encoding").is_some() {
        i64::try_from(body_length).ok()
    } else {
        c.request
            .header("content-length")
            .map_or_else(|| i64::try_from(body_length).ok(), |length| length.trim().parse::<i64>().ok())
    };
    token.content_type.as_deref().map(str::to_ascii_lowercase)
        == media_type.map(|m| m.to_ascii_lowercase())
        && Some(token.content_length) == content_length
        && u64::try_from(token.content_length).ok() == Some(body_length)
}

/// `ActiveStorage::DirectUploadsController#create`, behind CSRF and
/// `require_active_storage_authentication`.
pub async fn direct_uploads_create(c: &mut Ctx) -> Result {
    c.verify_authenticity_token()?;
    require_active_storage_authentication(c).await?;
    // `params.expect(blob: [:filename, :byte_size, :checksum, :content_type, metadata: {}])`
    let blob_params = c.params.require("blob")?.as_hash().cloned().ok_or_else(|| Error::ParameterMissing("blob".into()))?;
    // Strings, and numbers as their text: Active Storage's JavaScript sends `byte_size` as a number.
    let text = |key: &str| {
        blob_params.get(key).and_then(|p| match p {
            campfire_kit::Param::Str(s) => Some(s.clone()),
            campfire_kit::Param::Number(n) => Some(n.to_string()),
            _ => None,
        })
    };
    let (Some(filename), Some(checksum)) = (text("filename").filter(|f| !f.is_empty()), text("checksum").filter(|c| !c.is_empty())) else {
        return Err(Error::Status(StatusCode::UNPROCESSABLE_ENTITY));
    };
    let Some(byte_size) = text("byte_size").and_then(|s| crate::concerns::cast_integer(&s)) else {
        return Err(Error::Status(StatusCode::UNPROCESSABLE_ENTITY));
    };
    let content_type = text("content_type");
    let metadata = match blob_params.get("metadata").and_then(|m| m.as_hash()) {
        Some(metadata) => Json::parse(&metadata.to_json().to_string()).map_err(Error::internal)?,
        None => Json::object(),
    };

    let upload = create_direct_upload(c, filename, byte_size, checksum, content_type.clone(), metadata).await?;
    let url = c.url_for(&upload.path);
    let json = direct_upload_json(&upload.blob, &upload.signed_id, &url, content_type.as_deref());
    Ok(c.render_as(StatusCode::OK, campfire_kit::response::JSON_UTF8, json))
}

/// A blob [`create_direct_upload`] made, waiting for its bytes.
pub struct DirectUpload {
    pub blob: Blob,
    pub signed_id: String,
    /// Where to `PUT` the bytes, for `SERVICE_URLS_EXPIRE_IN`.
    pub path: String,
}

/// `ActiveStorage::Blob.create_before_direct_upload!` and the blob's direct upload URL.
pub async fn create_direct_upload(
    c: &mut Ctx,
    filename: String,
    byte_size: i64,
    checksum: String,
    content_type: Option<String>,
    mut metadata: Json,
) -> Result<DirectUpload> {
    let uploader_id = match crate::concerns::current_user(c) {
        Some(user) => user.id,
        None => find_session_by_cookie(c).await?
            .ok_or(Error::Status(StatusCode::UNAUTHORIZED))?.user_id,
    };
    let limit = upload_limit_bytes(c.app()).await?;
    if byte_size > limit {
        return halt(upload_limit_response(c, limit)?);
    }
    if let Json::Object(entries) = &mut metadata {
        entries.retain(|(key, _)| !key.starts_with("branding") && !key.starts_with("emoji_") && !key.starts_with("poll_"));
    }
    metadata.set("uploader_id", Json::Int(uploader_id));
    let storage = c.app().storage.clone();
    let now = c.now();
    let new_blob = campfire_storage::NewBlob {
        key: campfire_storage::key::generate_key(),
        filename: Filename::new(filename),
        content_type: content_type.clone(),
        metadata,
        service_name: storage.service.name().to_string(),
        byte_size,
        checksum: checksum.clone(),
    };
    let blob = c
        .app()
        .db
        .write(move |tx| new_blob.insert(tx.conn(), now).map_err(storage_error))
        .await
        .map_err(Error::internal)?;

    let expires_at = now + jiff::SignedDuration::from_secs(SERVICE_URLS_EXPIRE_IN);
    let path = storage.service.url_path_for_direct_upload(
        &*storage.verifier,
        &blob.key,
        expires_at,
        content_type.as_deref(),
        byte_size,
        &checksum,
    );
    let signed_id = paths::signed_blob_id(&*storage.verifier, blob.id, None);
    Ok(DirectUpload { blob, signed_id, path })
}

pub async fn upload_limit_bytes(app: &App) -> Result<i64> {
    app
        .db
        .read(|conn| {
            Ok(campfire_db::Account::first(conn)?.map_or(
                campfire_db::models::account::DEFAULT_UPLOAD_LIMIT_BYTES,
                |account| account.settings().upload_limit_bytes(),
            ))
        })
        .await
        .map_err(Error::internal)
}

/// Install file limits before the kit parses multipart bodies for method overrides or actions.
pub async fn limit_multipart_uploads(
    axum::extract::State(app): axum::extract::State<App>,
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    if request.method() == axum::http::Method::POST
        && let Some(encoded) = request.uri().path().strip_prefix("/rails/active_storage/disk/") {
        let encoded = percent_encoding::percent_decode_str(encoded.split('.').next().unwrap_or("")).decode_utf8_lossy();
        if let Some(token) = disk::decode_verified_token(&*app.storage.verifier, &encoded, app.clock.now())
            && let Ok(limit) = usize::try_from(token.content_length) {
            request.extensions_mut().insert(campfire_kit::body::RequestBodyLimit(limit));
        }
    }
    let media = campfire_kit::request::media_type(request.headers().get(header::CONTENT_TYPE).and_then(|value| value.to_str().ok()));
    if matches!(media.as_deref(), Some("multipart/form-data" | "multipart/related" | "multipart/mixed"))
        && !request.uri().path().starts_with("/rails/active_storage/disk/") {
        let limit = match upload_limit_bytes(&app).await {
            Ok(limit) => limit as u64,
            Err(error) => {
                tracing::error!(%error, "reading multipart upload limit");
                return axum::response::IntoResponse::into_response(StatusCode::INTERNAL_SERVER_ERROR);
            }
        };
        request.extensions_mut().insert(campfire_kit::body::MultipartFileLimits(vec![
            ("attachment".into(), limit),
            ("workspace_icon[image]".into(), 256 * 1024),
            ("account[logo]".into(), campfire_storage::branding::MAX_BYTES),
            ("account[banner]".into(), campfire_storage::branding::MAX_BYTES),
        ]));
    }
    next.run(request).await
}

fn upload_limit_response(c: &mut Ctx, limit: i64) -> Result<Response> {
    let mb = 1024 * 1024;
    let size = if limit % mb == 0 {
        format!("{} MB", limit / mb)
    } else {
        format!("{limit} bytes")
    };
    let message = format!("File exceeds the {size} upload limit.");
    c.json(StatusCode::UNPROCESSABLE_ENTITY, &serde_json::json!({
        "error": {"_tag": "Validation", "message": message, "fields": {"byteSize": [message]}}
    }))
}

/// `blob.as_json(root: false, methods: :signed_id).merge(direct_upload: { url:, headers: })`
fn direct_upload_json(blob: &Blob, signed_id: &str, url: &str, content_type: Option<&str>) -> String {
    let text = |s: Option<&str>| s.map_or(Json::Null, Json::from);
    Json::Object(vec![
        ("id".into(), Json::Int(blob.id)),
        ("key".into(), blob.key.as_str().into()),
        ("filename".into(), blob.filename.raw().into()),
        ("content_type".into(), text(blob.content_type.as_deref())),
        ("metadata".into(), blob.metadata.clone()),
        ("service_name".into(), blob.service_name.as_str().into()),
        ("byte_size".into(), Json::Int(blob.byte_size)),
        ("checksum".into(), text(blob.checksum.as_deref())),
        ("created_at".into(), Json::String(json_time(&blob.created_at))),
        ("signed_id".into(), signed_id.into()),
        (
            "direct_upload".into(),
            Json::Object(vec![
                ("url".into(), url.into()),
                ("headers".into(), Json::Object(vec![("Content-Type".into(), text(content_type))])),
            ]),
        ),
    ])
    .encode()
}

/// A stored `created_at` (`YYYY-MM-DD HH:MM:SS[.ffffff]`, UTC) as `ActiveSupport::JSON` encodes
/// times: ISO 8601 with milliseconds.
fn json_time(db_time: &str) -> String {
    let parsed = jiff::civil::DateTime::strptime("%Y-%m-%d %H:%M:%S%.f", db_time)
        .or_else(|_| jiff::civil::DateTime::strptime("%Y-%m-%d %H:%M:%S", db_time));
    match parsed {
        Ok(time) => time.strftime("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
        Err(_) => db_time.to_string(),
    }
}

/// `ActiveStorageAuthentication`: human sessions must have completed the second factor. These
/// framework controllers do not restore a session or run application enrollment/idle callbacks.
async fn require_active_storage_authentication(c: &mut Ctx) -> Result<()> {
    if let Some(session) = find_session_by_cookie(c).await? {
        if session.two_factor_verified() {
            return Ok(());
        }
        let user_id = session.user_id;
        let allowed = c.app().db.read(move |conn| {
            Ok(campfire_db::User::find(conn, user_id)?.requires_two_factor())
        }).await.map_err(Error::internal)?;
        if !allowed {
            return Ok(());
        }
    }
    halt(head(StatusCode::UNAUTHORIZED))
}

// --- Purging ---------------------------------------------------------------------------------------

/// `ActiveStorage::Blob#purge`: `destroy` (refused while any attachment still points at the
/// blob; destroys its variant records and preview image attachment, whose blobs are purged
/// later), then delete the files.
pub async fn purge(app: &App, blob_id: i64) -> anyhow::Result<()> {
    let marker = rails_compat::blob_branding::Marker::new(&app.secrets);
    let destroyed = app
        .db
        .write(move |tx| {
            let conn = tx.conn();
            let Some(blob) = Blob::find(conn, blob_id).map_err(storage_error)? else { return Ok(None) };
            // before_destroy(prepend: true) { raise ActiveRecord::InvalidForeignKey if attachments.exists? }
            if !campfire_storage::blob::attachment_records(conn, blob_id).map_err(storage_error)?.is_empty() {
                return Ok(None);
            }
            if branding_blob(conn, marker, &blob)? {
                mark_branding_tree(conn, marker, blob_id)?;
            }
            let mut dependents = Vec::new();
            // before_destroy { variant_records.destroy_all }: each record's image attachment goes too.
            let variant_records: Vec<i64> = query_ids(conn, "SELECT id FROM active_storage_variant_records WHERE blob_id = ?1", blob_id)?;
            for record_id in variant_records {
                dependents.extend(destroy_attachment(conn, "ActiveStorage::VariantRecord", record_id, "image")?);
                conn.execute_cached("DELETE FROM active_storage_variant_records WHERE id = ?1", [record_id])?;
            }
            // has_one_attached :preview_image (dependent: :destroy on the attachment)
            dependents.extend(destroy_attachment(conn, "ActiveStorage::Blob", blob_id, "preview_image")?);
            // Stills keep the room restriction after their source association is removed.
            if matches!(blob.metadata.get("poll_media"), Some(Json::Bool(true))) {
                for dependent in &dependents {
                    if let Some(mut image) = Blob::find(conn, *dependent).map_err(storage_error)? {
                        image.metadata.set("poll_media", Json::Bool(true));
                        image.update_metadata(conn, image.metadata.clone()).map_err(storage_error)?;
                    }
                }
            }
            conn.execute_cached("DELETE FROM active_storage_blobs WHERE id = ?1", [blob_id])?;
            // after_destroy_commit :purge_dependent_blob_later
            for dependent in &dependents {
                tx.emit_after_commit(campfire_db::Event::PurgeBlob { blob_id: *dependent });
            }
            Ok(Some(blob))
        })
        .await?;
    if let Some(blob) = destroyed {
        delete_files(app.storage.clone(), blob).await?;
    }
    Ok(())
}

async fn delete_files(storage: Arc<Storage>, blob: Blob) -> anyhow::Result<()> {
    tokio::task::spawn_blocking(move || storage.delete_files(&blob)).await??;
    Ok(())
}

/// Deletes the attachment row, returning its blob id.
fn destroy_attachment(conn: &rusqlite::Connection, record_type: &str, record_id: i64, name: &str) -> campfire_db::Result<Option<i64>> {
    let attachment: Option<(i64, i64)> = conn
        .query_row_cached(
            "SELECT id, blob_id FROM active_storage_attachments WHERE record_type = ?1 AND record_id = ?2 AND name = ?3 LIMIT 1",
            params![record_type, record_id, name],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map(Some)
        .or_else(|error| if error == rusqlite::Error::QueryReturnedNoRows { Ok(None) } else { Err(error) })?;
    let Some((id, blob_id)) = attachment else { return Ok(None) };
    conn.execute_cached("DELETE FROM active_storage_attachments WHERE id = ?1", [id])?;
    Ok(Some(blob_id))
}

fn query_ids(conn: &rusqlite::Connection, sql: &str, id: i64) -> campfire_db::Result<Vec<i64>> {
    let mut statement = conn.prepare_cached(sql)?;
    let ids = statement.query_map([id], |row| row.get(0))?.collect::<rusqlite::Result<Vec<i64>>>()?;
    Ok(ids)
}

fn storage_error(error: campfire_storage::Error) -> campfire_db::Error {
    match error {
        campfire_storage::Error::Sql(error) => error.into(),
        other => campfire_db::Error::Other(other.to_string()),
    }
}

fn storage_error_to_kit(error: campfire_storage::Error) -> Error {
    Error::internal(error)
}

fn random_hex(bytes: usize) -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    (0..bytes).map(|_| format!("{:02x}", rng.random::<u8>())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branding_analysis_rejects_client_metadata() {
        let marker =
            rails_compat::blob_branding::Marker::new(&rails_compat::Secrets::new("test-secret"));
        let conn = campfire_db::Connection::open_in_memory().unwrap();
        conn.execute_batch(campfire_db::schema::SCHEMA_SQL).unwrap();
        let mut blob = campfire_storage::NewBlob::unfurl(
            b"image",
            Filename::new("image.png"),
            Some("image/png"),
            "local",
            false,
        )
        .insert(&conn, jiff::Timestamp::now())
        .unwrap();
        for metadata in [
            r#"{"branding":true}"#,
            r#"{"branding_animated":false}"#,
            r#"{"branding":true,"branding_animated":true,"branding_mark":"made-up"}"#,
        ] {
            blob.metadata = Json::parse(metadata).unwrap();
            blob.update_metadata(&conn, blob.metadata.clone()).unwrap();
            assert!(!branding_blob(&conn, marker, &blob).unwrap(), "{metadata}");
        }
        blob.metadata.set(
            campfire_storage::branding::MARK_KEY,
            Json::String(marker.sign("another-key")),
        );
        blob.update_metadata(&conn, blob.metadata.clone()).unwrap();
        assert!(!branding_blob(&conn, marker, &blob).unwrap());
        blob.metadata.set(
            campfire_storage::branding::MARK_KEY,
            Json::String(marker.sign(&blob.key)),
        );
        blob.update_metadata(&conn, blob.metadata.clone()).unwrap();
        assert!(branding_blob(&conn, marker, &blob).unwrap());
        let other_secret =
            rails_compat::blob_branding::Marker::new(&rails_compat::Secrets::new("another-secret"));
        assert!(!branding_blob(&conn, other_secret, &blob).unwrap());
    }

    #[test]
    fn branding_analysis_follows_legacy_previews_and_stops_once_purged() {
        use campfire_storage::blob::{NewBlob, insert_attachment, insert_variant_record};

        let conn = campfire_db::Connection::open_in_memory().unwrap();
        conn.execute_batch(campfire_db::schema::SCHEMA_SQL).unwrap();
        let now = jiff::Timestamp::now();
        let marker =
            rails_compat::blob_branding::Marker::new(&rails_compat::Secrets::new("test-secret"));
        let new_blob = |name: &str, content_type: &str| {
            NewBlob::unfurl(b"bytes", Filename::new(name), Some(content_type), "local", false)
                .insert(&conn, now)
                .unwrap()
        };
        let original = new_blob("banner.mp4", "video/mp4");
        let preview = new_blob("banner.png", "image/png");
        let preview_variant = new_blob("banner.webp", "image/webp");
        insert_attachment(&conn, "preview_image", "ActiveStorage::Blob", original.id, preview.id, now)
            .unwrap();
        let record = insert_variant_record(&conn, preview.id, "digest").unwrap().unwrap();
        insert_attachment(
            &conn,
            "image",
            "ActiveStorage::VariantRecord",
            record,
            preview_variant.id,
            now,
        )
        .unwrap();
        for blob in [&original, &preview, &preview_variant] {
            assert!(!branding_blob(&conn, marker, blob).unwrap());
        }

        insert_attachment(&conn, "banner", "Account", 1, original.id, now).unwrap();
        for blob in [&original, &preview, &preview_variant] {
            assert_eq!(branding_classification(&conn, marker, blob).unwrap(), Some(true));
        }

        let purged = new_blob("purged.png", "image/png");
        conn.execute("DELETE FROM active_storage_blobs WHERE id = ?1", [purged.id])
            .unwrap();
        assert_eq!(branding_classification(&conn, marker, &purged).unwrap(), None);
    }

    #[test]
    fn branding_analysis_follows_legacy_variants_and_retains_detached_purpose() {
        use campfire_storage::blob::{NewBlob, insert_attachment, insert_variant_record};

        let conn = campfire_db::Connection::open_in_memory().unwrap();
        conn.execute_batch(campfire_db::schema::SCHEMA_SQL).unwrap();
        let now = jiff::Timestamp::now();
        let marker =
            rails_compat::blob_branding::Marker::new(&rails_compat::Secrets::new("test-secret"));
        let new_blob = || {
            NewBlob::unfurl(
                b"image",
                Filename::new("image.png"),
                Some("image/png"),
                "local",
                false,
            )
            .insert(&conn, now)
            .unwrap()
        };
        let mut original = new_blob();
        let variant = new_blob();
        let nested = new_blob();
        let attachment = new_blob();
        for (source, image) in [(&original, &variant), (&variant, &nested)] {
            let record = insert_variant_record(&conn, source.id, "digest")
                .unwrap()
                .unwrap();
            insert_attachment(
                &conn,
                "image",
                "ActiveStorage::VariantRecord",
                record,
                image.id,
                now,
            )
            .unwrap();
        }
        insert_attachment(&conn, "logo", "Account", 1, original.id, now).unwrap();
        insert_attachment(&conn, "attachment", "Message", 1, attachment.id, now).unwrap();
        insert_attachment(&conn, "other", "Account", 1, attachment.id, now).unwrap();
        for blob in [&original, &variant, &nested] {
            assert!(branding_blob(&conn, marker, blob).unwrap());
        }
        assert!(!branding_blob(&conn, marker, &attachment).unwrap());

        conn.execute("DELETE FROM active_storage_attachments WHERE record_type = 'Account' AND name = 'logo'", []).unwrap();
        original.metadata =
            Json::parse(r#"{"branding":true,"branding_animated":false,"branding_mark":"forged"}"#)
                .unwrap();
        original
            .update_metadata(&conn, original.metadata.clone())
            .unwrap();
        for blob in [&original, &variant, &nested] {
            assert!(!branding_blob(&conn, marker, blob).unwrap());
        }
        mark_branding_tree(&conn, marker, original.id).unwrap();
        for blob in [&original, &variant, &nested] {
            let marked = Blob::find(&conn, blob.id).unwrap().unwrap();
            assert!(branding_blob(&conn, marker, &marked).unwrap());
            assert!(verified_branding_mark(marker, &marked));
        }
        assert!(!branding_blob(&conn, marker, &attachment).unwrap());
    }

    #[tokio::test]
    async fn dropping_branding_request_cancels_work_and_releases_permit() {
        let (started, start) = tokio::sync::oneshot::channel();
        let (finished, finish) = tokio::sync::oneshot::channel();
        let request = tokio::spawn(process_branding_with_deadline(
            std::time::Duration::from_secs(10),
            move |cancel| {
                let _ = started.send(());
                while !cancel.is_cancelled() {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                let _ = finished.send(());
                cancel.check()
            },
        ));
        start.await.unwrap();
        assert_eq!(BRANDING_PERMITS.available_permits(), 0);
        assert_eq!(MEDIA_PERMITS.available_permits(), media_capacity());
        request.abort();
        let _ = request.await;
        tokio::time::timeout(std::time::Duration::from_millis(500), async {
            finish.await.unwrap();
            while BRANDING_PERMITS.available_permits() != MAX_BRANDING_JOBS {
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            }
        })
        .await
        .expect("dropping the request must cancel before its ten second deadline");
        assert_eq!(BRANDING_PERMITS.available_permits(), MAX_BRANDING_JOBS);
        assert_eq!(MEDIA_PERMITS.available_permits(), media_capacity());
    }

    #[tokio::test]
    async fn byte_ranges_are_not_read_into_memory() {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), (0..=255u8).cycle().take(200_000).collect::<Vec<u8>>()).unwrap();
        let path = file.path().to_path_buf();
        let range = |start, end| BodyPart::File { path: path.clone(), start, end };

        match parts_body(vec![range(10, 199_999)]) {
            campfire_kit::Body::File(body) => assert_eq!((body.offset, body.len), (10, 199_990)),
            other => panic!("a single range should be a file body, got {other:?}"),
        }

        let parts = vec![BodyPart::Bytes(b"<".to_vec()), range(0, 2), BodyPart::Bytes(b">".to_vec()), range(100_000, 170_000)];
        let length = parts_len(&parts);
        let campfire_kit::Body::Stream(stream) = parts_body(parts) else { panic!("several ranges should stream") };
        let streamed = axum::body::to_bytes(stream, usize::MAX).await.unwrap();
        let contents = std::fs::read(file.path()).unwrap();
        assert_eq!(streamed, [b"<".as_slice(), &contents[0..3], b">", &contents[100_000..=170_000]].concat());
        assert_eq!(streamed.len() as u64, length);
    }

    #[test]
    fn json_times_have_milliseconds() {
        assert_eq!(json_time("2026-03-02 16:00:00.123456"), "2026-03-02T16:00:00.123Z");
        assert_eq!(json_time("2026-03-02 16:00:00"), "2026-03-02T16:00:00.000Z");
    }
}
