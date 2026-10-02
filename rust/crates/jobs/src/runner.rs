//! Performs the queue's jobs: per queue, a loop that claims due jobs up to the queue's concurrency
//! and performs each on a task of its own; a heartbeat that extends the leases of the jobs being
//! performed; and a sweep that recovers jobs whose lease expired (their process died).

use std::collections::HashSet;
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::anyhow;
use campfire_db::{Database, Timestamp};
use futures_util::FutureExt as _;
use tokio::sync::watch;
use tokio::task::{JoinHandle, JoinSet};

use crate::kind::{Execution, JobError, JobResult, Outcome, RetryPolicy};
use crate::queue::JobQueue;
use crate::registry::Registry;
use crate::store::{self, Claimed};

/// A named queue and how many of its jobs run at once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueConfig {
    pub name: String,
    pub concurrency: usize,
}

impl QueueConfig {
    pub fn new(name: impl Into<String>, concurrency: usize) -> Self {
        Self { name: name.into(), concurrency: concurrency.max(1) }
    }
}

#[derive(Debug, Clone)]
pub struct RunnerConfig {
    pub queues: Vec<QueueConfig>,
    /// How long a claim holds a job without a heartbeat. A job whose process dies is retried
    /// this long after its last heartbeat.
    pub lease: Duration,
    /// How often the leases of the jobs being performed are extended. Well under `lease`.
    pub heartbeat: Duration,
    /// The longest an idle queue goes without looking for due jobs. Queues are woken when a job is
    /// committed and sleep until their next job is due, so this only bounds how late a job that
    /// arrived another way (enqueued by another process, recovered, or due by a clock that jumped)
    /// starts.
    pub poll: Duration,
    /// How often expired leases are looked for.
    pub recovery: Duration,
}

impl RunnerConfig {
    pub fn new(queues: Vec<QueueConfig>) -> Self {
        Self {
            queues,
            lease: Duration::from_secs(30),
            heartbeat: Duration::from_secs(10),
            poll: Duration::from_secs(5),
            recovery: Duration::from_secs(15),
        }
    }
}

/// A running runner. Dropping it leaves its tasks running; [`Runner::shutdown`] stops them.
pub struct Runner {
    id: String,
    db: Database,
    stopping: watch::Sender<bool>,
    /// Tells the queues to abort the jobs still running, once the grace period is over.
    abandoning: watch::Sender<bool>,
    queues: JoinSet<()>,
    background_stopping: watch::Sender<bool>,
    background: Vec<JoinHandle<()>>,
}

struct Shared<C> {
    id: String,
    db: Database,
    registry: Registry<C>,
    queue: JobQueue,
    context: C,
    config: RunnerConfig,
    /// The jobs being performed, whose leases the heartbeat extends.
    performing: Mutex<HashSet<i64>>,
}

/// Starts performing `registry`'s jobs from the database's queue, each handler getting a clone
/// of `context`. Recovers jobs orphaned by a previous process first (once their leases expire).
/// Must run inside a Tokio runtime.
pub fn start<C: Clone + Send + Sync + 'static>(db: Database, queue: JobQueue, registry: Registry<C>, context: C, config: RunnerConfig) -> Runner {
    let id = format!("{}-{}", std::process::id(), uuid::Uuid::new_v4().simple());
    let (stopping, _) = watch::channel(false);
    let (abandoning, _) = watch::channel(false);
    let (background_stopping, _) = watch::channel(false);
    let shared = Arc::new(Shared { id: id.clone(), db: db.clone(), registry, queue, context, config, performing: Mutex::new(HashSet::new()) });

    let background = vec![
        tokio::spawn(recovery_loop(shared.clone(), stopping.subscribe())),
        tokio::spawn(heartbeat_loop(shared.clone(), background_stopping.subscribe())),
    ];
    let mut queues = JoinSet::new();
    for queue in &shared.config.queues {
        queues.spawn(queue_loop(shared.clone(), queue.clone(), stopping.subscribe(), abandoning.subscribe()));
    }
    Runner { id, db, stopping, abandoning, queues, background_stopping, background }
}

