use std::sync::atomic::{AtomicBool, AtomicUsize};

use super::*;

// --- Enqueueing ------------------------------------------------------------------------------------

#[tokio::test]
async fn a_job_is_enqueued_in_its_writes_transaction() {
    let (registry, _) = echo_registry();
    let h = harness(&registry, &config());
    let id = h.enqueue(Echo { n: 1 }).await;
    let job = h.job(id).unwrap();
    assert_eq!((job.queue.as_str(), job.class.as_str(), job.status.as_str()), ("default", "EchoJob", READY));
    assert_eq!(job.arguments, serde_json::json!({"n": 1}));
    assert_eq!((job.payload_version, job.attempts), (1, 0));
    assert_eq!((job.run_at, job.created_at), (at(T0), at(T0)));
}

#[tokio::test]
async fn a_write_that_rolls_back_enqueues_nothing() {
    let (registry, _) = echo_registry();
    let h = harness(&registry, &config());
    let result = h
        .db
        .write(|tx| {
            tx.emit_after_commit(Event::job(&Echo { n: 1 }));
            tx.emit_now(Event::job(&Echo { n: 2 }));
            Err::<(), _>(campfire_db::Error::Other("validation failed".into()))
        })
        .await;
    assert!(result.is_err());
    assert!(h.jobs().is_empty(), "{:?}", h.jobs());
}

/// A job whose row can't be written fails the write that enqueued it, and so rolls back the rest
/// of that write: the write and its jobs commit together or not at all.
#[tokio::test]
async fn a_job_that_cant_be_enqueued_fails_its_write() {
    let (registry, _) = echo_registry();
    let h = harness(&registry, &config());
    h.sink.broken.store(true, Ordering::SeqCst);
    let result = h
        .db
        .write(|tx| {
            tx.conn().execute_batch("CREATE TABLE side_effects (id integer)")?;
            tx.emit_after_commit(Event::job(&Echo { n: 1 }));
            Ok(())
        })
        .await;
    assert!(result.is_err());
    let tables: i64 = h.db.read_blocking(|conn| Ok(conn.query_row("SELECT count(*) FROM sqlite_master WHERE name = 'side_effects'", [], |r| r.get(0))?)).unwrap();
    assert_eq!(tables, 0, "the write rolled back");
}

#[tokio::test]
async fn a_delayed_job_is_due_after_its_wait() {
    let (registry, _) = echo_registry();
    let h = harness(&registry, &config());
    let id = h.enqueue_request(JobRequest::new(&Echo { n: 1 }).wait(Duration::from_secs(90))).await;
    assert_eq!(h.job(id).unwrap().run_at, at("2026-09-29 12:01:30"));
}

/// The job only becomes visible to the runner when the write that enqueued it commits, so it
/// sees everything that write did (`after_commit { perform_later }`), and the runner is woken
/// after the commit rather than finding it at its next poll.
#[tokio::test]
async fn a_job_runs_after_its_write_commits_and_sees_it() {
    let (seen, mut seen_rx) = mpsc::unbounded_channel();
    let mut registry = Registry::new();
    let db_slot: Arc<std::sync::OnceLock<Database>> = Default::default();
    let db_for_job = db_slot.clone();
    registry.register(move |(), job: Echo, _: Execution| {
        let (seen, db) = (seen.clone(), db_for_job.get().unwrap().clone());
        async move {
            let rows: i64 = db.read(move |conn| Ok(conn.query_row("SELECT count(*) FROM triggers WHERE n = ?", [job.n], |r| r.get(0))?)).await?;
            let _ = seen.send((job.n, rows));
            Ok(Outcome::Done)
        }
    });
    let mut config = config();
    config.poll = Duration::from_secs(60); // only a wake can start it in time
    let h = harness(&registry, &config);
    assert!(db_slot.set(h.db.clone()).is_ok());
    h.db.write(|tx| Ok(tx.conn().execute_batch("CREATE TABLE triggers (n integer)")?)).await.unwrap();
    let runner = start(h.db.clone(), h.queue.clone(), registry, (), config);
    // Let the queue find nothing and go to sleep: only the commit's wake can start the job now.
    tokio::time::sleep(Duration::from_millis(200)).await;

    let committed = Arc::new(Mutex::new(None));
    let committed_at = committed.clone();
    h.db
        .write(move |tx| {
            tx.emit_after_commit(Event::job(&Echo { n: 7 }));
            // The runner can't claim it while this write holds the writer.
            std::thread::sleep(Duration::from_millis(200));
            tx.conn().execute("INSERT INTO triggers (n) VALUES (7)", [])?;
            tx.after_commit(move |_| {
                *committed_at.lock().unwrap() = Some(std::time::Instant::now());
                Ok(())
            });
            Ok(())
        })
        .await
        .unwrap();
    let (n, rows) = tokio::time::timeout(Duration::from_secs(5), seen_rx.recv()).await.expect("woken, not polled").unwrap();
    assert_eq!((n, rows), (7, 1), "the job saw its write's row");
    assert!(committed.lock().unwrap().is_some());
    h.wait_for("the job's row to go", |jobs| jobs.is_empty()).await;
    runner.shutdown(Duration::from_secs(5)).await;
}

