use crate::controllers::presenters::test_support::{DAVID, JASON, Req, TestApp};
use axum::http::{Method, StatusCode};
use campfire_db::PushSubscription;
fn path(id: i64) -> String {
    format!("/users/me/push_subscriptions/{id}/test_notifications")
}
async fn subscription(app: &TestApp, user: i64) -> i64 {
    app.db()
        .read(move |c| Ok(PushSubscription::for_user(c, user)?[0].id))
        .await
        .unwrap()
}
#[tokio::test]
async fn ws17_test_notification_enqueues_an_owned_payload_without_inline_delivery() {
    let app = TestApp::boot()
        .await
        .expect("WS17 requires the parity seed");
    app.db().write(|tx|{tx.conn().execute_batch("CREATE TRIGGER ws17_hold_test_push AFTER INSERT ON background_jobs WHEN NEW.job_class='Push::Subscription::TestNotificationJob' BEGIN UPDATE background_jobs SET run_at='2099-01-01 00:00:00' WHERE id=NEW.id; END;")?;Ok(())}).await.unwrap();
    let id = subscription(&app, DAVID).await;
    let mut browser = app.david();
    let reply = browser.write(Req::new(Method::POST, &path(id))).await;
    assert_eq!(
        reply.location(),
        Some("http://campfire.test/users/me/push_subscriptions")
    );
    let args: String=app.db().read(|c|Ok(c.query_row("SELECT arguments FROM background_jobs WHERE job_class='Push::Subscription::TestNotificationJob'",[],|r|r.get(0))?)).await.unwrap();
    let args: super::TestNotificationJob = serde_json::from_str(&args).unwrap();
    assert_eq!(args.subscription_id, id);
    assert_eq!(args.user_id, DAVID);
    assert_eq!(
        args.path,
        "http://campfire.test/users/me/push_subscriptions"
    );
    assert_eq!(
        uuid::Uuid::parse_str(&args.body).unwrap().get_version_num(),
        4
    );
}
#[tokio::test]
async fn ws17_test_notification_cannot_target_another_users_subscription_or_bypass_auth() {
    let app = TestApp::boot()
        .await
        .expect("WS17 requires the parity seed");
    let id = subscription(&app, JASON).await;
    let mut browser = app.david();
    for id in [id, -1] {
        assert_eq!(
            browser
                .write(Req::new(Method::POST, &path(id)))
                .await
                .status,
            StatusCode::NOT_FOUND
        );
    }
    assert_eq!(
        app.anonymous()
            .write(Req::new(Method::POST, &path(id)))
            .await
            .location(),
        Some("http://campfire.test/session/new")
    );
}
#[tokio::test]
async fn ws17_test_notification_enqueue_failure_is_a_full_http_rollback() {
    let app = TestApp::boot()
        .await
        .expect("WS17 requires the parity seed");
    let app = app.without_job_runner().await;
    let id = subscription(&app, DAVID).await;
    let before = app
        .db()
        .read(move |c| PushSubscription::find(c, id))
        .await
        .unwrap();
    app.db().write(|tx|{tx.conn().execute_batch("CREATE TRIGGER ws17_reject_test_push BEFORE INSERT ON background_jobs WHEN NEW.job_class='Push::Subscription::TestNotificationJob' BEGIN SELECT RAISE(ABORT,'ws17 deliberate enqueue failure'); END;")?;Ok(())}).await.unwrap();
    let mut browser = app.david();
    let reply = browser.write(Req::new(Method::POST, &path(id))).await;
    assert_eq!(reply.status, StatusCode::INTERNAL_SERVER_ERROR);
    let after = app
        .db()
        .read(move |c| PushSubscription::find(c, id))
        .await
        .unwrap();
    assert_eq!(after, before);
    assert_eq!(app.db().read(|c|Ok(c.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Push::Subscription::TestNotificationJob'",[],|r|r.get::<_,i64>(0))?)).await.unwrap(),0);
}