impl Runner {
    /// This runner's `claimed_by`.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Stops claiming jobs and waits up to `grace` for the ones running to finish. Jobs still
    /// running then are abandoned: aborted (gone by the time this returns), and handed back to
    /// their queues (due now, the interrupted execution not counted as an attempt) for the next
    /// process to perform. Jobs waiting in the queues stay there.
    pub async fn shutdown(self, grace: Duration) {
        let _ = self.stopping.send(true);
        let mut queues = self.queues;
        let drained = tokio::time::timeout(grace, async { while queues.join_next().await.is_some() {} }).await.is_ok();
        if !drained {
            // Each queue aborts its running jobs and waits for them to go.
            let _ = self.abandoning.send(true);
            while queues.join_next().await.is_some() {}
        }
        let _ = self.background_stopping.send(true);
        for task in self.background {
            task.abort();
            let _ = task.await;
        }
        let id = self.id.clone();
        match self.db.write(move |tx| store::release(tx.conn(), &id, tx.now())).await {
            Ok(0) => {}
            Ok(released) => tracing::warn!(released, "jobs still running at shutdown were abandoned and handed back to their queues"),
            Err(error) => tracing::error!(%error, "handing back abandoned jobs failed; they're retried when their leases expire"),
        }
    }
}

/// One queue: claims due jobs while it has free slots, and otherwise waits to be woken (a job
/// committed), for a job to finish, or for the next job to come due.
async fn queue_loop<C: Clone + Send + Sync + 'static>(shared: Arc<Shared<C>>, queue: QueueConfig, mut stopping: watch::Receiver<bool>, mut abandoning: watch::Receiver<bool>) {
    let waker = shared.queue.waker(&queue.name);
    let mut performing = JoinSet::new();
    loop {
        if *stopping.borrow() {
            break;
        }
        let mut wait = shared.config.poll;
        let free = queue.concurrency.saturating_sub(performing.len());
        if free > 0 {
            match claim(&shared, &queue, free).await {
                Ok((claimed, next_run_at)) => {
                    let full = claimed.len() == free;
                    for job in claimed {
                        shared.performing.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).insert(job.id);
                        performing.spawn(perform(shared.clone(), job));
                    }
                    if !full && let Some(next_run_at) = next_run_at {
                        wait = wait.min(until(next_run_at, shared.db.env().now()));
                    }
                }
                Err(error) => {
                    tracing::error!(queue = queue.name, %error, "claiming jobs failed");
                    wait = wait.min(Duration::from_secs(1));
                }
            }
        }
        tokio::select! {
            () = waker.notified() => {}
            Some(_) = performing.join_next(), if !performing.is_empty() => {}
            () = tokio::time::sleep(wait) => {}
            () = stopped(&mut stopping) => break,
        }
    }
    // Shutdown: finish what's running, until the runner abandons it after the grace period.
    tokio::select! {
        () = async { while performing.join_next().await.is_some() {} } => {}
        () = stopped(&mut abandoning) => performing.shutdown().await,
    }
}

/// Claims up to `free` of `queue`'s due jobs, and reads when its next waiting job is due.
async fn claim<C: Send + Sync + 'static>(shared: &Shared<C>, queue: &QueueConfig, free: usize) -> campfire_db::Result<(Vec<Claimed>, Option<Timestamp>)> {
    let (name, concurrency, id, lease) = (queue.name.clone(), queue.concurrency, shared.id.clone(), shared.config.lease);
    shared
        .db
        .write(move |tx| {
            let now = tx.now();
            let claimed = store::claim(tx.conn(), &name, concurrency, free, &id, now.since(signed(lease)), now)?;
            let next_run_at = store::next_run_at(tx.conn(), &name)?;
            Ok((claimed, next_run_at))
        })
        .await
}

/// The first wait before retrying a job's outcome that couldn't be written, doubling up to
/// [`COMPLETION_BACKOFF_LIMIT`].
const COMPLETION_BACKOFF: Duration = Duration::from_millis(50);
const COMPLETION_BACKOFF_LIMIT: Duration = Duration::from_secs(1);

