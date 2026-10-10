//! Domain runtime adapters consumed by WS8b's actions.
use campfire_db::{Blob, Result, Tx};
use campfire_storage::Storage;
use std::sync::Arc;

mod operations;
pub use operations::{canonicalize_body, process_attachment, process_attachment_now, save_staged};

pub struct ForwarderCopier {
    storage: Arc<Storage>,
}
impl ForwarderCopier {
    #[allow(dead_code, reason = "WS8b supplies this adapter to its forward action")]
    pub fn new(storage: Arc<Storage>) -> Self {
        Self {
            storage,
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
        if let Some(blob) = campfire_storage::Blob::attached(tx.conn(), "Message", message.id, "attachment").map_err(|e| campfire_db::Error::Other(e.to_string()))? {
            crate::controllers::presenters::attachments::enqueue_analysis(tx, &blob);
        }
        campfire_db::models::message_attachment_processing::schedule_message(tx, message)
    }
}

/// ChannelThread posts process after the outermost posting transaction commits.
pub fn process_message_attachment(
    tx: &mut Tx<'_>,
    _storage: Arc<Storage>,
    message: &campfire_db::Message,
) -> Result<()> {
    campfire_db::models::message_attachment_processing::schedule_message(tx, message)
}
