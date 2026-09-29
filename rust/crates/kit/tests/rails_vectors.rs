//! Old-browser-tab continuity across the switch, both ways, with real `RailsCrypto`:
//!
//! - A page the reference Rails app rendered (`vectors/kit_security.json`, `sign_in`) posts its
//!   session cookie and CSRF tokens here, and the kit accepts and rejects exactly the posts Rails
//!   did; what it writes back decodes to the same session in Rails' format.
//! - A page the kit rendered writes its session cookie and tokens to
//!   `target/kit_security_rust_output.json`, which `reference-tools/kit/security_verify_rust.rb`
//!   posts to the reference app (a tab opened on the port that submits after a rollback).
//!
//! A signed cookie Rails wrote (`vectors/rails_compat.json`) is read as well.
//!
//! Our Rails app derives its cookie keys with PBKDF2-HMAC-**SHA1**, not the SHA256 that
//! `load_defaults` asks for: `config/initializers/active_record_encryption.rb` calls
//! `Rails.application.key_generator` while the app initializes, which memoizes a generator built
//! before `key_generator_hash_digest_class` takes effect (an `after_initialize`). `rails_compat`'s
//! `KeyGenerator` (WS1's) only does SHA256, which reads stock Campfire's cookies but not ours, so
//! these tests use [`OurRailsCrypto`] until it can do both.

use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, header};
use campfire_kit::crypto::{Crypto, SharedCrypto};
use campfire_kit::{Ctx, FrozenClock, Kit, KitConfig, RailsCrypto, Result, StatusCode, action};
use jiff::Timestamp;
use rails_compat::message_verifier::{Digest, Encoding, Serializer};
use rails_compat::{MessageEncryptor, MessageVerifier};
use serde_json::{Value, json};
use tower::ServiceExt;

fn security_vectors() -> Value {
    serde_json::from_str(include_str!("../../../vectors/kit_security.json")).unwrap()
}

fn rails_compat_vectors() -> Value {
    serde_json::from_str(include_str!("../../../vectors/rails_compat.json")).unwrap()
}

/// `sessions#new`: the tokens the sign-in page renders (`csrf_meta_tags` and the form to
/// `session_url`).
async fn new_session(c: &mut Ctx) -> Result {
    let tokens = c.authenticity_tokens();
    let body = json!({ "meta": tokens.global(), "form": tokens.for_form("http://campfire.test/session", "post") });
    c.json(StatusCode::OK, &body)
}

/// `sessions#create` after the forgery check: echoes the session it sees, then changes it.
async fn create_session(c: &mut Ctx) -> Result {
    c.verify_authenticity_token()?;
    let session = c.session();
    let body = json!({ "id": session.id(), "csrf": session.get("_csrf_token") });
    session.insert("return_to_after_authenticating", "/rooms/1");
    c.json(StatusCode::OK, &body)
}

async fn whoami(c: &mut Ctx) -> Result {
    let token = c.cookies.signed("session_token");
    c.json(StatusCode::OK, &json!({ "token": token }))
}

/// Our Rails app's cookie jars: `cookies.signed` (HMAC-SHA1) and `cookies.encrypted`
/// (aes-256-gcm), both with the legacy `_rails` envelope and JSON values, keyed by
/// PBKDF2-HMAC-SHA1 over `secret_key_base` (1000 iterations).
struct OurRailsCrypto {
    verifier: MessageVerifier,
    encryptor: MessageEncryptor,
}

impl OurRailsCrypto {
    fn new(secret_key_base: &str) -> Self {
        let key = |salt: &str, length: usize| {
            let mut key = vec![0u8; length];
            pbkdf2::pbkdf2_hmac::<sha1::Sha1>(secret_key_base.as_bytes(), salt.as_bytes(), 1000, &mut key);
            key
        };
        Self {
            verifier: MessageVerifier::new(key("signed cookie", 64), Digest::Sha1, Encoding::Strict, Serializer::Null),
            encryptor: MessageEncryptor::new(&key("authenticated encrypted cookie", 32), Serializer::Null),
        }
    }

    fn load(dumped: Value) -> Option<Value> {
        serde_json::from_str(dumped.as_str()?).ok()
    }

    fn decrypt(&self, name: &str, raw: &str, now: Timestamp) -> Option<Value> {
        let purpose = format!("cookie.{name}");
        let dumped = self.encryptor.decrypt_and_verify(raw, Some(&purpose), now).or_else(|_| self.encryptor.decrypt_and_verify(raw, None, now));
        Self::load(dumped.ok()?)
    }
}

impl Crypto for OurRailsCrypto {
    fn sign_cookie(&self, name: &str, value: &str, expires_at: Option<Timestamp>) -> String {
        let dumped = Value::String(serde_json::to_string(value).unwrap());
        self.verifier.generate(&dumped, Some(&format!("cookie.{name}")), expires_at)
    }

