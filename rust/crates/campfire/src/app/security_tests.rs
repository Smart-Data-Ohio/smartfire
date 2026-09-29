//! The whole stack's security behavior against `vectors/kit_security.json`, which our Rails app
//! produced (`reference-tools/kit/security_vectors.rb`): the headers each class of route answers
//! with, over plain HTTP and behind TLS; the sign-in and CSP report rate limits; and the session
//! plumbing (idle expiry, the pending second factor, bouncing back after sign in, agent tokens).
//!
//! Each test boots over a fresh, empty database, as the vectors were made, so none needs the
//! parity seed.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use campfire_db::{Account, NewUser, PasswordDigest, Role, Session, User};
use campfire_kit::{Crypto, RailsCrypto};
use serde_json::{Value, json};
use tower::ServiceExt;

use super::*;
use crate::controllers::presenters::test_support::masked_session_token;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
const PASSWORD: &str = "secret123456";

fn vectors() -> Value {
    serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../vectors/kit_security.json"))).unwrap()
}

fn parity_env(name: &str) -> String {
    let env = std::fs::read_to_string(std::path::Path::new(ROOT).join("parity/.env.reference")).unwrap();
    env.lines().find_map(|line| line.strip_prefix(&format!("{name}=")).map(str::to_string)).unwrap()
}

struct Fresh {
    booted: Booted,
    _dir: tempfile::TempDir,
}

/// The app over an empty database, configured as the vectors' reference was: `DISABLE_SSL` for
/// plain HTTP; TLS assumed, with a LiveKit URL, otherwise.
async fn boot_fresh(ssl: bool) -> Fresh {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_string_lossy().into_owned();
    let secret = parity_env("SECRET_KEY_BASE");
    let config = Config::from_lookup(|name| match name {
        "SECRET_KEY_BASE" => Some(secret.clone()),
        "DISABLE_SSL" if !ssl => Some("true".into()),
        "LIVEKIT_URL" if ssl => Some("wss://livekit.campfire.test:7880".into()),
        "APP_VERSION" | "GIT_REVISION" => Some("parity".into()),
        "CAMPFIRE_STORAGE_PATH" => Some(root.clone()),
        _ => None,
    })
    .unwrap();
    Fresh { booted: boot(config).await.unwrap(), _dir: dir }
}

impl Fresh {
    /// `reset_database!` in reference-tools/support.rb: the account and David, an administrator.
    async fn seed(&self) -> User {
        let digest = PasswordDigest::hash(PASSWORD.into(), 4).await.unwrap();
        self.booted
            .app
            .db
            .write(move |tx| {
                Account::create(tx, "Campfire")?;
                let david = NewUser {
                    name: "David".into(),
                    email_address: Some("david@example.com".into()),
                    password_digest: Some(digest),
                    role: Role::Administrator,
                    bio: None,
                    bot_token: None,
                };
                User::create(tx, david)
            })
            .await
            .unwrap()
    }

    async fn send(&self, request: Request<Body>) -> Reply {
        let response = self.booted.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap().to_vec();
        Reply { status, headers, body }
    }

    fn crypto(&self) -> RailsCrypto {
        RailsCrypto::new(self.booted.app.secrets.clone())
    }

    /// The `_campfire_session` a response set, as sent back (still escaped).
    fn session_cookie(reply: &Reply) -> Option<String> {
        reply.set_cookies().iter().find_map(|cookie| cookie.split(';').next()?.strip_prefix("_campfire_session=").map(str::to_string))
    }

    fn session(&self, raw: &str) -> Value {
        let raw = percent_encoding::percent_decode_str(raw).decode_utf8_lossy();
        self.crypto().decrypt_cookie("_campfire_session", &raw, jiff::Timestamp::now()).unwrap()
    }

    fn session_token_cookie(&self, session: &Session) -> String {
        let signed = self.crypto().sign_cookie("session_token", &session.token, None);
        format!("session_token={}", campfire_kit::cookies::escape(&signed))
    }
}

