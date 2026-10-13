use super::presenters::test_support::{DAVID, Req, TestApp};
use campfire_kit::{FrozenClock, Method};
use serde_json::json;
use std::sync::Arc;

async fn setup(enabled: bool) -> TestApp {
    TestApp::boot_with_clock_and_env(
        Arc::new(FrozenClock::new("2026-10-13T16:00:00Z".parse().unwrap())),
        &[
            ("APP_URL", "https://smartfire.example"),
            ("SMARTFIRE_MCP_OAUTH", if enabled { "1" } else { "0" }),
        ],
    )
    .await
    .expect("restored default seed")
    .without_job_runner()
    .await
}

#[tokio::test]
async fn oauth_switch_off_preserves_mcp_bytes_and_hides_endpoints() {
    let app = setup(false).await;
    for path in [
        "/.well-known/oauth-authorization-server",
        "/.well-known/oauth-protected-resource",
        "/.well-known/oauth-protected-resource/agents/mcp",
        "/oauth/authorize",
        "/oauth/token",
        "/oauth/revoke",
        "/oauth/register",
        "/api/v1/oauth/authorizations/unknown",
    ] {
        for method in [Method::GET, Method::POST, Method::OPTIONS] {
            assert_eq!(
                app.anonymous()
                    .send(Req::new(method, path))
                    .await
                    .status
                    .as_u16(),
                404,
                "{path}"
            );
        }
    }
    let req = || {
        Req::new(Method::POST, "/agents/mcp")
            .json_body(&json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}))
    };
    let missing = app.sign_in(DAVID).await.send(req()).await;
    assert_eq!(missing.status.as_u16(), 403);
    assert_eq!(
        missing.body,
        br#"{"error":"Forbidden: Bearer agent token required"}"#
    );
    assert_eq!(missing.header("www-authenticate"), None);
    let invalid = app
        .anonymous()
        .send(req().header("authorization", &["Bearer", "invalid"].join(" ")))
        .await;
    assert_eq!(invalid.status.as_u16(), 401);
    assert!(invalid.body.is_empty());
    assert_eq!(invalid.header("www-authenticate"), None);
}

#[tokio::test]
async fn oauth_switch_on_advertises_cimd_dcr_client_authentication_and_challenges() {
    let app = setup(true).await;
    let meta = app
        .anonymous()
        .get("/.well-known/oauth-authorization-server")
        .await;
    assert_eq!(meta.status.as_u16(), 200, "{}", meta.text());
    assert_eq!(meta.json()["issuer"], "https://smartfire.example");
    assert_eq!(
        meta.json()["token_endpoint_auth_methods_supported"],
        json!(["none", "private_key_jwt"])
    );
    assert_eq!(
        meta.json()["registration_endpoint"],
        "https://smartfire.example/oauth/register"
    );
    assert_eq!(meta.json()["token_endpoint_auth_signing_alg_values_supported"], json!(["RS256", "ES256"]));
    let mut owner = app.sign_in(DAVID).await;
    assert_eq!(
        owner.get("/api/v1/me/agents").await.json()["oauthAvailable"],
        true
    );
    let missing = app
        .anonymous()
        .send(Req::new(Method::POST, "/agents/mcp").json_body(&json!({"method":"tools/list"})))
        .await;
    assert_eq!(missing.status.as_u16(), 401);
    assert_eq!(
        missing.header("www-authenticate"),
        Some(
            "Bearer resource_metadata=\"https://smartfire.example/.well-known/oauth-protected-resource/agents/mcp\", scope=\"agent\""
        )
    );
    let missing_session = owner
        .send(Req::new(Method::POST, "/agents/mcp").json_body(&json!({"method":"tools/list"})))
        .await;
    assert_eq!(
        missing_session.status.as_u16(),
        401,
        "session without agent token must get the OAuth challenge"
    );
    assert!(missing_session.header("www-authenticate").is_some());
}

use super::presenters::test_support::{Browser, Reply};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use campfire_app::net::{
    Network,
    test_support::{FakeResolver, FakeServer, MappingDialer, Route},
};
use campfire_kit::Clock;
use rusqlite::params;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::sync::Mutex;

const CLIENT: &str = "https://client.example/metadata.json";
const REDIRECT: &str = "https://client.example/callback";
const RESOURCE: &str = "https://smartfire.example/agents/mcp";
const VERIFIER: &str = "abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGH";

fn metadata() -> Value {
    json!({"client_id":CLIENT,"client_name":"Test client","redirect_uris":[REDIRECT,"http://127.0.0.1/callback","http://[::1]/callback"],"token_endpoint_auth_method":"none"})
}
struct Fixture {
    app: TestApp,
    server: FakeServer,
    resolver: Arc<FakeResolver>,
    dialer: Arc<MappingDialer>,
    clock: Arc<FrozenClock>,
}
async fn fixture(document: Value) -> Fixture {
    let host = url::Url::parse(document["client_id"].as_str().unwrap())
        .unwrap()
        .host_str()
        .unwrap()
        .to_string();
    let path = url::Url::parse(document["client_id"].as_str().unwrap())
        .unwrap()
        .path()
        .to_string();
    let route = Route::new("GET", &host, &path, 200)
        .header("Content-Type", "application/json")
        .header("Cache-Control", "max-age=600")
        .body(serde_json::to_vec(&document).unwrap());
    let (server, roots) = FakeServer::start_named_tls_ws15e(vec![route], vec![host.clone()]).await;
    let resolver = Arc::new(FakeResolver::new([(host.as_str(), vec!["93.184.216.34"])]));
    let dialer = Arc::new(MappingDialer {
        public: HashSet::from(["93.184.216.34".parse().unwrap()]),
        to: server.addr,
        dialed: Mutex::new(vec![]),
    });
    let network = Network {
        resolver: resolver.clone(),
        dialer: dialer.clone(),
        tls: campfire_app::net::tls_config(roots),
    };
    let clock = Arc::new(FrozenClock::new("2026-10-13T16:00:00Z".parse().unwrap()));
    let app = TestApp::boot_with_network_clock_and_env(
        network,
        clock.clone(),
        &[
            ("APP_URL", "https://smartfire.example"),
            ("SMARTFIRE_MCP_OAUTH", "1"),
        ],
    )
    .await
    .unwrap()
    .without_job_runner()
    .await;
    Fixture {
        app,
        server,
        resolver,
        dialer,
        clock,
    }
}
fn authorize_url(client: &str, redirect: &str) -> String {
    let mut url = url::Url::parse("http://campfire.test/oauth/authorize").unwrap();
    url.query_pairs_mut().extend_pairs([
        ("client_id", client),
        ("redirect_uri", redirect),
        ("response_type", "code"),
        ("code_challenge_method", "S256"),
        (
            "code_challenge",
            &URL_SAFE_NO_PAD.encode(Sha256::digest(VERIFIER)),
        ),
        ("resource", RESOURCE),
        ("scope", "agent"),
        ("state", "opaque state & value"),
    ]);
    format!("/oauth/authorize?{}", url.query().unwrap())
}
fn consent_path(reply: &Reply) -> String {
    let url = url::Url::parse(reply.location().expect("consent redirect")).unwrap();
    assert!(
        url.path().starts_with("/app/oauth/consent/"),
        "{}",
        reply.text()
    );
    url.path()
        .replace("/app/oauth/consent/", "/api/v1/oauth/authorizations/")
}
async fn approve(browser: &mut Browser<'_>, client: &str, redirect: &str) -> String {
    let reply = browser.get(&authorize_url(client, redirect)).await;
    let path = consent_path(&reply);
    let consent = browser.get(&path).await;
    assert_eq!(consent.status.as_u16(), 200, "{}", consent.text());
    assert!(
        consent
            .header("content-security-policy")
            .unwrap()
            .contains("frame-ancestors 'none'")
    );
    browser.grant_sudo().await;
    let reply = browser
        .write(
            Req::new(Method::POST, &format!("{path}/approve"))
                .json_body(&json!({"name":"Connected agent","roomIds":[],"capabilities":[]})),
        )
        .await;
    assert_eq!(reply.status.as_u16(), 200, "{}", reply.text());
    let url = url::Url::parse(reply.json()["redirectTo"].as_str().unwrap()).unwrap();
    let query = url
        .query_pairs()
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(query.get("state").unwrap(), "opaque state & value");
    assert_eq!(query.get("iss").unwrap(), "https://smartfire.example");
    query.get("code").unwrap().to_string()
}
async fn exchange(
    app: &TestApp,
    code: &str,
    client: &str,
    redirect: &str,
    verifier: &str,
    resource: &str,
) -> Reply {
    app.anonymous()
        .send(Req::new(Method::POST, "/oauth/token").form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("client_id", client),
            ("redirect_uri", redirect),
            ("code_verifier", verifier),
            ("resource", resource),
        ]))
        .await
}
async fn tokens(f: &Fixture) -> Value {
    let mut browser = f.app.sign_in(DAVID).await;
    let code = approve(&mut browser, CLIENT, REDIRECT).await;
    let reply = exchange(&f.app, &code, CLIENT, REDIRECT, VERIFIER, RESOURCE).await;
    assert_eq!(reply.status.as_u16(), 200, "{}", reply.text());
    reply.json()
}
async fn refresh(app: &TestApp, token: &str, client: &str, resource: &str, method: bool) -> Reply {
    let mut req = Req::new(Method::POST, "/oauth/token").form(&[
        ("grant_type", "refresh_token"),
        ("refresh_token", token),
        ("client_id", client),
        ("resource", resource),
    ]);
    if method {
        req = req.header("authorization", "Basic dGVzdDpzZWNyZXQ=");
    }
    app.anonymous().send(req).await
}
async fn mcp(app: &TestApp, token: &str) -> Reply {
    app.anonymous()
        .send(
            Req::new(Method::POST, "/agents/mcp")
                .header("authorization", &["Bearer", token].join(" "))
                .json_body(&json!({"jsonrpc":"2.0","id":1,"method":"tools/list"})),
        )
        .await
}
fn invalid(reply: Reply) {
    assert_eq!(reply.status.as_u16(), 400, "{}", reply.text());
    assert_eq!(reply.json()["error"], "invalid_grant");
    assert_eq!(reply.header("cache-control"), Some("no-store"));
}

