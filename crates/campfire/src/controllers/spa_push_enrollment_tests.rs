//! Browser enrollment uses the classic push controller over the same frozen database.

use std::sync::Arc;

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use campfire_kit::FrozenClock;
use serde_json::{Value, json};

use crate::controllers::presenters::test_support::{
    BENDER, BENDER_KEY, Browser, DAVID, KEVIN, Reply, Req, SEED_NOW, TestApp,
};
use crate::integrations::test_support::FakeResolver;

const ENDPOINT: &str = "https://fcm.googleapis.com/fcm/send/spa-enrollment";
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:141.0) Gecko/20100101 Firefox/141.0";
const PATH: &str = "/api/v1/settings/push_subscriptions";
// Public P-256 generator point and scalar 1: deterministic test keys, never production keys.
const PUBLIC_KEY: &str =
    "BGsX0fLhLEJH-Lzm5WOkQPJ3A32BLeszoPShOUXYmMKWT-NC4v4af5uO5-tKfA-eFivOM1drMV7Oy7ZAaDe_UfU=";
const PRIVATE_KEY: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAE=";

async fn app(extra: &[(&str, &str)]) -> Option<(TestApp, Arc<FrozenClock>, Arc<FakeResolver>)> {
    let clock = Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
    let resolver = Arc::new(FakeResolver::new([(
        "fcm.googleapis.com",
        vec!["142.250.1.1"],
    )]));
    let mut network = crate::net::Network::system();
    network.resolver = resolver.clone();
    let vars = [&[("SPA_ENABLED", "1")][..], extra].concat();
    let app = TestApp::boot_with_network_clock_and_env(network, clock.clone(), &vars)
        .await?
        .without_job_runner()
        .await;
    Some((app, clock, resolver))
}

fn get(path: &str) -> Req {
    Req::new(Method::GET, path).header("accept", "application/json")
}

fn json_body(method: Method, path: &str, body: Value) -> Req {
    Req::new(method, path)
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .header("user-agent", USER_AGENT)
        .body(body.to_string())
}

fn body(endpoint: &str, p256dh: &str, auth: &str) -> Value {
    json!({"endpoint": endpoint, "p256dhKey": p256dh, "authKey": auth})
}

fn parse<T: serde::de::DeserializeOwned>(reply: &Reply) -> T {
    serde_json::from_slice(&reply.body).unwrap_or_else(|error| panic!("{error}: {}", reply.text()))
}

async fn rows(app: &TestApp) -> Value {
    app.db().read(|conn| {
        let strings = |sql: &str| -> rusqlite::Result<Vec<String>> {
            conn.prepare(sql)?.query_map([], |row| row.get(0))?.collect()
        };
        Ok(json!({
            "subscriptions": strings("SELECT json_array(id, user_id, endpoint, p256dh_key, auth_key, user_agent, created_at, updated_at) FROM push_subscriptions ORDER BY id")?,
            "audit": strings("SELECT json_array(action, details, actor_id, target_id, created_at) FROM audit_logs ORDER BY id")?,
            "jobs": conn.query_row("SELECT COUNT(*) FROM background_jobs", [], |row| row.get::<_, i64>(0))?,
        }))
    }).await.unwrap()
}

async fn create(
    browser: &mut Browser<'_>,
    classic: bool,
    endpoint: &str,
    p256dh: &str,
    auth: &str,
) -> Reply {
    let request = if classic {
        Req::new(Method::POST, "/users/me/push_subscriptions")
            .header("user-agent", USER_AGENT)
            .form(&[
                ("push_subscription[endpoint]", endpoint),
                ("push_subscription[p256dh_key]", p256dh),
                ("push_subscription[auth_key]", auth),
            ])
    } else {
        json_body(Method::POST, PATH, body(endpoint, p256dh, auth))
    };
    browser.write(request).await
}

