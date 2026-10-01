use super::*;
use crate::integrations::test_support::TestDb;
use campfire_db::{BasicRichText, Env, Event, EventSink, TestClock, Tx};
use campfire_jobs::{JobQueue, QueueConfig, Registry, RunnerConfig};
use serde_json::{Value, json};
use std::sync::Arc;

struct QueueSink(JobQueue);
impl EventSink for QueueSink {
    fn emit(&self, _event: Event) {}
    fn persist(&self, tx: &Tx<'_>, event: &Event) -> campfire_db::Result<()> {
        if let Event::Job(request) = event {
            self.0.enqueue(tx, request)?;
        }
        Ok(())
    }
}
async fn database() -> TestDb {
    let clock = Arc::new(TestClock::frozen_at(Timestamp::from_jiff(
        "2026-01-01T12:00:00Z".parse().unwrap(),
    )));
    let queue = JobQueue::new(
        &Registry::<()>::new(),
        &RunnerConfig::new(vec![QueueConfig::new("default", 1)]),
    )
    .unwrap();
    let env = Env {
        clock,
        sink: Arc::new(QueueSink(queue)),
        rich_text: Arc::new(BasicRichText),
        bcrypt_cost: 4,
        ..Env::default()
    };
    let directory =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.scratch/ws15g");
    tokio::task::spawn_blocking(move || TestDb::with_env(env, &directory))
        .await
        .unwrap()
}
async fn seed(db: &Database, integration: Integration, age: i64, webhook: bool) {
    db.write(move |tx| {
        let now = tx.now();
        tx.conn().execute("INSERT INTO users (id, name, role, status, bot_token_digest, created_at, updated_at) VALUES (810, 'Machine', 2, 0, ?, ?, ?)", rusqlite::params![campfire_db::user::digest_bot_token("fixture-machine-token"), now, now])?;
        tx.conn().execute("INSERT INTO users (id, name, email_address, role, status, password_digest, created_at, updated_at) SELECT 811, 'Oracle', 'oracle@example.test', 1, 0, password_digest, ?, ? FROM users WHERE id = ?", rusqlite::params![now, now, TestDb::id("david")])?;
        tx.conn().execute("INSERT INTO agents (id, user_id, owner_id, created_at, updated_at) VALUES (812, 810, 811, ?, ?)", rusqlite::params![now, now])?;
        let action = if integration.event_type == GITHUB.event_type { "github.comment" } else { "fizzy.comment" };
        tx.conn().execute("INSERT INTO agent_approvals (id, action, agent_id, summary, status, expires_at, decided_by_id, created_at, updated_at) VALUES (813, ?, 812, 'Approved fixture', 'approved', ?, 811, ?, ?)", rusqlite::params![action, now.since(jiff::SignedDuration::from_hours(24)), now, now])?;
        tx.conn().execute("INSERT INTO agent_events (id, agent_id, agent_approval_id, actor_id, event_type, outcome, webhook_attempts, created_at, metadata) VALUES (814, 812, 813, 811, ?, 'delivered', 2, ?, ?)", rusqlite::params![integration.event_type, now.ago(jiff::SignedDuration::from_secs(age)), json!({"approval_id": 813,"action":action,"status":"running","extra":"preserved"}).to_string()])?;
        if webhook { tx.conn().execute("INSERT INTO webhooks (user_id, url, created_at, updated_at) VALUES (810, '', ?, ?)", rusqlite::params![now, now])?; }
        Ok(())
    }).await.unwrap();
}
async fn snapshot(db: &Database) -> Value {
    db.read(|conn| {
        let row = conn.query_row("SELECT metadata, detail, webhook_status, webhook_next_attempt_at, webhook_attempts, outcome, created_at FROM agent_events WHERE id = 814", [], |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?, r.get::<_, String>(2)?, r.get::<_, Option<Timestamp>>(3)?, r.get::<_, i64>(4)?, r.get::<_, String>(5)?, r.get::<_, Timestamp>(6)?)))?;
        let audits = conn.query_row("SELECT COUNT(*) FROM audit_logs", [], |r| r.get::<_, i64>(0))?;
        let jobs = conn.query_row("SELECT COUNT(*) FROM background_jobs", [], |r| r.get::<_, i64>(0))?;
        Ok(json!({"metadata":serde_json::from_str::<Value>(&row.0).unwrap(),"detail":row.1,"webhook_status":row.2,"webhook_next_attempt_at":row.3.map(|t|t.to_db()),"webhook_attempts":row.4,"outcome":row.5,"created_at":row.6.to_db(),"audits":audits,"jobs":jobs}))
    }).await.unwrap()
}