#[tokio::test]
async fn oauth_pkce_flow_fetches_pinned_cimd_and_uses_cache() {
    let f = fixture(metadata()).await;
    let pair = tokens(&f).await;
    assert_eq!(pair["token_type"], "Bearer");
    assert_eq!(pair["scope"], "agent");
    assert_eq!(pair["expires_in"], 3600);
    assert_eq!(
        mcp(&f.app, pair["access_token"].as_str().unwrap())
            .await
            .status
            .as_u16(),
        200
    );
    let mut owner = f.app.sign_in(DAVID).await;
    let second = owner.get(&authorize_url(CLIENT, REDIRECT)).await;
    let consent = owner.get(&consent_path(&second)).await.json();
    assert!(consent["selectedAgentId"].is_number());
    assert_eq!(f.server.received.lock().unwrap().len(), 1);
    assert_eq!(f.resolver.lookups(), ["client.example"]);
    assert_eq!(
        f.dialer.dialed.lock().unwrap()[0].ip().to_string(),
        "93.184.216.34"
    );
}

async fn oauth_signed_out_to_consent<'a>(
    f: &'a Fixture,
    start: &str,
    state: &str,
) -> (Browser<'a>, String) {
    let mut browser = f.app.anonymous();
    let authorized = browser.get(start).await;
    assert_eq!(authorized.status.as_u16(), 302);
    assert!(authorized.location().unwrap().contains("/app/session/new"));
    let sign_in = browser.get("/app/session/new").await;
    assert_eq!(sign_in.status.as_u16(), 200, "{}", sign_in.text());
    let login = browser
        .password_sign_in("david@37signals.com", "secret123456")
        .await;
    assert_eq!(login.status.as_u16(), 200, "{}", login.text());
    assert_eq!(login.json()["kind"], "secondFactorRequired");
    let encryption = f.app.booted.app.ar_encryption.clone();
    let code = f
        .app
        .db()
        .read(move |conn| {
            let factor = campfire_db::TwoFactorCredential::for_user(conn, DAVID)?.unwrap();
            let secret = factor.secret(&encryption)?;
            Ok(rails_compat::totp::at(
                &secret,
                "2026-10-13T16:00:00Z"
                    .parse::<jiff::Timestamp>()
                    .unwrap()
                    .as_second(),
            )
            .unwrap())
        })
        .await
        .unwrap();
    let verified = browser.answer_challenge(&code, false).await;
    assert_eq!(verified.status.as_u16(), 200, "{}", verified.text());
    let return_to = url::Url::parse(verified.json()["location"].as_str().unwrap()).unwrap();
    assert_eq!(return_to.path(), "/oauth/authorize");
    assert_eq!(
        return_to.as_str(),
        format!(
            "http://campfire.test{}",
            authorize_with_state(REDIRECT, state)
        )
    );
    let reply = browser
        .get(&format!(
            "{}?{}",
            return_to.path(),
            return_to.query().unwrap()
        ))
        .await;
    let path = consent_path(&reply);
    let consent = browser.get(&path).await;
    assert_eq!(consent.status.as_u16(), 200);
    assert_eq!(consent.json()["clientName"], "Test client");
    for response in [authorized, sign_in, login, verified, reply, consent] {
        assert_oauth_cookie_budget(&response);
    }
    (browser, path)
}

fn assert_oauth_cookie_budget(reply: &Reply) {
    assert_ne!(reply.status.as_u16(), 500, "{}", reply.text());
    for cookie in reply.headers.get_all("set-cookie") {
        let bytes = cookie.as_bytes().len();
        assert!(bytes < 4096, "Set-Cookie header was {bytes} bytes");
    }
}

fn oauth_return_json_size(state: &str) -> usize {
    let url = format!(
        "http://campfire.test{}",
        authorize_with_state(REDIRECT, state)
    );
    let json = serde_json::to_string(&url).unwrap();
    json.len()
        + json
            .bytes()
            .filter(|byte| matches!(byte, b'&' | b'<' | b'>'))
            .count()
            * 5
}

fn oauth_state_at_return_json_limit(piece: &str) -> String {
    let overhead = oauth_return_json_size("");
    let unit = oauth_return_json_size(piece) - overhead;
    let available = 1024 - overhead;
    let mut state = piece.repeat(available / unit);
    state.push_str(&"a".repeat(available % unit));
    assert_eq!(oauth_return_json_size(&state), 1024);
    state
}

#[tokio::test]
async fn oauth_signed_out_cookie_jar_returns_through_consent_and_password_sudo() {
    let f = fixture(metadata()).await;
    let state = oauth_state_at_return_json_limit("a");
    let start = authorize_with_state(REDIRECT, &state);
    let (mut browser, path) = oauth_signed_out_to_consent(&f, &start, &state).await;
    let approval = || {
        Req::new(Method::POST, &format!("{path}/approve"))
            .json_body(&json!({"name":"Signed out flow","roomIds":[],"capabilities":[]}))
    };
    let sudo = browser.write(approval()).await;
    assert_eq!(sudo.status.as_u16(), 403, "{}", sudo.text());
    assert_eq!(sudo.json()["error"]["_tag"], "SudoRequired");
    let confirmed = browser.confirm_sudo("secret123456").await;
    assert_eq!(confirmed.status.as_u16(), 200);
    let done = browser.write(approval()).await;
    assert_eq!(done.status.as_u16(), 200, "{}", done.text());
    let callback = url::Url::parse(done.json()["redirectTo"].as_str().unwrap()).unwrap();
    let query = callback
        .query_pairs()
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(query.get("state").unwrap(), &state);
    assert_eq!(query.get("iss").unwrap(), "https://smartfire.example");
    let exchanged = exchange(
        &f.app,
        query.get("code").unwrap(),
        CLIENT,
        REDIRECT,
        VERIFIER,
        RESOURCE,
    )
    .await;
    assert_eq!(exchanged.status.as_u16(), 200);
    for response in [sudo, confirmed, done, exchanged] {
        assert_oauth_cookie_budget(&response);
    }
}

#[tokio::test]
async fn oauth_bad_code_inputs_do_not_consume_code_and_replay_revokes_connection() {
    let f = fixture(metadata()).await;
    let mut owner = f.app.sign_in(DAVID).await;
    let code = approve(&mut owner, CLIENT, REDIRECT).await;
    for (client, redirect, verifier, resource) in [
        (CLIENT, REDIRECT, "wrong", RESOURCE),
        (CLIENT, "https://client.example/wrong", VERIFIER, RESOURCE),
        (
            CLIENT,
            REDIRECT,
            VERIFIER,
            "https://other.example/agents/mcp",
        ),
        (
            "https://wrong.example/metadata",
            REDIRECT,
            VERIFIER,
            RESOURCE,
        ),
    ] {
        invalid(exchange(&f.app, &code, client, redirect, verifier, resource).await);
    }
    let success = exchange(&f.app, &code, CLIENT, REDIRECT, VERIFIER, RESOURCE).await;
    assert_eq!(success.status.as_u16(), 200, "{}", success.text());
    invalid(exchange(&f.app, &code, CLIENT, REDIRECT, VERIFIER, RESOURCE).await);
    assert_eq!(
        mcp(&f.app, success.json()["access_token"].as_str().unwrap())
            .await
            .status
            .as_u16(),
        401
    );
}