struct Reply {
    status: StatusCode,
    headers: axum::http::HeaderMap,
    body: Vec<u8>,
}

impl Reply {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }

    fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    fn set_cookies(&self) -> Vec<String> {
        self.headers.get_all(header::SET_COOKIE).iter().map(|v| v.to_str().unwrap().to_string()).collect()
    }
}

fn request(method: &str, path: &str) -> axum::http::request::Builder {
    Request::builder().method(method).uri(path).header(header::HOST, "campfire.test")
}

fn form(pairs: &[(&str, &str)]) -> Body {
    let encode = |value: &str| percent_encoding::utf8_percent_encode(value, percent_encoding::NON_ALPHANUMERIC).to_string();
    Body::from(pairs.iter().map(|(k, v)| format!("{}={}", encode(k), encode(v))).collect::<Vec<_>>().join("&"))
}

// --- Headers per route class -------------------------------------------------------------------

/// The headers compared exactly, present or absent: the security headers (`default_headers`,
/// HSTS, the policy with its per-request nonce masked), the version headers and the caching
/// headers that differ between route classes.
const COMPARED: &[&str] = &[
    "x-frame-options",
    "x-xss-protection",
    "x-content-type-options",
    "x-permitted-cross-domain-policies",
    "referrer-policy",
    "permissions-policy",
    "strict-transport-security",
    "content-security-policy",
    "x-version",
    "x-rev",
    "cache-control",
    "location",
];

fn masked(name: &str, value: &str) -> String {
    if name == "content-security-policy" {
        let nonce = regex::Regex::new(r"'nonce-[^']*'").unwrap();
        nonce.replace_all(value, "'nonce-*'").into_owned()
    } else {
        value.to_string()
    }
}

fn compared(headers: impl Fn(&str) -> Option<String>) -> Vec<(String, Option<String>)> {
    COMPARED.iter().map(|name| (name.to_string(), headers(name).map(|value| masked(name, &value)))).collect()
}

async fn replay(app: &Fresh, name: &str, vector: &Value, session_cookie: Option<&str>, token: Option<&str>) -> Reply {
    let method = vector["method"].as_str().unwrap();
    let path = vector["path"].as_str().unwrap();
    let env = &vector["env"];
    let mut builder = request(method, path);
    if let Some(accept) = env["HTTP_ACCEPT"].as_str() {
        builder = builder.header(header::ACCEPT, accept);
    }
    if let Some(ip) = env["REMOTE_ADDR"].as_str() {
        builder = builder.header("x-forwarded-for", ip);
    }
    if vector["cookies"].as_array().is_some_and(|cookies| !cookies.is_empty()) {
        builder = builder.header(header::COOKIE, format!("_campfire_session={}", session_cookie.unwrap()));
    }
    let body = match (name, env["input"].as_str()) {
        (_, Some(input)) => {
            builder = builder.header(header::CONTENT_TYPE, env["CONTENT_TYPE"].as_str().unwrap());
            Body::from(input.to_string())
        }
        ("forgery" | "unauthorized_render", None) => {
            builder = builder.header(header::CONTENT_TYPE, "application/x-www-form-urlencoded");
            let mut pairs = vec![("email_address", "david@example.com"), ("password", "wrong")];
            if name == "unauthorized_render" {
                pairs.push(("authenticity_token", token.unwrap()));
            }
            form(&pairs)
        }
        _ => Body::empty(),
    };
    app.send(builder.body(body).unwrap()).await
}

