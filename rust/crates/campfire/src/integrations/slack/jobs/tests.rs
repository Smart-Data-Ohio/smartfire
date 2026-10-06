use super::*;
use campfire_db::models::slack::{NewConnection, SlackWorkspace};
use campfire_db::models::slack_import::{Kind, Mode, NewImport};
use campfire_db::{Config, Env, TestClock, Timestamp};
use campfire_jobs::{Execution, QueueConfig, Registry, RunnerConfig, inspect};
use serde_json::{Value, json};

pub(crate) async fn setup() -> (Database, Arc<ArEncryption>, tempfile::TempDir) {
    setup_at(
        Timestamp::parse_db("2026-03-02 16:00:00.123456").unwrap(),
        false,
    )
    .await
}
pub(crate) async fn setup_sequence() -> (Database, Arc<ArEncryption>, tempfile::TempDir) {
    setup_at(Timestamp::parse_db("2026-03-02 16:00:00").unwrap(), true).await
}
async fn setup_at(
    now: Timestamp,
    sequence: bool,
) -> (Database, Arc<ArEncryption>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let mut registry = Registry::<()>::new();
    registry
        .register(|_: (), _: ImportStep, _: Execution| async { Ok(campfire_jobs::Outcome::Done) });
    registry
        .register(|_: (), _: UndoStep, _: Execution| async { Ok(campfire_jobs::Outcome::Done) });
    let config = RunnerConfig::new(vec![
        QueueConfig::new("default", 2),
        QueueConfig::new("slack_import", 1),
    ]);
    let queue = campfire_jobs::JobQueue::new(&registry, &config).unwrap();
    struct Sink(campfire_jobs::JobQueue, bool);
    impl campfire_db::EventSink for Sink {
        fn persist(&self, tx: &campfire_db::Tx<'_>, event: &Event) -> campfire_db::Result<()> {
            if matches!(event, Event::Broadcast(_)) {
                return Err(campfire_db::Error::Other(
                    "Slack test captured an unintended broadcast".into(),
                ));
            }
            if let Some(request) = crate::queue::request_for(event)
                && !self.1
            {
                self.0.enqueue(tx, &request)?;
            }
            Ok(())
        }
        fn sync_message_references(
            &self,
            tx: &mut campfire_db::Tx<'_>,
            message: &campfire_db::Message,
            enqueue: bool,
        ) -> campfire_db::Result<()> {
            crate::integrations::sync_message_references(tx, message, enqueue, None)
        }
        fn emit(&self, _: Event) {}
    }
    let clock = TestClock::frozen_at(now);
    let uuid_index = std::sync::atomic::AtomicU64::new(0);
    let fixture_inputs = sequence.then(|| {
        Arc::new(campfire_db::database::FixtureInputs {
            sqlite_now: now,
            message_uuid: Arc::new(move || {
                format!(
                    "00000000-0000-4000-8000-{:012}",
                    uuid_index.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1
                )
            }),
        })
    });
    let db = Database::open(
        Config::new(dir.path().join("db.sqlite3")),
        Env {
            clock: Arc::new(clock),
            sink: Arc::new(Sink(queue, sequence)),
            fixture_inputs,
            rich_text: Arc::new(crate::rich_text::AppRichText::new(
                Arc::new(rails_compat::Secrets::new(
                    serde_json::from_str::<serde_json::Value>(include_str!(
                        "../../../../../../vectors/slack/crypto.json"
                    ))
                    .unwrap()["secret_key_base"]
                        .as_str()
                        .unwrap(),
                )),
                Arc::new(campfire_kit::clock::FrozenClock::new(now.jiff())),
            )),
            message_reference_syncs: vec![crate::integrations::github::references::sync],
            ..Default::default()
        },
    )
    .unwrap();
    let secret = if sequence {
        serde_json::from_str::<Value>(include_str!("../../../../../../vectors/slack/crypto.json"))
            .unwrap()["secret_key_base"]
            .as_str()
            .unwrap()
            .to_owned()
    } else {
        "ws16-job-key".repeat(32)
    };
    let encryption = Arc::new(ArEncryption::new(&rails_compat::Secrets::new(&secret)));
    let crypto = encryption.clone();
    db.write(move |tx| {
        tx.conn().execute(
            "INSERT INTO users (id,name,created_at,updated_at) VALUES (1,'Run owner',?,?)",
            rusqlite::params![tx.now(), tx.now()],
        )?;
        SlackWorkspace::create(tx, &crypto, "fixture-client", "fixture-secret", Some(1))?;
        SlackConnection::create(
            tx,
            &crypto,
            NewConnection {
                workspace_id: 1,
                user_id: 1,
                slack_user_id: "UOWNER",
                access_token: Some("fixture-user-token"),
                scopes: None,
            },
        )?;
        Ok(())
    })
    .await
    .unwrap();
    (db, encryption, dir)
}
pub(crate) async fn start(db: &Database) -> i64 {
    db.write(|tx| {
        Ok(SlackImport::create(
            tx,
            NewImport {
                workspace_id: 1,
                connection_id: Some(1),
                user_id: 1,
                kind: Kind::Workspace,
                mode: Mode::Import,
                options: json!({}),
            },
        )?
        .id)
    })
    .await
    .unwrap()
}
pub(crate) async fn run(db: &Database, id: i64) -> SlackImport {
    db.read(move |c| Ok(SlackImport::find(c, id)?.unwrap()))
        .await
        .unwrap()
}

