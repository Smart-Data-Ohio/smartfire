//! Real authorization, reset/audit atomicity, socket recovery and rate-limit windows.
use crate::controllers::presenters::test_support::{BENDER, DAVID, KEVIN, Req, TestApp};
use axum::http::{Method, StatusCode};
use campfire_db::{
    Session, TwoFactorBackupCode, TwoFactorCredential, TwoFactorRememberedDevice, User,
};
use std::sync::Arc;
fn path(id: i64) -> String {
    format!("/account/users/{id}/two_factor_reset")
}
async fn app() -> TestApp {
    TestApp::boot()
        .await
        .expect("build the pinned WS19 default seed")
}
async fn audit(a: &TestApp) -> i64 {
    a.db()
        .read(|c| {
            c.query_row(
                "SELECT count(*) FROM audit_logs WHERE action='two_factor.reset'",
                [],
                |r| r.get(0),
            )
            .map_err(Into::into)
        })
        .await
        .unwrap()
}
async fn enabled(a: &TestApp, id: i64) -> bool {
    a.db()
        .read(move |c| User::find(c, id)?.two_factor_enabled(c))
        .await
        .unwrap()
}
#[tokio::test]
async fn reset_requires_admin_authentication_real_csrf_and_active_human_target() {
    let a = app().await;
    let mut anon = a.anonymous();
    assert_eq!(
        anon.write(Req::new(Method::POST, &path(KEVIN)))
            .await
            .location(),
        Some("http://campfire.test/session/new")
    );
    let mut member = a.sign_in(KEVIN).await;
    assert_eq!(
        member
            .write(Req::new(Method::POST, &path(DAVID)))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    let mut admin = a.sign_in(DAVID).await;
    admin.get("/account/edit").await;
    assert_eq!(
        admin
            .send(Req::new(Method::POST, &path(KEVIN)))
            .await
            .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    for id in [BENDER, -1] {
        assert_eq!(
            admin.write(Req::new(Method::POST, &path(id))).await.status,
            StatusCode::NOT_FOUND
        );
    }
    a.db()
        .write(|tx| {
            tx.conn()
                .execute("UPDATE users SET status=1 WHERE id=?", [KEVIN])?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        admin
            .write(Req::new(Method::POST, &path(KEVIN)))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(audit(&a).await, 0);
    assert!(enabled(&a, DAVID).await);
    assert!(enabled(&a, KEVIN).await);
}
#[tokio::test]
async fn self_and_unenrolled_resets_change_nothing_and_have_exact_alerts() {
    let a = app().await;
    let mut admin = a.sign_in(DAVID).await;
    let before = a
        .db()
        .read(|c| Session::count_for_user(c, DAVID))
        .await
        .unwrap();
    assert_eq!(
        admin
            .write(Req::new(Method::POST, &path(DAVID)))
            .await
            .location(),
        Some("http://campfire.test/account/edit")
    );
    assert!(admin.get("/account/edit").await.text().contains("Reset someone else&#39;s two-step sign-in from here. To change your own, use Disable on your profile."));
    assert!(enabled(&a, DAVID).await);
    assert_eq!(
        a.db()
            .read(|c| Session::count_for_user(c, DAVID))
            .await
            .unwrap(),
        before
    );
    a.db()
        .write(|tx| User::find(tx.conn(), KEVIN)?.reset_two_factor(tx))
        .await
        .unwrap();
    assert_eq!(
        admin
            .write(Req::new(Method::POST, &path(KEVIN)))
            .await
            .location(),
        Some("http://campfire.test/account/edit")
    );
    assert!(
        admin
            .get("/account/edit")
            .await
            .text()
            .contains("Kevin doesn&#39;t have two-step sign-in enabled.")
    );
    assert_eq!(audit(&a).await, 0);
}
#[tokio::test]
async fn reset_removes_all_auth_rows_records_actor_target_and_requires_enrollment_next_sign_in() {
    let a = app().await;
    let mut admin = a.sign_in(DAVID).await;
    let id = a
        .db()
        .write(|tx| {
            let c = TwoFactorCredential::for_user(tx.conn(), KEVIN)?.unwrap();
            TwoFactorBackupCode::regenerate_set(tx, c.id)?;
            TwoFactorRememberedDevice::create_for(tx, KEVIN, Some("Browser"), None)?;
            Ok(c.id)
        })
        .await
        .unwrap();
    assert_eq!(
        admin
            .write(Req::new(Method::POST, &path(KEVIN)))
            .await
            .location(),
        Some("http://campfire.test/account/edit")
    );
    assert!(
        admin.get("/account/edit").await.text().contains(
            "Two-step sign-in reset for Kevin. They will set it up again at next sign-in."
        )
    );
    a.db()
        .read(move |c| {
            assert!(TwoFactorCredential::for_user(c, KEVIN)?.is_none());
            assert!(TwoFactorBackupCode::for_credential(c, id)?.is_empty());
            assert!(TwoFactorRememberedDevice::for_user(c, KEVIN)?.is_empty());
            assert!(Session::for_user(c, KEVIN)?.is_empty());
            let pair: (i64, i64) = c.query_row(
                "SELECT actor_id,target_id FROM audit_logs WHERE action='two_factor.reset'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            assert_eq!(pair, (DAVID, KEVIN));
            Ok(())
        })
        .await
        .unwrap();
    let mut member = a.anonymous();
    member.get("/session/new").await;
    assert_eq!(
        member
            .write(Req::new(Method::POST, "/session").form(&[
                ("email_address", "kevin@37signals.com"),
                ("password", "secret123456")
            ]))
            .await
            .location(),
        Some("http://campfire.test/")
    );
    assert_eq!(
        member.get("/").await.location(),
        Some("http://campfire.test/two_factor_setup")
    );
}
#[tokio::test]
async fn failed_audit_rolls_back_reset_credentials_codes_devices_and_sessions() {
    let a = app().await;
    let mut admin = a.sign_in(DAVID).await;
    let before = a
        .db()
        .read(|c| Session::count_for_user(c, KEVIN))
        .await
        .unwrap();
    let before_devices = a
        .db()
        .read(|c| Ok(TwoFactorRememberedDevice::for_user(c, KEVIN)?.len()))
        .await
        .unwrap();
    a.db().write(|tx|{TwoFactorRememberedDevice::create_for(tx,KEVIN,None,None)?;tx.conn().execute_batch("CREATE TRIGGER ws9_reject_reset_audit BEFORE INSERT ON audit_logs WHEN NEW.action='two_factor.reset' BEGIN SELECT RAISE(ABORT,'audit unavailable'); END")?;Ok(())}).await.unwrap();
    assert_eq!(
        admin
            .write(Req::new(Method::POST, &path(KEVIN)))
            .await
            .status,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert!(enabled(&a, KEVIN).await);
    assert_eq!(
        a.db()
            .read(|c| Session::count_for_user(c, KEVIN))
            .await
            .unwrap(),
        before
    );
    assert_eq!(audit(&a).await, 0);
    assert_eq!(
        a.db()
            .read(|c| Ok(TwoFactorRememberedDevice::for_user(c, KEVIN)?.len()))
            .await
            .unwrap(),
        before_devices + 1
    );
}
#[tokio::test]
async fn admin_reset_disconnects_real_sockets_and_reset_still_works_without_a_listener() {
    let a = app().await;
    let listener = crate::channels::tests::support::bind_listener().await;
    let addr = listener.local_addr().unwrap();
    let router = a.booted.router.clone();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let member = a.sign_in(KEVIN).await;
    let mut admin = a.sign_in(DAVID).await;
    let cookie = member.cookie_header();
    let mut client = super::session_management_tests::socket(&a, &cookie, addr).await;
    assert_eq!(client.next_text().await, r#"{"type":"welcome"}"#);
    assert_eq!(
        admin
            .write(Req::new(Method::POST, &path(KEVIN)))
            .await
            .location(),
        Some("http://campfire.test/account/edit")
    );
    assert!(
        client
            .until_closed()
            .await
            .iter()
            .any(|s| s.contains("\"reconnect\":true"))
    );
    server.abort();
    // There is no external realtime RPC in Rust: a stopped listener cannot prevent the reset.
    assert_eq!(
        admin
            .write(Req::new(
                Method::POST,
                &path(crate::controllers::presenters::test_support::JASON)
            ))
            .await
            .location(),
        Some("http://campfire.test/account/edit")
    );
    assert!(!enabled(&a, crate::controllers::presenters::test_support::JASON).await);
}
#[tokio::test]
async fn account_rows_offer_reset_only_for_other_enrolled_humans_to_admins() {
    let a = app().await;
    let mut admin = a.sign_in(DAVID).await;
    let text = admin.get("/account/edit").await.text();
    assert!(text.contains(&format!("action=\"{}\"", path(KEVIN))));
    assert!(!text.contains(&format!("action=\"{}\"", path(DAVID))));
    assert!(!text.contains(&format!("action=\"{}\"", path(BENDER))));
    let mut member = a.sign_in(KEVIN).await;
    assert!(
        !member
            .get("/account/edit")
            .await
            .text()
            .contains("two_factor_reset")
    );
    a.db()
        .write(|tx| User::find(tx.conn(), KEVIN)?.reset_two_factor(tx))
        .await
        .unwrap();
    assert!(
        !admin
            .get("/account/edit")
            .await
            .text()
            .contains(&format!("action=\"{}\"", path(KEVIN)))
    );
}
#[tokio::test]
async fn self_service_limits_use_ip_and_user_windows_and_remembered_actions_share_a_bucket() {
    for (method, endpoint) in [
        (Method::POST, "/two_factor_backup_codes"),
        (Method::DELETE, "/two_factor_setup"),
        (Method::DELETE, "/two_factor_remembered_devices"),
        (Method::POST, "/two_factor_reauthentication"),
    ] {
        let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
            "2026-03-02T16:00:00Z".parse().unwrap(),
        ));
        let a = TestApp::boot_with_clock(clock.clone())
            .await
            .expect("default seed");
        let mut b = a.sign_in(DAVID).await;
        let request = |n: usize, ip: &str| {
            let url = if endpoint == "/two_factor_remembered_devices" && n.is_multiple_of(2) {
                "/two_factor_remembered_devices/999"
            } else {
                endpoint
            };
            Req::new(method.clone(), url)
                .header("x-forwarded-for", ip)
                .form(&[("reauth", if n == 11 { "secret123456" } else { "wrong" })])
        };
        for n in 0..10 {
            assert_eq!(
                b.write(request(n, "198.18.0.1")).await.location(),
                Some("http://campfire.test/users/me/profile")
            );
        }
        b.write(request(11, "198.18.0.1")).await;
        assert!(
            b.get("/users/me/profile")
                .await
                .text()
                .contains("Too many attempts. Try again in a few minutes."),
            "{endpoint} IP limit"
        );
        let mut other = a.sign_in(KEVIN).await;
        other.write(request(11, "198.18.0.1")).await;
        assert!(
            other
                .get("/users/me/profile")
                .await
                .text()
                .contains("Too many attempts. Try again in a few minutes."),
            "{endpoint} IP bucket shared across users"
        );
        clock.advance(jiff::SignedDuration::from_secs(181));
        other.write(request(12, "198.18.0.1")).await;
        assert!(
            !other
                .get("/users/me/profile")
                .await
                .text()
                .contains("Too many attempts. Try again in a few minutes."),
            "{endpoint} IP window resets after three minutes"
        );
        b.write(request(12, "198.18.0.2")).await;
        assert!(
            b.get("/users/me/profile")
                .await
                .text()
                .contains("Too many attempts. Try again in a few minutes."),
            "{endpoint} user limit across IPs"
        );
        clock.advance(jiff::SignedDuration::from_secs(121));
        b.write(request(12, "198.18.0.4")).await;
        assert!(
            b.get("/users/me/profile")
                .await
                .text()
                .contains("Too many attempts. Try again in a few minutes."),
            "{endpoint} user limit survives five minutes"
        );
        clock.advance(jiff::SignedDuration::from_secs(600));
        b.write(request(13, "198.18.0.3")).await;
        assert!(
            !b.get("/users/me/profile")
                .await
                .text()
                .contains("Too many attempts. Try again in a few minutes."),
            "{endpoint} window expires"
        );
    }
}

#[tokio::test]
async fn challenge_ip_bucket_is_shared_and_rejects_valid_codes_until_three_minutes() {
    use rails_compat::{ar_encryption::ArEncryption, totp};
    let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
        "2026-03-02T16:00:00Z".parse().unwrap(),
    ));
    let a = TestApp::boot_with_clock(clock.clone())
        .await
        .expect("default seed");
    let secrets = a.booted.app.secrets.clone();
    let keys = a.db().write(move |tx| {
        tx.conn().execute("UPDATE two_factor_credentials SET last_totp_at=NULL,consecutive_failures=0,lockout_count=0,locked_until=NULL WHERE user_id IN (?,?)",[DAVID,KEVIN])?;
        [DAVID,KEVIN].into_iter().map(|id| TwoFactorCredential::for_user(tx.conn(),id)?.unwrap().secret(&ArEncryption::new(&secrets))).collect::<campfire_db::Result<Vec<_>>>()
    }).await.unwrap();
    let mut david = a.anonymous();
    let mut kevin = a.anonymous();
    for (browser, email) in [
        (&mut david, "david@37signals.com"),
        (&mut kevin, "kevin@37signals.com"),
    ] {
        assert_eq!(browser.get("/session/new").await.status, StatusCode::OK);
        assert_eq!(
            browser
                .write(
                    Req::new(Method::POST, "/session")
                        .form(&[("email_address", email), ("password", "secret123456")])
                )
                .await
                .location(),
            Some("http://campfire.test/two_factor_challenge")
        );
    }
    for _ in 0..10 {
        let reply = david
            .write(
                Req::new(Method::POST, "/two_factor_challenge")
                    .header("x-forwarded-for", "198.18.0.100")
                    .form(&[("code", "wrong")]),
            )
            .await;
        assert!(
            !reply.text().contains("Too many attempts."),
            "ten attempts fit the IP bucket"
        );
    }
    let count = a
        .db()
        .read(|c| Ok(Session::for_user(c, DAVID)?.len() + Session::for_user(c, KEVIN)?.len()))
        .await
        .unwrap();
    for (browser, key) in [(&mut david, &keys[0]), (&mut kevin, &keys[1])] {
        let code = totp::at(key, a.booted.app.clock.now().as_second()).unwrap();
        let reply = browser
            .write(
                Req::new(Method::POST, "/two_factor_challenge")
                    .header("x-forwarded-for", "198.18.0.100")
                    .form(&[("code", &code)]),
            )
            .await;
        assert_eq!(reply.status, StatusCode::TOO_MANY_REQUESTS);
        assert!(
            reply.text().contains("Too many attempts."),
            "valid code and a new user cannot bypass the shared IP bucket"
        );
    }
    assert_eq!(
        a.db()
            .read(|c| Ok(Session::for_user(c, DAVID)?.len() + Session::for_user(c, KEVIN)?.len()))
            .await
            .unwrap(),
        count
    );
    clock.advance(jiff::SignedDuration::from_secs(181));
    let code = totp::at(&keys[1], a.booted.app.clock.now().as_second()).unwrap();
    assert_eq!(
        kevin
            .write(
                Req::new(Method::POST, "/two_factor_challenge")
                    .header("x-forwarded-for", "198.18.0.100")
                    .form(&[("code", &code)])
            )
            .await
            .location(),
        Some("http://campfire.test/")
    );
}