#[tokio::test]
async fn oauth_loopback_redirect_accepts_any_port_but_exchange_binds_exact_uri() {
    let f = fixture(metadata()).await;
    for redirect in [
        "http://127.0.0.1:54123/callback",
        "http://[::1]:54124/callback",
    ] {
        let mut owner = f.app.sign_in(DAVID).await;
        let code = approve(&mut owner, CLIENT, redirect).await;
        invalid(
            exchange(
                &f.app,
                &code,
                CLIENT,
                "http://127.0.0.1:9999/callback",
                VERIFIER,
                RESOURCE,
            )
            .await,
        );
        assert_eq!(
            exchange(&f.app, &code, CLIENT, redirect, VERIFIER, RESOURCE)
                .await
                .status
                .as_u16(),
            200
        );
    }
}

#[tokio::test]
async fn oauth_wrong_refresh_client_method_resource_and_origin_leave_token_unconsumed() {
    let f = fixture(metadata()).await;
    let pair = tokens(&f).await;
    let secret = pair["refresh_token"].as_str().unwrap();
    invalid(
        refresh(
            &f.app,
            secret,
            "https://wrong.example/client.json",
            RESOURCE,
            false,
        )
        .await,
    );
    invalid(refresh(&f.app, secret, CLIENT, RESOURCE, true).await);
    invalid(
        refresh(
            &f.app,
            secret,
            CLIENT,
            "https://wrong.example/agents/mcp",
            false,
        )
        .await,
    );
    f.app
        .db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE oauth_connections SET resource='https://old.example/agents/mcp'",
                [],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    invalid(refresh(&f.app, secret, CLIENT, RESOURCE, false).await);
    f.app
        .db()
        .write(|tx| {
            tx.conn()
                .execute("UPDATE oauth_connections SET resource=?", [RESOURCE])?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        refresh(&f.app, secret, CLIENT, RESOURCE, false)
            .await
            .status
            .as_u16(),
        200
    );
}

async fn assert_reused(f: &Fixture, original: &Value, rotated: &Value) {
    for pair in [original, rotated] {
        assert_eq!(
            mcp(&f.app, pair["access_token"].as_str().unwrap())
                .await
                .status
                .as_u16(),
            401
        );
        invalid(
            refresh(
                &f.app,
                pair["refresh_token"].as_str().unwrap(),
                CLIENT,
                RESOURCE,
                false,
            )
            .await,
        );
    }
    f.app.db().read(|conn| {
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM audit_logs WHERE action='oauth.refresh_reuse'",[],|r|r.get::<_,i64>(0))?,1);
        let jobs = conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Smartfire::MailDeliveryJob' AND arguments LIKE '%OAuthRefreshReuse%'",[],|r|r.get::<_,i64>(0))?;
        assert_eq!(jobs,1,"owner notification committed with revocation");
        let payload: String = conn.query_row("SELECT arguments FROM background_jobs WHERE job_class='Smartfire::MailDeliveryJob' AND arguments LIKE '%OAuthRefreshReuse%'", [], |r|r.get(0))?;
        let payload: Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(payload["notification"]["OAuthRefreshReuse"]["user_id"], DAVID);

        Ok(())
    }).await.unwrap();
}
#[tokio::test]
async fn oauth_attacker_rotates_first_then_original_client_reuse_revokes_every_token_and_notifies_owner()
 {
    let f = fixture(metadata()).await;
    let original = tokens(&f).await;
    // A separate cookie-free party rotates the stolen public-client token first.
    let attacker = refresh(
        &f.app,
        original["refresh_token"].as_str().unwrap(),
        CLIENT,
        RESOURCE,
        false,
    )
    .await;
    assert_eq!(attacker.status.as_u16(), 200);
    assert_ne!(original["refresh_token"], attacker.json()["refresh_token"]);
    invalid(
        refresh(
            &f.app,
            original["refresh_token"].as_str().unwrap(),
            CLIENT,
            RESOURCE,
            false,
        )
        .await,
    );
    assert_reused(&f, &original, &attacker.json()).await;
}
#[tokio::test]
async fn oauth_two_concurrent_refreshes_have_one_winner_then_revoke() {
    let f = fixture(metadata()).await;
    let original = tokens(&f).await;
    let secret = original["refresh_token"].as_str().unwrap();
    let (one, two) = tokio::join!(
        refresh(&f.app, secret, CLIENT, RESOURCE, false),
        refresh(&f.app, secret, CLIENT, RESOURCE, false)
    );
    let (winner, loser) = if one.status.as_u16() == 200 {
        (one, two)
    } else {
        (two, one)
    };
    assert_eq!(winner.status.as_u16(), 200, "{}", winner.text());
    invalid(loser);
    assert_reused(&f, &original, &winner.json()).await;
}
#[tokio::test]
async fn oauth_reuse_after_sweep_of_old_expired_tombstone_still_revokes() {
    let f = fixture(metadata()).await;
    let original = tokens(&f).await;
    let rotated = refresh(
        &f.app,
        original["refresh_token"].as_str().unwrap(),
        CLIENT,
        RESOURCE,
        false,
    )
    .await
    .json();
    let digest = campfire_db::user::digest_bot_token(original["refresh_token"].as_str().unwrap());
    f.app
        .db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE oauth_tokens SET expires_at=? WHERE token_digest=?",
                params![
                    tx.now().since(jiff::SignedDuration::from_hours(-90 * 24)),
                    digest
                ],
            )?;
            campfire_db::models::oauth::sweep(tx)
        })
        .await
        .unwrap();
    invalid(
        refresh(
            &f.app,
            original["refresh_token"].as_str().unwrap(),
            CLIENT,
            RESOURCE,
            false,
        )
        .await,
    );
    assert_reused(&f, &original, &rotated).await;
}
#[tokio::test]
async fn oauth_tokens_are_capped_at_connection_lifetime_and_refresh_stops_at_180_days() {
    let f = fixture(metadata()).await;
    let pair = tokens(&f).await;
    let deadline = f
        .app
        .db()
        .read(|conn| {
            Ok(
                conn.query_row("SELECT expires_at FROM oauth_connections", [], |r| {
                    r.get::<_, campfire_db::Timestamp>(0)
                })?,
            )
        })
        .await
        .unwrap();
    assert_eq!(
        deadline.jiff(),
        f.clock.now() + jiff::SignedDuration::from_hours(180 * 24)
    );
    f.app
        .db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE oauth_connections SET expires_at=?",
                [tx.now().since(jiff::SignedDuration::from_secs(30))],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let near = refresh(
        &f.app,
        pair["refresh_token"].as_str().unwrap(),
        CLIENT,
        RESOURCE,
        false,
    )
    .await;
    assert_eq!(near.status.as_u16(), 200);
    assert_eq!(near.json()["expires_in"], 30);
    let near_pair = near.json();
    for key in ["access_token", "refresh_token"] {
        let secret = near_pair[key].as_str().unwrap().to_string();
        f.app
            .db()
            .read(move |conn| {
                let token =
                    campfire_db::models::oauth::OAuthToken::find_by_secret(conn, &secret)?.unwrap();
                let connection = campfire_db::models::oauth::OAuthConnection::find(
                    conn,
                    token.oauth_connection_id,
                )?
                .unwrap();
                assert_eq!(token.expires_at, connection.expires_at);
                Ok(())
            })
            .await
            .unwrap();
    }
    f.clock
        .set(deadline.jiff() + jiff::SignedDuration::from_secs(1));
    invalid(
        refresh(
            &f.app,
            near.json()["refresh_token"].as_str().unwrap(),
            CLIENT,
            RESOURCE,
            false,
        )
        .await,
    );
    assert_eq!(
        mcp(&f.app, near.json()["access_token"].as_str().unwrap())
            .await
            .status
            .as_u16(),
        401
    );
}

