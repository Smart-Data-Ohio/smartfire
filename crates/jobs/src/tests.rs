//! The queue and the runner against a real SQLite database with the app's schema.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use campfire_db::{Database, Event, EventSink, Job, JobRequest, TestClock, Timestamp, Tx};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::inspect::{self, JobRow};
use crate::runner::{Decision, decide};
use crate::*;

mod periodic_test;
mod runner_test;
mod ws8_messaging_test;

/// Enqueues `Event::Job`s on the queue, as the app's sink does.
pub(crate) struct QueueSink {
    queue: JobQueue,
    /// Fails every persist, to show that the write fails with it.
    broken: std::sync::atomic::AtomicBool,
}

impl EventSink for QueueSink {
    fn emit(&self, event: Event) {
        if let Event::Job(request) = event {
            self.queue.wake(request.class);
        }
    }

    fn persist(&self, tx: &Tx<'_>, event: &Event) -> campfire_db::Result<()> {
        if self.broken.load(Ordering::SeqCst) {
            return Err(campfire_db::Error::Other("the queue is broken".into()));
        }
        if let Event::Job(request) = event {
            self.queue.enqueue(tx, request)?;
        }
        Ok(())
    }
}

pub(crate) struct Harness {
    pub db: Database,
    pub clock: TestClock,
    pub queue: JobQueue,
    pub sink: Arc<QueueSink>,
    _dir: tempfile::TempDir,
}

pub(crate) fn config() -> RunnerConfig {
    RunnerConfig {
        queues: vec![QueueConfig::new("default", 3), QueueConfig::new("slack_import", 1)],
        lease: Duration::from_secs(30),
        heartbeat: Duration::from_secs(10),
        poll: Duration::from_millis(50),
        recovery: Duration::from_secs(15),
    }
}

pub(crate) const T0: &str = "2026-09-29 12:00:00";

pub(crate) fn at(text: &str) -> Timestamp {
    Timestamp::parse_db(text).unwrap()
}

/// A database with the app's schema, whose sink enqueues on a queue for `registry`, and a clock
/// frozen at [`T0`].
pub(crate) fn harness<C: Send + 'static>(registry: &Registry<C>, config: &RunnerConfig) -> Harness {
    let dir = tempfile::tempdir().unwrap();
    open(registry, config, dir)
}

pub(crate) fn open<C: Send + 'static>(registry: &Registry<C>, config: &RunnerConfig, dir: tempfile::TempDir) -> Harness {
    let clock = TestClock::frozen_at(at(T0));
    let queue = JobQueue::new(registry, config).unwrap();
    let sink = Arc::new(QueueSink { queue: queue.clone(), broken: Default::default() });
    let env = campfire_db::Env { clock: Arc::new(clock.clone()), sink: sink.clone(), ..Default::default() };
    let mut db_config = campfire_db::Config::new(dir.path().join("test.sqlite3"));
    db_config.readers = 2;
    let db = Database::open(db_config, env).unwrap();
    Harness { db, clock, queue, sink, _dir: dir }
}

impl Harness {
    pub async fn enqueue<J: Job>(&self, job: J) -> i64 {
        self.enqueue_request(JobRequest::new(&job)).await
    }

    pub async fn enqueue_request(&self, request: JobRequest) -> i64 {
        self.db
            .write(move |tx| {
                tx.emit_after_commit(Event::Job(request));
                Ok(tx.conn().last_insert_rowid())
            })
            .await
            .unwrap()
    }

    pub fn jobs(&self) -> Vec<JobRow> {
        self.db.read_blocking(inspect::all).unwrap()
    }

    pub fn job(&self, id: i64) -> Option<JobRow> {
        self.db.read_blocking(|conn| inspect::find(conn, id)).unwrap()
    }

    pub fn travel(&self, seconds: i64) {
        self.clock.travel(jiff::SignedDuration::from_secs(seconds));
    }

    /// Waits (up to 10 s) for `condition` on the jobs table.
    pub async fn wait_for(&self, what: &str, condition: impl Fn(&[JobRow]) -> bool) -> Vec<JobRow> {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        loop {
            let jobs = self.jobs();
            if condition(&jobs) {
                return jobs;
            }
            assert!(tokio::time::Instant::now() < deadline, "timed out waiting for {what}: {jobs:#?}");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}

/// A job that reports its number, and its execution, to the test.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Echo {
    pub n: i64,
}

impl Job for Echo {
    const CLASS: &'static str = "EchoJob";
}

impl JobKind for Echo {}

/// The performed jobs' numbers and executions, in the order they started.
pub(crate) type Performed = mpsc::UnboundedReceiver<(i64, u32)>;

pub(crate) fn echo_registry() -> (Registry<()>, Performed) {
    let (performed, receiver) = mpsc::unbounded_channel();
    let mut registry = Registry::new();
    registry.register(move |(), job: Echo, execution: Execution| {
        let performed = performed.clone();
        async move {
            let _ = performed.send((job.n, execution.executions));
            Ok(Outcome::Done)
        }
    });
    (registry, receiver)
}

pub(crate) async fn next(performed: &mut Performed) -> (i64, u32) {
    tokio::time::timeout(Duration::from_secs(10), performed.recv()).await.expect("a job was performed").unwrap()
}

/// Concurrency seen by a handler: how many run at once, and the most that ever did.
#[derive(Default)]
pub(crate) struct Concurrency {
    running: AtomicUsize,
    pub most: AtomicUsize,
    pub done: AtomicUsize,
}

impl Concurrency {
    pub async fn hold(&self, for_: Duration) {
        let now = self.running.fetch_add(1, Ordering::SeqCst) + 1;
        self.most.fetch_max(now, Ordering::SeqCst);
        tokio::time::sleep(for_).await;
        self.running.fetch_sub(1, Ordering::SeqCst);
        self.done.fetch_add(1, Ordering::SeqCst);
    }
}

pub(crate) type Log = Arc<Mutex<Vec<String>>>;

mod ws8_slash_test;

mod google_retry_test;
