//! What posting and processing a message's body and attachment take beyond the database: shared
//! by the messages controller, the mail and integration jobs that post messages, and the
//! attachment processing job.
use campfire_kit::{Error, Result};
use campfire_richtext::Content;
use campfire_storage::{Blob, Staged, Variation};

use crate::active_storage::{self, keep_after_commit};
use crate::app::App;
use crate::context::db_error;
use crate::controllers::presenters::{DbResolver, storage_error};

/// Inserts a staged blob's row, keeping its file once the transaction commits.
pub fn save_staged(tx: &mut campfire_db::Tx<'_>, staged: Staged) -> campfire_db::Result<Blob> {
    let blob = staged.insert(tx.conn(), tx.now().jiff()).map_err(storage_error)?;
    keep_after_commit(tx, staged);
    Ok(blob)
}

/// [`canonical_body`] on a reader, ahead of the write that stores it.
pub async fn canonicalize_body(app: &App, body: String, request_host: Option<String>) -> Result<String> {
    let app2 = app.clone();
    app.db.read(move |conn| Ok(canonical_body(conn, &app2, &body, request_host))).await.map_err(db_error)
}

/// Assigning a String to a rich text attribute stores the canonicalized content
/// (`ActionText::Content.new(body, canonicalize: true).to_html`).
pub fn canonical_body(conn: &campfire_db::Connection, app: &App, body: &str, request_host: Option<String>) -> String {
    let resolver = DbResolver::new(conn, &app.secrets, app.clock.now());
    let ctx = resolver.render_context(request_host);
    Content::load(body, &ctx).map(|content| content.to_html()).unwrap_or_else(|_| body.to_string())
}

/// `Message#process_attachment`: analyze the blob now (its `after_update` touches the message),
/// then generate the video preview or the `:thumb` representation.
pub async fn process_attachment(app: &App, blob: Blob) -> Result<()> {
    use campfire_db::models::message_attachment_processing as processing;
    let blob_id = blob.id;
    let token = uuid::Uuid::new_v4().to_string();
    let claimed = app.db.write({ let token = token.clone(); move |tx| processing::claim(tx, blob_id, &token) }).await;
    match claimed {
        Ok(true) => {
            if let Err(error) = process_attachment_now(app, blob).await {
                tracing::warn!(blob_id, %error, "Committed attachment processing failed");
            }
            if let Err(error) = app.db.write(move |tx| processing::release(tx, blob_id, &token)).await {
                tracing::warn!(blob_id, %error, "Attachment processing release failed");
            }
        }
        Err(error) => tracing::warn!(blob_id, %error, "Committed attachment processing claim failed"),
        Ok(false) => (),
    }
    Ok(())
}

pub async fn process_attachment_now(app: &App, blob: Blob) -> Result<()> {
    let blob = analyze_attachment(app, blob).await?;
    if blob.is_video() {
        // attachment.preview(format: :webp).processed
        active_storage::processed_preview(app, blob, Variation::format_only("webp")).await?;
    } else if blob.is_representable() {
        // attachment.representation(:thumb).processed
        let thumb = Variation::resize_to_limit(1200, 800, None);
        if blob.content_type() == "image/jpeg" {
            // Rails `.processed?` reuses the record. Serving endpoints handle a
            // missing final file without regenerating an already recorded variant.
            let storage = app.storage.clone();
            let source = blob.clone();
            let variation = storage.variation_for(&source, &thumb).map_err(Error::internal)?;
            let processed = app.db.read(move |conn| {
                storage.existing_variant(conn, &source, &variation)
                    .map(|image| image.is_some()).map_err(storage_error)
            }).await.map_err(db_error)?;
            if processed { return Ok(()); }
        }
        active_storage::processed_representation(app, blob, thumb).await?;
    }
    Ok(())
}

/// `blob.analyze`: its `after_update` touches the attached records. The file is analyzed off the
/// writer.
async fn analyze_attachment(app: &App, blob: Blob) -> Result<Blob> {
    active_storage::analyze_explicit(app, blob.id).await.map_err(Error::internal)?.ok_or(Error::NotFound)
}
