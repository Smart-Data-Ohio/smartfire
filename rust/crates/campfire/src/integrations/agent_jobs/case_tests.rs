//! Named EventWebhookJobTest comparisons. Only DNS/TCP routing are substituted;
//! real HTTP parsing, ledger claims, retries and durable queue writes run unchanged.
use super::super::test_support::{FakeResolver, FakeServer, MappingDialer, Route, network};
use super::*;
use crate::controllers::presenters::test_support::{ALL_TALK, BENDER, DAVID, TestApp};
use rusqlite::params;
use std::{
    collections::HashSet,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
async fn setup() -> (TestApp, AgentEvent) {
    let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
        "2026-03-02T16:00:00Z".parse().unwrap(),
    ));
    let t = TestApp::boot_with_clock(clock).await.expect("default seed");
    let e = t
        .db()
        .write(|tx| {
            let m = Message::create(
                tx,
                campfire_db::NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    body: Some("Trigger".into()),
                    ..Default::default()
                },
            )?;
            let aid =
                tx.conn()
                    .query_row("SELECT id FROM agents WHERE user_id=?", [BENDER], |r| {
                        r.get(0)
                    })?;
            let e = AgentEvent::create(
                tx,
                domain::NewEvent {
                    agent_id: aid,
                    message_id: Some(m.id),
                    room_id: Some(ALL_TALK),
                    event_type: "mention".into(),
                    outcome: Some("delivered".into()),
                    ..Default::default()
                },
            )?;
            tx.conn().execute(
                "UPDATE agent_events SET webhook_status='pending' WHERE id=?",
                [e.id],
            )?;
            tx.conn().execute(
                "UPDATE webhooks SET url='http://bots.example:8080/hook' WHERE user_id=?",
                [BENDER],
            )?;
            Ok(AgentEvent::find(tx.conn(), e.id)?.unwrap())
        })
        .await
        .unwrap();
    (t, e)
}
async fn read(t: &TestApp, id: i64) -> AgentEvent {
    t.db()
        .read(move |c| AgentEvent::find(c, id))
        .await
        .unwrap()
        .unwrap()
}
async fn queued(t: &TestApp, id: i64) -> i64 {
    t.db().read(move|c|Ok(c.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Agent::EventWebhookJob' AND json_extract(arguments,'$.event_id')=?",[id],|r|r.get(0))?)).await.unwrap()
}
async fn respond(route: Route) -> (FakeServer, Network) {
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
async fn post(t: &TestApp, id: i64, attempt: i64, n: &Network) {
    post_with_network(
        &t.booted.app,
        domain::EventWebhookJob {
            event_id: id,
            attempt: Some(attempt),
        },
        n,
    )
    .await
    .unwrap();
}
fn success(e: &AgentEvent, attempt: i64) {
    assert_eq!(e.webhook_status, "delivered");
    assert_eq!(e.webhook_attempts, attempt);
    assert!(e.webhook_last_error.is_none());
}
fn failed(e: &AgentEvent, attempt: i64, error: &str) {
    assert_eq!(e.webhook_status, "failed");
    assert_eq!(e.webhook_attempts, attempt);
    assert!(
        e.webhook_last_error.as_deref().unwrap().contains(error),
        "{:?}",
        e.webhook_last_error
    );
}
fn retry(e: &AgentEvent, error: &str, seconds: i64) {
    assert_eq!(e.webhook_status, "pending");
    assert_eq!(e.webhook_attempts, 1);
    assert!(e.webhook_last_error.as_deref().unwrap().contains(error), "expected {error}, got {:?}", e.webhook_last_error);
    assert_eq!(
        e.webhook_next_attempt_at.unwrap(),
        e.created_at.since(jiff::SignedDuration::from_secs(seconds))
    );
}
struct FailureDialer {
    calls: AtomicUsize,
    hang: bool,
}
impl super::super::net::Dialer for FailureDialer {
    fn connect(
        &self,
        _: std::net::SocketAddr,
    ) -> super::super::net::BoxFuture<'_, std::io::Result<tokio::net::TcpStream>> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Box::pin(async move {
            if self.hang {
                std::future::pending().await
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    "Connection refused",
                ))
            }
        })
    }
}
fn failures(hang: bool) -> (Network, Arc<FailureDialer>) {
    let d = Arc::new(FailureDialer {
        calls: AtomicUsize::new(0),
        hang,
    });
    let n = Network {
        resolver: Arc::new(FakeResolver::new([("bots.example", vec!["93.184.216.34"])])),
        dialer: d.clone(),
        tls: Network::system().tls,
    };
    (n, d)
}
#[tokio::test]
async fn ws11_event_webhook_case_success_marks_delivered() {
    let (t, e) = setup().await;
    let (s, n) = respond(Route::new("POST", "*", "/hook", 200)).await;
    post(&t, e.id, 0, &n).await;
    success(&read(&t, e.id).await, 1);
    assert_eq!(s.received().len(), 1);
}
#[tokio::test]
async fn ws11_event_webhook_case_transport_failure_retries_then_succeeds() {
    let (t, e) = setup().await;
    let (n, d) = failures(false);
    post(&t, e.id, 0, &n).await;
    retry(&read(&t, e.id).await, "Connection refused", 3);
    assert_eq!(queued(&t, e.id).await, 1);
    let (s, n) = respond(Route::new("POST", "*", "/hook", 200)).await;
    post(&t, e.id, 1, &n).await;
    success(&read(&t, e.id).await, 2);
    assert_eq!(d.calls.load(Ordering::Relaxed) + s.received().len(), 2);
}
#[tokio::test]
async fn ws11_event_webhook_case_timeout_records_attempt_without_reply() {
    let (t, e) = setup().await;
    let before = t
        .db()
        .read(|c| Ok(c.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get::<_, i64>(0))?))
        .await
        .unwrap();
    let (n, d) = failures(true);
    post(&t, e.id, 0, &n).await;
    retry(&read(&t, e.id).await, "OpenTimeout", 3);
    assert_eq!(d.calls.load(Ordering::Relaxed), 1);
    assert_eq!(queued(&t, e.id).await, 1);
    t.db()
        .read(move |c| {
            assert_eq!(
                c.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get::<_, i64>(0))?,
                before
            );
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn ws11_event_webhook_case_fifth_failure_exhausts_without_enqueue() {
    let (t, e) = setup().await;
    t.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE agent_events SET webhook_attempts=4 WHERE id=?",
                [e.id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let (n, _) = failures(false);
    post(&t, e.id, 4, &n).await;
    failed(&read(&t, e.id).await, 5, "Connection refused");
    assert_eq!(queued(&t, e.id).await, 0);
}
#[tokio::test]
async fn ws11_event_webhook_case_created_counts_as_delivered() {
    let (t, e) = setup().await;
    let (s, n) = respond(Route::new("POST", "*", "/hook", 201)).await;
    post(&t, e.id, 0, &n).await;
    success(&read(&t, e.id).await, 1);
    assert_eq!(s.received().len(), 1);
}
#[tokio::test]
async fn ws11_event_webhook_case_server_error_retries_then_exhausts() {
    let (t, e) = setup().await;
    let (s, n) = respond(Route::new("POST", "*", "/hook", 500).body("boom")).await;
    post(&t, e.id, 0, &n).await;
    retry(&read(&t, e.id).await, "500", 3);
    assert_eq!(queued(&t, e.id).await, 1);
    t.db().write(move|tx|{tx.conn().execute("DELETE FROM background_jobs WHERE job_class='Agent::EventWebhookJob' AND json_extract(arguments,'$.event_id')=?",[e.id])?;tx.conn().execute("UPDATE agent_events SET webhook_attempts=4 WHERE id=?",[e.id])?;Ok(())}).await.unwrap();
    post(&t, e.id, 4, &n).await;
    failed(&read(&t, e.id).await, 5, "500");
    assert_eq!(queued(&t, e.id).await, 0);
    assert_eq!(s.received().len(), 2);
}
#[tokio::test]
async fn ws11_event_webhook_case_retry_after_forty_five() {
    let (t, e) = setup().await;
    let (_server, n) = respond(Route::new("POST", "*", "/hook", 429).header("Retry-After", "45")).await;
    post(&t, e.id, 0, &n).await;
    retry(&read(&t, e.id).await, "429", 45);
    assert_eq!(queued(&t, e.id).await, 1);
    let id = e.id;
    t.db().read(move|c|{let at:campfire_db::Timestamp=c.query_row("SELECT run_at FROM background_jobs WHERE job_class='Agent::EventWebhookJob' AND json_extract(arguments,'$.event_id')=?",[id],|r|r.get(0))?;assert_eq!(at,campfire_db::Timestamp::parse_db("2026-03-02T16:00:45Z").unwrap());Ok(())}).await.unwrap();
}
#[tokio::test]
async fn ws11_event_webhook_case_retry_after_absent_uses_default() {
    let (t, e) = setup().await;
    let (_server, n) = respond(Route::new("POST", "*", "/hook", 429)).await;
    post(&t, e.id, 0, &n).await;
    retry(&read(&t, e.id).await, "429", 3);
    assert_eq!(queued(&t, e.id).await, 1);
}
#[tokio::test]
async fn ws11_event_webhook_case_request_timeout_retries() {
    let (t, e) = setup().await;
    let (_server, n) = respond(Route::new("POST", "*", "/hook", 408)).await;
    post(&t, e.id, 0, &n).await;
    retry(&read(&t, e.id).await, "408", 3);
    assert_eq!(queued(&t, e.id).await, 1);
}
#[tokio::test]
async fn ws11_event_webhook_case_not_found_fails_fast() {
    let (t, e) = setup().await;
    let (s, n) = respond(Route::new("POST", "*", "/hook", 404).body("gone")).await;
    post(&t, e.id, 0, &n).await;
    failed(&read(&t, e.id).await, 0, "404");
    assert_eq!(queued(&t, e.id).await, 0);
    assert_eq!(s.received().len(), 1);
}
#[tokio::test]
async fn ws11_event_webhook_case_error_attachment_creates_no_reply() {
    let (t, e) = setup().await;
    let before = t
        .db()
        .read(|c| Ok(c.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get::<_, i64>(0))?))
        .await
        .unwrap();
    let (_server, n) = respond(
        Route::new("POST", "*", "/hook", 500)
            .header("Content-Type", "image/jpeg")
            .body(vec![255, 216, 255, 217]),
    )
    .await;
    post(&t, e.id, 0, &n).await;
    assert_eq!(read(&t, e.id).await.webhook_status, "pending");
    t.db()
        .read(move |c| {
            assert_eq!(
                c.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get::<_, i64>(0))?,
                before
            );
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn ws11_event_webhook_case_guard_refuses_without_post() {
    let (t, e) = setup().await;
    t.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE webhooks SET url='http://127.0.0.1:9999/hook' WHERE user_id=?",
                [BENDER],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let (n, d) = failures(false);
    post(&t, e.id, 0, &n).await;
    failed(&read(&t, e.id).await, 0, "Violation");
    assert_eq!(queued(&t, e.id).await, 0);
    assert_eq!(d.calls.load(Ordering::Relaxed), 0);
}
#[tokio::test]
async fn ws11_event_webhook_case_unresolvable_fails_without_post() {
    let (t, e) = setup().await;
    let (mut n, d) = failures(false);
    n.resolver = Arc::new(FakeResolver::new([]));
    post(&t, e.id, 0, &n).await;
    failed(&read(&t, e.id).await, 0, "Unresolvable");
    assert_eq!(queued(&t, e.id).await, 0);
    assert_eq!(d.calls.load(Ordering::Relaxed), 0);
}
#[tokio::test]
async fn ws11_event_webhook_case_deleted_message_fails_without_post() {
    let (t, e) = setup().await;
    let mid = e.message_id.unwrap();
    t.db()
        .write(move |tx| Message::find(tx.conn(), mid)?.destroy(tx))
        .await
        .unwrap();
    let (n, d) = failures(false);
    post(&t, e.id, 0, &n).await;
    failed(&read(&t, e.id).await, 0, "no longer available");
    assert_eq!(queued(&t, e.id).await, 0);
    assert_eq!(d.calls.load(Ordering::Relaxed), 0);
}
#[tokio::test]
async fn ws11_event_webhook_case_removed_configuration_clears_owed_post() {
    let (t, e) = setup().await;
    t.db()
        .write(|tx| {
            tx.conn()
                .execute("DELETE FROM webhooks WHERE user_id=?", [BENDER])?;
            Ok(())
        })
        .await
        .unwrap();
    let (n, d) = failures(false);
    post(&t, e.id, 0, &n).await;
    assert_eq!(read(&t, e.id).await.webhook_status, "none");
    assert_eq!(d.calls.load(Ordering::Relaxed), 0);
}
#[tokio::test]
async fn ws11_event_webhook_case_delivered_row_posts_nothing() {
    let (t, e) = setup().await;
    t.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE agent_events SET webhook_status='delivered',webhook_attempts=1 WHERE id=?",
                [e.id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let (n, d) = failures(false);
    post(&t, e.id, 1, &n).await;
    success(&read(&t, e.id).await, 1);
    assert_eq!(d.calls.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn ws11_event_webhook_case_ack_does_not_cancel_pending_webhook() {
    use campfire_db::models::agent_event_access::{Acknowledgment, acknowledge};
    let (t, seed_event) = setup().await;
    let id=t.db().write(move|tx|{
        tx.conn().execute("DELETE FROM agent_events WHERE id=?",params![seed_event.id])?;
        tx.conn().execute("DELETE FROM agent_grants WHERE agent_id=?",[seed_event.agent_id])?;
        Room::find(tx.conn(),ALL_TALK)?.grant_to(tx,&[BENDER])?;
        // Rails' test queue does not auto-perform this ready job. Keep the real
        // durable row pending so this test can post it with its controlled DNS.
        tx.conn().execute_batch("CREATE TRIGGER ws11_hold_ack_post AFTER INSERT ON background_jobs WHEN NEW.job_class='Agent::EventWebhookJob' BEGIN UPDATE background_jobs SET run_at='2099-01-01 00:00:00' WHERE id=NEW.id; END;")?;
        let m=Message::create(tx,campfire_db::NewMessage{room_id:ALL_TALK,creator_id:DAVID,markdown_source:Some("Hey @[Bender Bot]".into()),..Default::default()})?;
        let id=tx.conn().query_row("SELECT id FROM agent_events WHERE agent_id=? AND message_id=? AND event_type='mention'",params![seed_event.agent_id,m.id],|r|r.get(0))?;
        assert!(matches!(acknowledge(tx,seed_event.agent_id,id)?,Acknowledgment::Acknowledged{..}));
        domain::perform_delivery(tx,id)?;Ok(id)
    }).await.unwrap();
    assert_eq!(queued(&t, id).await, 1);
    let (s, n) = respond(Route::new("POST", "*", "/hook", 200)).await;
    post(&t, id, 0, &n).await;
    let e = read(&t, id).await;
    assert_eq!(e.outcome.as_deref(), Some("acknowledged"));
    assert_eq!(e.webhook_status, "delivered");
    assert_eq!(s.received().len(), 1);
}

#[tokio::test]
async fn ws11_bot_case_auth_keeps_working_after_plaintext_clearing() {
    let (t, _) = setup().await;
    let (id, key) = t
        .db()
        .write(|tx| {
            let b = User::create_bot(tx, "Bender", None)?;
            let key = b.plain_bot_key().unwrap();
            tx.conn().execute(
                "UPDATE users SET bot_token=? WHERE id=?",
                params![b.plain_bot_token, b.id],
            )?;
            Ok((b.id, key))
        })
        .await
        .unwrap();
    crate::jobs::periodic::clear_plaintext_bot_tokens(t.db())
        .await
        .unwrap();
    t.db()
        .read(move |c| {
            assert!(
                c.query_row("SELECT bot_token FROM users WHERE id=?", [id], |r| r
                    .get::<_, Option<String>>(0))?
                    .is_none()
            );
            assert_eq!(User::authenticate_bot(c, &key)?.map(|u| u.id), Some(id));
            assert!(User::authenticate_bot(c, &format!("{id}-WrongToken12"))?.is_none());
            Ok(())
        })
        .await
        .unwrap();
}
