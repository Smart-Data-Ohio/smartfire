use crate::{
    app::App,
    config::Config,
    integrations::{
        github::{
            accounts::{Account, AccountInput},
            client::ReadClient,
            tests::{crypto, fake},
        },
        test_support::{FakeServer, Route},
    },
};
use axum::{body::Body, http::Request};
use campfire_db::{NewSession, Session, fixtures};
use campfire_kit::Crypto;
use rusqlite::params;
use serde_json::{Value, json};
use tower::ServiceExt;
pub(super) struct Fresh {
    pub(super) app: App,
    pub(super) router: axum::Router,
    _dir: tempfile::TempDir,
    pub(super) server: FakeServer,
    pub(super) cookie: String,
    pub(super) clock: std::sync::Arc<campfire_kit::FrozenClock>,
}
fn cases() -> Value {
    serde_json::from_str(include_str!("../../../../../vectors/github_card_http.json")).unwrap()
}
impl Fresh {
    pub(super) async fn new(case: &Value) -> Self {
        let status = case["access"].as_u64().unwrap_or(200) as u16;
        Self::with_routes(
            case,
            vec![Route::new("GET", "api.github.com", "/repos/rails/rails", status).body("{}")],
        )
        .await
    }
    pub(super) async fn with_routes(case: &Value, routes: Vec<Route>) -> Self {
        Self::with_networks(case, routes, crate::net::Network::system()).await
    }
    pub(super) async fn with_networks(
        case: &Value,
        routes: Vec<Route>,
        subscription_network: crate::net::Network,
    ) -> Self {
        let (server, network) = fake(routes).await;
        let scratch =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.scratch/ws15g");
        std::fs::create_dir_all(&scratch).unwrap();
        let dir = tempfile::tempdir_in(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.scratch/ws15g"),
        )
        .unwrap();
        let secret: Value =
            serde_json::from_str(include_str!("../../../../../vectors/github.json")).unwrap();
        let mut config = Config::from_lookup(|key| match key {
            "SECRET_KEY_BASE" => secret["secret_key_base"].as_str().map(str::to_owned),
            "CAMPFIRE_STORAGE_PATH" => Some(dir.path().to_string_lossy().into_owned()),
            "DISABLE_SSL" => Some("1".into()),
            "GITHUB_WEBHOOK_SECRET" => case["webhook_secret"].as_str().map(str::to_owned),
            _ => None,
        })
        .unwrap();
        // Cases without the SPA test the classic pages, until they're deleted.
        config.spa_enabled = case["spa_enabled"].as_bool().unwrap_or(false);
        let clock = std::sync::Arc::new(campfire_kit::FrozenClock::new(
            "2026-01-01T12:00:00Z".parse().unwrap(),
        ));
        let booted = crate::server::boot_with_all_services(
            config,
            clock.clone(),
            ReadClient::with_network(
                case["reader_token"].as_str().map(str::to_owned),
                network.clone(),
            ),
            crate::integrations::github::client::AppClient::with_network(
                case["app_configured"]
                    .as_bool()
                    .unwrap_or(false)
                    .then(|| "fixture-client".into()),
                case["app_configured"]
                    .as_bool()
                    .unwrap_or(false)
                    .then(|| "fixture-secret".into()),
                network.clone(),
            ),
            network,
            subscription_network,
            crate::jobs::periodic::Intervals::from_env(),
        )
        .await
        .unwrap();
        booted
            .jobs
            .shutdown(std::time::Duration::from_secs(1))
            .await;
        let input = case.clone();
        let encryption = crypto();
        let session=booted.app.db.write(move|tx| {
   fixtures::load(tx.conn(),&fixtures::reference_dir(),&fixtures::Options {now:tx.now(),bcrypt_cost:4})?;
   let now=tx.now();tx.conn().execute("INSERT INTO users (id,name,role,created_at,updated_at) VALUES (811,'Oracle',?, ?,?)",params![input["role"].as_i64().unwrap_or(1),now,now])?;
   tx.conn().execute("INSERT INTO users (id,name,created_at,updated_at) VALUES (812,'Other',?,?)",params![now,now])?;
   for id in [815,825] {tx.conn().execute("INSERT INTO rooms (id,type,creator_id,name,created_at,updated_at) VALUES (?,?,?,'Cards',?,?)",params![id,input["kind"].as_str().unwrap_or("Rooms::Closed"),input["room_creator"].as_i64().unwrap_or(811),now,now])?;}
   if input["member"]!=false {tx.conn().execute("INSERT INTO memberships (room_id,user_id,created_at,updated_at) VALUES (815,811,?,?)",params![now,now])?;}
   for (id,room,key) in [(818,815,"card-parent"),(828,825,"other-parent")] {tx.conn().execute("INSERT INTO messages (id,room_id,creator_id,client_message_id,created_at,updated_at) VALUES (?,?,811,?,?,?)",params![id,room,key,now,now])?;}
   tx.conn().execute("INSERT INTO channel_threads (id,room_id,creator_id,parent_message_id,name,last_activity_at,created_at,updated_at) VALUES (817,815,811,818,'Discussion',?,?,?)",params![now,now,now])?;
   tx.conn().execute("INSERT INTO github_pull_requests (id,owner,repo,number,title,state,private,fetched_at,changed_files,created_at,updated_at) VALUES (816,'rails','rails',12,'Secret title','open',?,?,?,?,?)",params![input.get("private").map(|v|v.as_bool()).unwrap_or(Some(true)),now,json!({"files":[{"filename":"app/a.rb","status":"modified","additions":4,"deletions":2}],"total_count":3}).to_string(),now,now])?;
   if input["reference"]!=false {tx.conn().execute("INSERT INTO github_pull_request_references (github_pull_request_id,message_id,created_at,updated_at) VALUES (816,818,?,?)",params![now,now])?;}
   if input["mapping"]!=false {tx.conn().execute("INSERT INTO github_pull_request_threads (github_pull_request_id,room_id,channel_thread_id,created_at,updated_at) VALUES (816,815,817,?,?)",params![now,now])?;}
   if input["linked"]==true {let a=Account::create(tx,&encryption,&AccountInput {user_id:811,github_login:"oracle",access_token:"fixture-viewer-token",refresh_token:None,token_expires_at:None,token_source:"pat"})?;if input["initially_disconnected"]==true{Account::mark_disconnected(tx,a.id,"Disconnected")?;}}
   if input["deleted"]==true {tx.conn().execute("UPDATE rooms SET deleted_at=? WHERE id=815",[now])?;}
   Session::start_with(tx,811,NewSession {two_factor_verified:true,..Default::default()})
  }).await.unwrap();
        let cookie = campfire_kit::RailsCrypto::new(booted.app.secrets.clone())
            .sign_cookie("session_token", &session.token, None)
            .replace('+', "%2B")
            .replace('/', "%2F")
            .replace('=', "%3D");
        Self {
            app: booted.app,
            router: booted.router,
            _dir: dir,
            server,
            cookie: format!("session_token={cookie}"),
            clock,
        }
    }
    async fn request(&self, case: &Value) -> (u16, String, String) {
        let context = if case["context"] == false {
            String::new()
        } else if case["thread"] == true {
            format!(
                "?thread_id=817{}",
                if case["both"] == true {
                    "&message_id=818"
                } else {
                    ""
                }
            )
        } else {
            format!(
                "?message_id={}",
                case.get("message")
                    .map(|v| v
                        .as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| v.to_string()))
                    .unwrap_or("818".into())
            )
        };
        let request = Request::builder()
            .uri(format!("/rooms/815/github/pull_requests/816/card{context}"))
            .header("Host", "example.org")
            .header("Cookie", &self.cookie)
            .body(Body::empty())
            .unwrap();
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status().as_u16();
        let content = response
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        (status, String::from_utf8(body.to_vec()).unwrap(), content)
    }
}
#[tokio::test]
async fn github_viewer_card_security_checks_membership_and_exact_context_before_token_access() {
    for name in [
        "nonmember",
        "cross_room",
        "unreferenced",
        "missing",
        "missing_thread_mapping",
        "deleted_room",
    ] {
        let case = cases()
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == name)
            .unwrap()
            .clone();
        let fresh = Fresh::new(&case).await;
        let (status, _, _) = fresh.request(&case).await;
        assert_eq!(status, 404, "{name}");
        assert!(fresh.server.received().is_empty(), "{name}");
    }
}
#[tokio::test]
async fn github_viewer_card_http_frames_statuses_permissions_and_bodies_match_rails() {
    for case in cases().as_array().unwrap() {
        let fresh = Fresh::new(case).await;
        for expected in case["responses"].as_array().unwrap() {
            let (status, body, content) = fresh.request(case).await;
            assert_eq!(status, expected["status"], "{}", case["name"]);
            if status == 200 {
                assert_eq!(body, expected["body"], "{}", case["name"]);
                assert_eq!(content, expected["content_type"], "{}", case["name"]);
            } else {
                assert!(!body.contains("Secret title"));
            }
        }
        assert_eq!(
            fresh.server.received().len(),
            case["requests"].as_array().unwrap().len(),
            "{}",
            case["name"]
        );
        let disconnected = fresh
            .app
            .db
            .read(|conn| Ok(Account::for_user(conn, 811)?.and_then(|a| a.disconnected_reason)))
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_value(disconnected).unwrap(),
            case["disconnected"],
            "{}",
            case["name"]
        );
    }
}

