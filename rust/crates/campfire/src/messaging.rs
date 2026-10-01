//! Domain runtime adapters consumed by WS8b's actions.
use crate::controllers::presenters::storage_error;
use campfire_db::{Blob, Result, Tx};
use campfire_storage::{Storage, Variation};
use std::sync::Arc;

pub struct ForwarderCopier {
    storage: Arc<Storage>,
    nested_variant_upload: bool,
}
impl ForwarderCopier {
    #[allow(dead_code, reason = "WS8b supplies this adapter to its forward action")]
    pub fn new(storage: Arc<Storage>) -> Self {
        Self {
            storage,
            nested_variant_upload: false,
        }
    }
}
impl campfire_db::models::forwarder::BlobCopier for ForwarderCopier {
    fn copy(&self, tx: &mut Tx<'_>, blob: &Blob) -> Result<Blob> {
        let source = campfire_storage::Blob::find(tx.conn(), blob.id)
            .map_err(|e| campfire_db::Error::Other(e.to_string()))?
            .ok_or(campfire_db::Error::RecordNotFound("ActiveStorage::Blob"))?;
        let staged = self
            .storage
            .stage_copy(&source)
            .map_err(|e| campfire_db::Error::Other(e.to_string()))?;
        let saved = staged
            .insert(tx.conn(), tx.now().jiff())
            .map_err(|e| campfire_db::Error::Other(e.to_string()))?;
        let row = Blob::find(tx.conn(), saved.id)?;
        // The guard belongs to this transaction, not just to forward()'s return value.
        // A deferred enqueue failure drops the guard and deletes the uploaded file.
        crate::active_storage::keep_after_commit(tx, staged);
        Ok(row)
    }
    fn discard(&self, blobs: &[Blob]) {
        for blob in blobs {
            if let Err(error) = self.storage.service.delete(&blob.key) {
                tracing::warn!(key=%blob.key,%error,"Could not remove failed forward attachment");
            }
        }
    }
    fn process(&self, tx: &mut Tx<'_>, message: &campfire_db::Message) -> Result<()> {
        let Some(mut blob) =
            campfire_storage::Blob::attached(tx.conn(), "Message", message.id, "attachment")
                .map_err(storage_error)?
        else {
            return Ok(());
        };
        // Message#process_attachment explicitly calls Blob#analyze even when it was
        // already analyzed; unlike a retried AnalyzeJob, this save touches all owners.
        let metadata = self
            .storage
            .analyzed_metadata(&blob)
            .map_err(storage_error)?;
        blob.update_metadata(tx.conn(), metadata)
            .map_err(storage_error)?;
        crate::active_storage::touch_attachment_records(tx, blob.id)?;
        if blob.is_video() {
            self.preview(tx, &blob, Variation::format_only("webp"))?;
        } else if blob.is_previewable() {
            self.preview(tx, &blob, Variation::resize_to_limit(1200, 800, None))?;
        } else if blob.is_variable() {
            self.variant(tx, &blob, Variation::resize_to_limit(1200, 800, None))?;
        }
        Ok(())
    }
}

impl ForwarderCopier {
    // Unlike Storage's combined tool helpers, the staged originals, preview images and
    // variants all belong to the transaction's commit guards. Later target/job failures
    // therefore clean up earlier generated files as well as the copied original.
    fn preview(
        &self,
        tx: &mut Tx<'_>,
        blob: &campfire_storage::Blob,
        variation: Variation,
    ) -> Result<()> {
        let image = match self
            .storage
            .existing_preview_image(tx.conn(), blob)
            .map_err(storage_error)?
        {
            Some(image) => image,
            None => {
                let staged = self
                    .storage
                    .draw_preview_image(blob)
                    .map_err(storage_error)?;
                let image = self
                    .storage
                    .record_preview_image(tx.conn(), blob, &staged, tx.now().jiff())
                    .map_err(storage_error)?
                    .ok_or(campfire_db::Error::RecordNotFound("ActiveStorage::Blob"))?;
                crate::active_storage::keep_after_commit(tx, staged);
                image
            }
        };
        self.variant(tx, &image, variation)
    }
    fn variant(
        &self,
        tx: &mut Tx<'_>,
        blob: &campfire_storage::Blob,
        variation: Variation,
    ) -> Result<()> {
        let variation = self
            .storage
            .variation_for(blob, &variation)
            .map_err(storage_error)?;
        if self
            .storage
            .existing_variant(tx.conn(), blob, &variation)
            .map_err(storage_error)?
            .is_none()
        {
            let staged = self
                .storage
                .transform_variant(blob, &variation)
                .map_err(storage_error)?
                .defer_analysis();
            if let Some(recorded) = self
                .storage
                .record_variant(tx.conn(), blob, &variation, &staged, tx.now().jiff())
                .map_err(storage_error)?
            {
                crate::controllers::presenters::attachments::enqueue_analysis(tx, &recorded);
                if self.nested_variant_upload {
                    // Pinned VariantWithRecord#transform_blob closes its output IO before the
                    // enclosing ChannelThread transaction commits. CreateOne#upload then raises
                    // IOError after the variant, message and lifecycle rows have committed.
                    // Existing variants never run that callback; failures creating their rows
                    // remain ordinary in-transaction failures. The failed upload leaves no file.
                    tx.after_commit(move |_| {
                        drop(staged);
                        Err(campfire_db::Error::Other(
                            "IOError: closed stream in variant after_commit upload".into(),
                        ))
                    });
                } else {
                    crate::active_storage::keep_after_commit(tx, staged);
                }
            }
        }
        Ok(())
    }
}

/// Thread posts process media inside their lifecycle/message transaction (Rails
/// `ChannelThread#post_message!`). Reuse forwards' staged preview/variant guards;
/// an in-transaction media/job failure discards all generated files and rows. A new
/// variant has Rails' distinct failed after-commit upload boundary, preserving committed rows.
pub(crate) fn process_message_attachment(
    tx: &mut Tx<'_>,
    storage: Arc<Storage>,
    message: &campfire_db::Message,
) -> Result<()> {
    use campfire_db::models::forwarder::BlobCopier as _;
    ForwarderCopier {
        storage,
        nested_variant_upload: true,
    }
    .process(tx, message)
}
