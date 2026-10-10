//! `Message::AttachmentProcessingJob`: committed media, per-blob retries, every current owner.
use super::Registry;
use crate::app::App;
use campfire_db::{Message, broadcasts, models::message_attachment_processing as processing};
use campfire_jobs::{Execution, JobError, JobKind, JobResult, Outcome, RetryPolicy};
use campfire_storage::Blob;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct AttachmentProcessingJob(pub processing::AttachmentProcessingJob);
impl campfire_db::Job for AttachmentProcessingJob {
    const CLASS: &'static str = "Message::AttachmentProcessingJob";
}

impl JobKind for AttachmentProcessingJob {
    fn retry_policy() -> RetryPolicy {
        RetryPolicy::application_job()
            .attempts(3)
            .retry_on(|_| true)
    }
}

pub(super) fn register(registry: &mut Registry) {
    registry.register(perform);
    registry.on_exhausted_in::<AttachmentProcessingJob, _>(|tx, _, job| {
        processing::fail(tx, job.0.blob_id, &job.0.token)
    });
}

pub async fn perform(app: App, job: AttachmentProcessingJob, _: Execution) -> JobResult {
    perform_owned(app, job).await.map_err(|error| match error {
        error @ JobError::Discard(_) => error,
        error => JobError::retry_group(anyhow::anyhow!("{:#}", error.error()), "StandardError", 3),
    })
}

async fn perform_owned(app: App, job: AttachmentProcessingJob) -> JobResult {
    let job = job.0;
    // Rails deserializes the scheduling message first. Its deletion discards this job;
    // another owner's view may recover the blob after the old lease expires.
    app.db
        .read({
            let id = job.message_id;
            move |c| Message::find(c, id)
        })
        .await
        .map_err(super::discard_missing)?;
    let work = app.db.write({
        let job = job.clone();
        move |tx| {
            let owner: bool = tx.conn().query_row("SELECT EXISTS(SELECT 1 FROM active_storage_attachments a JOIN messages m ON m.id=a.record_id WHERE a.record_type='Message' AND a.name='attachment' AND a.blob_id=?)", [job.blob_id], |r| r.get(0))?;
            if !owner {
                processing::release(tx, job.blob_id, &job.token)?;
                return Ok(None);
            }
            if !processing::claim(tx, job.blob_id, &job.token)? { return Ok(None); }
            Blob::find(tx.conn(), job.blob_id).map_err(crate::controllers::presenters::storage_error)
        }
    }).await?;
    let Some(blob) = work else {
        return Ok(Outcome::Done);
    };
    if let Err(error) = crate::messaging::process_attachment_now(&app, blob).await {
        let missing = match &error {
            campfire_kit::Error::NotFound => true,
            campfire_kit::Error::Internal(error) => error.chain().any(|cause| {
                matches!(
                    cause.downcast_ref::<campfire_db::Error>(),
                    Some(campfire_db::Error::RecordNotFound(_))
                )
            }),
            _ => false,
        };
        if missing {
            app.db
                .write(move |tx| processing::release(tx, job.blob_id, &job.token))
                .await?;
            return Ok(Outcome::Done);
        }
        return Err(anyhow::anyhow!(error.to_string()).into());
    }
    // Keep the identity check, touch and broadcast description on the writer. Rendering
    // runs after commit in its callback, before another edit can acquire the writer.
    let completion = app.db.write({
        let job = job.clone();
        let app = app.clone();
        move |tx| {
            let ids = tx.conn().prepare("SELECT m.id FROM messages m JOIN active_storage_attachments a ON a.record_id=m.id WHERE a.record_type='Message' AND a.name='attachment' AND a.blob_id=? ORDER BY m.id")?
                .query_map([job.blob_id], |r| r.get::<_, i64>(0))?.collect::<Result<Vec<_>, _>>()?;
            for id in ids {
                let Some(mut message) = Message::find_by_id(tx.conn(), id)? else { continue; };
                if !processing::owns(tx.conn(), id, job.blob_id)? { continue; }
                message.touch(tx)?;
                let broadcast = broadcasts::Broadcast::MessageUpdated { message_id: id };
                let app = app.clone();
                // Rails calls broadcast_replace_to directly here. Ordinary model callbacks
                // report and swallow failures, but this job must retry a failed completion.
                tx.after_commit(move |_| {
                    match crate::channels::sink::messaging(&app.cable, Some(&app), &broadcast) {
                        Ok(()) => Ok(()),
                        Err(error) if error.chain().any(|cause| matches!(
                            cause.downcast_ref::<campfire_db::Error>(),
                            Some(campfire_db::Error::RecordNotFound(_))
                        )) => Ok(()), // A removed owner must not suppress the other owners.
                        Err(error) => Err(campfire_db::Error::Other(format!("{error:#}"))),
                    }
                });
            }
            Ok(())
        }
    }).await;
    app.db
        .write(move |tx| processing::release(tx, job.blob_id, &job.token))
        .await?;
    completion?;
    Ok(Outcome::Done)
}
