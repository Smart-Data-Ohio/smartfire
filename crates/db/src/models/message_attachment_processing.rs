//! #226's per-blob scheduling state. Decoding never holds the database writer.
use crate::{Connection, Event, Job, Message, Result, Tx, run_write};
use jiff::SignedDuration;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

pub const LEASE: SignedDuration = SignedDuration::from_secs(15 * 60);
pub const FAILED: &str = "failed";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttachmentProcessingJob {
    pub message_id: i64,
    pub blob_id: i64,
    pub token: String,
}
impl Job for AttachmentProcessingJob {
    const CLASS: &'static str = "Message::AttachmentProcessingJob";
}

pub fn claim(tx: &Tx<'_>, blob_id: i64, token: &str) -> Result<bool> {
    Ok(tx.conn().execute(
        "UPDATE active_storage_blobs SET message_processing_token=?2,message_processing_expires_at=?3
         WHERE id=?1 AND (message_processing_token IS NULL OR message_processing_token!=?4)
         AND (message_processing_token=?2 OR message_processing_expires_at IS NULL OR message_processing_expires_at<=?5)",
        params![blob_id, token, tx.now().since(LEASE), FAILED, tx.now()],
    )? == 1)
}

pub fn release(tx: &Tx<'_>, blob_id: i64, token: &str) -> Result<()> {
    tx.conn().execute("UPDATE active_storage_blobs SET message_processing_token=NULL,message_processing_expires_at=NULL WHERE id=? AND message_processing_token=?", params![blob_id, token])?;
    Ok(())
}

pub fn fail(tx: &Tx<'_>, blob_id: i64, token: &str) -> Result<()> {
    tx.conn().execute("UPDATE active_storage_blobs SET message_processing_token=?2,message_processing_expires_at=NULL WHERE id=?1 AND message_processing_token=?3", params![blob_id, FAILED, token])?;
    Ok(())
}

fn enqueue_failures(token: Option<&str>) -> u32 {
    token
        .and_then(|s| s.rsplit_once(':'))
        .and_then(|(_, n)| n.parse().ok())
        .unwrap_or(0)
}

pub fn defer(tx: &Tx<'_>, blob_id: i64, token: &str) -> Result<()> {
    let failures = enqueue_failures(Some(token)).saturating_add(1).min(5);
    let seconds = (60 * 2_i64.pow(failures - 1)).min(900);
    tx.conn().execute("UPDATE active_storage_blobs SET message_processing_token=?2,message_processing_expires_at=?3 WHERE id=?1 AND message_processing_token=?4",
        params![blob_id, format!("enqueue_failed:{failures}"), tx.now().since(SignedDuration::from_secs(seconds)), token])?;
    Ok(())
}

pub fn owns(conn: &Connection, message_id: i64, blob_id: i64) -> Result<bool> {
    Ok(conn.query_row("SELECT EXISTS(SELECT 1 FROM active_storage_attachments WHERE record_type='Message' AND name IN ('attachment','attachments') AND record_id=? AND blob_id=?)", params![message_id, blob_id], |r| r.get(0))?)
}

/// `ActiveRecord.after_all_transactions_commit`: scheduling cannot fail the committed post.
/// Claim and enqueue commit separately, as Rails' SQL claim and Redis enqueue do. A refused
/// enqueue therefore retains its claim long enough to persist the guarded cooldown.
pub fn schedule(tx: &mut Tx<'_>, message_id: i64, blob_id: i64) {
    tx.after_commit(move |after| {
        let result = run_write(after.conn(), after.env(), |tx| {
            if !owns(tx.conn(), message_id, blob_id)? { return Ok(None); }
            let token: Option<Option<String>> = tx.conn().query_row("SELECT message_processing_token FROM active_storage_blobs WHERE id=?", [blob_id], |r| r.get(0)).optional()?;
            let Some(previous) = token else { return Ok(None); };
            let token = format!("{}:{}", crate::sql::uuid(), enqueue_failures(previous.as_deref()));
            Ok(claim(tx, blob_id, &token)?.then_some(AttachmentProcessingJob { message_id, blob_id, token }))
        });
        match result {
            Ok(Some(job)) => {
                if let Err(error) = run_write(after.conn(), after.env(), |tx| {
                    tx.emit_after_commit(Event::job(&job));
                    Ok(())
                }) {
                    tracing::warn!(message_id, blob_id, %error, "Attachment processing enqueue failed");
                    if let Err(error) = run_write(after.conn(), after.env(), |tx| defer(tx, blob_id, &job.token)) {
                        tracing::warn!(message_id, blob_id, %error, "Attachment enqueue cooldown failed");
                    }
                }
            }
            Err(error) => tracing::warn!(message_id, blob_id, %error, "Attachment processing claim failed"),
            Ok(None) => (),
        }
        Ok(())
    });
}

pub fn schedule_message(tx: &mut Tx<'_>, message: &Message) -> Result<()> {
    for (_, blob) in message.attachments(tx.conn())? {
        schedule(tx, message.id, blob.id);
    }
    Ok(())
}

/// Rendering calls this even on a PR-card/message fragment-cache hit.
pub fn recover(tx: &mut Tx<'_>, message_id: i64, blob_id: i64) -> Result<()> {
    let pending: bool = tx.conn().query_row("SELECT EXISTS(SELECT 1 FROM active_storage_blobs b WHERE b.id=? AND b.content_type LIKE 'video/%' AND NOT EXISTS(SELECT 1 FROM active_storage_attachments a WHERE a.record_type='ActiveStorage::Blob' AND a.name='preview_image' AND a.record_id=b.id))", [blob_id], |r| r.get(0))?;
    if pending {
        schedule(tx, message_id, blob_id);
    }
    Ok(())
}
