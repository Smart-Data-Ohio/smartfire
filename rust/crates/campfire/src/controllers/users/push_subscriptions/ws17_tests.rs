use crate::controllers::presenters::test_support::{DAVID, JASON, Req, TestApp};
use crate::net::{BoxFuture, Network, Resolver};
struct FixedDns(std::net::IpAddr);
impl Resolver for FixedDns {
    fn lookup<'a>(&'a self, _: &'a str) -> BoxFuture<'a, std::io::Result<Vec<std::net::IpAddr>>> {
        Box::pin(async move { Ok(vec![self.0]) })
    }
}
use axum::http::{Method, StatusCode};
use std::sync::Arc;
const PATH: &str = "/users/me/push_subscriptions";
async fn app(ip: &str) -> TestApp {
    let network = Network {
        resolver: Arc::new(FixedDns(ip.parse().unwrap())),
        ..Network::system()
    };
    TestApp::boot_with_network(network)
        .await
        .expect("parity seed")
}
fn registration(endpoint: &str) -> Req {
    Req::new(Method::POST, PATH).form(&[
        ("push_subscription[endpoint]", endpoint),
        ("push_subscription[p256dh_key]", "123"),
        ("push_subscription[auth_key]", "456"),
    ])
}
async fn count(app: &TestApp) -> i64 {
    app.db()
        .read(campfire_db::PushSubscription::count)
        .await
        .unwrap()
}
#[tokio::test]
async fn ws17_create_new_push_subscription() {
    let app = app("142.250.123.45").await;
    let mut browser = app.david();
    let before = count(&app).await;
    assert_eq!(
        browser
            .write(
                registration("https://fcm.googleapis.com/fcm/send/abc123")
                    .header("user-agent", "Mozilla/5.0")
            )
            .await
            .status,
        StatusCode::OK
    );
    assert_eq!(count(&app).await, before + 1);
    let s = app
        .db()
        .read(|c| {
            Ok(campfire_db::PushSubscription::for_user(c, DAVID)?
                .pop()
                .unwrap())
        })
        .await
        .unwrap();
    assert_eq!(
        s.endpoint.as_deref(),
        Some("https://fcm.googleapis.com/fcm/send/abc123")
    );
    assert_eq!(s.p256dh_key.as_deref(), Some("123"));
    assert_eq!(s.auth_key.as_deref(), Some("456"));
    assert_eq!(s.user_agent.as_deref(), Some("Mozilla/5.0"));
}
#[tokio::test]
async fn ws17_touch_existing_subscription() {
    let app = app("142.250.123.45").await;
    let mut browser = app.david();
    let before = count(&app).await;
    let s = app
        .db()
        .read(|c| Ok(campfire_db::PushSubscription::for_user(c, DAVID)?.remove(0)))
        .await
        .unwrap();
    let reply = browser
        .write(Req::new(Method::POST, PATH).form(&[
            (
                "push_subscription[endpoint]",
                s.endpoint.as_deref().unwrap(),
            ),
            (
                "push_subscription[p256dh_key]",
                s.p256dh_key.as_deref().unwrap(),
            ),
            (
                "push_subscription[auth_key]",
                s.auth_key.as_deref().unwrap(),
            ),
        ]))
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(count(&app).await, before);
    assert!(
        app.db()
            .read(move |c| campfire_db::PushSubscription::find(c, s.id))
            .await
            .unwrap()
            .updated_at
            > s.updated_at
    );
}
#[tokio::test]
async fn ws17_rejects_subscription_with_non_permitted_endpoint() {
    let app = app("142.250.123.45").await;
    let mut browser = app.david();
    let before = count(&app).await;
    assert_eq!(
        browser
            .write(registration("https://attacker.example.com/steal"))
            .await
            .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(count(&app).await, before);
}
#[tokio::test]
async fn ws17_rejects_subscription_with_endpoint_resolving_to_private_ip() {
    let app = app("169.254.169.254").await;
    let mut browser = app.david();
    let before = count(&app).await;
    assert_eq!(
        browser
            .write(registration("https://fcm.googleapis.com/fcm/send/abc123"))
            .await
            .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(count(&app).await, before);
}
#[tokio::test]
async fn ws17_reregistering_legacy_invalid_subscription_is_rejected() {
    let app = app("142.250.123.45").await;
    let mut browser = app.david();
    app.db().write(|tx| {tx.conn().execute("INSERT INTO push_subscriptions(user_id,endpoint,p256dh_key,auth_key,created_at,updated_at) VALUES (?,'https://attacker.example.com/steal','123','456',?,?)",rusqlite::params![DAVID,tx.now(),tx.now()])?;Ok(())}).await.unwrap();
    let before = count(&app).await;
    assert_eq!(
        browser
            .write(registration("https://attacker.example.com/steal"))
            .await
            .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(count(&app).await, before);
}
#[tokio::test]
async fn ws17_destroy_push_subscription_via_dev_mode() {
    let app = app("142.250.123.45").await;
    let mut browser = app.david();
    let before = count(&app).await;
    let s = app
        .db()
        .read(|c| Ok(campfire_db::PushSubscription::for_user(c, DAVID)?.remove(0)))
        .await
        .unwrap();
    let reply = browser
        .write(Req::new(Method::DELETE, &format!("{PATH}/{}", s.id)))
        .await;
    assert_eq!(
        reply.location(),
        Some("http://campfire.test/users/me/push_subscriptions")
    );
    assert_eq!(count(&app).await, before - 1);
    let other = app
        .db()
        .read(|c| Ok(campfire_db::PushSubscription::for_user(c, JASON)?.remove(0)))
        .await
        .unwrap();
    browser
        .write(Req::new(Method::DELETE, &format!("{PATH}/{}", other.id)))
        .await;
    assert!(
        app.db()
            .read(move |c| campfire_db::PushSubscription::find(c, other.id))
            .await
            .is_ok()
    );
}
#[test]
fn ws17_subscription_user_agent_matches_rails_display() {
    let golden: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../views/tests/golden/ws17-profile-ui.json"
    ))
    .unwrap();
    for row in golden["subscriptions"].as_array().unwrap() {
        let mut s = campfire_db::PushSubscription::new(
            DAVID,
            row["endpoint"].as_str(),
            None,
            None,
            row["user_agent"].as_str(),
        );
        s.id = row["id"].as_i64().unwrap();
        let actual = crate::controllers::presenters::accounts::push_subscription(&s);
        assert_eq!(actual.browser, row["browser"].as_str().unwrap());
        assert_eq!(actual.version, row["version"].as_str().unwrap());
        assert_eq!(actual.platform, row["platform"].as_str().unwrap());
    }
}

#[tokio::test]
async fn ws17_review_malformed_endpoints_reject_new_and_existing_http_writes() {
    let app = app("142.250.123.45").await;
    let mut browser = app.david();
    for endpoint in ["https://fcm.googleapis.com/%zz", "https://fcm.googleapis.com/{invalid}", "https://fcm.googleapis.com/é"] {
        let before = count(&app).await;
        assert_eq!(browser.write(registration(endpoint)).await.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(count(&app).await, before);
        let endpoint = endpoint.to_owned();
        // A row from an old client must be revalidated before its touch, too.
        let stored = app.db().write(move |tx| {
            let id: i64 = tx.conn().query_row("INSERT INTO push_subscriptions(user_id,endpoint,p256dh_key,auth_key,created_at,updated_at) VALUES (?,?, '123','456',?,?) RETURNING id", rusqlite::params![DAVID, endpoint, tx.now(), tx.now()], |row| row.get(0))?;
            campfire_db::PushSubscription::find(tx.conn(), id)
        }).await.unwrap();
        assert_eq!(browser.write(registration(stored.endpoint.as_deref().unwrap())).await.status, StatusCode::UNPROCESSABLE_ENTITY);
        let id = stored.id;
        let after = app.db().read(move |c| campfire_db::PushSubscription::find(c, id)).await.unwrap();
        assert_eq!(after, stored, "rejected legacy registration must not touch the row");
    }
}
