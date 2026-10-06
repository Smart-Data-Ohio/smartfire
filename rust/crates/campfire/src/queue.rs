//! The job queue's enqueueing side: the job classes core declares, their queues, and [`Jobs`], the
//! durable sink the database emits job events into. What performs them (the handlers, the runner
//! and the periodic loops) is [`crate::jobs`], above the presenters and channels it renders with;
//! this half sits with the app state, so integrations, mail and presenters can enqueue without it.

#[cfg(test)]
use std::future::Future;
use std::sync::{Arc, OnceLock, Weak};
use std::time::Duration;

use campfire_db::models::board_automations::DigestNotes;
use campfire_db::{Event, EventSink, Job, JobRequest, Tx};
use campfire_jobs::{JobError, JobKind, JobQueue, QueueConfig, RetryPolicy, RunnerConfig, Wait};
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::app::{App, AppState};
use crate::cable::{Cable, CableUser};
use crate::config::Config;

mod peer_callbacks;

/// The app's job classes and their handlers, which get the [`App`].
pub type Registry = campfire_jobs::Registry<App>;

pub const DEFAULT_QUEUE: &str = "default";
pub const PUSH_QUEUE: &str = "push";
pub const WEBHOOKS_QUEUE: &str = "webhooks";
pub const SLACK_IMPORT_QUEUE: &str = "slack_import";

/// How many ad hoc jobs may wait before new ones are dropped (and logged).
pub const AD_HOC_CAPACITY: usize = 1024;

/// `Room::PushMessageJob.perform_later(room, message)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PushMessageJob {
    pub room_id: i64,
    pub message_id: i64,
}

impl Job for PushMessageJob {
    const CLASS: &'static str = "Room::PushMessageJob";
}

impl JobKind for PushMessageJob {
    const QUEUE: &'static str = PUSH_QUEUE;
}

/// `Bot::WebhookJob.perform_later(bot, message)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebhookJob {
    pub bot_id: i64,
    pub message_id: i64,
}

impl Job for WebhookJob {
    const CLASS: &'static str = "Bot::WebhookJob";
}

impl JobKind for WebhookJob {
    const QUEUE: &'static str = WEBHOOKS_QUEUE;
}

/// How long a posted message's webhooks wait for the post to be broadcast before they're due
/// anyway (see `controllers::messages::create_message`): long enough for the slowest attachment
/// processing, so only a process that dies mid-request waits it out.
pub const WEBHOOK_HOLD: Duration = Duration::from_secs(5 * 60);

/// `RemoveBannedContentJob.perform_later(user)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemoveBannedContentJob {
    pub user_id: i64,
}

impl Job for RemoveBannedContentJob {
    const CLASS: &'static str = "RemoveBannedContentJob";
}

impl JobKind for RemoveBannedContentJob {}

/// `ActiveStorage::PurgeJob.perform_later(blob)` (`purge_later`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PurgeJob {
    pub blob_id: i64,
}

impl Job for PurgeJob {
    const CLASS: &'static str = "ActiveStorage::PurgeJob";
}

impl JobKind for PurgeJob {
    /// `ActiveStorage::BaseJob` isn't an `ApplicationJob`: `retry_on ActiveRecord::Deadlocked,
    /// attempts: 10, wait: :polynomially_longer`, which SQLite never raises, so it doesn't retry.
    fn retry_policy() -> RetryPolicy {
        RetryPolicy::application_job().attempts(10).wait(Wait::PolynomiallyLonger { jitter: 0.15 }).retry_on(|_| false)
    }
}

/// `ActiveStorage::AnalyzeJob`: durable arguments for attachment analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyzeJob {
    pub blob_id: i64,
}

impl Job for AnalyzeJob {
    const CLASS: &'static str = "ActiveStorage::AnalyzeJob";
}

impl JobKind for AnalyzeJob {
    fn retry_policy() -> RetryPolicy {
        RetryPolicy::application_job()
            .attempts(10)
            .retry_on(|error| {
                error.chain().any(|cause| {
                    matches!(
                        cause.downcast_ref::<campfire_storage::Error>(),
                        Some(campfire_storage::Error::Integrity)
                    )
                })
            })
    }
}

