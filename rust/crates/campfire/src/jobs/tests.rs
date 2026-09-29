use std::sync::Mutex as StdMutex;

use campfire_jobs::inspect::{self, JobRow};
use tokio::sync::Notify;

use super::*;
use crate::app::{Booted, boot};

/// An app booted over an empty storage directory.
async fn app_in(dir: &std::path::Path) -> Booted {
    let root = dir.to_string_lossy().into_owned();
    let config = Config::from_lookup(|name| match name {
        "SECRET_KEY_BASE_DUMMY" => Some("1".into()),
        "CAMPFIRE_STORAGE_PATH" => Some(root.clone()),
        _ => None,
    })
    .unwrap();
    boot(config).await.unwrap()
}

async fn app() -> (Booted, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    (app_in(dir.path()).await, dir)
}

fn jobs(app: &App) -> Vec<JobRow> {
    app.db.read_blocking(inspect::all).unwrap()
}

async fn wait_for(app: &App, what: &str, condition: impl Fn(&[JobRow]) -> bool) -> Vec<JobRow> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        let jobs = jobs(app);
        if condition(&jobs) {
            return jobs;
        }
        assert!(tokio::time::Instant::now() < deadline, "timed out waiting for {what}: {jobs:#?}");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// Log lines written while `f` runs on this thread (the test runtime's only thread).
struct Logs(Arc<StdMutex<Vec<u8>>>);

impl Logs {
    fn capture() -> (Self, tracing::subscriber::DefaultGuard) {
        let buffer = Arc::new(StdMutex::new(Vec::new()));
        let writer = buffer.clone();
        let subscriber = tracing_subscriber::fmt().with_ansi(false).with_writer(move || LogWriter(writer.clone())).finish();
        (Self(buffer), tracing::subscriber::set_default(subscriber))
    }

    fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
    }
}

struct LogWriter(Arc<StdMutex<Vec<u8>>>);

impl std::io::Write for LogWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

// --- The sink ------------------------------------------------------------------------------------------

#[test]
fn job_events_ask_for_their_rails_job_classes() {
    let request = |event| request_for(&event).map(|request| (request.class, request.arguments));
    assert_eq!(request(Event::PushMessage { room_id: 1, message_id: 2 }), Some(("Room::PushMessageJob", serde_json::json!({"room_id": 1, "message_id": 2}))));
    assert_eq!(request(Event::DeliverWebhook { bot_id: 3, message_id: 4 }), Some(("Bot::WebhookJob", serde_json::json!({"bot_id": 3, "message_id": 4}))));
    assert_eq!(request(Event::RemoveBannedContent { user_id: 5 }), Some(("RemoveBannedContentJob", serde_json::json!({"user_id": 5}))));
    assert_eq!(request(Event::PurgeBlob { blob_id: 6 }), Some(("ActiveStorage::PurgeJob", serde_json::json!({"blob_id": 6}))));
    assert_eq!(request(Event::DisconnectUser { user_id: 7, reconnect: false }), None);
}

/// A job event is a row written in its write's transaction, on its class's queue: it commits,
/// or rolls back, with the write.
#[tokio::test]
async fn job_events_are_enqueued_with_their_write() {
    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    booted.jobs.shutdown(Duration::from_secs(5)).await; // leave the rows be

    let rolled_back = app
        .db
        .write(|tx| {
            tx.emit_after_commit(Event::DeliverWebhook { bot_id: 1, message_id: 2 });
            Err::<(), _>(campfire_db::Error::Other("validation failed".into()))
        })
        .await;
    assert!(rolled_back.is_err());
    assert!(jobs(&app).is_empty());

    app.db
        .write(|tx| {
            tx.emit_after_commit(Event::DeliverWebhook { bot_id: 1, message_id: 2 });
            tx.emit_after_commit(Event::PushMessage { room_id: 3, message_id: 2 });
            tx.emit_now(Event::DisconnectUser { user_id: 1, reconnect: true });
            Ok(())
        })
        .await
        .unwrap();
    let rows: Vec<_> = jobs(&app).into_iter().map(|job| (job.queue, job.class, job.arguments, job.status)).collect();
    assert_eq!(
        rows,
        [
            (WEBHOOKS_QUEUE.to_string(), "Bot::WebhookJob".to_string(), serde_json::json!({"bot_id": 1, "message_id": 2}), "ready".to_string()),
            (PUSH_QUEUE.to_string(), "Room::PushMessageJob".to_string(), serde_json::json!({"room_id": 3, "message_id": 2}), "ready".to_string()),
        ]
    );
}