// --- Retries -------------------------------------------------------------------------------------------

/// Fails transiently until its `succeed_on`th execution.
#[derive(Debug, Serialize, Deserialize)]
struct Flaky {
    succeed_on: u32,
}

impl Job for Flaky {
    const CLASS: &'static str = "FlakyJob";
}

impl JobKind for Flaky {
    fn retry_policy() -> RetryPolicy {
        RetryPolicy::application_job().attempts(3).wait(Wait::PolynomiallyLonger { jitter: 0.0 })
    }
}

fn flaky_registry() -> (Registry<()>, Performed) {
    let (performed, receiver) = mpsc::unbounded_channel();
    let mut registry = Registry::new();
    registry.register(move |(), job: Flaky, execution: Execution| {
        let performed = performed.clone();
        async move {
            let _ = performed.send((i64::from(job.succeed_on), execution.executions));
            if execution.executions < job.succeed_on {
                return Err(std::io::Error::new(std::io::ErrorKind::TimedOut, format!("timed out on execution {}", execution.executions)).into());
            }
            Ok(Outcome::Done)
        }
    });
    (registry, receiver)
}

/// `retry_on ..., wait: :polynomially_longer`: 3 s after the first failure, 18 s after the second,
/// by the clock, and not before.
#[tokio::test]
async fn transient_failures_retry_with_polynomial_backoff() {
    let (registry, mut performed) = flaky_registry();
    let h = harness(&registry, &config());
    let runner = start(h.db.clone(), h.queue.clone(), registry, (), config());
    let id = h.enqueue(Flaky { succeed_on: 3 }).await;

    assert_eq!(next(&mut performed).await, (3, 1));
    let job = h.wait_for("the first retry", |jobs| jobs[0].status == READY && jobs[0].attempts == 1).await.remove(0);
    assert_eq!(job.run_at, at("2026-09-29 12:00:03"));
    assert_eq!(job.last_error.as_deref(), Some("timed out on execution 1"));

    // Not due yet: polls come and go without it.
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(performed.try_recv().is_err());
    h.travel(2);
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(performed.try_recv().is_err(), "still 1 s early");

    h.travel(1);
    assert_eq!(next(&mut performed).await, (3, 2));
    let job = h.wait_for("the second retry", |jobs| jobs[0].attempts == 2 && jobs[0].status == READY).await.remove(0);
    assert_eq!(job.run_at, at("2026-09-29 12:00:21"), "18 s after the second failure");

    h.travel(18);
    assert_eq!(next(&mut performed).await, (3, 3));
    h.wait_for("the job to finish", |jobs| jobs.is_empty()).await;
    assert!(h.job(id).is_none());
    runner.shutdown(Duration::from_secs(5)).await;
}