// Independent review regressions: these assertions also compile against 1325b624.
#[tokio::test]
async fn review_required_github_routes_reach_authenticated_handlers() {
    let fresh = Fresh::new(&json!({"private":false})).await;
    let raw_csrf =
        base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, [7u8; 32]);
    let values = json!({"_csrf_token":raw_csrf,"sudo_verified_at":1767268800});
    let raw = campfire_kit::RailsCrypto::new(fresh.app.secrets.clone()).encrypt_cookie(
        "_campfire_session",
        &values,
        None,
    );
    let cookie = format!(
        "{}; _campfire_session={}",
        fresh.cookie,
        url::form_urlencoded::byte_serialize(raw.as_bytes()).collect::<String>()
    );
    let mut observed = Vec::new();
    let mut expected = Vec::new();
    for (method, path, status) in [
        ("GET", "/github/app/connect", 404),
        ("GET", "/github/app/callback", 404),
        ("POST", "/github/connection", 302),
        ("DELETE", "/github/connection", 302),
        (
            "GET",
            "/rooms/815/github/pull_request_write_actions/816",
            200,
        ),
        ("POST", "/rooms/815/github/pull_request_comments", 422),
        ("POST", "/rooms/815/agents/github/pull_request_actions", 403),
    ] {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .header("Host", "example.org")
            .header("Cookie", &cookie)
            .header(
                "X-CSRF-Token",
                campfire_kit::csrf::mask(&[7u8; 32], [9u8; 32]),
            )
            .header("Content-Type", "application/json")
            .body(Body::from(
                json!({"pull_request_id":816,"body":""}).to_string(),
            ))
            .unwrap();
        let response = fresh.router.clone().oneshot(request).await.unwrap();
        observed.push((path, response.status().as_u16()));
        expected.push((path, status));
    }
    assert_eq!(
        observed, expected,
        "reviewed workflows must reach their own authenticated controller, never the 501 fallback"
    );
    assert!(fresh.server.received().is_empty());
}
#[tokio::test]
async fn review_stale_room_card_enqueues_one_refresh_and_serves_queue_failure() {
    let fresh = Fresh::new(&json!({"private":false})).await;
    fresh.app.db.write(|tx| {tx.conn().execute_batch("DELETE FROM background_jobs; UPDATE github_pull_requests SET fetched_at=NULL,fetch_requested_at=NULL;")?;Ok(())}).await.unwrap();
    for _ in 0..2 {
        let response = fresh
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/rooms/815")
                    .header("Host", "example.org")
                    .header("Cookie", &fresh.cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let html = String::from_utf8(
            axum::body::to_bytes(response.into_body(), 1024 * 1024)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        assert!(html.contains("github-pr-card"));
        let jobs:i64=fresh.app.db.read(|conn|Ok(conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Github::FetchPullRequestJob'",[],|r|r.get(0))?)).await.unwrap();
        assert_eq!(jobs, 1, "stale rendered PR must enqueue once as Rails does");
    }
    // An uncached version of the message renders even if the durable queue cannot accept a fetch.

    fresh.app.db.write(|tx|{tx.conn().execute_batch("DELETE FROM background_jobs; UPDATE github_pull_requests SET fetch_requested_at=NULL; UPDATE messages SET updated_at='2026-01-01 12:01:00' WHERE id=818; CREATE TRIGGER reject_render_refresh BEFORE INSERT ON background_jobs WHEN NEW.job_class='Github::FetchPullRequestJob' BEGIN SELECT RAISE(ABORT,'queue unavailable'); END;")?;crate::integrations::github::tests::enqueue_retention_job(tx)}).await.unwrap();
    let response = fresh
        .router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/rooms/815")
                .header("Host", "example.org")
                .header("Cookie", &fresh.cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let html = String::from_utf8(
        axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(html.contains("Secret title"));
    fresh
        .app
        .db
        .read(|conn| {
            let claim: Option<campfire_db::Timestamp> = conn.query_row(
                "SELECT fetch_requested_at FROM github_pull_requests WHERE id=816",
                [],
                |r| r.get(0),
            )?;
            assert!(claim.is_none());
            assert_eq!(
                conn.query_row(
                    "SELECT COUNT(*) FROM background_jobs WHERE job_class GLOB 'Github::*'",
                    [],
                    |r| r.get::<_, i64>(0)
                )?,
                0
            );
            Ok(())
        })
        .await
        .unwrap();
    assert!(fresh.server.received().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn review_refreshes_use_real_message_broadcast_and_refresh_callers() {
    let fresh = Fresh::new(&json!({"private":false})).await;
    fresh.app.db.write(|tx|{tx.conn().execute_batch("DELETE FROM background_jobs; UPDATE github_pull_requests SET fetched_at=NULL,fetch_requested_at=NULL")?;Ok(())}).await.unwrap();
    let requests = futures_util::future::join_all((0..12).map(|i| {
        let f = &fresh;
        async move {
            let path = if i % 2 == 0 {
                "/rooms/815/messages/818"
            } else {
                "/rooms/815/refresh"
            };
            let accept = if i % 2 == 0 {
                "text/html"
            } else {
                "text/vnd.turbo-stream.html"
            };
            let response = f
                .router
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(path)
                        .header("Host", "example.org")
                        .header("Accept", accept)
                        .header("Cookie", &f.cookie)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            if i % 2 == 0 {
                assert_eq!(response.status().as_u16(), 200, "{path}");
                let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
                    .await
                    .unwrap();
                assert!(String::from_utf8_lossy(&bytes).contains("Secret title"), "{path}");
            } else {
                assert_eq!(response.status().as_u16(), 302, "{path}");
                assert!(response.headers()["location"].to_str().unwrap().ends_with("/rooms/815"));
            }
        }
    }));
    requests.await;
    fresh.app.db.read(|conn|{assert_eq!(conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Github::FetchPullRequestJob'",[],|r|r.get::<_,i64>(0))?,1);Ok(())}).await.unwrap();
    // Rails accepts bodyless legacy edits and ignores embeds_suppressed here.
    // Keep the original case, add unchanged rich-text root/thread bodies, and
    // require the render itself to collect refresh intent in every case.
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/github-edit-refresh.json"
    ))
    .unwrap();
    for row in oracle["rows"].as_array().unwrap() {
        let id = row["id"].as_i64().unwrap();
        let body = row["body_before"].as_str().map(str::to_owned);
        fresh.app.db.write(move|tx| {
            if id==819 {
                tx.conn().execute("INSERT INTO messages (id,room_id,thread_id,creator_id,client_message_id,created_at,updated_at) VALUES (819,815,817,811,'card-thread',?,?)",params![tx.now(),tx.now()])?;
                tx.conn().execute("INSERT INTO github_pull_request_references (github_pull_request_id,message_id,created_at,updated_at) VALUES (816,819,?,?)",params![tx.now(),tx.now()])?;
            }
            if let Some(body)=body {campfire_db::RichTextRecord::create(tx,"Message",id,"body",&body)?;}
            tx.conn().execute_batch("DELETE FROM background_jobs; UPDATE github_pull_requests SET fetched_at=NULL,fetch_requested_at=NULL")?;
            Ok(())
        }).await.unwrap();
        let (status, headers, body) = super::test_support::request(
            &fresh,
            "PATCH",
            row["path"].as_str().unwrap(),
            json!({"message":{"embeds_suppressed":true}}),
            json!({}),
        )
        .await;
        assert_eq!(
            status,
            row["status"].as_u64().unwrap() as u16,
            "{}",
            row["name"]
        );
        assert_eq!(body, row["body"].as_str().unwrap(), "{}", row["name"]);
        for (header, key) in [("location", "location"), ("content-type", "content_type")] {
            assert_eq!(
                headers.get(header).map(|v| v.to_str().unwrap()),
                row[key].as_str(),
                "{}/{header}",
                row["name"]
            );
        }
        let row = row.clone();
        fresh.app.db.read(move|conn| {
            let message=campfire_db::Message::find(conn,id)?;
            assert_eq!(message.body_html(conn)?.as_deref(),row["saved_body"].as_str(),"{}/body",row["name"]);
            assert_eq!(message.embeds_suppressed,row["suppressed"].as_bool().unwrap());
            assert_eq!(message.edited_at.map(|t|campfire_views::messages::support::json_time(t.jiff())),row["edited_at"].as_str().map(str::to_owned));
            let claim:Option<campfire_db::Timestamp>=conn.query_row("SELECT fetch_requested_at FROM github_pull_requests WHERE id=816",[],|r|r.get(0))?;
            assert_eq!(claim.is_some(),row["claimed"].as_bool().unwrap(),"{}/claim",row["name"]);
            assert_eq!(conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Github::FetchPullRequestJob'",[],|r|r.get::<_,i64>(0))?,row["fetch_jobs"].as_i64().unwrap(),"{}/jobs",row["name"]);
            Ok(())
        }).await.unwrap();
    }
    // Render refresh failure remains best effort; its durable job and claim roll back.
    fresh.app.db.write(|tx|{tx.conn().execute_batch("DELETE FROM background_jobs; UPDATE github_pull_requests SET fetch_requested_at=NULL; CREATE TRIGGER reject_edit_refresh BEFORE INSERT ON background_jobs WHEN NEW.job_class='Github::FetchPullRequestJob' BEGIN SELECT RAISE(ABORT,'queue unavailable'); END;")?;Ok(())}).await.unwrap();
    for path in [
        "/rooms/815/messages/818",
        "/rooms/815/threads/817/messages/819",
    ] {
        let (status, _, _) = super::test_support::request(
            &fresh,
            "PATCH",
            path,
            json!({"message":{"embeds_suppressed":true}}),
            json!({}),
        )
        .await;
        assert_eq!(status, 302, "{path}");
        fresh.app.db.read(|conn|{let claim:Option<campfire_db::Timestamp>=conn.query_row("SELECT fetch_requested_at FROM github_pull_requests WHERE id=816",[],|r|r.get(0))?;assert!(claim.is_none());assert_eq!(conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Github::FetchPullRequestJob'",[],|r|r.get::<_,i64>(0))?,0);Ok(())}).await.unwrap();
    }
    assert!(fresh.server.received().is_empty());
}

#[tokio::test]
async fn deleting_a_reply_source_retains_idle_pr_fetches_for_its_tombstones() {
    let fresh = Fresh::new(&json!({"private": false})).await;
    assert!(!fresh.app.cable.sync_wanted());
    fresh.app.db.write(|tx| {
        let now = tx.now();
        tx.conn().execute("INSERT INTO github_pull_requests (id,owner,repo,number,title,state,private,created_at,updated_at) VALUES (826,'rails','rails',13,'Second card','open',0,?,?)", params![now, now])?;
        for (message, pr) in [(819, 826), (829, 816)] {
            tx.conn().execute("INSERT INTO messages (id,room_id,creator_id,client_message_id,reply_to_message_id,created_at,updated_at) VALUES (?,815,811,?,818,?,?)", params![message, format!("quoted-card-{message}"), now, now])?;
            tx.conn().execute("INSERT INTO github_pull_request_references (github_pull_request_id,message_id,created_at,updated_at) VALUES (?,?,?,?)", params![pr, message, now, now])?;
        }
        tx.conn().execute_batch("DELETE FROM background_jobs; UPDATE github_pull_requests SET fetched_at=NULL,fetch_requested_at=NULL")?;
        Ok(())
    }).await.unwrap();
    let (status, _, body) = super::test_support::request(
        &fresh, "DELETE", "/rooms/815/messages/818", json!({}), json!({}),
    ).await;
    assert_eq!(status, 204, "{body}");
    fresh.app.db.read(|conn| {
        for id in [819, 829] {
            let reply = campfire_db::Message::find(conn, id)?;
            assert_eq!(reply.reply_to_message_id, None);
            assert!(reply.reply_target_deleted_at.is_some());
        }
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM github_pull_requests WHERE fetch_requested_at IS NOT NULL", [], |row| row.get::<_, i64>(0))?, 2);
        let jobs = conn.prepare("SELECT json_extract(arguments,'$.pull_request_id') FROM background_jobs WHERE job_class='Github::FetchPullRequestJob' ORDER BY id")?.query_map([], |row| row.get::<_, i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        assert_eq!(jobs, vec![816, 826]);
        Ok(())
    }).await.unwrap();
    assert!(fresh.server.received().is_empty());
}
