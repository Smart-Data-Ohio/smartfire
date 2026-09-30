use crate::{
    app::{self, App},
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
struct Fresh {
    app: App,
    router: axum::Router,
    _dir: tempfile::TempDir,
    server: FakeServer,
    cookie: String,
}
fn cases() -> Value {
    serde_json::from_str(include_str!("../../../../../vectors/github_card_http.json")).unwrap()
}
impl Fresh {
    async fn new(case: &Value) -> Self {
        let status = case["access"].as_u64().unwrap_or(200) as u16;
        let (server, network) = fake(vec![
            Route::new("GET", "api.github.com", "/repos/rails/rails", status).body("{}"),
        ])
        .await;
        let dir = tempfile::tempdir_in(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.scratch/ws15g"),
        )
        .unwrap();
        let secret: Value =
            serde_json::from_str(include_str!("../../../../../vectors/github.json")).unwrap();
        let config = Config::from_lookup(|key| match key {
            "SECRET_KEY_BASE" => secret["secret_key_base"].as_str().map(str::to_owned),
            "CAMPFIRE_STORAGE_PATH" => Some(dir.path().to_string_lossy().into_owned()),
            "DISABLE_SSL" => Some("1".into()),
            _ => None,
        })
        .unwrap();
        let booted = app::boot_with_github_network(
            config,
            std::sync::Arc::new(campfire_kit::FrozenClock::new(
                "2026-01-01T12:00:00Z".parse().unwrap(),
            )),
            ReadClient::new(None),
            network,
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
   let now=tx.now();tx.conn().execute("INSERT INTO users (id,name,role,created_at,updated_at) VALUES (811,'Oracle',1,?,?)",params![now,now])?;
   for id in [815,825] {tx.conn().execute("INSERT INTO rooms (id,type,creator_id,name,created_at,updated_at) VALUES (?,'Rooms::Closed',811,'Cards',?,?)",params![id,now,now])?;}
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
