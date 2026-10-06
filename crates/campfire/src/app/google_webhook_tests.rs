//! API webhook over the full router; durable enqueues share the claim/delete transaction.
use crate::controllers::presenters::test_support::{DAVID, Req, TestApp};
use axum::http::{Method, StatusCode};
use campfire_db::models::google_calendar::PushChannel;
async fn app() -> TestApp {
    let a = TestApp::boot().await.expect("pinned default seed required");
    a.db()
        .write(|tx| {
            tx.conn()
                .execute("DELETE FROM calendar_push_channels", [])?;
            tx.conn().execute("DELETE FROM background_jobs", [])?;
            PushChannel::create(
                tx,
                DAVID,
                "fixture-channel",
                &PushChannel::digest("fixture-token"),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    super::google_test_support::observe_jobs(&a).await;
    a
}
fn req(token: &str, state: &str, number: &str) -> Req {
    Req::new(Method::POST, "/google/calendar/notifications")
        .header("x-goog-channel-id", "fixture-channel")
        .header("x-goog-channel-token", token)
        .header("x-goog-resource-state", state)
        .header("x-goog-message-number", number)
}
async fn jobs(a: &TestApp) -> Vec<(String, String)> {
    a.db().read(|c|Ok(c.prepare("SELECT job_class,arguments FROM ws14g_emitted_jobs WHERE job_class LIKE 'Calendar::%' ORDER BY id")?.query_map([],|r|Ok((r.get(0)?,r.get(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?)).await.unwrap()
}
#[tokio::test]
async fn security_wrong_token_missing_channel_and_sync_do_not_enqueue() {
    let a = app().await;
    let mut b = a.anonymous();
    assert_eq!(
        b.send(req("wrong", "exists", "1")).await.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        b.send(req("", "exists", "1")).await.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        b.send(Req::new(Method::POST, "/google/calendar/notifications"))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        b.send(
            req("fixture-token", "sync", "1")
                .body(b"malformed json".to_vec())
                .header("content-type", "application/json")
        )
        .await
        .status,
        StatusCode::OK
    );
    assert!(jobs(&a).await.is_empty());
}
#[tokio::test]
async fn security_replay_and_concurrent_duplicate_only_enqueue_once() {
    let a = app().await;
    let mut one = a.anonymous();
    let mut two = a.anonymous();
    let (one, two) = tokio::join!(
        one.send(req("fixture-token", "exists", "  +7abc")),
        two.send(req("fixture-token", "exists", "7"))
    );
    assert_eq!(one.status, StatusCode::OK);
    assert_eq!(two.status, StatusCode::OK);
    let actual = jobs(&a).await;
    assert_eq!(
        actual,
        vec![
            ("Calendar::InboundSyncJob".into(), format!("[{DAVID}]")),
            (
                "Calendar::MeetingRefreshJob".into(),
                format!("{{\"user_id\":{DAVID}}}")
            )
        ]
    );
    let mut b = a.anonymous();
    for n in ["7", "6", "0", "-1", "bad"] {
        assert_eq!(
            b.send(req("fixture-token", "exists", n)).await.status,
            StatusCode::OK
        );
    }
    assert_eq!(jobs(&a).await, actual);
}
#[tokio::test]
async fn not_exists_deletes_and_enqueues_rewatch_once() {
    let a = app().await;
    let mut b = a.anonymous();
    assert_eq!(
        b.send(req("fixture-token", "not_exists", "1")).await.status,
        StatusCode::OK
    );
    assert_eq!(
        jobs(&a).await,
        vec![("Calendar::WatchChannelJob".into(), format!("[{DAVID}]"))]
    );
    assert_eq!(
        b.send(req("fixture-token", "not_exists", "1")).await.status,
        StatusCode::NOT_FOUND
    );
}
#[tokio::test]
async fn job_insert_failure_rolls_back_claim_and_channel_deletion() {
    let a = app().await;
    a.db().write(|tx| {tx.conn().execute_batch("CREATE TRIGGER reject_calendar_job BEFORE INSERT ON background_jobs WHEN NEW.job_class LIKE 'Calendar::%' BEGIN SELECT RAISE(ABORT,'fixture queue rollback'); END;")?;Ok(())}).await.unwrap();
    let mut b = a.anonymous();
    for state in ["exists", "not_exists"] {
        assert_eq!(
            b.send(req("fixture-token", state, "9")).await.status,
            StatusCode::INTERNAL_SERVER_ERROR
        );
        a.db().read(|c| {assert!(PushChannel::for_channel(c,"fixture-channel")?.is_some());assert_eq!(c.query_row("SELECT last_message_number FROM calendar_push_channels WHERE channel_id='fixture-channel'",[],|r|r.get::<_,i64>(0))?,0);Ok(())}).await.unwrap();
    }
    assert!(jobs(&a).await.is_empty());
}
#[tokio::test]
async fn claims_and_digest_match_pinned_rails_vectors() {
    let v: serde_json::Value =
        serde_json::from_str(include_str!("../../../../vectors/google_webhook.json")).unwrap();
    assert_eq!(PushChannel::digest("fixture-token"), v["digest"]);
    let a = app().await;
    let mut b = a.anonymous();
    for case in v["claims"].as_array().unwrap() {
        a.db()
            .write(|tx| {
                tx.conn().execute(
                    "UPDATE calendar_push_channels SET last_message_number=0",
                    [],
                )?;
                tx.conn().execute("DELETE FROM background_jobs", [])?;
                tx.conn().execute("DELETE FROM ws14g_emitted_jobs", [])?;
                Ok(())
            })
            .await
            .unwrap();
        let r = b
            .send(req(
                "fixture-token",
                "exists",
                case["input"].as_str().unwrap(),
            ))
            .await;
        assert_eq!(
            r.status,
            if case.get("error").is_some() {
                StatusCode::INTERNAL_SERVER_ERROR
            } else {
                StatusCode::OK
            },
            "{case}"
        );
        assert_eq!(
            jobs(&a).await.len(),
            if case["won"] == true { 2 } else { 0 },
            "{case}"
        );
    }
}
