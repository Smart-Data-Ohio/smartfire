//! Real HTTP/DB gates: enrollment, late restores, idle expiry and framework uploads.
use crate::controllers::presenters::test_support::{ALL_TALK, DAVID, JASON, Req, TestApp};
use axum::http::{Method, StatusCode};
use campfire_db::{Session, TwoFactorCredential, User};
use std::sync::Arc;

async fn app() -> TestApp {
    TestApp::boot()
        .await
        .expect("build the pinned WS19 default parity seed")
}
async fn unenroll(a: &TestApp) {
    a.db()
        .write(|tx| User::find(tx.conn(), DAVID)?.reset_two_factor(tx))
        .await
        .unwrap();
}
async fn login(a: &TestApp) -> crate::controllers::presenters::test_support::Browser<'_> {
    let mut b = a.anonymous();
    b.get("/session/new").await;
    assert_eq!(
        b.write(Req::new(Method::POST, "/session").form(&[
            ("email_address", "david@37signals.com"),
            ("password", "secret123456")
        ]))
        .await
        .location(),
        Some("http://campfire.test/app/")
    );
    b
}
async fn stale(a: &TestApp) -> crate::controllers::presenters::test_support::Browser<'_> {
    let mut b = a.sign_in(DAVID).await;
    b.get("/users/me/profile").await;
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE sessions SET two_factor_verified_at=NULL WHERE user_id=?",
                [DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    b
}

#[tokio::test]
async fn unenrolled_password_session_is_gated_on_every_application_page_and_format() {
    let a = app().await;
    unenroll(&a).await;
    let mut b = login(&a).await;
    for path in [
        "/".into(),
        format!("/rooms/{ALL_TALK}"),
        "/users/me/profile".into(),
    ] {
        assert_eq!(
            b.get(&path).await.location(),
            Some("http://campfire.test/two_factor_setup"),
            "{path}"
        );
    }
    let json = b
        .send(Req::new(Method::GET, "/").header("accept", "application/json"))
        .await;
    assert_eq!(json.status, StatusCode::FORBIDDEN);
    assert_eq!(
        json.json(),
        serde_json::json!({"error":{"_tag":"TwoFactorRequired","message":"Set up two-step sign-in to continue","requirement":{"kind":"setup","location":"http://campfire.test/two_factor_setup"}}})
    );
    assert_eq!(json.header("cache-control"), Some("no-store"));
    assert_eq!(
        b.send(
            Req::new(Method::GET, &format!("/rooms/{ALL_TALK}"))
                .header("accept", "text/vnd.turbo-stream.html")
        )
        .await
        .location(),
        Some("http://campfire.test/two_factor_setup")
    );
    // User-supplied controller/action parameters cannot turn this into an exemption.
    assert_eq!(
        b.get("/users/me/profile?controller=two_factor/setups&action=show")
            .await
            .location(),
        Some("http://campfire.test/two_factor_setup")
    );
}

#[tokio::test]
async fn enrollment_exemptions_allow_setup_static_challenge_and_sign_out_only() {
    let a = app().await;
    unenroll(&a).await;
    let mut b = login(&a).await;
    assert_eq!(b.get("/two_factor_setup").await.status, StatusCode::OK);
    assert_eq!(b.get("/webmanifest.json").await.status, StatusCode::OK);
    assert_eq!(b.get("/service-worker.js").await.status, StatusCode::OK);
    assert_eq!(b.get("/up").await.status, StatusCode::OK);
    assert_eq!(
        b.get("/two_factor_challenge").await.location(),
        Some("http://campfire.test/")
    );
    assert_eq!(
        b.write(Req::new(Method::DELETE, "/two_factor_setup").form(&[("reauth", "secret123456")]))
            .await
            .location(),
        Some("http://campfire.test/two_factor_setup")
    );
    assert_eq!(
        b.write(Req::new(Method::DELETE, "/session"))
            .await
            .location(),
        Some("http://campfire.test/")
    );
    assert_eq!(
        b.get("/two_factor_setup").await.location(),
        Some("http://campfire.test/session/new")
    );
}

#[tokio::test]
async fn stale_enrolled_sessions_are_destroyed_for_html_json_and_stream() {
    for accept in [
        "text/html",
        "application/json",
        "text/vnd.turbo-stream.html",
    ] {
        let a = app().await;
        let mut b = stale(&a).await;
        let before = a
            .db()
            .read(|c| Ok(Session::for_user(c, DAVID)?.len()))
            .await
            .unwrap();
        let response = b
            .send(Req::new(Method::GET, "/").header("accept", accept))
            .await;
        if accept.contains("html") {
            assert_eq!(
                response.location(),
                Some("http://campfire.test/session/new")
            );
            assert!(
                b.get("/session/new")
                    .await
                    .text()
                    .contains("Sign in again to verify two-step sign-in.")
            );
        } else {
            assert_eq!(response.status, StatusCode::UNAUTHORIZED);
            assert!(response.body.is_empty());
        }
        assert_eq!(
            a.db()
                .read(|c| Ok(Session::for_user(c, DAVID)?.len()))
                .await
                .unwrap(),
            before - 1
        );
        assert_eq!(
            b.get("/").await.location(),
            Some("http://campfire.test/session/new")
        );
    }
}

