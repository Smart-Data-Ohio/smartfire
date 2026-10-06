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
//! straight to the cable server. `Jobs::perform_later` remains test-only for best-effort ad hoc work;
//! application jobs use the durable event sink.
//!
//! [`periodic`] is `bin/periodic` and the huddle reconciler's host.

use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Duration;

use campfire_db::Job;
use campfire_jobs::{Execution, JobKind, JobResult, Outcome, RunnerConfig};
use futures_util::FutureExt as _;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, mpsc, watch};
use tokio::task::JoinHandle;

use crate::app::App;

pub mod periodic;
mod messaging;
// The integrations' job handlers, which render messages.
pub mod integrations;
pub mod agent_jobs;
pub mod attachment_processing;
pub mod huddle;
mod notifications;

use crate::queue::{AdHocQueue, AdHocWork, AnalyzeJob, PurgeJob, Registry, RemoveBannedContentJob, discard_missing};

/// Every job class the app performs: core's, and the integrations'.
pub fn registry() -> Registry {
    let mut registry = Registry::new();
    registry.register(remove_banned_content);
    registry.register(purge_blob);
    registry.register(analyze_blob);
    registry.register(quote_cards_refresh);
    attachment_processing::register(&mut registry);
    messaging::register(&mut registry);
    huddle::register(&mut registry);
    notifications::register(&mut registry);
    // Room::PushMessageJob and Bot::WebhookJob
    integrations::register_jobs(&mut registry);
    crate::mail::register(&mut registry);
    crate::integrations::google::calendar::register(&mut registry);
    crate::integrations::google::meeting_refresh::register(&mut registry);
    crate::integrations::google::entry_sync::register(&mut registry);
    crate::integrations::google::calendar_sync::register(&mut registry);
    registry
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

    #[cfg(any(test, feature = "test-support"))]
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
    app.jobs.set_delivery(crate::queue::Delivery {
        broadcast: crate::channels::sink::deliver,
        digest_notes: crate::channels::board_digests::deliver,
    });
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

async fn analyze_blob(app: App, job: AnalyzeJob, _: Execution) -> JobResult {
    crate::active_storage::analyze(&app, job.blob_id).await?;
    Ok(Outcome::Done)
}

/// `ActiveStorage::PurgeJob`: `blob.purge`. A blob that's gone is nothing to purge.
async fn purge_blob(app: App, job: PurgeJob, _: Execution) -> JobResult {
    crate::active_storage::purge(&app, job.blob_id).await?;
    Ok(Outcome::Done)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
struct QuoteCardsRefresh(campfire_db::models::message_reference::QuoteCardsRefreshJob);
impl Job for QuoteCardsRefresh {
    const CLASS: &'static str =
        <campfire_db::models::message_reference::QuoteCardsRefreshJob as Job>::CLASS;
}
impl JobKind for QuoteCardsRefresh {}
async fn quote_cards_refresh(app: App, job: QuoteCardsRefresh, _: Execution) -> JobResult {
    app.db
        .write(move |tx| {
            campfire_db::models::message_reference::refresh_quote_cards(
                tx,
                job.0.source_message_id,
                campfire_db::models::message_reference::REFRESH_MAXIMUM,
                campfire_db::models::message_reference::REFRESH_BATCH,
            )
        })
        .await?;
    Ok(Outcome::Done)
}

