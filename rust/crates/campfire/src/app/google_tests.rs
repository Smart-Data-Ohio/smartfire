//! Google callbacks through the real router, cookie session, JWT verifier and SQLite writer.
use crate::{
    controllers::presenters::test_support::{Browser, DAVID, JASON, KEVIN, Req, TestApp},
    integrations::{
        google::{
            client::{Client, Unavailable},
            sign_in::{Config, SignIn},
        },
        net::BoxFuture,
    },
};
use axum::http::{Method, StatusCode};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

type HttpResult = Result<(u16, Vec<u8>), ()>;
struct Recorded {
    response: Mutex<Result<(u16, Vec<u8>), ()>>,
    calls: Mutex<Vec<(String, String, Vec<u8>)>>,
    certs: Mutex<Option<HttpResult>>,
}
impl Client for Recorded {
    fn request<'a>(
        &'a self,
        host: &'a str,
        method: Method,
        target: &'a str,
        _headers: Vec<(String, String)>,
        body: Vec<u8>,
    ) -> BoxFuture<'a, Result<(u16, Vec<u8>), Unavailable>> {
        Box::pin(async move {
            self.calls
                .lock()
                .unwrap()
                .push((method.to_string(), format!("{host}{target}"), body));
            if host == "www.googleapis.com" && target == "/oauth2/v3/certs" {
                return self
                    .certs
                    .lock()
                    .unwrap()
                    .clone()
                    .unwrap_or_else(|| {
                        Ok((
                            200,
                            include_bytes!("../integrations/google/test-jwks.json").to_vec(),
                        ))
                    })
                    .map_err(|_| crate::integrations::net::http::HttpError::OpenTimeout.into());
            }
            assert_eq!(
                (host, method, target),
                ("oauth2.googleapis.com", Method::POST, "/token")
            );
            self.response
                .lock()
                .unwrap()
                .clone()
                .map_err(|_| crate::integrations::net::http::HttpError::OpenTimeout.into())
        })
    }
}
async fn app() -> (TestApp, Arc<Recorded>) {
    let app = TestApp::boot().await.expect("pinned default seed required");
    let recorded = Arc::new(Recorded {
        response: Mutex::new(Err(())),
        calls: Mutex::new(vec![]),
        certs: Mutex::new(None),
    });
    app.booted.app.google.install(SignIn::with_client(
        Config {
            client_id: "test-client-id".into(),
            client_secret: "FAKE-google-client-secret".into(),
            domains: vec!["smartdata.net".into()],
        },
        recorded.clone(),
    ));
    app.db()
        .write(|tx| {
            tx.conn().execute("DELETE FROM audit_logs", [])?;
            tx.conn().execute("DELETE FROM google_identities", [])?;
            Ok(())
        })
        .await
        .unwrap();
    (app, recorded)
}
fn token(claims: Value) -> String {
    token_with_key(claims, "fixture", include_bytes!("../integrations/google/signing.der"))
}
fn token_with_key(claims: Value, kid: &str, key_bytes: &[u8]) -> String {
    use ring::signature::{RSA_PKCS1_SHA256, RsaKeyPair};
    let key = RsaKeyPair::from_pkcs8(key_bytes).unwrap();
    let input = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&json!({"alg":"RS256","kid":kid})).unwrap()),
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap())
    );
    let mut signature = vec![0; key.public().modulus_len()];
    key.sign(
        &RSA_PKCS1_SHA256,
        &ring::rand::SystemRandom::new(),
        input.as_bytes(),
        &mut signature,
    )
    .unwrap();
    format!("{input}.{}", URL_SAFE_NO_PAD.encode(signature))
}
async fn start(b: &mut Browser<'_>, path: &str) -> std::collections::BTreeMap<String, String> {
    let r = b.write(Req::new(Method::POST, path)).await;
    assert_eq!(r.status, StatusCode::FOUND, "{}", r.text());
    let u = url::Url::parse(r.location().unwrap()).unwrap();
    assert_eq!(u.host_str(), Some("accounts.google.com"));
    u.query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect()
}
fn claims(
    a: &TestApp,
    q: &std::collections::BTreeMap<String, String>,
    subject: &str,
    email: &str,
) -> Value {
    let now = a.booted.app.clock.now().as_second();
    json!({"iss":"https://accounts.google.com","aud":"test-client-id","sub":subject,"email":email,"email_verified":true,"hd":"smartdata.net","name":"Alice","nonce":q["nonce"],"exp":now+3600,"auth_time":now})
}
fn answer(r: &Recorded, v: Value) {
    *r.response.lock().unwrap() = Ok((
        200,
        serde_json::to_vec(&json!({"id_token":token(v)})).unwrap(),
    ));
}
async fn callback(
    b: &mut Browser<'_>,
    state: &str,
) -> crate::controllers::presenters::test_support::Reply {
    b.get(&format!(
        "/session/google/callback?state={}&code=fixture-code",
        crate::controllers::presenters::test_support::encode(state)
    ))
    .await
}
async fn actions(a: &TestApp) -> Vec<String> {
    a.db()
        .read(|c| {
            Ok(c.prepare("SELECT action FROM audit_logs ORDER BY id")?
                .query_map([], |r| r.get(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?)
        })
        .await
        .unwrap()
}
async fn sessions(a: &TestApp) -> i64 {
    a.db()
        .read(|c| Ok(c.query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))?))
        .await
        .unwrap()
}
#[tokio::test]
async fn provisions_then_subject_signs_in_without_repeating_link_audit() {
    let (a, r) = app().await;
    let mut b = a.anonymous();
    b.get("/session/new").await;
    let before = sessions(&a).await;
    let q = start(&mut b, "/session/google").await;
    assert_eq!(q["scope"], "openid email profile");
    assert!(!q.contains_key("prompt"));
    answer(&r, claims(&a, &q, "alice", "alice@smartdata.net"));
    assert_eq!(
        callback(&mut b, &q["state"]).await.location(),
        Some("http://campfire.test/")
    );
    assert_eq!(sessions(&a).await, before + 1);
    assert_eq!(
        actions(&a).await,
        vec!["user.create", "session.sign_in.success"]
    );
    let mut b = a.anonymous();
    b.get("/session/new").await;
    let q = start(&mut b, "/session/google").await;
    answer(&r, claims(&a, &q, "alice", "new@smartdata.net"));
    callback(&mut b, &q["state"]).await;
    assert_eq!(
        actions(&a).await,
        vec![
            "user.create",
            "session.sign_in.success",
            "session.sign_in.success"
        ]
    );
    let body = r.calls.lock().unwrap()[0].2.clone();
    let form = url::form_urlencoded::parse(&body).collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(
        form["redirect_uri"],
        "http://campfire.test/session/google/callback"
    );
    assert_eq!(form["grant_type"], "authorization_code");
    assert!(form.contains_key("code_verifier"));
}
#[tokio::test]
async fn security_wrong_state_consumes_flow_without_google_or_failure_audit() {
    let (a, r) = app().await;
    let mut b = a.anonymous();
    b.get("/session/new").await;
    let q = start(&mut b, "/session/google").await;
    assert_eq!(
        callback(&mut b, "forged").await.location(),
        Some("http://campfire.test/session/new")
    );
    answer(&r, claims(&a, &q, "alice", "alice@smartdata.net"));
    callback(&mut b, &q["state"]).await;
    assert!(r.calls.lock().unwrap().is_empty());
    assert!(actions(&a).await.is_empty());
}
#[tokio::test]
async fn security_signed_tokens_with_bad_nonce_audience_domain_or_expiry_never_create_session() {
    let (a, r) = app().await;
    for (key, value) in [
        ("nonce", json!("wrong")),
        ("aud", json!("other")),
        ("hd", json!("evil.test")),
        ("exp", json!(1)),
    ] {
        let mut b = a.anonymous();
        b.get("/session/new").await;
        let q = start(&mut b, "/session/google").await;
        let before = sessions(&a).await;
        let mut v = claims(&a, &q, "alice", "alice@smartdata.net");
        v[key] = value;
        answer(&r, v);
        assert_eq!(
            callback(&mut b, &q["state"]).await.location(),
            Some("http://campfire.test/session/new")
        );
        assert_eq!(sessions(&a).await, before);
    }
    assert_eq!(actions(&a).await, vec!["session.sign_in.failure"; 4]);
}
#[tokio::test]
async fn link_owns_flow_and_sudo_and_reauth_require_fresh_matching_subject() {
    let (a, r) = app().await;
    let mut b = a.sign_in(DAVID).await;
    b.get("/users/me/profile").await;
    let q = start(&mut b, "/user/profile/google_sign_in_link").await;
    answer(&r, claims(&a, &q, "david", "different@smartdata.net"));
    assert_eq!(
        callback(&mut b, &q["state"]).await.location(),
        Some("http://campfire.test/users/me/profile")
    );
    assert_eq!(actions(&a).await, vec!["google.sign_in.link"]);
    for (path, action) in [
        ("/two_factor_reauthentication", "two_factor.reauthenticate"),
        ("/sudo/google", "sudo.confirm.success"),
    ] {
        let q = start(&mut b, path).await;
        assert_eq!(q["prompt"], "login");
        assert_eq!(q["max_age"], "0");
        let mut v = claims(&a, &q, "david", "different@smartdata.net");
        v["auth_time"] = json!(a.booted.app.clock.now().as_second() - 3600);
        answer(&r, v);
        callback(&mut b, &q["state"]).await;
        assert!(!actions(&a).await.iter().any(|s| s == action));
        let q = start(&mut b, path).await;
        answer(&r, claims(&a, &q, "david", "different@smartdata.net"));
        callback(&mut b, &q["state"]).await;
        assert!(actions(&a).await.iter().any(|s| s == action));
    }
    let q = start(&mut b, "/sudo/google").await;
    answer(&r, claims(&a, &q, "other", "other@smartdata.net"));
    assert_eq!(
        callback(&mut b, &q["state"]).await.location(),
        Some("http://campfire.test/sudo/new")
    );
    assert_eq!(actions(&a).await.last().unwrap(), "sudo.confirm.failure");
    let mut b = a.sign_in(JASON).await;
    b.get("/users/me/profile").await;
    assert_eq!(
        callback(&mut b, &q["state"]).await.location(),
        Some("http://campfire.test/")
    );
}
#[tokio::test]
async fn session_insert_failure_rolls_back_provision_identity_and_audits() {
    let (a, r) = app().await;
    let mut b = a.anonymous();
    b.get("/session/new").await;
    let q = start(&mut b, "/session/google").await;
    a.db().write(|tx| {tx.conn().execute_batch("CREATE TRIGGER reject_google_session BEFORE INSERT ON sessions BEGIN SELECT RAISE(ABORT,'fixture rollback'); END;")?;Ok(())}).await.unwrap();
    answer(&r, claims(&a, &q, "rollback", "rollback@smartdata.net"));
    assert_eq!(
        callback(&mut b, &q["state"]).await.status,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    let count = a
        .db()
        .read(|c| {
            Ok(c.query_row(
                "SELECT COUNT(*) FROM users WHERE email_address='rollback@smartdata.net'",
                [],
                |r| r.get::<_, i64>(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(count, 0);
    assert!(actions(&a).await.is_empty());
}
#[tokio::test]
async fn admin_trust_and_unlink_audit_only_actual_changes_and_deny_members() {
    let (a, _) = app().await;
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE users SET email_self_changed_at=?,google_email_link_allowed=0 WHERE id=?",
                rusqlite::params![tx.now(), JASON],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let path = format!("/account/users/{JASON}/google_link");
    let mut member = a.sign_in(KEVIN).await;
    member.get("/users/me/profile").await;
    assert_eq!(
        member.write(Req::new(Method::POST, &path)).await.status,
        StatusCode::FORBIDDEN
    );
    let mut admin = a.sign_in(DAVID).await;
    admin.get("/users/me/profile").await;
    for _ in 0..2 {
        assert_eq!(
            admin.write(Req::new(Method::POST, &path)).await.location(),
            Some("http://campfire.test/account/edit")
        );
    }
    assert_eq!(actions(&a).await, vec!["google.sign_in.link_allow"]);
    for _ in 0..2 {
        admin.write(Req::new(Method::DELETE, &path)).await;
    }
    assert_eq!(actions(&a).await, vec!["google.sign_in.link_allow"]);
}
#[tokio::test]
async fn enrolled_google_sign_in_is_pending_until_second_factor_and_remembered_device_skips_challenge()
 {
    let (a, r) = app().await;
    a.db()
        .write(|tx| {
            campfire_db::models::google_identity::GoogleIdentity::link_to_user(
                tx,
                json!({"sub":"david","email":"david@smartdata.net","hd":"smartdata.net"})
                    .as_object()
                    .unwrap(),
                DAVID,
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut b = a.anonymous();
    b.get("/session/new").await;
    let q = start(&mut b, "/session/google").await;
    let before = sessions(&a).await;
    answer(&r, claims(&a, &q, "david", "david@smartdata.net"));
    assert_eq!(
        callback(&mut b, &q["state"]).await.location(),
        Some("http://campfire.test/two_factor_challenge")
    );
    assert_eq!(sessions(&a).await, before);
    assert!(actions(&a).await.is_empty());
    assert_eq!(b.get("/two_factor_challenge").await.status, StatusCode::OK);
    let token = a
        .db()
        .write(|tx| {
            Ok(campfire_db::TwoFactorRememberedDevice::create_for(tx, DAVID, None, None)?.1)
        })
        .await
        .unwrap();
    use campfire_kit::Crypto;
    let signed = campfire_kit::RailsCrypto::new(a.booted.app.secrets.clone()).sign_cookie(
        "two_factor_remember",
        &token,
        None,
    );
    let mut b = a.anonymous();
    b.absorb_cookie_header(&format!(
        "two_factor_remember={}",
        campfire_kit::cookies::escape(&signed)
    ));
    b.get("/session/new").await;
    let q = start(&mut b, "/session/google").await;
    answer(&r, claims(&a, &q, "david", "david@smartdata.net"));
    assert_eq!(
        callback(&mut b, &q["state"]).await.location(),
        Some("http://campfire.test/")
    );
    assert_eq!(sessions(&a).await, before + 1);
    assert_eq!(actions(&a).await, vec!["session.sign_in.success"]);
}
#[tokio::test]
async fn security_link_and_step_up_flow_cannot_move_to_another_signed_in_member() {
    let (a, r) = app().await;
    let mut owner = a.sign_in(DAVID).await;
    owner.get("/users/me/profile").await;
    let q = start(&mut owner, "/user/profile/google_sign_in_link").await;
    let flow_cookie = owner
        .cookie_header()
        .split(';')
        .find(|pair| pair.trim().starts_with("_campfire_session="))
        .unwrap()
        .trim()
        .to_owned();
    let mut other = a.sign_in(KEVIN).await;
    other.absorb_cookie_header(&flow_cookie);
    assert_eq!(
        callback(&mut other, &q["state"]).await.location(),
        Some("http://campfire.test/users/me/profile")
    );
    assert!(r.calls.lock().unwrap().is_empty());
    assert!(actions(&a).await.is_empty());
}

async fn counts(a: &TestApp) -> (i64, i64, i64) {
    a.db()
        .read(|c| {
            Ok((
                c.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))?,
                c.query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))?,
                c.query_row("SELECT COUNT(*) FROM google_identities", [], |r| r.get(0))?,
            ))
        })
        .await
        .unwrap()
}
fn config(domains: &[&str]) -> Config {
    Config {
        client_id: "test-client-id".into(),
        client_secret: "FAKE-google-client-secret".into(),
        domains: domains.iter().map(|s| (*s).into()).collect(),
    }
}
#[tokio::test]
async fn google_sessions_configured_page_and_disabled_credentials_or_domains_keep_password_login() {
    let (a, r) = app().await;
    a.db()
        .write(|tx| {
            if let Some(c) = campfire_db::TwoFactorCredential::for_user(tx.conn(), DAVID)? {
                c.destroy(tx)?;
            }
            Ok(())
        })
        .await
        .unwrap();
    a.booted.app.google.install(SignIn::with_client(
        config(&["smartdata.net", "cnbssoftware.com"]),
        r.clone(),
    ));
    let mut b = a.anonymous();
    let html = b.get("/session/new").await.text();
    for text in [
        "Sign in with Google",
        "#EA4335",
        "@smartdata.net and @cnbssoftware.com",
        "Other email addresses can sign in with email and password.",
        "name=\"email_address\"",
        "name=\"password\"",
    ] {
        assert!(html.contains(text), "{text}");
    }
    // Each app-scoped config installation models Rails' current env for that request.
    for mode in 0..2 {
        let mut cfg = config(&[]);
        if mode == 0 {
            cfg = config(&["smartdata.net"]);
            cfg.client_secret.clear();
        }
        a.booted
            .app
            .google
            .install(SignIn::with_client(cfg, r.clone()));
        let mut b = a.anonymous();
        let html = b.get("/session/new").await.text();
        assert!(!html.contains("Sign in with Google"));
        assert!(html.contains("name=\"password\""));
        assert_eq!(
            b.write(Req::new(Method::POST, "/session/google"))
                .await
                .status,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            callback(&mut b, "unused").await.status,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            b.write(Req::new(Method::POST, "/session").form(&[
                ("email_address", "david@37signals.com"),
                ("password", "secret123456")
            ]))
            .await
            .location(),
            Some("http://campfire.test/")
        );
    }
    assert!(r.calls.lock().unwrap().is_empty());
}
#[tokio::test]
async fn google_sessions_csrf_pkce_and_signed_in_guards_match_rails() {
    let (a, r) = app().await;
    let mut b = a.anonymous();
    b.get("/session/new").await;
    assert_eq!(
        b.send(Req::new(Method::POST, "/session/google"))
            .await
            .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let q = start(&mut b, "/session/google").await;
    assert_eq!(q["response_type"], "code");
    assert_eq!(q["client_id"], "test-client-id");
    assert_eq!(
        q["redirect_uri"],
        "http://campfire.test/session/google/callback"
    );
    assert_eq!(q["code_challenge_method"], "S256");
    for key in [
        "access_type",
        "prompt",
        "max_age",
        "hd",
        "include_granted_scopes",
    ] {
        assert!(!q.contains_key(key), "{key}");
    }
    let mut signed = a.sign_in(DAVID).await;
    signed.get("/users/me/profile").await;
    assert_eq!(
        signed
            .write(Req::new(Method::POST, "/session/google"))
            .await
            .location(),
        Some("http://campfire.test/")
    );
    assert_eq!(
        callback(&mut signed, "unused").await.location(),
        Some("http://campfire.test/")
    );
    assert!(r.calls.lock().unwrap().is_empty());
}
#[tokio::test]
async fn google_sessions_first_run_cannot_be_bypassed() {
    let (a, r) = app().await;
    a.db()
        .write(|tx| {
            tx.conn().execute("DELETE FROM accounts", [])?;
            Ok(())
        })
        .await
        .unwrap();
    let mut b = a.anonymous();
    b.get("/session/new").await;
    assert_eq!(
        b.write(Req::new(Method::POST, "/session/google"))
            .await
            .location(),
        Some("http://campfire.test/first_run")
    );
    assert_eq!(
        callback(&mut b, "unused").await.location(),
        Some("http://campfire.test/first_run")
    );
    assert!(r.calls.lock().unwrap().is_empty());
}
#[tokio::test]
async fn google_sessions_claim_rejections_preserve_all_rows_and_audit_each_attempt() {
    let (a, r) = app().await;
    let cases = [
        ("nonce", Value::Null),
        ("nonce", json!("wrong")),
        ("aud", json!("other")),
        ("aud", json!(["test-client-id", "other"])),
        ("azp", json!("other")),
        ("iss", json!("https://evil.test")),
        ("hd", Value::Null),
        ("hd", json!("evil.test")),
        ("email", json!("member@evil.test")),
        ("email", Value::Null),
        ("email_verified", json!(false)),
        ("sub", Value::Null),
        ("exp", json!(1)),
    ];
    let before = counts(&a).await;
    for (key, value) in cases {
        let mut b = a.anonymous();
        b.get("/session/new").await;
        let q = start(&mut b, "/session/google").await;
        let mut c = claims(&a, &q, "member", "member@smartdata.net");
        c[key] = value;
        answer(&r, c);
        assert_eq!(
            callback(&mut b, &q["state"]).await.location(),
            Some("http://campfire.test/session/new"),
            "{key}"
        );
        assert_eq!(counts(&a).await, before, "{key}");
    }
    assert_eq!(actions(&a).await, vec!["session.sign_in.failure"; 13]);
}
#[tokio::test]
async fn google_sessions_denied_missing_token_and_transport_outages_have_exact_notices() {
    let (a, r) = app().await;
    let before = counts(&a).await;
    for (response, keys, message) in [
        (
            Ok((
                400,
                serde_json::to_vec(&json!({"error":"invalid_grant"})).unwrap(),
            )),
            false,
            "Google sign-in failed. Try again or sign in with email and password.",
        ),
        (
            Ok((200, serde_json::to_vec(&json!({"id_token":null})).unwrap())),
            false,
            "Google sign-in failed. Try again or sign in with email and password.",
        ),
        (
            Err(()),
            false,
            "Google sign-in is unavailable right now. Try again or sign in with email and password.",
        ),
        (
            Ok((200, vec![])),
            true,
            "Google sign-in is unavailable right now. Try again or sign in with email and password.",
        ),
    ] {
        let mut b = a.anonymous();
        b.get("/session/new").await;
        let q = start(&mut b, "/session/google").await;
        if keys {
            answer(&r, claims(&a, &q, "outage", "outage@smartdata.net"));
            *r.certs.lock().unwrap() = Some(Err(()));
        } else {
            *r.response.lock().unwrap() = response;
        }
        assert_eq!(
            callback(&mut b, &q["state"]).await.location(),
            Some("http://campfire.test/session/new")
        );
        assert!(b.get("/session/new").await.text().contains(message));
        assert_eq!(counts(&a).await, before);
    }
    let body = r.calls.lock().unwrap()[0].2.clone();
    let form = url::form_urlencoded::parse(&body).collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(form["grant_type"], "authorization_code");
    assert!(!form["code_verifier"].is_empty());
    assert_eq!(actions(&a).await, vec!["session.sign_in.failure"; 2]);
}
#[tokio::test]
async fn google_sessions_cancel_missing_code_expired_and_replaced_flow_do_not_contact_google() {
    let (a, r) = app().await;
    let before = counts(&a).await;
    for cancel in [true, false] {
        let mut b = a.anonymous();
        b.get("/session/new").await;
        let q = start(&mut b, "/session/google").await;
        let path = format!(
            "/session/google/callback?state={}{}",
            crate::controllers::presenters::test_support::encode(&q["state"]),
            if cancel { "&error=access_denied" } else { "" }
        );
        assert_eq!(
            b.get(&path).await.location(),
            Some("http://campfire.test/session/new")
        );
        assert!(b.get("/session/new").await.text().contains(if cancel {
            "Google sign-in was cancelled."
        } else {
            "Google sign-in failed."
        }));
    }
    let mut b = a.anonymous();
    b.get("/session/new").await;
    let old = start(&mut b, "/session/google").await;
    start(&mut b, "/session/google").await;
    assert_eq!(
        callback(&mut b, &old["state"]).await.location(),
        Some("http://campfire.test/session/new")
    );
    let q = start(&mut b, "/session/google").await;
    use campfire_kit::Crypto;
    let crypto = campfire_kit::RailsCrypto::new(a.booted.app.secrets.clone());
    let raw = b
        .cookie_header()
        .split(';')
        .find(|s| s.trim().starts_with("_campfire_session="))
        .unwrap()
        .trim()
        .split_once('=')
        .unwrap()
        .1
        .to_owned();
    let mut value = crypto
        .decrypt_cookie(
            "_campfire_session",
            &rails_compat::cookies::unescape(&raw),
            a.booted.app.clock.now(),
        )
        .unwrap();
    value["google_sign_in_request"]["exp"] = json!(a.booted.app.clock.now().as_second() - 1);
    let encrypted = crypto.encrypt_cookie("_campfire_session", &value, None);
    b.absorb_cookie_header(&format!(
        "_campfire_session={}",
        campfire_kit::cookies::escape(&encrypted)
    ));
    assert_eq!(
        callback(&mut b, &q["state"]).await.location(),
        Some("http://campfire.test/session/new")
    );
    assert!(
        b.get("/session/new")
            .await
            .text()
            .contains("Google sign-in expired.")
    );
    assert_eq!(counts(&a).await, before);
    assert!(r.calls.lock().unwrap().is_empty());
    assert!(actions(&a).await.is_empty());
}
#[tokio::test]
async fn google_sessions_secondary_domains_multi_audience_and_return_path_provision_only_identity()
{
    let (a, r) = app().await;
    a.booted.app.google.install(SignIn::with_client(
        config(&["smartdata.net", "cnbssoftware.com"]),
        r.clone(),
    ));
    let mut b = a.anonymous();
    let path = format!(
        "/rooms/{}",
        crate::controllers::presenters::test_support::ALL_TALK
    );
    assert_eq!(
        b.get(&path).await.location(),
        Some("http://campfire.test/session/new")
    );
    b.get("/session/new").await;
    let q = start(&mut b, "/session/google").await;
    let mut c = claims(&a, &q, "secondary", "secondary@cnbssoftware.com");
    c["aud"] = json!(["test-client-id", "other"]);
    c["azp"] = json!("test-client-id");
    answer(&r, c);
    let before = counts(&a).await;
    assert_eq!(
        callback(&mut b, &q["state"]).await.location(),
        Some(format!("http://campfire.test{path}").as_str())
    );
    assert_eq!(counts(&a).await, (before.0 + 1, before.1 + 1, before.2 + 1));
    let user = a
        .db()
        .read(|conn| campfire_db::User::find_by_email_address(conn, "secondary@cnbssoftware.com"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(user.role, campfire_db::Role::Member);
    assert!(user.password_digest.is_none());
    let id = user.id;
    assert!(
        !a.db()
            .read(move |conn| Ok(conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM google_accounts WHERE user_id=?)",
                [id],
                |r| r.get::<_, bool>(0)
            )?))
            .await
            .unwrap()
    );
    b.write(Req::new(Method::DELETE, "/session")).await;
    assert_eq!(
        b.write(Req::new(Method::POST, "/session").form(&[
            ("email_address", "secondary@cnbssoftware.com"),
            ("password", "anything")
        ]))
        .await
        .status,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn google_sessions_ineligible_users_and_ambiguous_emails_never_create_identity_or_session() {
    use campfire_db::{NewUser, Role, User, UserChanges};
    for kind in [
        "deactivated",
        "retained",
        "banned",
        "bot",
        "agent",
        "ambiguous",
        "taken",
    ] {
        let (a, r) = app().await;
        a.db().write(move|tx| {
            let mut user=User::create(tx,NewUser{name:"Restricted".into(),email_address:Some("restricted@smartdata.net".into()),role:if kind=="bot"{Role::Bot}else{Role::Member},..Default::default()})?;
            user.update(tx,UserChanges{allow_google_email_link:true,..Default::default()})?;
            if matches!(kind,"retained"|"taken") {campfire_db::models::google_identity::GoogleIdentity::link_to_user(tx,json!({"sub":"original","email":"restricted@smartdata.net","hd":"smartdata.net"}).as_object().unwrap(),user.id)?;}
            match kind {
                "deactivated"|"retained"=>user.deactivate(tx)?,"banned"=>user.ban(tx)?,
                "agent"=>{tx.conn().execute("INSERT INTO agents(user_id,owner_id,created_at,updated_at) VALUES(?,?,?,?)",rusqlite::params![user.id,DAVID,tx.now(),tx.now()])?;},
                "ambiguous"=>{User::create(tx,NewUser{name:"Case duplicate".into(),email_address:Some("Restricted@smartdata.net".into()),..Default::default()})?;},_=>{}
            }
            Ok(())
        }).await.unwrap();
        let before = counts(&a).await;
        let mut b = a.anonymous();
        b.get("/session/new").await;
        let q = start(&mut b, "/session/google").await;
        answer(
            &r,
            claims(
                &a,
                &q,
                if kind == "retained" {
                    "original"
                } else {
                    "attempt"
                },
                "restricted@smartdata.net",
            ),
        );
        assert_eq!(
            callback(&mut b, &q["state"]).await.location(),
            Some("http://campfire.test/session/new"),
            "{kind}"
        );
        assert_eq!(counts(&a).await, before, "{kind}");
        let notices = b.get("/session/new").await.text();
        assert!(
            notices.contains(match kind {
                "deactivated" | "retained" | "banned" => "This account is no longer active.",
                "ambiguous" | "taken" => "Google sign-in could not pick your account.",
                _ => "Google sign-in failed.",
            }),
            "{kind}"
        );
        assert_eq!(actions(&a).await, vec!["session.sign_in.failure"]);
    }
}
#[tokio::test]
async fn google_sessions_malformed_signature_algorithm_and_unknown_key_fail_closed_through_router()
{
    let (a, r) = app().await;
    let before = counts(&a).await;
    for kind in ["malformed", "signature", "algorithm", "unknown-key"] {
        let mut b = a.anonymous();
        b.get("/session/new").await;
        let q = start(&mut b, "/session/google").await;
        let good = token(claims(&a, &q, "forged", "forged@smartdata.net"));
        let mut parts = good.split('.').map(str::to_owned).collect::<Vec<_>>();
        let bad = match kind {
            "malformed" => "not-a-jwt".to_string(),
            "signature" => {
                let mut bytes = URL_SAFE_NO_PAD.decode(&parts[2]).unwrap();
                bytes[0] ^= 1;
                parts[2] = URL_SAFE_NO_PAD.encode(bytes);
                parts.join(".")
            }
            "algorithm" => {
                parts[0] = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256","kid":"fixture"}"#);
                parts.join(".")
            }
            _ => {
                parts[0] = URL_SAFE_NO_PAD.encode(br#"{"alg":"RS256","kid":"unknown"}"#);
                parts.join(".")
            }
        };
        // Rails' cold unknown-key case fetches twice; give this attempt a cold cache too.
        if kind == "unknown-key" {
            a.booted
                .app
                .google
                .install(SignIn::with_client(config(&["smartdata.net"]), r.clone()));
        }
        let calls = r.calls.lock().unwrap().len();
        *r.response.lock().unwrap() =
            Ok((200, serde_json::to_vec(&json!({"id_token":bad})).unwrap()));
        assert_eq!(
            callback(&mut b, &q["state"]).await.location(),
            Some("http://campfire.test/session/new"),
            "{kind}"
        );
        assert_eq!(counts(&a).await, before);
        if kind == "unknown-key" {
            assert_eq!(
                r.calls.lock().unwrap()[calls..]
                    .iter()
                    .filter(|(_, path, _)| path.ends_with("/certs"))
                    .count(),
                2
            );
        }
    }
    assert_eq!(actions(&a).await, vec!["session.sign_in.failure"; 4]);
}

#[derive(Clone)]
struct LogCapture(Arc<Mutex<Vec<u8>>>);
impl std::io::Write for LogCapture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
#[tokio::test]
async fn google_sessions_rejection_log_names_reason_without_token_or_authorization_code() {
    let log = Arc::new(Mutex::new(vec![]));
    let writer = log.clone();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_writer(move || LogCapture(writer.clone()))
        .finish();
    // This Tokio test uses the current-thread runtime. Keep capture installed
    // through router setup and all awaits, as the queue log tests do.
    let _capture = tracing::subscriber::set_default(subscriber);
    // tracing-core's single-dispatch shortcut registers a callsite against the
    // registering thread's subscriber. Parallel tests have no local subscriber;
    // retain a second registrar so interest is computed from both live dispatches.
    let _other_dispatch = tracing::Dispatch::new(tracing::subscriber::NoSubscriber::default());
    let (a, r) = app().await;
    let mut b = a.anonymous();
    b.get("/session/new").await;
    let q = start(&mut b, "/session/google").await;
    let signed = token(claims(&a, &q, "log", "log@evil.test"));
    *r.response.lock().unwrap() = Ok((
        200,
        serde_json::to_vec(&json!({"id_token":signed})).unwrap(),
    ));
    let path = format!(
        "/session/google/callback?state={}&code=secret-auth-code",
        crate::controllers::presenters::test_support::encode(&q["state"])
    );
    assert_eq!(
        b.get(&path).await.location(),
        Some("http://campfire.test/session/new")
    );
    let output = String::from_utf8(log.lock().unwrap().clone()).unwrap();
    assert!(
        output.contains("Google sign-in rejected: wrong_domain"),
        "rejection log assertion: {output}"
    );
    assert!(!output.contains(&signed));
    assert!(!output.contains("secret-auth-code"));
}

mod parity_cases;

mod security_cases;

// WS16 owner comparison uses WS14g's real verifier and signing/JWKS fixtures.
mod slack_claim;