/// Out of attempts, a job is kept as failed, with its error, for inspection and a manual retry.
#[tokio::test]
async fn a_job_out_of_attempts_is_kept_as_failed() {
    let (registry, mut performed) = flaky_registry();
    let h = harness(&registry, &config());
    let runner = start(h.db.clone(), h.queue.clone(), registry, (), config());
    let id = h.enqueue(Flaky { succeed_on: 10 }).await;
    for (executions, wait) in [(1, 3), (2, 18)] {
        assert_eq!(next(&mut performed).await, (10, executions));
        h.wait_for("the retry", |jobs| jobs[0].status == READY && jobs[0].attempts == executions).await;
        h.travel(wait);
    }
    assert_eq!(next(&mut performed).await, (10, 3));
    let job = h.wait_for("the failure", |jobs| jobs[0].status == FAILED).await.remove(0);
    assert_eq!(job.attempts, 3);
    assert_eq!(job.last_error.as_deref(), Some("timed out on execution 3"));
    assert_eq!(job.failed_at, Some(at("2026-09-29 12:00:21")));
    assert_eq!(job.claimed_by, None);
    let failed = h.db.read_blocking(|conn| inspect::with_status(conn, FAILED)).unwrap();
    assert_eq!(failed.iter().map(|job| job.id).collect::<Vec<_>>(), [id]);

    // Retried by hand, it gets all its attempts back.
    assert!(h.db.write(move |tx| inspect::retry_failed(tx, id)).await.unwrap());
    assert_eq!(next(&mut performed).await, (10, 1));
    runner.shutdown(Duration::from_secs(5)).await;
}

/// An error the class doesn't retry fails the job at once.
#[tokio::test]
async fn an_error_outside_retry_on_fails_the_job_at_once() {
    let mut registry = Registry::new();
    registry.register(|(), _: Echo, _: Execution| async { Err(anyhow::anyhow!("undefined method 'deliver' for nil").into()) });
    let h = harness(&registry, &config());
    let runner = start(h.db.clone(), h.queue.clone(), registry, (), config());
    h.enqueue(Echo { n: 1 }).await;
    let job = h.wait_for("the failure", |jobs| jobs.first().is_some_and(|job| job.status == FAILED)).await.remove(0);
    assert_eq!((job.attempts, job.last_error.as_deref()), (1, Some("undefined method 'deliver' for nil")));
    runner.shutdown(Duration::from_secs(5)).await;
}

#[derive(Debug, Serialize, Deserialize)]
struct RateLimited {
    retry_after: u64,
}

impl Job for RateLimited {
    const CLASS: &'static str = "RateLimitedJob";
}

impl JobKind for RateLimited {}

/// An endpoint's `Retry-After` replaces the backoff, up to an hour.
#[tokio::test]
async fn retry_after_sets_the_retry_and_is_capped_at_an_hour() {
    let mut registry = Registry::new();
    registry.register(|(), job: RateLimited, _: Execution| async move {
        Err(JobError::retry_after(anyhow::anyhow!("429 Too Many Requests"), Duration::from_secs(job.retry_after)))
    });
    let h = harness(&registry, &config());
    let runner = start(h.db.clone(), h.queue.clone(), registry, (), config());
    let polite = h.enqueue(RateLimited { retry_after: 120 }).await;
    let greedy = h.enqueue(RateLimited { retry_after: 86_400 }).await;
    h.wait_for("both retries", |jobs| jobs.iter().all(|job| job.attempts == 1 && job.status == READY)).await;
    assert_eq!(h.job(polite).unwrap().run_at, at("2026-09-29 12:02:00"));
    assert_eq!(h.job(greedy).unwrap().run_at, at("2026-09-29 13:00:00"));
    assert_eq!(h.job(polite).unwrap().last_error.as_deref(), Some("429 Too Many Requests"));
    runner.shutdown(Duration::from_secs(5)).await;
}

/// `discard_on`: the job goes, and isn't kept as failed.
#[tokio::test]
async fn a_discarded_job_is_deleted() {
    let mut registry = Registry::new();
    registry.register(|(), _: Echo, _: Execution| async { Err(JobError::discard(anyhow::anyhow!("Couldn't find Message"))) });
    let h = harness(&registry, &config());
    let runner = start(h.db.clone(), h.queue.clone(), registry, (), config());
    h.enqueue(Echo { n: 1 }).await;
    h.wait_for("the job to go", |jobs| jobs.is_empty()).await;
    runner.shutdown(Duration::from_secs(5)).await;
}

