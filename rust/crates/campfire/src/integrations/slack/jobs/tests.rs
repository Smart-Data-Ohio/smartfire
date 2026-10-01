use super::*;
use campfire_db::models::slack::{NewConnection, SlackWorkspace};
use campfire_db::models::slack_import::{Kind, Mode, NewImport};
use campfire_db::{Config, Env, TestClock, Timestamp};
use campfire_jobs::{Execution, QueueConfig, Registry, RunnerConfig, inspect};
use serde_json::json;

pub(crate) async fn setup() -> (Database, Arc<ArEncryption>, tempfile::TempDir) {
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
    struct Sink(campfire_jobs::JobQueue);
    impl campfire_db::EventSink for Sink {
        fn persist(&self, tx: &campfire_db::Tx<'_>, event: &Event) -> campfire_db::Result<()> {
            if let Event::Job(request) = event {
                self.0.enqueue(tx, request)?;
            }
            Ok(())
        }
        fn emit(&self, _: Event) {}
    }
    let clock = TestClock::frozen_at(Timestamp::parse_db("2026-03-02 16:00:00.123456").unwrap());
    let db = Database::open(
        Config::new(dir.path().join("db.sqlite3")),
        Env {
            clock: Arc::new(clock),
            sink: Arc::new(Sink(queue)),
            ..Default::default()
        },
    )
    .unwrap();
    let encryption = Arc::new(ArEncryption::new(&rails_compat::Secrets::new(
        &"ws16-job-key".repeat(32),
    )));
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