#[tokio::test]
async fn public_key_is_the_classic_presented_key_and_missing_or_invalid_config_is_null() {
    for vars in [
        vec![],
        vec![
            ("VAPID_PUBLIC_KEY", "invalid"),
            ("VAPID_PRIVATE_KEY", "invalid"),
        ],
        vec![
            ("VAPID_PUBLIC_KEY", PUBLIC_KEY),
            ("VAPID_PRIVATE_KEY", PRIVATE_KEY),
        ],
    ] {
        let Some((app, _, _)) = app(&vars).await else {
            return;
        };
        let mut browser = app.sign_in(DAVID).await;
        let reply = browser.send(get(&format!("{PATH}/key"))).await;
        assert_eq!(reply.status, StatusCode::OK);
        assert_eq!(reply.header("cache-control"), Some("no-store"));
        let key: api::PushPublicKey = parse(&reply);
        assert_eq!(key.public_key, app.booted.app.vapid_public_key());
        assert_eq!(
            key.public_key.is_some(),
            vars.iter().any(|(_, value)| *value == PUBLIC_KEY)
        );

    }
}

#[tokio::test]
async fn enrollment_matches_classic_rows_exact_deduplication_user_agent_and_user_scope() {
    let Some((classic_app, classic_clock, _)) = app(&[]).await else {
        return;
    };
    let Some((spa_app, spa_clock, _)) = app(&[]).await else {
        return;
    };
    let mut classic = classic_app.sign_in(DAVID).await;
    let mut spa = spa_app.sign_in(DAVID).await;
    let classic_capture = classic_app.booted.app.cable.capture_every_publication();
    let spa_capture = spa_app.booted.app.cable.capture_every_publication();
    let before = rows(&spa_app).await;
    for (p256dh, auth) in [
        ("p256", "auth"),
        ("p256", "auth"),
        ("rotated", "auth"),
        ("rotated", "rotated"),
    ] {
        classic_clock.advance(jiff::SignedDuration::from_secs(60));
        spa_clock.advance(jiff::SignedDuration::from_secs(60));
        assert_eq!(
            create(&mut classic, true, ENDPOINT, p256dh, auth)
                .await
                .status,
            StatusCode::OK
        );
        let reply = create(&mut spa, false, ENDPOINT, p256dh, auth).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        let list: api::PushSubscriptionList = parse(&reply);
        assert!(
            list.push_subscriptions
                .iter()
                .any(|row| row.endpoint == ENDPOINT && row.browser == "Firefox")
        );
        assert_eq!(rows(&classic_app).await, rows(&spa_app).await);
    }
    let mut classic_other = classic_app.sign_in(KEVIN).await;
    let mut spa_other = spa_app.sign_in(KEVIN).await;
    create(&mut classic_other, true, ENDPOINT, "p256", "auth").await;
    create(&mut spa_other, false, ENDPOINT, "p256", "auth").await;
    assert_eq!(rows(&classic_app).await, rows(&spa_app).await);
    let subscriptions = spa_app
        .db()
        .read(|conn| campfire_db::PushSubscription::for_user(conn, DAVID))
        .await
        .unwrap();
    let matches: Vec<_> = subscriptions
        .iter()
        .filter(|row| row.endpoint.as_deref() == Some(ENDPOINT))
        .collect();
    assert_eq!(
        matches.len(),
        3,
        "repeat enrollment touches; rotated keys create separate rows"
    );
    assert!(
        matches
            .iter()
            .all(|row| row.user_agent.as_deref() == Some(USER_AGENT))
    );
    assert!(matches.iter().any(|row| row.created_at != row.updated_at));
    let after = rows(&spa_app).await;
    assert_eq!(after["audit"], before["audit"]);
    assert_eq!(after["jobs"], before["jobs"]);
    tokio::task::yield_now().await;
    assert!(classic_capture.take().is_empty());
    assert!(spa_capture.take().is_empty());
}

