use crate::controllers::presenters::test_support::{ALL_TALK, DAVID, TestApp};
use crate::huddle::Config;
use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use campfire_db::models::huddle_grant::HuddleGrant;
use campfire_db::{CachedStatements, Membership, Session, Timestamp};
use tower::ServiceExt;

const KEY: &str = "ws13-fixture-api-key";
const SECRET: &str = "ws13-fixture-api-secret";
const GATEWAY: &str = "ws13-fixture-gateway-secret";
pub(super) fn config() -> Config {
    Config::from_lookup(|name| {
        Some(
            match name {
                "LIVEKIT_URL" => "wss://public.example.test",
                "LIVEKIT_INTERNAL_URL" => "http://internal.example.test:7880",
                "LIVEKIT_API_KEY" => KEY,
                "LIVEKIT_API_SECRET" => SECRET,
                _ => GATEWAY,
            }
            .into(),
        )
    })
}
pub(super) async fn request(
    app: &TestApp,
    method: Method,
    path: &str,
    secret: Option<&str>,
    bearer: Option<&str>,
    body: serde_json::Value,
) -> (u16, serde_json::Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if let Some(secret) = secret {
        builder = builder.header("X-Huddle-Gateway-Secret", secret);
    }
    if let Some(bearer) = bearer {
        builder = builder.header("Authorization", bearer);
    }
    let response = app
        .booted
        .router
        .clone()
        .oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    assert_eq!(response.headers().get("cache-control").unwrap(), "no-store");
    let status = response.status().as_u16();
    let bytes = to_bytes(response.into_body(), 100_000).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
    )
}
pub(super) async fn grant(app: &TestApp) -> HuddleGrant {
    app.db()
        .write(|tx| {
            let session = Session::start(tx, DAVID, None, None)?;
            let membership =
                Membership::find_by_room_and_user(tx.conn(), ALL_TALK, DAVID)?.unwrap();
            HuddleGrant::issue(
                tx,
                session.id,
                membership.id,
                membership.room_id,
                &campfire_db::models::room_delete::HuddleConfig {
                    api_secret: Some(SECRET.into()),
                    admin_configured: false,
                },
            )
        })
        .await
        .unwrap()
}
pub(super) fn bearer(app: &TestApp, grant: &HuddleGrant, offset: i64) -> String {
    let token = rails_compat::jwt::livekit::participant_token(
        KEY,
        SECRET,
        &rails_compat::jwt::livekit::Participant {
            name: "David",
            identity: &grant.identity,
            room_name: &grant.room_name,
            can_publish: true,
        },
        app.booted.app.clock.now().as_second() + offset,
    );
    format!("Bearer {token}")
}

#[tokio::test]
async fn ws13b_review_offset_disconnect_preserves_a_newer_rejoin() {
    let app = TestApp::boot_with_huddle(config()).await.expect("parity seed required");
    let grant = grant(&app).await;
    let oracle: serde_json::Value = serde_json::from_str(include_str!("../../../db/src/tests/ws13b_review_fixes.json")).unwrap();
    for case in oracle["boundaries"].as_array().unwrap() {
        let seen: jiff::Timestamp = case["seen_at"].as_str().unwrap().parse().unwrap();
        let id = grant.id;
        app.db().write(move |tx| Ok(tx.conn().execute("UPDATE huddle_grants SET last_seen_at=? WHERE id=?", rusqlite::params![Timestamp::from_jiff(seen), id])?)).await.unwrap();
        let (status, _) = request(&app, Method::POST, &format!("/internal/huddle/grants/{id}/left"), Some(GATEWAY), None, serde_json::json!({"disconnected_at":"2026-01-01 17:00:00 +0500"})).await;
        assert_eq!(status, 200);
        let actual = app.db().read(move |conn| Ok(HuddleGrant::find_by_id(conn, id)?.unwrap().last_seen_at)).await.unwrap();
        let expected = case["seen_after"].as_str().map(|s| Timestamp::from_jiff(s.parse().unwrap()));
        assert_eq!(actual, expected, "{}", case["seen_at"]);
    }
}