    fn verify_signed_cookie(&self, name: &str, raw: &str, now: Timestamp) -> Option<String> {
        let purpose = format!("cookie.{name}");
        let dumped = self.verifier.verify(raw, Some(&purpose), now).or_else(|_| self.verifier.verify(raw, None, now)).ok()?;
        Self::load(dumped)?.as_str().map(str::to_string)
    }

    fn encrypt_cookie(&self, name: &str, value: &Value, expires_at: Option<Timestamp>) -> String {
        let dumped = Value::String(serde_json::to_string(value).unwrap());
        self.encryptor.encrypt_and_sign(&dumped, Some(&format!("cookie.{name}")), expires_at)
    }

    fn decrypt_cookie(&self, name: &str, raw: &str, now: Timestamp) -> Option<Value> {
        self.decrypt(name, raw, now)
    }
}

fn app_with(crypto: SharedCrypto, vectors: &Value) -> Router {
    let now = vectors["now"].as_str().unwrap().parse().unwrap();
    let kit = Kit::new(KitConfig::default(), crypto, Arc::new(FrozenClock::new(now)), ());
    let router = Router::new()
        .route("/session/new", campfire_kit::get(new_session))
        .route("/session", axum::routing::post(action(create_session)))
        .route("/whoami", campfire_kit::get(whoami));
    campfire_kit::app(router, kit)
}

/// The kit with our Rails app's cookie crypto.
fn app(vectors: &Value) -> (Router, Arc<OurRailsCrypto>) {
    let crypto = Arc::new(OurRailsCrypto::new(vectors["secret_key_base"].as_str().unwrap()));
    (app_with(crypto.clone(), vectors), crypto)
}

struct Post<'a> {
    cookie: Option<String>,
    token: Option<&'a str>,
    header: Option<&'a str>,
    origin: Option<&'a str>,
}

async fn post_session(app: &Router, post: Post<'_>) -> axum::response::Response {
    let mut request = Request::post("/session")
        .header(header::HOST, "campfire.test")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded");
    if let Some(cookie) = post.cookie {
        request = request.header(header::COOKIE, cookie);
    }
    if let Some(header) = post.header {
        request = request.header("x-csrf-token", header);
    }
    if let Some(origin) = post.origin {
        request = request.header(header::ORIGIN, origin);
    }
    let mut body = "email_address=david%40example.com&password=wrong".to_string();
    if let Some(token) = post.token {
        body.push_str(&format!("&authenticity_token={}", campfire_kit::cookies::escape(token)));
    }
    app.clone().oneshot(request.body(Body::from(body)).unwrap()).await.unwrap()
}

fn session_cookie(raw: &str) -> String {
    format!("_campfire_session={}", campfire_kit::cookies::escape(raw))
}

fn set_cookies(response: &axum::response::Response) -> Vec<String> {
    response.headers().get_all(header::SET_COOKIE).iter().map(|v| v.to_str().unwrap().to_string()).collect()
}

async fn json_body(response: axum::response::Response) -> Value {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&body).unwrap()
}

#[tokio::test]
async fn rails_issued_tokens_are_accepted_and_rejected_as_rails_did() {
    let vectors = security_vectors();
    let sign_in = &vectors["sign_in"];
    let (app, _) = app(&vectors);
    let posts = sign_in["posts"].as_array().unwrap();
    assert!(posts.len() >= 18);

    for case in posts {
        let label = case["case"].as_str().unwrap();
        let cookie = match (case["cookies"].as_array().unwrap().is_empty(), label) {
            (true, _) => None,
            (false, "form token with the other session's cookie") => Some(session_cookie(sign_in["other_session_cookie_raw"].as_str().unwrap())),
            (false, _) => Some(session_cookie(sign_in["session_cookie_raw"].as_str().unwrap())),
        };
        let post = Post { cookie, token: case["token"].as_str(), header: case["header"].as_str(), origin: case["origin"].as_str() };
        let response = post_session(&app, post).await;
        // Rails answers an accepted post with sessions#create's 401 (wrong password), and a
        // rejected one with 422 and no cookies (the exception skips the session commit).
        match case["status"].as_u64().unwrap() {
            401 => {
                assert_eq!(response.status(), StatusCode::OK, "{label}: accepted");
                let body = json_body(response).await;
                assert_eq!(body["id"], sign_in["session"]["session_id"], "{label}");
                assert_eq!(body["csrf"], sign_in["session"]["_csrf_token"], "{label}: Rails' token stays the session's");
            }
            422 => {
                assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY, "{label}: rejected");
                assert!(set_cookies(&response).is_empty(), "{label}: no cookies on a rejected post");
            }
            other => panic!("{label}: unexpected Rails status {other}"),
        }
    }
}