/// `Outcome::Again`: the job re-enqueues itself (the Slack step job's `:continue`, or waiting out
/// a rate limit), as a fresh job with all its attempts.
#[tokio::test]
async fn a_job_can_run_again_as_a_fresh_job() {
    let (performed, mut performed_rx) = mpsc::unbounded_channel();
    let mut registry = Registry::new();
    registry.register(move |(), _: Echo, execution: Execution| {
        let performed = performed.clone();
        async move {
            let _ = performed.send(execution.executions);
            Ok(Outcome::Again(Duration::from_secs(25)))
        }
    });
    let h = harness(&registry, &config());
    let runner = start(h.db.clone(), h.queue.clone(), registry, (), config());
    let id = h.enqueue(Echo { n: 1 }).await;
    assert_eq!(tokio::time::timeout(Duration::from_secs(5), performed_rx.recv()).await.unwrap(), Some(1));
    let job = h.wait_for("the rerun", |jobs| jobs[0].status == READY).await.remove(0);
    assert_eq!((job.id, job.attempts, job.run_at), (id, 0, at("2026-09-29 12:00:25")));
    h.travel(25);
    assert_eq!(tokio::time::timeout(Duration::from_secs(5), performed_rx.recv()).await.unwrap(), Some(1), "all its attempts again");
    runner.shutdown(Duration::from_secs(5)).await;
}

#[derive(Debug, Serialize, Deserialize)]
struct Renamed {
    message_id: i64,
}

impl Job for Renamed {
    const CLASS: &'static str = "RenamedJob";
}

impl JobKind for Renamed {
    const VERSION: u32 = 2;

    fn upgrade(version: u32, mut arguments: serde_json::Value) -> Result<serde_json::Value, String> {
        match version {
            1 => {
                let id = arguments["id"].take();
                Ok(serde_json::json!({"message_id": id}))
            }
            version => Err(format!("unknown payload version {version}")),
        }
    }
}