#[tokio::test]
async fn invalid_endpoints_and_repeated_enrollment_with_private_dns_refuse_as_classic_does() {
    let Some((classic_app, _, classic_dns)) = app(&[]).await else {
        return;
    };
    let Some((spa_app, _, spa_dns)) = app(&[]).await else {
        return;
    };
    let mut classic = classic_app.sign_in(DAVID).await;
    let mut spa = spa_app.sign_in(DAVID).await;
    create(&mut classic, true, ENDPOINT, "p256", "auth").await;
    create(&mut spa, false, ENDPOINT, "p256", "auth").await;
    let before = rows(&spa_app).await;
    classic_dns.set(
        "fcm.googleapis.com",
        vec![vec!["127.0.0.1".parse().unwrap()]],
    );
    spa_dns.set(
        "fcm.googleapis.com",
        vec![vec!["127.0.0.1".parse().unwrap()]],
    );
    for endpoint in [
        "",
        "invalid",
        "http://fcm.googleapis.com/push",
        "https://fcm.googleapis.com:8443/push",
        "https://example.com/push",
        ENDPOINT,
    ] {
        assert_eq!(
            create(&mut classic, true, endpoint, "p256", "auth")
                .await
                .status,
            StatusCode::UNPROCESSABLE_ENTITY
        );
        let reply = create(&mut spa, false, endpoint, "p256", "auth").await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            reply.text()
        );
        let error: api::ApiErrorResponse = parse(&reply);
        assert!(matches!(error.error, api::ApiError::Validation { .. }));
        assert_eq!(rows(&classic_app).await, before);
        assert_eq!(rows(&spa_app).await, before);
    }
}

