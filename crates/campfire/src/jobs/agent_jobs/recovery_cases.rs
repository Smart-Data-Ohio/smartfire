//! DeliveryRecoveryTest through the production periodic sweep and durable queue.
use crate::integrations::test_support::{FakeResolver, FakeServer, MappingDialer, Route, network};
use super::*;
use campfire_db::Message;
use campfire_db::models::agent_delivery as domain;
use campfire_db::models::agent_delivery::AgentEvent;
use crate::app::App;
use crate::net::Network;
use crate::controllers::presenters::test_support::{ALL_TALK, BENDER, DAVID, TestApp};
use rusqlite::params;
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};
async fn setup() -> (App, tempfile::TempDir) {
    let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
        "2026-03-02T16:00:00Z".parse().unwrap(),
    ));
    let (app, dir) = TestApp::boot_with_clock(clock)
        .await
        .expect("default seed")
        .stop_jobs()
        .await;
    app.db
        .write(|tx| {
            tx.conn().execute("DELETE FROM agent_events", [])?;
            tx.conn().execute("DELETE FROM background_jobs", [])?;
            tx.conn().execute(
                "UPDATE webhooks SET url='http://bots.example:8080/hook' WHERE user_id=?",
                [BENDER],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    (app, dir)
}
async fn event(app: &App, age: i64, attempts: i64, status: &str, outcome: &str) -> AgentEvent {
    let status = status.to_owned();
    let outcome = outcome.to_owned();
    app.db.write(move|tx|{
        let message=Message::create(tx,campfire_db::NewMessage{room_id:ALL_TALK,creator_id:DAVID,markdown_source:Some("Hey @[Bender Bot]".into()),..Default::default()})?;
        let id=tx.conn().query_row("SELECT id FROM agent_events WHERE message_id=? AND event_type='mention' ORDER BY id DESC LIMIT 1",[message.id],|r|r.get(0))?;
        tx.conn().execute("UPDATE agent_events SET outcome=?,webhook_status=?,webhook_attempts=?,created_at=? WHERE id=?",params![outcome,status,attempts,tx.now().ago(jiff::SignedDuration::from_secs(age)),id])?;
        tx.conn().execute("DELETE FROM background_jobs",[])?;
        Ok(AgentEvent::find(tx.conn(),id)?.unwrap())
    }).await.unwrap()
}
async fn rows(app: &App, class: &'static str) -> Vec<serde_json::Value> {
    app.db
        .read(move |c| {
            let mut q =
                c.prepare("SELECT arguments FROM background_jobs WHERE job_class=? ORDER BY id")?;
            Ok(q.query_map([class], |r| r.get(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?)
        })
        .await
        .unwrap()
}
async fn read(app: &App, id: i64) -> AgentEvent {
    app.db
        .read(move |c| AgentEvent::find(c, id))
        .await
        .unwrap()
        .unwrap()
}
async fn sweep(app: &App) {
    crate::jobs::periodic::stranded_agent_webhooks(&app.db)
        .await
        .unwrap();
}
async fn simple(age: i64, attempts: i64, status: &str, expected: i64) {
    let (app, _dir) = setup().await;
    let e = event(&app, age, attempts, status, "delivered").await;
    sweep(&app).await;
    let q = rows(&app, "Agent::EventWebhookJob").await;
    assert_eq!(q.len() as i64, expected);
    if expected > 0 {
        assert_eq!(q[0]["event_id"], e.id);
        assert_eq!(q[0]["attempt"], attempts);
    }
}
#[tokio::test]
async fn ws11_recovery_case_old_pending_recovered() {
    simple(180, 0, "pending", 1).await;
}
#[tokio::test]
async fn ws11_recovery_case_spent_attempts_recovered() {
    simple(600, 3, "pending", 1).await;
}
#[tokio::test]
async fn ws11_recovery_case_fresh_pending_left_alone() {
    simple(60, 0, "pending", 0).await;
}
#[tokio::test]
async fn ws11_recovery_case_exhausted_old_null_and_scheduled_fail() {
    let (app, _dir) = setup().await;
    let a = event(&app, 600, 5, "pending", "delivered").await;
    let b = event(&app, 600, 5, "pending", "delivered").await;
    app.db
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE agent_events SET webhook_next_attempt_at=? WHERE id=?",
                params![tx.now().ago(jiff::SignedDuration::from_mins(8)), b.id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    sweep(&app).await;
    assert!(rows(&app, "Agent::EventWebhookJob").await.is_empty());
    for id in [a.id, b.id] {
        let e = read(&app, id).await;
        assert_eq!(e.webhook_status, "failed");
        assert_eq!(
            e.webhook_last_error.as_deref(),
            Some("Delivery attempts exhausted without a recorded outcome")
        );
    }
}
#[tokio::test]
async fn ws11_recovery_case_exhausted_recent_left_pending() {
    let (app, _dir) = setup().await;
    let e = event(&app, 180, 5, "pending", "delivered").await;
    sweep(&app).await;
    assert!(rows(&app, "Agent::EventWebhookJob").await.is_empty());
    assert_eq!(read(&app, e.id).await.webhook_status, "pending");
}
#[tokio::test]
async fn ws11_recovery_case_settled_rows_left_alone() {
    let (app, _dir) = setup().await;
    let a = event(&app, 600, 1, "delivered", "delivered").await;
    let b = event(&app, 600, 5, "failed", "delivered").await;
    sweep(&app).await;
    assert!(rows(&app, "Agent::EventWebhookJob").await.is_empty());
    assert_eq!(read(&app, a.id).await.webhook_status, "delivered");
    assert_eq!(read(&app, b.id).await.webhook_status, "failed");
}
async fn responder(status: u16, hint: Option<&str>) -> (FakeServer, Network) {
    let mut route = Route::new("POST", "*", "/hook", status);
    if let Some(hint) = hint {
        route = route.header("Retry-After", hint);
    }
    let s = FakeServer::start(vec![route]).await;
    let n = network(
        Arc::new(FakeResolver::new([("bots.example", vec!["93.184.216.34"])])),
        Arc::new(MappingDialer {
            public: HashSet::from(["93.184.216.34".parse().unwrap()]),
            to: s.addr,
            dialed: Mutex::new(vec![]),
        }),
    );
    (s, n)
}
async fn post(app: &App, id: i64, attempt: i64, n: &Network) {
    post_with_network(
        app,
        domain::EventWebhookJob {
            event_id: id,
            attempt: Some(attempt),
        },
        n,
    )
    .await
    .unwrap();
}
#[tokio::test]
async fn ws11_recovery_case_retry_after_never_reposted_early() {
    let (app, _dir) = setup().await;
    let e = event(&app, 0, 0, "pending", "delivered").await;
    let (s, n) = responder(429, Some("600")).await;
    post(&app, e.id, 0, &n).await;
    app.db
        .write(|tx| {
            tx.conn().execute("DELETE FROM background_jobs", [])?;
            Ok(())
        })
        .await
        .unwrap();
    for seconds in [180, 599] {
        let now = app
            .db
            .env()
            .now()
            .since(jiff::SignedDuration::from_secs(seconds));
        app.db
            .write(move |tx| {
                domain::fail_exhausted(tx, now)?;
                for c in domain::recovery_candidates(tx.conn(), now)? {
                    domain::recover_one(tx, c)?;
                }
                Ok(())
            })
            .await
            .unwrap();
        assert!(rows(&app, "Agent::EventWebhookJob").await.is_empty());
    }
    assert_eq!(s.received().len(), 1);
    let after = read(&app, e.id).await;
    assert_eq!(after.webhook_attempts, 1);
    assert_eq!(after.webhook_status, "pending");
}
#[tokio::test]
async fn ws11_recovery_case_snapshot_cannot_overwrite_new_retry_after() {
    let (app, _dir) = setup().await;
    let e = event(&app, 180, 0, "pending", "delivered").await;
    let now = app.db.env().now();
    let candidate = app
        .db
        .read(move |c| {
            Ok(domain::recovery_candidates(c, now)?
                .into_iter()
                .next()
                .unwrap())
        })
        .await
        .unwrap();
    let (s, n) = responder(429, Some("600")).await;
    post(&app, e.id, 0, &n).await;
    let before = read(&app, e.id).await;
    app.db
        .write(move |tx| domain::recover_one(tx, candidate))
        .await
        .unwrap();
    let after = read(&app, e.id).await;
    assert_eq!(after.webhook_attempts, 1);
    assert_eq!(
        after.webhook_next_attempt_at,
        before.webhook_next_attempt_at
    );
    assert_eq!(s.received().len(), 1);
}
#[tokio::test]
async fn ws11_recovery_case_enqueue_failure_keeps_recovering() {
    let (app, _dir) = setup().await;
    let a = event(&app, 300, 0, "pending", "delivered").await;
    let b = event(&app, 360, 0, "pending", "delivered").await;
    app.db.write(move|tx|{tx.conn().execute_batch(&format!("CREATE TRIGGER ws11_reject_one_recovery BEFORE INSERT ON background_jobs WHEN NEW.job_class='Agent::EventWebhookJob' AND json_extract(NEW.arguments,'$.event_id')={} BEGIN SELECT RAISE(ABORT,'rejected one recovery'); END",a.id))?;Ok(())}).await.unwrap();
    sweep(&app).await;
    let q = rows(&app, "Agent::EventWebhookJob").await;
    assert_eq!(q.len(), 1);
    assert_eq!(q[0]["event_id"], b.id);
    assert!(read(&app, a.id).await.webhook_next_attempt_at.is_none());
    assert!(read(&app, b.id).await.webhook_next_attempt_at.is_some());
}
#[tokio::test]
async fn ws11_recovery_case_stale_attempt_exits_without_post() {
    let (app, _dir) = setup().await;
    let e = event(&app, 0, 1, "pending", "delivered").await;
    let (s, n) = responder(200, None).await;
    post(&app, e.id, 0, &n).await;
    assert!(s.received().is_empty());
    let after = read(&app, e.id).await;
    assert_eq!(after.webhook_attempts, 1);
    assert_eq!(after.webhook_status, "pending");
}
#[tokio::test]
async fn ws11_recovery_case_scheduled_retry_records_future() {
    let (app, _dir) = setup().await;
    let e = event(&app, 0, 0, "pending", "delivered").await;
    let (s, n) = responder(429, Some("600")).await;
    post(&app, e.id, 0, &n).await;
    let after = read(&app, e.id).await;
    assert_eq!(after.webhook_status, "pending");
    assert_eq!(
        after.webhook_next_attempt_at,
        Some(
            app.db
                .env()
                .now()
                .since(jiff::SignedDuration::from_secs(600))
        )
    );
    let q = rows(&app, "Agent::EventWebhookJob").await;
    assert_eq!(q.len(), 1);
    assert_eq!(q[0]["attempt"], 1);
    assert_eq!(s.received().len(), 1);
}
#[tokio::test]
async fn ws11_recovery_case_stranded_pending_delivery_recovered() {
    let (app, _dir) = setup().await;
    let e = event(&app, 180, 0, "none", "pending").await;
    assert_eq!(e.webhook_status, "none");
    sweep(&app).await;
    let q = rows(&app, "Agent::DeliveryJob").await;
    assert_eq!(q.len(), 1);
    assert_eq!(q[0]["event_id"], e.id);
}
#[tokio::test]
async fn ws11_recovery_case_fresh_pending_delivery_left_alone() {
    let (app, _dir) = setup().await;
    event(&app, 60, 0, "none", "pending").await;
    sweep(&app).await;
    assert!(rows(&app, "Agent::DeliveryJob").await.is_empty());
}

#[tokio::test]
async fn ws11_recovery_continues_after_one_durable_enqueue_failure() {
    let test = TestApp::boot()
        .await
        .expect("default seed")
        .without_job_runner()
        .await;
    let db = test.db();
    let (first,second)=db.write(|tx| {
        let agent_id=tx.conn().query_row("SELECT id FROM agents WHERE user_id=?",[BENDER],|r|r.get(0))?;
        let new=||domain::NewEvent {agent_id,event_type:"github_action_completed".into(),..Default::default()};
        let a=AgentEvent::create(tx,new())?;let b=AgentEvent::create(tx,new())?;
        tx.conn().execute("UPDATE agent_events SET webhook_status='pending',webhook_next_attempt_at=? WHERE id IN (?,?)",rusqlite::params![tx.now().ago(jiff::SignedDuration::from_mins(3)),a.id,b.id])?;
        tx.conn().execute_batch(&format!("CREATE TRIGGER reject_recovery BEFORE INSERT ON background_jobs WHEN NEW.job_class='Agent::EventWebhookJob' AND json_extract(NEW.arguments,'$.event_id')={} BEGIN SELECT RAISE(ABORT,'WS11 one rejected candidate'); END;",a.id))?;
        Ok((a.id,b.id))
    }).await.unwrap();
    crate::jobs::periodic::stranded_agent_webhooks(db)
        .await
        .unwrap();
    db.read(move |c| {
        let a=AgentEvent::find(c,first)?.unwrap();let b=AgentEvent::find(c,second)?.unwrap();
        assert!(a.webhook_next_attempt_at<b.webhook_next_attempt_at);
        let jobs:i64=c.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Agent::EventWebhookJob' AND json_extract(arguments,'$.event_id')=?",[second],|r|r.get(0))?;
        assert_eq!(jobs,1);
        Ok(())
    }).await.unwrap();
}
