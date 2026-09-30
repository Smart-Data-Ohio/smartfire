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
        "DISABLE_SSL" => Some("true".into()), // plain HTTP requests, for the controller tests
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

/// Runs `f` in a write while a trigger rejects every new `background_jobs` row.
async fn rejecting_jobs<T: Send + 'static>(app: &App, f: impl FnOnce(&mut Tx<'_>) -> campfire_db::Result<T> + Send + 'static) -> campfire_db::Result<T> {
    const TRIGGER: &str = "CREATE TRIGGER ws3_reject_jobs BEFORE INSERT ON background_jobs BEGIN SELECT RAISE(ABORT, 'jobs rejected'); END";
    app.db.write(|tx| Ok(tx.conn().execute_batch(TRIGGER)?)).await.unwrap();
    let result = app.db.write(f).await;
    app.db.write(|tx| Ok(tx.conn().execute_batch("DROP TRIGGER ws3_reject_jobs")?)).await.unwrap();
    result
}

fn count(app: &App, sql: &'static str) -> i64 {
    app.db.read_blocking(move |conn| Ok(conn.query_row(sql, [], |row| row.get(0))?)).unwrap()
}

/// The classes of the queued jobs, which are then cleared.
async fn take_jobs(app: &App) -> Vec<String> {
    let classes = jobs(app).into_iter().map(|job| job.class).collect();
    app.db.write(|tx| Ok(tx.conn().execute_batch("DELETE FROM background_jobs")?)).await.unwrap();
    classes
}

/// Every path that enqueues one of the migrated jobs, through the app's real sink, writes the
/// job's row in the write's own transaction: when the row can't be written the write fails and
/// none of it commits; when it can, the row commits with the write.
#[tokio::test]
async fn a_job_that_cant_be_enqueued_fails_the_write_that_asks_for_it() {
    use campfire_db::{Message, NewMessage, NewUser, Room, RoomType, User};

    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    booted.jobs.shutdown(Duration::from_secs(5)).await; // leave the rows be
    let (storage, now) = (app.storage.clone(), app.clock.now());
    let (author, bot, room, blob) = app
        .db
        .write(move |tx| {
            let author = User::create(tx, NewUser { name: "Author".into(), ..Default::default() })?;
            let bot = User::create_bot(tx, "Bender", Some("https://example.com/hook"))?;
            let room = Room::create_for(tx, RoomType::Closed, Some("Jobs"), author.id, &[author.id, bot.id])?;
            let blob = storage
                .create_and_upload(tx.conn(), b"hello", campfire_storage::Filename::new("hello.txt"), None, now)
                .map_err(|e| campfire_db::Error::Other(e.to_string()))?;
            Ok((author.id, bot.id, room.id, blob.id))
        })
        .await
        .unwrap();
    take_jobs(&app).await;
    let post = move |attachment_blob_id| move |tx: &mut Tx<'_>| {
        Message::create(tx, NewMessage { room_id: room, creator_id: author, body: Some("<div>Hi</div>".into()), attachment_blob_id, ..Default::default() }).map(|message| message.id)
    };

    // Message#receive_in_conversation → Room#push_later
    assert!(rejecting_jobs(&app, post(None)).await.is_err(), "posting a message");
    assert_eq!(count(&app, "SELECT count(*) FROM messages"), 0);
    let message = app.db.write(post(Some(blob))).await.unwrap();
    assert_eq!(count(&app, "SELECT count(*) FROM messages"), 1);
    assert_eq!(take_jobs(&app).await, ["Room::PushMessageJob"]);

    // User::Bot#deliver_webhook_later
    let webhook = move |tx: &mut Tx<'_>| User::find(tx.conn(), bot)?.deliver_webhook_later(tx, message);
    assert!(rejecting_jobs(&app, webhook).await.is_err(), "a webhook");
    app.db.write(webhook).await.unwrap();
    assert_eq!(take_jobs(&app).await, ["Bot::WebhookJob"]);

    // Message#destroy → the attachment's `dependent: :purge_later`
    let destroy = move |tx: &mut Tx<'_>| Message::find(tx.conn(), message)?.destroy(tx);
    assert!(rejecting_jobs(&app, destroy).await.is_err(), "destroying a message");
    assert_eq!(count(&app, "SELECT count(*) FROM messages"), 1);
    app.db.write(destroy).await.unwrap();
    assert_eq!(count(&app, "SELECT count(*) FROM messages"), 0);
    assert_eq!(take_jobs(&app).await, ["ActiveStorage::PurgeJob"]);

    // User::Bannable#ban → apply_ban
    let ban = move |tx: &mut Tx<'_>| User::find(tx.conn(), author)?.ban(tx);
    assert!(rejecting_jobs(&app, ban).await.is_err(), "a ban");
    assert_eq!(count(&app, "SELECT count(*) FROM users WHERE status = 0"), 2, "nobody banned");
    app.db.write(ban).await.unwrap();
    assert_eq!(take_jobs(&app).await, ["RemoveBannedContentJob"]);
}

