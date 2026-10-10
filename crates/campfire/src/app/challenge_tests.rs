//! Real second-factor and sensitive self-service requests over the pinned seed.
use crate::controllers::presenters::test_support::{DAVID, JASON, Req, TestApp};
use axum::http::{Method, StatusCode};
use campfire_db::{Session, TwoFactorBackupCode, TwoFactorCredential, TwoFactorRememberedDevice};
use rails_compat::{ar_encryption::ArEncryption, totp};
async fn app() -> TestApp {
    TestApp::boot()
        .await
        .expect("build the current pinned WS19 default parity seed")
}
async fn prepare(a: &TestApp) -> (i64, String, Vec<String>) {
    let enc = ArEncryption::new(&a.booted.app.secrets);
    a.db().write(move |tx| {
        let c = TwoFactorCredential::for_user(tx.conn(), DAVID)?.unwrap();
        tx.conn().execute("UPDATE two_factor_credentials SET last_totp_at=NULL, consecutive_failures=0, lockout_count=0, locked_until=NULL WHERE id=?", [c.id])?;
        Ok((c.id, c.secret(&enc)?, TwoFactorBackupCode::regenerate_set(tx,c.id)?))
    }).await.unwrap()
}
async fn sign_in(
    b: &mut crate::controllers::presenters::test_support::Browser<'_>,
) -> crate::controllers::presenters::test_support::Reply {
    b.get("/session/new").await;
    b.write(Req::new(Method::POST, "/session").form(&[
        ("email_address", "david@37signals.com"),
        ("password", "secret123456"),
    ]))
    .await
}
#[tokio::test]
async fn challenge_first_factor_grants_no_session_and_totp_starts_verified_session() {
    let a = app().await;
    let (_, secret, _) = prepare(&a).await;
    let before_ids = a
        .db()
        .read(|c| {
            Ok(Session::for_user(c, DAVID)?
                .into_iter()
                .map(|s| s.id)
                .collect::<Vec<_>>())
        })
        .await
        .unwrap();
    let mut b = a.anonymous();
    assert_eq!(
        sign_in(&mut b).await.location(),
        Some("http://campfire.test/two_factor_challenge")
    );
    assert_eq!(
        a.db()
            .read(|c| Ok(Session::for_user(c, DAVID)?.len()))
            .await
            .unwrap(),
        before_ids.len()
    );
    assert_eq!(
        b.get("/").await.location(),
        Some("http://campfire.test/two_factor_challenge")
    );
    let prompt = b.get("/two_factor_challenge").await;
    assert_eq!(prompt.status, StatusCode::OK);
    assert_eq!(prompt.header("cache-control"), Some("no-store"));
    let code = totp::at(&secret, a.booted.app.clock.now().as_second()).unwrap();
    let ok = b
        .write(Req::new(Method::POST, "/two_factor_challenge").form(&[("code", &code)]))
        .await;
    assert_eq!(ok.location(), Some("http://campfire.test/app/"));
    a.db().read(move |c| {
        let created = Session::for_user(c,DAVID)?.into_iter().filter(|s| !before_ids.contains(&s.id)).collect::<Vec<_>>();
        assert_eq!(created.len(), 1);
        assert!(created[0].two_factor_verified(), "verify the newly created session, not a seeded session");
        assert_eq!(c.query_row("SELECT details FROM audit_logs WHERE action='session.sign_in.success' ORDER BY id DESC LIMIT 1",[],|r|r.get::<_,String>(0))?,"{\"method\":\"password\",\"two_factor\":\"totp\"}");
        Ok(())
    }).await.unwrap();
    assert_eq!(
        b.get("/two_factor_challenge").await.location(),
        Some("http://campfire.test/")
    );
    b.write(Req::new(Method::DELETE, "/session")).await;
    assert_eq!(
        sign_in(&mut b).await.location(),
        Some("http://campfire.test/two_factor_challenge")
    );
    assert_eq!(
        b.write(Req::new(Method::POST, "/two_factor_challenge").form(&[("code", &code)]))
            .await
            .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
}
#[tokio::test]
async fn challenge_backup_is_single_use_and_locked_attempts_spend_nothing() {
    let a = app().await;
    let (id, secret, codes) = prepare(&a).await;
    let mut b = a.anonymous();
    assert_eq!(
        sign_in(&mut b).await.location(),
        Some("http://campfire.test/two_factor_challenge")
    );
    assert_eq!(
        b.write(Req::new(Method::POST, "/two_factor_challenge").form(&[("code", &codes[0])]))
            .await
            .location(),
        Some("http://campfire.test/app/")
    );
    b.write(Req::new(Method::DELETE, "/session")).await;
    assert_eq!(
        sign_in(&mut b).await.location(),
        Some("http://campfire.test/two_factor_challenge")
    );
    for n in 1..=5 {
        let response = b
            .write(Req::new(Method::POST, "/two_factor_challenge").form(&[("code", &codes[0])]))
            .await;
        assert_eq!(
            response.status,
            if n == 5 {
                StatusCode::TOO_MANY_REQUESTS
            } else {
                StatusCode::UNPROCESSABLE_ENTITY
            }
        );
    }
    let code = totp::at(&secret, a.booted.app.clock.now().as_second()).unwrap();
    for code in [&code, &codes[1]] {
        assert_eq!(
            b.write(Req::new(Method::POST, "/two_factor_challenge").form(&[("code", code)]))
                .await
                .status,
            StatusCode::TOO_MANY_REQUESTS
        );
    }
    a.db().read(move |c| {
        assert_eq!(TwoFactorBackupCode::for_credential(c,id)?.iter().filter(|v|v.used_at.is_none()).count(),9);
        assert!(TwoFactorCredential::find(c,id)?.last_totp_at.is_none());
        assert_eq!(c.query_row("SELECT count(*) FROM activity_items WHERE user_id=? AND event_type='two_factor_lockout'",[DAVID],|r|r.get::<_,i64>(0))?,1);
        assert_eq!(c.query_row("SELECT count(*) FROM audit_logs WHERE action='sign_in.two_factor.lockout'",[],|r|r.get::<_,i64>(0))?,1);
        Ok(())
    }).await.unwrap();
}
#[tokio::test]
async fn challenge_requires_live_pending_user_and_csrf() {
    let a = app().await;
    let (_, secret, _) = prepare(&a).await;
    let mut b = a.anonymous();
    assert_eq!(
        b.get("/two_factor_challenge").await.location(),
        Some("http://campfire.test/session/new")
    );
    assert_eq!(
        sign_in(&mut b).await.location(),
        Some("http://campfire.test/two_factor_challenge")
    );
    let code = totp::at(&secret, a.booted.app.clock.now().as_second()).unwrap();
    assert_eq!(
        b.send(Req::new(Method::POST, "/two_factor_challenge").form(&[("code", &code)]))
            .await
            .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    a.db()
        .write(|tx| {
            tx.conn()
                .execute("UPDATE users SET status=1 WHERE id=?", [DAVID])?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        b.write(Req::new(Method::POST, "/two_factor_challenge").form(&[("code", &code)]))
            .await
            .location(),
        Some("http://campfire.test/session/new")
    );
}
#[tokio::test]
async fn self_service_requires_fresh_credentials_and_scopes_devices() {
    let a = app().await;
    let (id, _, codes) = prepare(&a).await;
    let mut b = a.sign_in(DAVID).await;
    let (device, _) = a
        .db()
        .write(|tx| TwoFactorRememberedDevice::create_for(tx, JASON, Some("Other"), None))
        .await
        .unwrap();
    let device_id = device.id;
    for code in ["", "wrong", &codes[0]] {
        assert_eq!(
            b.write(Req::new(Method::POST, "/two_factor_backup_codes").form(&[("reauth", code)]))
                .await
                .location(),
            Some("http://campfire.test/users/me/profile")
        );
    }
    assert!(
        a.db()
            .write(move |tx| TwoFactorBackupCode::consume(tx, id, &codes[0]))
            .await
            .unwrap(),
        "reauthentication never consumes backups"
    );
    assert_eq!(
        b.write(
            Req::new(Method::POST, "/two_factor_backup_codes").form(&[("reauth", "secret123456")])
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        b.write(
            Req::new(
                Method::DELETE,
                &format!("/two_factor_remembered_devices/{}", device.id)
            )
            .form(&[("reauth", "secret123456")])
        )
        .await
        .location(),
        Some("http://campfire.test/users/me/profile")
    );
    assert!(
        a.db()
            .read(move |c| Ok(TwoFactorRememberedDevice::for_user(c, JASON)?
                .iter()
                .any(|d| d.id == device_id)))
            .await
            .unwrap()
    );
}
#[tokio::test]
async fn disable_refuses_unverified_session_even_with_correct_password() {
    let a = app().await;
    prepare(&a).await;
    let mut b = a.sign_in(DAVID).await;
    b.get("/sudo/new").await; // establish CSRF before deliberately making this session stale
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
    let response = b
        .write(Req::new(Method::DELETE, "/two_factor_setup").form(&[("reauth", "secret123456")]))
        .await;
    assert_eq!(
        response.location(),
        Some("http://campfire.test/session/new")
    );
    assert!(
        a.db()
            .read(|c| Ok(TwoFactorCredential::for_user(c, DAVID)?.unwrap().enabled()))
            .await
            .unwrap()
    );
}
#[tokio::test]
async fn disable_resets_all_verification_remembered_devices_and_audits() {
    let a = app().await;
    prepare(&a).await;
    let mut b = a.sign_in(DAVID).await;
    a.db()
        .write(|tx| TwoFactorRememberedDevice::create_for(tx, DAVID, None, None))
        .await
        .unwrap();
    assert_eq!(
        b.write(Req::new(Method::DELETE, "/two_factor_setup").form(&[("reauth", "wrong")]))
            .await
            .location(),
        Some("http://campfire.test/users/me/profile")
    );
    assert_eq!(
        b.write(Req::new(Method::DELETE, "/two_factor_setup").form(&[("reauth", "secret123456")]))
            .await
            .location(),
        Some("http://campfire.test/two_factor_setup")
    );
    a.db().read(|c| {
        assert!(TwoFactorCredential::for_user(c,DAVID)?.is_none());
        assert!(TwoFactorRememberedDevice::for_user(c,DAVID)?.is_empty());
        assert!(Session::for_user(c,DAVID)?.iter().all(|s|!s.two_factor_verified()));
        assert_eq!(c.query_row("SELECT count(*) FROM audit_logs WHERE action='two_factor.disable' AND target_id=?",[DAVID],|r|r.get::<_,i64>(0))?,1);Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn challenge_pending_expires_at_ten_minutes_and_reset_cannot_complete() {
    use std::sync::Arc;
    let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
        "2026-03-02T16:00:00Z".parse().unwrap(),
    ));
    let a = TestApp::boot_with_clock(clock.clone())
        .await
        .expect("build the default parity seed");
    prepare(&a).await;
    let mut b = a.anonymous();
    assert_eq!(
        sign_in(&mut b).await.location(),
        Some("http://campfire.test/two_factor_challenge")
    );
    clock.advance(jiff::SignedDuration::from_secs(600));
    assert_eq!(
        b.get("/two_factor_challenge").await.status,
        StatusCode::OK,
        "the expiry second remains valid in Rails"
    );
    clock.advance(jiff::SignedDuration::from_secs(1));
    assert_eq!(
        b.get("/two_factor_challenge").await.location(),
        Some("http://campfire.test/session/new")
    );
    assert_eq!(
        sign_in(&mut b).await.location(),
        Some("http://campfire.test/two_factor_challenge")
    );
    a.db()
        .write(|tx| campfire_db::User::find(tx.conn(), DAVID)?.reset_two_factor(tx))
        .await
        .unwrap();
    assert_eq!(
        b.get("/two_factor_challenge").await.location(),
        Some("http://campfire.test/session/new")
    );
}
#[tokio::test]
async fn challenge_lockouts_escalate_refresh_one_item_and_limits_block_across_ips() {
    use std::sync::Arc;
    let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
        "2026-03-02T16:00:00Z".parse().unwrap(),
    ));
    let a = TestApp::boot_with_clock(clock.clone())
        .await
        .expect("build the default parity seed");
    let (id, secret, _) = prepare(&a).await;
    let mut b = a.anonymous();
    sign_in(&mut b).await;
    for window in 0..2 {
        for n in 0..5 {
            let response = b
                .write(
                    Req::new(Method::POST, "/two_factor_challenge")
                        .header("x-forwarded-for", &format!("198.18.{window}.{n}"))
                        .form(&[("code", "invalid")]),
                )
                .await;
            assert_eq!(
                response.status,
                if n == 4 {
                    StatusCode::TOO_MANY_REQUESTS
                } else {
                    StatusCode::UNPROCESSABLE_ENTITY
                }
            );
        }
        a.db().write(|tx| {tx.conn().execute("UPDATE activity_items SET read_at=? WHERE user_id=? AND event_type='two_factor_lockout'",rusqlite::params![tx.now(),DAVID])?;Ok(())}).await.unwrap();
        if window == 0 {
            clock.advance(jiff::SignedDuration::from_secs(120));
        }
    }
    let response = b
        .write(
            Req::new(Method::POST, "/two_factor_challenge")
                .header("x-forwarded-for", "198.18.7.7")
                .form(&[("code", "invalid")]),
        )
        .await;
    assert_eq!(response.status, StatusCode::TOO_MANY_REQUESTS);
    assert!(
        response.text().contains("Too many attempts."),
        "per-user limiter fires across all IPs"
    );
    a.db().read(move |c| {
        assert_eq!(TwoFactorCredential::find(c,id)?.lockout_count,2);
        assert_eq!(c.query_row("SELECT count(*) FROM activity_items WHERE user_id=? AND event_type='two_factor_lockout'",[DAVID],|r|r.get::<_,i64>(0))?,1);Ok(())
    }).await.unwrap();
    clock.advance(jiff::SignedDuration::from_secs(15 * 60));
    sign_in(&mut b).await;
    let code = totp::at(&secret, a.booted.app.clock.now().as_second()).unwrap();
    assert_eq!(
        b.write(Req::new(Method::POST, "/two_factor_challenge").form(&[("code", &code)]))
            .await
            .location(),
        Some("http://campfire.test/app/")
    );
    a.db()
        .read(move |c| {
            let credential = TwoFactorCredential::find(c, id)?;
            assert_eq!(credential.lockout_count, 0);
            assert_eq!(credential.consecutive_failures, 0);
            assert!(credential.locked_until.is_none());
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn remembered_cookie_is_secure_and_revocation_restores_challenge() {
    let a = app().await;
    let (_, secret, _) = prepare(&a).await;
    let mut b = a.anonymous();
    b.send(Req::new(Method::GET, "/session/new").header("x-forwarded-proto", "https"))
        .await;
    assert_eq!(
        b.write(
            Req::new(Method::POST, "/session")
                .header("x-forwarded-proto", "https")
                .form(&[
                    ("email_address", "david@37signals.com"),
                    ("password", "secret123456")
                ])
        )
        .await
        .location(),
        Some("https://campfire.test/two_factor_challenge")
    );
    let code = totp::at(&secret, a.booted.app.clock.now().as_second()).unwrap();
    let result = b
        .write(
            Req::new(Method::POST, "/two_factor_challenge")
                .header("x-forwarded-proto", "https")
                .form(&[("code", &code), ("remember_device", "1")]),
        )
        .await;
    assert_eq!(result.location(), Some("https://campfire.test/app/"));
    let cookie = result
        .headers
        .get_all("set-cookie")
        .iter()
        .map(|v| v.to_str().unwrap())
        .find(|s| s.starts_with("two_factor_remember="))
        .expect("remember cookie");
    for flag in ["secure", "httponly", "samesite=lax", "expires="] {
        assert!(cookie.to_lowercase().contains(flag));
    }
    b.write(Req::new(Method::DELETE, "/session").header("x-forwarded-proto", "https"))
        .await;
    b.send(Req::new(Method::GET, "/session/new").header("x-forwarded-proto", "https"))
        .await;
    let result = b
        .write(
            Req::new(Method::POST, "/session")
                .header("x-forwarded-proto", "https")
                .form(&[
                    ("email_address", "david@37signals.com"),
                    ("password", "secret123456"),
                ]),
        )
        .await;
    assert_eq!(result.location(), Some("https://campfire.test/app/"));
    assert_eq!(
        b.write(
            Req::new(Method::DELETE, "/two_factor_remembered_devices")
                .form(&[("reauth", "secret123456")])
        )
        .await
        .location(),
        Some("http://campfire.test/users/me/profile")
    );
    b.write(Req::new(Method::DELETE, "/session")).await;
    assert_eq!(
        sign_in(&mut b).await.location(),
        Some("http://campfire.test/two_factor_challenge")
    );
}
#[tokio::test]
async fn totp_reauthentication_is_spent_and_never_accepts_backup_or_wrong_password() {
    let a = app().await;
    let (id, secret, _) = prepare(&a).await;
    let mut b = a.sign_in(DAVID).await;
    let code = totp::at(&secret, a.booted.app.clock.now().as_second()).unwrap();
    let result = b
        .write(Req::new(Method::POST, "/two_factor_backup_codes").form(&[("reauth", &code)]))
        .await;
    assert_eq!(result.status, StatusCode::OK);
    assert_eq!(result.header("cache-control"), Some("no-store"));
    assert_eq!(
        b.write(Req::new(Method::POST, "/two_factor_backup_codes").form(&[("reauth", &code)]))
            .await
            .location(),
        Some("http://campfire.test/users/me/profile")
    );
    assert!(
        a.db()
            .read(move |c| Ok(TwoFactorCredential::find(c, id)?.last_totp_at.is_some()))
            .await
            .unwrap()
    );
}

// The external verifier belongs to WS14. This route supplies already-verified claims to the
// production hand-off; it does not model Google signatures or browser-bound OAuth state.
async fn google_probe(c: &mut campfire_kit::Ctx) -> campfire_kit::Result {
    crate::concerns::before_actions(c, crate::concerns::Before::default()).await?;
    let owner = c.param_str("owner").unwrap().parse().unwrap();
    let subject = c.param_str("sub").unwrap().to_string();
    let at = c.param_str("at").and_then(|s| s.parse().ok());
    crate::controllers::two_factor::finish_google_reauthentication(c, owner, &subject, at).await
}
#[tokio::test]
async fn google_step_up_seam_checks_owner_subject_freshness_and_consumes_once() {
    use axum::{body::Body, http::Request};
    use campfire_kit::{Crypto, Kit, KitConfig, RailsCrypto};
    use std::sync::Arc;
    use tower::ServiceExt;
    let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
        "2026-03-02T16:00:00Z".parse().unwrap(),
    ));
    let a = TestApp::boot_with_clock(clock.clone())
        .await
        .expect("build the default parity seed");
    prepare(&a).await;
    a.db().write(|tx| {
        tx.conn().execute("DELETE FROM google_identities WHERE user_id=?",[DAVID])?;
        tx.conn().execute("INSERT INTO google_identities(user_id,subject,email,created_at,updated_at) VALUES(?, 'ws9-sub', 'david@smartdata.net', ?, ?)",rusqlite::params![DAVID,tx.now(),tx.now()])?;Ok(())
    }).await.unwrap();
    let session = a
        .db()
        .write(|tx| {
            Session::start_with(
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
    let session_cookie = format!(
        "session_token={}",
        campfire_kit::cookies::escape(&crypto.sign_cookie("session_token", &session.token, None))
    );
    let router = campfire_kit::app(
        axum::Router::new().route("/callback", campfire_kit::get(google_probe)),
        Kit::new(
            KitConfig::production(true),
            Arc::new(RailsCrypto::new(a.booted.app.secrets.clone())),
            clock.clone(),
            a.booted.app.clone(),
        ),
    );
    let now = a.booted.app.clock.now().as_second();
    for (owner, subject, at, armed) in [
        (DAVID, "wrong", now.to_string(), false),
        (JASON, "ws9-sub", now.to_string(), false),
        (DAVID, "ws9-sub", String::new(), false),
        (DAVID, "ws9-sub", (now - 330).to_string(), false),
        (DAVID, "ws9-sub", (now - 329).to_string(), true),
    ] {
        let path = format!("/callback?owner={owner}&sub={subject}&at={at}");
        let response = router
            .clone()
            .oneshot(
                Request::get(path)
                    .header("host", "campfire.test")
                    .header("cookie", &session_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(
            response.headers()["location"],
            "http://campfire.test/app/settings/security"
        );
        let state_cookie = response
            .headers()
            .get_all("set-cookie")
            .iter()
            .map(|h| h.to_str().unwrap())
            .find(|s| s.starts_with("_campfire_session="))
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_string();
        let state = crypto
            .decrypt_cookie(
                "_campfire_session",
                &rails_compat::cookies::unescape(state_cookie.split_once('=').unwrap().1),
                a.booted.app.clock.now(),
            )
            .unwrap();
        assert_eq!(state.get("two_factor_reauthenticated_at").is_some(), armed);
        if armed {
            let mut b = a.anonymous();
            b.absorb_cookie_header(&format!("{session_cookie}; {state_cookie}"));
            b.get("/sudo/new").await;
            assert_eq!(
                b.write(
                    Req::new(Method::POST, "/two_factor_backup_codes")
                        .form(&[("reauth", "incorrect")])
                )
                .await
                .location(),
                Some("http://campfire.test/users/me/profile")
            );
            let response = b
                .write(Req::new(Method::POST, "/two_factor_backup_codes"))
                .await;
            assert_eq!(
                response.status,
                StatusCode::OK,
                "a wrong in-request credential preserves the pending Google step-up"
            );
            assert_eq!(
                b.write(Req::new(Method::POST, "/two_factor_backup_codes"))
                    .await
                    .location(),
                Some("http://campfire.test/users/me/profile")
            );
        }
    }
}
#[tokio::test]
async fn self_service_limits_and_google_unavailability_are_enforced() {
    let a = app().await;
    prepare(&a).await;
    let mut b = a.sign_in(DAVID).await;
    let result = b
        .write(Req::new(Method::POST, "/two_factor_reauthentication"))
        .await;
    assert_eq!(
        result.location(),
        Some("http://campfire.test/app/settings/security")
    );
    for n in 0..10 {
        assert_eq!(
            b.write(
                Req::new(Method::POST, "/two_factor_backup_codes")
                    .header("x-forwarded-for", &format!("198.18.1.{n}"))
                    .form(&[("reauth", "wrong")])
            )
            .await
            .location(),
            Some("http://campfire.test/users/me/profile")
        );
    }
    let result = b
        .write(
            Req::new(Method::POST, "/two_factor_backup_codes")
                .header("x-forwarded-for", "198.18.3.3")
                .form(&[("reauth", "secret123456")]),
        )
        .await;
    assert_eq!(
        result.location(),
        Some("http://campfire.test/users/me/profile")
    );
    assert_eq!(
        a.db()
            .read(|c| Ok(c.query_row(
                "SELECT count(*) FROM audit_logs WHERE action='two_factor.backup_codes.regenerate'",
                [],
                |r| r.get::<_, i64>(0)
            )?))
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn account_json_lists_only_live_devices_and_reauthentication_options() {
    let a = app().await;
    prepare(&a).await;
    let (live, _) = a
        .db()
        .write(|tx| {
            TwoFactorRememberedDevice::create_for(tx, DAVID, Some("WS9Browser"), Some("1.2.3.4"))
        })
        .await
        .unwrap();
    let (expired, _) = a
        .db()
        .write(|tx| {
            let (device, token) =
                TwoFactorRememberedDevice::create_for(tx, DAVID, Some("ExpiredBrowser"), None)?;
            tx.conn().execute(
                "UPDATE two_factor_remembered_devices SET expires_at=? WHERE id=?",
                rusqlite::params![tx.now(), device.id],
            )?;
            Ok((device, token))
        })
        .await
        .unwrap();
    let mut b = a.sign_in(DAVID).await;
    let page = b.get("/api/v1/settings/account").await;
    assert_eq!(page.status, StatusCode::OK);
    let payload: serde_json::Value = serde_json::from_str(&page.text()).unwrap();
    let panel = &payload["twoFactor"];
    let devices = panel["devices"].as_array().unwrap();
    assert!(devices.iter().any(|device| device["id"] == live.id));
    assert!(!devices.iter().any(|device| device["id"] == expired.id));
    assert_eq!(panel["google"], false);
    assert_eq!(panel["hasPassword"], true);
}

#[tokio::test]
async fn password_change_revokes_remembered_devices_but_name_change_keeps_them() {
    let a = app().await;
    prepare(&a).await;
    a.db()
        .write(|tx| TwoFactorRememberedDevice::create_for(tx, DAVID, None, None))
        .await
        .unwrap();
    let mut b = a.sign_in(DAVID).await;
    assert_eq!(
        b.write(Req::new(Method::PATCH, "/users/me/profile").form(&[("user[name]", "David WS9")]))
            .await
            .location(),
        Some("http://campfire.test/users/me/profile")
    );
    assert!(
        !a.db()
            .read(|c| Ok(TwoFactorRememberedDevice::for_user(c, DAVID)?.is_empty()))
            .await
            .unwrap()
    );
    assert_eq!(
        b.write(
            Req::new(Method::PATCH, "/users/me/profile")
                .form(&[("user[password]", "new-secret-123456")])
        )
        .await
        .location(),
        Some("http://campfire.test/users/me/profile")
    );
    a.db().read(|c| {
        assert!(TwoFactorRememberedDevice::for_user(c,DAVID)?.is_empty());
        assert_eq!(c.query_row("SELECT details FROM audit_logs WHERE action='user.password.change' AND target_id=?",[DAVID],|r|r.get::<_,String>(0))?,"{}");Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn configured_lockout_uses_ws10_queue_and_enqueue_failure_rolls_back_lockout() {
    let clock = std::sync::Arc::new(campfire_kit::clock::FrozenClock::new(
        "2026-03-02T16:00:00Z".parse().unwrap(),
    ));
    let a = TestApp::boot_with_clock_and_env(
        clock,
        &[
            ("SMTP_ADDRESS", "127.0.0.1"),
            ("SMTP_PORT", "9"),
            ("SMTP_ENABLE_STARTTLS", "false"),
            ("MAILER_FROM", "security@campfire.test"),
            ("APP_URL", "http://campfire.test"),
        ],
    )
    .await
    .expect("build the default parity seed");
    let (credential_id, _, _) = prepare(&a).await;
    let mut b = a.anonymous();
    sign_in(&mut b).await;
    for _ in 0..4 {
        assert_eq!(
            b.write(Req::new(Method::POST, "/two_factor_challenge").form(&[("code", "wrong")]))
                .await
                .status,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    a.db()
        .write(|tx| {
            tx.conn().execute_batch(
                "CREATE TABLE ws9_mail_observed(args TEXT NOT NULL);
            CREATE TRIGGER observe_ws9_mail AFTER INSERT ON background_jobs
            WHEN NEW.job_class='Smartfire::MailDeliveryJob'
            BEGIN INSERT INTO ws9_mail_observed VALUES(NEW.arguments); END;
            CREATE TRIGGER reject_ws9_mail BEFORE INSERT ON background_jobs
            WHEN NEW.job_class='Smartfire::MailDeliveryJob'
            BEGIN SELECT RAISE(ABORT, 'queue unavailable'); END",
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        b.write(Req::new(Method::POST, "/two_factor_challenge").form(&[("code", "wrong")]))
            .await
            .status,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    a.db().read(move |conn| {
        let credential = TwoFactorCredential::find(conn, credential_id)?;
        assert_eq!(credential.consecutive_failures, 4);
        assert!(credential.locked_until.is_none());
        assert_eq!(conn.query_row("SELECT count(*) FROM activity_items WHERE user_id=? AND event_type='two_factor_lockout'", [DAVID], |r| r.get::<_,i64>(0))?, 0);
        assert_eq!(conn.query_row("SELECT count(*) FROM audit_logs WHERE action='sign_in.two_factor.lockout'", [], |r| r.get::<_,i64>(0))?, 0);
        Ok(())
    }).await.unwrap();
    a.db()
        .write(|tx| {
            tx.conn().execute_batch("DROP TRIGGER reject_ws9_mail")?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        b.write(Req::new(Method::POST, "/two_factor_challenge").form(&[("code", "wrong")]))
            .await
            .status,
        StatusCode::TOO_MANY_REQUESTS
    );
    a.db()
        .read(move |conn| {
            assert!(
                TwoFactorCredential::find(conn, credential_id)?
                    .locked_until
                    .is_some()
            );
            let args: String =
                conn.query_row("SELECT args FROM ws9_mail_observed", [], |r| r.get(0))?;
            let args: serde_json::Value = serde_json::from_str(&args).unwrap();
            assert_eq!(
                args,
                serde_json::json!({"notification":{"Lockout":{"user_id":DAVID}}})
            );
            assert_eq!(
                conn.query_row("SELECT count(*) FROM ws9_mail_observed", [], |r| r
                    .get::<_, i64>(0))?,
                1
            );
            Ok(())
        })
        .await
        .unwrap();
}
