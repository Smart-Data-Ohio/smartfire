use campfire_db::{
    Clock, Database, Event, EventSink, Message, RecordingSink, Room, TestClock, Timestamp, Tx,
    fixtures,
};
use campfire_jobs::{JobQueue, Outcome, QueueConfig, Registry, RunnerConfig};
use campfire_mail::{
    config::Config,
    inbound::{self, Routed, Status, Throttle},
    jobs::{DeliveryJob, IncinerationJob, MessageCreated, RoutingJob},
    parse::{Email, MAX_ATTACHMENT_BYTES, Verdict, attachment_verdict},
};
use campfire_storage::{DiskService, Storage, Verifier};
use std::{sync::Arc, time::Duration};

fn review_fixture(key: &str) -> String {
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("../../../vectors/mail/reference.json")).unwrap();
    corpus["review"][key].as_str().unwrap().to_owned()
}

fn busy(_: &campfire_db::Connection, _: &Room, _: &str) -> campfire_db::Result<String> {
    Err(campfire_db::Error::Sqlite(rusqlite::Error::SqliteFailure(
        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
        None,
    )))
}

#[tokio::test]
async fn review_transient_failure_remains_retryable() {
    let h = Harness::new().await;
    let raw = review_fixture("retry_raw").replace(
        "room-token@mail.test",
        &format!("room-{}@mail.test", h.token),
    );
    let id = inbound::accept(&h.db, h.storage.clone(), raw.into_bytes())
        .await
        .unwrap()
        .unwrap();
    let error = inbound::route(
        &h.db,
        h.storage.clone(),
        h.config.clone(),
        h.throttle.clone(),
        Some(Arc::new(busy)),
        id,
    )
    .await
    .unwrap_err();
    assert!(campfire_jobs::is_transient(&error));
    let (status, incinerations) = h.db.read(move |c| Ok((
        c.query_row("SELECT status FROM action_mailbox_inbound_emails WHERE id = ?", [id], |r| r.get::<_, i64>(0))?,
        c.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class = 'ActionMailbox::IncinerationJob'", [], |r| r.get::<_, i64>(0))?,
    ))).await.unwrap();
    assert_eq!((status, incinerations), (Status::Pending as i64, 0));
    let retry = inbound::route(
        &h.db,
        h.storage.clone(),
        h.config.clone(),
        h.throttle.clone(),
        Some(Arc::new(render)),
        id,
    )
    .await
    .unwrap();
    println!("retry result: {retry:?}");
    assert!(
        matches!(retry, Routed::Posted(_)),
        "transient retry returned {retry:?}"
    );
    let count =
        h.db.read(|c| {
            Ok(c.query_row(
                "SELECT COUNT(*) FROM messages WHERE markdown_source LIKE '%Retry body%'",
                [],
                |r| r.get::<_, i64>(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[derive(Clone)]
struct RoutingContext {
    db: Database,
    storage: Arc<Storage>,
    config: Config,
    throttle: Throttle,
}

#[tokio::test]
async fn durable_routing_retries_sqlite_busy_then_posts_once() {
    let h = Harness::new().await;
    let raw = review_fixture("retry_raw").replace(
        "room-token@mail.test",
        &format!("room-{}@mail.test", h.token),
    );
    let id = inbound::accept(&h.db, h.storage.clone(), raw.into_bytes())
        .await
        .unwrap()
        .unwrap();
    let mut registry = Registry::<RoutingContext>::new();
    registry.register::<RoutingJob, _, _>(|context, job, execution| async move {
        let renderer: Arc<dyn inbound::Renderer> = if execution.executions == 1 {
            Arc::new(busy)
        } else {
            Arc::new(render)
        };
        inbound::route(
            &context.db,
            context.storage,
            context.config,
            context.throttle,
            Some(renderer),
            job.inbound_email_id,
        )
        .await?;
        Ok(Outcome::Done)
    });
    registry.register::<IncinerationJob, _, _>(|_, _, _| async { Ok(Outcome::Done) });
    registry.register::<MessageCreated, _, _>(|_, _, _| async { Ok(Outcome::Done) });
    let mut config = RunnerConfig::new(vec![QueueConfig::new("default", 1)]);
    config.poll = Duration::from_millis(10);
    let runner = campfire_jobs::start(
        h.db.clone(),
        h.sink.queue.clone(),
        registry,
        RoutingContext {
            db: h.db.clone(),
            storage: h.storage.clone(),
            config: h.config.clone(),
            throttle: h.throttle.clone(),
        },
        config,
    );
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let retry_ready = h.db.read(|c| Ok(c.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class = 'ActionMailbox::RoutingJob' AND status = 'ready' AND attempts = 1", [], |r| r.get::<_, i64>(0))?)).await.unwrap();
            if retry_ready == 1 { break; }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await.unwrap();
    let status =
        h.db.read(move |c| {
            Ok(c.query_row(
                "SELECT status FROM action_mailbox_inbound_emails WHERE id = ?",
                [id],
                |r| r.get::<_, i64>(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(status, Status::Pending as i64);
    h.clock.travel(jiff::SignedDuration::from_secs(4));
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let routes = h.db.read(|c| Ok(c.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class = 'ActionMailbox::RoutingJob'", [], |r| r.get::<_, i64>(0))?)).await.unwrap();
            if routes == 0 { break; }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await.unwrap();
    runner.shutdown(Duration::from_secs(1)).await;
    let (status, posts, failed_jobs) =
        h.db.read(move |c| {
            Ok((
                c.query_row(
                    "SELECT status FROM action_mailbox_inbound_emails WHERE id = ?",
                    [id],
                    |r| r.get::<_, i64>(0),
                )?,
                c.query_row(
                    "SELECT COUNT(*) FROM messages WHERE markdown_source LIKE '%Retry body%'",
                    [],
                    |r| r.get::<_, i64>(0),
                )?,
                c.query_row(
                    "SELECT COUNT(*) FROM background_jobs WHERE status = 'failed'",
                    [],
                    |r| r.get::<_, i64>(0),
                )?,
            ))
        })
        .await
        .unwrap();
    assert_eq!(
        (status, posts, failed_jobs),
        (Status::Delivered as i64, 1, 0)
    );
    assert_eq!(
        inbound::route(
            &h.db,
            h.storage.clone(),
            h.config.clone(),
            h.throttle.clone(),
            Some(Arc::new(render)),
            id
        )
        .await
        .unwrap(),
        Routed::AlreadyRouted
    );
}

#[tokio::test]
async fn review_deep_mime_parses_without_abort() {
    let raw = review_fixture("deep_mime_raw");
    assert_eq!(raw.len(), 134754);
    println!("fixture bytes: {}", raw.len());
    let result = tokio::task::spawn_blocking(move || Email::parse(raw.as_bytes()))
        .await
        .unwrap();
    assert_eq!(result.unwrap().body, "Hello");
}

#[tokio::test]
async fn review_multipart_matches_pinned_rails() {
    use base64::Engine as _;
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../vectors/mail/multipart.json")).unwrap();
    assert_eq!(cases.as_array().unwrap().len(), 6);
    for case in cases.as_array().unwrap() {
        let h = Harness::new().await;
        let raw = case["raw"].as_str().unwrap().replace(
            "room-token@mail.test",
            &format!("room-{}@mail.test", h.token),
        );
        let parsed = Email::parse_for_routing(raw.as_bytes()).unwrap();
        let message = h.message(h.deliver(raw.into_bytes()).await).await;
        assert_eq!(
            message.markdown_source.as_deref(),
            case["source"].as_str(),
            "fixture {}",
            case["label"]
        );
        let attached =
            h.db.read(move |c| {
                campfire_db::Attachment::find_for(c, "Message", message.id, "attachment")
            })
            .await
            .unwrap();
        if let Some(filename) = case["filename"].as_str() {
            let blob = h.db.read(move |c| attached.unwrap().blob(c)).await.unwrap();
            assert_eq!(blob.filename, filename);
            assert_eq!(
                std::fs::read(h.storage.service.path_for(&blob.key)).unwrap(),
                base64::engine::general_purpose::STANDARD
                    .decode(case["bytes"].as_str().unwrap())
                    .unwrap()
            );
        } else {
            assert!(attached.is_none());
        }
        assert_eq!(parsed.source(false).as_deref(), case["source"].as_str());
    }
}

#[tokio::test]
async fn review_routing_replay_does_not_duplicate_message() {
    let h = Harness::new().await;
    let raw = review_fixture("replay_raw").replace(
        "room-token@mail.test",
        &format!("room-{}@mail.test", h.token),
    );
    let id = inbound::accept(&h.db, h.storage.clone(), raw.into_bytes())
        .await
        .unwrap()
        .unwrap();
    for _ in 0..2 {
        let routed = inbound::route(
            &h.db,
            h.storage.clone(),
            h.config.clone(),
            h.throttle.clone(),
            Some(Arc::new(render)),
            id,
        )
        .await
        .unwrap();
        println!("routing result: {routed:?}");
    }
    let count =
        h.db.read(|c| {
            Ok(c.query_row(
                "SELECT COUNT(*) FROM messages WHERE markdown_source LIKE '%Once only%'",
                [],
                |r| r.get::<_, i64>(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(count, 1, "routing replay duplicated the email");
}

#[tokio::test]
async fn review_deep_mime_bounces_without_posting() {
    let h = Harness::new().await;
    let id = inbound::accept(
        &h.db,
        h.storage.clone(),
        review_fixture("deep_mime_raw").into_bytes(),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        inbound::route(
            &h.db,
            h.storage.clone(),
            h.config.clone(),
            h.throttle.clone(),
            Some(Arc::new(render)),
            id
        )
        .await
        .unwrap(),
        Routed::Bounced
    );
    let (status, count) =
        h.db.read(move |c| {
            Ok((
                c.query_row(
                    "SELECT status FROM action_mailbox_inbound_emails WHERE id = ?",
                    [id],
                    |r| r.get::<_, i64>(0),
                )?,
                c.query_row(
                    "SELECT COUNT(*) FROM messages WHERE markdown_source IS NOT NULL",
                    [],
                    |r| r.get::<_, i64>(0),
                )?,
            ))
        })
        .await
        .unwrap();
    assert_eq!((status, count), (Status::Bounced as i64, 0));
}

#[tokio::test]
async fn concurrent_routing_replays_post_and_enqueue_once() {
    let h = Harness::new().await;
    let id = inbound::accept(
        &h.db,
        h.storage.clone(),
        h.raw("outside@example.com", &[], "Replay", "Once only"),
    )
    .await
    .unwrap()
    .unwrap();
    let route = || {
        inbound::route(
            &h.db,
            h.storage.clone(),
            h.config.clone(),
            h.throttle.clone(),
            Some(Arc::new(render)),
            id,
        )
    };
    let (first, second) = tokio::join!(route(), route());
    let results = [first.unwrap(), second.unwrap()];
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Routed::Posted(_)))
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|r| **r == Routed::AlreadyRouted)
            .count(),
        1
    );
    let counts =
        h.db.read(|c| {
            [
                "Smartfire::EmailMessageCreatedJob",
                "ActionMailbox::IncinerationJob",
            ]
            .map(|class| {
                c.query_row(
                    "SELECT COUNT(*) FROM background_jobs WHERE job_class = ?",
                    [class],
                    |r| r.get::<_, i64>(0),
                )
            })
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
        })
        .await
        .unwrap();
    assert_eq!(counts, [1, 1]);
}

struct Sink {
    queue: JobQueue,
    record: RecordingSink,
}
impl EventSink for Sink {
    fn persist(&self, tx: &Tx<'_>, event: &Event) -> campfire_db::Result<()> {
        if let Event::Job(request) = event {
            self.queue.enqueue(tx, request)?;
        }
        Ok(())
    }
    fn emit(&self, event: Event) {
        if let Event::Job(request) = &event {
            self.queue.wake(request.class);
        }
        self.record.emit(event);
    }
}
struct UnusedVerifier;
impl Verifier for UnusedVerifier {
    fn generate(&self, _: &str, _: &str, _: Option<jiff::Timestamp>) -> String {
        panic!("these tests never mint URLs")
    }
    fn verified(&self, _: &str, _: &str, _: jiff::Timestamp) -> Option<String> {
        None
    }
}
struct Harness {
    db: Database,
    storage: Arc<Storage>,
    config: Config,
    throttle: Throttle,
    clock: TestClock,
    sink: Arc<Sink>,
    token: String,
    room_id: i64,
    _dir: tempfile::TempDir,
}
impl Harness {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let clock = TestClock::frozen_at(Timestamp::from_second(1_790_685_000));
        let mut registry = Registry::<()>::new();
        registry.register::<RoutingJob, _, _>(|_, _, _| async { Ok(Outcome::Done) });
        registry.register::<IncinerationJob, _, _>(|_, _, _| async { Ok(Outcome::Done) });
        registry.register::<MessageCreated, _, _>(|_, _, _| async { Ok(Outcome::Done) });
        registry.register::<DeliveryJob, _, _>(|_, _, _| async { Ok(Outcome::Done) });
        let queue = JobQueue::new(
            &registry,
            &RunnerConfig::new(vec![QueueConfig::new("default", 1)]),
        )
        .unwrap();
        let sink = Arc::new(Sink {
            queue,
            record: RecordingSink::default(),
        });
        let db = Database::open(
            campfire_db::Config::new(dir.path().join("test.sqlite3")),
            campfire_db::Env {
                clock: Arc::new(clock.clone()),
                sink: sink.clone(),
                bcrypt_cost: 4,
                ..Default::default()
            },
        )
        .unwrap();
        db.write(|tx| {
            fixtures::load(
                tx.conn(),
                &fixtures::reference_dir(),
                &fixtures::Options {
                    now: tx.now(),
                    bcrypt_cost: 4,
                },
            )
            .map(|_| ())
        })
        .await
        .unwrap();
        let room_id = fixtures::identify("designers");
        let token = db
            .write(move |tx| Room::find(tx.conn(), room_id)?.regenerate_inbound_email_token(tx))
            .await
            .unwrap();
        let storage = Arc::new(Storage::new(
            DiskService::new(dir.path().join("files"), "local"),
            Arc::new(UnusedVerifier),
        ));
        Self {
            db,
            storage,
            config: Config {
                domain: Some("mail.test".into()),
                authserv_id: Some("mx.mail.test".into()),
                ..Default::default()
            },
            throttle: Throttle::default(),
            clock,
            sink,
            token,
            room_id,
            _dir: dir,
        }
    }
    fn raw(&self, from: &str, results: &[&str], subject: &str, body: &str) -> Vec<u8> {
        let mut lines = vec![
            format!("From: {from}"),
            format!("To: room-{}@mail.test", self.token),
            format!("Subject: {subject}"),
        ];
        lines.extend(
            results
                .iter()
                .map(|h| format!("Authentication-Results: {h}")),
        );
        lines.extend([
            "Content-Type: text/plain; charset=utf-8".into(),
            "".into(),
            body.into(),
        ]);
        lines.join("\r\n").into_bytes()
    }
    async fn deliver(&self, raw: Vec<u8>) -> Routed {
        let id = inbound::accept(&self.db, self.storage.clone(), raw)
            .await
            .unwrap()
            .unwrap();
        inbound::route(
            &self.db,
            self.storage.clone(),
            self.config.clone(),
            self.throttle.clone(),
            Some(Arc::new(render)),
            id,
        )
        .await
        .unwrap()
    }
    async fn message(&self, routed: Routed) -> Message {
        let Routed::Posted(id) = routed else {
            panic!("{routed:?}")
        };
        self.db.read(move |c| Message::find(c, id)).await.unwrap()
    }
}
// The shared Markdown renderer is injected, not under test here. Posting still runs the real
// WS2 Message API, callbacks, queue transactions and disk storage. Renderer parity is WS5's gate.
fn render(_: &campfire_db::Connection, _: &Room, source: &str) -> campfire_db::Result<String> {
    Ok(format!(
        "<p>{}</p>",
        source
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    ))
}

macro_rules! member_test {
    ($name:ident, $from:expr, $headers:expr, $member:expr) => {
        #[tokio::test]
        async fn $name() {
            let h = Harness::new().await;
            let message = h
                .message(
                    h.deliver(h.raw($from, $headers, "Launch update", "We ship Friday."))
                        .await,
                )
                .await;
            let creator =
                h.db.read(move |c| campfire_db::User::find(c, message.creator_id))
                    .await
                    .unwrap();
            if $member {
                assert_eq!(creator.id, fixtures::identify("david"));
            } else {
                assert!(creator.is_bot());
                assert_eq!(creator.name, "Email");
            }
        }
    };
}
member_test!(
    member_posts_as_member,
    "david@37signals.com",
    &["mx.mail.test; dkim=pass header.d=37signals.com"],
    true
);
member_test!(
    member_address_is_case_insensitive,
    "David@37Signals.com",
    &["mx.mail.test; dkim=pass header.d=37signals.com"],
    true
);
member_test!(
    member_without_pass_posts_as_bot,
    "david@37signals.com",
    &[],
    false
);
member_test!(
    dmarc_member_pass,
    "david@37signals.com",
    &["mx.mail.test; dmarc=pass (p=REJECT) header.from=37signals.com"],
    true
);
member_test!(
    foreign_domain_pass_is_bot,
    "david@37signals.com",
    &["mx.mail.test; dkim=pass header.d=evil.test"],
    false
);
member_test!(
    foreign_authserv_is_bot,
    "david@37signals.com",
    &["foreign.test; dkim=pass header.d=37signals.com"],
    false
);
member_test!(
    spf_helo_is_bot,
    "david@37signals.com",
    &["mx.mail.test; spf=pass smtp.helo=37signals.com"],
    false
);
member_test!(
    spf_mailfrom_is_member,
    "david@37signals.com",
    &["mx.mail.test; spf=pass smtp.mailfrom=david@37signals.com"],
    true
);
member_test!(
    forged_pass_above_relay_failure_is_bot,
    "david@37signals.com",
    &[
        "attacker.test; dkim=pass header.d=37signals.com",
        "mx.mail.test; dkim=fail header.d=37signals.com"
    ],
    false
);
member_test!(
    relay_pass_below_foreign_failure_is_member,
    "david@37signals.com",
    &[
        "attacker.test; dkim=fail header.d=37signals.com",
        "mx.mail.test; dkim=pass header.d=37signals.com"
    ],
    true
);
member_test!(
    non_member_is_bot,
    "Outsider <outside@example.com>",
    &[],
    false
);
#[tokio::test]
async fn unconfigured_authserv_is_bot() {
    let mut h = Harness::new().await;
    h.config.authserv_id = None;
    let m = h
        .message(
            h.deliver(h.raw(
                "david@37signals.com",
                &["mx.mail.test; dkim=pass header.d=37signals.com"],
                "",
                "Hello",
            ))
            .await,
        )
        .await;
    assert_ne!(m.creator_id, fixtures::identify("david"));
}
#[tokio::test]
async fn member_of_another_room_is_bot() {
    let h = Harness::new().await;
    let room = h.room_id;
    h.db.write(move |tx| {
        tx.conn().execute(
            "DELETE FROM memberships WHERE room_id = ? AND user_id = ?",
            rusqlite::params![room, fixtures::identify("kevin")],
        )?;
        Ok(())
    })
    .await
    .unwrap();
    let m = h
        .message(
            h.deliver(h.raw(
                "kevin@37signals.com",
                &["mx.mail.test; dkim=pass header.d=37signals.com"],
                "",
                "Hello",
            ))
            .await,
        )
        .await;
    assert_ne!(m.creator_id, fixtures::identify("kevin"));
}
#[tokio::test]
async fn unknown_token_drops_silently() {
    let mut h = Harness::new().await;
    h.token = "bogus".into();
    assert_eq!(
        h.deliver(h.raw("david@37signals.com", &[], "", "Hello"))
            .await,
        Routed::Dropped
    );
}
#[tokio::test]
async fn disabled_domain_drops_silently() {
    let mut h = Harness::new().await;
    h.config.domain = None;
    assert_eq!(
        h.deliver(h.raw("david@37signals.com", &[], "", "Hello"))
            .await,
        Routed::Dropped
    );
}
#[tokio::test]
async fn rotated_token_retires_old_address() {
    let h = Harness::new().await;
    let room = h.room_id;
    let next =
        h.db.write(move |tx| Room::find(tx.conn(), room)?.regenerate_inbound_email_token(tx))
            .await
            .unwrap();
    assert_ne!(next, h.token);
    assert_eq!(next.len(), 32);
    assert!(next.chars().all(|c| c.is_ascii_hexdigit()));
    assert_eq!(
        h.deliver(h.raw("david@37signals.com", &[], "", "Hello"))
            .await,
        Routed::Dropped
    );
}
macro_rules! drop_room {
    ($test:ident, $assignment:expr) => {
        #[tokio::test]
        async fn $test() {
            let h = Harness::new().await;
            let id = h.room_id;
            h.db.write(move |tx| {
                tx.conn().execute(
                    &format!("UPDATE rooms SET {} WHERE id = ?", $assignment),
                    [id],
                )?;
                Ok(())
            })
            .await
            .unwrap();
            assert_eq!(
                h.deliver(h.raw("david@37signals.com", &[], "", "Hello"))
                    .await,
                Routed::Dropped
            );
        }
    };
}
drop_room!(deleted_room_drops, "deleted_at = '2026-09-29 12:00:00'");
drop_room!(board_room_drops, "type = 'Rooms::Board'");
drop_room!(direct_room_drops, "type = 'Rooms::Direct'");
#[tokio::test]
async fn html_only_mail_is_text() {
    let h = Harness::new().await;
    let raw = String::from_utf8(h.raw(
        "outside@example.com",
        &[],
        "Note",
        "<p>Hello <b>there</b></p><script>bad()</script>",
    ))
    .unwrap()
    .replace("text/plain", "text/html");
    let m = h.message(h.deliver(raw.into_bytes()).await).await;
    let source = m.markdown_source.unwrap();
    assert!(source.contains("Hello there"));
    assert!(!source.contains("bad()"));
    assert!(!source.contains("<b>"));
}
fn attachment_mail(h: &Harness, filename: &str, mime: &str, body: &str) -> Vec<u8> {
    format!("From: outside@example.com\r\nTo: room-{}@mail.test\r\nSubject: File\r\nMIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=file\r\n\r\n--file\r\nContent-Type: text/plain\r\n\r\nSee attached.\r\n--file\r\nContent-Type: {mime}\r\nContent-Disposition: attachment; filename=\"{filename}\"\r\nContent-Transfer-Encoding: base64\r\n\r\n{body}\r\n--file--\r\n", h.token).into_bytes()
}
macro_rules! attachment_test {
    ($test:ident, $filename:expr, $mime:expr, $bytes:expr, $attached:expr, $note:expr) => {
        #[tokio::test]
        async fn $test() {
            use base64::Engine as _;
            let h = Harness::new().await;
            let bytes: Vec<u8> = $bytes;
            let raw = attachment_mail(
                &h,
                $filename,
                $mime,
                &base64::engine::general_purpose::STANDARD.encode(&bytes),
            );
            let m = h.message(h.deliver(raw).await).await;
            assert!(m.markdown_source.as_deref().unwrap().contains($note));
            let attachment = h
                .db
                .read(move |c| campfire_db::Attachment::find_for(c, "Message", m.id, "attachment"))
                .await
                .unwrap();
            assert_eq!(attachment.is_some(), $attached);
            if let Some(a) = attachment {
                let blob = h.db.read(move |c| a.blob(c)).await.unwrap();
                assert_eq!(blob.filename, $filename);
                assert_eq!(
                    std::fs::read(h.storage.service.path_for(&blob.key)).unwrap(),
                    bytes
                );
            }
        }
    };
}
attachment_test!(
    text_attachment_lands,
    "notes.txt",
    "text/plain",
    b"file-bytes".to_vec(),
    true,
    "notes.txt"
);
attachment_test!(
    near_limit_survives_encoding,
    "near.docx",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    vec![b'x'; MAX_ATTACHMENT_BYTES - 50_000],
    true,
    "near.docx"
);
attachment_test!(
    oversized_is_named,
    "big.bin",
    "application/octet-stream",
    vec![b'x'; MAX_ATTACHMENT_BYTES + 1],
    false,
    "big.bin (not attached: over the 10 MB limit)"
);
attachment_test!(
    executable_is_named,
    "tool.exe",
    "application/x-msdownload",
    b"MZ-bytes".to_vec(),
    false,
    "tool.exe (not attached: file type not allowed)"
);
attachment_test!(
    office_attachment_lands,
    "report.docx",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    b"PK-bytes".to_vec(),
    true,
    "report.docx"
);
attachment_test!(
    image_attachment_lands,
    "photo.png",
    "image/png",
    {
        use base64::Engine as _;
        base64::engine::general_purpose::STANDARD.decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==").unwrap()
    },
    true,
    "photo.png"
);
#[test]
fn attachment_limit_and_allowlist_boundaries() {
    assert_eq!(
        attachment_verdict("image/png", MAX_ATTACHMENT_BYTES, 0),
        Verdict::Ok
    );
    assert_eq!(
        attachment_verdict("text/plain", MAX_ATTACHMENT_BYTES + 1, 0),
        Verdict::TooBig
    );
    assert_eq!(
        attachment_verdict("application/x-sh", 1, 1),
        Verdict::Disallowed
    );
    assert_eq!(attachment_verdict("APPLICATION/PDF", 1, 1), Verdict::Ok);
}
#[tokio::test]
async fn room_rate_limit_is_thirty_and_resets_each_hour() {
    let h = Harness::new().await;
    for i in 0..30 {
        assert!(matches!(
            h.deliver(h.raw(&format!("outside{i}@example.com"), &[], "", "Hello"))
                .await,
            Routed::Posted(_)
        ));
    }
    assert_eq!(
        h.deliver(h.raw("late@example.com", &[], "", "Hello")).await,
        Routed::Dropped
    );
    h.clock
        .travel_to(Timestamp::from_second(h.clock.now().as_second() + 3600));
    assert!(matches!(
        h.deliver(h.raw("next@example.com", &[], "", "Hello")).await,
        Routed::Posted(_)
    ));
}
#[tokio::test]
async fn empty_mail_posts_nothing() {
    let h = Harness::new().await;
    assert_eq!(
        h.deliver(h.raw("david@37signals.com", &[], "", "  ")).await,
        Routed::Dropped
    );
}
#[tokio::test]
async fn mentioned_legacy_bot_leaves_durable_fanout_hook() {
    let h = Harness::new().await;
    let m = h
        .message(
            h.deliver(h.raw(
                "david@37signals.com",
                &[],
                "",
                "Hey @[Legacy Bot], look at this.",
            ))
            .await,
        )
        .await;
    assert!(h.sink.record.events().iter().any(|event| matches!(event, Event::Job(r) if r.decode::<MessageCreated>().is_some_and(|j| j.unwrap().message_id == m.id))));
}
#[tokio::test]
async fn bounce_posts_nothing_and_never_sends_a_reply() {
    let h = Harness::new().await;
    let raw = h.raw("spam@example.com", &[], "", "Buy this.");
    let raw = String::from_utf8(raw)
        .unwrap()
        .replace(&format!("room-{}@mail.test", h.token), "hello@mail.test")
        .into_bytes();
    assert_eq!(h.deliver(raw).await, Routed::Bounced);
    assert!(
        !h.sink
            .record
            .events()
            .iter()
            .any(|e| matches!(e, Event::Job(r) if r.class == "Smartfire::MailDeliveryJob"))
    );
    let status =
        h.db.read(|c| {
            Ok(c.query_row(
                "SELECT status FROM action_mailbox_inbound_emails ORDER BY id DESC LIMIT 1",
                [],
                |r| r.get::<_, i64>(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(status, 4);
}
#[tokio::test]
async fn raw_storage_deduplication_and_incineration_are_durable() {
    let h = Harness::new().await;
    let raw = h.raw("spam@example.com", &[], "", "Hello");
    let id = inbound::accept(&h.db, h.storage.clone(), raw.clone())
        .await
        .unwrap()
        .unwrap();
    assert!(
        inbound::accept(&h.db, h.storage.clone(), raw.clone())
            .await
            .unwrap()
            .is_none()
    );
    let blob =
        h.db.read(move |c| {
            campfire_db::Attachment::find_for(c, inbound::RECORD_TYPE, id, "raw_email")?
                .unwrap()
                .blob(c)
        })
        .await
        .unwrap();
    assert_eq!(
        std::fs::read(h.storage.service.path_for(&blob.key)).unwrap(),
        raw
    );
    assert!(
        !h.db
            .write(move |tx| inbound::incinerate(tx, id))
            .await
            .unwrap()
    );
    h.db.write(move |tx| inbound::set_status(tx, id, Status::Delivered))
        .await
        .unwrap();
    assert!(h.sink.record.events().iter().any(|e| matches!(e, Event::Job(r) if r.class == "ActionMailbox::IncinerationJob" && r.wait == Some(Duration::from_secs(30*86400)))));
    h.clock.travel_to(Timestamp::from_second(
        h.clock.now().as_second() + 30 * 86400,
    ));
    assert!(
        h.db.write(move |tx| inbound::incinerate(tx, id))
            .await
            .unwrap()
    );
    assert!(
        h.sink
            .record
            .events()
            .iter()
            .any(|e| matches!(e, Event::PurgeBlob {blob_id} if *blob_id == blob.id))
    );
    assert!(
        !h.db
            .write(move |tx| inbound::incinerate(tx, id))
            .await
            .unwrap()
    );
}
#[tokio::test]
async fn token_rotation_touches_room_timestamp() {
    let h = Harness::new().await;
    let id = h.room_id;
    h.clock
        .travel_to(Timestamp::from_second(h.clock.now().as_second() + 10));
    let now = h.clock.now();
    h.db.write(move |tx| Room::find(tx.conn(), id)?.regenerate_inbound_email_token(tx))
        .await
        .unwrap();
    assert_eq!(
        h.db.read(move |c| Room::find(c, id))
            .await
            .unwrap()
            .updated_at,
        now
    );
}
#[tokio::test]
async fn email_bot_only_joins_target_room() {
    let h = Harness::new().await;
    let m = h
        .message(
            h.deliver(h.raw("outside@example.com", &[], "", "Hello"))
                .await,
        )
        .await;
    let count =
        h.db.read(move |c| {
            Ok(c.query_row(
                "SELECT COUNT(*) FROM memberships WHERE user_id = ?",
                [m.creator_id],
                |r| r.get::<_, i64>(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(count, 1);
}
#[tokio::test]
async fn processing_failure_marks_failed_and_schedules_incineration() {
    let h = Harness::new().await;
    let id = inbound::accept(
        &h.db,
        h.storage.clone(),
        h.raw("outside@example.com", &[], "", "Hello"),
    )
    .await
    .unwrap()
    .unwrap();
    fn broken(_: &campfire_db::Connection, _: &Room, _: &str) -> campfire_db::Result<String> {
        Err(campfire_db::Error::Other(
            "deliberate renderer failure".into(),
        ))
    }
    assert!(
        inbound::route(
            &h.db,
            h.storage.clone(),
            h.config.clone(),
            h.throttle.clone(),
            Some(Arc::new(broken)),
            id
        )
        .await
        .is_err()
    );
    assert_eq!(
        h.db.read(move |c| Ok(c.query_row(
            "SELECT status FROM action_mailbox_inbound_emails WHERE id = ?",
            [id],
            |r| r.get::<_, i64>(0)
        )?))
        .await
        .unwrap(),
        3
    );
}
#[tokio::test]
async fn notification_enqueue_rolls_back_with_triggering_write() {
    let h = Harness::new().await;
    let before =
        h.db.read(|c| {
            Ok(
                c.query_row("SELECT COUNT(*) FROM background_jobs", [], |r| {
                    r.get::<_, i64>(0)
                })?,
            )
        })
        .await
        .unwrap();
    assert!(
        h.db.write(|tx| {
            campfire_mail::jobs::lockout_notice_later(tx, 1);
            Err::<(), _>(campfire_db::Error::Other("roll back".into()))
        })
        .await
        .is_err()
    );
    assert_eq!(
        h.db.read(|c| Ok(
            c.query_row("SELECT COUNT(*) FROM background_jobs", [], |r| r
                .get::<_, i64>(0))?
        ))
        .await
        .unwrap(),
        before
    );
    h.db.write(|tx| {
        campfire_mail::jobs::new_sign_in_alert_later(tx, 7);
        campfire_mail::jobs::lockout_notice_later(tx, 9);
        Ok(())
    })
    .await
    .unwrap();
    assert_eq!(
        h.db.read(|c| Ok(
            c.query_row("SELECT COUNT(*) FROM background_jobs", [], |r| r
                .get::<_, i64>(0))?
        ))
        .await
        .unwrap(),
        before + 2
    );
}
#[test]
fn one_attachment_is_selected_and_all_are_named() {
    let raw = b"From: a@example.com\r\nTo: room-token@mail.test\r\nContent-Type: multipart/mixed; boundary=m\r\n\r\n--m\r\nContent-Type: text/plain\r\n\r\nHello\r\n--m\r\nContent-Type: text/plain\r\nContent-Disposition: attachment; filename=one.txt\r\n\r\none\r\n--m\r\nContent-Type: application/pdf\r\nContent-Disposition: attachment; filename=two.pdf\r\n\r\ntwo\r\n--m--\r\n";
    let e = Email::parse(raw).unwrap();
    assert_eq!(
        e.files
            .iter()
            .find(|f| f.verdict == Verdict::Ok)
            .unwrap()
            .filename,
        "one.txt"
    );
    assert_eq!(
        e.attachment_note().unwrap(),
        "Attached files: one.txt, two.pdf"
    );
}
#[test]
fn plain_text_precedes_html_and_cc_and_bcc_route() {
    let raw = b"From: a@example.com\r\nCc: room-token@mail.test\r\nContent-Type: multipart/alternative; boundary=m\r\n\r\n--m\r\nContent-Type: text/html\r\n\r\n<b>HTML</b>\r\n--m\r\nContent-Type: text/plain\r\n\r\nplain\r\n--m--\r\n";
    let e = Email::parse(raw).unwrap();
    assert_eq!(e.body, "plain");
    assert_eq!(e.room_token().unwrap(), "token");
}
#[test]
fn source_truncation_counts_unicode_characters() {
    let raw = format!(
        "From: a@example.com\r\nTo: room-token@mail.test\r\nContent-Type: text/plain; charset=UTF-8\r\n\r\n{}",
        "é".repeat(50_050)
    );
    let e = Email::parse(raw.as_bytes()).unwrap();
    let source = e.source(true).unwrap();
    assert_eq!(source.chars().count(), 50_000);
    assert!(source.ends_with("..."));
}

#[tokio::test]
#[ignore = "exports a database for the Rails rollback check; requires CAMPFIRE_MAIL_EXPORT_DIR"]
async fn export_for_rails() {
    let out = std::path::PathBuf::from(
        std::env::var("CAMPFIRE_MAIL_EXPORT_DIR").expect("set CAMPFIRE_MAIL_EXPORT_DIR"),
    );
    std::fs::create_dir_all(out.join("db")).unwrap();
    let h = Harness::new().await;
    assert!(matches!(
        h.deliver(h.raw(
            "david@37signals.com",
            &["mx.mail.test; dkim=pass header.d=37signals.com"],
            "Launch",
            "Friday."
        ))
        .await,
        Routed::Posted(_)
    ));
    let raw = attachment_mail(&h, "notes.txt", "text/plain", "ZmlsZS1ieXRlcw==");
    assert!(matches!(h.deliver(raw).await, Routed::Posted(_)));
    let raw = String::from_utf8(h.raw("outside@example.com", &[], "", "Hello"))
        .unwrap()
        .replace(&format!("room-{}@mail.test", h.token), "nobody@mail.test");
    assert_eq!(h.deliver(raw.into_bytes()).await, Routed::Bounced);
    let conn = rusqlite::Connection::open(h.db.path()).unwrap();
    let target = out.join("db/test.sqlite3");
    assert!(!target.exists(), "choose an empty export directory");
    conn.execute_batch(&format!(
        "VACUUM INTO '{}'",
        target.to_string_lossy().replace('\'', "''")
    ))
    .unwrap();
    fn copy(from: &std::path::Path, to: &std::path::Path) {
        std::fs::create_dir_all(to).unwrap();
        for entry in std::fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let target = to.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), target).unwrap();
            }
        }
    }
    copy(h.storage.service.root(), &out.join("files"));
    println!(
        "mail rollback artifact: 2 posted messages, 3 inbound emails, raw email and attachment files"
    );
}

#[tokio::test]
async fn markdown_api_rejects_blank_and_overlong_sources() {
    let h = Harness::new().await;
    let room = h.room_id;
    for source in [" \n\t".to_owned(), "é".repeat(50_001)] {
        let error =
            h.db.write(move |tx| {
                Message::create_markdown(
                    tx,
                    campfire_db::NewMessage {
                        room_id: room,
                        creator_id: fixtures::identify("david"),
                        body: Some("<p>body</p>".into()),
                        ..Default::default()
                    },
                    &source,
                )
            })
            .await
            .unwrap_err();
        assert!(matches!(error, campfire_db::Error::RecordInvalid(_)));
    }
}

#[tokio::test]
async fn committed_attachment_survives_an_after_commit_callback_failure() {
    let h = Harness::new().await;
    let id = inbound::accept(
        &h.db,
        h.storage.clone(),
        attachment_mail(&h, "notes.txt", "text/plain", "ZmlsZS1ieXRlcw=="),
    )
    .await
    .unwrap()
    .unwrap();
    h.db.write(|tx| {
        tx.conn().execute_batch("DROP TABLE message_search_index")?;
        Ok(())
    })
    .await
    .unwrap();
    assert!(
        inbound::route(
            &h.db,
            h.storage.clone(),
            h.config.clone(),
            h.throttle.clone(),
            Some(Arc::new(render)),
            id
        )
        .await
        .is_err()
    );
    let blob=h.db.read(|c| {
        let id=c.query_row("SELECT id FROM messages WHERE markdown_source IS NOT NULL ORDER BY id DESC LIMIT 1",[],|r|r.get::<_,i64>(0))?;
        campfire_db::Attachment::find_for(c,"Message",id,"attachment")?.unwrap().blob(c)
    }).await.unwrap();
    assert_eq!(
        std::fs::read(h.storage.service.path_for(&blob.key)).unwrap(),
        b"file-bytes"
    );
    assert_eq!(
        h.db.read(move |c| Ok(c.query_row(
            "SELECT status FROM action_mailbox_inbound_emails WHERE id = ?",
            [id],
            |r| r.get::<_, i64>(0)
        )?))
        .await
        .unwrap(),
        Status::Delivered as i64
    );
    assert_eq!(
        inbound::route(
            &h.db,
            h.storage.clone(),
            h.config.clone(),
            h.throttle.clone(),
            Some(Arc::new(render)),
            id
        )
        .await
        .unwrap(),
        Routed::AlreadyRouted
    );
}

#[tokio::test]
async fn missing_renderer_retains_valid_mail_pending_without_consuming_throttle() {
    let h = Harness::new().await;
    let id = inbound::accept(
        &h.db,
        h.storage.clone(),
        h.raw("outside@example.com", &[], "Hello", "World"),
    )
    .await
    .unwrap()
    .unwrap();
    for _ in 0..31 {
        assert_eq!(
            inbound::route(
                &h.db,
                h.storage.clone(),
                h.config.clone(),
                h.throttle.clone(),
                None,
                id
            )
            .await
            .unwrap(),
            Routed::WaitingForRenderer
        );
    }
    assert_eq!(
        h.db.read(move |c| Ok(c.query_row(
            "SELECT status FROM action_mailbox_inbound_emails WHERE id = ?",
            [id],
            |r| r.get::<_, i64>(0)
        )?))
        .await
        .unwrap(),
        0
    );
    assert!(matches!(
        inbound::route(
            &h.db,
            h.storage.clone(),
            h.config.clone(),
            h.throttle.clone(),
            Some(Arc::new(render)),
            id
        )
        .await
        .unwrap(),
        Routed::Posted(_)
    ));
}
#[tokio::test]
async fn bounce_and_invalid_token_do_not_require_a_renderer() {
    let h = Harness::new().await;
    for (to, expected) in [
        ("nobody@mail.test", Routed::Bounced),
        ("room-unknown@mail.test", Routed::Dropped),
    ] {
        let raw = String::from_utf8(h.raw("outside@example.com", &[], "Hello", "World"))
            .unwrap()
            .replace(&format!("room-{}@mail.test", h.token), to);
        let id = inbound::accept(&h.db, h.storage.clone(), raw.into_bytes())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            inbound::route(
                &h.db,
                h.storage.clone(),
                h.config.clone(),
                h.throttle.clone(),
                None,
                id
            )
            .await
            .unwrap(),
            expected
        );
    }
}

#[tokio::test]
async fn stage_email_bot_membership_defaults_to_listener() {
    let h = Harness::new().await;
    let room = h.room_id;
    h.db.write(move |tx| {
        tx.conn().execute(
            "UPDATE rooms SET type = 'Rooms::Stage' WHERE id = ?",
            [room],
        )?;
        Ok(())
    })
    .await
    .unwrap();
    let m = h
        .message(
            h.deliver(h.raw("outside@example.com", &[], "Hello", "World"))
                .await,
        )
        .await;
    let role =
        h.db.read(move |c| {
            Ok(c.query_row(
                "SELECT stage_role FROM memberships WHERE room_id = ? AND user_id = ?",
                rusqlite::params![room, m.creator_id],
                |r| r.get::<_, String>(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(role, "listener");
}