#[tokio::test]
async fn huddle_gateway_missing_and_wrong_secret_fail_closed() {
    let Some(app) = TestApp::boot_with_huddle(config()).await else {
        return;
    };
    let grant = grant(&app).await;
    let token = bearer(&app, &grant, 0);
    for secret in [
        None,
        Some(""),
        Some(" "),
        Some("wrong"),
        Some("ws13-fixture-gateway-secreu"),
    ] {
        for (method, path, bearer) in [
            (
                Method::POST,
                "/internal/huddle/authorize".to_string(),
                Some(token.as_str()),
            ),
            (
                Method::GET,
                format!("/internal/huddle/grants/{}", grant.id),
                None,
            ),
            (
                Method::POST,
                format!("/internal/huddle/grants/{}/left", grant.id),
                None,
            ),
        ] {
            assert_eq!(
                request(&app, method, &path, secret, bearer, serde_json::json!({}))
                    .await
                    .0,
                401
            );
        }
    }
}
#[tokio::test]
async fn huddle_gateway_expired_and_malformed_tokens_fail_closed() {
    let Some(app) = TestApp::boot_with_huddle(config()).await else {
        return;
    };
    let grant = grant(&app).await;
    for token in [
        None,
        Some("Bearer malformed"),
        Some("Basic malformed"),
        Some("Bearer "),
    ] {
        assert_eq!(
            request(
                &app,
                Method::POST,
                "/internal/huddle/authorize",
                Some(GATEWAY),
                token,
                serde_json::json!({})
            )
            .await
            .0,
            401
        );
    }
    let expired = bearer(&app, &grant, -121);
    assert_eq!(
        request(
            &app,
            Method::POST,
            "/internal/huddle/authorize",
            Some(GATEWAY),
            Some(&expired),
            serde_json::json!({})
        )
        .await
        .0,
        401
    );
}
#[tokio::test]
async fn huddle_gateway_revoked_and_removed_member_grants_fail_closed() {
    let Some(app) = TestApp::boot_with_huddle(config()).await else {
        return;
    };
    let grant = grant(&app).await;
    let token = bearer(&app, &grant, 0);
    app.db()
        .write(move |tx| Membership::find(tx.conn(), grant.membership_id)?.destroy(tx))
        .await
        .unwrap();
    assert_eq!(
        request(
            &app,
            Method::POST,
            "/internal/huddle/authorize",
            Some(GATEWAY),
            Some(&token),
            serde_json::json!({})
        )
        .await
        .0,
        403
    );
    assert_eq!(
        request(
            &app,
            Method::GET,
            &format!("/internal/huddle/grants/{}", grant.id),
            Some(GATEWAY),
            None,
            serde_json::json!({})
        )
        .await
        .0,
        404
    );
}
#[tokio::test]
async fn huddle_gateway_unconfigured_returns_503_after_authentication() {
    let mut cfg = config();
    cfg.public_url = None;
    let Some(app) = TestApp::boot_with_huddle(cfg).await else {
        return;
    };
    assert_eq!(
        request(
            &app,
            Method::POST,
            "/internal/huddle/authorize",
            None,
            None,
            serde_json::json!({})
        )
        .await
        .0,
        401
    );
    assert_eq!(
        request(
            &app,
            Method::POST,
            "/internal/huddle/authorize",
            Some(GATEWAY),
            None,
            serde_json::json!({})
        )
        .await
        .0,
        StatusCode::SERVICE_UNAVAILABLE.as_u16()
    );
}

