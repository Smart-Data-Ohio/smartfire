//! Real HTTP/SQLite security tests for `test/controllers/sudos_controller_test.rb`.
//! These require the WS19 default seed; missing input is a failure, never a silent skip.
use crate::controllers::presenters::test_support::{
    BENDER, Browser, DAVID, JASON, KEVIN, Reply, Req, TestApp,
};
use axum::http::{Method, StatusCode};
use campfire_db::{Account, TwoFactorBackupCode, TwoFactorCredential};
use campfire_kit::{Crypto, Ctx, Kit, KitConfig, RailsCrypto, Result};
use rails_compat::ar_encryption::ArEncryption;
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

async fn app() -> TestApp {
    TestApp::boot()
        .await
        .expect("build the current pinned WS19 default parity seed")
}

trait RedirectResponse {
    fn redirect_location(&self) -> Option<&str>;
}
impl RedirectResponse for Reply {
    fn redirect_location(&self) -> Option<&str> {
        assert!(
            self.status.is_redirection(),
            "{}: {}",
            self.status,
            self.text()
        );
        self.location()
    }
}

/// Confirms (or tries to) as the SPA's sudo page does: `POST /api/v1/sudo` with `body`.
async fn confirm(b: &mut Browser<'_>, body: Value) -> Reply {
    b.write(Req::new(Method::POST, "/api/v1/sudo").json_body(&body))
        .await
}

/// The browser's cookie session, where the gate keeps the request it held.
fn session_state(a: &TestApp, b: &Browser<'_>) -> Value {
    let cookies = b.cookie_header();
    let raw = cookies
        .split("; ")
        .find_map(|cookie| cookie.strip_prefix("_campfire_session="))
        .unwrap();
    RailsCrypto::new(a.booted.app.secrets.clone())
        .decrypt_cookie(
            "_campfire_session",
            &rails_compat::cookies::unescape(raw),
            a.booted.app.clock.now(),
        )
        .unwrap()
}

