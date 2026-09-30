//! The app's jobs, on the durable queue in the main database (`campfire_jobs`), replacing Resque.
//!
//! Models emit [`Event`]s at the point Rails would `perform_later` (the database's
//! [`EventSink`]). [`Jobs`] turns each job event into a row in `background_jobs`, written in the
//! transaction of the write that emitted it ([`EventSink::persist`]), so a job exists exactly when
//! the write that asked for it committed, and survives a crash or restart; once the write
//! commits, it wakes the job's queue. The runner then performs it with its class's retries
//! (`retry_on`/`discard_on`, see [`campfire_jobs::RetryPolicy`]).
//!
//! Rails enqueues these in the transaction too (`enqueue_after_transaction_commit` is off), but
//! into Redis, so a rolled-back write could still leave its job behind (performed later against
//! rows that don't exist, and discarded), and a job could run before its write committed. Here
//! the job's row commits or rolls back with the write, and becomes visible to the runner only
//! when it commits.
//!
//! The queues: `push` (`Room::PushMessageJob`) and `webhooks` (`Bot::WebhookJob`) each have
//! their own workers, so a slow bot's webhooks can't hold up notifications, nor either of them
//! the rest (`default`); `slack_import` runs one job at a time across every process
//! (`config/resque-pool.yml`). Each has `JOB_CONCURRENCY` workers but `slack_import`.
//!
//! `DisconnectUser` and `Broadcast` are not jobs in Rails (it's a synchronous Action Cable broadcast), so it goes
//! straight to the cable server. [`Jobs::perform_later`] still runs ad hoc futures in memory
//! (`ActiveStorage::AnalyzeJob`, whose callers hand over a future rather than arguments): lost if
//! the process stops before they run, as before.
//!
//! [`periodic`] is `bin/periodic` and the huddle reconciler's host.

use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, OnceLock, Weak};
use std::time::Duration;

use campfire_db::{Event, EventSink, Job, JobRequest, Tx};
use campfire_jobs::{Execution, JobError, JobKind, JobQueue, JobResult, Outcome, QueueConfig, RetryPolicy, RunnerConfig, Wait};
use futures_util::FutureExt as _;
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, mpsc, watch};
use tokio::task::JoinHandle;

use crate::app::{App, AppState, Cable};
use crate::config::Config;

pub mod periodic;

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