/// A browser on the booted app's router: a cookie jar, Chrome, and Rails' CSRF tokens.
struct Browser {
    router: axum::Router,
    cookies: std::collections::BTreeMap<String, String>,
    secrets: Arc<rails_compat::Secrets>,
}

impl Browser {
    fn new(router: &axum::Router, secrets: Arc<rails_compat::Secrets>) -> Self {
        Self { router: router.clone(), cookies: Default::default(), secrets }
    }

    async fn get(&mut self, path: &str) -> (axum::http::StatusCode, String) {
        self.send(axum::http::Request::get(path).header("accept", "text/html"), String::new()).await
    }

    async fn post(&mut self, path: &str, accept: &str, fields: &[(&str, &str)]) -> (axum::http::StatusCode, String) {
        let body = fields.iter().map(|(k, v)| format!("{}={}", campfire_views::helpers::url::cgi_escape(k), campfire_views::helpers::url::cgi_escape(v))).collect::<Vec<_>>().join("&");
        let session = self.cookies.get(campfire_kit::session::SESSION_KEY).expect("a page established the browser's session");
        let token = crate::controllers::presenters::test_support::masked_session_token(&self.secrets, session).expect("the page gave the session a CSRF token");
        let request = axum::http::Request::post(path)
            .header(campfire_kit::csrf::HEADER, token)
            .header("accept", accept)
            .header("content-type", "application/x-www-form-urlencoded");
        self.send(request, body).await
    }

    async fn send(&mut self, request: axum::http::request::Builder, body: String) -> (axum::http::StatusCode, String) {
        use tower::ServiceExt as _;
        let cookie = self.cookies.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("; ");
        let request = request
            .header("host", "campfire.test")
            .header("user-agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36")
            .header("cookie", cookie)
            .body(axum::body::Body::from(body))
            .unwrap();
        let response = self.router.clone().oneshot(request).await.unwrap();
        for cookie in response.headers().get_all("set-cookie") {
            let (name, value) = cookie.to_str().unwrap().split(';').next().unwrap().split_once('=').unwrap();
            self.cookies.insert(name.to_string(), value.to_string());
        }
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, String::from_utf8_lossy(&body).into_owned())
    }
}