#[tokio::test]
async fn oauth_chatgpt_published_method_array_selects_none() {
    let document = json!({"client_id":"https://chatgpt.com/oauth/client.json","client_uri":"https://chatgpt.com/","redirect_uris":["https://chatgpt.com/connector_platform_oauth_redirect"],"token_endpoint_auth_method":"private_key_jwt","token_endpoint_auth_methods_supported":["none","private_key_jwt"],"grant_types":["authorization_code","refresh_token"],"response_types":["code"],"client_name":"ChatGPT","logo_uri":"https://persistent.oaistatic.com/sonic/misc/openai-logo.png","token_endpoint_auth_signing_alg":"RS256","jwks_uri":"https://chatgpt.com/oauth/jwks.json"});
    let f = fixture(document).await;
    let mut owner = f.app.sign_in(DAVID).await;
    let client = "https://chatgpt.com/oauth/client.json";
    let redirect = "https://chatgpt.com/connector_platform_oauth_redirect";
    let code = approve(&mut owner, client, redirect).await;
    assert_eq!(
        exchange(&f.app, &code, client, redirect, VERIFIER, RESOURCE)
            .await
            .status
            .as_u16(),
        200
    );
}
#[tokio::test]
async fn oauth_unsupported_only_client_and_untrusted_redirect_use_error_page() {
    let mut document = metadata();
    document["token_endpoint_auth_method"] = json!("client_secret_basic");
    let f = fixture(document).await;
    for redirect in [REDIRECT, "https://attacker.example/callback"] {
        let reply = f
            .app
            .anonymous()
            .get(&authorize_url(CLIENT, redirect))
            .await;
        assert!(reply.location().unwrap().contains("/app/oauth/error"));
        assert!(!reply.location().unwrap().contains("attacker.example"));
    }
}
#[tokio::test]
async fn oauth_internal_cimd_dns_answers_refused_without_dialing() {
    let f = fixture(metadata()).await;
    for ip in [
        "127.0.0.1",
        "10.0.0.1",
        "169.254.169.254",
        "192.168.1.1",
        "100.64.0.1",
        "::1",
        "fd00::1",
        "fe80::1",
    ] {
        f.resolver
            .set("client.example", vec![vec![ip.parse().unwrap()]]);
        let reply = f
            .app
            .anonymous()
            .get(&authorize_url(CLIENT, REDIRECT))
            .await;
        assert!(
            reply.location().unwrap().contains("/app/oauth/error"),
            "{ip}"
        );
    }
    assert!(f.dialer.dialed.lock().unwrap().is_empty());
}

#[tokio::test]
async fn oauth_consent_owner_csrf_policy_deny_and_single_approval() {
    let f = fixture(metadata()).await;
    let mut owner = f.app.sign_in(DAVID).await;
    let reply = owner.get(&authorize_url(CLIENT, REDIRECT)).await;
    let path = consent_path(&reply);
    let mut other = f.app.sign_in(super::presenters::test_support::JASON).await;
    other.grant_sudo().await;
    assert_eq!(other.get(&path).await.status.as_u16(), 404);
    let request = |body: &Value| Req::new(Method::POST, &format!("{path}/approve")).json_body(body);
    let input = json!({"name":"Consent test","roomIds":[],"capabilities":[]});
    owner.grant_sudo().await;
    assert_eq!(owner.send(request(&input)).await.status.as_u16(), 422);
    assert_eq!(other.write(request(&input)).await.status.as_u16(), 404);
    assert_eq!(
        owner
            .write(request(
                &json!({"name":"Consent test","roomIds":[],"capabilities":["dm_anyone"]})
            ))
            .await
            .status
            .as_u16(),
        422
    );
    let no_rows = f
        .app
        .db()
        .read(|conn| {
            Ok(
                conn.query_row("SELECT COUNT(*) FROM oauth_connections", [], |r| {
                    r.get::<_, i64>(0)
                })?,
            )
        })
        .await
        .unwrap();
    assert_eq!(no_rows, 0);
    let denied = owner
        .write(Req::new(Method::POST, &format!("{path}/deny")).json_body(&json!({})))
        .await;
    assert_eq!(denied.status.as_u16(), 200);
    let url = url::Url::parse(denied.json()["redirectTo"].as_str().unwrap()).unwrap();
    let query = url
        .query_pairs()
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(query.get("error").unwrap(), "access_denied");
    assert_eq!(query.get("iss").unwrap(), "https://smartfire.example");
    assert_eq!(query.get("state").unwrap(), "opaque state & value");
    assert_eq!(owner.write(request(&input)).await.status.as_u16(), 404);
}

#[tokio::test]
async fn oauth_revoke_access_only_itself_refresh_entire_connection_and_unknown_always_200() {
    let f = fixture(metadata()).await;
    let original = tokens(&f).await;
    let rotated = refresh(
        &f.app,
        original["refresh_token"].as_str().unwrap(),
        CLIENT,
        RESOURCE,
        false,
    )
    .await
    .json();
    for secret in ["unknown", original["access_token"].as_str().unwrap()] {
        let reply = f
            .app
            .anonymous()
            .send(
                Req::new(Method::POST, "/oauth/revoke")
                    .form(&[("client_id", CLIENT), ("token", secret)]),
            )
            .await;
        assert_eq!(reply.status.as_u16(), 200);
        assert_eq!(reply.header("access-control-allow-origin"), Some("*"));
    }
    assert_eq!(
        mcp(&f.app, original["access_token"].as_str().unwrap())
            .await
            .status
            .as_u16(),
        401
    );
    assert_eq!(
        mcp(&f.app, rotated["access_token"].as_str().unwrap())
            .await
            .status
            .as_u16(),
        200
    );
    assert_eq!(
        f.app
            .anonymous()
            .send(Req::new(Method::POST, "/oauth/revoke").form(&[
                ("client_id", CLIENT),
                ("token", rotated["refresh_token"].as_str().unwrap())
            ]))
            .await
            .status
            .as_u16(),
        200
    );
    assert_eq!(
        mcp(&f.app, rotated["access_token"].as_str().unwrap())
            .await
            .status
            .as_u16(),
        401
    );
    invalid(
        refresh(
            &f.app,
            rotated["refresh_token"].as_str().unwrap(),
            CLIENT,
            RESOURCE,
            false,
        )
        .await,
    );
}

