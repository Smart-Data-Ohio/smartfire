//! Rust-only browser ports of the pinned Rails Drive and sudo system tests:
//! `drive_attachments_test.rb`, `drive_link_previews_test.rb` and `sudo_mode_test.rb`
//! (`parity/system/drive-sudo.test.mjs`) and `drive_share_test.rb`
//! (`parity/system/drive-share.test.mjs`). Playwright drives the real router, assets,
//! forms, Turbo and Cable in the pinned image; each declaration gets its own app on the
//! Rails fixtures. Google's server-side API is an in-process fake that answers only the
//! WebMock stubs a declaration registers. The `/__drive_browser__/` control routes stand
//! in for the Ruby bodies' direct model calls and exist only in this test module.
//! Run with `parity/system/drive`.
use super::presenters::test_support::{SEED_NOW, TestApp, test_session_router};
use crate::integrations::{google, net};
use axum::{
    Json,
    extract::Query,
    routing::{get, post},
};
use campfire_db::models::google_account::{ConnectionGrant, GoogleAccount};
use campfire_db::{
    ChannelThread, NewChannelThread, NewUser, PasswordDigest, Room, ThreadMembership, User,
};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

/// One `stub_request(method, url).with(query: hash_including(query))`.
struct Stub {
    method: String,
    host: String,
    path: String,
    query: Vec<(String, String)>,
    status: u16,
    body: Vec<u8>,
}

/// WebMock in process: registered stubs answer, the latest first; anything else is
/// the original boundary's "unregistered Google exchange" and is recorded.
#[derive(Default)]
struct FakeGoogle {
    stubs: Mutex<Vec<Stub>>,
    requests: Mutex<Vec<Value>>,
}

impl FakeGoogle {
    fn answer(&self, host: &str, method: &str, target: &str) -> (u16, Vec<u8>, bool) {
        let url = url::Url::parse(&format!("https://{host}{target}")).unwrap();
        let query: Vec<(String, String)> = url.query_pairs().into_owned().collect();
        let stubs = self.stubs.lock().unwrap();
        let stub = stubs.iter().rev().find(|stub| {
            stub.method == method
                && stub.host == host
                && stub.path == url.path()
                && stub.query.iter().all(|pair| query.contains(pair))
        });
        match stub {
            Some(stub) => (stub.status, stub.body.clone(), true),
            None => (599, b"unregistered Google exchange".to_vec(), false),
        }
    }
}

impl google::client::Client for FakeGoogle {
    fn request<'a>(
        &'a self,
        host: &'a str,
        method: hyper::Method,
        target: &'a str,
        _headers: Vec<(String, String)>,
        _body: Vec<u8>,
    ) -> net::BoxFuture<'a, Result<(u16, Vec<u8>), google::client::Unavailable>> {
        Box::pin(async move {
            let (status, body, stubbed) = self.answer(host, method.as_str(), target);
            self.requests.lock().unwrap().push(json!({
                "host": host, "method": method.as_str(), "target": target, "stubbed": stubbed,
            }));
            Ok((status, body))
        })
    }
}