#[tokio::test]
async fn github_claim_sweep_fails_only_overdue_running_claims_and_preserves_metadata() {
    for age in [899, 900, 901] {
        let fixture = database().await;
        seed(&fixture.db, GITHUB, age, false).await;
        let before = snapshot(&fixture.db).await;
        let recovered = recover_stuck_claims(&fixture.db, GITHUB, fixture.db.env().now()).await;
        if age <= 900 {
            assert_eq!(recovered, 0);
            assert_eq!(snapshot(&fixture.db).await, before);
        } else {
            assert_eq!(recovered, 1);
            let after = snapshot(&fixture.db).await;
            assert_eq!(after["metadata"]["status"], "failed");
            assert_eq!(after["metadata"]["message"], GITHUB.timeout_message);
            assert_eq!(after["detail"], GITHUB.timeout_message);
            assert_eq!(after["metadata"]["extra"], "preserved");
            assert_eq!(after["outcome"], before["outcome"]);
            assert_eq!(after["created_at"], before["created_at"]);
            assert_eq!(after["webhook_status"], "none");
            assert_eq!(after["audits"], 1);
            assert_eq!(after["jobs"], 0);
        }
    }
}

#[tokio::test]
async fn github_claim_sweep_enqueues_even_a_blank_url_webhook_once_with_current_attempt() {
    let fixture = database().await;
    seed(&fixture.db, GITHUB, 960, true).await;
    assert_eq!(
        recover_stuck_claims(&fixture.db, GITHUB, fixture.db.env().now()).await,
        1
    );
    let first = snapshot(&fixture.db).await;
    assert_eq!(first["webhook_status"], "pending");
    assert_eq!(first["webhook_next_attempt_at"], "2026-01-01 12:00:00");
    assert_eq!(first["webhook_attempts"], 2);
    assert_eq!(first["jobs"], 1);
    fixture
        .db
        .read(|conn| {
            let (class, arguments): (String, String) = conn.query_row(
                "SELECT job_class, arguments FROM background_jobs",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            assert_eq!(class, "Agent::EventWebhookJob");
            assert_eq!(
                serde_json::from_str::<Value>(&arguments).unwrap(),
                json!({"event_id":814,"attempt":2})
            );
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        recover_stuck_claims(&fixture.db, GITHUB, fixture.db.env().now()).await,
        0
    );
    assert_eq!(snapshot(&fixture.db).await, first);
}

#[tokio::test]
async fn github_claim_sweep_and_late_finish_keep_the_first_outcome_and_enqueue_once() {
    for sweep_first in [true, false] {
        let fixture = database().await;
        seed(&fixture.db, GITHUB, 960, true).await;
        if sweep_first {
            assert_eq!(
                recover_stuck_claims(&fixture.db, GITHUB, fixture.db.env().now()).await,
                1
            );
        }
        let wrote = fixture.db.write(move |tx| rewrite_running(tx, 814, json!({"approval_id":813,"action":"github.comment","status":"completed","url":"https://github.com/rails/rails/pull/12#issuecomment-9"}), None)).await.unwrap();
        assert_eq!(wrote, !sweep_first);
        if !sweep_first {
            assert_eq!(
                recover_stuck_claims(&fixture.db, GITHUB, fixture.db.env().now()).await,
                0
            );
        }
        let after = snapshot(&fixture.db).await;
        assert_eq!(
            after["metadata"]["status"],
            if sweep_first { "failed" } else { "completed" }
        );
        assert_eq!(after["jobs"], 1);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn github_claim_concurrent_finishes_have_one_winner() {
    let fixture = database().await;
    seed(&fixture.db, GITHUB, 960, true).await;
    let outcomes = futures_util::future::join_all((0..24).map(|i| {
        fixture.db.write(move |tx| {
            rewrite_running(tx, 814, json!({"status":"completed","winner":i}), None)
        })
    }))
    .await;
    assert_eq!(
        outcomes
            .iter()
            .filter(|result| result.as_ref().is_ok_and(|won| *won))
            .count(),
        1
    );
    assert_eq!(snapshot(&fixture.db).await["jobs"], 1);
}

#[tokio::test]
async fn github_claim_helper_also_recovers_fizzy_without_touching_github() {
    let fixture = database().await;
    seed(&fixture.db, FIZZY, 960, false).await;
    assert_eq!(
        recover_stuck_claims(&fixture.db, GITHUB, fixture.db.env().now()).await,
        0
    );
    assert_eq!(
        recover_stuck_claims(&fixture.db, FIZZY, fixture.db.env().now()).await,
        1
    );
    let after = snapshot(&fixture.db).await;
    assert_eq!(after["metadata"]["message"], FIZZY.timeout_message);
    fixture
        .db
        .read(|conn| {
            assert_eq!(
                conn.query_row("SELECT action FROM audit_logs", [], |r| r
                    .get::<_, String>(0))?,
                FIZZY.audit_action
            );
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn github_claim_audit_failure_does_not_skip_webhook_and_queue_failure_rolls_back() {
    for reject_jobs in [false, true] {
        let fixture = database().await;
        seed(&fixture.db, GITHUB, 960, true).await;
        fixture.db.write(move |tx| {
            tx.conn().execute_batch(if reject_jobs { "CREATE TRIGGER reject_job BEFORE INSERT ON background_jobs BEGIN SELECT RAISE(ABORT, 'fixture queue down'); END" } else { "CREATE TRIGGER reject_audit BEFORE INSERT ON audit_logs BEGIN SELECT RAISE(ABORT, 'fixture audit down'); END" })?;
            Ok(())
        }).await.unwrap();
        let count = recover_stuck_claims(&fixture.db, GITHUB, fixture.db.env().now()).await;
        let after = snapshot(&fixture.db).await;
        if reject_jobs {
            assert_eq!(count, 0);
            assert_eq!(after["metadata"]["status"], "running");
            assert_eq!(after["audits"], 0);
            assert_eq!(after["jobs"], 0);
        } else {
            assert_eq!(count, 1);
            assert_eq!(after["audits"], 0);
            assert_eq!(after["jobs"], 1);
        }
    }
}

#[test]
fn github_claim_periodic_task_is_registered_every_thirty_seconds() {
    let tasks = crate::jobs::periodic::periodic(crate::jobs::periodic::PeriodicIntervals {
        reminders: std::time::Duration::from_secs(30),
        retention: std::time::Duration::from_secs(86400),
    });
    let task = tasks
        .tasks()
        .find(|task| task.name() == "stuck GitHub claims")
        .expect("GitHub recovery task must be active");
    assert_eq!(task.interval(), SWEEP_INTERVAL);
}

#[tokio::test]
async fn github_claim_registered_periodic_task_executes_and_obeys_its_interval() {
    for integration in [GITHUB, FIZZY] {
        let scratch = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.scratch/ws15g");
        std::fs::create_dir_all(&scratch).unwrap();
        let directory = tempfile::tempdir_in(scratch).unwrap();
        let config = crate::config::Config::from_lookup(|name| match name {
            "SECRET_KEY_BASE" => Some("a".repeat(128)),
            "DISABLE_SSL" => Some("1".into()),
            "CAMPFIRE_STORAGE_PATH" => Some(directory.path().to_string_lossy().into_owned()),
            _ => None,
        })
        .unwrap();
        let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
            "2026-01-01T12:00:00Z".parse().unwrap(),
        ));
        let booted = crate::app::boot_with_clock(config, clock.clone())
            .await
            .unwrap();
        booted
            .jobs
            .shutdown(std::time::Duration::from_secs(1))
            .await;
        let db = &booted.app.db;
        db.write(|tx| {
            campfire_db::fixtures::load(
                tx.conn(),
                &campfire_db::fixtures::reference_dir(),
                &campfire_db::fixtures::Options {
                    now: tx.now(),
                    bcrypt_cost: 4,
                },
            )
        })
        .await
        .unwrap();
        seed(db, integration, 960, false).await;
        let mut periodic = crate::jobs::periodic::periodic(crate::jobs::periodic::PeriodicIntervals {
            reminders: SWEEP_INTERVAL,
            retention: std::time::Duration::from_secs(86400),
        });
        assert_eq!(
            periodic.tick(booted.app.clone(), db.env().now()).await,
            ["clear plaintext bot tokens", "stranded agent webhooks", "event reminders", "saved item reminders", "scheduled messages", "poll closing", "stuck rooms", "stuck GitHub claims", "stuck Fizzy claims", "retention prune", "presence leases", "meeting status", "out of office"]
        );
        assert_eq!(snapshot(db).await["metadata"]["status"], "failed");
        clock.advance(jiff::SignedDuration::from_secs(29));
        assert!(
            periodic
                .tick(booted.app.clone(), db.env().now())
                .await
                .is_empty()
        );
        clock.advance(jiff::SignedDuration::from_secs(1));
        assert_eq!(
            periodic.tick(booted.app.clone(), db.env().now()).await,
            ["stranded agent webhooks", "event reminders", "saved item reminders", "scheduled messages", "poll closing", "stuck GitHub claims", "stuck Fizzy claims"]
        );
        assert_eq!(snapshot(db).await["audits"], 1);
    }
}

#[tokio::test]
async fn github_claim_persisted_outcomes_and_audits_match_pinned_rails() {
    let vectors: Value =
        serde_json::from_str(include_str!("../../../../../vectors/github_claims.json")).unwrap();
    for case in vectors["cases"].as_array().unwrap() {
        let fixture = database().await;
        let integration = if case["fizzy"] == true { FIZZY } else { GITHUB };
        seed(
            &fixture.db,
            integration,
            case["age"].as_i64().unwrap(),
            case["webhook"] == true,
        )
        .await;
        let historical = case["historical"] == true;
        let missing = case["missing"] == true;
        let audit_failure = case["audit_failure"] == true;
        let preserved_url = case["url"].as_str().map(str::to_owned);
        fixture.db.write(move |tx| {
            if let Some(url) = preserved_url { tx.conn().execute("UPDATE agent_events SET metadata = json_set(metadata, '$.url', ?) WHERE id = 814", [url])?; }
            if historical { tx.conn().execute("UPDATE agent_events SET agent_approval_id = NULL WHERE id = 814", [])?; }
            if missing { tx.conn().execute("UPDATE agent_events SET agent_approval_id = 999, metadata = json_set(metadata, '$.approval_id', 999) WHERE id = 814", [])?; }
            if audit_failure { tx.conn().execute_batch("CREATE TRIGGER reject_audit BEFORE INSERT ON audit_logs BEGIN SELECT RAISE(ABORT, 'fixture audit down'); END")?; }
            Ok(())
        }).await.unwrap();
        let finish = || {
            fixture.db.write(move |tx| {
            let action = if integration.event_type == GITHUB.event_type { "github.comment" } else { "fizzy.comment" };
            if rewrite_running(tx, 814, json!({"approval_id":813,"action":action,"status":"completed","url":"https://github.com/rails/rails/pull/12#issuecomment-9"}), None)? {
                record_execution_audit(tx, 814, integration)?;
            }
            Ok(())
        })
        };
        if case["finish_first"] == true {
            finish().await.unwrap();
        }
        recover_stuck_claims(&fixture.db, integration, fixture.db.env().now()).await;
        if case["late_finish"] == true {
            finish().await.unwrap();
        }
        assert_eq!(
            snapshot(&fixture.db).await,
            case["expected"],
            "{}",
            case["name"]
        );
        let (audits, jobs) = fixture.db.read(|conn| {
            let audits = conn.prepare_cached("SELECT action, actor_id, actor_label, target_type, target_id, target_label, details FROM audit_logs ORDER BY id")?.query_map([], |r| Ok(json!({"action":r.get::<_, String>(0)?,"actor_id":r.get::<_, Option<i64>>(1)?,"actor_label":r.get::<_, Option<String>>(2)?,"target_type":r.get::<_, String>(3)?,"target_id":r.get::<_,i64>(4)?,"target_label":r.get::<_,String>(5)?,"details":serde_json::from_str::<Value>(&r.get::<_,String>(6)?).unwrap()})))?.collect::<rusqlite::Result<Vec<_>>>()?;
            let jobs = conn.prepare_cached("SELECT job_class, arguments FROM background_jobs ORDER BY id")?.query_map([], |r| { let args: Value = serde_json::from_str(&r.get::<_,String>(1)?).unwrap(); Ok(json!({"class":r.get::<_,String>(0)?,"args":[args["event_id"],args["attempt"]]})) })?.collect::<rusqlite::Result<Vec<_>>>()?;
            Ok((audits,jobs))
        }).await.unwrap();
        assert_eq!(json!(audits), case["audits"], "{} audits", case["name"]);
        assert_eq!(json!(jobs), case["jobs"], "{} jobs", case["name"]);
    }
}
