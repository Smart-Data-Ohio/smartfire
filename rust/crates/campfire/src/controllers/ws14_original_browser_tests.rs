//! Exact pinned Rails system declarations execute in native Capybara/Selenium.
//! The real Rust router, assets, models, forms, Turbo and Cable produce the UI.
//! Ruby reads/writes the private fixture DB and stubs only external Google HTTP.
use super::presenters::test_support::{SEED_NOW, TestApp};
use crate::integrations::{google, net};
use std::sync::Arc;

struct OriginalGoogle(std::path::PathBuf);
impl google::client::Client for OriginalGoogle {
    fn request<'a>(
        &'a self,
        host: &'a str,
        method: hyper::Method,
        target: &'a str,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    ) -> net::BoxFuture<'a, Result<(u16, Vec<u8>), google::client::Unavailable>> {
        Box::pin(async move {
            use net::http::{self, Body, Endpoint, Request, Timeouts};
            let port = std::fs::read_to_string(self.0.join("google-port"))
                .expect("original WebMock boundary started")
                .parse::<u16>()
                .unwrap();
            let endpoint = Endpoint {
                https: false,
                host: "127.0.0.1".into(),
                port,
                pinned_ip: None,
            };
            let mut request = Request::net_http(
                hyper::Method::POST,
                "/".into(),
                Some("127.0.0.1".into()),
                vec![("Content-Type".into(), "application/json".into())],
            );
            request.body = serde_json::to_vec(&serde_json::json!({
                "host": host, "method": method.as_str(), "target": target,
                "headers": headers, "body": String::from_utf8(body).unwrap(),
            }))
            .unwrap();
            let response = http::exchange(
                &net::Network::system(),
                &endpoint,
                request,
                &Timeouts {
                    open: std::time::Duration::from_secs(10),
                    read: std::time::Duration::from_secs(10),
                    write: std::time::Duration::from_secs(10),
                },
            )
            .await?;
            let Body::Complete(bytes) = response.read_body(usize::MAX).await? else {
                unreachable!()
            };
            let response: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            Ok((
                response["status"].as_u64().unwrap().try_into().unwrap(),
                response["body"].as_str().unwrap().as_bytes().to_vec(),
            ))
        })
    }
}