/// The job a legacy job event asks for, or an [`Event::Job`]'s own. `None` for an event that
/// isn't a job.
pub fn request_for(event: &Event) -> Option<JobRequest> {
    Some(match event {
        Event::PushMessage { room_id, message_id } => JobRequest::new(&PushMessageJob { room_id: *room_id, message_id: *message_id }),
        Event::DeliverWebhook { bot_id, message_id } => JobRequest::new(&WebhookJob { bot_id: *bot_id, message_id: *message_id }),
        Event::RemoveBannedContent { user_id } => JobRequest::new(&RemoveBannedContentJob { user_id: *user_id }),
        Event::PurgeBlob { blob_id } => JobRequest::new(&PurgeJob { blob_id: *blob_id }),
        Event::Job(request) => request.clone(),
        Event::DisconnectUser { .. } | Event::Broadcast(_) => return None,
    })
}

/// `discard_on ActiveJob::DeserializationError`: a job whose argument records are gone by the
/// time it runs is discarded. Any other error is the job's to retry or fail.
pub fn discard_missing(error: campfire_db::Error) -> JobError {
    match error {
        campfire_db::Error::RecordNotFound(_) => JobError::discard(error),
        error => JobError::from(error),
    }
}


/// The queues and their workers.
pub fn runner_config(config: &Config) -> RunnerConfig {
    let workers = config.job_concurrency.max(1);
    RunnerConfig::new(vec![
        QueueConfig::new(DEFAULT_QUEUE, workers),
        QueueConfig::new(PUSH_QUEUE, workers),
        QueueConfig::new(WEBHOOKS_QUEUE, workers),
        QueueConfig::new(SLACK_IMPORT_QUEUE, 1),
    ])
}

/// Where the sink hands what goes out over the cable (disconnects, broadcasts, board digest
/// notes): the channels' renderers, which sit above this layer. [`crate::jobs::start`] installs
/// them with the cable server.
#[derive(Clone, Copy)]
pub(crate) struct Delivery {
    pub broadcast: fn(&Cable, Option<&App>, &Event) -> bool,
    pub digest_notes: fn(&Cable, &App, &DigestNotes) -> anyhow::Result<Vec<i64>>,
}

pub(crate) type AdHocWork = (&'static str, BoxFuture<'static, anyhow::Result<()>>);

/// The enqueueing side: the database's durable event sink.
/// Cheap to clone.
#[derive(Clone)]
pub struct Jobs {
    /// Inside a write, emit [`Event::Job`] so the job commits with it; outside one,
    /// `queue.perform_later(&db, request)` enqueues it in a write of its own.
    pub queue: JobQueue,
    pub model_callbacks: Arc<campfire_db::callbacks::Registry>,
    #[cfg(test)]

    ad_hoc: mpsc::Sender<AdHocWork>,
    // The server's authenticator owns the database, whose Env owns this sink.
    // Holding Cable strongly here would retain every database after shutdown.
    cable: Arc<OnceLock<campfire_cable::WeakServer<CableUser>>>,
    /// Weak because the app holds the database, which holds this sink.
    app: Arc<OnceLock<Weak<AppState>>>,
    delivery: Arc<OnceLock<Delivery>>,
}

/// The ad hoc queue's receiving end, until the runner starts.
pub struct AdHocQueue(pub(crate) mpsc::Receiver<AdHocWork>);

impl Jobs {
    /// The sink for `registry`'s classes on `config`'s queues, and the ad hoc queue, which
    /// [`start`] turns into the runner.
    pub fn new(registry: &Registry, config: &RunnerConfig) -> anyhow::Result<(Self, AdHocQueue)> {
        let queue = JobQueue::new(registry, config)?;
        let (ad_hoc, receiver) = mpsc::channel(AD_HOC_CAPACITY);
        let model_callbacks = Arc::new(campfire_db::callbacks::Registry::default());
        peer_callbacks::install(&model_callbacks);
        #[cfg(not(test))]
        drop(ad_hoc);
        Ok((Self {
            queue,
            model_callbacks,
            #[cfg(test)]
            ad_hoc,
            cable: Arc::new(OnceLock::new()),
            app: Arc::new(OnceLock::new()),
            delivery: Arc::new(OnceLock::new()),
        }, AdHocQueue(receiver)))

    }

