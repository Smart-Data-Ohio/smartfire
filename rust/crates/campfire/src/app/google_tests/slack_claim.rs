//! WS16 owner handoff: WS14g callbacks through HTTP, then Slack opt-in and ownership.
use super::*;
use crate::{
    controllers::presenters::test_support::{Reply, with_fixed_render_secrets},
    integrations::{
        slack::{self, conversations, users, writer},
        test_support::{FakeServer, Route},
    },
};
use campfire_db::models::slack::{SlackConnection, SlackWorkspace};
use campfire_db::models::slack_import::{Kind, Mode, NewImport, SlackImport};
use campfire_kit::{Crypto, RailsCrypto};
use http_body_util::BodyExt;
use std::net::SocketAddr;

const INPUT: &str = include_str!("../../integrations/slack/fixtures/google_claim.json");
const ORACLE: &str = include_str!("../../../../../vectors/slack/google_claim_http.json");
const NOW: &str = "2026-03-02T16:00:00Z";
fn grant() -> String {
    ["fixture", "claim", "grant"].join("-")
}
fn routes() -> Vec<Route> {
    let input: Value = serde_json::from_str(INPUT).unwrap();
    let route = |path: &str, value: Value| {
        Route::new("GET", "slack.com", path, 200).body(value.to_string())
    };
    vec![
        Route::new("POST", "slack.com", "/api/oauth.v2.access", 200).body(json!({"ok":true,"team":{"id":"TCLAIM","name":"Claim fixture"},"authed_user":{"id":"UCLAIM","access_token":grant(),"scope":slack::oauth::USER_SCOPES.join(",")}}).to_string()),
        route("/api/users.list?limit=200", json!({"ok":true,"members":input["members"]})),
        route("/api/conversations.list?types=im%2Cmpim%2Cprivate_channel&exclude_archived=false&limit=200", json!({"ok":true,"channels":[input["conversation"]]})),
        route("/api/conversations.members?channel=DCLAIM&limit=1000", json!({"ok":true,"members":["UCLAIM","UPEER"]})),
        route("/api/conversations.history?channel=DCLAIM&limit=200", json!({"ok":true,"messages":input["history"],"has_more":false})),
        route("/api/conversations.replies?channel=DCLAIM&ts=1700000002.000001&limit=200", json!({"ok":true,"messages":input["replies"],"has_more":false})),
    ]
}
async fn app() -> (TestApp, Arc<Recorded>, FakeServer) {
    let (server, network) = slack::client::tests::fake(routes()).await;
    let auth: Value =
        serde_json::from_str(include_str!("../../../../../vectors/two_factor_views.json")).unwrap();
    let a = crate::test_support::with_auth_inputs(
        campfire_db::FixtureAuthInputs {
            totp_secret: "JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP".into(),
            backup_codes: auth["codes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_owned())
                .collect(),
        },
        TestApp::boot_with_network_clock_and_env(
            network,
            Arc::new(campfire_kit::FrozenClock::new(NOW.parse().unwrap())),
            &[
                ("GOOGLE_CLIENT_ID", "test-client-id"),
                ("GOOGLE_CLIENT_SECRET", "FAKE-google-client-secret"),
                ("GOOGLE_SIGN_IN_DOMAINS", "smartdata.net"),
            ],
        ),
    )
    .await
    .expect("pinned default seed required")
    .without_job_runner()
    .await;
    let r = Arc::new(Recorded {
        response: Mutex::new(Err(())),
        calls: Mutex::new(vec![]),
        certs: Mutex::new(None),
    });
    a.booted.app.google.install(SignIn::with_client(
        Config {
            client_id: "test-client-id".into(),
            client_secret: "FAKE-google-client-secret".into(),
            domains: vec!["smartdata.net".into()],
        },
        r.clone(),
    ));
    (a, r, server)
}
async fn arrange(a: &TestApp) -> i64 {
    let crypto = a.booted.app.ar_encryption.clone();
    let run_id = a
        .db()
        .write(move |tx| {
            tx.conn().execute("DELETE FROM google_identities", [])?;
            tx.conn().execute("DELETE FROM audit_logs", [])?;
            for table in [
                "users",
                "rooms",
                "messages",
                "channel_threads",
                "memberships",
                "thread_memberships",
                "slack_imports",
                "slack_import_records",
                "slack_workspaces",
                "slack_connections",
                "google_identities",
            ] {
                tx.conn()
                    .execute("DELETE FROM sqlite_sequence WHERE name=?", [table])?;
                tx.conn().execute(
                    "INSERT INTO sqlite_sequence(name,seq) VALUES(?,9000000000)",
                    [table],
                )?;
            }
            let w = SlackWorkspace::create(
                tx,
                &crypto,
                "fixture-client",
                "fixture-secret",
                Some(DAVID),
            )?;
            w.name_team(tx, &crypto, "TCLAIM", Some("Claim fixture"), None)?;
            let run = SlackImport::create(
                tx,
                NewImport {
                    workspace_id: w.id,
                    connection_id: None,
                    user_id: DAVID,
                    kind: Kind::Workspace,
                    mode: Mode::Import,
                    options: json!({}),
                },
            )?;
            tx.conn().execute(
                "UPDATE slack_imports SET status='completed' WHERE id=?",
                [run.id],
            )?;
            let input: Value = serde_json::from_str(INPUT).unwrap();
            users::map_page(
                tx,
                &run,
                &input["members"],
                false,
                &["smartdata.net".into()].into(),
            )?;
            Ok(run.id)
        })
        .await
        .unwrap();
    // Match Rails' commit boundary: active placeholders receive open rooms before
    // the conversation phase assigns memberships and mapping record IDs.
    a.db()
        .write(move |tx| {
            let run = SlackImport::find(tx.conn(), run_id)?.unwrap();
            let input: Value = serde_json::from_str(INPUT).unwrap();
            let ids = [json!("UCLAIM"), json!("UPEER")];
            let mut mapped = users::users_for(tx.conn(), &run, &ids)?;
            let room = conversations::resolve(
                tx,
                &run,
                &input["initial_conversation"],
                &ids,
                &mapped,
                false,
            )?
            .room
            .unwrap();
            let history = writer::history(
                tx,
                &run,
                &room,
                "CCLAIM",
                input["initial_history"].as_array().unwrap(),
                writer::Bounds::default(),
                &mut mapped,
            )?;
            writer::replies(
                tx,
                &run,
                &room,
                writer::Replies {
                    conversation: "CCLAIM",
                    parent_ts: "1700000000.000001",
                    parent_id: history.parents[0]["message_id"].as_i64().unwrap(),
                    messages: input["initial_replies"].as_array().unwrap(),
                    bounds: writer::Bounds::default(),
                    state: &mut json!({}),
                },
                &mut mapped,
            )?;
            Ok(mapped["UCLAIM"].id)
        })
        .await
        .unwrap()
}
async fn snapshot(a: &TestApp, id: i64, columns: Value) -> Value {
    let crypto = a.booted.app.ar_encryption.clone();
    a.db().read(move |db| {
        let mut state = json!({});
        for (table, columns) in columns.as_object().unwrap() {
            let predicate = match table.as_str() {
                "users" => format!("WHERE id={id}"),
                "memberships" => format!("WHERE user_id={id} OR room_id IN (SELECT record_id FROM slack_import_records WHERE record_type='Room')"),
                "messages" | "channel_threads" => "WHERE id>9000000000".into(),
                "thread_memberships" => "WHERE thread_id>9000000000".into(),
                _ => String::new(),
            };
            let columns: Vec<_> = columns.as_array().unwrap().iter().map(|c| c.as_str().unwrap()).collect();
            let select = columns.iter().map(|c|format!("\"{c}\"")).collect::<Vec<_>>().join(",");
            let order = if table == "google_identities" { "user_id" } else { "id" };
            let sql = format!("SELECT {select} FROM {table} {predicate} ORDER BY {order}");
            let rows = db.prepare(&sql)?.query_map([], |row| {
                let mut value = json!({});
                for (i, c) in columns.iter().enumerate() {
                    use rusqlite::types::Value as Sql;
                    value[*c] = match row.get::<_,Sql>(i)? {
                        Sql::Null => Value::Null,
                        Sql::Integer(n) => json!(n),
                        Sql::Real(n) => json!(n),
                        Sql::Text(s) if ["options","details"].contains(c) => serde_json::from_str(&s).unwrap(),
                        Sql::Text(s) => json!(s),
                        Sql::Blob(_) => panic!("unexpected blob"),
                    };
                }
                Ok(value)
            })?.collect::<rusqlite::Result<Vec<_>>>()?;
            state[table] = json!(rows);
        }
        state["sessions"] = json!(db.prepare("SELECT user_id,two_factor_verified_at,last_active_at FROM sessions WHERE user_id=? ORDER BY id")?.query_map([id], |r| Ok(json!({"user_id":r.get::<_,i64>(0)?,"two_factor_verified_at":r.get::<_,Option<String>>(1)?,"last_active_at":r.get::<_,String>(2)?})))?.collect::<rusqlite::Result<Vec<_>>>()?);
        state["google_accounts"] = json!(db.query_row("SELECT COUNT(*) FROM google_accounts WHERE user_id=?", [id], |r| r.get::<_,i64>(0))?);
        let connection = SlackConnection::for_user(db,id)?;
        state["connection"] = match &connection {
            Some(c) => json!({"id":c.id,"slack_workspace_id":c.slack_workspace_id,"user_id":c.user_id,"slack_user_id":c.slack_user_id,"scopes":c.scopes,"disconnected_reason":c.disconnected_reason}),
            None => Value::Null,
        };
        let encrypted = connection.as_ref().map(|c| db.query_row("SELECT access_token FROM slack_connections WHERE id=?", [c.id], |r| r.get::<_,String>(0)).map(|token|token!=grant())).transpose()?;
        let token_matches = connection.as_ref().map(|c| c.access_token(&crypto).map(|token|token.as_deref()==Some(&grant()))).transpose()?;
        let user = campfire_db::User::find(db,id)?;
        state["credential_checks"] = json!({"password_absent":user.password_digest.is_none(),"token_matches":token_matches,"encrypted":encrypted,"enrolled":user.two_factor_enabled(db)?,"backup_count":db.query_row("SELECT COUNT(*) FROM two_factor_backup_codes b JOIN two_factor_credentials c ON c.id=b.two_factor_credential_id WHERE c.user_id=?", [id], |r|r.get::<_,i64>(0))?});
        Ok(state)
    }).await.unwrap()
}
// Actual TCP HTTP, with one browser cookie jar throughout. This is not Router::oneshot.
async fn send(
    b: &mut Browser<'_>,
    address: SocketAddr,
    method: Method,
    path: &str,
    params: &[(&str, &str)],
    bad_csrf: bool,
) -> Reply {
    let stream = tokio::net::TcpStream::connect(address).await.unwrap();
    let (mut sender, connection) =
        hyper::client::conn::http1::handshake(hyper_util::rt::TokioIo::new(stream))
            .await
            .unwrap();
    let connection = tokio::spawn(connection);
    let mut request = axum::http::Request::builder()
        .method(method.clone())
        .uri(path)
        .header("Host", "campfire.test")
        .header("Accept", "text/html")
        .header("Cookie", b.cookie_header())
        .header("Connection", "close");
    let body = if method == Method::GET {
        String::new()
    } else {
        let csrf = if bad_csrf {
            "invalid".into()
        } else {
            b.real_authenticity_token()
                .expect("real CSRF cookie")
                .masked(None)
        };
        request = request
            .header("X-CSRF-Token", csrf)
            .header("Content-Type", "application/x-www-form-urlencoded");
        url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs(params.iter().copied())
            .finish()
    };
    let response = sender
        .send_request(
            request
                .body(http_body_util::Full::new(bytes::Bytes::from(body)))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .to_vec();
    for cookie in headers.get_all("set-cookie") {
        b.absorb_cookie_header(cookie.to_str().unwrap().split(';').next().unwrap());
    }
    connection.await.unwrap().unwrap();
    Reply {
        status,
        headers,
        body,
    }
}
async fn drive(a: &TestApp, id: i64) {
    for _ in 0..100 {
        let db = a.db().clone();
        let network = a.booted.app.slack_network.clone();
        slack::jobs::perform_import(
            a.db().clone(),
            a.booted.app.ar_encryption.clone(),
            id,
            move |run, lease, token| async move {
                slack::runner::Runner::new(
                    slack::store::SqlStore {
                        db,
                        lease: Some(lease),
                        allowed_domains: ["smartdata.net".into()].into(),
                    },
                    slack::client::Client::with_network(token, None, false, network),
                    run,
                )
                .with_budget(std::time::Duration::ZERO)
                .step()
                .await
            },
        )
        .await
        .unwrap();
        let run = a
            .db()
            .read(move |db| SlackImport::find(db, id))
            .await
            .unwrap()
            .unwrap();
        if run.status == "completed" {
            return;
        }
        assert!(
            matches!(run.status.as_str(), "queued" | "running"),
            "{}",
            run.error.unwrap_or_default()
        );
    }
    panic!("personal fixture did not finish");
}
fn cookie_session(a: &TestApp, b: &Browser<'_>) -> Value {
    let raw = b
        .cookie_header()
        .split(';')
        .find_map(|p| {
            p.trim()
                .strip_prefix("_campfire_session=")
                .map(str::to_owned)
        })
        .unwrap();
    RailsCrypto::new(a.booted.app.secrets.clone())
        .decrypt_cookie(
            "_campfire_session",
            &rails_compat::cookies::unescape(&raw),
            a.booted.app.clock.now(),
        )
        .unwrap()
}
#[tokio::test]
async fn google_callback_slack_opt_in_claim_matches_rails_over_real_http() {
    let expected: Value = serde_json::from_str(ORACLE).unwrap();
    let (a, r, slack_server) = app().await;
    let claimant = arrange(&a).await;
    assert_eq!(
        snapshot(&a, claimant, expected["columns"].clone()).await,
        expected["initial"],
        "before first sign-in"
    );
    let listener = crate::test_support::bind_listener().await;
    let address = listener.local_addr().unwrap();
    let router = a.booted.router.clone().layer(axum::middleware::from_fn(
        |request: axum::extract::Request, next: axum::middleware::Next| async move {
            let mut entropy = [7; 64];
            if request.uri().path() == "/session/google" {
                entropy[..16].fill(1);
                entropy[16..32].fill(2);
                entropy[32..].fill(3);
            }
            if request.uri().path() == "/sudo/google" {
                entropy[..16].fill(4);
                entropy[16..32].fill(5);
                entropy[32..].fill(6);
            }
            crate::test_support::with_oauth_entropy(
                entropy,
                with_fixed_render_secrets(next.run(request)),
            )
            .await
        },
    ));
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut b = a.anonymous();
    let mut google = std::collections::BTreeMap::<String, String>::new();
    let mut slack_state = String::new();
    let mut preview = String::new();
    let mut personal = String::new();
    let code = rails_compat::totp::at(
        "JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP",
        a.booted.app.clock.now().as_second(),
    )
    .unwrap();
    let mut differences = Vec::new();
    for row in expected["rows"].as_array().unwrap() {
        let name = row["name"].as_str().unwrap();
        if name == "preview-complete" {
            drive(&a, preview.parse().unwrap()).await;
        }
        if name == "import-complete" {
            drive(&a, personal.parse().unwrap()).await;
        }
        let (method, path, params) = match name {
            "login" => (Method::GET, "/session/new".into(), vec![]),
            "google-csrf" | "google-start" => (Method::POST, "/session/google".into(), vec![]),
            "google-claim" | "google-sudo-confirm" => {
                let mut v = claims(&a, &google, "ws16-claim-subject", "jane@smartdata.net");
                v["name"] = json!("Google Jane");
                answer(&r, v);
                (
                    Method::GET,
                    format!(
                        "/session/google/callback?state={}&code=fixture-code",
                        crate::controllers::presenters::test_support::encode(&google["state"])
                    ),
                    vec![],
                )
            }
            "enrollment-setup" => (Method::GET, "/two_factor_setup".into(), vec![]),
            "enrollment-csrf" | "enrollment-confirm" => (
                Method::POST,
                "/two_factor_setup".into(),
                vec![("code", code.as_str())],
            ),
            "preview-needs-opt-in" | "preview-csrf" | "preview-start" => {
                (Method::POST, "/slack/imports".into(), vec![])
            }
            "slack-needs-sudo" | "slack-start" | "slack-retry" => (
                Method::GET,
                "/slack/oauth/start?return_to=/slack/imports".into(),
                vec![],
            ),
            "sudo-prompt" => (Method::GET, "/sudo/new".into(), vec![]),
            "sudo-csrf" | "google-sudo-start" => (Method::POST, "/sudo/google".into(), vec![]),
            "slack-forged-state" | "slack-opt-in" | "slack-replay" => (
                Method::GET,
                format!(
                    "/slack/oauth/callback?state={}&code=fixture-slack-code",
                    crate::controllers::presenters::test_support::encode(
                        if name == "slack-forged-state" {
                            "forged"
                        } else {
                            &slack_state
                        }
                    )
                ),
                vec![],
            ),
            "preview-complete" => (Method::GET, format!("/slack/imports/{preview}"), vec![]),
            "import-start" => (
                Method::POST,
                "/slack/imports".into(),
                vec![
                    ("mode", "import"),
                    ("dry_run_id", preview.as_str()),
                    ("conversation_ids[]", "DCLAIM"),
                ],
            ),
            "import-complete" => (Method::GET, format!("/slack/imports/{personal}"), vec![]),
            "enrollment-required"
            | "personal-unconnected"
            | "opt-in-alert"
            | "slack-expired-page"
            | "personal-connected"
            | "personal-final"
            | "replay-page" => (Method::GET, "/slack/imports".into(), vec![]),
            _ => panic!("unknown Rails interaction: {name}"),
        };
        let reply = send(
            &mut b,
            address,
            method,
            &path,
            &params,
            name.ends_with("csrf"),
        )
        .await;
        assert_eq!(
            reply.status.as_u16() as u64,
            row["status"].as_u64().unwrap(),
            "{name}: status; {}",
            reply.text()
        );
        assert_eq!(json!(reply.location()), row["location"], "{name}: redirect");
        assert_eq!(
            json!(reply.content_type()),
            row["content_type"],
            "{name}: media type"
        );
        if !crate::app::asset_goldens::compare(name, &reply.text(), row["body"].as_str().unwrap()) {
            let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../.scratch/claim-diff");
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(format!("{name}.actual")), reply.body.clone()).unwrap();
            std::fs::write(
                dir.join(format!("{name}.expected")),
                row["body"].as_str().unwrap(),
            )
            .unwrap();
            differences.push(format!("{name}: body"));
        }
        let session = cookie_session(&a, &b);
        let flash = if reply.status.is_redirection() {
            session
                .get("flash")
                .and_then(|f| f.get("flashes"))
                .cloned()
                .unwrap_or(json!({}))
        } else {
            json!({})
        };
        assert_eq!(flash, row["flash"], "{name}: flash");
        assert_eq!(
            snapshot(&a, claimant, expected["columns"].clone()).await,
            row["state"],
            "{name}: ownership"
        );
        if name == "google-start" || name == "google-sudo-start" {
            google = url::Url::parse(reply.location().unwrap())
                .unwrap()
                .query_pairs()
                .into_owned()
                .collect();
        }
        if name == "slack-start" || name == "slack-retry" {
            slack_state = url::Url::parse(reply.location().unwrap())
                .unwrap()
                .query_pairs()
                .find(|(k, _)| k == "state")
                .unwrap()
                .1
                .into_owned();
        }
        if name == "preview-start" {
            preview = reply.location().unwrap().rsplit('/').next().unwrap().into();
        }
        if name == "import-start" {
            personal = reply.location().unwrap().rsplit('/').next().unwrap().into();
        }
    }
    server.abort();
    let _ = server.await;
    let mut calls = Vec::new();
    for (_, target, body) in r.calls.lock().unwrap().iter() {
        if target.ends_with("/token") {
            let form: std::collections::BTreeMap<_, _> = url::form_urlencoded::parse(body)
                .into_owned()
                .filter(|(k, _)| {
                    ["code", "redirect_uri", "grant_type", "code_verifier"].contains(&k.as_str())
                })
                .collect();
            calls.push(json!({"provider":"google","path":"/token","form":form}));
        } else {
            calls.push(json!({"provider":"google","path":"/oauth2/v3/certs"}));
        }
    }
    for request in slack_server.received() {
        let url = url::Url::parse(&format!("https://slack.com{}", request.target)).unwrap();
        if request.method == "POST" {
            let form: std::collections::BTreeMap<_, _> = url::form_urlencoded::parse(&request.body)
                .into_owned()
                .filter(|(k, _)| k != "client_secret")
                .collect();
            calls.push(json!({"provider":"slack","path":url.path(),"form":form}));
        } else {
            assert_eq!(
                request.header("authorization"),
                Some(["Bearer", &grant()].join(" ").as_str())
            );
            calls.push(json!({"provider":"slack","path":url.path(),"query":url.query_pairs().into_owned().collect::<std::collections::BTreeMap<_,_>>()}));
        }
    }
    assert_eq!(
        json!(calls),
        expected["calls"],
        "recorded transport and exact PKCE exchange inputs"
    );
    println!(
        "Google → Slack claim HTTP parity: {} responses; 2 Google verifications; 2 personal jobs; {} ownership tables; {} byte mismatches",
        expected["rows"].as_array().unwrap().len(),
        expected["columns"].as_object().unwrap().len(),
        differences.len()
    );
    assert!(differences.is_empty(), "{differences:?}");
}