#[tokio::test]
async fn late_restores_cannot_bypass_enrollment_and_challenge_is_still_exempt() {
    let a = app().await;
    unenroll(&a).await;
    let code = a
        .db()
        .read(|c| {
            c.query_row("SELECT join_code FROM accounts LIMIT 1", [], |r| {
                r.get::<_, String>(0)
            })
            .map_err(Into::into)
        })
        .await
        .unwrap();
    let mut b = login(&a).await;
    assert_eq!(
        b.get(&format!("/join/{code}")).await.location(),
        Some("http://campfire.test/two_factor_setup")
    );
    let mut verified = a.sign_in(DAVID).await;
    assert_eq!(
        verified.get(&format!("/join/{code}")).await.location(),
        Some("http://campfire.test/")
    );
    assert_eq!(
        verified.get("/two_factor_challenge").await.location(),
        Some("http://campfire.test/")
    );
    // A stale enrolled session may reach the challenge, but the redirect's next request is gated.
    let a = app().await;
    let mut b = stale(&a).await;
    assert_eq!(
        b.get("/two_factor_challenge").await.location(),
        Some("http://campfire.test/")
    );
    assert_eq!(
        b.get("/").await.location(),
        Some("http://campfire.test/session/new")
    );
}

#[tokio::test]
async fn verified_sessions_browse_even_without_a_credential() {
    let a = app().await;
    unenroll(&a).await;
    let mut b = a.sign_in(DAVID).await;
    assert_eq!(b.get("/api/v1/settings/account").await.status, StatusCode::OK);
    assert_ne!(
        b.get("/").await.location(),
        Some("http://campfire.test/two_factor_setup")
    );
}