    /// Runs best-effort work in memory. Dropped with an error log when the ad hoc queue is full,
    /// or lost if the process stops first.
    #[cfg(test)]
    pub fn perform_later(&self, name: &'static str, work: impl Future<Output = anyhow::Result<()>> + Send + 'static) {
        match self.ad_hoc.try_send((name, Box::pin(work))) {
            Ok(()) => tracing::debug!(job = name, "enqueued"),
            Err(mpsc::error::TrySendError::Full(_)) => tracing::error!(job = name, "job queue is full, dropping job"),
            Err(mpsc::error::TrySendError::Closed(_)) => tracing::warn!(job = name, "job runner stopped, dropping job"),
        }
    }

    pub(crate) fn set_app(&self, app: &App) {
        let _ = self.app.set(Arc::downgrade(app));
    }

    pub(crate) fn set_delivery(&self, delivery: Delivery) {
        let _ = self.delivery.set(delivery);
    }

    pub(crate) fn set_cable(&self, cable: Cable) {
        let _ = self.cable.set(cable.downgrade());
    }
}

impl EventSink for Jobs {
    fn broadcast_digest_notes(&self, notes: &campfire_db::models::board_automations::DigestNotes) -> campfire_db::Result<Vec<i64>> {
        let cable = self.cable.get().and_then(campfire_cable::WeakServer::upgrade)
            .ok_or_else(|| campfire_db::Error::Other("digest cable server not booted".into()))?;
        let app = self.app.get().and_then(Weak::upgrade)
            .ok_or_else(|| campfire_db::Error::Other("digest app not booted".into()))?;
        // Installed with the cable server, so it's there whenever the cable server is.
        let delivery = self.delivery.get()
            .ok_or_else(|| campfire_db::Error::Other("digest cable server not booted".into()))?;
        (delivery.digest_notes)(&cable, &app, notes)
            .map_err(|error| campfire_db::Error::Other(error.to_string()))
    }
    fn model_callback(&self, tx: &mut Tx<'_>, callback: campfire_db::callbacks::Callback) -> campfire_db::Result<()> {
        self.model_callbacks.call(tx, callback)
    }

    fn disconnect_user_accounts(&self, tx: &mut Tx<'_>, user_id: i64) -> campfire_db::Result<()> {
        if let Some(app) = self.app.get().and_then(Weak::upgrade) {
            campfire_db::models::google_connection::deactivate(tx, user_id, &app.secrets)?;
        }
        if let Some(account) = crate::integrations::fizzy::accounts::Account::for_user(tx.conn(), user_id)? {
            account.mark_disconnected(tx, "Account deactivated")?;
        }
        // GitHub deactivation uses the Env hook installed by WS15g at boot.
        Ok(())
    }

    fn sync_message_references(&self, tx: &mut Tx<'_>, message: &campfire_db::Message, enqueue: bool) -> campfire_db::Result<()> {
        let app = self.app.get().and_then(Weak::upgrade);
        let crypto = app.as_ref().map(|app| app.ar_encryption.as_ref());
        crate::integrations::sync_message_references(tx, message, enqueue, crypto)
    }

    fn sync_message_reference_phase(&self, tx: &mut Tx<'_>, message: &campfire_db::Message, phase: campfire_db::callbacks::Phase, enqueue: bool) -> campfire_db::Result<()> {
        let app = self.app.get().and_then(Weak::upgrade);
        let crypto = app.as_ref().map(|app| app.ar_encryption.as_ref());
        crate::integrations::sync_message_reference_phase(tx,message,phase,enqueue,crypto)
    }

    fn persist(&self, tx: &Tx<'_>, event: &Event) -> campfire_db::Result<()> {
        if let Some(request) = request_for(event) {
            if !tx.in_transaction() {
                // From an after-commit hook: the row commits on its own, not with the write.
                tracing::warn!(job = request.class, "enqueued after commit, outside its write's transaction");
            }
            let id = self.queue.enqueue(tx, &request)?;
            tracing::debug!(job = request.class, id, "enqueued");
        }
        Ok(())
    }

