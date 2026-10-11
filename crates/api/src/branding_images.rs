//! Branding images (the workspace logo and banner, and each person's profile banner): one check
//! of an uploaded image and one save of its probe, so every slot accepts the same files, keeps an
//! animated image's first frame and stays bounded through analysis.

use campfire_app::app::AppCtx;
use campfire_kit::{Ctx, Error, Result};
use campfire_runtime::presenters::attachments;
use campfire_storage::branding::{self, Kind, Prepared};

use crate::error::{fail, validation};

/// The uploaded blob behind `signed_id`, checked and probed as a `kind` image of at most
/// `max_bytes`: a PNG, JPEG, GIF or WebP within the kind's dimensions, with a first-frame still
/// when it's animated. Refusals name `signedId`.
pub(crate) async fn prepare(
    c: &mut Ctx,
    signed_id: &str,
    kind: Kind,
    max_bytes: u64,
) -> Result<Prepared> {
    let id = campfire_storage::paths::verify_signed_blob_id(
        &*c.app().storage.verifier,
        signed_id,
        c.app().clock.now(),
    )
    .ok_or_else(|| fail(c, validation("signedId", "isn't an uploaded file")))?;
    let blob = c
        .app()
        .db
        .read(move |conn| {
            campfire_storage::Blob::find(conn, id).map_err(attachments::storage_error)
        })
        .await
        .map_err(Error::internal)?
        .ok_or_else(|| fail(c, validation("signedId", "isn't an uploaded file")))?;
    if !matches!(
        blob.content_type(),
        "image/png" | "image/jpeg" | "image/gif" | "image/webp"
    ) {
        return Err(fail(
            c,
            validation("signedId", "must be a PNG, JPEG, GIF or WebP image"),
        ));
    }
    if blob.byte_size > max_bytes as i64 {
        return Err(fail(c, validation("signedId", size_message(max_bytes))));
    }
    let storage = c.app().storage.clone();
    let prepared = campfire_runtime::active_storage::process_branding_with_deadline(
        branding::processing_timeout(&blob),
        move |cancel| Ok(branding::prepare_with_limit(&storage, blob, kind, &cancel, max_bytes)),
    )
    .await
    .map_err(|_| fail(c, validation("signedId", "couldn't be read as an image")))?;
    prepared.map_err(|invalid| {
        let message = match invalid {
            branding::Invalid::Size => size_message(max_bytes),
            other => other.message(kind),
        };
        fail(c, validation("signedId", message))
    })
}

/// The branding cap, or the workspace's smaller upload limit.
fn size_message(max_bytes: u64) -> &'static str {
    if max_bytes < branding::MAX_BYTES {
        "exceeds the workspace upload limit"
    } else {
        branding::Invalid::Size.message(Kind::Banner)
    }
}

/// Save the probe and its still atomically with the attachment, retaining analyzer metadata.
pub(crate) fn save(
    tx: &mut campfire_db::Tx<'_>,
    storage: &campfire_storage::Storage,
    marker: rails_compat::blob_branding::Marker,
    prepared: Prepared,
) -> campfire_db::Result<()> {
    let mut blob = campfire_storage::Blob::find(tx.conn(), prepared.blob.id)
        .map_err(attachments::storage_error)?
        .ok_or(campfire_db::Error::RecordNotFound("ActiveStorage::Blob"))?;
    let mut animated = false;
    if let Some((variation, image)) = prepared.still {
        if storage
            .record_variant(tx.conn(), &blob, &variation, &image, tx.now().jiff())
            .map_err(attachments::storage_error)?
            .is_some()
        {
            campfire_runtime::active_storage::keep_after_commit(tx, image);
            animated = true;
        } else {
            // A competing upload may have recorded it, or a previous still may be missing.
            animated = storage
                .existing_variant_file(tx.conn(), &blob, &variation)
                .ok()
                .flatten()
                .is_some();
        }
    }
    blob.metadata.merge(&prepared.blob.metadata);
    blob.metadata.set(
        branding::ANIMATED_KEY,
        campfire_storage::Json::Bool(animated),
    );
    blob.metadata.set(
        branding::MARK_KEY,
        campfire_storage::Json::String(marker.sign(&blob.key)),
    );
    tx.conn().execute(
        "UPDATE active_storage_blobs SET content_type = ?1, metadata = ?2 WHERE id = ?3",
        rusqlite::params![prepared.blob.content_type, blob.metadata.encode(), blob.id],
    )?;
    campfire_runtime::active_storage::mark_branding_tree(tx.conn(), marker, blob.id)?;
    Ok(())
}