struct Server(tokio::task::JoinHandle<()>);
impl Drop for Server {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// `docker run --rm` leaves the container running when the CLI is killed.
struct Container(String);
impl Drop for Container {
    fn drop(&mut self) {
        let _ = std::process::Command::new("docker")
            .args(["rm", "-f", &self.0])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
}

/// `drive_share_test.rb` (WS14g-232..267) sets the Picker configuration and expects
/// the browser alone to talk to Google.
fn share_case(key: &str) -> bool {
    (232..=267).contains(&key["WS14g-".len()..].parse::<u16>().unwrap())
}

pub(super) async fn browser(key: &str) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let share = share_case(key);
    let mut environment = vec![
        ("GOOGLE_CLIENT_ID", "test-client-id"),
        ("GOOGLE_CLIENT_SECRET", "test-client-secret"),
    ];
    if share {
        environment.extend([
            ("GOOGLE_PICKER_API_KEY", "test-picker-key"),
            ("GOOGLE_CLOUD_PROJECT_NUMBER", "123456789012"),
        ]);
    }
    // The Rails declarations run inside travel_to Time.utc(2026, 3, 2, 16).
    let clock = Arc::new(campfire_kit::FrozenClock::new(SEED_NOW.parse().unwrap()));
    let app = TestApp::boot_with_network_clock_and_env(net::Network::system(), clock, &environment)
        .await
        .expect("drive browser requires the default seed")
        .without_job_runner()
        .await;
    load_rails_fixtures(&app).await;
    let fake = Arc::new(FakeGoogle::default());
    app.booted.app.google.install_api(google::api::Api::new(
        app.booted.app.config.google_client.clone(),
        fake.clone(),
    ));
    let router = app
        .booted
        .router
        .clone()
        .merge(test_session_router(&app.booted.app))
        .merge(control_routes(&app.booted.app, fake.clone()));
    let listener = crate::test_support::bind_listener().await;
    let target = format!("http://{}", listener.local_addr().unwrap());
    let _server = Server(tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    }));

    let cache = std::env::var_os("DRIVE_BROWSER_SCRATCH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("drive-browser"));
    std::fs::create_dir_all(&cache).unwrap();
    let scratch = tempfile::Builder::new()
        .prefix("browser-")
        .tempdir_in(cache)
        .unwrap();
    let socket = scratch.path().join("upstream.sock");
    let mut forward = tokio::process::Command::new("node")
        .arg(root.join("parity/capture/forward.ts"))
        .arg(&socket)
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    crate::test_support::eventually("drive browser loopback forwarder", || async {
        socket.exists()
    })
    .await;
    let image = std::env::var("WS13_PLAYWRIGHT_IMAGE")
        .expect("run parity/system/drive to build the pinned browser image");
    let container = Container(format!(
        "drive-browser-{}-{}",
        std::process::id(),
        key.to_lowercase()
    ));
    let file = if share {
        "drive-share.test.mjs"
    } else {
        "drive-sudo.test.mjs"
    };
    let output = tokio::process::Command::new("docker")
        .args(["run", "--rm", "--init", "--name", &container.0])
        .args(["--network", "none", "--ipc", "host", "--cpus", "1"])
        .arg("--user")
        .arg(format!("{}:{}", unsafe { libc::getuid() }, unsafe {
            libc::getgid()
        }))
        .arg("--volume")
        .arg(format!("{}:{}:ro", root.display(), root.display()))
        .arg("--volume")
        .arg(format!(
            "{}:{}",
            scratch.path().display(),
            scratch.path().display()
        ))
        .arg("--env")
        .arg(format!("HOME={}", scratch.path().display()))
        .arg("--env")
        .arg(format!("TMPDIR={}", scratch.path().display()))
        .arg("--env")
        .arg(format!("PARITY_UPSTREAM_SOCKET={}", socket.display()))
        .arg("--env")
        .arg(format!("DRIVE_BROWSER_TARGET={target}"))
        .arg("--env")
        .arg(format!(
            "DRIVE_BROWSER_MUTATION={}",
            std::env::var("DRIVE_BROWSER_MUTATION").unwrap_or_default()
        ))
        .arg(image)
        .args(["node", "--test-reporter=tap"])
        .arg(format!("--test-name-pattern=^{key} "))
        .arg(root.join("parity/system").join(file))
        .kill_on_drop(true)
        .output()
        .await
        .unwrap();
    forward.kill().await.unwrap();
    forward.wait().await.unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let requests = fake.requests.lock().unwrap().clone();
    println!(
        "{stdout}\nDRIVE_BROWSER_GOOGLE {key} {}",
        Value::from(requests.clone())
    );
    assert!(
        output.status.success(),
        "{key}: browser declaration failed\n{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // node:test counts only the selected declaration; the receipt is its last line.
    assert!(
        stdout.contains("# tests 1\n") && stdout.contains("# pass 1\n"),
        "{key}: exactly one declaration ran"
    );
    assert!(
        stdout.contains(&format!("DRIVE_BROWSER_RECEIPT {key}\n")),
        "{key}: declaration completed"
    );
    if share {
        assert_eq!(
            requests,
            Vec::<Value>::new(),
            "{key}: the share flow never calls Google from the server"
        );
    }
}

/// Each Rails system test starts from `fixtures :all`, never the parity seed's extras.
async fn load_rails_fixtures(app: &TestApp) {
    app.db().write(|tx| {
        tx.conn().execute_batch("PRAGMA defer_foreign_keys=ON")?;
        let tables = tx.conn().prepare("SELECT name FROM pragma_table_list WHERE schema='main' AND type='table' AND name NOT LIKE 'sqlite_%' AND name NOT IN ('schema_migrations','ar_internal_metadata')")?
            .query_map([], |row| row.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        for table in tables {
            tx.conn().execute(&format!("DELETE FROM \"{}\"", table.replace('"', "\"\"")), [])?;
        }
        campfire_db::fixtures::load(tx.conn(), &campfire_db::fixtures::reference_dir(), &campfire_db::fixtures::Options {
            now: tx.now(), bcrypt_cost: 4,
        })
    }).await.unwrap();
}

/// `String#parameterize` for the fixture names connect_google! sees.
fn parameterize(name: &str) -> String {
    name.to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn id(input: &Value, name: &str) -> i64 {
    input[name].as_i64().unwrap_or_else(|| panic!("{name} id"))
}

/// The Ruby bodies' model calls, through the same domain functions the app uses.
fn control_routes(app: &crate::app::App, fake: Arc<FakeGoogle>) -> axum::Router {
    let (accounts, threads, messages, revocations, emails, users) = (
        app.clone(),
        app.clone(),
        app.clone(),
        app.clone(),
        app.clone(),
        app.clone(),
    );
    let requests = fake.clone();
    axum::Router::new()
        // stub_request(:method, url).with(query: hash_including(query)).to_return(status:, body:)
        .route("/__drive_browser__/google/stubs", post(move |Json(input): Json<Value>| {
            let fake = fake.clone();
            async move {
                let url = url::Url::parse(input["url"].as_str().unwrap()).unwrap();
                let query = input["query"].as_object().into_iter().flatten()
                    .map(|(name, value)| (name.clone(), value.as_str().unwrap().to_owned())).collect();
                fake.stubs.lock().unwrap().push(Stub {
                    method: input["method"].as_str().unwrap().to_uppercase(),
                    host: url.host_str().unwrap().to_owned(),
                    path: url.path().to_owned(),
                    query,
                    status: input["status"].as_u64().unwrap().try_into().unwrap(),
                    body: serde_json::to_vec(&input["body"]).unwrap(),
                });
                Json(json!({}))
            }
        }))
        .route("/__drive_browser__/google/requests", get(move || {
            let requests = requests.requests.lock().unwrap().clone();
            async move { Json(Value::from(requests)) }
        }))
        // connect_google!(user, scopes:): GoogleAccount.create! with encrypted tokens.
        .route("/__drive_browser__/google/accounts", post(move |Json(input): Json<Value>| {
            let app = accounts.clone();
            async move {
                let crypto = app.ar_encryption.clone();
                app.db.write(move |tx| {
                    let user = User::find(tx.conn(), id(&input, "user"))?;
                    GoogleAccount::create(tx, &crypto, ConnectionGrant {
                        user_id: user.id,
                        email: format!("{}@gmail.test", parameterize(&user.name)),
                        access_token: Some(format!("access-token-{}", user.id)),
                        refresh_token: Some(format!("refresh-token-{}", user.id)),
                        access_token_expires_at: Some(tx.now().since(jiff::SignedDuration::from_hours(1))),
                        scopes: input["scopes"].as_str().map(str::to_owned),
                    })?;
                    Ok(())
                }).await.unwrap();
                Json(json!({}))
            }
        }))
        // ChannelThread.create!(room:, creator:, name:); ThreadMembership.join!(thread, creator)
        .route("/__drive_browser__/threads", post(move |Json(input): Json<Value>| {
            let app = threads.clone();
            async move {
                let thread = app.db.write(move |tx| {
                    let creator = id(&input, "creator");
                    let thread = ChannelThread::create(tx, NewChannelThread {
                        room_id: id(&input, "room"),
                        creator_id: creator,
                        name: input["name"].as_str().map(str::to_owned),
                        ..Default::default()
                    })?;
                    ThreadMembership::join(tx, thread.id, creator)?;
                    Ok(thread.id)
                }).await.unwrap();
                Json(json!({"id": thread}))
            }
        }))
        // Message.last (or thread.messages.order(:id).last) and its drive_attachments.
        .route("/__drive_browser__/messages/last", get(move |Query(query): Query<std::collections::HashMap<String, i64>>| {
            let app = messages.clone();
            async move {
                Json(app.db.read(move |conn| {
                    let (id, client_message_id): (i64, String) = match query.get("thread") {
                        Some(thread) => conn.query_row("SELECT id,client_message_id FROM messages WHERE thread_id=? ORDER BY id DESC LIMIT 1", [thread], |r| Ok((r.get(0)?, r.get(1)?)))?,
                        None => conn.query_row("SELECT id,client_message_id FROM messages ORDER BY id DESC LIMIT 1", [], |r| Ok((r.get(0)?, r.get(1)?)))?,
                    };
                    let files = conn.prepare("SELECT file_id FROM drive_attachments WHERE message_id=? ORDER BY id")?
                        .query_map([id], |r| r.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
                    Ok(json!({"id": id, "client_message_id": client_message_id, "drive_file_ids": files}))
                }).await.unwrap())
            }
        }))
        // rooms(:room).memberships.revoke_from(users(:user))
        .route("/__drive_browser__/memberships/revoke", post(move |Json(input): Json<Value>| {
            let app = revocations.clone();
            async move {
                app.db.write(move |tx| {
                    Room::find(tx.conn(), id(&input, "room"))?.revoke_from(tx, &[id(&input, "user")])
                }).await.unwrap();
                Json(json!({}))
            }
        }))
        // users(:user).update_column(:email_address, email): no callbacks, no updated_at.
        .route("/__drive_browser__/users/email", post(move |Json(input): Json<Value>| {
            let app = emails.clone();
            async move {
                app.db.write(move |tx| {
                    tx.conn().execute("UPDATE users SET email_address=? WHERE id=?", rusqlite::params![input["email_address"].as_str().unwrap(), id(&input, "user")])?;
                    Ok(())
                }).await.unwrap();
                Json(json!({}))
            }
        }))
        // User.create!(name:, email_address:, password:).tap { rooms(:room).memberships.grant_to(_1) }
        .route("/__drive_browser__/users", post(move |Json(input): Json<Value>| {
            let app = users.clone();
            async move {
                let digest = PasswordDigest::create(input["password"].as_str().unwrap(), 4).unwrap();
                let user = app.db.write(move |tx| {
                    let user = User::create(tx, NewUser {
                        name: input["name"].as_str().unwrap().to_owned(),
                        email_address: input["email_address"].as_str().map(str::to_owned),
                        password_digest: Some(digest),
                        ..Default::default()
                    })?;
                    Room::find(tx.conn(), id(&input, "room"))?.grant_to(tx, &[user.id])?;
                    Ok(user.id)
                }).await.unwrap();
                Json(json!({"id": user}))
            }
        }))
}

/// Every in-scope pinned declaration has exactly one node declaration, under its Rails title,
/// and one ignored Rust test below, so an exact CI selection can't silently lose a case.
#[test]
fn every_pinned_declaration_has_one_browser_test() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let expected: Vec<String> = (224..=230)
        .chain(232..=267)
        .chain([272])
        .map(|number| format!("WS14g-{number}"))
        .collect();
    let manifest: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("reference-tools/users/original_browser/manifest.json"))
            .unwrap(),
    )
    .unwrap();
    let titles: std::collections::HashMap<&str, &str> = manifest["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|record| {
            (
                record["id"].as_str().unwrap(),
                record["title"].as_str().unwrap(),
            )
        })
        .collect();
    let mut declared = Vec::new();
    for file in ["drive-sudo.test.mjs", "drive-share.test.mjs"] {
        let source = std::fs::read_to_string(root.join("parity/system").join(file)).unwrap();
        for call in source.lines().filter_map(|line| line.strip_prefix("test(")) {
            let quote = &call[..1];
            let (key, title) = call[1..]
                .split(quote)
                .next()
                .unwrap()
                .split_once(' ')
                .unwrap();
            assert_eq!(
                titles.get(key),
                Some(&title),
                "{file}: {key} keeps its Rails title"
            );
            declared.push(key.to_owned());
        }
    }
    declared.sort();
    assert_eq!(declared, expected);
    let functions: Vec<String> = include_str!("drive_browser_tests.rs")
        .lines()
        .filter_map(|line| line.strip_prefix("async fn drive_browser_ws14g_"))
        .map(|rest| format!("WS14g-{}", &rest[..3]))
        .collect();
    assert_eq!(functions, expected);
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_224() {
    browser("WS14g-224").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_225() {
    browser("WS14g-225").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_226() {
    browser("WS14g-226").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_227() {
    browser("WS14g-227").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_228() {
    browser("WS14g-228").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_229() {
    browser("WS14g-229").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_230() {
    browser("WS14g-230").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_232() {
    browser("WS14g-232").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_233() {
    browser("WS14g-233").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_234() {
    browser("WS14g-234").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_235() {
    browser("WS14g-235").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_236() {
    browser("WS14g-236").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_237() {
    browser("WS14g-237").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_238() {
    browser("WS14g-238").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_239() {
    browser("WS14g-239").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_240() {
    browser("WS14g-240").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_241() {
    browser("WS14g-241").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_242() {
    browser("WS14g-242").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_243() {
    browser("WS14g-243").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_244() {
    browser("WS14g-244").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_245() {
    browser("WS14g-245").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_246() {
    browser("WS14g-246").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_247() {
    browser("WS14g-247").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_248() {
    browser("WS14g-248").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_249() {
    browser("WS14g-249").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_250() {
    browser("WS14g-250").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_251() {
    browser("WS14g-251").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_252() {
    browser("WS14g-252").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_253() {
    browser("WS14g-253").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_254() {
    browser("WS14g-254").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_255() {
    browser("WS14g-255").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_256() {
    browser("WS14g-256").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_257() {
    browser("WS14g-257").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_258() {
    browser("WS14g-258").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_259() {
    browser("WS14g-259").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_260() {
    browser("WS14g-260").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_261() {
    browser("WS14g-261").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_262() {
    browser("WS14g-262").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_263() {
    browser("WS14g-263").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_264() {
    browser("WS14g-264").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_265() {
    browser("WS14g-265").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_266() {
    browser("WS14g-266").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_267() {
    browser("WS14g-267").await;
}

#[tokio::test]
#[ignore = "requires Node, Docker and the pinned Playwright image; run parity/system/drive"]
async fn drive_browser_ws14g_272() {
    browser("WS14g-272").await;
}