#[tokio::test]
async fn enrollment_requires_csrf_a_typed_body_and_the_settings_body_limit() {
    let Some((app, _, _)) = app(&[]).await else {
        return;
    };
    let mut browser = app.sign_in(DAVID).await;
    browser.authenticity_token().await;
    let before = rows(&app).await;
    let reply = browser
        .send(json_body(
            Method::POST,
            PATH,
            body(ENDPOINT, "p256", "auth"),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(matches!(
        parse::<api::ApiErrorResponse>(&reply).error,
        api::ApiError::InvalidAuthenticityToken { .. }
    ));
    for body in [
        json!({}),
        json!({"endpoint": ENDPOINT, "p256dhKey": 2, "authKey": "auth"}),
        json!({"endpoint": ENDPOINT, "p256dhKey": "p256", "authKey": "auth", "userId": KEVIN}),
        body(ENDPOINT, &"a".repeat(65 * 1024), "auth"),
    ] {
        let reply = browser.write(json_body(Method::POST, PATH, body)).await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            reply.text()
        );
        assert!(matches!(
            parse::<api::ApiErrorResponse>(&reply).error,
            api::ApiError::Validation { .. }
        ));
        assert_eq!(rows(&app).await, before);
    }
}

#[tokio::test]
async fn every_push_personal_route_refuses_anonymous_bot_sessions_bot_keys_and_warmed_agent_tokens()
{
    use crate::controllers::agent_http_tests::{SECRET, initialize};
    let Some((app, _, _)) = app(&[]).await else {
        return;
    };
    initialize(&app).await;
    let mut anonymous = app.anonymous();
    let mut bot = app.sign_in(BENDER).await;
    let authorization = format!("Bearer {SECRET}");
    anonymous
        .send(get("/api/v1/me").header("authorization", &authorization))
        .await;
    let before = rows(&app).await;
    for (method, path) in [
        (Method::GET, PATH.to_string()),
        (Method::GET, format!("{PATH}/key")),
        (Method::POST, PATH.to_string()),
        (Method::DELETE, format!("{PATH}/1")),
    ] {
        for (kind, expected) in [
            ("anonymous", StatusCode::UNAUTHORIZED),
            ("bot_key", StatusCode::FORBIDDEN),
            ("agent", StatusCode::FORBIDDEN),
            ("bot_session", StatusCode::FORBIDDEN),
        ] {
            let path = if kind == "bot_key" {
                format!("{path}?bot_key={BENDER_KEY}")
            } else {
                path.clone()
            };
            let mut request = json_body(method.clone(), &path, body(ENDPOINT, "p256", "auth"));
            if kind == "agent" {
                request = request.header("authorization", &authorization);
            }
            let reply = if kind == "bot_session" {
                bot.write(request).await
            } else {
                anonymous.send(request).await
            };
            assert_eq!(reply.status, expected, "{kind} {path}: {}", reply.text());
            assert_eq!(rows(&app).await, before);
        }
    }
}

#[tokio::test]
async fn delete_is_classic_scoped_and_idempotent_without_audit_jobs_or_broadcasts() {
    let Some((classic_app, _, _)) = app(&[]).await else {
        return;
    };
    let Some((spa_app, _, _)) = app(&[]).await else {
        return;
    };
    let mut classic = classic_app.sign_in(DAVID).await;
    let mut spa = spa_app.sign_in(DAVID).await;
    create(&mut classic, true, ENDPOINT, "p256", "auth").await;
    create(&mut spa, false, ENDPOINT, "p256", "auth").await;
    let id = spa_app
        .db()
        .read(|conn| {
            Ok(campfire_db::PushSubscription::find_for_user_by_keys(
                conn, DAVID, ENDPOINT, "p256", "auth",
            )?
            .unwrap()
            .id)
        })
        .await
        .unwrap();
    let capture = spa_app.booted.app.cable.capture_every_publication();
    for viewer in [KEVIN, DAVID, DAVID] {
        let mut classic = classic_app.sign_in(viewer).await;
        let mut spa = spa_app.sign_in(viewer).await;
        let classic_reply = classic
            .write(Req::new(
                Method::DELETE,
                &format!("/users/me/push_subscriptions/{id}"),
            ))
            .await;
        assert_eq!(classic_reply.status, StatusCode::FOUND);
        let spa_reply = spa
            .write(json_body(
                Method::DELETE,
                &format!("{PATH}/{id}"),
                Value::Null,
            ))
            .await;
        assert_eq!(spa_reply.status, StatusCode::OK);
        assert_eq!(rows(&classic_app).await, rows(&spa_app).await);
    }
    tokio::task::yield_now().await;
    assert!(capture.take().is_empty());
}

/// Holds the first two lookups until both have arrived, so two enrollments have both missed
/// the row before either saves it. The first to arrive then resolves publicly at once; the
/// second waits for `release` (sent once the first enrollment's row is committed), then answers
/// `second`. Later lookups resolve publicly at once.
struct OverlappingResolver {
    arrived: std::sync::atomic::AtomicUsize,
    both: tokio::sync::watch::Sender<bool>,
    release: tokio::sync::watch::Sender<bool>,
    second: Option<&'static str>,
}

const PUBLIC: &str = "142.250.1.1";

/// The second lookup's answer: resolution fails outright.
const UNRESOLVABLE: Option<&str> = None;

impl crate::net::Resolver for OverlappingResolver {
    fn lookup<'a>(
        &'a self,
        _host: &'a str,
    ) -> crate::net::BoxFuture<'a, std::io::Result<Vec<std::net::IpAddr>>> {
        Box::pin(async move {
            let mut both = self.both.subscribe();
            let mut release = self.release.subscribe();
            let arrival = self
                .arrived
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                + 1;
            if arrival >= 2 {
                self.both.send_replace(true);
            }
            let _ = both.wait_for(|ready| *ready).await;
            if arrival != 2 {
                return Ok(vec![PUBLIC.parse().unwrap()]);
            }
            let _ = release.wait_for(|released| *released).await;
            match self.second {
                Some(address) => Ok(vec![address.parse().unwrap()]),
                None => Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "no address",
                )),
            }
        })
    }
}

