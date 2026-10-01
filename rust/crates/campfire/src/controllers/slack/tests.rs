use crate::{
    app::{self, App},
    config::Config,
    integrations::{
        net::Network,
        slack::client::tests::fake,
        test_support::{FakeServer, Route},
    },
};
use campfire_db::{NewSession, Session, fixtures};
use campfire_kit::Crypto;
use serde_json::{Value, json};
use std::sync::Arc;
#[path = "test_support.rs"]
mod support;
use support::{request, response_session, sudo};
pub(super) struct Fresh {
    pub app: App,
    pub router: axum::Router,
    pub server: FakeServer,
    pub cookie: String,
    _dir: tempfile::TempDir,
}
impl Fresh {
    async fn new(role: i64, routes: Vec<Route>) -> Self {
        let (server, network): (FakeServer, Network) = fake(routes).await;
        let scratch =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.scratch/ws16-http");
        std::fs::create_dir_all(&scratch).unwrap();
        let dir = tempfile::tempdir_in(scratch).unwrap();
        let config = Config::from_lookup(|key| match key {
            "SECRET_KEY_BASE" => Some("a".repeat(128)),
            "CAMPFIRE_STORAGE_PATH" => Some(dir.path().to_string_lossy().into_owned()),
            "DISABLE_SSL" => Some("1".into()),
            _ => None,
        })
        .unwrap();
        let boot = app::boot_with_network(
            config,
            Arc::new(campfire_kit::FrozenClock::new(
                "2026-01-01T12:00:00Z".parse().unwrap(),
            )),
            network,
        )
        .await
        .unwrap();
        boot.jobs.shutdown(std::time::Duration::from_secs(1)).await;
        let session=boot.app.db.write(move|tx| {
   fixtures::load(tx.conn(),&fixtures::reference_dir(),&fixtures::Options {now:tx.now(),bcrypt_cost:4})?;
   tx.conn().execute("INSERT INTO users(id,name,role,created_at,updated_at) VALUES(811,'Oracle',?,?,?)",rusqlite::params![role,tx.now(),tx.now()])?;
   tx.conn().execute("INSERT INTO users(id,name,created_at,updated_at) VALUES(812,'Other',?,?)",rusqlite::params![tx.now(),tx.now()])?;
   Session::start_with(tx,811,NewSession{two_factor_verified:true,..Default::default()})
  }).await.unwrap();
        let signed = campfire_kit::RailsCrypto::new(boot.app.secrets.clone()).sign_cookie(
            "session_token",
            &session.token,
            None,
        );
        let cookie = format!(
            "session_token={}",
            url::form_urlencoded::byte_serialize(signed.as_bytes()).collect::<String>()
        );
        Self {
            app: boot.app,
            router: boot.router,
            server,
            cookie,
            _dir: dir,
        }
    }
    async fn workspace(&self) {
        let crypto = rails_compat::ar_encryption::ArEncryption::new(&self.app.secrets);
        self.app
            .db
            .write(move |tx| {
                campfire_db::models::slack::SlackWorkspace::create(
                    tx,
                    &crypto,
                    "fixture-client",
                    "fixture-secret",
                    Some(811),
                )
            })
            .await
            .unwrap();
    }
}
#[tokio::test]
async fn slack_oauth_security_requires_sudo_and_consumes_invalid_state_before_network() {
    let f = Fresh::new(1, vec![]).await;
    f.workspace().await;
    for (method, path) in [
        ("GET", "/slack/oauth/start"),
        ("DELETE", "/slack/connection"),
    ] {
        let (status, headers, _) = request(&f, method, path, Value::Null, json!({})).await;
        assert_eq!(status, 302);
        assert_eq!(headers["location"], "http://example.org/sudo/new");
    }
    let (status,headers,_)=request(&f,"GET","/slack/oauth/callback?state=bogus&code=fixture-code",Value::Null,json!({"slack_oauth_state":{"state":"fixture-state","user_id":811},"slack_oauth_return_to":"/slack/imports"})).await;
    assert_eq!(status, 302);
    assert_eq!(headers["location"], "http://example.org/slack/imports");
    let session = response_session(&f, &headers);
    assert_eq!(
        session["flash"]["flashes"]["alert"],
        "Slack connection expired. Try again."
    );
    assert!(session.get("slack_oauth_state").is_none());
    assert!(session.get("slack_oauth_return_to").is_none());
    assert!(f.server.received().is_empty());
}
#[tokio::test]
async fn slack_oauth_start_is_user_scoped_and_return_path_is_allowlisted() {
    let f = Fresh::new(0, vec![]).await;
    f.workspace().await;
    let (status, headers, _) = request(
        &f,
        "GET",
        "/slack/oauth/start?return_to=https%3A%2F%2Fevil.test",
        Value::Null,
        sudo(),
    )
    .await;
    assert_eq!(status, 302);
    let location = url::Url::parse(headers["location"].to_str().unwrap()).unwrap();
    assert_eq!(location.host_str(), Some("slack.com"));
    let q: std::collections::HashMap<_, _> = location.query_pairs().into_owned().collect();
    assert!(!q.contains_key("scope"));
    assert_eq!(q["redirect_uri"], "http://example.org/slack/oauth/callback");
    let session = response_session(&f, &headers);
    assert_eq!(session["slack_oauth_state"]["user_id"], 811);
    assert_eq!(session["slack_oauth_return_to"], "/slack/imports");
    assert_eq!(
        session["slack_oauth_state"]["state"]
            .as_str()
            .unwrap()
            .len(),
        32
    );
}

