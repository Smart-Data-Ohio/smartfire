//! The lease and durable continuation protocol shared by the Slack job handlers.
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use campfire_db::models::slack::SlackConnection;
use campfire_db::models::slack_import::{SlackImport, StepJob, StepStatus, UndoJob};
use campfire_db::{Database, Event, Job, JobRequest};
use campfire_jobs::{JobKind, is_transient};
use rails_compat::ar_encryption::ArEncryption;
use serde::{Deserialize, Serialize};

use super::client::{Error, ErrorKind};
use super::runner::Outcome;

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ImportStep(pub StepJob);
impl Job for ImportStep {
    const CLASS: &'static str = StepJob::CLASS;
}
impl JobKind for ImportStep {
    const QUEUE: &'static str = crate::jobs::SLACK_IMPORT_QUEUE;
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UndoStep(pub UndoJob);
impl Job for UndoStep {
    const CLASS: &'static str = UndoJob::CLASS;
}
impl JobKind for UndoStep {
    const QUEUE: &'static str = crate::jobs::SLACK_IMPORT_QUEUE;
}

pub fn register(registry: &mut crate::jobs::Registry) {
    registry.register(import_step);
    registry.register(undo_step);
}

async fn import_step(
    app: crate::app::App,
    job: ImportStep,
    _: campfire_jobs::Execution,
) -> campfire_jobs::JobResult {
    let db = app.db.clone();
    let network = app.slack_network.clone();
    let pacing = app.config.environment != "test";
    perform_import(
        app.db.clone(),
        app.ar_encryption.clone(),
        job.0.import_id,
        move |run, lease, token| async move {
            super::runner::Runner::new(
                super::store::SqlStore {
                    db,
                    lease,
                    allowed_domains: super::store::allowed_domains(),
                },
                super::client::Client::with_network(token, None, pacing, network),
                run,
            )
            .step()
            .await
        },
    )
    .await?;
    Ok(campfire_jobs::Outcome::Done)
}

async fn undo_step(
    app: crate::app::App,
    job: UndoStep,
    _: campfire_jobs::Execution,
) -> campfire_jobs::JobResult {
    let id = job.0.import_id;
    let db = app.db.clone();
    perform_undo(app.db.clone(), id, move |_, lease| async move {
        super::undoer::Undoer { db, id, lease }.step().await
    })
    .await?;
    Ok(campfire_jobs::Outcome::Done)
}

pub async fn perform_import<F, Fut>(
    db: Database,
    encryption: Arc<ArEncryption>,
    id: i64,
    work: F,
) -> anyhow::Result<()>
where
    F: FnOnce(SlackImport, String, String) -> Fut,
    Fut: Future<Output = anyhow::Result<Outcome>>,
{
    let claimed = db
        .write(move |tx| {
            let Some(mut run) = SlackImport::find(tx.conn(), id)? else {
                return Ok(None);
            };
            if run.status == "queued" {
                if !SlackImport::claim_running(tx, id)? {
                    SlackImport::clear_pending_step_job(tx, id)?;
                    return Ok(None);
                }
                run = SlackImport::find(tx.conn(), id)?.expect("claimed run");
            }
            if run.status != "running" {
                return Ok(None);
            }
            let connection = run
                .slack_connection_id
                .map(|id| SlackConnection::find(tx.conn(), id))
                .transpose()?
                .flatten();
            let token = if let Some(connection) = connection {
                if connection.connected(&encryption)? {
                    connection.access_token(&encryption)?
                } else {
                    None
                }
            } else {
                None
            };
            let Some(token) = token else {
                SlackImport::mark_failed(tx, id, "Slack connection is missing or disconnected")?;
                return Ok(None);
            };
            Ok(
                SlackImport::acquire_step_lease(tx, id, StepStatus::Running)?
                    .map(|lease| (run, lease, token)),
            )
        })
        .await?;
    let Some((run, lease, token)) = claimed else {
        return Ok(());
    };
    let result = work(run, lease.clone(), token).await;
    // Always release before publishing the next job. A crash leaves the lease for the sweep.
    db.write(move |tx| SlackImport::release_step_lease(tx, id, &lease))
        .await?;
    match result {
        Ok(outcome) => {
            db.write(move |tx| {
                match outcome {
                    Outcome::Continue => {
                        tx.emit_after_commit(Event::job(&StepJob { import_id: id }))
                    }
                    Outcome::Stopped => {
                        SlackImport::kick_next_queued(tx)?;
                    }
                    Outcome::Done => (),
                }
                Ok(())
            })
            .await?;
        }
        Err(error) => {
            if is_transient(&error) {
                return Err(error);
            }
            let slack = error.downcast_ref::<Error>();
            let delay = slack
                .filter(|e| e.kind == ErrorKind::RateLimited)
                .and_then(|e| e.retry_after);
            let auth = slack.is_some_and(|e| e.kind == ErrorKind::Auth);
            let message = error.to_string();
            db.write(move |tx| {
                let Some(run) = SlackImport::find(tx.conn(), id)? else { return Ok(()); };
                if run.status == "running" {
                    if let Some(delay) = delay {
                        tx.conn().execute("UPDATE slack_imports SET heartbeat_at = ? WHERE id = ?", rusqlite::params![tx.now(),id])?;
                        tx.emit_after_commit(Event::Job(JobRequest::new(&StepJob { import_id:id }).wait(Duration::from_secs(delay))));
                    } else { SlackImport::mark_failed(tx, id, &message)?; }
                }
                // Rails disconnects the credential even if cancellation landed during auth failure.
                if auth && let Some(connection_id) = run.slack_connection_id {
                    tx.conn().execute("UPDATE slack_connections SET disconnected_reason = ?, updated_at = ? WHERE id = ?",
                        rusqlite::params![message,tx.now(),connection_id])?;
                }
                Ok(())
            }).await?;
        }
    }
    Ok(())
}

pub async fn perform_undo<F, Fut>(db: Database, id: i64, work: F) -> anyhow::Result<()>
where
    F: FnOnce(SlackImport, String) -> Fut,
    Fut: Future<Output = anyhow::Result<Outcome>>,
{
    let claimed = db
        .write(move |tx| {
            let Some(run) = SlackImport::find(tx.conn(), id)? else {
                return Ok(None);
            };
            if run.status != "undoing" {
                return Ok(None);
            }
            Ok(
                SlackImport::acquire_step_lease(tx, id, StepStatus::Undoing)?
                    .map(|token| (run, token)),
            )
        })
        .await?;
    let Some((run, lease)) = claimed else {
        return Ok(());
    };
    let result = work(run, lease.clone()).await;
    db.write(move |tx| SlackImport::release_step_lease(tx, id, &lease))
        .await?;
    let outcome = result?;
    db.write(move |tx| {
        match outcome {
            Outcome::Continue => tx.emit_after_commit(Event::job(&UndoJob { import_id: id })),
            Outcome::Stopped => {
                SlackImport::kick_next_queued(tx)?;
            }
            Outcome::Done => (),
        }
        Ok(())
    })
    .await?;
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests;