/// Every job class the app performs: core's, and the integrations'.
pub fn registry() -> Registry {
    let mut registry = Registry::new();
    registry.register(remove_banned_content);
    registry.register(purge_blob);
    // Room::PushMessageJob and Bot::WebhookJob
    crate::integrations::register_jobs(&mut registry);
    registry
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

type AdHocWork = (&'static str, BoxFuture<'static, anyhow::Result<()>>);

/// The enqueueing side: the database's event sink, and [`Jobs::perform_later`] for ad hoc work.
/// Cheap to clone.
#[derive(Clone)]
pub struct Jobs {
    /// Inside a write, emit [`Event::Job`] so the job commits with it; outside one,
    /// `queue.perform_later(&db, request)` enqueues it in a write of its own.
    pub queue: JobQueue,
    ad_hoc: mpsc::Sender<AdHocWork>,
    cable: Arc<OnceLock<Cable>>,
    /// Weak because the app holds the database, which holds this sink.
    app: Arc<OnceLock<Weak<AppState>>>,
}

/// The ad hoc queue's receiving end, until the runner starts.
pub struct AdHocQueue(mpsc::Receiver<AdHocWork>);

impl Jobs {
    /// The sink for `registry`'s classes on `config`'s queues, and the ad hoc queue, which
    /// [`start`] turns into the runner.
    pub fn new(registry: &Registry, config: &RunnerConfig) -> anyhow::Result<(Self, AdHocQueue)> {
        let queue = JobQueue::new(registry, config)?;
        let (ad_hoc, receiver) = mpsc::channel(AD_HOC_CAPACITY);
        Ok((Self { queue, ad_hoc, cable: Arc::new(OnceLock::new()), app: Arc::new(OnceLock::new()) }, AdHocQueue(receiver)))
    }

    /// Runs best-effort work in memory. Dropped with an error log when the ad hoc queue is full,
    /// or lost if the process stops first.
    pub fn perform_later(&self, name: &'static str, work: impl Future<Output = anyhow::Result<()>> + Send + 'static) {
        match self.ad_hoc.try_send((name, Box::pin(work))) {
            Ok(()) => tracing::debug!(job = name, "enqueued"),
            Err(mpsc::error::TrySendError::Full(_)) => tracing::error!(job = name, "job queue is full, dropping job"),
            Err(mpsc::error::TrySendError::Closed(_)) => tracing::warn!(job = name, "job runner stopped, dropping job"),
        }
    }

    fn set_app(&self, app: &App) {
        let _ = self.app.set(Arc::downgrade(app));
    }

    fn set_cable(&self, cable: Cable) {
        let _ = self.cable.set(cable);
    }
}

impl EventSink for Jobs {
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

    fn emit(&self, event: Event) {
        match (request_for(&event), event) {
            // Committed (or emitted after commit): the runner can claim it now.
            (Some(request), _) => self.queue.wake(request.class),
            // `ActionCable.server.remote_connections.where(current_user: user).disconnect`: a
            // pub/sub broadcast in Rails, done right away. Before boot finishes there are no
            // connections to disconnect.
            (None, event @ (Event::DisconnectUser { .. } | Event::Broadcast(_))) => {
                if let Some(cable) = self.cable.get() {
                    let app = self.app.get().and_then(Weak::upgrade);
                    crate::channels::sink::deliver(cable, app.as_ref(), &event);
                }
            }
            (None, event) => tracing::warn!(?event, "not a job, dropping event"),
        }
    }
}

/// The running jobs: the durable queue's runner, the ad hoc workers, and the periodic loops.
pub struct Runner {
    durable: Option<campfire_jobs::Runner>,
    ad_hoc_stopping: watch::Sender<bool>,
    ad_hoc: Vec<JoinHandle<()>>,
    periodic_stopping: watch::Sender<bool>,
    periodic: Vec<JoinHandle<()>>,
}

impl Runner {
    /// Stops the periodic loops (a task in progress gets `grace` to finish), then the durable
    /// runner (running jobs get `grace` to finish; the rest stay queued for the next process),
    /// then the ad hoc workers (they perform what's queued, up to `grace`). Whatever outlasts its
    /// grace period is aborted, and gone by the time this returns.
    pub async fn shutdown(mut self, grace: Duration) {
        self.stop_inner(grace).await;
    }

    #[cfg(test)]
    pub async fn stop(&mut self, grace: Duration) {
        self.stop_inner(grace).await;
    }

    async fn stop_inner(&mut self, grace: Duration) {
        let _ = self.periodic_stopping.send(true);
        join_or_abort(
            std::mem::take(&mut self.periodic), grace, "periodic tasks still running at shutdown were aborted",
        ).await;
        if let Some(durable) = self.durable.take() {
            durable.shutdown(grace).await;
        }
        let _ = self.ad_hoc_stopping.send(true);
        join_or_abort(
            std::mem::take(&mut self.ad_hoc), grace, "ad hoc jobs still running at shutdown were aborted",
        ).await;
    }
}

/// Waits up to `grace` for `tasks`, then aborts the ones still running and waits for them to go.
async fn join_or_abort(tasks: Vec<JoinHandle<()>>, grace: Duration, abandoned: &str) {
    let aborts: Vec<_> = tasks.iter().map(JoinHandle::abort_handle).collect();
    let mut all = std::pin::pin!(futures_util::future::join_all(tasks));
    if tokio::time::timeout(grace, all.as_mut()).await.is_err() {
        tracing::warn!("{abandoned}");
        aborts.iter().for_each(tokio::task::AbortHandle::abort);
        all.await;
    }
}

/// Starts performing jobs: the durable queue's (recovering those a previous process left
/// running once their leases expire), the ad hoc ones, and the periodic loops.
pub fn start(app: App, registry: Registry, ad_hoc: AdHocQueue, config: RunnerConfig, periodic: periodic::Loops) -> Runner {
    app.jobs.set_cable(app.cable.clone());
    app.jobs.set_app(&app);
    let workers = app.config.job_concurrency.max(1);
    let durable = campfire_jobs::start(app.db.clone(), app.jobs.queue.clone(), registry, app.clone(), config);

    let (ad_hoc_stopping, _) = watch::channel(false);
    let receiver = Arc::new(Mutex::new(ad_hoc.0));
    let ad_hoc = (0..workers).map(|_| tokio::spawn(work(receiver.clone(), ad_hoc_stopping.subscribe()))).collect();

    let (periodic_stopping, _) = watch::channel(false);
    let periodic = periodic.spawn(&app, &periodic_stopping);
    Runner { durable: Some(durable), ad_hoc_stopping, ad_hoc, periodic_stopping, periodic,
    }
}

/// An ad hoc worker: performs ad hoc jobs one at a time, until the queue has closed and drained.
async fn work(queue: Arc<Mutex<mpsc::Receiver<AdHocWork>>>, mut stopping: watch::Receiver<bool>) {
    while let Some((name, work)) = next(&queue, &mut stopping).await {
        match AssertUnwindSafe(work).catch_unwind().await {
            Ok(Ok(())) => tracing::info!(job = name, "performed"),
            Ok(Err(error)) => tracing::error!(job = name, %error, "job failed"),
            Err(panic) => tracing::error!(job = name, panic = panic_message(&*panic), "job panicked"),
        }
    }
}

/// The next ad hoc job; once the runner is stopping, the queue closes and what's left drains.
async fn next(queue: &Mutex<mpsc::Receiver<AdHocWork>>, stopping: &mut watch::Receiver<bool>) -> Option<AdHocWork> {
    let mut queue = queue.lock().await;
    tokio::select! {
        work = queue.recv() => work,
        () = stopped(stopping) => {
            queue.close();
            queue.recv().await
        }
    }
}

/// Resolves once shutdown starts. A runner dropped without a shutdown leaves its workers running.
async fn stopped(stopping: &mut watch::Receiver<bool>) {
    if stopping.wait_for(|stopping| *stopping).await.is_err() {
        std::future::pending::<()>().await;
    }
}

fn panic_message(panic: &(dyn std::any::Any + Send)) -> &str {
    panic.downcast_ref::<&str>().copied().or_else(|| panic.downcast_ref::<String>().map(String::as_str)).unwrap_or("Box<dyn Any>")
}

/// `RemoveBannedContentJob`: `user.remove_banned_content`, which destroys each of the user's
/// messages (each in its own transaction) and broadcasts its removal
/// (`reference/app/models/user/bannable.rb`, `Message::Broadcasts#broadcast_remove`).
async fn remove_banned_content(app: App, job: RemoveBannedContentJob, _: Execution) -> JobResult {
    let user_id = job.user_id;
    let messages = app
        .db
        .read(move |conn| {
            campfire_db::User::find(conn, user_id)?;
            campfire_db::Message::by_creator(conn, user_id)
        })
        .await
        .map_err(discard_missing)?;
    for message in messages {
        let (removed, room_id) = (message.clone(), message.room_id);
        app.db.write(move |tx| message.destroy(tx)).await?;
        let room = app.db.read(move |conn| campfire_db::Room::find(conn, room_id)).await?;
        app.broadcasts.message_remove(&room, &removed);
    }
    Ok(Outcome::Done)
}

/// `ActiveStorage::PurgeJob`: `blob.purge`. A blob that's gone is nothing to purge.
async fn purge_blob(app: App, job: PurgeJob, _: Execution) -> JobResult {
    crate::active_storage::purge(&app, job.blob_id).await?;
    Ok(Outcome::Done)
}

#[cfg(test)]
mod tests;