#[tokio::test]
async fn huddle_gateway_removed_member_without_callbacks_is_revoked() {
    let Some(app) = TestApp::boot_with_huddle(config()).await else {
        return;
    };
    let grant = grant(&app).await;
    let token = bearer(&app, &grant, 0);
    app.db()
        .write(move |tx| {
            Ok(tx
                .conn()
                .execute_cached("DELETE FROM memberships WHERE id=?", [grant.membership_id])?)
        })
        .await
        .unwrap();
    assert_eq!(
        request(
            &app,
            Method::POST,
            "/internal/huddle/authorize",
            Some(GATEWAY),
            Some(&token),
            serde_json::json!({})
        )
        .await
        .0,
        403
    );
    assert!(
        app.db()
            .read(move |conn| Ok(HuddleGrant::find_by_id(conn, grant.id)?.unwrap().revoked()))
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn huddle_gateway_request_response_vectors_match_pinned_rails() {
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("../huddle/gateway_vectors.json")).unwrap();
    let now = vectors["now"].as_i64().unwrap();
    for case in vectors["cases"].as_array().unwrap() {
        let mut cfg = config();
        if case["configured"] == false {
            cfg.public_url = None;
        }
        let clock = std::sync::Arc::new(campfire_kit::clock::FrozenClock::new(
            jiff::Timestamp::from_second(now).unwrap(),
        ));
        let Some(app) = TestApp::boot_with_huddle_and_clock(cfg, clock).await else {
            return;
        };
        // Inspect the enqueue from this request before the real cleanup worker consumes it.
        let app = app.without_job_runner().await;
        let seen = case["seen"] == true;
        let revoked = case["revoked"] == true;
        let removed = case["removed"] == true;
        app.db().write(move |tx| {
            // Assert committed producer intents even when the real worker has
            // already consumed the row. This audit rolls back with the INSERT.
            tx.conn().execute_batch("CREATE TABLE ws13_gateway_enqueues (job_class TEXT NOT NULL); CREATE TRIGGER ws13_gateway_enqueued AFTER INSERT ON background_jobs WHEN NEW.job_class LIKE 'Huddle::%' BEGIN INSERT INTO ws13_gateway_enqueues(job_class) VALUES(NEW.job_class); END;")?;
            let session = Session::start(tx, DAVID, None, None)?;
            let membership = Membership::find_by_room_and_user(tx.conn(), ALL_TALK, DAVID)?.unwrap();
            let stamp = tx.now();
            tx.conn().execute_cached("INSERT INTO huddle_grants(id,identity,room_name,session_id,user_id,membership_id,room_id,last_seen_at,revoked_at,created_at,updated_at) VALUES(17,?,?,?,?,?,?,?,?,?,?)", rusqlite::params!["ws13-security-participant", "ws13-security-room", session.id, DAVID, membership.id, ALL_TALK, seen.then_some(stamp), revoked.then_some(stamp), stamp, stamp])?;
            if removed { tx.conn().execute_cached("DELETE FROM memberships WHERE id=?", [membership.id])?; }
            Ok(())
        }).await.unwrap();
        let secret = match case.get("secret") {
            Some(v) => v.as_str(),
            None => Some(GATEWAY),
        };
        let token = match case.get("bearer") {
            Some(v) => v.as_str(),
            None => vectors["token"].as_str(),
        };
        let bearer = token.map(|t| format!("{} {t}", case["scheme"].as_str().unwrap_or("Bearer")));
        let method = if case["method"] == "get" {
            Method::GET
        } else {
            Method::POST
        };
        let mut path = case["path"]
            .as_str()
            .unwrap_or("/internal/huddle/authorize")
            .to_string();
        let mut body = case
            .get("params")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));
        if method == Method::GET && case.get("params").is_some() {
            path.push_str("?record_seen=0");
            body = serde_json::json!({});
        }
        let (status, response) =
            request(&app, method, &path, secret, bearer.as_deref(), body).await;
        assert_eq!(
            status,
            case["status"].as_u64().unwrap() as u16,
            "{}",
            case["name"]
        );
        assert_eq!(response, case["body"], "{}", case["name"]);
        let (seen_after, jobs) = app.db().read(|conn| {
            let grant = HuddleGrant::find_by_id(conn, 17)?.unwrap();
            let mut query = conn.prepare("SELECT job_class FROM ws13_gateway_enqueues ORDER BY rowid")?;
            let jobs = query.query_map([], |r| r.get::<_, String>(0))?.collect::<std::result::Result<Vec<_>, _>>()?.into_iter().map(|c| match c.as_str() {"Huddle::BroadcastPresenceJob" => "presence", "Huddle::JoinNoticeJob" => "join", "Huddle::CleanupJob" => "cleanup", _ => panic!("unexpected job {c}")}).collect::<Vec<_>>();
            Ok((grant.last_seen_at.map(Timestamp::as_second), jobs))
        }).await.unwrap();
        assert_eq!(
            serde_json::json!(seen_after),
            case["seen_after"],
            "{}",
            case["name"]
        );
        assert_eq!(serde_json::json!(jobs), case["jobs"], "{}", case["name"]);
    }
}