async fn assert_headers_match(ssl: bool) {
    let vectors = vectors();
    let section = &vectors[if ssl { "headers_ssl" } else { "headers_plain" }]["requests"];
    let app = boot_fresh(ssl).await;
    let mut mismatches = Vec::new();
    let mut check = |name: &str, reply: &Reply| {
        let vector = &section[name];
        let expected = compared(|header| vector["headers"][header].as_str().map(str::to_string));
        let actual = compared(|header| reply.headers.get(header).map(|value| value.to_str().unwrap().to_string()));
        let status = vector["status"].as_u64().unwrap() as u16;
        if reply.status.as_u16() != status {
            mismatches.push(format!("{name}: status {} (reference {status})", reply.status));
        }
        for ((header, expected), (_, actual)) in expected.iter().zip(&actual) {
            if expected != actual {
                mismatches.push(format!("{name}: {header}: {actual:?} (reference {expected:?})"));
            }
        }
    };

    for name in ["html_page", "redirect", "health", "json", "javascript", "asset", "asset_head", "missing_asset", "public_file", "not_found", "csp_report"] {
        let reply = replay(&app, name, &section[name], None, None).await;
        check(name, &reply);
    }

    app.seed().await;
    let page = app.send(request("GET", "/session/new").body(Body::empty()).unwrap()).await;
    let session_cookie = Fresh::session_cookie(&page).expect("the sign-in page starts a session");
    let token = masked_session_token(&app.booted.app.secrets, &session_cookie).unwrap();
    for name in ["sign_in_page", "sign_in_page_with_session", "json_head", "forgery", "unauthorized_render"] {
        let reply = replay(&app, name, &section[name], Some(&session_cookie), Some(&token)).await;
        check(name, &reply);
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[tokio::test]
async fn headers_match_the_reference_per_route_class_over_plain_http() {
    assert_headers_match(false).await;
}

#[tokio::test]
async fn headers_match_the_reference_per_route_class_behind_tls() {
    assert_headers_match(true).await;
}

// --- Rate limits -------------------------------------------------------------------------------

#[tokio::test]
async fn sign_ins_are_rate_limited_per_ip_and_forgeries_are_not_counted() {
    let vectors = vectors();
    let expected = &vectors["rate_limit"];
    let app = boot_fresh(false).await;
    app.seed().await;
    let page = app.send(request("GET", "/session/new").body(Body::empty()).unwrap()).await;
    let cookie = Fresh::session_cookie(&page).unwrap();
    let token = masked_session_token(&app.booted.app.secrets, &cookie).unwrap();
    let attempt = |ip: &'static str, token: Option<&str>| {
        let mut pairs = vec![("email_address", "david@example.com"), ("password", "wrong")];
        if let Some(token) = token {
            pairs.push(("authenticity_token", token));
        }
        request("POST", "/session")
            .header("x-forwarded-for", ip)
            .header(header::COOKIE, format!("_campfire_session={cookie}"))
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(form(&pairs))
            .unwrap()
    };

    // Forgery protection runs before the limit, so forged attempts don't use it up.
    for _ in 0..3 {
        assert_eq!(app.send(attempt("10.3.0.1", Some("forged"))).await.status.as_u16(), expected["forged_status"].as_u64().unwrap() as u16);
    }
    let mut statuses = Vec::new();
    let mut last = None;
    for _ in 0..expected["statuses"].as_array().unwrap().len() {
        let reply = app.send(attempt("10.3.0.1", Some(&token))).await;
        statuses.push(json!(reply.status.as_u16()));
        last = Some(reply);
    }
    assert_eq!(Value::Array(statuses), expected["statuses"]);
    let limited = last.unwrap();
    assert_eq!(limited.header("content-type"), expected["limited"]["content_type"].as_str());
    assert!(limited.text().contains("<form"), "the sign-in form again: {}", limited.text());
    assert_eq!(app.send(attempt("10.3.0.2", Some(&token))).await.status.as_u16(), expected["other_ip_status"].as_u64().unwrap() as u16);
}

#[tokio::test]
async fn csp_reports_are_rate_limited_per_ip() {
    let vectors = vectors();
    let expected = &vectors["rate_limit"]["csp_reports"];
    let app = boot_fresh(false).await;
    let report = |ip: &str| {
        request("POST", "/csp_reports")
            .header("x-forwarded-for", ip)
            .header(header::CONTENT_TYPE, "application/csp-report")
            .body(Body::from(r#"{"csp-report":{"violated-directive":"img-src"}}"#))
            .unwrap()
    };
    let mut statuses = Vec::new();
    for _ in 0..expected["statuses"].as_array().unwrap().len() {
        statuses.push(json!(app.send(report("10.4.0.1")).await.status.as_u16()));
    }
    assert_eq!(Value::Array(statuses), expected["statuses"]);
    assert_eq!(app.send(report("10.4.0.2")).await.status, StatusCode::NO_CONTENT);
}

// --- Session plumbing --------------------------------------------------------------------------

#[tokio::test]
async fn idle_administrator_sessions_expire_when_restored() {
    let app = boot_fresh(false).await;
    let david = app.seed().await;
    let days = |n: i64| campfire_db::Timestamp::from_jiff(jiff::Timestamp::now() - jiff::SignedDuration::from_hours(24 * n));
    let (stale, fresh) = app
        .booted
        .app
        .db
        .write(move |tx| {
            let stale = Session::start(tx, david.id, Some("test"), None)?;
            let fresh = Session::start(tx, david.id, Some("test"), None)?;
            tx.conn().execute("UPDATE sessions SET last_active_at = ? WHERE id = ?", rusqlite::params![days(8), stale.id])?;
            tx.conn().execute("UPDATE sessions SET last_active_at = ? WHERE id = ?", rusqlite::params![days(6), fresh.id])?;
            Ok((stale, fresh))
        })
        .await
        .unwrap();

    let expired = app.send(request("GET", "/users/me/profile").header(header::COOKIE, app.session_token_cookie(&stale)).body(Body::empty()).unwrap()).await;
    assert_eq!((expired.status, expired.header("location")), (StatusCode::FOUND, Some("http://campfire.test/session/new")));
    assert!(expired.set_cookies().iter().any(|cookie| cookie.starts_with("session_token=;")), "{:?}", expired.set_cookies());
    let token = stale.token.clone();
    assert!(app.booted.app.db.read(move |conn| Session::find_by_token(conn, &token)).await.unwrap().is_none(), "destroyed");

    let restored = app.send(request("GET", "/users/me/profile").header(header::COOKIE, app.session_token_cookie(&fresh)).body(Body::empty()).unwrap()).await;
    assert_eq!(restored.status, StatusCode::OK, "{}", restored.text());
}

#[tokio::test]
async fn a_pending_second_factor_goes_back_to_its_challenge() {
    let app = boot_fresh(false).await;
    let david = app.seed().await;
    let pending = |expires_in: i64| {
        let session = json!({
            "session_id": "0123456789abcdef0123456789abcdef",
            "two_factor_pending_user_id": david.id,
            "two_factor_pending_expires_at": jiff::Timestamp::now().as_second() + expires_in,
            "two_factor_pending_method": "totp",
        });
        let raw = app.crypto().encrypt_cookie("_campfire_session", &session, None);
        format!("_campfire_session={}", campfire_kit::cookies::escape(&raw))
    };

    let waiting = app.send(request("GET", "/users/me/profile").header(header::COOKIE, pending(300)).body(Body::empty()).unwrap()).await;
    assert_eq!(waiting.header("location"), Some("http://campfire.test/two_factor_challenge"));
    let lapsed = app.send(request("GET", "/users/me/profile").header(header::COOKIE, pending(-1)).body(Body::empty()).unwrap()).await;
    assert_eq!(lapsed.header("location"), Some("http://campfire.test/session/new"));
}

#[tokio::test]
async fn page_navigations_bounce_back_after_sign_in() {
    let app = boot_fresh(false).await;
    app.seed().await;
    let page = app.send(request("GET", "/users/me/profile?tab=1").header(header::ACCEPT, "text/html").body(Body::empty()).unwrap()).await;
    assert_eq!(page.header("location"), Some("http://campfire.test/session/new"));
    let session = app.session(&Fresh::session_cookie(&page).unwrap());
    assert_eq!(session["return_to_after_authenticating"], "http://campfire.test/users/me/profile?tab=1");

    for accept in ["application/json", "text/vnd.turbo-stream.html, text/html"] {
        let poll = app.send(request("GET", "/users/me/profile").header(header::ACCEPT, accept).body(Body::empty()).unwrap()).await;
        let session = Fresh::session_cookie(&poll).map(|raw| app.session(&raw));
        assert!(session.is_none_or(|session| session.get("return_to_after_authenticating").is_none()), "{accept}");
    }
}

#[tokio::test]
async fn agent_bearer_tokens_are_refused_not_sent_to_sign_in() {
    let app = boot_fresh(false).await;
    app.seed().await;
    let agent = app.send(request("GET", "/users/me/profile").header(header::AUTHORIZATION, "Bearer cfa_unknown").body(Body::empty()).unwrap()).await;
    assert_eq!(agent.status, StatusCode::UNAUTHORIZED);
    let other_scheme = app.send(request("GET", "/users/me/profile").header(header::AUTHORIZATION, "Basic abc").body(Body::empty()).unwrap()).await;
    assert_eq!(other_scheme.status, StatusCode::FOUND);
}

#[tokio::test]
async fn sudo_mode_stashes_the_request_until_confirmed() {
    use crate::concerns::{require_sudo_mode, session_keys};

    async fn guarded(c: &mut campfire_kit::Ctx) -> campfire_kit::Result {
        require_sudo_mode(c)?;
        Ok(c.html("done"))
    }
    async fn confirm(c: &mut campfire_kit::Ctx) -> campfire_kit::Result {
        let now = c.now();
        session_keys::mark_sudo_verified(c.session(), now);
        Ok(c.html("confirmed"))
    }

    let app = boot_fresh(false).await;
    let kit = campfire_kit::Kit::new(
        campfire_kit::KitConfig::production(true),
        Arc::new(RailsCrypto::new(app.booted.app.secrets.clone())),
        app.booted.app.clock.clone(),
        app.booted.app.clone(),
    );
    let routes = axum::Router::new()
        .route("/danger", campfire_kit::get(guarded).post(campfire_kit::action(guarded)))
        .route("/confirm", campfire_kit::get(confirm));
    let router = campfire_kit::app(routes, kit);
    let send = |request: Request<Body>| {
        let router = router.clone();
        async move {
            let response = router.oneshot(request).await.unwrap();
            let (parts, body) = response.into_parts();
            Reply { status: parts.status, headers: parts.headers, body: axum::body::to_bytes(body, usize::MAX).await.unwrap().to_vec() }
        }
    };

    let prompt = send(request("GET", "/danger?x=1").header(header::REFERER, "http://campfire.test/rooms/1").body(Body::empty()).unwrap()).await;
    assert_eq!(prompt.header("location"), Some("http://campfire.test/sudo/new"));
    let cookie = Fresh::session_cookie(&prompt).unwrap();
    let stashed = app.session(&cookie);
    assert_eq!(stashed["sudo_pending_request"], json!({ "method": "GET", "path": "/danger?x=1", "params": null, "origin": "/rooms/1" }));

    let confirmed = send(request("GET", "/confirm").header(header::COOKIE, format!("_campfire_session={cookie}")).body(Body::empty()).unwrap()).await;
    let cookie = Fresh::session_cookie(&confirmed).unwrap();
    let through = send(request("GET", "/danger").header(header::COOKIE, format!("_campfire_session={cookie}")).body(Body::empty()).unwrap()).await;
    assert_eq!((through.status, through.text()), (StatusCode::OK, "done".to_string()));
}

#[test]
fn sudo_confirmations_are_limited_to_ten_in_three_minutes() {
    let limit = crate::concerns::sudo_rate_limit();
    let store = campfire_kit::RateLimitStore::new();
    let start = jiff::Timestamp::from_second(1_767_268_800).unwrap();
    for attempt in 1..=10 {
        assert!(!limit.exceeded(&store, "10.5.0.1", start), "attempt {attempt}");
    }
    assert!(limit.exceeded(&store, "10.5.0.1", start), "the eleventh");
    assert!(!limit.exceeded(&store, "10.5.0.2", start), "per IP");
    assert!(!limit.exceeded(&store, "10.5.0.1", start + jiff::SignedDuration::from_mins(3)), "reset");
}