/// Payloads carry their version: an old one is upgraded, and one that can't be read is
/// discarded (`discard_on ActiveJob::DeserializationError`).
#[tokio::test]
async fn old_payloads_are_upgraded_and_unreadable_ones_discarded() {
    let (performed, mut performed_rx) = mpsc::unbounded_channel();
    let mut registry = Registry::new();
    registry.register(move |(), job: Renamed, _: Execution| {
        let performed = performed.clone();
        async move {
            let _ = performed.send(job.message_id);
            Ok(Outcome::Done)
        }
    });
    let h = harness(&registry, &config());
    let rows = [(1, r#"{"id": 41}"#), (2, r#"{"message_id": 42}"#), (3, r#"{"message_id": 43}"#), (2, r#"{"message": "not an id"}"#), (2, "not json")];
    h.db.write(move |tx| {
        for (version, arguments) in rows {
            tx.conn().execute(
                "INSERT INTO background_jobs (queue_name, job_class, arguments, payload_version, run_at, created_at, updated_at) VALUES ('default', 'RenamedJob', ?1, ?2, ?3, ?3, ?3)",
                rusqlite::params![arguments, version, at(T0)],
            )?;
        }
        Ok(())
    })
    .await
    .unwrap();
    let runner = start(h.db.clone(), h.queue.clone(), registry, (), config());
    let mut ids = vec![];
    for _ in 0..2 {
        ids.push(tokio::time::timeout(Duration::from_secs(5), performed_rx.recv()).await.unwrap().unwrap());
    }
    ids.sort();
    assert_eq!(ids, [41, 42]);
    h.wait_for("every job to go", |jobs| jobs.is_empty()).await;
    runner.shutdown(Duration::from_secs(5)).await;
}

#[tokio::test]
async fn a_job_without_a_handler_fails() {
    let (registry, _) = echo_registry();
    let h = harness(&registry, &config());
    let runner = start(h.db.clone(), h.queue.clone(), registry, (), config());
    #[derive(Serialize, Deserialize)]
    struct Unknown {}
    impl Job for Unknown {
        const CLASS: &'static str = "UnknownJob";
    }
    h.enqueue(Unknown {}).await;
    let job = h.wait_for("the failure", |jobs| jobs[0].status == FAILED).await.remove(0);
    assert_eq!(job.last_error.as_deref(), Some("no handler is registered for UnknownJob"));
    runner.shutdown(Duration::from_secs(5)).await;
}

#[tokio::test]
async fn a_panicking_job_fails_and_its_queue_carries_on() {
    let (performed, mut performed_rx) = mpsc::unbounded_channel();
    let mut registry = Registry::new();
    registry.register(move |(), job: Echo, _: Execution| {
        let performed = performed.clone();
        async move {
            assert!(job.n != 1, "kaboom");
            let _ = performed.send(job.n);
            Ok(Outcome::Done)
        }
    });
    let mut config = config();
    config.queues = vec![QueueConfig::new("default", 1)];
    let h = harness(&registry, &config);
    let runner = start(h.db.clone(), h.queue.clone(), registry, (), config);
    h.enqueue(Echo { n: 1 }).await;
    h.enqueue(Echo { n: 2 }).await;
    assert_eq!(tokio::time::timeout(Duration::from_secs(5), performed_rx.recv()).await.unwrap(), Some(2));
    let job = h.wait_for("the failure", |jobs| jobs.len() == 1 && jobs[0].status == FAILED).await.remove(0);
    assert_eq!(job.last_error.as_deref(), Some("panicked: kaboom"));
    runner.shutdown(Duration::from_secs(5)).await;
}

#[derive(Debug, Serialize, Deserialize)]
struct Hanging {}

impl Job for Hanging {
    const CLASS: &'static str = "HangingJob";
}

impl JobKind for Hanging {
    fn retry_policy() -> RetryPolicy {
        RetryPolicy::application_job().timeout(Duration::from_millis(100)).wait(Wait::Fixed { wait: Duration::from_secs(60), jitter: 0.0 })
    }
}

#[tokio::test]
async fn a_job_past_its_timeout_is_retried() {
    let mut registry = Registry::new();
    registry.register(|(), _: Hanging, _: Execution| async {
        std::future::pending::<()>().await;
        Ok(Outcome::Done)
    });
    let h = harness(&registry, &config());
    let runner = start(h.db.clone(), h.queue.clone(), registry, (), config());
    h.enqueue(Hanging {}).await;
    let job = h.wait_for("the retry", |jobs| jobs[0].status == READY && jobs[0].attempts == 1).await.remove(0);
    assert_eq!(job.run_at, at("2026-09-29 12:01:00"));
    assert!(job.last_error.unwrap().starts_with("ran longer than 100ms"));
    runner.shutdown(Duration::from_secs(5)).await;
}

// --- Queues ------------------------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
struct SlackStep {
    n: i64,
}

impl Job for SlackStep {
    const CLASS: &'static str = "SlackImport::StepJob";
}

impl JobKind for SlackStep {
    const QUEUE: &'static str = "slack_import";
}

/// `slack_import: 1` holds across runners: the queue's concurrency is counted in the database,
/// so two runners (an old and a new process during a deploy) never run two of its jobs at once.
/// The default queue runs up to its concurrency in parallel meanwhile.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_serial_queue_runs_one_job_at_a_time_across_runners() {
    let serial = Arc::new(Concurrency::default());
    let parallel = Arc::new(Concurrency::default());
    let registry = || {
        let (serial, parallel) = (serial.clone(), parallel.clone());
        let mut registry = Registry::new();
        registry.register(move |(), _: SlackStep, _: Execution| {
            let serial = serial.clone();
            async move {
                serial.hold(Duration::from_millis(30)).await;
                Ok(Outcome::Done)
            }
        });
        registry.register(move |(), _: Echo, _: Execution| {
            let parallel = parallel.clone();
            async move {
                parallel.hold(Duration::from_millis(100)).await;
                Ok(Outcome::Done)
            }
        });
        registry
    };
    let config = config();
    let h = harness(&registry(), &config);
    for n in 0..12 {
        h.enqueue(SlackStep { n }).await;
        h.enqueue(Echo { n }).await;
    }
    let first = start(h.db.clone(), h.queue.clone(), registry(), (), config.clone());
    let second = start(h.db.clone(), JobQueue::new(&registry(), &config).unwrap(), registry(), (), config.clone());
    h.wait_for("every job to finish", |jobs| jobs.is_empty()).await;
    assert_eq!(serial.done.load(Ordering::SeqCst), 12);
    assert_eq!(serial.most.load(Ordering::SeqCst), 1, "slack_import ran two jobs at once");
    assert_eq!(parallel.done.load(Ordering::SeqCst), 12);
    assert_eq!(parallel.most.load(Ordering::SeqCst), 3, "default runs 3 at once, across both runners");
    first.shutdown(Duration::from_secs(5)).await;
    second.shutdown(Duration::from_secs(5)).await;
}

#[test]
fn a_class_on_a_queue_without_workers_is_refused() {
    let mut registry: Registry<()> = Registry::new();
    registry.register(|(), _: SlackStep, _: Execution| async { Ok(Outcome::Done) });
    let config = RunnerConfig::new(vec![QueueConfig::new("default", 2)]);
    let error = JobQueue::new(&registry, &config).err().expect("refused").to_string();
    assert_eq!(error, r#"SlackImport::StepJob runs on the "slack_import" queue, which has no workers"#);
}

// --- Leases, crashes and shutdown ------------------------------------------------------------------------

/// A job that outlives its lease keeps it while its runner heartbeats: another runner's
/// recovery sweep doesn't take it.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_heartbeat_keeps_a_long_jobs_lease() {
    let performed = Arc::new(Concurrency::default());
    let registry = || {
        let performed = performed.clone();
        let mut registry = Registry::new();
        registry.register(move |(), _: Echo, _: Execution| {
            let performed = performed.clone();
            async move {
                performed.hold(Duration::from_millis(1500)).await;
                Ok(Outcome::Done)
            }
        });
        registry
    };
    let mut config = config();
    config.lease = Duration::from_millis(400);
    config.heartbeat = Duration::from_millis(100);
    config.recovery = Duration::from_millis(50);
    let h = harness(&registry(), &config);
    h.clock.travel_back(); // real time: leases expire as the heartbeat stops
    h.enqueue(Echo { n: 1 }).await;
    let first = start(h.db.clone(), h.queue.clone(), registry(), (), config.clone());
    h.wait_for("the claim", |jobs| jobs[0].status == RUNNING).await;
    let second = start(h.db.clone(), JobQueue::new(&registry(), &config).unwrap(), registry(), (), config.clone());
    h.wait_for("the job to finish", |jobs| jobs.is_empty()).await;
    assert_eq!(performed.done.load(Ordering::SeqCst), 1, "performed once");
    assert_eq!(performed.most.load(Ordering::SeqCst), 1);
    first.shutdown(Duration::from_secs(5)).await;
    second.shutdown(Duration::from_secs(5)).await;
}

/// A job whose runner stopped heartbeating (its process died) is retried once its lease
/// expires, the lost execution counted as an attempt; one with no attempts left fails.
#[tokio::test]
async fn an_orphaned_job_is_recovered_when_its_lease_expires() {
    let (registry, mut performed) = echo_registry();
    let h = harness(&registry, &config());
    let retried = h.enqueue(Echo { n: 1 }).await;
    let exhausted = h.enqueue(Echo { n: 2 }).await;
    // A dead process's claims: leases that ran out 1 s from now.
    h.db.write(move |tx| {
        tx.conn().execute("UPDATE background_jobs SET status = 'running', claimed_by = 'dead', lease_expires_at = '2026-09-29 12:00:01', attempts = 1 WHERE id = ?", [retried])?;
        tx.conn().execute("UPDATE background_jobs SET status = 'running', claimed_by = 'dead', lease_expires_at = '2026-09-29 12:00:01', attempts = 5 WHERE id = ?", [exhausted])?;
        Ok(())
    })
    .await
    .unwrap();
    let mut config = config();
    config.recovery = Duration::from_millis(50);
    let runner = start(h.db.clone(), h.queue.clone(), registry, (), config);
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(performed.try_recv().is_err(), "leases still live");
    assert!(h.jobs().iter().all(|job| job.status == RUNNING));

    h.travel(1);
    assert_eq!(next(&mut performed).await, (1, 2), "the second execution");
    let failed = h.wait_for("the exhausted job to fail", |jobs| jobs.len() == 1 && jobs[0].status == FAILED).await.remove(0);
    assert_eq!(failed.id, exhausted);
    assert_eq!(failed.last_error.as_deref(), Some("the process performing it stopped (execution 5 of 5)"));
    runner.shutdown(Duration::from_secs(5)).await;
}

/// A job its runner is still performing isn't recovered from under it, even with its lease
/// expired (a heartbeat that couldn't be written in time).
#[tokio::test]
async fn a_job_still_being_performed_is_not_recovered_by_its_runner() {
    let (release, released) = (Arc::new(tokio::sync::Notify::new()), Arc::new(AtomicUsize::new(0)));
    let (performed, mut performed_rx) = mpsc::unbounded_channel();
    let mut registry = Registry::new();
    let (release_by_job, released_by_job) = (release.clone(), released.clone());
    registry.register(move |(), job: Echo, execution: Execution| {
        let (release, released, performed) = (release_by_job.clone(), released_by_job.clone(), performed.clone());
        async move {
            let _ = performed.send((job.n, execution.executions));
            release.notified().await;
            released.fetch_add(1, Ordering::SeqCst);
            Ok(Outcome::Done)
        }
    });
    let mut config = config();
    config.recovery = Duration::from_millis(50);
    let h = harness(&registry, &config);
    h.enqueue(Echo { n: 1 }).await;
    let runner = start(h.db.clone(), h.queue.clone(), registry, (), config);
    assert_eq!(next(&mut performed_rx).await, (1, 1));
    h.travel(60); // the lease has expired; the heartbeat is 10 s away
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(performed_rx.try_recv().is_err(), "not performed again");
    let job = &h.jobs()[0];
    assert_eq!((job.status.as_str(), job.attempts), (RUNNING, 1));
    release.notify_one();
    h.wait_for("the job to finish", |jobs| jobs.is_empty()).await;
    assert_eq!(released.load(Ordering::SeqCst), 1);
    runner.shutdown(Duration::from_secs(5)).await;
}

/// Rejects every DELETE from `background_jobs` (a job's completion) until dropped.
async fn reject_completions(h: &Harness) {
    h.db.write(|tx| Ok(tx.conn().execute_batch("CREATE TRIGGER reject_completions BEFORE DELETE ON background_jobs BEGIN SELECT RAISE(ABORT, 'completion rejected'); END")?)).await.unwrap();
}

async fn accept_completions(h: &Harness) {
    h.db.write(|tx| Ok(tx.conn().execute_batch("DROP TRIGGER reject_completions")?)).await.unwrap();
}

/// A completion that can't be written is retried while its runner lives, lease or no lease.
#[tokio::test]
async fn a_failed_completion_write_is_retried() {
    let (registry, mut performed) = echo_registry();
    let h = harness(&registry, &config());
    reject_completions(&h).await;
    h.enqueue(Echo { n: 1 }).await;
    let runner = start(h.db.clone(), h.queue.clone(), registry, (), config());
    assert_eq!(next(&mut performed).await, (1, 1));
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(h.jobs()[0].status, RUNNING, "not recorded yet");
    accept_completions(&h).await;
    h.wait_for("the completion", |jobs| jobs.is_empty()).await; // the clock is frozen: no lease expired
    assert!(performed.try_recv().is_err(), "performed once");
    runner.shutdown(Duration::from_secs(5)).await;
}

/// A claim its own runner holds but no longer performs (its completion couldn't be written in
/// time) is recovered once its lease expires, like a dead runner's.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_claim_its_runner_stopped_performing_is_recovered() {
    let (registry, mut performed) = echo_registry();
    let mut config = config();
    config.lease = Duration::from_millis(300);
    config.heartbeat = Duration::from_millis(100);
    config.recovery = Duration::from_millis(50);
    let h = harness(&registry, &config);
    h.clock.travel_back(); // real time: the lease runs out
    reject_completions(&h).await;
    h.enqueue(Echo { n: 1 }).await;
    let runner = start(h.db.clone(), h.queue.clone(), registry, (), config);
    assert_eq!(next(&mut performed).await, (1, 1));
    tokio::time::sleep(Duration::from_millis(1000)).await; // past the completion's retries
    accept_completions(&h).await;
    h.wait_for("the job to be recovered and performed", |jobs| jobs.is_empty()).await;
    let (n, executions) = next(&mut performed).await;
    assert!(n == 1 && executions >= 2, "performed again: {executions}");
    runner.shutdown(Duration::from_secs(5)).await;
}