#[tokio::test]
async fn huddle_sighting_enqueue_rejection_rolls_back_http_request() {
    let Some(app) = TestApp::boot_with_huddle(config()).await else {
        return;
    };
    let app = app.without_job_runner().await;
    let grant = grant(&app).await;
    let token = bearer(&app, &grant, 0);
    app.db().write(|tx| Ok(tx.conn().execute_batch("CREATE TRIGGER reject_huddle_sighting BEFORE INSERT ON background_jobs WHEN NEW.job_class='Huddle::JoinNoticeJob' BEGIN SELECT RAISE(ABORT,'fixture queue failure'); END;")?)).await.unwrap();
    let mut builder = Request::builder()
        .method(Method::POST)
        .uri("/internal/huddle/authorize")
        .header("content-type", "application/json")
        .header("X-Huddle-Gateway-Secret", GATEWAY);
    builder = builder.header("Authorization", token);
    let response = app
        .booted
        .router
        .clone()
        .oneshot(builder.body(Body::from("{}")).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    app.db()
        .read(move |conn| {
            assert!(
                HuddleGrant::find_by_id(conn, grant.id)?
                    .unwrap()
                    .last_seen_at
                    .is_none()
            );
            assert_eq!(
                conn.query_row(
                    "SELECT count(*) FROM background_jobs WHERE job_class LIKE 'Huddle::%'",
                    [],
                    |r| r.get::<_, i64>(0)
                )?,
                0
            );
            Ok(())
        })
        .await
        .unwrap();
}

async fn bind_fixture() -> tokio::net::TcpListener {
    if std::env::var_os("CABLE_TEST_PORT_RANGE").is_some() {
        return crate::channels::tests::support::bind_listener().await;
    }
    for port in 52300..=52339 {
        if let Ok(listener) = tokio::net::TcpListener::bind(("127.0.0.1", port)).await {
            return listener;
        }
    }
    panic!("WS13 Rust fixture port range exhausted")
}
async fn start_node_fixture() -> String {
    use axum::{
        Json, Router,
        routing::{get, post},
    };
    let clock = std::sync::Arc::new(campfire_kit::clock::FrozenClock::new(
        "2026-01-01T12:00:00Z".parse().unwrap(),
    ));
    let app = TestApp::boot_with_huddle_and_clock(config(), clock)
        .await
        .expect("WS13 Node checks require the seed");
    app.db().write(|tx| {
        let session = Session::start(tx, DAVID, None, None)?;
        let membership = Membership::find_by_room_and_user(tx.conn(), ALL_TALK, DAVID)?.unwrap();
        let now = tx.now();
        Ok(tx.conn().execute_cached("INSERT INTO huddle_grants(id,identity,room_name,session_id,user_id,membership_id,room_id,created_at,updated_at) VALUES(17,?,?,?,?,?,?,?,?)", rusqlite::params!["ws13-security-participant", "ws13-security-room", session.id, DAVID, membership.id, ALL_TALK, now, now])?)
    }).await.unwrap();
    let db = app.db().clone();
    let revoke_db = db.clone();
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let stop = std::sync::Arc::new(std::sync::Mutex::new(Some(stop)));
    let controls = Router::new()
        .route("/__ws13/revoke", post(move || {
            let db = revoke_db.clone(); async move {
                db.write(|tx| HuddleGrant::find_by_id(tx.conn(), 17)?.unwrap().revoke(tx, true, &campfire_db::models::room_delete::HuddleConfig { api_secret: Some(SECRET.into()), admin_configured: false })).await.unwrap();
                StatusCode::OK
            }
        }))
        .route("/__ws13/state", get(move || {
            let db = db.clone(); async move {
                Json(db.read(|conn| { let grant = HuddleGrant::find_by_id(conn,17)?.unwrap(); Ok(serde_json::json!({"seen": grant.last_seen_at.map(Timestamp::as_second), "revoked": grant.revoked()})) }).await.unwrap())
            }
        }))
        .route("/__ws13/close", post(move || {
            let stop = stop.clone(); async move { if let Some(stop) = stop.lock().unwrap().take() { let _ = stop.send(()); } StatusCode::OK }
        }));
    let listener = bind_fixture().await;
    let url = format!("http://{}", listener.local_addr().unwrap());
    let router = controls.merge(app.booted.router.clone());
    tokio::spawn(async move {
        let _app = app;
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });
    url
}

#[tokio::test]
#[ignore = "requires Node and the gateway pinned ws package; run explicitly with --ignored"]
async fn huddle_gateway_own_node_suite_against_rust_endpoints() {
    use axum::{Json, Router, routing::post};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    // Build from the committed gateway lockfile in this test's own temporary directory.
    // A clean checkout must not rely on the worker's untracked .scratch/node-deps.
    let dependencies = tempfile::Builder::new().prefix("ws13-gateway-").tempdir().unwrap();
    for file in ["package.json", "package-lock.json"] {
        std::fs::copy(root.join("script/livekit-gateway").join(file), dependencies.path().join(file)).unwrap();
    }
    let install = tokio::process::Command::new("npm")
        .args(["ci", "--ignore-scripts", "--no-audit", "--no-fund"])
        .current_dir(dependencies.path()).kill_on_drop(true).output().await.unwrap();
    assert!(install.status.success(), "gateway dependency install failed: {}", String::from_utf8_lossy(&install.stderr));
    let module = dependencies.path().join("node_modules/ws/wrapper.mjs");
    let listener = bind_fixture().await;
    let control = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new().route(
                "/start",
                post(|| async { Json(serde_json::json!({"url": start_node_fixture().await})) }),
            ),
        )
        .await
        .unwrap();
    });
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("../huddle/gateway_vectors.json")).unwrap();
    let output = tokio::process::Command::new("node")
        .args([
            "--import",
            "./rust/reference-tools/huddle_gateway_node.mjs",
            "--test",
            "script/livekit-gateway/",
        ])
        .current_dir(&root)
        .env("WS13_FIXTURE_CONTROL", control)
        .env("WS13_WS_MODULE", module)
        .env("WS13_JOIN_TOKEN", vectors["token"].as_str().unwrap())
        .kill_on_drop(true)
        .output()
        .await
        .unwrap();
    server.abort();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    print!("{stdout}");
    assert!(
        output.status.success(),
        "gateway Node suite failed:\n{stdout}\n{stderr}"
    );
}