#[derive(Clone)]
struct LogCapture(Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for LogCapture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct Overlap {
    statuses: Vec<StatusCode>,
    saved: usize,
    log: String,
}

/// The saved rows with this test's subscription tuple.
async fn saved_tuples(app: &TestApp) -> usize {
    app.db()
        .read(|conn| campfire_db::PushSubscription::for_user(conn, DAVID))
        .await
        .unwrap()
        .into_iter()
        .filter(|row| {
            row.endpoint.as_deref() == Some(ENDPOINT)
                && row.p256dh_key.as_deref() == Some("p256")
                && row.auth_key.as_deref() == Some("auth")
        })
        .count()
}

/// A classic post and an SPA post for one subscription, overlapping as two tabs would: both
/// miss the row, the first commits its insert, and only then does the second resolve and write.
async fn overlapping_enrollments(second: Option<&'static str>) -> Option<Overlap> {
    let log = Arc::new(std::sync::Mutex::new(vec![]));
    let writer = log.clone();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(move || LogCapture(writer.clone()))
        .finish();
    // Current-thread test runtime: the capture covers every await below. A second registrar
    // keeps callsite interest computed from both live dispatches (see the Google log test).
    let _capture = tracing::subscriber::set_default(subscriber);
    let _other_dispatch = tracing::Dispatch::new(tracing::subscriber::NoSubscriber::default());

    let clock = Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
    let resolver = Arc::new(OverlappingResolver {
        arrived: Default::default(),
        both: tokio::sync::watch::channel(false).0,
        release: tokio::sync::watch::channel(false).0,
        second,
    });
    let mut network = crate::net::Network::system();
    network.resolver = resolver.clone();
    let app =
        TestApp::boot_with_network_clock_and_env(network, clock, &[("SPA_ENABLED", "1")]).await?;
    let app = app.without_job_runner().await;
    let mut classic = app.sign_in(DAVID).await;
    let mut spa = app.sign_in(DAVID).await;
    let ((classic_reply, spa_reply), ()) =
        tokio::time::timeout(std::time::Duration::from_secs(20), async {
            tokio::join!(
                async {
                    tokio::join!(
                        create(&mut classic, true, ENDPOINT, "p256", "auth"),
                        create(&mut spa, false, ENDPOINT, "p256", "auth"),
                    )
                },
                async {
                    // The second lookup answers only after the first insert is committed.
                    while saved_tuples(&app).await == 0 {
                        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                    }
                    resolver.release.send_replace(true);
                },
            )
        })
        .await
        .expect("both enrollments reach name resolution and finish");
    let mut statuses = vec![classic_reply.status, spa_reply.status];
    statuses.sort();
    let log = String::from_utf8(log.lock().unwrap().clone()).unwrap();
    Some(Overlap {
        statuses,
        saved: saved_tuples(&app).await,
        log,
    })
}

const OVERLAP_LOG: &str = "push subscription enrollment found the row an overlapping request saved";

#[tokio::test]
async fn overlapping_classic_and_spa_enrollments_save_one_subscription() {
    let Some(overlap) = overlapping_enrollments(Some(PUBLIC)).await else {
        return;
    };
    assert_eq!(overlap.statuses, [StatusCode::OK, StatusCode::OK]);
    assert_eq!(
        overlap.saved, 1,
        "the later enrollment touches the row the earlier one saved"
    );
    assert_eq!(
        overlap
            .log
            .matches(&format!("{OVERLAP_LOG} outcome=OverlapTouched"))
            .count(),
        1,
        "the later enrollment took the in-transaction recheck: {}",
        overlap.log
    );
}

#[tokio::test]
async fn an_overlapping_enrollment_that_fails_its_own_validation_is_refused() {
    // Sequentially, the second request would find the saved row, re-validate it with its own
    // resolution, and answer 422 when that resolution is private or fails. Overlapping must too.
    for second in [UNRESOLVABLE, Some("10.0.0.7"), Some("127.0.0.1")] {
        let Some(overlap) = overlapping_enrollments(second).await else {
            return;
        };
        assert_eq!(
            overlap.statuses,
            [StatusCode::OK, StatusCode::UNPROCESSABLE_ENTITY],
            "second resolution {second:?}"
        );
        assert_eq!(overlap.saved, 1, "second resolution {second:?}");
        assert_eq!(
            overlap
                .log
                .matches(&format!("{OVERLAP_LOG} outcome=OverlapRefused"))
                .count(),
            1,
            "second resolution {second:?} was refused by the in-transaction recheck: {}",
            overlap.log
        );
    }
}