async fn perform<C: Clone + Send + Sync + 'static>(shared: Arc<Shared<C>>, job: Claimed) {
    let started = Instant::now();
    let kind = shared.registry.get(&job.class).cloned();
    let policy = kind.as_ref().map_or_else(RetryPolicy::no_retries, |kind| kind.policy);
    let execution = Execution { id: job.id, executions: job.attempts, enqueued_at: job.created_at, scheduled_at: job.run_at };
    tracing::debug!(job = job.class, id = job.id, executions = job.attempts, "performing");
    let result = match kind {
        None => Err(JobError::fail(anyhow!("no handler is registered for {}", job.class))),
        Some(kind) => match serde_json::from_str(&job.arguments) {
            Err(error) => Err(JobError::discard(anyhow!("{} arguments aren't JSON: {error}", job.class))),
            Ok(arguments) => {
                let execute = AssertUnwindSafe((kind.perform)(shared.context.clone(), crate::retry::arguments(arguments), job.version, execution)).catch_unwind();
                let finished = match policy.timeout {
                    Some(timeout) => tokio::time::timeout(timeout, execute).await,
                    None => Ok(execute.await),
                };
                match finished {
                    Ok(Ok(result)) => result,
                    Ok(Err(panic)) => Err(JobError::fail(anyhow!("panicked: {}", panic_message(&*panic)))),
                    Err(elapsed) => Err(JobError::retry(anyhow::Error::new(elapsed).context(format!("ran longer than {timeout:?}", timeout = policy.timeout.unwrap_or_default())))),
                }
            }
        },
    };
    finish(&shared, &job, &policy, result, started.elapsed()).await;
    shared.performing.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).remove(&job.id);
}

/// What becomes of a job after an execution.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Decision {
    /// Done or discarded: the row goes.
    Delete,
    /// Due again after `wait`: a retry (recording the error), or a fresh run.
    Reschedule { wait: Duration, error: Option<String>, reset_attempts: bool },
    /// Failed for good.
    Fail(String),
}

/// `retry_on`/`discard_on` for one execution's result. `random` is in `[0, 1)`, for the jitter.
pub(crate) fn decide(policy: &RetryPolicy, executions: u32, result: &JobResult, random: f64) -> Decision {
    let error = match result {
        Ok(Outcome::Done) => return Decision::Delete,
        Ok(Outcome::Again(wait)) => return Decision::Reschedule { wait: *wait, error: None, reset_attempts: true },
        Err(JobError::Discard(_)) => return Decision::Delete,
        Err(JobError::Fail(error)) => return Decision::Fail(describe(error)),
        Err(JobError::RetryGroup {
            error,
            attempts,
            discard_exhausted,
            ..
        }) => {
            return match policy
                .attempts(*attempts)
                .retry_delay(executions, None, random)
            {
                Some(wait) => Decision::Reschedule {
                    wait,
                    error: Some(describe(error)),
                    reset_attempts: false,
                },
                None if *discard_exhausted => Decision::Delete,
                None => Decision::Fail(describe(error)),
            };
        }
        Err(error) => error,
    };
    let (retryable, retry_after) = match error {
        JobError::Retry { retry_after, .. } => (true, *retry_after),
        JobError::Error(error) => ((policy.retry_on)(error), None),
        JobError::Discard(_) | JobError::Fail(_) | JobError::RetryGroup { .. } => unreachable!(),
    };
    let message = describe(error.error());
    match retryable.then(|| policy.retry_delay(executions, retry_after, random)).flatten() {
        Some(wait) => Decision::Reschedule { wait, error: Some(message), reset_attempts: false },
        None => Decision::Fail(message),
    }
}