    fn emit_committed(&self, after: &mut Tx<'_>, event: Event) {
        let message_ids = attachment_render_message_ids(&event);
        self.emit(event);
        // Detached message broadcasts render on the committing thread. Use its connection
        // after the reader is released; enqueueing through Database::write would deadlock.
        if !message_ids.is_empty() {
            let result = (|| {
                // Digest events can render many messages. One bound JSON list keeps recovery
                // reads flat and respects the same small SQLite bind limit as their renderer.
                let recoveries = after.conn().prepare(
                    "SELECT a.record_id,a.blob_id FROM active_storage_attachments a
                     JOIN active_storage_blobs b ON b.id=a.blob_id
                     JOIN messages m ON m.id=a.record_id
                     WHERE a.record_type='Message' AND a.name='attachment'
                     AND a.record_id IN (SELECT value FROM json_each(?))
                     AND b.content_type LIKE 'video/%'
                     AND NOT EXISTS(SELECT 1 FROM active_storage_attachments p
                         WHERE p.record_type='ActiveStorage::Blob' AND p.name='preview_image' AND p.record_id=b.id)
                     ORDER BY a.record_id,a.id",
                )?.query_map([serde_json::json!(message_ids).to_string()], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
                })?.collect::<Result<Vec<_>, _>>()?;
                for (id, blob_id) in recoveries {
                    campfire_db::models::message_attachment_processing::schedule(after, id, blob_id);
                }
                Ok::<_, campfire_db::Error>(())
            })();
            if let Err(error) = result { tracing::warn!(%error, "Skipping broadcast attachment recovery"); }
        }
    }

    fn emit(&self, event: Event) {
        match (request_for(&event), event) {
            // Committed (or emitted after commit): the runner can claim it now.
            (Some(request), _) => self.queue.wake(request.class),
            // `ActionCable.server.remote_connections.where(current_user: user).disconnect`: a
            // pub/sub broadcast in Rails, done right away. Before boot finishes there are no
            // connections to disconnect.
            (None, event @ (Event::DisconnectUser { .. } | Event::Broadcast(_))) => {
                if let Some(cable) = self.cable.get().and_then(campfire_cable::WeakServer::upgrade)
                    && let Some(delivery) = self.delivery.get()
                {
                    let app = self.app.get().and_then(Weak::upgrade);
                    (delivery.broadcast)(&cable, app.as_ref(), &event);
                }
            }
            (None, event) => tracing::warn!(?event, "not a job, dropping event"),
        }
    }
}

/// Full attachment/presentation renders in channels::sink. Keep the non-Turbo parent
/// renderers here too: Rails' Agents::Steps#broadcast_parent renders the whole message.
/// Component-only events (cards, reactions, thread steps/indicators, room/status/huddle
/// controls) never render a video or its PR-card collection cache key.
fn attachment_render_message_ids(event: &Event) -> Vec<i64> {
    use campfire_db::{
        Broadcast as _,
        broadcasts::{Broadcast, Partial},
        models::{agent_step::StepParentChange, board_automations::DigestNotes,
            huddle_effects::StageEndedNote, user::lifecycle::QuietStreamFinal},
    };
    use crate::integrations::github::notifier::MessageCreated;
    let Event::Broadcast(request) = event else { return Vec::new(); };
    match request.kind {
        Broadcast::KIND => request.decode::<Broadcast>().and_then(Result::ok)
            .and_then(|broadcast| match broadcast {
                Broadcast::Turbo(frame) => match frame.partial {
                    Some(Partial::Message { message_id } | Partial::MessageReplace { message_id }
                        | Partial::MessagePresentation { message_id }) => Some(message_id),
                    _ => None,
                },
                _ => None,
            }).into_iter().collect(),
        StepParentChange::KIND => request.decode::<StepParentChange>().and_then(Result::ok)
            .and_then(|change| change.message_id).into_iter().collect(),
        QuietStreamFinal::KIND => request.decode::<QuietStreamFinal>().and_then(Result::ok)
            .map(|change| change.message_id).into_iter().collect(),
        MessageCreated::KIND => request.decode::<MessageCreated>().and_then(Result::ok)
            .map(|change| change.message_id).into_iter().collect(),
        StageEndedNote::KIND => request.decode::<StageEndedNote>().and_then(Result::ok)
            .map(|change| change.message_id).into_iter().collect(),
        DigestNotes::KIND => request.decode::<DigestNotes>().and_then(Result::ok)
            .map(|notes| notes.message_ids).unwrap_or_default(),
        _ => Vec::new(),
    }
}