#[tokio::test]
async fn slack_connections_http_persistence_audits_and_requests_match_rails() {
    use campfire_db::models::slack::{NewConnection, SlackConnection, SlackWorkspace};
    let cases: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/slack/connections_http.json"
    ))
    .unwrap();
    for case in cases.as_array().unwrap() {
        let exchange = if case["exchange_error"] == true {
            json!({"ok":false,"error":"invalid_code"})
        } else {
            json!({"ok":true,"team":{"id":"TFIXTURE","name":if case["blank_team_name"]==true {Value::Null}else{json!("Fixture")}},"authed_user":{"id":"UFIXTURE","access_token":"fixture-user-grant","scope":case["scopes"].as_str().map(str::to_owned).unwrap_or_else(||crate::integrations::slack::oauth::USER_SCOPES.join(","))}})
        };
        let team_body = if case["invalid_team_info"] == true {
            r#"{"ok":true,"team":42}"#
        } else if case["team_missing"] == true {
            "{}"
        } else {
            r#"{"ok":true,"team":{"id":"TFIXTURE","name":"Team fallback","domain":"fixture"}}"#
        };
        let routes = vec![
            Route::new("POST", "slack.com", "/api/oauth.v2.access", 200).body(exchange.to_string()),
            Route::new("POST", "slack.com", "/api/auth.revoke", 200).body(r#"{"revoked":true}"#),
            Route::new("GET", "slack.com", "/api/team.info", 200).body(team_body),
        ];
        let f = Fresh::new(case["role"].as_i64().unwrap_or(1), routes).await;
        let input = case.clone();
        let crypto = rails_compat::ar_encryption::ArEncryption::new(&f.app.secrets);
        f.app.db.write(move|tx| {
   if input["unconfigured"]!=true {
    let w=SlackWorkspace::create(tx,&crypto,"fixture-client","fixture-secret",Some(811))?;
    tx.conn().execute("UPDATE slack_workspaces SET id=851,team_id=?,team_name=? WHERE id=?",rusqlite::params![input["team_id"].as_str(),input["team_name"].as_str(),w.id])?;
    if input["preset"]==true {
     let c=SlackConnection::create(tx,&crypto,NewConnection{workspace_id:851,user_id:811,slack_user_id:"UFIXTURE",access_token:Some("fixture-user-grant"),scopes:Some(&crate::integrations::slack::oauth::USER_SCOPES.join(","))})?;
     tx.conn().execute("UPDATE slack_connections SET id=852,disconnected_reason=? WHERE id=?",rusqlite::params![if input["disconnected"]==true {Some("Rejected")}else{None},c.id])?;
     if input["unreadable"]==true {tx.conn().execute("UPDATE slack_connections SET access_token='unreadable-fixture' WHERE id=852",[])?;}
    }
    if input["claimed"]==true {SlackConnection::create(tx,&crypto,NewConnection{workspace_id:851,user_id:812,slack_user_id:"UFIXTURE",access_token:Some("fixture-other"),scopes:None})?;}
    if input["active"]==true || input["other_active"]==true || input["history"]==true {
     tx.conn().execute("INSERT INTO slack_imports(id,slack_workspace_id,user_id,slack_connection_id,kind,mode,status,created_at,updated_at) VALUES(853,851,?,?,'personal','import',?,?,?)",rusqlite::params![if input["other_active"]==true {812}else{811},if input["preset"]==true {Some(852)}else{None},if input["history"]==true {"completed"}else{"running"},tx.now(),tx.now()])?;
    }
   }
   tx.conn().execute("DELETE FROM sqlite_sequence WHERE name IN ('slack_workspaces','slack_connections','slack_imports')",[])?;
   for row in input["sequences"].as_array().unwrap() {tx.conn().execute("INSERT INTO sqlite_sequence(name,seq) VALUES(?,?)",rusqlite::params![row["name"].as_str().unwrap(),row["seq"].as_i64().unwrap()])?;}
   Ok(())
  }).await.unwrap();
        let mut values = if case["no_sudo"] == true {
            json!({})
        } else {
            sudo()
        };
        values["slack_oauth_state"] = json!({"state":if case["wrong_state"]==true {"wrong"}else{"fixture-state"},"user_id":case["state_user"].as_i64().unwrap_or(811)});
        values["slack_oauth_return_to"] = case["return_to"].clone();
        let method = case["method"].as_str().unwrap_or("GET");
        let mut path = case["path"]
            .as_str()
            .unwrap_or("/slack/oauth/callback")
            .to_owned();
        let mut body = case["body"]
            .as_object()
            .cloned()
            .map(Value::Object)
            .unwrap_or(json!({}));
        if method == "GET" {
            let signed = if case["bad_state"] == true {
                "bogus".into()
            } else {
                crate::integrations::slack::oauth::sign_state(&f.app.secrets, "fixture-state")
            };
            let mut q = url::form_urlencoded::Serializer::new(String::new());
            q.append_pair("code", "fixture-code")
                .append_pair("state", &signed);
            if let Some(error) = case["error"].as_str() {
                q.append_pair("error", error);
            }
            path.push('?');
            path.push_str(&q.finish());
        } else if !case["return_to"].is_null() {
            body["return_to"] = case["return_to"].clone();
        }
        let (status, headers, html) = request(&f, method, &path, body, values).await;
        assert_eq!(status, case["status"], "{}: {html}", case["name"]);
        assert_eq!(
            headers.get("location").and_then(|h| h.to_str().ok()),
            case["location"].as_str(),
            "{}",
            case["name"]
        );
        let state = response_session(&f, &headers);
        assert_eq!(
            state["flash"]["flashes"]
                .as_object()
                .cloned()
                .map(Value::Object)
                .unwrap_or(json!({})),
            case["flash"],
            "{}",
            case["name"]
        );
        if method == "GET" {
            assert!(state.get("slack_oauth_state").is_none());
            assert!(state.get("slack_oauth_return_to").is_none());
        }
        let crypto = rails_compat::ar_encryption::ArEncryption::new(&f.app.secrets);
        let (connection,workspace,audits,history)=f.app.db.read(move|c| {
   let conn=SlackConnection::for_user(c,811)?.map(|v| {let token=v.access_token(&crypto).ok().flatten();if let Some(token)=&token {let raw:String=c.query_row("SELECT access_token FROM slack_connections WHERE id=?",[v.id],|r|r.get(0)).unwrap();assert_ne!(raw,*token);}
    json!({"slack_workspace_id":v.slack_workspace_id,"user_id":v.user_id,"slack_user_id":v.slack_user_id,"scopes":v.scopes,"disconnected_reason":v.disconnected_reason,"access_token":token})}).unwrap_or(Value::Null);
   let workspace=SlackWorkspace::current(c)?.map(|w| {let secret=w.client_secret(&crypto).unwrap();if let Some(secret)=&secret {let raw:String=c.query_row("SELECT client_secret FROM slack_workspaces WHERE id=?",[w.id],|r|r.get(0)).unwrap();assert_ne!(raw,*secret);}
    json!({"client_id":w.client_id,"client_secret":secret,"configured_by_id":w.configured_by_id,"team_id":w.team_id,"team_name":w.team_name,"team_domain":w.team_domain})}).unwrap_or(Value::Null);
   let audits=c.prepare("SELECT action,actor_id,target_type,target_id,details FROM audit_logs WHERE action LIKE 'slack.%' ORDER BY id")?.query_map([],|r|Ok(json!({"action":r.get::<_,String>(0)?,"actor_id":r.get::<_,Option<i64>>(1)?,"target_type":r.get::<_,Option<String>>(2)?,"target_id":r.get::<_,Option<i64>>(3)?,"details":serde_json::from_str::<Value>(&r.get::<_,String>(4)?).unwrap()})))?.collect::<rusqlite::Result<Vec<_>>>()?;
   use rusqlite::OptionalExtension;
   let history=c.query_row("SELECT slack_connection_id FROM slack_imports WHERE id=853",[],|r|r.get::<_,Option<i64>>(0)).optional()?.flatten();Ok((conn,workspace,json!(audits),json!(history)))
  }).await.unwrap();
        assert_eq!(connection, case["connection"], "{}", case["name"]);
        assert_eq!(workspace, case["workspace"], "{}", case["name"]);
        assert_eq!(audits, case["audit"], "{}", case["name"]);
        assert_eq!(history, case["history_connection_id"], "{}", case["name"]);
        let actual = f.server.received();
        let expected = case["requests"].as_array().unwrap();
        assert_eq!(actual.len(), expected.len(), "{}", case["name"]);
        for (actual, expected) in actual.iter().zip(expected) {
            assert_eq!(actual.method, expected["method"]);
            assert_eq!(actual.target, expected["path"]);
            if let Some(form) = expected.get("form") {
                let actual: serde_json::Map<String, Value> =
                    url::form_urlencoded::parse(&actual.body)
                        .map(|(k, v)| (k.into_owned(), json!(v)))
                        .collect();
                assert_eq!(json!(actual), *form);
            }
        }
    }
}
#[tokio::test]
async fn slack_setup_views_are_byte_identical_to_rails_and_write_only() {
    use askama::Template;
    use campfire_views::{
        helpers::request_forgery::{self, AuthenticityTokens, RequestSecrets},
        slack::{Setup, SetupData},
    };
    struct Tokens;
    impl AuthenticityTokens for Tokens {
        fn global(&self) -> String {
            "GLOBAL".into()
        }
        fn for_form(&self, a: &str, m: &str) -> String {
            format!("{m}:{a}")
        }
    }
    let f = Fresh::new(1, vec![]).await;
    let v: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/slack/setup_views.json"
    ))
    .unwrap();
    for case in v["cases"].as_array().unwrap() {
        let data: SetupData = serde_json::from_value(case["data"].clone()).unwrap();
        let actual = crate::controllers::presenters::page::render_detached_at(
            &f.app,
            None,
            "http://example.org",
            |ctx| {
                request_forgery::rendering_with(
                    RequestSecrets {
                        tokens: Box::new(Tokens),
                        csp_nonce: Some("NONCE".into()),
                    },
                    || Setup { ctx, data: &data }.as_content().render().unwrap(),
                )
            },
        );
        let expected = case["html"].as_str().unwrap();
        if actual != expected {
            let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../.scratch/ws16-http");
            std::fs::write(dir.join("setup.actual.html"), &actual).unwrap();
            std::fs::write(dir.join("setup.expected.html"), expected).unwrap();
        }
        assert_eq!(actual, expected, "{}", case["name"]);
        assert!(!actual.contains("fixture-secret"));
    }
}