/// Shutdown stops claiming, lets running jobs finish within the grace period, and leaves waiting
/// jobs in the queue for the next process.
#[tokio::test]
async fn shutdown_drains_running_jobs_and_leaves_the_rest_queued() {
    let performed = Arc::new(Concurrency::default());
    let performed_by_job = performed.clone();
    let mut registry = Registry::new();
    registry.register(move |(), _: Echo, _: Execution| {
        let performed = performed_by_job.clone();
        async move {
            performed.hold(Duration::from_millis(300)).await;
            Ok(Outcome::Done)
        }
    });
    let mut config = config();
    config.queues = vec![QueueConfig::new("default", 2)];
    let h = harness(&registry, &config);
    for n in 0..5 {
        h.enqueue(Echo { n }).await;
    }
    let runner = start(h.db.clone(), h.queue.clone(), registry, (), config);
    h.wait_for("two claims", |jobs| jobs.iter().filter(|job| job.status == RUNNING).count() == 2).await;
    runner.shutdown(Duration::from_secs(5)).await;
    assert_eq!(performed.done.load(Ordering::SeqCst), 2, "the running jobs finished");
    let jobs = h.jobs();
    assert_eq!(jobs.len(), 3);
    assert!(jobs.iter().all(|job| job.status == READY && job.attempts == 0), "{jobs:#?}");
}