#[tokio::test]
async fn rails_sessions_carry_over_in_rails_format() {
    let vectors = security_vectors();
    let sign_in = &vectors["sign_in"];
    let (app, crypto) = app(&vectors);
    let cookie = session_cookie(sign_in["session_cookie_raw"].as_str().unwrap());
    let form_token = sign_in["session_form_token"].as_str().unwrap();

    let response = post_session(&app, Post { cookie: Some(cookie), token: Some(form_token), header: None, origin: None }).await;
    assert_eq!(response.status(), StatusCode::OK);
    let set_cookie = set_cookies(&response).into_iter().next().unwrap();

    // What we write back after a change is the same session (and CSRF token) plus the change.
    let raw = set_cookie.strip_prefix("_campfire_session=").unwrap().split(';').next().unwrap();
    let raw = rails_compat::cookies::unescape(raw);
    let now = vectors["now"].as_str().unwrap().parse().unwrap();
    let decoded = crypto.decrypt("_campfire_session", &raw, now).unwrap();
    let mut expected = sign_in["session"].clone();
    expected["return_to_after_authenticating"] = "/rooms/1".into();
    assert_eq!(decoded, expected);
    assert!(set_cookie.ends_with("; path=/; expires=Mon, 01 Jan 2046 12:00:00 GMT; httponly; samesite=lax"));
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A page rendered here, for a new visitor and for a visitor with Rails' session, then a post of
/// each token back here. The same tokens and cookies go to the reference app to verify.
#[tokio::test]
async fn kit_issued_tokens_verify_here_and_are_written_for_rails() {
    let vectors = security_vectors();
    let sign_in = &vectors["sign_in"];
    let (app, crypto) = app(&vectors);
    let now = vectors["now"].as_str().unwrap().parse().unwrap();

    let page = |cookie: Option<String>| {
        let app = app.clone();
        async move {
            let mut request = Request::get("/session/new").header(header::HOST, "campfire.test");
            if let Some(cookie) = cookie {
                request = request.header(header::COOKIE, cookie);
            }
            let response = app.oneshot(request.body(Body::empty()).unwrap()).await.unwrap();
            let cookies = set_cookies(&response);
            (cookies, json_body(response).await)
        }
    };

    // A new visitor: rendering tokens starts a session holding the token.
    let (cookies, fresh) = page(None).await;
    assert_eq!(cookies.len(), 1, "the page's tokens need a session: {cookies:?}");
    let fresh_raw = rails_compat::cookies::unescape(cookies[0].strip_prefix("_campfire_session=").unwrap().split(';').next().unwrap());
    let fresh_session = crypto.decrypt("_campfire_session", &fresh_raw, now).unwrap();
    assert_eq!(fresh_session["_csrf_token"].as_str().unwrap().len(), 43);
    assert_eq!(fresh_session["session_id"].as_str().unwrap().len(), 32);

    // A visitor with Rails' session: the page reuses its token and needs no new cookie.
    let rails_cookie = session_cookie(sign_in["session_cookie_raw"].as_str().unwrap());
    let (cookies, carried) = page(Some(rails_cookie.clone())).await;
    assert!(cookies.is_empty(), "an unchanged session isn't rewritten: {cookies:?}");

    for (cookie, tokens) in [(session_cookie(&fresh_raw), &fresh), (rails_cookie.clone(), &carried)] {
        for token in [tokens["meta"].as_str().unwrap(), tokens["form"].as_str().unwrap()] {
            let ok = post_session(&app, Post { cookie: Some(cookie.clone()), token: Some(token), header: None, origin: None }).await;
            assert_eq!(ok.status(), StatusCode::OK);
            let as_header = post_session(&app, Post { cookie: Some(cookie.clone()), token: None, header: Some(token), origin: None }).await;
            assert_eq!(as_header.status(), StatusCode::OK);
        }
    }
    // Each session's tokens only work with that session.
    let crossed = Post { cookie: Some(rails_cookie), token: fresh["form"].as_str(), header: None, origin: None };
    assert_eq!(post_session(&app, crossed).await.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let output = json!({
        "now": vectors["now"],
        "fresh": { "session_cookie_raw": fresh_raw, "session": fresh_session, "meta": fresh["meta"], "form": fresh["form"] },
        "rails_session": { "session_cookie_raw": sign_in["session_cookie_raw"], "meta": carried["meta"], "form": carried["form"] },
    });
    let path = workspace_root().join("target/kit_security_rust_output.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, serde_json::to_string_pretty(&output).unwrap() + "\n").unwrap();
}

#[tokio::test]
async fn rails_signed_session_token_cookie_is_read() {
    let vectors = rails_compat_vectors();
    let session = &vectors["session"];
    // Stock Campfire's cookie (SHA256 keys), which `RailsCrypto` reads.
    let secrets = Arc::new(rails_compat::Secrets::new(vectors["secret_key_base"].as_str().unwrap()));
    let app = app_with(Arc::new(RailsCrypto::new(secrets)), &vectors);
    let cookie = format!("session_token={}", campfire_kit::cookies::escape(session["session_token_raw"].as_str().unwrap()));
    let request = Request::get("/whoami").header(header::COOKIE, cookie).body(Body::empty()).unwrap();
    let response = app.oneshot(request).await.unwrap();
    let body = json_body(response).await;
    assert_eq!(body["token"], session["session_token_value"]);
}