struct Server(tokio::task::JoinHandle<()>);
impl Drop for Server {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub(super) async fn original(key: &str) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let clock = Arc::new(campfire_kit::FrozenClock::new(SEED_NOW.parse().unwrap()));
    let control = tempfile::Builder::new()
        .prefix("ws14-original-")
        .tempdir_in(root.join("target"))
        .unwrap();
    let (network, _github_server) = if key.starts_with("WS15g-") {
        let (network, server) =
            super::ws15_original_github_browser_tests::network(control.path().to_owned()).await;
        (network, Some(server))
    } else {
        (net::Network::system(), None)
    };
    let mut environment = vec![
        ("GOOGLE_CLIENT_ID", "test-client-id"),
        ("GOOGLE_CLIENT_SECRET", "test-client-secret"),
    ];
    if key.starts_with("WS14g-") && (232..=267).contains(&key[6..].parse::<u16>().unwrap()) {
        environment.extend([
            ("GOOGLE_PICKER_API_KEY", "test-picker-key"),
            ("GOOGLE_CLOUD_PROJECT_NUMBER", "123456789012"),
        ]);
    }
    let app = if key.starts_with("WS15g-") {
        TestApp::boot_with_github_network_clock_and_env(network, clock.clone(), &environment).await
    } else {
        TestApp::boot_with_network_clock_and_env(network, clock.clone(), &environment).await
    }
        .expect("original browser requires reference-built seed")
        .without_job_runner()
        .await;
    app.db().write(|tx| {
        tx.conn().execute_batch("PRAGMA defer_foreign_keys=ON")?;
        let tables = tx.conn().prepare("SELECT name FROM pragma_table_list WHERE schema='main' AND type='table' AND name NOT LIKE 'sqlite_%' AND name NOT IN ('schema_migrations','ar_internal_metadata')")?
            .query_map([], |row| row.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        for table in tables {
            tx.conn().execute(&format!("DELETE FROM \"{}\"", table.replace('"', "\"\"")), [])?;
        }
        campfire_db::fixtures::load(tx.conn(), &campfire_db::fixtures::reference_dir(), &campfire_db::fixtures::Options {
            now: tx.now(), bcrypt_cost: 4,
        })
    }).await.unwrap();
    app.booted.app.google.install_api(google::api::Api::new(
        app.booted.app.config.google_client.clone(),
        Arc::new(OriginalGoogle(control.path().to_owned())),
    ));
    let jobs_app = app.booted.app.clone();
    let dispatch_app = jobs_app.clone();
    let auth_app = jobs_app.clone();
    let routes = axum::Router::new()
        .route(
            "/test_session",
            axum::routing::get(move |axum::extract::Query(input): axum::extract::Query<std::collections::HashMap<String,String>>, headers: axum::http::HeaderMap| {
                let app = auth_app.clone();
                async move {
                    use campfire_kit::Crypto;
                    let email = input["email_address"].to_owned();
                    let password = input["password"].to_owned();
                    let candidate = app.db.read(move |conn| campfire_db::User::find_active_by_email_address(conn, &email)).await.unwrap();
                    let user = tokio::task::spawn_blocking(move || campfire_db::User::authenticated(candidate, &password)).await.unwrap().expect("original fast sign-in verifies fixture credentials");
                    let user_agent = headers[axum::http::header::USER_AGENT].to_str().unwrap().to_owned();
                    let session = app.db.write(move |tx| campfire_db::Session::start_with(tx, user.id, campfire_db::NewSession {
                        user_agent: Some(&user_agent), ip_address: Some("127.0.0.1"), device_id: None, two_factor_verified: true,
                    })).await.unwrap();
                    let value = campfire_kit::RailsCrypto::new(app.secrets.clone()).sign_cookie("session_token", &session.token, None);
                    axum::http::Response::builder().status(axum::http::StatusCode::SEE_OTHER)
                        .header(axum::http::header::LOCATION, "/")
                        .header(axum::http::header::SET_COOKIE, format!("session_token={}; Path=/; HttpOnly; SameSite=Lax", campfire_kit::cookies::escape(&value)))
                        .body(axum::body::Body::empty()).unwrap()
                }
            }),
        )
        .route(
            "/__ws14_browser__/clock",
            axum::routing::post(move |axum::Json(input): axum::Json<serde_json::Value>| {
                let clock = clock.clone();
                async move {
                    clock.set(input["time"].as_str().unwrap().parse().unwrap());
                    axum::Json(serde_json::json!({"changed": true}))
                }
            }),
        )
        .route(
            "/__ws14_browser__/meeting-dispatch",
            axum::routing::post(move || {
                let app = dispatch_app.clone();
                async move {
                    campfire_db::models::calendar_dispatch::dispatch_meetings(
                        &app.db,
                        campfire_db::Timestamp::from_jiff(app.clock.now()),
                    )
                    .await
                    .unwrap();
                    axum::Json(serde_json::json!({"dispatched": true}))
                }
            }),
        )
        .route(
            "/__ws14_browser__/meeting-jobs",
            axum::routing::post(move || {
                let app = jobs_app.clone();
                async move {
                    let held = app.db.write(|tx| {
                        let mut held = tx.conn().prepare("SELECT id,run_at FROM background_jobs WHERE status='ready' AND job_class!='Calendar::MeetingRefreshJob'")?
                            .query_map([], |r| Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?)))?
                            .collect::<Result<Vec<_>,_>>()?;
                        tx.conn().execute("UPDATE background_jobs SET run_at='2999-01-01 00:00:00' WHERE status='ready' AND job_class!='Calendar::MeetingRefreshJob'", [])?;
                        held.shrink_to_fit();
                        Ok(held)
                    }).await.unwrap();
                    let count: i64 = app.db.read(|c| {
                        c.query_row("SELECT COUNT(*) FROM background_jobs WHERE status='ready' AND job_class='Calendar::MeetingRefreshJob'",[],|r|r.get(0)).map_err(Into::into)
                    }).await.unwrap();
                    assert!(count > 0, "original perform_enqueued_jobs encounters the real queued refresh");
                    let mut registry = campfire_jobs::Registry::new();
                    google::meeting_refresh::register(&mut registry);
                    let runner = campfire_jobs::start(app.db.clone(), app.jobs.queue.clone(), registry, app.clone(),
                        campfire_jobs::RunnerConfig::new(vec![campfire_jobs::QueueConfig::new("default", 1)]));
                    crate::test_support::eventually("original registered meeting jobs", || {
                        let db = app.db.clone();
                        async move {db.read(|c| c.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Calendar::MeetingRefreshJob'",[],|r|r.get::<_,i64>(0)).map_err(Into::into)).await.unwrap()==0}
                    }).await;
                    runner.shutdown(std::time::Duration::from_secs(1)).await;
                    app.db.write(move |tx| {for (id,time) in held {tx.conn().execute("UPDATE background_jobs SET run_at=? WHERE id=?",rusqlite::params![time,id])?;} Ok(())}).await.unwrap();
                    axum::Json(serde_json::json!({"performed": count}))
                }
            }),
        );
    let listener = crate::test_support::bind_listener().await;
    let base = format!("http://{}", listener.local_addr().unwrap());
    let router = app.booted.router.clone().merge(routes);
    let _server = Server(tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    }));
    let output = tokio::process::Command::new("node")
        .arg(root.join("reference-tools/users/ws14_original_browser.mjs"))
        .arg(base)
        .arg(&app.booted.app.config.storage.database)
        .arg(key)
        .arg(control.path())
        .kill_on_drop(true)
        .output()
        .await
        .unwrap();
    println!("{}", String::from_utf8_lossy(&output.stdout));
    assert!(
        output.status.success(),
        "{key}: exact original browser declaration failed\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains(&format!("WS14_ORIGINAL_RECEIPT {key} ")),
        "{key}: original assertions must execute and emit a receipt"
    );
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14e_101() {
    original("WS14e-101").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_224() {
    original("WS14g-224").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_225() {
    original("WS14g-225").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_226() {
    original("WS14g-226").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_227() {
    original("WS14g-227").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_228() {
    original("WS14g-228").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_229() {
    original("WS14g-229").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_230() {
    original("WS14g-230").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_231() {
    original("WS14g-231").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_232() {
    original("WS14g-232").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_233() {
    original("WS14g-233").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_234() {
    original("WS14g-234").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_235() {
    original("WS14g-235").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_236() {
    original("WS14g-236").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_237() {
    original("WS14g-237").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_238() {
    original("WS14g-238").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_239() {
    original("WS14g-239").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_240() {
    original("WS14g-240").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_241() {
    original("WS14g-241").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_242() {
    original("WS14g-242").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_243() {
    original("WS14g-243").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_244() {
    original("WS14g-244").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_245() {
    original("WS14g-245").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_246() {
    original("WS14g-246").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_247() {
    original("WS14g-247").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_248() {
    original("WS14g-248").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_249() {
    original("WS14g-249").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_250() {
    original("WS14g-250").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_251() {
    original("WS14g-251").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_252() {
    original("WS14g-252").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_253() {
    original("WS14g-253").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_254() {
    original("WS14g-254").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_255() {
    original("WS14g-255").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_256() {
    original("WS14g-256").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_257() {
    original("WS14g-257").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_258() {
    original("WS14g-258").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_259() {
    original("WS14g-259").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_260() {
    original("WS14g-260").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_261() {
    original("WS14g-261").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_262() {
    original("WS14g-262").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_263() {
    original("WS14g-263").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_264() {
    original("WS14g-264").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_265() {
    original("WS14g-265").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_266() {
    original("WS14g-266").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_267() {
    original("WS14g-267").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_269() {
    original("WS14g-269").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_270() {
    original("WS14g-270").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_271() {
    original("WS14g-271").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_272() {
    original("WS14g-272").await;
}

#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws14g_273() {
    original("WS14g-273").await;
}