#[tokio::test]
async fn sudo_password_prompt_and_rejection_statuses() {
    let a = app().await;
    let mut b = a.sign_in(DAVID).await;
    let prompt = b.get("/sudo/new").await;
    assert_eq!(prompt.status, StatusCode::FOUND);
    assert_eq!(
        prompt.redirect_location(),
        Some("http://campfire.test/app/sudo/new")
    );
    // The SPA's page offers what `GET /api/v1/sudo` names: password and TOTP, no Google.
    let methods = b.get("/api/v1/sudo").await;
    assert_eq!(methods.status, StatusCode::OK);
    assert_eq!(methods.json()["kind"], "ready");
    assert_eq!(
        methods.json()["reauthentication"]["methods"],
        json!(["password", "totp"])
    );
    let failures_before = a
        .db()
        .read(|c| {
            Ok(c.query_row(
                "SELECT count(*) FROM audit_logs WHERE action='sudo.confirm.failure'",
                [],
                |row| row.get::<_, i64>(0),
            )?)
        })
        .await
        .unwrap();
    let bad = b.confirm_sudo("wrong").await;
    assert_eq!(bad.status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        a.db()
            .read(|c| Ok(c.query_row(
                "SELECT count(*) FROM audit_logs WHERE action='sudo.confirm.failure'",
                [],
                |row| row.get::<_, i64>(0)
            )?))
            .await
            .unwrap(),
        failures_before + 1
    );
    assert_eq!(
        bad.json(),
        json!({"kind":"error","message":"Confirmation failed. Try again."})
    );
    assert_eq!(
        b.write(Req::new(Method::POST, "/account/join_code"))
            .await
            .redirect_location(),
        Some("http://campfire.test/app/sudo/new")
    );
    let unsupported = confirm(&mut b, json!({"kind":"other"})).await;
    assert_eq!(unsupported.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        unsupported.json(),
        json!({"kind":"error","message":"The request body isn't valid."})
    );
    // The retained form confirms nothing and counts nothing: it goes to the SPA's page.
    let retained = b
        .write(Req::new(Method::POST, "/sudo").form(&[("password", "wrong")]))
        .await;
    assert_eq!(retained.status, StatusCode::SEE_OTHER);
    assert_eq!(
        retained.redirect_location(),
        Some("http://campfire.test/app/sudo/new")
    );
    let rows = a
        .db()
        .read(|c| {
            Ok(c.query_row(
                "SELECT count(*) FROM audit_logs WHERE action = 'sudo.confirm.failure'",
                [],
                |r| r.get::<_, i64>(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(
        rows, 1,
        "unsupported methods do not record a credential failure"
    );
}

#[tokio::test]
async fn sudo_gate_holds_the_write_until_confirmed_and_names_it_once() {
    let a = app().await;
    let mut b = a.sign_in(DAVID).await;
    let before = a
        .db()
        .read(Account::first)
        .await
        .unwrap()
        .unwrap()
        .join_code;
    let gated = b.write(Req::new(Method::POST, "/account/join_code")).await;
    assert_eq!(
        gated.redirect_location(),
        Some("http://campfire.test/app/sudo/new")
    );
    // The retained page goes to the SPA's, keeping its query and the held request.
    assert_eq!(
        b.get("/sudo/new?from=mail").await.redirect_location(),
        Some("http://campfire.test/app/sudo/new?from=mail")
    );
    let held = json!({"method":"POST","path":"/account/join_code","returnTo":"/app/"});
    assert_eq!(
        b.get("/api/v1/sudo").await.json()["reauthentication"]["retry"],
        held
    );
    assert_eq!(
        a.db()
            .read(Account::first)
            .await
            .unwrap()
            .unwrap()
            .join_code,
        before
    );
    let confirmed = b.confirm_sudo("secret123456").await;
    assert_eq!(confirmed.status, StatusCode::OK, "{}", confirmed.text());
    assert_eq!(confirmed.json()["kind"], "confirmed");
    assert_eq!(confirmed.json()["retry"], held);
    // Confirming sends nothing on: the held write runs only when it's sent again.
    assert_eq!(
        a.db()
            .read(Account::first)
            .await
            .unwrap()
            .unwrap()
            .join_code,
        before
    );
    let resumed = b.write(Req::new(Method::POST, "/account/join_code")).await;
    assert_eq!(
        resumed.redirect_location(),
        Some("http://campfire.test/account/edit")
    );
    assert_ne!(
        a.db()
            .read(Account::first)
            .await
            .unwrap()
            .unwrap()
            .join_code,
        before
    );
    let again = b.confirm_sudo("secret123456").await;
    assert_eq!(again.status, StatusCode::OK);
    assert_eq!(again.json()["retry"], Value::Null, "pending request is consumed");
    assert_eq!(a.db().read(|conn| Ok(conn.query_row("SELECT count(*) FROM audit_logs WHERE action = 'sudo.confirm.success' AND details = '{\"verifier\":\"password\"}'", [], |row| row.get::<_, i64>(0))?)).await.unwrap(), 2);
}

#[tokio::test]
async fn sudo_authorization_and_record_lookup_run_before_the_gate() {
    let a = app().await;
    let mut member = a.sign_in(KEVIN).await;
    let denied = member
        .write(Req::new(Method::POST, "/account/join_code"))
        .await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);
    let mut admin = a.sign_in(DAVID).await;
    let missing = admin
        .write(
            Req::new(Method::PATCH, "/account/users/999999999").form(&[("user[role]", "member")]),
        )
        .await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn sudo_applies_to_all_present_rails_sensitive_actions() {
    let a = app().await;
    for (method, path, pairs) in [
        (
            Method::POST,
            "/account/bots".into(),
            vec![("user[name]", "Sudo test")],
        ),
        (Method::PUT, format!("/account/bots/{BENDER}/key"), vec![]),
        (
            Method::PATCH,
            "/account/custom_styles".into(),
            vec![("account[custom_styles]", "body{}")],
        ),
        (
            Method::PATCH,
            format!("/account/users/{JASON}"),
            vec![("user[role]", "member")],
        ),
        (Method::DELETE, format!("/account/users/{JASON}"), vec![]),
        (Method::POST, format!("/users/{KEVIN}/ban"), vec![]),
        (Method::DELETE, format!("/users/{KEVIN}/ban"), vec![]),
    ] {
        let mut b = a.sign_in(DAVID).await;
        let reply = b.write(Req::new(method.clone(), &path).form(&pairs)).await;
        assert_eq!(
            reply.redirect_location(),
            Some("http://campfire.test/app/sudo/new"),
            "{method} {path}: {} {}",
            reply.status,
            reply.text()
        );
    }
}

#[tokio::test]
async fn sudo_secret_and_large_bodies_return_to_the_origin_and_never_replay() {
    let a = app().await;
    let large = "x".repeat(2049);
    for pairs in [
        vec![
            ("user[name]", "Bot"),
            ("user[webhook_url]", "https://example.com/private"),
        ],
        vec![("user[name]", &large)],
    ] {
        let mut b = a.sign_in(DAVID).await;
        let gated = b
            .write(
                Req::new(Method::POST, "/account/bots")
                    .header("referer", "http://campfire.test/account/bots/new?ignored=1")
                    .form(&pairs),
            )
            .await;
        assert_eq!(
            gated.redirect_location(),
            Some("http://campfire.test/app/sudo/new")
        );
        let state = session_state(&a, &b);
        assert!(state["sudo_pending_request"]["params"].is_null());
        assert_eq!(state["sudo_pending_request"]["origin"], "/account/bots/new");
        let confirmed = b.confirm_sudo("secret123456").await;
        assert_eq!(confirmed.status, StatusCode::OK, "{}", confirmed.text());
        assert_eq!(
            confirmed.json()["retry"],
            json!({"method":"POST","path":"/account/bots","returnTo":"/app/"})
        );
        // The retained continuation of the same held request goes back to where it was made.
        let (status, headers, _) = probe(&a, "/continue", state).await;
        assert_eq!(status, StatusCode::FOUND);
        assert_eq!(headers["location"], "http://campfire.test/account/bots/new");
    }
}

#[tokio::test]
async fn sudo_non_replayable_fizzy_post_uses_only_a_same_host_referrer() {
    let a = app().await;
    for (referrer, expected) in [
        (None, "http://campfire.test/"),
        (
            Some("https://evil.example.test/phish"),
            "http://campfire.test/",
        ),
        (
            Some("http://campfire.test/users/me/profile"),
            "http://campfire.test/users/me/profile",
        ),
    ] {
        let mut b = a.sign_in(KEVIN).await;
        let mut request =
            Req::new(Method::POST, "/fizzy/connection").form(&[("access_token", "pasted-token")]);
        if let Some(referrer) = referrer {
            request = request.header("referer", referrer);
        }
        let gated = b.write(request).await;
        assert_eq!(
            gated.redirect_location(),
            Some("http://campfire.test/app/sudo/new")
        );
        let state = session_state(&a, &b);
        assert!(state["sudo_pending_request"]["params"].is_null());
        let confirmed = b.confirm_sudo("secret123456").await;
        assert_eq!(confirmed.status, StatusCode::OK, "{}", confirmed.text());
        assert_eq!(
            confirmed.json()["retry"],
            json!({"method":"POST","path":"/fizzy/connection","returnTo":"/app/"})
        );
        let (status, headers, body) = probe(&a, "/continue", state).await;
        assert_eq!(status, StatusCode::FOUND);
        assert_eq!(headers["location"], expected);
        assert!(!body.contains("action=\"/fizzy/connection\""));
    }
}

#[tokio::test]
async fn sudo_totp_is_replay_protected_shares_lockout_and_does_not_spend_backup_codes() {
    let a = app().await;
    let encryption = ArEncryption::new(&a.booted.app.secrets);
    let credential = a
        .db()
        .read(|c| TwoFactorCredential::for_user(c, DAVID))
        .await
        .unwrap()
        .unwrap();
    let secret = credential.secret(&encryption).unwrap();
    let code = rails_compat::totp::at(&secret, a.booted.app.clock.now().as_second()).unwrap();
    let codes = a
        .db()
        .write(move |tx| TwoFactorBackupCode::regenerate_set(tx, credential.id))
        .await
        .unwrap();
    let mut b = a.sign_in(DAVID).await;
    let ok = confirm(&mut b, json!({"kind":"totp","code":code})).await;
    assert_eq!(ok.status, StatusCode::OK, "{}", ok.text());
    assert_eq!(ok.json()["kind"], "confirmed");
    let success = a.db().read(|c| Ok((
        c.query_row("SELECT count(*) FROM audit_logs WHERE action='sudo.confirm.success'", [], |row| row.get::<_, i64>(0))?,
        c.query_row("SELECT json_extract(details, '$.verifier') FROM audit_logs WHERE action='sudo.confirm.success' ORDER BY id DESC LIMIT 1", [], |row| row.get::<_, String>(0))?
    ))).await.unwrap();
    assert_eq!(success, (1, "totp".into()));
    for submitted in [&code, &codes[0], "wrong", "wrong", "wrong"] {
        let rejected = confirm(&mut b, json!({"kind":"totp","code":submitted})).await;
        assert_eq!(rejected.status, StatusCode::UNAUTHORIZED);
    }
    let (locked, unused) = a
        .db()
        .read(|c| {
            let credential = TwoFactorCredential::for_user(c, DAVID)?.unwrap();
            Ok((
                credential.lockout_count,
                TwoFactorBackupCode::for_credential(c, credential.id)?
                    .iter()
                    .filter(|code| code.used_at.is_none())
                    .count(),
            ))
        })
        .await
        .unwrap();
    assert_eq!(locked, 1);
    assert_eq!(unused, 10);
    assert_eq!(a.db().read(|conn| Ok(conn.query_row("SELECT count(*) FROM audit_logs WHERE action = 'sudo.confirm.success' AND details = '{\"verifier\":\"totp\"}'", [], |row| row.get::<_, i64>(0))?)).await.unwrap(), 1);
    let next = rails_compat::totp::at(&secret, a.booted.app.clock.now().as_second() + 30).unwrap();
    assert_eq!(
        confirm(&mut b, json!({"kind":"totp","code":next}))
            .await
            .status,
        StatusCode::UNAUTHORIZED,
        "a fresh valid code is refused while locked out"
    );
    let a = app().await;
    let mut b = a.sign_in(DAVID).await;
    for _ in 0..5 {
        assert_eq!(
            confirm(&mut b, json!({"kind":"totp","code":"000000"}))
                .await
                .status,
            StatusCode::UNAUTHORIZED
        );
    }
    let credential = a
        .db()
        .read(|c| TwoFactorCredential::for_user(c, DAVID))
        .await
        .unwrap()
        .unwrap();
    assert!(credential.locked_out(campfire_db::Timestamp::from_jiff(a.booted.app.clock.now())));
    let secret = credential
        .secret(&ArEncryption::new(&a.booted.app.secrets))
        .unwrap();
    let code = rails_compat::totp::at(&secret, a.booted.app.clock.now().as_second()).unwrap();
    assert_eq!(
        confirm(&mut b, json!({"kind":"totp","code":code}))
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn sudo_unsupported_totp_without_enrollment_and_google_without_link() {
    let a = app().await;
    a.db()
        .write(|tx| {
            if let Some(credential) = TwoFactorCredential::for_user(tx.conn(), DAVID)? {
                credential.destroy(tx)?;
            }
            Ok(())
        })
        .await
        .unwrap();
    let mut b = a.sign_in(DAVID).await;
    let prompt = b.get("/api/v1/sudo").await;
    assert_eq!(prompt.status, StatusCode::OK);
    assert_eq!(
        prompt.json()["reauthentication"]["methods"],
        json!(["password"])
    );
    let rejected = confirm(&mut b, json!({"kind":"totp","code":"123456"})).await;
    assert_eq!(rejected.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        rejected.json()["message"],
        "That confirmation method is not available."
    );
    let google = b
        .write(Req::new(Method::POST, "/api/v1/sudo/google").json_body(&json!({})))
        .await;
    assert_eq!(google.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        google.json(),
        json!({"kind":"error","message":"Google confirmation is not available for your account."})
    );
    // The retained Google form starts nothing: it goes to the SPA's page.
    let retained = b.write(Req::new(Method::POST, "/sudo/google")).await;
    assert_eq!(retained.status, StatusCode::SEE_OTHER);
    assert_eq!(
        retained.redirect_location(),
        Some("http://campfire.test/app/sudo/new")
    );
}

#[tokio::test]
async fn sudo_create_and_google_share_ten_attempts_in_three_minutes() {
    let a = app().await;
    for _ in 0..5 {
        let mut b = a.sign_in(DAVID).await;
        assert_eq!(
            b.confirm_sudo("wrong").await.status,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            b.write(Req::new(Method::POST, "/api/v1/sudo/google").json_body(&json!({})))
                .await
                .status,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    let mut b = a.sign_in(DAVID).await;
    let rejected = b.confirm_sudo("secret123456").await;
    assert_eq!(rejected.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        rejected.json()["message"],
        "Too many confirmation attempts. Try again in a few minutes."
    );
    let a = app().await;
    let mut b = a.sign_in(DAVID).await;
    for _ in 0..10 {
        assert_eq!(
            b.confirm_sudo("wrong").await.status,
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        b.confirm_sudo("wrong").await.status,
        StatusCode::TOO_MANY_REQUESTS
    );
}

#[tokio::test]
async fn sudo_confirmations_require_authentication_and_csrf() {
    let a = app().await;
    let mut anonymous = a.anonymous();
    assert_eq!(
        anonymous.get("/sudo/new").await.redirect_location(),
        Some("http://campfire.test/app/session/new")
    );
    let mut b = a.sign_in(DAVID).await;
    let rejected = b
        .send(
            Req::new(Method::POST, "/api/v1/sudo")
                .json_body(&json!({"kind":"password","password":"secret123456"})),
        )
        .await;
    assert_eq!(rejected.status, StatusCode::UNPROCESSABLE_ENTITY);
    // The retained form goes to the SPA's page before any check, confirming nothing.
    let retained = b
        .send(Req::new(Method::POST, "/sudo").form(&[("password", "secret123456")]))
        .await;
    assert_eq!(retained.status, StatusCode::SEE_OTHER);
    assert_eq!(
        retained.redirect_location(),
        Some("http://campfire.test/app/sudo/new")
    );
    assert_eq!(
        a.db()
            .read(|c| Ok(c.query_row(
                "SELECT count(*) FROM audit_logs WHERE action LIKE 'sudo.confirm.%'",
                [],
                |r| r.get::<_, i64>(0)
            )?))
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn sudo_bot_webhook_gate_compares_the_stripped_value_and_leaves_name_edits_open() {
    let a = app().await;
    let existing = a
        .db()
        .read(|c| campfire_db::Webhook::find_by_user(c, BENDER))
        .await
        .unwrap()
        .unwrap()
        .url
        .unwrap();
    let action = format!("/account/bots/{BENDER}");
    for pairs in [
        vec![("user[name]", "Bender renamed")],
        vec![
            ("user[name]", "Bender renamed"),
            ("user[webhook_url]", existing.as_str()),
        ],
    ] {
        let mut b = a.sign_in(DAVID).await;
        assert_eq!(
            b.write(Req::new(Method::PATCH, &action).form(&pairs))
                .await
                .redirect_location(),
            Some("http://campfire.test/account/bots")
        );
        assert_eq!(
            a.db()
                .read(|c| campfire_db::User::find(c, BENDER))
                .await
                .unwrap()
                .name,
            "Bender renamed"
        );
    }
    let mut b = a.sign_in(DAVID).await;
    assert_eq!(
        b.write(
            Req::new(Method::PATCH, &action)
                .form(&[("user[webhook_url]", "https://example.com/changed")])
        )
        .await
        .redirect_location(),
        Some("http://campfire.test/app/sudo/new")
    );
}

#[tokio::test]
async fn sudo_uploaded_files_are_never_stashed_or_staged_before_confirmation() {
    let a = app().await;
    let before = a
        .db()
        .read(|c| {
            Ok(
                c.query_row("SELECT count(*) FROM active_storage_blobs", [], |r| {
                    r.get::<_, i64>(0)
                })?,
            )
        })
        .await
        .unwrap();
    let mut b = a.sign_in(DAVID).await;
    let gated = b
        .write(
            Req::new(Method::POST, "/account/bots")
                .header("referer", "http://campfire.test/account/bots/new")
                .multipart(
                    &[("user[name]", "Uploaded bot")],
                    ("user[avatar]", "avatar.png", "image/png", b"not staged"),
                ),
        )
        .await;
    assert_eq!(
        gated.redirect_location(),
        Some("http://campfire.test/app/sudo/new")
    );
    assert_eq!(
        a.db()
            .read(|c| Ok(
                c.query_row("SELECT count(*) FROM active_storage_blobs", [], |r| r
                    .get::<_, i64>(0))?
            ))
            .await
            .unwrap(),
        before
    );
    let state = session_state(&a, &b);
    assert!(state["sudo_pending_request"]["params"].is_null());
    assert_eq!(state["sudo_pending_request"]["origin"], "/account/bots/new");
    let confirmed = b.confirm_sudo("secret123456").await;
    assert_eq!(confirmed.status, StatusCode::OK, "{}", confirmed.text());
    assert_eq!(
        confirmed.json()["retry"],
        json!({"method":"POST","path":"/account/bots","returnTo":"/app/"})
    );
    let (status, headers, _) = probe(&a, "/continue", state).await;
    assert_eq!(status, StatusCode::FOUND);
    assert_eq!(headers["location"], "http://campfire.test/account/bots/new");
    assert_eq!(
        a.db()
            .read(|c| Ok(
                c.query_row("SELECT count(*) FROM active_storage_blobs", [], |r| r
                    .get::<_, i64>(0))?
            ))
            .await
            .unwrap(),
        before
    );
}

async fn gated_probe(c: &mut Ctx) -> Result {
    crate::concerns::before_actions(c, crate::concerns::Before::default()).await?;
    crate::concerns::sudo::require_sudo_mode(c).await?;
    Ok(c.html("confirmed"))
}

// The external identity provider is outside this test. The hand-off exercises the production
// subject/user/fresh-login checks with already-verified claims, not a mocked implementation.
async fn google_callback_probe(c: &mut Ctx) -> Result {
    crate::concerns::before_actions(c, crate::concerns::Before::default()).await?;
    let flow_user = c.param_str("flow_user").unwrap().parse().unwrap();
    let subject = c.param_str("subject").unwrap().to_string();
    let auth_time = c.param_str("auth_time").and_then(|time| time.parse().ok());
    crate::controllers::sudos::finish_google(c, flow_user, &subject, auth_time).await
}

async fn continuation_probe(c: &mut Ctx) -> Result {
    crate::concerns::before_actions(c, crate::concerns::Before::default()).await?;
    crate::controllers::sudos::continue_after_sudo(c).await
}

fn probe_router(a: &TestApp) -> axum::Router {
    let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
        crate::controllers::presenters::test_support::SEED_NOW
            .parse()
            .unwrap(),
    ));
    let kit = Kit::new(
        KitConfig::production(true),
        Arc::new(RailsCrypto::new(a.booted.app.secrets.clone())),
        clock,
        a.booted.app.clone(),
    );
    campfire_kit::app(
        axum::Router::new()
            .route("/probe", campfire_kit::get(gated_probe))
            .route("/callback", campfire_kit::get(google_callback_probe))
            .route("/continue", campfire_kit::get(continuation_probe)),
        kit,
    )
}

async fn probe(
    a: &TestApp,
    path: &str,
    state: Value,
) -> (StatusCode, axum::http::HeaderMap, String) {
    use axum::{body::Body, http::Request};
    let session = a
        .db()
        .write(|tx| {
            campfire_db::Session::start_with(
                tx,
                DAVID,
                campfire_db::NewSession {
                    two_factor_verified: true,
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let crypto = RailsCrypto::new(a.booted.app.secrets.clone());
    let cookie = format!(
        "session_token={}; _campfire_session={}",
        campfire_kit::cookies::escape(&crypto.sign_cookie("session_token", &session.token, None)),
        campfire_kit::cookies::escape(&crypto.encrypt_cookie("_campfire_session", &state, None))
    );
    let response = probe_router(a)
        .oneshot(
            Request::get(path)
                .header("host", "campfire.test")
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, headers, String::from_utf8(body.to_vec()).unwrap())
}

#[tokio::test]
async fn sudo_requires_an_integer_timestamp_strictly_within_fifteen_minutes() {
    let a = app().await;
    let now = crate::controllers::presenters::test_support::SEED_NOW
        .parse::<jiff::Timestamp>()
        .unwrap()
        .as_second();
    for (stamp, expected) in [
        (json!(now - 899), StatusCode::OK),
        (json!(now - 900), StatusCode::FOUND),
        (json!(now - 901), StatusCode::FOUND),
        (json!((now - 1).to_string()), StatusCode::FOUND),
        (json!(null), StatusCode::FOUND),
        (json!(now + 1), StatusCode::OK),
    ] {
        assert_eq!(
            probe(
                &a,
                "/probe",
                json!({"session_id": "ws9sudo", "sudo_verified_at": stamp})
            )
            .await
            .0,
            expected
        );
    }
}

#[tokio::test]
async fn sudo_google_seam_binds_the_member_subject_and_fresh_numeric_auth_time() {
    let a = app().await;
    let now = crate::controllers::presenters::test_support::SEED_NOW
        .parse::<jiff::Timestamp>()
        .unwrap()
        .as_second();
    a.db().write(|tx| {
        tx.conn().execute("DELETE FROM google_identities WHERE user_id = ?", [DAVID])?;
        tx.conn().execute("INSERT INTO google_identities (user_id, subject, email, created_at, updated_at) VALUES (?, 'ws9-google-subject', 'david@smartdata.net', ?, ?)", rusqlite::params![DAVID, tx.now(), tx.now()])?;
        Ok(())
    }).await.unwrap();
    for (flow_user, subject, auth_time, expected_path) in [
        (
            DAVID,
            "ws9-google-subject",
            (now - 329).to_string(),
            "/account/edit",
        ),
        (DAVID, "someone-else", now.to_string(), "/app/sudo/new"),
        (
            JASON,
            "ws9-google-subject",
            now.to_string(),
            "/users/me/profile",
        ),
        (
            DAVID,
            "ws9-google-subject",
            (now - 330).to_string(),
            "/app/sudo/new",
        ),
        (DAVID, "ws9-google-subject", String::new(), "/app/sudo/new"),
    ] {
        let path =
            format!("/callback?flow_user={flow_user}&subject={subject}&auth_time={auth_time}");
        let (status, headers, _) = probe(&a, &path, json!({"session_id":"ws9sudo", "sudo_pending_request":{"method":"GET","path":"/account/edit","params":null,"origin":"/"}})).await;
        assert_eq!(status, StatusCode::FOUND);
        assert_eq!(
            headers["location"],
            format!("http://campfire.test{expected_path}")
        );
    }
    let rows = a.db().read(|conn| {
        let mut stmt = conn.prepare("SELECT action, details FROM audit_logs WHERE action LIKE 'sudo.confirm.%' ORDER BY id")?;
        Ok(stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?.collect::<std::result::Result<Vec<_>, _>>()?)
    }).await.unwrap();
    assert_eq!(
        rows.len(),
        2,
        "stale/bad flows do not audit a confirmed identity"
    );
    assert_eq!(rows[0].0, "sudo.confirm.success");
    assert_eq!(
        serde_json::from_str::<Value>(&rows[1].1).unwrap(),
        json!({"verifier":"google","reason":"subject_mismatch"})
    );
}

#[tokio::test]
async fn sudo_continuation_consumes_gets_and_rejects_external_paths() {
    let a = app().await;
    for (path, expected) in [
        ("https://evil.test", "/"),
        ("//evil.test", "/"),
        ("/account/edit?x=1", "/account/edit?x=1"),
    ] {
        let (status, headers, _) = probe(&a, "/continue", json!({"session_id":"ws9sudo", "sudo_verified_at":1767268800, "sudo_pending_request":{"method":"GET","path":path,"origin":"/"}})).await;
        assert_eq!(status, StatusCode::FOUND);
        assert_eq!(
            headers["location"],
            format!("http://campfire.test{expected}")
        );
        let cookie = headers
            .get_all("set-cookie")
            .iter()
            .map(|h| h.to_str().unwrap())
            .find(|h| h.starts_with("_campfire_session="))
            .unwrap();
        let raw = cookie.split(';').next().unwrap().split_once('=').unwrap().1;
        let state = RailsCrypto::new(a.booted.app.secrets.clone())
            .decrypt_cookie(
                "_campfire_session",
                &rails_compat::cookies::unescape(raw),
                a.booted.app.clock.now(),
            )
            .unwrap();
        assert!(
            state.get("sudo_pending_request").is_none(),
            "consumed exactly once"
        );
    }
}

#[tokio::test]
async fn sudo_views_preserve_six_rails_form_contracts() {
    use crate::controllers::users::people_tests::retained;
    use askama::Template;
    use campfire_view_kit::helpers as h;
    struct Tokens;
    impl h::request_forgery::AuthenticityTokens for Tokens {
        fn global(&self) -> String {
            "GLOBAL".into()
        }
        fn for_form(&self, action: &str, method: &str) -> String {
            format!("{method}:{action}")
        }
    }
    let a = app().await;
    let goldens: Value =
        serde_json::from_str(include_str!("../../../../vectors/sudo_views.json")).unwrap();
    for (name, expected) in goldens.as_object().unwrap() {
        let actual = h::request_forgery::rendering_with(
            h::request_forgery::RequestSecrets {
                tokens: Box::new(Tokens),
                csp_nonce: Some("NONCE".into()),
            },
            || {
                crate::controllers::users::people_tests::render_with(
                    &a,
                    |_| {},
                    |ctx| {
                        let ctx = retained(ctx);
                        if name == "continue" {
                            campfire_retained::sudos::Continue { ctx: &ctx, method: "patch".into(), path: "/account/users/127326141?x=1&y=2".into(), params: json!({ "user": {"name": "David <&>", "tags": ["a",true,false,null,12,{"skip":1}], "empty":null}, "invalid name":"skip", "bad]name":"allowed" }) }.as_content().render().unwrap()
                        } else {
                            campfire_retained::sudos::New {
                                ctx: &ctx,
                                password: name.contains("password"),
                                totp: name.contains("totp"),
                                google: name.contains("google"),
                            }
                            .as_content()
                            .render()
                            .unwrap()
                        }
                    },
                )
            },
        );
        crate::form_contracts::assert_forms(name, &actual, expected.as_str().unwrap());
        if name != "continue" { crate::form_contracts::assert_text(&actual, "Confirm it's you"); }
    }
}