// --- The handlers --------------------------------------------------------------------------------------

/// `discard_on ActiveJob::DeserializationError`: jobs whose records are gone are discarded, not
/// failed or retried.
#[tokio::test]
async fn jobs_whose_records_are_gone_are_discarded() {
    let (booted, _dir) = app().await;
    let (logs, _guard) = Logs::capture();
    let app = booted.app.clone();
    app.db
        .write(|tx| {
            tx.emit_after_commit(Event::DeliverWebhook { bot_id: 404, message_id: 404 });
            tx.emit_after_commit(Event::RemoveBannedContent { user_id: 404 });
            Ok(())
        })
        .await
        .unwrap();
    wait_for(&app, "the jobs to go", |jobs| jobs.is_empty()).await;
    booted.jobs.shutdown(Duration::from_secs(5)).await;
    let logs = logs.text();
    assert!(logs.contains(r#"discarded job="Bot::WebhookJob""#), "{logs}");
    assert!(logs.contains(r#"discarded job="RemoveBannedContentJob""#), "{logs}");
}

#[tokio::test]
async fn purging_a_blob_later_deletes_it_and_its_file() {
    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    let (storage, now) = (app.storage.clone(), app.clock.now());
    let blob = app
        .db
        .write(move |tx| {
            let blob = storage
                .create_and_upload(tx.conn(), b"hello", campfire_storage::Filename::new("hello.txt"), None, now)
                .map_err(|e| campfire_db::Error::Other(e.to_string()))?;
            tx.emit_after_commit(Event::PurgeBlob { blob_id: blob.id });
            Ok(blob)
        })
        .await
        .unwrap();
    let path = app.storage.path_for(&blob);
    wait_for(&app, "the purge", |jobs| jobs.is_empty()).await;
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while path.exists() {
        assert!(std::time::Instant::now() < deadline, "the file is still there");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let id = blob.id;
    let found = app.db.read(move |conn| Ok(conn.query_row("SELECT count(*) FROM active_storage_blobs WHERE id = ?", [id], |row| row.get::<_, i64>(0))?)).await.unwrap();
    assert_eq!(found, 0);
    booted.jobs.shutdown(Duration::from_secs(5)).await;
}

// --- Queues ---------------------------------------------------------------------------------------------

/// A handler that reports each job it performs, after `gate` lets it through (when given).
fn reporting<J: Send + 'static>(performed: mpsc::UnboundedSender<String>, gate: Option<Arc<Notify>>) -> impl Fn(App, J, Execution) -> BoxFuture<'static, JobResult> + Send + Sync + 'static {
    move |_app: App, _job: J, _: Execution| {
        let (performed, gate) = (performed.clone(), gate.clone());
        Box::pin(async move {
            if let Some(gate) = gate {
                gate.notified().await;
            }
            let _ = performed.send(std::any::type_name::<J>().rsplit("::").next().unwrap().to_string());
            Ok(Outcome::Done)
        })
    }
}

async fn next_performed(performed: &mut mpsc::UnboundedReceiver<String>) -> String {
    tokio::time::timeout(Duration::from_secs(5), performed.recv()).await.expect("a job was performed").unwrap()
}

/// Webhooks to a slow bot fill their own queue's workers, not the ones notifications and the
/// rest run on.
#[tokio::test]
async fn a_busy_queue_doesnt_hold_up_the_others() {
    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    booted.jobs.shutdown(Duration::from_secs(5)).await;

    let (performed, mut performed_rx) = mpsc::unbounded_channel();
    let slow_bot = Arc::new(Notify::new());
    let mut registry = Registry::new();
    registry.register(reporting::<WebhookJob>(performed.clone(), Some(slow_bot.clone())));
    registry.register(reporting::<PushMessageJob>(performed.clone(), None));
    registry.register(reporting::<PurgeJob>(performed, None));
    let config = runner_config(&app.config);
    let queue = JobQueue::new(&registry, &config).unwrap();
    let runner = campfire_jobs::start(app.db.clone(), queue.clone(), registry, app.clone(), config);

    for message_id in 1..=10 {
        queue.perform_later(&app.db, JobRequest::new(&WebhookJob { bot_id: 1, message_id })).await.unwrap();
    }
    let workers = app.config.job_concurrency;
    wait_for(&app, "the webhooks queue to fill", |jobs| jobs.iter().filter(|job| job.status == "running").count() == workers).await;
    queue.perform_later(&app.db, JobRequest::new(&PushMessageJob { room_id: 1, message_id: 1 })).await.unwrap();
    queue.perform_later(&app.db, JobRequest::new(&PurgeJob { blob_id: 1 })).await.unwrap();
    let mut others = [next_performed(&mut performed_rx).await, next_performed(&mut performed_rx).await];
    others.sort();
    assert_eq!(others, ["PurgeJob", "PushMessageJob"]);

    for _ in 0..3 {
        slow_bot.notify_one();
        assert_eq!(next_performed(&mut performed_rx).await, "WebhookJob");
    }
    runner.shutdown(Duration::from_millis(100)).await;
    let waiting = jobs(&app);
    assert_eq!(waiting.len(), 7, "the rest wait for the next process");
    assert!(waiting.iter().all(|job| job.status == "ready" && job.class == "Bot::WebhookJob"));
}

// --- Ad hoc jobs ----------------------------------------------------------------------------------------

#[tokio::test]
async fn a_panicking_ad_hoc_job_is_logged_and_its_worker_carries_on() {
    let (booted, _dir) = app().await;
    let (logs, _guard) = Logs::capture();
    let app = booted.app.clone();

    let (done, finished) = tokio::sync::oneshot::channel();
    app.jobs.perform_later("Exploding", async { panic!("kaboom") });
    app.jobs.perform_later("After", async move {
        let _ = done.send(());
        Ok(())
    });
    tokio::time::timeout(Duration::from_secs(5), finished).await.unwrap().unwrap();
    booted.jobs.shutdown(Duration::from_secs(5)).await;

    let logs = logs.text();
    assert!(logs.contains("job panicked job=\"Exploding\" panic=\"kaboom\""), "{logs}");
}

#[tokio::test]
async fn shutdown_performs_the_queued_ad_hoc_jobs_and_then_takes_no_more() {
    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    let (performed, mut performed_rx) = mpsc::unbounded_channel();
    for n in 1..=5 {
        let performed = performed.clone();
        app.jobs.perform_later("Counting", async move {
            let _ = performed.send(n);
            Ok(())
        });
    }
    booted.jobs.shutdown(Duration::from_secs(5)).await;
    app.jobs.perform_later("Late", async move {
        let _ = performed.send(6);
        Ok(())
    });
    let mut ns: Vec<i64> = std::iter::from_fn(|| performed_rx.try_recv().ok()).collect();
    ns.sort();
    assert_eq!(ns, [1, 2, 3, 4, 5]);
}

// --- Periodic -------------------------------------------------------------------------------------------

async fn insert_user(app: &App, name: &str, bot_token: Option<&str>, digest: Option<&str>) -> i64 {
    let (name, bot_token, digest, now) = (name.to_string(), bot_token.map(str::to_string), digest.map(str::to_string), app.db.env().now());
    app.db
        .write(move |tx| {
            Ok(tx.conn().query_row(
                "INSERT INTO users (name, role, bot_token, bot_token_digest, created_at, updated_at) VALUES (?1, 2, ?2, ?3, ?4, ?4) RETURNING id",
                rusqlite::params![name, bot_token, digest, now],
                |row| row.get(0),
            )?)
        })
        .await
        .unwrap()
}

async fn tokens(app: &App, id: i64) -> (Option<String>, Option<String>) {
    app.db.read(move |conn| Ok(conn.query_row("SELECT bot_token, bot_token_digest FROM users WHERE id = ?", [id], |row| Ok((row.get(0)?, row.get(1)?)))?)).await.unwrap()
}

/// `Bots::ClearPlaintextTokens.run!`: the digest recomputed from the plaintext, which is nulled;
/// rows without one untouched.
#[tokio::test]
async fn clearing_plaintext_bot_tokens_heals_their_digests() {
    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    booted.jobs.shutdown(Duration::from_secs(5)).await;
    let stale = insert_user(&app, "Stale", Some("BenderToken1"), Some("stale digest")).await;
    let current = insert_user(&app, "Current", None, Some("kept digest")).await;

    assert_eq!(periodic::clear_plaintext_bot_tokens(&app.db).await.unwrap(), 1);
    assert_eq!(tokens(&app, stale).await, (None, Some(campfire_db::user::digest_bot_token("BenderToken1"))));
    assert_eq!(tokens(&app, current).await, (None, Some("kept digest".into())));
    assert_eq!(periodic::clear_plaintext_bot_tokens(&app.db).await.unwrap(), 0, "idempotent");
}

/// The periodic loop runs in the app: a booting process clears the plaintext tokens.
#[tokio::test]
async fn a_booting_app_clears_plaintext_bot_tokens() {
    let dir = tempfile::tempdir().unwrap();
    let booted = app_in(dir.path()).await;
    booted.jobs.shutdown(Duration::from_secs(5)).await;
    let stale = insert_user(&booted.app, "Stale", Some("BenderToken1"), None).await;

    let booted = app_in(dir.path()).await;
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while tokens(&booted.app, stale).await.0.is_some() {
        assert!(std::time::Instant::now() < deadline, "not cleared");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    booted.jobs.shutdown(Duration::from_secs(5)).await;
}

#[test]
fn periodic_intervals_come_from_the_environment() {
    let intervals = periodic::Intervals::from_lookup(|name| (name == "EVENT_REMINDERS_INTERVAL").then(|| "10".into())).unwrap();
    assert_eq!(
        intervals,
        periodic::Intervals { reminders: Duration::from_secs(10), retention: Duration::from_secs(86_400), huddle: Duration::from_secs(5) }
    );
    let error = periodic::Intervals::from_lookup(|name| (name == "HUDDLE_RECONCILE_INTERVAL").then(|| "0".into())).unwrap_err();
    assert_eq!(error.to_string(), "HUDDLE_RECONCILE_INTERVAL must be a positive finite number");
}

// --- Latency ------------------------------------------------------------------------------------------

/// How long a push notification's job waits between the write that asks for it and its handler
/// starting: the enqueue (a row in the write's transaction), the commit, the wake and the claim.
/// `cargo test -p campfire --release push_latency -- --ignored --nocapture`
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "a measurement, not a test"]
async fn push_latency() {
    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    booted.jobs.shutdown(Duration::from_secs(5)).await;

    let (started, mut started_rx) = mpsc::unbounded_channel();
    let mut registry = Registry::new();
    registry.register(move |_: App, job: PushMessageJob, _: Execution| {
        let started = started.clone();
        async move {
            let _ = started.send((job.message_id, std::time::Instant::now()));
            Ok(Outcome::Done)
        }
    });
    let config = runner_config(&app.config);
    let (jobs, _) = Jobs::new(&registry, &config).unwrap();
    let queue = jobs.queue.clone();
    let runner = campfire_jobs::start(app.db.clone(), queue.clone(), registry, app.clone(), config);

    let mut latencies = Vec::new();
    for message_id in 0..500 {
        let sink = jobs.clone();
        let asked = std::time::Instant::now();
        app.db
            .write(move |tx| {
                // As the app's sink does for `Event::PushMessage`.
                let request = request_for(&Event::PushMessage { room_id: 1, message_id }).unwrap();
                sink.persist(tx, &Event::Job(request.clone()))?;
                tx.after_commit(move |_| {
                    sink.emit(Event::Job(request));
                    Ok(())
                });
                Ok(())
            })
            .await
            .unwrap();
        let (performed, at) = tokio::time::timeout(Duration::from_secs(5), started_rx.recv()).await.unwrap().unwrap();
        assert_eq!(performed, message_id);
        latencies.push(at - asked);
    }
    runner.shutdown(Duration::from_secs(5)).await;
    latencies.sort();
    let percentile = |p: usize| latencies[(latencies.len() * p / 100).min(latencies.len() - 1)];
    println!("push enqueue-to-start over {} jobs: p50 {:?} p95 {:?} p99 {:?} max {:?}", latencies.len(), percentile(50), percentile(95), percentile(99), latencies.last().unwrap());
}