/// Sets its flag when dropped, after a while (a job's cleanup).
struct SlowDrop(Arc<AtomicBool>);

impl Drop for SlowDrop {
    fn drop(&mut self) {
        std::thread::sleep(Duration::from_millis(100));
        self.0.store(true, Ordering::SeqCst);
    }
}

/// Jobs still running when the grace period ends are abandoned (aborted, and gone by the time
/// shutdown returns) and handed back, due now, the interrupted execution not counted.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn shutdown_hands_back_jobs_that_outlast_the_grace_period() {
    let dropped = Arc::new(AtomicBool::new(false));
    let mut registry = Registry::new();
    let dropped_by_job = dropped.clone();
    registry.register(move |(), _: Echo, _: Execution| {
        let guard = SlowDrop(dropped_by_job.clone());
        async move {
            let _guard = guard;
            std::future::pending::<()>().await;
            Ok(Outcome::Done)
        }
    });
    let h = harness(&registry, &config());
    let id = h.enqueue(Echo { n: 1 }).await;
    let runner = start(h.db.clone(), h.queue.clone(), registry, (), config());
    h.wait_for("the claim", |jobs| jobs[0].status == RUNNING).await;
    h.travel(10);
    runner.shutdown(Duration::from_millis(200)).await;
    assert!(dropped.load(Ordering::SeqCst), "the abandoned job was aborted before shutdown returned");
    let job = h.job(id).unwrap();
    assert_eq!((job.status.as_str(), job.attempts, job.claimed_by), (READY, 0, None));
    assert_eq!(job.run_at, at("2026-09-29 12:00:10"));
}