async fn finish<C: Clone + Send + Sync + 'static>(shared: &Shared<C>, job: &Claimed, policy: &RetryPolicy, result: JobResult, elapsed: Duration) {
    let (decision, arguments) = decision_with_metadata(policy, job, &result, rand::random::<f64>());
    let (class, id, executions) = (job.class.as_str(), job.id, job.attempts);
    let elapsed_ms = elapsed.as_millis() as u64;
    match (&result, &decision) {
        (Ok(Outcome::Done), _) => tracing::info!(job = class, id, elapsed_ms, "performed"),
        (Ok(Outcome::Again(wait)), _) => tracing::info!(job = class, id, elapsed_ms, again_in = ?wait, "performed, and performs again"),
        (Err(JobError::Discard(error)), _) => tracing::warn!(job = class, id, error = describe(error), "discarded"),
        (Err(_), Decision::Reschedule { wait, error, .. }) => tracing::warn!(job = class, id, executions, retry_in = ?wait, error, "failed, retrying"),
        (Err(_), Decision::Fail(error)) => tracing::error!(job = class, id, executions, error, "failed"),
        (Err(JobError::RetryGroup { error, .. }), Decision::Delete) => tracing::error!(
            job = class,
            id,
            error = describe(error),
            "retry handler exhausted"
        ),
        (Err(_), Decision::Delete) => {}
    }

    // A write that fails is retried, backing off, for up to a lease (the heartbeat keeps the
    // claim meanwhile). Then the job is let go, and recovered once its lease expires.
    let deadline = Instant::now() + shared.config.lease;
    let mut backoff = COMPLETION_BACKOFF;
    let written = loop {
        let (runner, decision, arguments) = (shared.id.clone(), decision.clone(), arguments.clone());
        let written = shared
            .db
            .write(move |tx| {
                let now = tx.now();
                if let Some(arguments) = arguments {
                    tx.conn().execute("UPDATE background_jobs SET arguments=? WHERE id=? AND status='running' AND claimed_by=?", rusqlite::params![arguments,id,runner])?;
                }
                match decision {
                    Decision::Delete => store::delete(tx.conn(), id, &runner),
                    Decision::Reschedule { wait, error, reset_attempts } => {
                        store::reschedule(tx.conn(), id, &runner, now.since(signed(wait)), error.as_deref(), reset_attempts, now)
                    }
                    Decision::Fail(error) => store::fail(tx.conn(), id, &runner, &error, now),
                }
            })
            .await;
        match written {
            Err(error) if Instant::now() + backoff < deadline => {
                tracing::warn!(job = class, id, %error, retry_in = ?backoff, "recording the job's outcome failed, retrying");
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(COMPLETION_BACKOFF_LIMIT);
            }
            written => break written,
        }
    };
    match written {
        Ok(true) => {
            if let (Err(JobError::RetryGroup {error,discard_exhausted:true,..}),Decision::Delete)=(&result,&decision)
                && let Some(callback)=shared.registry.get(class).and_then(|kind|kind.exhausted.as_ref())
            {
                callback(shared.context.clone(),crate::retry::arguments(serde_json::from_str(&job.arguments).unwrap_or_default()),job.version,error);
            }
        }
        // Its lease expired and it was recovered: the other claim's outcome stands.
        Ok(false) => tracing::warn!(job = class, id, "the job's claim was lost while it ran; its outcome wasn't recorded"),
        Err(error) => tracing::error!(job = class, id, %error, "recording the job's outcome failed; it's retried when its lease expires"),
    }
    // The queue claims again when this task ends, so a retry that's already due runs then.
}

pub(crate) fn decision_with_metadata(
    policy: &RetryPolicy,
    job: &Claimed,
    result: &JobResult,
    random: f64,
) -> (Decision, Option<String>) {
    let mut state =
        crate::retry::State::from_value(serde_json::from_str(&job.arguments).unwrap_or_default());
    match result {
        Err(JobError::RetryGroup { key, .. }) => {
            let count = state.increment(key);
            (decide(policy, count, result, random), Some(state.encode()))
        }
        Ok(Outcome::Again(_)) if !state.counts.is_empty() => (
            decide(policy, job.attempts, result, random),
            Some(state.arguments.to_string()),
        ),
        _ => (decide(policy, job.attempts, result, random), None),
    }
}