#[tokio::test]
async fn oauth_trusted_authorize_errors_include_state_issuer_and_no_pending_row() {
    let f = fixture(metadata()).await;
    for (key, value, error) in [
        ("response_type", "token", "unsupported_response_type"),
        ("code_challenge_method", "plain", "invalid_request"),
        (
            "resource",
            "https://wrong.example/agents/mcp",
            "invalid_target",
        ),
        ("scope", "agent admin", "invalid_scope"),
    ] {
        let mut url = url::Url::parse(&format!(
            "http://campfire.test{}",
            authorize_url(CLIENT, REDIRECT)
        ))
        .unwrap();
        let mut params = url.query_pairs().into_owned().collect::<Vec<_>>();
        for pair in &mut params {
            if pair.0 == key {
                pair.1 = value.into();
            }
        }
        url.set_query(None);
        url.query_pairs_mut().extend_pairs(params);
        let reply = f
            .app
            .anonymous()
            .get(&format!("/oauth/authorize?{}", url.query().unwrap()))
            .await;
        let callback = url::Url::parse(reply.location().unwrap()).unwrap();
        assert_eq!(callback.host_str(), Some("client.example"));
        let query = callback
            .query_pairs()
            .collect::<std::collections::HashMap<_, _>>();
        assert_eq!(query.get("error").unwrap(), error);
        assert_eq!(query.get("state").unwrap(), "opaque state & value");
        assert_eq!(query.get("iss").unwrap(), "https://smartfire.example");
    }
    assert_eq!(
        f.app
            .db()
            .read(|conn| Ok(conn.query_row(
                "SELECT COUNT(*) FROM oauth_authorizations",
                [],
                |r| r.get::<_, i64>(0)
            )?))
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn oauth_cimd_redirect_size_type_and_identity_limits_and_cache_refetch() {
    let f = fixture(metadata()).await;
    for route in [
        Route::new("GET","client.example","/metadata.json",302).header("Location","https://client.example/other"),
        Route::new("GET","client.example","/metadata.json",200).header("Content-Type","text/plain").body(serde_json::to_vec(&metadata()).unwrap()),
        Route::new("GET","client.example","/metadata.json",200).header("Content-Type","application/json").body(vec![b' ';16385]),
        Route::new("GET","client.example","/metadata.json",200).header("Content-Type","application/json").body(br#"{"client_id":"https://other.example/metadata","client_name":"Invalid","redirect_uris":["https://client.example/callback"],"token_endpoint_auth_method":"none"}"#.to_vec()),
    ] {
        f.server.replace_routes(vec![route]);
        let reply=f.app.anonymous().get(&authorize_url(CLIENT,REDIRECT)).await;
        assert!(reply.location().unwrap().contains("/app/oauth/error"));
    }
    f.server.replace_routes(vec![
        Route::new("GET", "client.example", "/metadata.json", 200)
            .header("Content-Type", "application/json")
            .header("Cache-Control", "max-age=1")
            .body(serde_json::to_vec(&metadata()).unwrap()),
    ]);
    let mut owner = f.app.sign_in(DAVID).await;
    assert!(
        owner
            .get(&authorize_url(CLIENT, REDIRECT))
            .await
            .location()
            .unwrap()
            .contains("/app/oauth/consent/")
    );
    let fetched = f.server.received.lock().unwrap().len();
    f.clock.advance(jiff::SignedDuration::from_secs(299));
    owner.get(&authorize_url(CLIENT, REDIRECT)).await;
    assert_eq!(f.server.received.lock().unwrap().len(), fetched);
    f.clock.advance(jiff::SignedDuration::from_secs(2));
    owner.get(&authorize_url(CLIENT, REDIRECT)).await;
    assert_eq!(f.server.received.lock().unwrap().len(), fetched + 1);
}

#[tokio::test]
async fn oauth_cimd_mixed_private_public_answer_is_refused_and_blocked_client_stays_blocked() {
    let f = fixture(metadata()).await;
    f.resolver.set(
        "client.example",
        vec![vec![
            "10.0.0.1".parse().unwrap(),
            "93.184.216.34".parse().unwrap(),
        ]],
    );
    assert!(
        f.app
            .anonymous()
            .get(&authorize_url(CLIENT, REDIRECT))
            .await
            .location()
            .unwrap()
            .contains("/app/oauth/error")
    );
    assert!(f.dialer.dialed.lock().unwrap().is_empty());
    f.resolver.set(
        "client.example",
        vec![vec!["93.184.216.34".parse().unwrap()]],
    );
    let pair = tokens(&f).await;
    f.app
        .db()
        .write(|tx| {
            tx.conn()
                .execute("UPDATE oauth_clients SET blocked_at=?", [tx.now()])?;
            Ok(())
        })
        .await
        .unwrap();
    assert!(
        f.app
            .anonymous()
            .get(&authorize_url(CLIENT, REDIRECT))
            .await
            .location()
            .unwrap()
            .contains("/app/oauth/error")
    );
    assert_eq!(
        mcp(&f.app, pair["access_token"].as_str().unwrap())
            .await
            .status
            .as_u16(),
        401
    );
    invalid(
        refresh(
            &f.app,
            pair["refresh_token"].as_str().unwrap(),
            CLIENT,
            RESOURCE,
            false,
        )
        .await,
    );
}

#[tokio::test]
async fn oauth_expired_code_and_idle_refresh_fail_and_refresh_resource_can_be_omitted() {
    let f = fixture(metadata()).await;
    let mut owner = f.app.sign_in(DAVID).await;
    let code = approve(&mut owner, CLIENT, REDIRECT).await;
    f.clock.advance(jiff::SignedDuration::from_secs(61));
    invalid(exchange(&f.app, &code, CLIENT, REDIRECT, VERIFIER, RESOURCE).await);
    let pair = tokens(&f).await;
    let reply = f
        .app
        .anonymous()
        .send(Req::new(Method::POST, "/oauth/token").form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", pair["refresh_token"].as_str().unwrap()),
            ("client_id", CLIENT),
        ]))
        .await;
    assert_eq!(reply.status.as_u16(), 200);
    f.clock.advance(jiff::SignedDuration::from_hours(30 * 24));
    invalid(
        refresh(
            &f.app,
            reply.json()["refresh_token"].as_str().unwrap(),
            CLIENT,
            RESOURCE,
            false,
        )
        .await,
    );
}
#[tokio::test]
async fn oauth_changed_app_url_rejects_refresh_without_consuming_it() {
    let f = fixture(metadata()).await;
    let pair = tokens(&f).await;
    let app = f
        .app
        .restart(
            f.clock,
            &[
                ("APP_URL", "https://changed.example"),
                ("SMARTFIRE_MCP_OAUTH", "1"),
            ],
        )
        .await
        .without_job_runner()
        .await;
    invalid(
        refresh(
            &app,
            pair["refresh_token"].as_str().unwrap(),
            CLIENT,
            RESOURCE,
            false,
        )
        .await,
    );
    let secret = pair["refresh_token"].as_str().unwrap().to_string();
    app.db()
        .read(move |conn| {
            assert!(
                campfire_db::models::oauth::OAuthToken::find_by_secret(conn, &secret)?
                    .unwrap()
                    .used_at
                    .is_none()
            );
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn oauth_token_and_discovery_cors_and_token_rate_limit() {
    let app = setup(true).await;
    for path in [
        "/.well-known/oauth-authorization-server",
        "/.well-known/oauth-protected-resource/agents/mcp",
        "/oauth/token",
        "/oauth/revoke",
    ] {
        let reply = app
            .anonymous()
            .send(Req::new(Method::OPTIONS, path).header("origin", "https://client.example"))
            .await;
        assert_eq!(reply.status.as_u16(), 204);
        assert_eq!(reply.header("access-control-allow-origin"), Some("*"));
        assert_eq!(reply.header("access-control-allow-credentials"), None);
    }
    let resource = app
        .anonymous()
        .get("/.well-known/oauth-protected-resource")
        .await
        .json();
    assert_eq!(resource["resource"], RESOURCE);
    assert_eq!(
        resource["authorization_servers"],
        json!(["https://smartfire.example"])
    );
    assert_eq!(resource["bearer_methods_supported"], json!(["header"]));
    for _ in 0..60 {
        invalid(refresh(&app, "invalid", CLIENT, RESOURCE, false).await);
    }
    assert_eq!(
        refresh(&app, "invalid", CLIENT, RESOURCE, false)
            .await
            .status
            .as_u16(),
        429
    );
}

#[tokio::test]
async fn oauth_consent_reuses_owned_agent_and_replaces_access_for_all_connections() {
    let f = fixture(metadata()).await;
    let _pair = tokens(&f).await;
    let mut owner = f.app.sign_in(DAVID).await;
    let next = owner.get(&authorize_url(CLIENT, REDIRECT)).await;
    let path = consent_path(&next);
    let consent = owner.get(&path).await.json();
    let agent_id = consent["selectedAgentId"].as_i64().unwrap();
    assert_eq!(consent["accessAppliesToAllConnections"], true);
    owner.grant_sudo().await;
    let approved=owner.write(Req::new(Method::POST,&format!("{path}/approve")).json_body(&json!({"agentId":agent_id,"roomIds":[super::presenters::test_support::ALL_TALK],"capabilities":["read_messages","post_messages"]}))).await;
    assert_eq!(approved.status.as_u16(), 200, "{}", approved.text());
    f.app.db().read(move |conn| {
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM oauth_connections c JOIN agent_credentials k ON k.id=c.agent_credential_id WHERE k.agent_id=?",[agent_id],|r|r.get::<_,i64>(0))?,2);
        let grants=campfire_db::AgentGrant::effective_for_agent(conn,agent_id)?;
        assert_eq!(grants.iter().map(|g|g.capability.as_str()).collect::<std::collections::BTreeSet<_>>(),["post_messages","read_messages"].into());
        Ok(())
    }).await.unwrap();
    let mut other = f.app.sign_in(super::presenters::test_support::JASON).await;
    let next = other.get(&authorize_url(CLIENT, REDIRECT)).await;
    let path = consent_path(&next);
    other.grant_sudo().await;
    let refused = other
        .write(
            Req::new(Method::POST, &format!("{path}/approve"))
                .json_body(&json!({"agentId":agent_id,"roomIds":[],"capabilities":[]})),
        )
        .await;
    assert_eq!(refused.status.as_u16(), 404);
}
#[tokio::test]
async fn oauth_consent_obeys_workspace_switch_even_for_existing_agent() {
    let f = fixture(metadata()).await;
    let _pair = tokens(&f).await;
    let mut owner = f.app.sign_in(DAVID).await;
    let next = owner.get(&authorize_url(CLIENT, REDIRECT)).await;
    let path = consent_path(&next);
    let agent_id = owner.get(&path).await.json()["selectedAgentId"]
        .as_i64()
        .unwrap();
    owner.grant_sudo().await;
    f.app.db().write(|tx|{tx.conn().execute("UPDATE accounts SET settings=json_set(COALESCE(settings,'{}'),'$.member_agents_enabled',json('false'))",[])?;Ok(())}).await.unwrap();
    let refused = owner
        .write(
            Req::new(Method::POST, &format!("{path}/approve"))
                .json_body(&json!({"agentId":agent_id,"roomIds":[],"capabilities":[]})),
        )
        .await;
    assert_eq!(refused.status.as_u16(), 403);
    assert_eq!(
        f.app
            .db()
            .read(|conn| Ok(
                conn.query_row("SELECT COUNT(*) FROM oauth_connections", [], |r| r
                    .get::<_, i64>(0))?
            ))
            .await
            .unwrap(),
        1
    );
}

fn authorize_with_state(redirect: &str, state: &str) -> String {
    let mut url = url::Url::parse(&format!(
        "http://campfire.test{}",
        authorize_url(CLIENT, redirect)
    ))
    .unwrap();
    let pairs = url
        .query_pairs()
        .into_owned()
        .map(|(key, value)| {
            let value = if key == "state" {
                state.to_string()
            } else {
                value
            };
            (key, value)
        })
        .collect::<Vec<_>>();
    url.set_query(None);
    url.query_pairs_mut().extend_pairs(pairs);
    format!("/oauth/authorize?{}", url.query().unwrap())
}

#[tokio::test]
async fn oauth_cookie_serialization_discards_empty_and_unknown_query_parameters() {
    for suffix in [
        "&".repeat(700),
        format!(
            "&ignored={}",
            url::form_urlencoded::byte_serialize("&<>\"'\\é".repeat(700).as_bytes())
                .collect::<String>()
        ),
    ] {
        let f = fixture(metadata()).await;
        let path = format!("{}{}", authorize_with_state(REDIRECT, ""), suffix);
        let _ = oauth_signed_out_to_consent(&f, &path, "").await;
    }
}

#[tokio::test]
async fn oauth_cookie_serialization_escaping_at_and_over_boundary() {
    for piece in [
        "&",
        "<",
        ">",
        "'",
        "\"",
        "\\",
        "é",
        "🙂",
        "%26%3C%5C",
        "\u{2028}",
        "&<>'\"\\é🙂%26\u{2028}",
    ] {
        let f = fixture(metadata()).await;
        let state = oauth_state_at_return_json_limit(piece);
        let path = format!(
            "{}{}&ignored=%26%3c%3e%22%5c%F0%9F%99%82",
            authorize_with_state(REDIRECT, &state)
                .replace("%3A", "%3a")
                .replace("%3C", "%3c")
                .replace("%3E", "%3e")
                .replace("%5C", "%5c"),
            "&".repeat(700),
        );
        let _ = oauth_signed_out_to_consent(&f, &path, &state).await;
        let over = format!("{state}a");
        assert_eq!(oauth_return_json_size(&over), 1025);
        let reply = f
            .app
            .anonymous()
            .get(&authorize_with_state(REDIRECT, &over))
            .await;
        assert_oauth_cookie_budget(&reply);
        assert_eq!(reply.status.as_u16(), 302);
        let callback = url::Url::parse(reply.location().unwrap()).unwrap();
        assert_eq!(
            callback.host_str(),
            Some("client.example"),
            "escaping input {piece:?}"
        );
        assert_eq!(callback.path(), "/callback");
        let query = callback
            .query_pairs()
            .collect::<std::collections::HashMap<_, _>>();
        assert_eq!(query.get("error").unwrap(), "invalid_request");
        assert_eq!(query.get("state").unwrap(), &over);
        assert_eq!(query.get("iss").unwrap(), "https://smartfire.example");
        let untrusted = f
            .app
            .anonymous()
            .get(&authorize_with_state(
                "https://attacker.example/callback",
                &over,
            ))
            .await;
        assert_oauth_cookie_budget(&untrusted);
        assert_eq!(untrusted.status.as_u16(), 302);
        assert_eq!(
            url::Url::parse(untrusted.location().unwrap())
                .unwrap()
                .path(),
            "/app/oauth/error"
        );
    }
}

#[tokio::test]
async fn oauth_cookie_serialization_overflow_with_existing_session_redirects() {
    use campfire_kit::Crypto;
    let f = fixture(metadata()).await;
    let crypto = campfire_kit::RailsCrypto::new(f.app.booted.app.secrets.clone());
    let key = campfire_kit::session::SESSION_KEY;
    let raw = crypto.encrypt_cookie(
        key,
        &json!({"session_id":"a".repeat(32),"existing":"a".repeat(1800)}),
        None,
    );
    let cookie = format!("{key}={}", campfire_kit::cookies::escape(&raw));
    assert!(
        cookie.len() + 100 < 4096,
        "incoming session must fit: {} bytes",
        cookie.len()
    );
    let mut browser = f.app.anonymous();
    browser.absorb_cookie_header(&cookie);
    let state = "overflow state & < >";
    let reply = browser.get(&authorize_with_state(REDIRECT, state)).await;
    assert_oauth_cookie_budget(&reply);
    assert_eq!(reply.status.as_u16(), 302);
    let callback = url::Url::parse(reply.location().unwrap()).unwrap();
    assert_eq!(callback.host_str(), Some("client.example"));
    assert_eq!(callback.path(), "/callback");
    let query = callback
        .query_pairs()
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(query.get("error").unwrap(), "invalid_request");
    assert_eq!(query.get("state").unwrap(), state);
    assert_eq!(query.get("iss").unwrap(), "https://smartfire.example");
}

#[tokio::test]
async fn oauth_return_url_budget_rejects_one_byte_over_boundary() {
    let f = fixture(metadata()).await;
    let state = format!("{}a", oauth_state_at_return_json_limit("a"));
    let path = authorize_with_state(REDIRECT, &state);
    assert_eq!(oauth_return_json_size(&state), 1025);
    let reply = f.app.anonymous().get(&path).await;
    assert_eq!(reply.status.as_u16(), 302);
    let callback = url::Url::parse(reply.location().unwrap()).unwrap();
    assert_eq!(callback.host_str(), Some("client.example"));
    assert_eq!(callback.path(), "/callback");
    let query = callback
        .query_pairs()
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(query.get("error").unwrap(), "invalid_request");
    assert_eq!(query.get("state").unwrap(), &state);
    assert_eq!(query.get("iss").unwrap(), "https://smartfire.example");
}

#[tokio::test]
async fn oauth_return_url_budget_rejects_previous_2048_byte_boundary() {
    let f = fixture(metadata()).await;
    let state = "a".repeat(1724);
    let path = authorize_with_state(REDIRECT, &state);
    assert_eq!(format!("http://campfire.test{path}").len(), 2048);
    let mut browser = f.app.anonymous();
    let reply = browser.get(&path).await;
    assert_eq!(reply.status.as_u16(), 302);
    let callback = url::Url::parse(reply.location().unwrap()).unwrap();
    // Include the reported downstream failure in diagnostics if this regresses.
    if callback.path() == "/app/session/new" {
        let page = browser.get("/app/session/new").await;
        assert_eq!(page.status.as_u16(), 200, "{}", page.text());
    }
    assert_eq!(callback.host_str(), Some("client.example"));
    assert_eq!(callback.path(), "/callback");
    let query = callback
        .query_pairs()
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(query.get("error").unwrap(), "invalid_request");
    assert_eq!(query.get("state").unwrap(), &state);
    assert_eq!(query.get("iss").unwrap(), "https://smartfire.example");
}

#[tokio::test]
async fn oauth_review_long_encoded_return_url_redirects_without_overflowing_signed_out_cookie() {
    let f = fixture(metadata()).await;
    let state = "/".repeat(1400);
    let path = authorize_with_state(REDIRECT, &state);
    assert!(path.len() > 4500);
    let mut browser = f.app.anonymous();
    let reply = browser.get(&path).await;
    assert_eq!(
        reply.status.as_u16(),
        302,
        "long state must return an OAuth error, not a cookie overflow: {}",
        reply.text()
    );
    let callback = url::Url::parse(reply.location().unwrap()).unwrap();
    assert_eq!(callback.host_str(), Some("client.example"));
    assert_eq!(callback.path(), "/callback");
    let query = callback
        .query_pairs()
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(query.get("error").unwrap(), "invalid_request");
    assert_eq!(query.get("state").unwrap(), &state);
    assert_eq!(query.get("iss").unwrap(), "https://smartfire.example");
    assert!(browser.cookie_header().len() < 4096);
    assert_eq!(
        f.app
            .db()
            .read(|conn| Ok(conn.query_row(
                "SELECT COUNT(*) FROM oauth_authorizations",
                [],
                |r| r.get::<_, i64>(0)
            )?))
            .await
            .unwrap(),
        0
    );
    let untrusted = browser
        .get(&authorize_with_state(
            "https://attacker.example/callback",
            &state,
        ))
        .await;
    assert_eq!(untrusted.status.as_u16(), 302);
    assert!(
        url::Url::parse(untrusted.location().unwrap())
            .unwrap()
            .path()
            .starts_with("/app/oauth/error")
    );
    let shorter = browser
        .get(&authorize_with_state(REDIRECT, &"/".repeat(100)))
        .await;
    assert_eq!(shorter.status.as_u16(), 302);
    assert!(shorter.location().unwrap().contains("/app/session/new"));
    assert_eq!(browser.get("/app/session/new").await.status.as_u16(), 200);
}

#[tokio::test]
async fn oauth_review_code_exchange_allows_omitted_redirect_uri_and_binds_present_values() {
    let f = fixture(metadata()).await;
    let mut owner = f.app.sign_in(DAVID).await;
    for redirect in [None, Some(REDIRECT)] {
        let code = approve(&mut owner, CLIENT, REDIRECT).await;
        // Supplied redirects still match exactly, including a supplied empty value.
        for wrong in ["", "https://client.example/wrong"] {
            invalid(exchange(&f.app, &code, CLIENT, wrong, VERIFIER, RESOURCE).await);
        }
        let mut pairs = vec![
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("client_id", CLIENT),
            ("code_verifier", VERIFIER),
            ("resource", RESOURCE),
        ];
        if let Some(redirect) = redirect {
            pairs.push(("redirect_uri", redirect));
        }
        for (field, value) in [
            ("code", "unknown"),
            ("client_id", "https://wrong.example/client.json"),
            ("resource", "https://wrong.example/agents/mcp"),
            ("code_verifier", "wrong"),
        ] {
            let mut invalid_pairs = pairs.clone();
            for (key, parameter) in &mut invalid_pairs {
                if *key == field {
                    *parameter = value;
                }
            }
            invalid(
                f.app
                    .anonymous()
                    .send(Req::new(Method::POST, "/oauth/token").form(&invalid_pairs))
                    .await,
            );
        }
        let reply = f
            .app
            .anonymous()
            .send(Req::new(Method::POST, "/oauth/token").form(&pairs))
            .await;
        assert_eq!(
            reply.status.as_u16(),
            200,
            "omitted or matching redirect_uri must exchange the code: {}",
            reply.text()
        );
        assert_eq!(reply.json()["token_type"], "Bearer");
        assert_eq!(
            mcp(&f.app, reply.json()["access_token"].as_str().unwrap())
                .await
                .status
                .as_u16(),
            200
        );
    }
}

#[tokio::test]
async fn oauth_review_disabled_personal_agents_redirect_access_denied_to_trusted_client() {
    let f = fixture(metadata()).await;
    f.app.db().write(|tx| {
        tx.conn().execute("UPDATE accounts SET settings=json_set(COALESCE(settings,'{}'),'$.member_agents_enabled',json('false'))", [])?;
        Ok(())
    }).await.unwrap();
    let mut owner = f.app.sign_in(DAVID).await;
    let reply = owner.get(&authorize_url(CLIENT, REDIRECT)).await;
    assert_eq!(
        reply.status.as_u16(),
        302,
        "workspace refusal must complete the OAuth callback: {}",
        reply.text()
    );
    let callback = url::Url::parse(reply.location().unwrap()).unwrap();
    assert_eq!(callback.host_str(), Some("client.example"));
    assert_eq!(callback.path(), "/callback");
    let query = callback
        .query_pairs()
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(query.get("error").unwrap(), "access_denied");
    assert_eq!(
        query.get("error_description").unwrap(),
        "Personal agents are turned off for this workspace."
    );
    assert_eq!(query.get("state").unwrap(), "opaque state & value");
    assert_eq!(query.get("iss").unwrap(), "https://smartfire.example");
    assert_eq!(
        f.app
            .db()
            .read(|conn| Ok(conn.query_row(
                "SELECT COUNT(*) FROM oauth_authorizations",
                [],
                |r| r.get::<_, i64>(0)
            )?))
            .await
            .unwrap(),
        0
    );
}

fn registration() -> Value {
    json!({"client_name":"Registered client","redirect_uris":[REDIRECT,"http://127.0.0.1/callback","http://[::1]/callback"],"token_endpoint_auth_method":"none"})
}
async fn register(app: &TestApp, input: &Value, ip: &str) -> Reply {
    app.anonymous()
        .send(
            Req::new(Method::POST, "/oauth/register")
                .header("x-forwarded-for", ip)
                .header("origin", "https://connector.example")
                .json_body(input),
        )
        .await
}

#[tokio::test]
async fn oauth_dcr_public_client_completes_pkce_refresh_and_blocking() {
    let f = fixture(metadata()).await;
    let registered = register(&f.app, &registration(), "203.0.113.7").await;
    assert_eq!(registered.status.as_u16(), 201, "{}", registered.text());
    let body = registered.json();
    let client_id = body["client_id"].as_str().unwrap().to_string();
    assert!(!client_id.starts_with("https:"));
    assert_eq!(body["client_id_issued_at"], f.clock.now().as_second());
    assert_eq!(body["client_name"], "Registered client");
    assert_eq!(body["redirect_uris"], registration()["redirect_uris"]);
    assert_eq!(body["token_endpoint_auth_method"], "none");
    assert_eq!(
        body["grant_types"],
        json!(["authorization_code", "refresh_token"])
    );
    assert_eq!(body["response_types"], json!(["code"]));
    assert_eq!(body["scope"], "agent");
    assert!(body.get("client_secret").is_none());
    assert_eq!(registered.header("access-control-allow-origin"), Some("*"));
    assert_eq!(registered.header("access-control-allow-credentials"), None);
    assert_eq!(registered.header("set-cookie"), None);
    assert_eq!(registered.header("cache-control"), Some("no-store"));
    let id = client_id.clone();
    let stored = f
        .app
        .db()
        .read(move |conn| {
            Ok(campfire_db::models::oauth::OAuthClient::find_by_client_id(conn, &id)?.unwrap())
        })
        .await
        .unwrap();
    assert_eq!(stored.registration, "dcr");
    assert_eq!(stored.auth_methods, r#"["none"]"#);
    assert_eq!(
        serde_json::from_str::<Value>(stored.metadata.as_deref().unwrap()).unwrap(),
        body
    );
    assert!(stored.fetched_at.is_none());
    assert!(stored.refetch_after.is_none());

    let mut browser = f.app.sign_in(DAVID).await;
    let mismatch = browser
        .get(&authorize_url(&client_id, "https://client.example/other"))
        .await;
    assert!(
        mismatch
            .location()
            .unwrap()
            .ends_with("/app/oauth/error?error=invalid_redirect_uri")
    );
    // The DCR resolver never performs a metadata fetch, even beyond the CIMD cache lifetime.
    f.clock.advance(jiff::SignedDuration::from_secs(601));
    let code = approve(&mut browser, &client_id, REDIRECT).await;
    invalid(
        exchange(
            &f.app,
            &code,
            &client_id,
            REDIRECT,
            "wrong-verifier",
            RESOURCE,
        )
        .await,
    );
    let pair = exchange(&f.app, &code, &client_id, REDIRECT, VERIFIER, RESOURCE).await;
    assert_eq!(pair.status.as_u16(), 200, "{}", pair.text());
    let pair = pair.json();
    assert_eq!(
        mcp(&f.app, pair["access_token"].as_str().unwrap())
            .await
            .status
            .as_u16(),
        200
    );
    invalid(
        refresh(
            &f.app,
            pair["refresh_token"].as_str().unwrap(),
            &client_id,
            RESOURCE,
            true,
        )
        .await,
    );
    let rotated = refresh(
        &f.app,
        pair["refresh_token"].as_str().unwrap(),
        &client_id,
        RESOURCE,
        false,
    )
    .await;
    assert_eq!(rotated.status.as_u16(), 200, "{}", rotated.text());
    let rotated = rotated.json();
    let code = approve(&mut browser, &client_id, "http://[::1]:54321/callback").await;
    assert_eq!(
        exchange(
            &f.app,
            &code,
            &client_id,
            "http://[::1]:54321/callback",
            VERIFIER,
            RESOURCE
        )
        .await
        .status
        .as_u16(),
        200
    );
    assert!(f.server.received.lock().unwrap().is_empty());
    assert!(f.resolver.lookups().is_empty());
    f.app
        .db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE oauth_clients SET blocked_at=? WHERE id=?",
                params![tx.now(), stored.id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        mcp(&f.app, rotated["access_token"].as_str().unwrap())
            .await
            .status
            .as_u16(),
        401
    );
    invalid(
        refresh(
            &f.app,
            rotated["refresh_token"].as_str().unwrap(),
            &client_id,
            RESOURCE,
            false,
        )
        .await,
    );
    let blocked = browser.get(&authorize_url(&client_id, REDIRECT)).await;
    assert!(
        blocked
            .location()
            .unwrap()
            .ends_with("/app/oauth/error?error=invalid_client")
    );
}

#[tokio::test]
async fn oauth_dcr_rejects_confidential_other_and_missing_methods() {
    let app = setup(true).await;
    for (index, method) in [
        json!("client_secret_basic"),
        json!("client_secret_post"),
        json!("private_key_jwt"),
        json!("unknown"),
        json!(null),
        json!(["none"]),
    ]
    .into_iter()
    .enumerate()
    {
        let mut input = registration();
        input["token_endpoint_auth_method"] = method;
        let reply = register(&app, &input, &format!("203.0.113.{}", index + 1)).await;
        assert_eq!(reply.status.as_u16(), 400, "{}", reply.text());
        assert_eq!(reply.json()["error"], "invalid_client_metadata");
        assert_eq!(reply.header("access-control-allow-origin"), Some("*"));
    }
    let mut input = registration();
    input
        .as_object_mut()
        .unwrap()
        .remove("token_endpoint_auth_method");
    let reply = register(&app, &input, "203.0.113.20").await;
    assert_eq!(reply.status.as_u16(), 400);
    assert_eq!(reply.json()["error"], "invalid_client_metadata");
    assert_eq!(
        app.db()
            .read(|conn| Ok(
                conn.query_row("SELECT COUNT(*) FROM oauth_clients", [], |r| r
                    .get::<_, i64>(0))?
            ))
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn oauth_dcr_validates_redirects_names_and_supported_metadata() {
    let app = setup(true).await;
    let bad_redirects = [
        json!([]),
        json!(null),
        json!("https://client.example/callback"),
        json!(["http://client.example/callback"]),
        json!(["http://localhost/callback"]),
        json!(["http://192.168.1.1/callback"]),
        json!(["ftp://client.example/callback"]),
        json!(["https://client.example/callback#fragment"]),
        json!(["https://user:secret@client.example/callback"]),
        json!([REDIRECT, "http://bad.example/callback"]),
    ];
    for (index, redirects) in bad_redirects.into_iter().enumerate() {
        let mut input = registration();
        input["redirect_uris"] = redirects;
        let reply = register(&app, &input, &format!("203.0.113.{}", index + 1)).await;
        assert_eq!(reply.status.as_u16(), 400, "{}", reply.text());
        assert_eq!(reply.json()["error"], "invalid_redirect_uri");
    }
    let mut input = registration();
    input.as_object_mut().unwrap().remove("redirect_uris");
    let reply = register(&app, &input, "203.0.113.30").await;
    assert_eq!(reply.status.as_u16(), 400);
    assert_eq!(reply.json()["error"], "invalid_redirect_uri");
    for (index, name) in [
        json!(""),
        json!("   "),
        json!(null),
        json!(42),
        json!("x".repeat(101)),
        json!("é".repeat(101)),
    ]
    .into_iter()
    .enumerate()
    {
        let mut input = registration();
        input["client_name"] = name;
        let reply = register(&app, &input, &format!("203.0.113.{}", index + 50)).await;
        assert_eq!(reply.status.as_u16(), 400, "{}", reply.text());
        assert_eq!(reply.json()["error"], "invalid_client_metadata");
    }
    let mut input = registration();
    input.as_object_mut().unwrap().remove("client_name");
    let reply = register(&app, &input, "203.0.113.70").await;
    assert_eq!(reply.status.as_u16(), 400);
    assert_eq!(reply.json()["error"], "invalid_client_metadata");
    for (key, value) in [
        ("grant_types", json!(["client_credentials"])),
        ("response_types", json!(["token"])),
        ("scope", json!("admin")),
        ("client_uri", json!("javascript:alert(1)")),
    ] {
        let mut input = registration();
        input[key] = value;
        let reply = register(&app, &input, "203.0.113.80").await;
        assert_eq!(reply.status.as_u16(), 400);
        assert_eq!(reply.json()["error"], "invalid_client_metadata");
    }
    let mut input = registration();
    input["client_name"] = json!("é".repeat(100));
    input["redirect_uris"] = json!([
        "http://127.0.0.2:56789/callback",
        "http://[::1]:12345/callback"
    ]);
    input["client_uri"] = json!("https://client.example/");
    input["ignored_extension"] = json!("ignored");
    let reply = register(&app, &input, "203.0.113.90").await;
    assert_eq!(reply.status.as_u16(), 201, "{}", reply.text());
    assert_eq!(reply.json()["client_name"], input["client_name"]);
    assert_eq!(reply.json()["client_uri"], input["client_uri"]);
    assert!(reply.json().get("ignored_extension").is_none());
}

#[tokio::test]
async fn oauth_dcr_rate_limit_is_ten_per_hour_per_ip_and_counts_invalid_requests() {
    let clock = Arc::new(FrozenClock::new("2026-10-13T16:00:00Z".parse().unwrap()));
    let app = TestApp::boot_with_clock_and_env(
        clock.clone(),
        &[
            ("APP_URL", "https://smartfire.example"),
            ("SMARTFIRE_MCP_OAUTH", "1"),
        ],
    )
    .await
    .unwrap()
    .without_job_runner()
    .await;
    let mut ids = HashSet::new();
    for _ in 0..10 {
        let reply = register(&app, &registration(), "203.0.113.1").await;
        assert_eq!(reply.status.as_u16(), 201, "{}", reply.text());
        assert!(ids.insert(reply.json()["client_id"].as_str().unwrap().to_string()));
    }
    let reply = register(&app, &registration(), "203.0.113.1").await;
    assert_eq!(reply.status.as_u16(), 429);
    assert_eq!(reply.header("retry-after"), Some("3600"));
    assert_eq!(reply.header("access-control-allow-origin"), Some("*"));
    assert_eq!(reply.header("set-cookie"), None);
    assert_eq!(
        register(&app, &registration(), "203.0.113.2")
            .await
            .status
            .as_u16(),
        201
    );
    clock.advance(jiff::SignedDuration::from_secs(3599));
    assert_eq!(
        register(&app, &registration(), "203.0.113.1")
            .await
            .status
            .as_u16(),
        429
    );
    clock.advance(jiff::SignedDuration::from_secs(1));
    assert_eq!(
        register(&app, &registration(), "203.0.113.1")
            .await
            .status
            .as_u16(),
        201
    );
    for _ in 0..10 {
        assert_eq!(
            register(&app, &json!({}), "203.0.113.3")
                .await
                .status
                .as_u16(),
            400
        );
    }
    assert_eq!(
        register(&app, &registration(), "203.0.113.3")
            .await
            .status
            .as_u16(),
        429
    );
}

#[tokio::test]
async fn oauth_dcr_requires_bounded_json_and_preflight_sets_no_cookies() {
    let app = setup(true).await;
    for req in [
        Req::new(Method::POST, "/oauth/register").body(b"{}"),
        Req::new(Method::POST, "/oauth/register")
            .header("content-type", "application/json")
            .body(b"{"),
        Req::new(Method::POST, "/oauth/register")
            .header("content-type", "application/json")
            .body(vec![b' '; 16385]),
    ] {
        let reply = app.anonymous().send(req).await;
        assert_eq!(reply.status.as_u16(), 400, "{}", reply.text());
        assert_eq!(reply.json()["error"], "invalid_client_metadata");
        assert_eq!(reply.header("access-control-allow-origin"), Some("*"));
    }
    let mut browser = app.sign_in(DAVID).await;
    let reply = browser
        .send(
            Req::new(Method::OPTIONS, "/oauth/register")
                .header("origin", "https://connector.example"),
        )
        .await;
    assert_eq!(reply.status.as_u16(), 204);
    assert_eq!(reply.header("access-control-allow-origin"), Some("*"));
    assert_eq!(reply.header("access-control-allow-credentials"), None);
    assert_eq!(reply.header("set-cookie"), None);
    let reply = browser
        .send(Req::new(Method::POST, "/oauth/register").json_body(&registration()))
        .await;
    assert_eq!(reply.status.as_u16(), 201, "{}", reply.text());
    assert_eq!(reply.header("set-cookie"), None);
}

#[tokio::test]
async fn oauth_dcr_rejects_secret_fields_even_with_public_auth_method() {
    let app = setup(true).await;
    for (key, value) in [
        ("client_secret", json!("secret")),
        ("client_secret", json!("")),
        ("client_secret", json!(null)),
        ("client_secret_expires_at", json!(0)),
        ("client_secret_expires_at", json!(null)),
    ] {
        let mut input = registration();
        input[key] = value;
        let reply = register(&app, &input, "203.0.113.31").await;
        assert_eq!(reply.status.as_u16(), 400, "{key}: {}", reply.text());
        assert_eq!(reply.json()["error"], "invalid_client_metadata");
    }
    assert_eq!(
        app.db()
            .read(|conn| Ok(
                conn.query_row("SELECT COUNT(*) FROM oauth_clients", [], |r| r
                    .get::<_, i64>(0))?
            ))
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn oauth_dcr_requires_and_returns_the_fixed_grant_pair() {
    let app = setup(true).await;
    for (grants, status) in [
        (None, 201),
        (Some(json!(["authorization_code", "refresh_token"])), 201),
        (Some(json!(["authorization_code"])), 400),
        (Some(json!(["unknown"])), 400),
        (Some(json!(["refresh_token", "authorization_code"])), 201),
        (Some(json!([])), 400),
        (Some(json!(["refresh_token"])), 400),
        (
            Some(json!(["authorization_code", "authorization_code"])),
            400,
        ),
        (
            Some(json!(["authorization_code", "refresh_token", "unknown"])),
            400,
        ),
        (Some(json!(null)), 400),
    ] {
        let mut input = registration();
        if let Some(grants) = grants {
            input["grant_types"] = grants;
        }
        let reply = register(&app, &input, "203.0.113.32").await;
        assert_eq!(reply.status.as_u16(), status, "{input}: {}", reply.text());
        if status == 201 {
            assert_eq!(
                reply.json()["grant_types"],
                json!(["authorization_code", "refresh_token"])
            );
        } else {
            assert_eq!(reply.json()["error"], "invalid_client_metadata");
        }
    }
    assert_eq!(
        app.db()
            .read(|conn| Ok(
                conn.query_row("SELECT COUNT(*) FROM oauth_clients", [], |r| r
                    .get::<_, i64>(0))?
            ))
            .await
            .unwrap(),
        3
    );
}

#[path = "oauth_assertion_tests.rs"]
mod assertions;
