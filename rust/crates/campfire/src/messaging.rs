//! Domain runtime adapters consumed by WS8b's actions.
use campfire_db::{Blob, Result, Tx};
use campfire_storage::Storage;
use std::sync::Arc;

pub struct ForwarderCopier {
    storage: Arc<Storage>,
}
impl ForwarderCopier {
    #[allow(dead_code, reason = "WS8b supplies this adapter to its forward action")]
    pub fn new(storage: Arc<Storage>) -> Self {
        Self { storage }
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
}