/// Extends the leases of the jobs being performed, until the runner has stopped.
async fn heartbeat_loop<C: Send + Sync + 'static>(shared: Arc<Shared<C>>, mut stopping: watch::Receiver<bool>) {
    loop {
        tokio::select! {
            () = tokio::time::sleep(shared.config.heartbeat) => {}
            () = stopped(&mut stopping) => return,
        }
        let ids: Vec<i64> = shared.performing.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).iter().copied().collect();
        if ids.is_empty() {
            continue;
        }
        let (runner, lease) = (shared.id.clone(), shared.config.lease);
        if let Err(error) = shared.db.write(move |tx| store::heartbeat(tx.conn(), &runner, &ids, tx.now().since(signed(lease)), tx.now())).await {
            tracing::error!(%error, "extending job leases failed");
        }
    }
}

/// Recovers jobs whose lease expired, now (at boot) and then every `recovery` interval.
async fn recovery_loop<C: Send + Sync + 'static>(shared: Arc<Shared<C>>, mut stopping: watch::Receiver<bool>) {
    loop {
        match recover(&shared).await {
            Ok(queues) => {
                for queue in queues {
                    shared.queue.wake_queue(&queue);
                }
            }
            Err(error) => tracing::error!(%error, "recovering jobs with expired leases failed"),
        }
        tokio::select! {
            () = tokio::time::sleep(shared.config.recovery) => {}
            () = stopped(&mut stopping) => return,
        }
    }
}

/// Takes back jobs whose lease expired: their process stopped heartbeating, mid-execution, or
/// this runner let one go when its outcome couldn't be written. That
/// execution counts as an attempt, so a job with attempts left is due again now, and one without
/// (a job that kills its process every time, or one that must not run twice) fails for good.
/// Returns the queues with jobs due again.
async fn recover<C: Send + Sync + 'static>(shared: &Shared<C>) -> campfire_db::Result<Vec<String>> {
    let runner = shared.id.clone();
    let performing: Vec<i64> = shared.performing.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).iter().copied().collect();
    let orphans = shared.db.write(move |tx| store::orphans(tx.conn(), &runner, &performing, tx.now())).await?;
    let mut due = Vec::new();
    for orphan in orphans {
        let attempts = shared.registry.get(&orphan.class).map_or(1, |kind| kind.policy.attempts);
        let state = crate::retry::State::from_value(serde_json::from_str(&orphan.arguments).unwrap_or_default());
        let interrupted = state.interrupted_executions(orphan.attempts);
        let retry = interrupted < attempts;
        let error = format!("the process performing it stopped (execution {interrupted} of {attempts})");
        let (id, message) = (orphan.id, error.clone());
        if shared.db.write(move |tx| store::recover(tx.conn(), id, retry, &message, tx.now())).await? {
            if retry {
                tracing::warn!(job = orphan.class, id, error, "recovered an orphaned job");
                due.push(orphan.queue);
            } else {
                tracing::error!(job = orphan.class, id, error, "an orphaned job has no attempts left, failed");
            }
        }
    }
    due.sort();
    due.dedup();
    Ok(due)
}

/// How long from `now` until `at` (zero if it's past).
fn until(at: Timestamp, now: Timestamp) -> Duration {
    Duration::from_micros(at.as_microsecond().saturating_sub(now.as_microsecond()).max(0) as u64)
}

fn signed(duration: Duration) -> jiff::SignedDuration {
    jiff::SignedDuration::try_from(duration).unwrap_or(jiff::SignedDuration::MAX)
}

fn describe(error: &anyhow::Error) -> String {
    format!("{error:#}")
}

/// Resolves once shutdown starts. A dropped runner never stops its tasks.
async fn stopped(stopping: &mut watch::Receiver<bool>) {
    if stopping.wait_for(|stopping| *stopping).await.is_err() {
        std::future::pending::<()>().await;
    }
}

fn panic_message(panic: &(dyn std::any::Any + Send)) -> &str {
    panic.downcast_ref::<&str>().copied().or_else(|| panic.downcast_ref::<String>().map(String::as_str)).unwrap_or("Box<dyn Any>")
}