// --- Decisions -----------------------------------------------------------------------------------------

#[test]
fn decisions_follow_retry_on_and_discard_on() {
    let policy = RetryPolicy::application_job().wait(Wait::PolynomiallyLonger { jitter: 0.0 });
    let timeout = || JobError::Error(anyhow::Error::new(std::io::Error::new(std::io::ErrorKind::TimedOut, "timed out")));
    let retry = |seconds, error: &str| Decision::Reschedule { wait: Duration::from_secs(seconds), error: Some(error.into()), reset_attempts: false };
    assert_eq!(decide(&policy, 1, &Ok(Outcome::Done), 0.0), Decision::Delete);
    assert_eq!(decide(&policy, 1, &Err(timeout()), 0.0), retry(3, "timed out"));
    assert_eq!(decide(&policy, 4, &Err(timeout()), 0.0), retry(258, "timed out"));
    assert_eq!(decide(&policy, 5, &Err(timeout()), 0.0), Decision::Fail("timed out".into()));
    assert_eq!(decide(&policy, 1, &Err(JobError::Error(anyhow::anyhow!("boom"))), 0.0), Decision::Fail("boom".into()));
    assert_eq!(decide(&policy, 1, &Err(JobError::retry(anyhow::anyhow!("boom"))), 0.0), retry(3, "boom"));
    assert_eq!(decide(&policy, 1, &Err(JobError::fail(anyhow::anyhow!(timeout().error().to_string()))), 0.0), Decision::Fail("timed out".into()));
    assert_eq!(decide(&policy, 1, &Err(JobError::discard(anyhow::anyhow!("gone"))), 0.0), Decision::Delete);
    assert_eq!(
        decide(&policy, 1, &Ok(Outcome::Again(Duration::from_secs(5))), 0.0),
        Decision::Reschedule { wait: Duration::from_secs(5), error: None, reset_attempts: true }
    );
}