#[tokio::test]
async fn slack_oauth_replay_unique_constraint_and_setup_csrf_are_rejected() {
    use campfire_db::models::slack::{NewConnection, SlackConnection};
    let exchange = json!({"ok":true,"team":{"id":"TFIXTURE","name":"Fixture"},"authed_user":{"id":"UFIXTURE","access_token":"fixture-user-grant","scope":crate::integrations::slack::oauth::USER_SCOPES.join(",")}});
    let f = Fresh::new(
        1,
        vec![
            Route::new("POST", "slack.com", "/api/oauth.v2.access", 200).body(exchange.to_string()),
        ],
    )
    .await;
    f.workspace().await;
    let crypto = rails_compat::ar_encryption::ArEncryption::new(&f.app.secrets);
    f.app
        .db
        .write(move |tx| {
            let w = campfire_db::models::slack::SlackWorkspace::current(tx.conn())?.unwrap();
            SlackConnection::create(
                tx,
                &crypto,
                NewConnection {
                    workspace_id: w.id,
                    user_id: 812,
                    slack_user_id: "UOTHER",
                    access_token: Some("fixture-other"),
                    scopes: None,
                },
            )?;
            // A real unique-index rejection exercises the raced-claim rescue after the precheck.
            tx.conn().execute(
                "CREATE UNIQUE INDEX ws16_raced_connection ON slack_connections((1))",
                [],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let signed = crate::integrations::slack::oauth::sign_state(&f.app.secrets, "fixture-state");
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("state", &signed)
        .append_pair("code", "fixture-code")
        .finish();
    let path = format!("/slack/oauth/callback?{query}");
    let (_, headers, _) = request(
        &f,
        "GET",
        &path,
        Value::Null,
        json!({"slack_oauth_state":{"state":"fixture-state","user_id":811}}),
    )
    .await;
    let values = response_session(&f, &headers);
    assert_eq!(
        values["flash"]["flashes"]["alert"],
        "That Slack account is already connected to another Smartfire user."
    );
    let (_, headers, _) = request(&f, "GET", &path, Value::Null, values).await;
    assert_eq!(
        response_session(&f, &headers)["flash"]["flashes"]["alert"],
        "Slack connection expired. Try again."
    );
    assert_eq!(f.server.received().len(), 1);
    assert!(
        f.app
            .db
            .read(|c| SlackConnection::for_user(c, 811))
            .await
            .unwrap()
            .is_none()
    );
    use tower::ServiceExt;
    let response = f
        .router
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("PATCH")
                .uri("/account/slack_import")
                .header("Host", "example.org")
                .header("Cookie", support::session(&f, &sudo()))
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(
                    r#"{"client_id":"attacker","client_secret":"fixture-secret"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 422);
    let id = f
        .app
        .db
        .read(|c| {
            Ok(campfire_db::models::slack::SlackWorkspace::current(c)?
                .unwrap()
                .client_id)
        })
        .await
        .unwrap();
    assert_eq!(id.as_deref(), Some("fixture-client"));
    let member = Fresh::new(0, vec![]).await;
    assert_eq!(
        request(&member, "GET", "/account/slack_import", Value::Null, sudo())
            .await
            .0,
        403
    );
    let filtered=crate::security::parameter_filter().filter(&json!({"code":"fixture-code","access_token":"fixture-user-grant","client_secret":"fixture-secret"}));
    assert_eq!(
        filtered,
        json!({"code":"[FILTERED]","access_token":"[FILTERED]","client_secret":"[FILTERED]"})
    );
}
#[path = "run_tests.rs"]
mod run_tests;
