//! Seeded HTTP tests for two-factor enrollment. Missing seeds fail explicitly.
use crate::controllers::presenters::test_support::{DAVID, Req, TestApp};
use axum::http::{Method, StatusCode};
use campfire_db::{Session, TwoFactorCredential, TwoFactorSetupSecret, User};
use rails_compat::{ar_encryption::ArEncryption, totp};

async fn app() -> TestApp {
    TestApp::boot()
        .await
        .expect("build the d7c7de92 WS19 default parity seed")
}
async fn unenroll(a: &TestApp) {
    a.db()
        .write(|tx| User::find(tx.conn(), DAVID)?.reset_two_factor(tx))
        .await
        .unwrap();
}
async fn setup(a: &TestApp) -> (i64, String) {
    let enc = ArEncryption::new(&a.booted.app.secrets);
    let now = campfire_db::Timestamp::from_jiff(a.booted.app.clock.now());
    a.db()
        .read(move |c| {
            let id = c.query_row(
                "SELECT session_id FROM two_factor_setup_secrets ORDER BY id DESC LIMIT 1",
                [],
                |r| r.get(0),
            )?;
            let s = TwoFactorSetupSecret::valid_for(c, id, now)?.expect("live setup");
            Ok((id, s.secret(&enc)?))
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn enrollment_reuses_live_secret_and_renders_private_qr() {
    let a = app().await;
    unenroll(&a).await;
    let mut b = a.sign_in(DAVID).await;
    let first = b.get("/two_factor_setup").await;
    assert_eq!(first.status, StatusCode::OK);
    assert_eq!(first.header("cache-control"), Some("no-store"));
    let (_, secret) = setup(&a).await;
    assert!(
        first.text().contains(
            &secret
                .as_bytes()
                .chunks(4)
                .map(|v| std::str::from_utf8(v).unwrap())
                .collect::<Vec<_>>()
                .join(" ")
        )
    );
    assert!(first.text().contains("<svg"));
    assert!(
        !first.text().contains("/qr_code/"),
        "the secret must not travel in an image URL"
    );
    assert_eq!(b.get("/two_factor_setup").await.status, StatusCode::OK);
    assert_eq!(setup(&a).await.1, secret);
    let bad = b
        .write(Req::new(Method::POST, "/two_factor_setup").form(&[("code", "invalid")]))
        .await;
    assert_eq!(bad.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(setup(&a).await.1, secret);
    assert!(
        !a.db()
            .read(|c| User::find(c, DAVID)?.two_factor_enabled(c))
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn enrollment_is_bound_to_current_session_and_never_accepts_expired_secret() {
    let a = app().await;
    unenroll(&a).await;
    let mut one = a.sign_in(DAVID).await;
    let mut two = a.sign_in(DAVID).await;
    assert_eq!(one.get("/two_factor_setup").await.status, StatusCode::OK);
    let (_, first) = setup(&a).await;
    assert_eq!(two.get("/two_factor_setup").await.status, StatusCode::OK);
    let (id, second) = setup(&a).await;
    assert_ne!(first, second);
    let code = totp::at(&first, a.booted.app.clock.now().as_second()).unwrap();
    assert_eq!(
        two.write(Req::new(Method::POST, "/two_factor_setup").form(&[("code", &code)]))
            .await
            .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(setup(&a).await.1, second);
    let code = totp::at(&second, a.booted.app.clock.now().as_second()).unwrap();
    a.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE two_factor_setup_secrets SET expires_at=? WHERE session_id=?",
                rusqlite::params![tx.now().ago(jiff::SignedDuration::from_secs(1)), id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        two.write(Req::new(Method::POST, "/two_factor_setup").form(&[("code", &code)]))
            .await
            .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_ne!(setup(&a).await.1, second);
    assert!(
        !a.db()
            .read(|c| User::find(c, DAVID)?.two_factor_enabled(c))
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn enrollment_confirms_spends_step_signs_out_others_and_shows_backup_codes_once() {
    let a = app().await;
    unenroll(&a).await;
    let mut b = a.anonymous();
    assert_eq!(b.get("/session/new").await.status, StatusCode::OK);
    assert_eq!(
        b.write(Req::new(Method::POST, "/session").form(&[
            ("email_address", "david@37signals.com"),
            ("password", "secret123456")
        ]))
        .await
        .location(),
        Some("http://campfire.test/")
    );
    assert_eq!(b.get("/two_factor_setup").await.status, StatusCode::OK);
    let (id, secret) = setup(&a).await;
    assert!(
        !a.db()
            .read(move |c| Ok(Session::find(c, id)?.two_factor_verified()))
            .await
            .unwrap()
    );
    let code = totp::at(&secret, a.booted.app.clock.now().as_second()).unwrap();
    let result = b
        .write(Req::new(Method::POST, "/two_factor_setup").form(&[("code", &code)]))
        .await;
    assert_eq!(result.status, StatusCode::OK);
    assert_eq!(result.header("cache-control"), Some("no-store"));
    assert!(result.text().contains("Signed out your other devices"));
    assert!(result.text().contains("smartfire-backup-codes.txt"));
    let codes = regex::Regex::new(r#"<li><code class="txt-large">([^<]+)</code></li>"#)
        .unwrap()
        .captures_iter(&result.text())
        .map(|c| c[1].to_string())
        .collect::<Vec<_>>();
    assert_eq!(codes.len(), 10);
    let enc = ArEncryption::new(&a.booted.app.secrets);
    a.db().read(move |c| {
        let credential = TwoFactorCredential::for_user(c, DAVID)?.unwrap();
        assert!(credential.enabled());
        assert_eq!(credential.secret(&enc)?, secret);
        assert!(credential.last_totp_at.is_some());
        assert!(Session::find(c, id)?.two_factor_verified());
        assert_eq!(Session::for_user(c, DAVID)?.len(), 1);
        assert_eq!(c.query_row("SELECT count(*) FROM two_factor_setup_secrets WHERE session_id=?", [id], |r| r.get::<_,i64>(0))?, 0);
        assert_eq!(c.query_row("SELECT count(*) FROM audit_logs WHERE action='two_factor.enable' AND target_id=?", [DAVID], |r| r.get::<_,i64>(0))?, 1);
        assert_eq!(c.query_row("SELECT count(*) FROM two_factor_backup_codes WHERE two_factor_credential_id=? AND used_at IS NULL", [credential.id], |r| r.get::<_,i64>(0))?, 10);
        Ok(())
    }).await.unwrap();
    assert_eq!(
        b.get("/two_factor_setup").await.location(),
        Some("http://campfire.test/users/me/profile")
    );
}

#[tokio::test]
async fn enrollment_requires_authentication_and_real_csrf() {
    let a = app().await;
    let mut anon = a.anonymous();
    assert_eq!(
        anon.get("/two_factor_setup").await.location(),
        Some("http://campfire.test/session/new")
    );
    unenroll(&a).await;
    let mut b = a.sign_in(DAVID).await;
    assert_eq!(b.get("/two_factor_setup").await.status, StatusCode::OK);
    let code = totp::at(&setup(&a).await.1, a.booted.app.clock.now().as_second()).unwrap();
    assert_eq!(
        b.send(Req::new(Method::POST, "/two_factor_setup").form(&[("code", &code)]))
            .await
            .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert!(
        !a.db()
            .read(|c| User::find(c, DAVID)?.two_factor_enabled(c))
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn two_factor_views_match_rails_seed_bytes_including_inline_qr() {
    use askama::Template;
    use campfire_views::{helpers as h, two_factor};
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
    let account = a.db().read(campfire_db::Account::first).await.unwrap();
    let goldens: serde_json::Value =
        serde_json::from_str(include_str!("../../../../vectors/two_factor_views.json")).unwrap();
    let qr = crate::controllers::qr_code::two_factor_svg(goldens["uri"].as_str().unwrap()).unwrap();
    same_bytes("qr", &qr, goldens["qr"].as_str().unwrap());
    for name in [
        "setup",
        "challenge",
        "backups",
        "backups_signed_out",
        "profile",
        "profile_devices",
        "profile_disabled",
    ] {
        let actual = h::request_forgery::rendering_with(
            h::request_forgery::RequestSecrets {
                tokens: Box::new(Tokens),
                csp_nonce: Some("NONCE".into()),
            },
            || {
                crate::controllers::presenters::page::render_detached_at(
                    &a.booted.app,
                    account.as_ref(),
                    "http://campfire.test",
                    |ctx| match name {
                        "setup" => two_factor::Setup {
                            ctx,
                            key: goldens["key"].as_str().unwrap().into(),
                            qr: qr.clone(),
                        }
                        .as_content()
                        .render()
                        .unwrap(),
                        "challenge" => two_factor::Challenge { ctx }.as_content().render().unwrap(),
                        "profile" | "profile_devices" | "profile_disabled" => two_factor::Profile {
                            ctx,
                            now: "2026-03-02T16:00:00Z".parse().unwrap(),
                            data: two_factor::ProfileData {
                                confirmed_at: (name != "profile_disabled").then(|| {
                                    goldens["profile_data"]["confirmed_at"]
                                        .as_str()
                                        .unwrap()
                                        .parse()
                                        .unwrap()
                                }),
                                devices: if name == "profile_devices" {
                                    vec![two_factor::Device {
                                        id: 777,
                                        user_agent: Some("TestBrowser <&>".into()),
                                        ip_address: Some("1.2.3.4".into()),
                                        last_used_at: Some(
                                            goldens["profile_data"]["last_used_at"]
                                                .as_str()
                                                .unwrap()
                                                .parse()
                                                .unwrap(),
                                        ),
                                    }]
                                } else {
                                    vec![]
                                },
                                google: false,
                            },
                        }
                        .render()
                        .unwrap(),
                        _ => two_factor::BackupCodes {
                            ctx,
                            codes: goldens["codes"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(|v| v.as_str().unwrap().into())
                                .collect(),
                            signed_out: usize::from(name == "backups_signed_out") * 2,
                            continue_url: if name == "backups" {
                                "http://campfire.test/users/me/profile"
                            } else {
                                "http://campfire.test/rooms/486777696"
                            }
                            .into(),
                        }
                        .as_content()
                        .render()
                        .unwrap(),
                    },
                )
            },
        );
        same_bytes(name, &actual, goldens[name].as_str().unwrap());
    }
}
fn same_bytes(name: &str, actual: &str, expected: &str) {
    if !super::asset_goldens::compare(name, actual, expected) {
        let at = actual
            .bytes()
            .zip(expected.bytes())
            .position(|(a, b)| a != b)
            .unwrap_or(actual.len().min(expected.len()));
        panic!(
            "{name}: mismatch at byte {at}, lengths {} vs {}; actual {:?}, Rails {:?}",
            actual.len(),
            expected.len(),
            &actual[at.saturating_sub(50)..(at + 150).min(actual.len())],
            &expected[at.saturating_sub(50)..(at + 150).min(expected.len())]
        );
    }
}

#[tokio::test]
async fn enrollment_limits_attempts_without_confirming_or_auditing() {
    let a = app().await;
    unenroll(&a).await;
    let mut b = a.sign_in(DAVID).await;
    assert_eq!(b.get("/two_factor_setup").await.status, StatusCode::OK);
    for _ in 0..10 {
        assert_eq!(
            b.write(Req::new(Method::POST, "/two_factor_setup").form(&[("code", "invalid")]))
                .await
                .status,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    let code = totp::at(&setup(&a).await.1, a.booted.app.clock.now().as_second()).unwrap();
    let result = b
        .write(Req::new(Method::POST, "/two_factor_setup").form(&[("code", &code)]))
        .await;
    assert_eq!(result.status, StatusCode::TOO_MANY_REQUESTS);
    assert!(result.text().contains("Too many attempts."));
    assert!(
        !a.db()
            .read(|c| User::find(c, DAVID)?.two_factor_enabled(c))
            .await
            .unwrap()
    );
    assert_eq!(
        a.db()
            .read(|c| Ok(c.query_row(
                "SELECT count(*) FROM audit_logs WHERE action='two_factor.enable'",
                [],
                |r| r.get::<_, i64>(0)
            )?))
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn enrollment_user_limit_follows_the_account_across_ips() {
    let a = app().await;
    unenroll(&a).await;
    let mut b = a.sign_in(DAVID).await;
    assert_eq!(b.get("/two_factor_setup").await.status, StatusCode::OK);
    for n in 1..=10 {
        let req = Req::new(Method::POST, "/two_factor_setup")
            .header("x-forwarded-for", &format!("198.18.0.{n}"))
            .form(&[("code", "invalid")]);
        assert_eq!(b.write(req).await.status, StatusCode::UNPROCESSABLE_ENTITY);
    }
    let code = totp::at(&setup(&a).await.1, a.booted.app.clock.now().as_second()).unwrap();
    assert_eq!(
        b.write(
            Req::new(Method::POST, "/two_factor_setup")
                .header("x-forwarded-for", "198.51.100.7")
                .form(&[("code", &code)])
        )
        .await
        .status,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert!(
        !a.db()
            .read(|c| User::find(c, DAVID)?.two_factor_enabled(c))
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn enrollment_rolls_back_every_write_if_the_audit_cannot_be_saved() {
    let a = app().await;
    unenroll(&a).await;
    let mut b = a.sign_in(DAVID).await;
    assert_eq!(b.get("/two_factor_setup").await.status, StatusCode::OK);
    let (id, secret) = setup(&a).await;
    let before = a
        .db()
        .read(|c| Ok(Session::for_user(c, DAVID)?.len()))
        .await
        .unwrap();
    a.db().write(|tx| { tx.conn().execute_batch("CREATE TRIGGER refuse_enable_audit BEFORE INSERT ON audit_logs WHEN NEW.action='two_factor.enable' BEGIN SELECT RAISE(ABORT, 'audit unavailable'); END")?; Ok(()) }).await.unwrap();
    let code = totp::at(&secret, a.booted.app.clock.now().as_second()).unwrap();
    assert_eq!(
        b.write(Req::new(Method::POST, "/two_factor_setup").form(&[("code", &code)]))
            .await
            .status,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(setup(&a).await.1, secret);
    a.db()
        .read(move |c| {
            assert!(!User::find(c, DAVID)?.two_factor_enabled(c)?);
            assert_eq!(Session::for_user(c, DAVID)?.len(), before);
            assert!(Session::find(c, id)?.two_factor_verified()); // test helper began verified
            assert_eq!(
                c.query_row("SELECT count(*) FROM two_factor_backup_codes", [], |r| r
                    .get::<_, i64>(0))?,
                0
            );
            Ok(())
        })
        .await
        .unwrap();
}
