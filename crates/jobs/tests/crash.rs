//! A process that dies mid-job: the job is retried by the next process, once the dead one's lease
//! has expired and not before. The test re-runs its own binary as the process that dies, and
//! kills it (SIGKILL) while it performs the job.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use campfire_db::{Database, Job, JobRequest, TestClock, Timestamp};
use campfire_jobs::inspect;
use campfire_jobs::{Execution, JobKind, JobQueue, Outcome, QueueConfig, RUNNING, Registry, RunnerConfig, start};
use serde::{Deserialize, Serialize};

const CHILD: &str = "CAMPFIRE_JOBS_CRASH_CHILD";
const T0: &str = "2026-09-29 12:00:00";

#[derive(Debug, Serialize, Deserialize)]
struct Deliver {
    message_id: i64,
}

impl Job for Deliver {
    const CLASS: &'static str = "DeliverJob";
}

impl JobKind for Deliver {}

fn config() -> RunnerConfig {
    let mut config = RunnerConfig::new(vec![QueueConfig::new("default", 2)]);
    config.lease = Duration::from_secs(30);
    config.poll = Duration::from_millis(50);
    config.recovery = Duration::from_millis(50);
    config
}

fn open(dir: &Path, clock: &TestClock) -> Database {
    let env = campfire_db::Env { clock: Arc::new(clock.clone()), ..Default::default() };
    Database::open(campfire_db::Config::new(dir.join("jobs.sqlite3")), env).unwrap()
}

/// The process that dies: performs the job, says so, and hangs until it's killed.
#[test]
fn crash_child() {
    let Some(dir) = std::env::var_os(CHILD).map(PathBuf::from) else { return };
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async move {
        let clock = TestClock::frozen_at(Timestamp::parse_db(T0).unwrap());
        let db = open(&dir, &clock);
        let mut registry = Registry::new();
        let started = dir.join("started");
        registry.register(move |(), _: Deliver, execution: Execution| {
            let started = started.clone();
            async move {
                std::fs::write(&started, execution.executions.to_string()).unwrap();
                std::future::pending::<()>().await;
                Ok(Outcome::Done)
            }
        });
        let queue = JobQueue::new(&registry, &config()).unwrap();
        let _runner = start(db, queue, registry, (), config());
        std::future::pending::<()>().await;
    });
}

#[tokio::test]
async fn a_job_whose_process_dies_is_retried_after_its_lease_expires() {
    let dir = tempfile::tempdir().unwrap();
    let clock = TestClock::frozen_at(Timestamp::parse_db(T0).unwrap());
    let db = open(dir.path(), &clock);
    let (performed, mut performed_rx) = tokio::sync::mpsc::unbounded_channel();
    let mut registry = Registry::new();
    registry.register(move |(), job: Deliver, execution: Execution| {
        let performed = performed.clone();
        async move {
            let _ = performed.send((job.message_id, execution.executions));
            Ok(Outcome::Done)
        }
    });
    let queue = JobQueue::new(&registry, &config()).unwrap();
    let id = queue.perform_later(&db, JobRequest::new(&Deliver { message_id: 42 })).await.unwrap();

    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "crash_child", "--nocapture", "--test-threads=1"])
        .env(CHILD, dir.path())
        .spawn()
        .unwrap();
    let started = dir.path().join("started");
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while !started.exists() {
        assert!(std::time::Instant::now() < deadline, "the child never started the job");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(std::fs::read_to_string(&started).unwrap(), "1");
    child.kill().unwrap(); // SIGKILL: no shutdown, no hand-back
    child.wait().unwrap();

    let job = db.read(move |conn| inspect::find(conn, id)).await.unwrap().unwrap();
    assert_eq!((job.status.as_str(), job.attempts), (RUNNING, 1));
    assert_eq!(job.lease_expires_at, Timestamp::parse_db("2026-09-29 12:00:30"));
    let dead = job.claimed_by.unwrap();

    // The next process boots while the dead one's lease is live: it leaves the job alone.
    let runner = start(db.clone(), queue, registry, (), config());
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(performed_rx.try_recv().is_err(), "retried while the lease was live");
    let job = db.read(move |conn| inspect::find(conn, id)).await.unwrap().unwrap();
    assert_eq!((job.status.as_str(), job.claimed_by.as_deref()), (RUNNING, Some(dead.as_str())));

    clock.travel(jiff::SignedDuration::from_secs(29));
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(performed_rx.try_recv().is_err(), "retried a second early");

    clock.travel(jiff::SignedDuration::from_secs(1));
    let (message_id, executions) = tokio::time::timeout(Duration::from_secs(10), performed_rx.recv()).await.expect("retried").unwrap();
    assert_eq!((message_id, executions), (42, 2), "the lost execution counts as an attempt");
    runner.shutdown(Duration::from_secs(5)).await;
    assert!(db.read(move |conn| inspect::find(conn, id)).await.unwrap().is_none(), "done");
}