#[tokio::test]
async fn admin_idle_expiry_is_strict_configurable_and_does_not_expire_members() {
    for timeout in [None, Some("30")] {
        let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
            "2026-03-02T16:00:00Z".parse().unwrap(),
        ));
        let env = timeout
            .map(|v| vec![("ADMIN_SESSION_IDLE_TIMEOUT_DAYS", v)])
            .unwrap_or_default();
        let a = TestApp::boot_with_clock_and_env(clock.clone(), &env)
            .await
            .expect("default seed");
        let mut b = a.sign_in(DAVID).await;
        let days = if timeout.is_some() { 30 } else { 7 };
        a.db()
            .write(move |tx| {
                tx.conn().execute(
                    "UPDATE sessions SET last_active_at=? WHERE user_id=?",
                    rusqlite::params![
                        tx.now().ago(jiff::SignedDuration::from_secs(days * 86400)),
                        DAVID
                    ],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        assert_eq!(
            b.get("/api/v1/settings/account").await.status,
            StatusCode::OK,
            "exact expiry boundary is valid"
        );
        clock.advance(jiff::SignedDuration::from_secs(days * 86400 + 1));
        let before = a
            .db()
            .read(|c| Ok(Session::for_user(c, DAVID)?.len()))
            .await
            .unwrap();
        assert_eq!(
            b.get("/users/me/profile").await.location(),
            Some("http://campfire.test/session/new")
        );
        assert_eq!(
            a.db()
                .read(|c| Ok(Session::for_user(c, DAVID)?.len()))
                .await
                .unwrap(),
            before - 1
        );
        a.db()
            .write(|tx| {
                tx.conn()
                    .execute("UPDATE users SET role=0 WHERE id=?", [JASON])?;
                Ok(())
            })
            .await
            .unwrap();
        let mut member = a.sign_in(JASON).await;
        a.db()
            .write(|tx| {
                tx.conn().execute(
                    "UPDATE sessions SET last_active_at=? WHERE user_id=?",
                    rusqlite::params![
                        tx.now().ago(jiff::SignedDuration::from_secs(365 * 86400)),
                        JASON
                    ],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        assert_eq!(member.get("/api/v1/settings/account").await.status, StatusCode::OK);
    }
}

fn metadata() -> Req {
    Req::new(Method::POST, "/rails/active_storage/direct_uploads").header("content-type", "application/json").body(
        serde_json::json!({"blob":{"filename":"hi.txt","byte_size":6,"checksum":"Wo3TrQdWqT3tcrgjsZ3Ydw==","content_type":"application/octet-stream"}}).to_string()
    )
}
async fn blobs(a: &TestApp) -> i64 {
    a.db()
        .read(|c| {
            c.query_row("SELECT count(*) FROM active_storage_blobs", [], |r| {
                r.get(0)
            })
            .map_err(Into::into)
        })
        .await
        .unwrap()
}
#[tokio::test]
async fn storage_metadata_rejects_anonymous_unenrolled_and_stale_without_writes() {
    let a = app().await;
    let mut anonymous = a.anonymous();
    anonymous.get("/session/new").await;
    let count = blobs(&a).await;
    assert_eq!(
        anonymous.write(metadata()).await.status,
        StatusCode::UNAUTHORIZED
    );
    unenroll(&a).await;
    let mut unenrolled = login(&a).await;
    assert_eq!(
        unenrolled.write(metadata()).await.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(blobs(&a).await, count);
    let a = app().await;
    let mut b = stale(&a).await;
    let count = blobs(&a).await;
    assert_eq!(b.write(metadata()).await.status, StatusCode::UNAUTHORIZED);
    assert_eq!(blobs(&a).await, count);
}

#[tokio::test]
async fn storage_upload_gate_and_public_download_survive_real_byte_round_trip() {
    let a = app().await;
    let mut uploader = a.sign_in(DAVID).await;
    let result = uploader.write(metadata()).await;
    assert_eq!(result.status, StatusCode::OK, "{}", result.text());
    let json = result.json();
    let upload = json["direct_upload"]["url"].as_str().unwrap();
    let mut unverified = stale(&a).await;
    assert_eq!(
        unverified
            .write(
                Req::new(Method::PUT, upload)
                    .header("content-type", "application/octet-stream")
                    .header("content-length", "6")
                    .body("hello!")
            )
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    let mut anonymous = a.anonymous();
    anonymous.get("/session/new").await;
    assert_eq!(
        anonymous
            .write(
                Req::new(Method::PUT, upload)
                    .header("content-type", "application/octet-stream")
                    .header("content-length", "6")
                    .body("hello!")
            )
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    // stale() cleared every session; restore only the uploader's verification for this positive path.
    let id = json["id"].as_i64().unwrap();
    let mut uploader = a.sign_in(DAVID).await;
    assert_eq!(
        uploader
            .write(
                Req::new(Method::PUT, upload)
                    .header("content-type", "application/octet-stream")
                    .header("content-length", "6")
                    .body("hello!")
            )
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    let storage = a.booted.app.storage.clone();
    let path = a
        .db()
        .read(move |c| {
            let blob = campfire_storage::Blob::find(c, id)
                .map_err(|e| campfire_db::Error::Other(e.to_string()))?
                .unwrap();
            Ok(storage.service.url_path(
                &*storage.verifier,
                &blob.key,
                None,
                &blob.filename,
                Some("text/plain"),
                "inline",
            ))
        })
        .await
        .unwrap();
    let result = anonymous.get(&path).await;
    assert_eq!(result.status, StatusCode::OK);
    assert_eq!(result.body, b"hello!");
}

#[tokio::test]
async fn storage_accepts_actual_completed_second_factor_and_nonhuman_sessions() {
    let a = app().await;
    let enc = rails_compat::ar_encryption::ArEncryption::new(&a.booted.app.secrets);
    let secret = a
        .db()
        .write(move |tx| {
            let cred = TwoFactorCredential::for_user(tx.conn(), DAVID)?.unwrap();
            tx.conn().execute(
                "UPDATE two_factor_credentials SET last_totp_at=NULL WHERE id=?",
                [cred.id],
            )?;
            cred.secret(&enc)
        })
        .await
        .unwrap();
    let mut b = a.anonymous();
    b.get("/session/new").await;
    b.write(Req::new(Method::POST, "/session").form(&[
        ("email_address", "david@37signals.com"),
        ("password", "secret123456"),
    ]))
    .await;
    let code = rails_compat::totp::at(&secret, a.booted.app.clock.now().as_second()).unwrap();
    assert_eq!(
        b.write(Req::new(Method::POST, "/two_factor_challenge").form(&[("code", &code)]))
            .await
            .location(),
        Some("http://campfire.test/app/")
    );
    assert_eq!(b.write(metadata()).await.status, StatusCode::OK);
    let mut bot = a
        .sign_in(crate::controllers::presenters::test_support::BENDER)
        .await;
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE sessions SET two_factor_verified_at=NULL WHERE user_id=?",
                [crate::controllers::presenters::test_support::BENDER],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(bot.write(metadata()).await.status, StatusCode::OK);
}