/// Posting a message through `MessagesController#create` enqueues its push and its bots'
/// webhooks in the message's own transaction: a webhook that can't be enqueued fails the post
/// with nothing committed. Committed, the webhooks are held until the message has been broadcast,
/// then released; if releasing them fails (as if the process died), they're delivered when the
/// hold runs out rather than lost.
#[tokio::test]
async fn a_posted_message_and_its_webhooks_commit_together() {
    use campfire_db::{FirstRun, PasswordDigest, Room, RoomType, User};

    let (booted, _dir) = app().await;
    let (app, router) = (booted.app.clone(), booted.router.clone());
    booted.jobs.shutdown(Duration::from_secs(5)).await; // leave the rows be
    let digest = PasswordDigest::create("secret123456", 4).unwrap();
    let room = app
        .db
        .write(move |tx| {
            let person = FirstRun::create(tx, "Person", "person@example.com", digest)?;
            let bot = User::create_bot(tx, "Bender", Some("https://example.com/hook"))?;
            Ok(Room::create_for(tx, RoomType::Direct, None, person.id, &[person.id, bot.id])?.id)
        })
        .await
        .unwrap();
    take_jobs(&app).await;
    let mut browser = Browser::new(&router, app.secrets.clone());
    let (status, body) = browser.get("/session/new").await;
    assert_eq!(status, axum::http::StatusCode::OK, "sign-in page: {body}");
    let (status, body) = browser.post("/session", "text/html", &[("email_address", "person@example.com"), ("password", "secret123456")]).await;
    assert_eq!(status, axum::http::StatusCode::FOUND, "signed in: {body}");
    let (status, _) = browser.get("/two_factor_setup").await;
    assert_eq!(status, axum::http::StatusCode::OK);
    let encryption = rails_compat::ar_encryption::ArEncryption::new(&app.secrets);
    let enrollment_now = campfire_db::Timestamp::from_jiff(app.clock.now());
    let secret = app.db.read(move |conn| {
        let session_id = conn.query_row("SELECT session_id FROM two_factor_setup_secrets", [], |r| r.get(0))?;
        campfire_db::TwoFactorSetupSecret::valid_for(conn, session_id, enrollment_now)?.expect("live enrollment").secret(&encryption)
    }).await.unwrap();
    let code = rails_compat::totp::at(&secret, app.clock.now().as_second()).unwrap();
    let (status, body) = browser.post("/two_factor_setup", "text/html", &[("code", &code)]).await;
    assert_eq!(status, axum::http::StatusCode::OK, "completed enrollment: {body}");
    let path = format!("/rooms/{room}/messages");
    let post = |n: &'static str| [("message[body]", "<p>Hello bot</p>"), ("message[client_message_id]", n)];
    let messages = || count(&app, "SELECT count(*) FROM messages");

    const REJECT: &str = "CREATE TRIGGER ws3_reject_webhooks BEFORE INSERT ON background_jobs WHEN NEW.job_class = 'Bot::WebhookJob' BEGIN SELECT RAISE(ABORT, 'webhooks rejected'); END";
    app.db.write(|tx| Ok(tx.conn().execute_batch(REJECT)?)).await.unwrap();
    let (status, _) = browser.post(&path, "text/vnd.turbo-stream.html", &post("rejected")).await;
    assert!(status.is_server_error(), "{status}");
    assert_eq!((messages(), jobs(&app).len()), (0, 0), "the message rolled back with its webhook");
    app.db.write(|tx| Ok(tx.conn().execute_batch("DROP TRIGGER ws3_reject_webhooks")?)).await.unwrap();

    let (status, body) = browser.post(&path, "text/vnd.turbo-stream.html", &post("posted")).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    assert_eq!(messages(), 1);
    let now = app.db.env().now();
    let mut queued: Vec<_> = jobs(&app).into_iter().map(|job| (job.class, job.run_at <= now)).collect();
    queued.sort();
    assert_eq!(queued, [("Bot::WebhookJob".to_string(), true), ("Room::PushMessageJob".to_string(), true)], "released once broadcast");
    take_jobs(&app).await;

    const KEEP_HELD: &str = "CREATE TRIGGER ws3_keep_webhooks_held BEFORE UPDATE OF run_at ON background_jobs WHEN NEW.job_class = 'Bot::WebhookJob' BEGIN SELECT RAISE(ABORT, 'release rejected'); END";
    app.db.write(|tx| Ok(tx.conn().execute_batch(KEEP_HELD)?)).await.unwrap();
    let (status, body) = browser.post(&path, "text/vnd.turbo-stream.html", &post("held")).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    assert_eq!(messages(), 2);
    let webhook = jobs(&app).into_iter().find(|job| job.class == "Bot::WebhookJob").expect("the webhook is queued");
    let held = webhook.run_at.as_microsecond() - app.db.env().now().as_microsecond();
    assert!(held > 60_000_000 && held <= WEBHOOK_HOLD.as_micros() as i64, "held for {held} µs");
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

/// Records whether a task finished, or was dropped (aborted) before it did.
#[derive(Clone, Default)]
struct Fate {
    finished: Arc<std::sync::atomic::AtomicBool>,
    dropped: Arc<std::sync::atomic::AtomicBool>,
}

struct DropFlag(Arc<std::sync::atomic::AtomicBool>);

impl Drop for DropFlag {
    fn drop(&mut self) {
        self.0.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

impl Fate {
    /// Sleeps 250 ms, after reporting it started.
    fn work(&self, started: mpsc::UnboundedSender<()>) -> impl Future<Output = anyhow::Result<()>> + Send + use<> {
        let (finished, guard) = (self.finished.clone(), DropFlag(self.dropped.clone()));
        async move {
            let _guard = guard;
            let _ = started.send(());
            tokio::time::sleep(Duration::from_millis(250)).await;
            finished.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
    }

    fn get(&self) -> (bool, bool) {
        (self.finished.load(std::sync::atomic::Ordering::SeqCst), self.dropped.load(std::sync::atomic::Ordering::SeqCst))
    }
}

/// Periodic tasks and ad hoc jobs still running when the grace period ends are aborted, and gone
/// by the time shutdown returns: none carries on detached.
#[tokio::test]
async fn shutdown_aborts_what_outlasts_the_grace_period() {
    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    booted.jobs.shutdown(Duration::from_secs(5)).await;

    let (periodic_fate, ad_hoc_fate) = (Fate::default(), Fate::default());
    let (started, mut starts) = mpsc::unbounded_channel();
    let mut loops = periodic::Loops::new(periodic::Intervals::from_lookup(|_| None));
    let (fate, started_by_task) = (periodic_fate.clone(), started.clone());
    loops.periodic.as_mut().unwrap().task(campfire_jobs::periodic::Task::new("slow", Duration::from_secs(60), move |_: App| fate.work(started_by_task.clone())));
    let config = runner_config(&app.config);
    let (jobs, ad_hoc) = Jobs::new(&registry(), &config).unwrap();
    let runner = start(app.clone(), registry(), ad_hoc, config, loops);
    jobs.perform_later("Slow", ad_hoc_fate.work(started));
    for _ in 0..2 {
        tokio::time::timeout(Duration::from_secs(5), starts.recv()).await.expect("started").unwrap();
    }

    runner.shutdown(Duration::from_millis(20)).await;
    assert_eq!(periodic_fate.get(), (false, true), "the periodic task was aborted");
    assert_eq!(ad_hoc_fate.get(), (false, true), "the ad hoc job was aborted");
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert_eq!(periodic_fate.get(), (false, true), "and didn't carry on");
    assert_eq!(ad_hoc_fate.get(), (false, true), "and didn't carry on");
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

/// The periodic loops run beside the job runner, until it shuts down; the plaintext token task
/// does its work once.
#[tokio::test]
async fn the_periodic_loops_run_with_the_jobs() {
    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    booted.jobs.shutdown(Duration::from_secs(5)).await;
    let stale = insert_user(&app, "Stale", Some("BenderToken1"), None).await;

    let (ticked, mut ticks) = mpsc::unbounded_channel();
    let mut loops = periodic::Loops::new(periodic::Intervals::from_lookup(|_| None));
    loops.periodic.as_mut().unwrap().task(periodic::clear_plaintext_bot_tokens_task());
    loops.huddle.as_mut().unwrap().task(campfire_jobs::periodic::Task::new("reconcile", Duration::from_millis(20), move |_: App| {
        let ticked = ticked.clone();
        async move {
            let _ = ticked.send(());
            Ok(())
        }
    }));
    let config = runner_config(&app.config);
    let (_, ad_hoc) = Jobs::new(&registry(), &config).unwrap();
    let runner = start(app.clone(), registry(), ad_hoc, config, loops);
    for _ in 0..3 {
        tokio::time::timeout(Duration::from_secs(5), ticks.recv()).await.expect("the huddle loop ticks").unwrap();
    }
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while tokens(&app, stale).await.0.is_some() {
        assert!(std::time::Instant::now() < deadline, "not cleared");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    runner.shutdown(Duration::from_secs(5)).await;
    while ticks.try_recv().is_ok() {}
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(ticks.try_recv().is_err(), "stopped with the runner");
}

#[test]
fn periodic_intervals_come_from_the_environment() {
    let intervals = periodic::Intervals::from_lookup(|name| (name == "EVENT_REMINDERS_INTERVAL").then(|| "10".into()));
    assert_eq!(
        intervals,
        periodic::Intervals {
            periodic: Some(periodic::PeriodicIntervals { reminders: Duration::from_secs(10), retention: Duration::from_secs(86_400) }),
            huddle: Some(Duration::from_secs(5)),
        }
    );
    let intervals = periodic::Intervals::from_lookup(|name| (name == "HUDDLE_RECONCILE_INTERVAL").then(|| "0.5".into()));
    assert_eq!(intervals.huddle, Some(Duration::from_millis(500)));
}

/// An invalid interval disables the loop that reads it, as it aborts the Rails script that
/// reads it: the error is logged, and jobs and the other loop keep running.
#[tokio::test]
async fn an_invalid_interval_disables_only_its_loop() {
    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    booted.jobs.shutdown(Duration::from_secs(5)).await;
    let (logs, _guard) = Logs::capture();

    for (name, value) in [("EVENT_REMINDERS_INTERVAL", "0"), ("RETENTION_PRUNE_INTERVAL", "daily"), ("RETENTION_PRUNE_INTERVAL", "inf")] {
        let intervals = periodic::Intervals::from_lookup(|wanted| (wanted == name).then(|| value.into()));
        assert_eq!(intervals.periodic, None, "{name}={value}");
        assert_eq!(intervals.huddle, Some(Duration::from_secs(5)), "{name}={value}");
        let text = logs.text();
        assert!(text.contains(&format!("ERROR campfire::jobs::periodic: Periodic disabled error={name} must be a positive finite number")), "{text}");
    }
    let intervals = periodic::Intervals::from_lookup(|name| (name == "HUDDLE_RECONCILE_INTERVAL").then(|| "-5".into()));
    assert_eq!(intervals.huddle, None);
    assert!(intervals.periodic.is_some());
    let text = logs.text();
    assert!(text.contains("Huddle reconciliation disabled error=HUDDLE_RECONCILE_INTERVAL must be a positive finite number"), "{text}");

    // The huddle loop and the jobs run beside the disabled periodic loop.
    let mut loops = periodic::Loops::new(periodic::Intervals::from_lookup(|name| (name == "RETENTION_PRUNE_INTERVAL").then(|| "0".into())));
    assert!(loops.periodic.is_none());
    let (ticked, mut ticks) = mpsc::unbounded_channel();
    loops.huddle.as_mut().expect("the huddle loop runs").task(campfire_jobs::periodic::Task::new("reconcile", Duration::from_millis(20), move |_: App| {
        let ticked = ticked.clone();
        async move {
            let _ = ticked.send(());
            Ok(())
        }
    }));
    let config = runner_config(&app.config);
    let (_, ad_hoc) = Jobs::new(&registry(), &config).unwrap();
    let runner = start(app.clone(), registry(), ad_hoc, config, loops);
    tokio::time::timeout(Duration::from_secs(5), ticks.recv()).await.expect("the huddle loop ticks").unwrap();
    app.db
        .write(|tx| {
            tx.emit_after_commit(Event::RemoveBannedContent { user_id: 404 });
            Ok(())
        })
        .await
        .unwrap();
    wait_for(&app, "the job to run", |jobs| jobs.is_empty()).await;
    runner.shutdown(Duration::from_secs(5)).await;
    let logs = logs.text();
    assert!(logs.contains(r#"discarded job="RemoveBannedContentJob""#), "{logs}");
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

#[test]
fn ws8_periodic_tasks_match_rails_names_and_intervals() {
    let golden: serde_json::Value =
        serde_json::from_str(include_str!("../ws8_runtime_vectors.json")).unwrap();
    let periodic = periodic::periodic(periodic::PeriodicIntervals {
        reminders: Duration::from_secs(17),
        retention: Duration::from_secs(123),
    });
    let tasks: Vec<_> = periodic
        .tasks()
        .map(|t| serde_json::json!({"name":t.name(),"seconds":t.interval().as_secs()}))
        .collect();
    assert_eq!(serde_json::json!(tasks), golden["tasks"]);
}

#[tokio::test]
async fn ws8_quote_refresh_jobs_execute_in_the_real_app_runner() {
    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    app.db.write(|tx| {
        // Other periodic work can remain queued while the quote job completes.
        tx.emit_after_commit(Event::job_in(Duration::from_secs(3600), &campfire_db::models::retention::PruneJob {}));
        tx.emit_after_commit(Event::job(&campfire_db::models::message_reference::QuoteCardsRefreshJob { source_message_id: 999 }));
        assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Message::QuoteCardsRefreshJob'", [], |row| row.get::<_, i64>(0))?, 1);
        Ok(())
    }).await.unwrap();
    let rows = wait_for(&app, "quote refresh execution", |rows| {
        rows.iter()
            .all(|row| row.class != "Message::QuoteCardsRefreshJob")
            || rows
                .iter()
                .any(|row| row.class == "Message::QuoteCardsRefreshJob" && row.status == "failed")
    })
    .await;
    assert!(rows.iter().all(|row| row.class != "Message::QuoteCardsRefreshJob"), "{rows:?}");
    assert!(rows.iter().any(|row| row.class == "Retention::PruneJob" && row.run_at > campfire_db::Timestamp::from_jiff(app.clock.now())), "{rows:?}");
    booted.jobs.shutdown(Duration::from_secs(5)).await;
}

#[test]
fn ws8_template_free_broadcast_payloads_match_rails() {
    use campfire_db::broadcasts::{Broadcast, Streamable};
    let golden: serde_json::Value =
        serde_json::from_str(include_str!("../ws8_runtime_vectors.json")).unwrap();
    for row in golden["broadcasts"].as_array().unwrap() {
        if row["kind"] != "remove" { continue; }
        let event = Broadcast::remove(
            vec![Streamable::User(row["user_id"].as_i64().unwrap()), Streamable::Name("rooms".into())],
            row["target"].as_str().unwrap().into(),
        );
        assert_eq!(
            template_free_broadcast(&event),
            Some((
                row["stream"].as_str().unwrap().into(),
                row["payload"].clone()
            ))
        );
    }
}

#[test]
fn ws8_thread_unread_broadcasts_match_real_rails_callbacks() {
    use campfire_db::{ChannelThread, Config, Database, Env, Message, NewChannelThread, NewMessage, RecordingSink, ThreadMembership, fixtures};
    let golden: serde_json::Value = serde_json::from_str(include_str!("../ws8_runtime_vectors.json")).unwrap();
    let setup = golden["thread_broadcast"].clone();
    let sink = RecordingSink::new();
    let dir = tempfile::tempdir().unwrap();
    let now = campfire_db::Timestamp::parse_db("2026-03-10 12:00:00").unwrap();
    let db = Database::open(Config::new(dir.path().join("thread.sqlite3")), Env { sink: std::sync::Arc::new(sink.clone()), ..Env::default() }).unwrap();
    db.write_blocking(move |tx| {
        fixtures::load(tx.conn(), &fixtures::reference_dir(), &fixtures::Options { now, bcrypt_cost: 4 })?;
        let thread = ChannelThread::create(tx, NewChannelThread {
            room_id: setup["room_id"].as_i64().unwrap(), creator_id: setup["creator_id"].as_i64().unwrap(),
            name: Some(setup["name"].as_str().unwrap().into()), ..Default::default()
        })?;
        ThreadMembership::join(tx, thread.id, setup["recipient_id"].as_i64().unwrap())?;
        Message::create(tx, NewMessage {
            room_id: thread.room_id, creator_id: thread.creator_id, thread_id: Some(thread.id),
            markdown_source: Some(setup["source"].as_str().unwrap().into()), ..Default::default()
        })?;
        Ok(())
    }).unwrap();
    let actual: Vec<_> = sink.events().iter().filter_map(|event| match event.as_broadcast()? {
        event @ campfire_db::broadcasts::Broadcast::Cable { .. } if event.stream_name().ends_with("_unread_threads") => {
            let (stream, payload) = template_free_broadcast(&event).unwrap();
            Some(serde_json::json!({"kind":"cable", "stream":stream, "payload":payload}))
        }
        _ => None,
    }).collect();
    let expected: Vec<_> = golden["broadcasts"].as_array().unwrap().iter().filter(|row| row["kind"] == "cable").cloned().collect();
    assert_eq!(actual, expected);
}

#[tokio::test]
async fn ws8_room_and_retention_workers_run_in_the_real_app() {
    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    app.db.write(|tx| {
        tx.emit_after_commit(Event::job(&campfire_db::models::room_delete::DestroyJob{room_id:999}));
        tx.emit_after_commit(Event::job(&campfire_db::models::retention::PruneJob{}));
        assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class IN ('Room::DestroyJob','Retention::PruneJob')",[],|r|r.get::<_,i64>(0))?,2);
        Ok(())
    }).await.unwrap();
    let rows = wait_for(&app, "maintenance jobs", |r| {
        r.is_empty() || r.iter().any(|r| r.status == "failed")
    })
    .await;
    assert!(rows.is_empty(), "{rows:?}");
    booted.jobs.shutdown(Duration::from_secs(5)).await;
}

#[tokio::test]
async fn ws8_storage_copies_match_rails_and_rollback_on_durable_enqueue_failure() {
    use campfire_db::models::forwarder::{self, Destination};
    use campfire_db::{Message, NewMessage, NewUser, Room, RoomType, User};
    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    booted.jobs.shutdown(Duration::from_secs(5)).await;
    let g: serde_json::Value =
        serde_json::from_str(include_str!("../ws8_runtime_vectors.json")).unwrap();
    let storage = app.storage.clone();
    let staged = storage
        .stage_bytes(
            b"WS8 copy\n",
            campfire_storage::Filename::new("ws8.txt"),
            Some("text/plain"),
        )
        .unwrap();
    // The source carries metadata that must survive copying. Use the Rails blob fields.
    let mut source = staged.blob().clone();
    source.metadata =
        campfire_storage::Json::parse(&g["attachment_copy"]["metadata"].to_string()).unwrap();
    let source_key = source.key.clone();
    let (message, dest) = app
        .db
        .write(move |tx| {
            let u = User::create(
                tx,
                NewUser {
                    name: "Copy sender".into(),
                    ..Default::default()
                },
            )?;
            let room = Room::create_for(tx, RoomType::Closed, Some("Copy source"), u.id, &[u.id])?;
            let dest = Room::create_for(
                tx,
                RoomType::Closed,
                Some("Copy destination"),
                u.id,
                &[u.id],
            )?;
            let saved = source
                .insert(tx.conn(), tx.now().jiff())
                .map_err(|e| campfire_db::Error::Other(e.to_string()))?;
            crate::active_storage::keep_after_commit(tx, staged);
            let message = Message::create(
                tx,
                NewMessage {
                    creator_id: u.id,
                    room_id: room.id,
                    body: Some("Attachment".into()),
                    attachment_blob_id: Some(saved.id),
                    ..Default::default()
                },
            )?;
            tx.conn().execute_batch("DELETE FROM background_jobs")?;
            Ok((message, dest.id))
        })
        .await
        .unwrap();
    let service = app.storage.service.clone();
    let copier = crate::messaging::ForwarderCopier::new(app.storage.clone());
    let msg = message.clone();
    let copy = app
        .db
        .write(move |tx| {
            Ok(forwarder::forward(
                tx,
                &msg,
                &[Destination::room(dest)],
                None,
                msg.creator_id,
                &copier,
            )?
            .unwrap()
            .remove(0)
            .message)
        })
        .await
        .unwrap();
    let blob = app
        .db
        .read(move |c| Ok(copy.attachment(c)?.unwrap().1))
        .await
        .unwrap();
    assert_ne!(blob.key, source_key);
    assert_eq!(
        service.download(&blob.key).unwrap(),
        service.download(&source_key).unwrap()
    );
    let expected = &g["attachment_copy"];
    assert_eq!(
        serde_json::json!({"filename":blob.filename,"content_type":blob.content_type,"byte_size":blob.byte_size,"checksum":blob.checksum,"metadata":serde_json::from_str::<serde_json::Value>(blob.metadata.as_deref().unwrap()).unwrap()}),
        serde_json::json!({"filename":expected["filename"],"content_type":expected["content_type"],"byte_size":expected["byte_size"],"checksum":expected["checksum"],"metadata":expected["metadata"]})
    );
    let before = app
        .db
        .read(|c| {
            Ok(
                c.query_row::<i64, _, _>("SELECT COUNT(*) FROM active_storage_blobs", [], |r| {
                    r.get(0)
                })?,
            )
        })
        .await
        .unwrap();
    app.db.write(|tx|{tx.conn().execute_batch("DELETE FROM background_jobs; CREATE TRIGGER reject_ws8_copy_job BEFORE INSERT ON background_jobs BEGIN SELECT RAISE(ABORT,'queue unavailable'); END")?;Ok(())}).await.unwrap();
    let copier = crate::messaging::ForwarderCopier::new(app.storage.clone());
    assert!(
        app.db
            .write(move |tx| forwarder::forward(
                tx,
                &message,
                &[Destination::room(dest)],
                None,
                message.creator_id,
                &copier
            ))
            .await
            .is_err()
    );
    let count = app
        .db
        .read(|c| {
            Ok(
                c.query_row::<i64, _, _>("SELECT COUNT(*) FROM active_storage_blobs", [], |r| {
                    r.get(0)
                })?,
            )
        })
        .await
        .unwrap();
    assert_eq!(count, before);
    // The failed copy has no row whose key can be queried; inspect real storage files.
    fn file_count(path: &std::path::Path) -> usize {
        std::fs::read_dir(path)
            .unwrap()
            .map(|e| e.unwrap().path())
            .map(|p| if p.is_dir() { file_count(&p) } else { 1 })
            .sum()
    }
    assert_eq!(file_count(service.root()), before as usize);
    assert!(service.exist(&source_key));
}

#[tokio::test]
async fn ws8_periodic_row_failures_continue_like_rails() {
    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    booted.jobs.shutdown(Duration::from_secs(5)).await;
    let g: serde_json::Value =
        serde_json::from_str(include_str!("../ws8_loop_vectors.json")).unwrap();
    let now = campfire_db::Timestamp::parse_db(g["now"].as_str().unwrap()).unwrap();
    let sql = g["setup_sql"].as_str().unwrap().to_owned();
    app.db
        .write(move |tx| {
            campfire_db::fixtures::load(
                tx.conn(),
                &campfire_db::fixtures::reference_dir(),
                &campfire_db::fixtures::Options {
                    now,
                    bcrypt_cost: 4,
                },
            )?;
            tx.conn().execute_batch("PRAGMA defer_foreign_keys=ON")?;
            tx.conn().execute_batch(&sql)?;
            Ok(())
        })
        .await
        .unwrap();
    periodic::saved_item_reminders(&app.db).await.unwrap();
    periodic::scheduled_messages(&app.db).await.unwrap();
    periodic::poll_closing(&app.db).await.unwrap();
    for check in g["checks"].as_array().unwrap() {
        let sql = check["sql"].as_str().unwrap().to_owned();
        let rows = app
            .db
            .read(move |c| {
                let mut stmt = c.prepare(&sql)?;
                let n = stmt.column_count();
                Ok(stmt
                    .query_map([], |r| {
                        (0..n)
                            .map(|i| r.get::<_, i64>(i))
                            .collect::<rusqlite::Result<Vec<_>>>()
                    })?
                    .collect::<rusqlite::Result<Vec<_>>>()?)
            })
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_value(rows).unwrap(),
            check["rows"],
            "{}",
            check["sql"]
        );
    }
}