#[tokio::test]
async fn slack_job_releases_before_durable_continuation_and_uses_serial_queue() {
    let (db, crypto, _dir) = setup().await;
    let id = start(&db).await;
    db.write(|tx| { tx.conn().execute_batch("CREATE TRIGGER require_released_slack_lease BEFORE INSERT ON background_jobs WHEN EXISTS(SELECT 1 FROM slack_imports WHERE json_extract(state, '$.step_lease_token') IS NOT NULL) BEGIN SELECT RAISE(ABORT,'continuation published while busy'); END;")?; Ok(()) }).await.unwrap();
    let executing = db.clone();
    perform_import(db.clone(), crypto, id, move |_, lease, token| async move {
        assert_eq!(token, "fixture-user-token");
        assert_eq!(run(&executing, id).await.state["step_lease_token"], lease);
        Ok(Outcome::Continue)
    })
    .await
    .unwrap();
    let row = run(&db, id).await;
    assert_eq!(row.status, "running");
    assert!(row.state.get("step_lease_token").is_none());
    let jobs = db.read(inspect::all).await.unwrap();
    assert_eq!(jobs.len(), 2);
    assert!(jobs.iter().all(|j| j.queue == "slack_import"));
    // The next execution can claim immediately, without waiting for the periodic sweep.
    let next = db
        .write(move |tx| SlackImport::acquire_step_lease(tx, id, StepStatus::Running))
        .await
        .unwrap();
    assert!(next.is_some());
}
#[tokio::test]
async fn slack_job_live_lease_and_lost_run_claim_never_execute_callback() {
    let (db, crypto, _dir) = setup().await;
    let first = start(&db).await;
    db.write(move |tx| {
        SlackImport::claim_running(tx, first)?;
        SlackImport::acquire_step_lease(tx, first, StepStatus::Running)?;
        Ok(())
    })
    .await
    .unwrap();
    let second = start(&db).await;
    for id in [first, second] {
        perform_import(db.clone(), crypto.clone(), id, |_, _, _| async {
            panic!("loser executed");
            #[allow(unreachable_code)]
            Ok(Outcome::Done)
        })
        .await
        .unwrap();
    }
    assert!(run(&db, second).await.state.get("enqueued_at").is_none());
    assert_eq!(run(&db, second).await.status, "queued");
}
#[tokio::test]
async fn slack_job_retry_after_commits_heartbeat_and_delayed_job_atomically() {
    let (db, crypto, _dir) = setup().await;
    let id = start(&db).await;
    perform_import(db.clone(), crypto, id, |_, _, _| async {
        let mut error = Error::new(ErrorKind::RateLimited, "Slack rate limit reached");
        error.retry_after = Some(73);
        Err(error.into())
    })
    .await
    .unwrap();
    let row = run(&db, id).await;
    assert_eq!(row.status, "running");
    assert!(row.state.get("step_lease_token").is_none());
    let jobs = db.read(inspect::all).await.unwrap();
    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[1].run_at.as_second() - jobs[0].run_at.as_second(), 73);
    assert_eq!(row.heartbeat_at, Some(row.started_at.unwrap()));
}
#[tokio::test]
async fn slack_job_cancel_during_auth_error_preserves_cancel_and_disconnects() {
    let (db, crypto, _dir) = setup().await;
    let id = start(&db).await;
    let executing = db.clone();
    perform_import(db.clone(), crypto, id, move |_, _, _| async move {
        executing
            .write(move |tx| SlackImport::cancel(tx, id))
            .await?;
        Err(Error::new(ErrorKind::Auth, "Slack token was revoked").into())
    })
    .await
    .unwrap();
    assert_eq!(run(&db, id).await.status, "cancelled");
    assert!(run(&db, id).await.error.is_none());
    let reason: Option<String> = db
        .read(|c| {
            Ok(c.query_row(
                "SELECT disconnected_reason FROM slack_connections WHERE id=1",
                [],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(reason.as_deref(), Some("Slack token was revoked"));
    assert!(run(&db, id).await.state.get("step_lease_token").is_none());
}
#[tokio::test]
async fn slack_job_missing_disconnected_and_finished_runs_do_not_fetch() {
    let (db, crypto, _dir) = setup().await;
    perform_import(db.clone(), crypto.clone(), 999, |_, _, _| async {
        panic!("missing fetched");
        #[allow(unreachable_code)]
        Ok(Outcome::Done)
    })
    .await
    .unwrap();
    let id = start(&db).await;
    db.write(|tx| {
        tx.conn().execute("DELETE FROM slack_connections", [])?;
        Ok(())
    })
    .await
    .unwrap();
    perform_import(db.clone(), crypto, id, |_, _, _| async {
        panic!("disconnected fetched");
        #[allow(unreachable_code)]
        Ok(Outcome::Done)
    })
    .await
    .unwrap();
    assert_eq!(
        run(&db, id).await.error.as_deref(),
        Some("Slack connection is missing or disconnected")
    );
    assert_eq!(run(&db, id).await.status, "failed");
}
#[tokio::test]
async fn slack_job_undo_releases_before_continuation() {
    let (db, _, _dir) = setup().await;
    let id = start(&db).await;
    db.write(move |tx| {
        tx.conn()
            .execute("UPDATE slack_imports SET status='undoing' WHERE id=?", [id])?;
        Ok(())
    })
    .await
    .unwrap();
    perform_undo(db.clone(), id, |_, _| async { Ok(Outcome::Continue) })
        .await
        .unwrap();
    assert!(run(&db, id).await.state.get("step_lease_token").is_none());
    let jobs = db.read(inspect::all).await.unwrap();
    assert_eq!(jobs[1].class, UndoJob::CLASS);
    assert_eq!(jobs[1].queue, "slack_import");
}
#[tokio::test]
async fn slack_job_retry_after_enqueue_failure_rolls_back_heartbeat() {
    let (db, crypto, _dir) = setup().await;
    let id = start(&db).await;
    db.write(move |tx| {
        tx.conn().execute("UPDATE slack_imports SET heartbeat_at='2020-01-01 00:00:00' WHERE id=?",[id])?;
        tx.conn().execute_batch("CREATE TRIGGER reject_slack_continuation BEFORE INSERT ON background_jobs BEGIN SELECT RAISE(ABORT,'fixture queue failure'); END;")?;
        Ok(())
    }).await.unwrap();
    // Claim/acquire legitimately refresh heartbeat. The rate-limit rescue must roll back its
    // own heartbeat write together with the failed delayed enqueue.
    let executing = db.clone();
    let result = perform_import(db.clone(), crypto, id, move |_, _, _| async move {
        executing
            .write(move |tx| {
                tx.conn().execute(
                    "UPDATE slack_imports SET heartbeat_at='2020-01-01 00:00:00' WHERE id=?",
                    [id],
                )?;
                Ok(())
            })
            .await?;
        Err(Error::new(ErrorKind::RateLimited, "limited").into())
    })
    .await;
    assert!(result.is_err());
    assert_eq!(
        run(&db, id).await.heartbeat_at.unwrap().to_string(),
        "2020-01-01 00:00:00"
    );
    assert_eq!(db.read(inspect::all).await.unwrap().len(), 1);
}

#[tokio::test]
async fn slack_job_overlapping_executions_have_exactly_one_lease_holder() {
    let (db, crypto, _dir) = setup().await;
    let id = start(&db).await;
    let entered = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let first_db = db.clone();
    let first_crypto = crypto.clone();
    let first_entered = entered.clone();
    let first_release = release.clone();
    let first = tokio::spawn(async move {
        perform_import(first_db, first_crypto, id, move |_, _, _| async move {
            first_entered.notify_one();
            first_release.notified().await;
            Ok(Outcome::Continue)
        })
        .await
    });
    tokio::time::timeout(Duration::from_secs(5), entered.notified())
        .await
        .unwrap();
    perform_import(db.clone(), crypto, id, |_, _, _| async {
        panic!("overlapping execution wrote");
        #[allow(unreachable_code)]
        Ok(Outcome::Done)
    })
    .await
    .unwrap();
    release.notify_one();
    first.await.unwrap().unwrap();
    assert_eq!(db.read(inspect::all).await.unwrap().len(), 2);
    assert!(run(&db, id).await.state.get("step_lease_token").is_none());
}

#[tokio::test]
async fn slack_registered_serial_worker_imports_undoes_and_registers_30_second_sweep() {
    let directory = tempfile::tempdir().unwrap();
    let config = crate::config::Config::from_lookup(|name| match name {
        "SECRET_KEY_BASE" => Some("a".repeat(128)),
        "DISABLE_SSL" => Some("1".into()),
        "RAILS_ENV" => Some("test".into()),
        "CAMPFIRE_STORAGE_PATH" => Some(directory.path().to_string_lossy().into_owned()),
        _ => None,
    })
    .unwrap();
    let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
        "2026-03-02T16:00:00Z".parse().unwrap(),
    ));
    let (_server, network) =
        super::super::client::tests::fake(super::super::store::tests::routes(false)).await;
    let booted = crate::server::boot_with_services(
        config,
        clock,
        network,
        crate::jobs::periodic::Intervals {
            periodic: None,
            huddle: None,
        },
    )
    .await
    .unwrap();
    let crypto = booted.app.ar_encryption.clone();
    let id = booted
        .app
        .db
        .write(move |tx| {
            let user = campfire_db::User::create(
                tx,
                campfire_db::NewUser {
                    name: "Run owner".into(),
                    ..Default::default()
                },
            )?;
            let workspace = SlackWorkspace::create(
                tx,
                &crypto,
                "fixture-client",
                "fixture-secret",
                Some(user.id),
            )?;
            let connection = SlackConnection::create(
                tx,
                &crypto,
                NewConnection {
                    workspace_id: workspace.id,
                    user_id: user.id,
                    slack_user_id: "UADMIN",
                    access_token: Some("fixture-user-token"),
                    scopes: None,
                },
            )?;
            Ok(SlackImport::create(
                tx,
                NewImport {
                    workspace_id: workspace.id,
                    connection_id: Some(connection.id),
                    user_id: user.id,
                    kind: Kind::Workspace,
                    mode: Mode::Import,
                    options: json!({}),
                },
            )?
            .id)
        })
        .await
        .unwrap();
    let db = booted.app.db.clone();
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            let row = run(&db, id).await;
            if row.status == "completed" {
                break;
            }
            assert!(row.error.is_none(), "{:?}", row.error);
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("registered import worker timed out");
    assert!(db.write(move |tx| SlackImport::undo(tx, id)).await.unwrap());
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            if run(&db, id).await.status == "undone" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("registered undo worker timed out");
    db.read(|c| {
        assert_eq!(campfire_db::Message::count(c)?, 0);
        assert_eq!(campfire_db::Room::all(c)?.len(), 0);
        Ok(())
    })
    .await
    .unwrap();
    let periodic = crate::jobs::periodic::periodic(crate::jobs::periodic::PeriodicIntervals {
        reminders: Duration::from_secs(30),
        retention: Duration::from_secs(86400),
    });
    let tasks: Vec<_> = periodic
        .tasks()
        .filter(|t| t.name() == "slack imports")
        .collect();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].interval(), Duration::from_secs(30));
    booted.jobs.shutdown(Duration::from_secs(1)).await;
}
