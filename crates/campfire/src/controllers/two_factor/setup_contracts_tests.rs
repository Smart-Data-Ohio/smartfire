use crate::controllers::presenters::test_support::{Browser, DAVID, Req, TestApp};
use axum::http::{Method, StatusCode};
use campfire_db::{Session, TwoFactorBackupCode, TwoFactorCredential, TwoFactorSetupSecret, User};
use campfire_kit::{Crypto, RailsCrypto};
use rails_compat::{ar_encryption::ArEncryption, totp};
use serde_json::{Value, json};

const API: &str = "/api/v1/two_factor/setup";
const WRONG: &str = "That code didn't work. Check your authenticator app and try again.";

async fn app() -> TestApp {
    let app = crate::test_support::with_auth_inputs(
        campfire_db::FixtureAuthInputs {
            totp_secret: "JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP".into(),
            backup_codes: (0..10).map(|n| format!("abcd-{n:04}-efgh")).collect(),
        },
        TestApp::boot_frozen(),
    )
    .await
    .unwrap()
    .without_job_runner()
    .await;
    app.db()
        .write(|tx| User::find(tx.conn(), DAVID)?.reset_two_factor(tx))
        .await
        .unwrap();
    app
}

fn request(method: Method, path: &str, body: Value) -> Req {
    Req::new(method, path)
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&body).unwrap())
}

fn submission(json_mode: bool, code: &str) -> Req {
    if json_mode {
        request(Method::POST, API, json!({"code":code}))
    } else {
        Req::new(Method::POST, "/two_factor_setup").form(&[("code", code)])
    }
}

fn path(json_mode: bool) -> &'static str {
    if json_mode { API } else { "/two_factor_setup" }
}

fn cookie_state(app: &TestApp, browser: &Browser<'_>) -> Value {
    let cookies = browser.cookie_header();
    let raw = cookies
        .split("; ")
        .find_map(|v| v.strip_prefix("_campfire_session="))
        .unwrap();
    let mut state = RailsCrypto::new(app.booted.app.secrets.clone())
        .decrypt_cookie(
            "_campfire_session",
            &rails_compat::cookies::unescape(raw),
            app.booted.app.clock.now(),
        )
        .unwrap();
    for key in ["_csrf_token", "session_id", "flash"] {
        state.as_object_mut().unwrap().remove(key);
    }
    state
}

async fn setup(app: &TestApp) -> (i64, String) {
    let encryption = ArEncryption::new(&app.booted.app.secrets);
    let now = campfire_db::Timestamp::from_jiff(app.booted.app.clock.now());
    app.db()
        .read(move |conn| {
            let id = conn.query_row(
                "SELECT session_id FROM two_factor_setup_secrets ORDER BY id DESC LIMIT 1",
                [],
                |r| r.get(0),
            )?;
            Ok((
                id,
                TwoFactorSetupSecret::valid_for(conn, id, now)?
                    .unwrap()
                    .secret(&encryption)?,
            ))
        })
        .await
        .unwrap()
}

async fn facts(app: &TestApp) -> Value {
    app.db().read(|conn| {
        let queries = [
            ("credentials", "SELECT id,user_id,confirmed_at,last_totp_at,consecutive_failures,lockout_count,locked_until,created_at,updated_at FROM two_factor_credentials ORDER BY id"),
            ("setups", "SELECT id,session_id,expires_at,created_at,updated_at FROM two_factor_setup_secrets ORDER BY id"),
            ("backups", "SELECT * FROM two_factor_backup_codes ORDER BY id"),
            ("devices", "SELECT * FROM two_factor_remembered_devices ORDER BY id"),
            ("sessions", "SELECT id,user_id,user_agent,ip_address,created_at,updated_at,last_active_at,device_id,two_factor_verified_at FROM sessions ORDER BY id"),
            ("audits", "SELECT * FROM audit_logs ORDER BY id"),
        ];
        let mut result = serde_json::Map::new();
        for (name, sql) in queries {
            let mut stmt = conn.prepare(sql)?;
            let count = stmt.column_count();
            let rows = stmt.query_map([], |row| {
                (0..count).map(|i| {
                    Ok(match row.get_ref(i)? {
                        rusqlite::types::ValueRef::Null => Value::Null,
                        rusqlite::types::ValueRef::Integer(n) => json!(n),
                        rusqlite::types::ValueRef::Text(v) => json!(std::str::from_utf8(v).unwrap()),
                        value => panic!("unexpected database value: {value:?}"),
                    })
                }).collect::<rusqlite::Result<Vec<_>>>().map(Value::from)
            })?.collect::<rusqlite::Result<Vec<_>>>()?;
            result.insert(name.into(), json!(rows));
        }
        Ok(Value::Object(result))
    }).await.unwrap()
}

#[tokio::test]
async fn setup_contract_each_step_matches_html_rows_sessions_audits_and_private_qr() {
    let recovery_codes = regex::Regex::new(r#"<li><code>([^<]+)</code></li>"#).unwrap();
    let mut outcomes = Vec::new();
    for json_mode in [false, true] {
        let app = app().await;
        let mut browser = app.anonymous();
        let device = RailsCrypto::new(app.booted.app.secrets.clone()).sign_cookie(
            "device_id",
            "setup-contract-device",
            None,
        );
        browser.absorb_cookie_header(&format!(
            "device_id={}",
            campfire_kit::cookies::escape(&device)
        ));
        browser.get("/session/new").await;
        browser
            .write(Req::new(Method::POST, "/session").form(&[
                ("email_address", "david@37signals.com"),
                ("password", "secret123456"),
            ]))
            .await;
        let mut steps = Vec::new();
        let first = browser.get(path(json_mode)).await;
        assert_eq!(first.status, StatusCode::OK, "{}", first.text());
        assert_eq!(first.header("cache-control"), Some("no-store"));
        assert_eq!(first.header("pragma"), Some("no-cache"));
        let (id, secret) = setup(&app).await;
        let uri = totp::provisioning_uri(&secret, "david@37signals.com");
        let qr = crate::controllers::qr_code::two_factor_svg(&uri).unwrap();
        let key = secret
            .as_bytes()
            .chunks(4)
            .map(|c| std::str::from_utf8(c).unwrap())
            .collect::<Vec<_>>()
            .join(" ");
        if json_mode {
            assert_eq!(
                first.json(),
                json!({"kind":"ready","setup":{"secret":secret,"manualKey":key,"otpauthUri":uri,"qrSvg":qr}})
            );
        } else {
            assert!(first.text().contains(&qr));
            assert!(first.text().contains(&key));
        }
        assert!(
            cookie_state(&app, &browser)
                .get("sudo_verified_at")
                .is_none()
        );
        steps.push((facts(&app).await, cookie_state(&app, &browser)));
        let again = browser.get(path(json_mode)).await;
        assert_eq!(again.status, StatusCode::OK);
        assert_eq!(setup(&app).await.1, secret);
        steps.push((facts(&app).await, cookie_state(&app, &browser)));
        let wrong = browser.write(submission(json_mode, "wrong")).await;
        assert_eq!(wrong.status, StatusCode::UNPROCESSABLE_ENTITY);
        if json_mode {
            assert_eq!(wrong.json()["message"], WRONG);
        } else {
            crate::form_contracts::assert_text(&wrong.text(), WRONG);
        }
        if json_mode {
            assert_eq!(wrong.json()["setup"], first.json()["setup"]);
        }
        assert_eq!(setup(&app).await.1, secret);
        steps.push((facts(&app).await, cookie_state(&app, &browser)));
        let code = totp::at(&secret, app.booted.app.clock.now().as_second()).unwrap();
        let confirmed = browser.write(submission(json_mode, &code)).await;
        assert_eq!(confirmed.status, StatusCode::OK, "{}", confirmed.text());
        assert_eq!(confirmed.header("cache-control"), Some("no-store"));
        let codes: Vec<String> = if json_mode {
            assert_eq!(confirmed.json()["kind"], "recoveryCodes");
            assert_eq!(confirmed.json()["continueUrl"], "http://campfire.test/app/");
            assert_eq!(confirmed.json()["signedOut"], 2);
            serde_json::from_value(confirmed.json()["codes"].clone()).unwrap()
        } else {
            assert!(confirmed.text().contains("http://campfire.test/app/"));
            recovery_codes
                .captures_iter(&confirmed.text())
                .map(|c| c[1].into())
                .collect()
        };
        assert_eq!(
            codes,
            (0..10)
                .map(|n| format!("abcd-{n:04}-efgh"))
                .collect::<Vec<_>>()
        );
        let encryption = ArEncryption::new(&app.booted.app.secrets);
        app.db()
            .read(move |conn| {
                let factor = TwoFactorCredential::for_user(conn, DAVID)?.unwrap();
                assert_eq!(factor.secret(&encryption)?, secret);
                assert!(factor.enabled());
                assert!(factor.last_totp_at.is_some());
                assert!(Session::find(conn, id)?.two_factor_verified());
                assert_eq!(Session::for_user(conn, DAVID)?.len(), 1);
                assert_eq!(
                    conn.query_row(
                        "SELECT count(*) FROM two_factor_setup_secrets WHERE session_id=?",
                        [id],
                        |r| r.get::<_, i64>(0)
                    )?,
                    0
                );
                let backups = TwoFactorBackupCode::for_credential(conn, factor.id)?;
                assert_eq!(backups.len(), codes.len());
                for (stored, code) in backups.iter().zip(codes) {
                    assert_eq!(stored.code_digest, TwoFactorBackupCode::digest(&code));
                }
                Ok(())
            })
            .await
            .unwrap();
        steps.push((facts(&app).await, cookie_state(&app, &browser)));
        for reply in [
            browser.get(path(json_mode)).await,
            browser.write(submission(json_mode, &code)).await,
        ] {
            if json_mode {
                assert_eq!(
                    reply.json(),
                    json!({"kind":"navigate","location":"http://campfire.test/users/me/profile"})
                );
            } else {
                assert_eq!(
                    reply.location(),
                    Some("http://campfire.test/users/me/profile")
                );
            }
            assert!(!reply.text().contains("abcd-0000-efgh"));
        }
        steps.push((facts(&app).await, cookie_state(&app, &browser)));
        outcomes.push(steps);
    }
    for (step, (html, json)) in outcomes[0].iter().zip(&outcomes[1]).enumerate() {
        for key in [
            "credentials",
            "setups",
            "backups",
            "devices",
            "sessions",
            "audits",
        ] {
            assert_eq!(html.0[key], json.0[key], "step {step}: {key}");
        }
        assert_eq!(html.1, json.1, "step {step}: cookie session");
    }
}

#[tokio::test]
async fn setup_contract_expired_and_abandoned_setups_match_html() {
    for abandoned in [false, true] {
        let mut outcomes = Vec::new();
        for json_mode in [false, true] {
            let app = app().await;
            let mut browser = app.sign_in(DAVID).await;
            browser.get(path(json_mode)).await;
            let (id, secret) = setup(&app).await;
            app.db()
                .write(move |tx| {
                    if abandoned {
                        tx.conn().execute(
                            "DELETE FROM two_factor_setup_secrets WHERE session_id=?",
                            [id],
                        )?;
                    } else {
                        tx.conn().execute(
                            "UPDATE two_factor_setup_secrets SET expires_at=? WHERE session_id=?",
                            rusqlite::params![tx.now().ago(jiff::SignedDuration::from_secs(1)), id],
                        )?;
                    }
                    Ok(())
                })
                .await
                .unwrap();
            let code = totp::at(&secret, app.booted.app.clock.now().as_second()).unwrap();
            let reply = browser.write(submission(json_mode, &code)).await;
            assert_eq!(
                reply.status,
                StatusCode::UNPROCESSABLE_ENTITY,
                "{}",
                reply.text()
            );
            if json_mode {
                assert_eq!(reply.json()["message"], WRONG);
            } else {
                crate::form_contracts::assert_text(&reply.text(), WRONG);
            }
            assert_eq!(facts(&app).await["backups"], json!([]));
            outcomes.push((facts(&app).await, cookie_state(&app, &browser)));
        }
        assert_eq!(outcomes[0], outcomes[1]);
    }
}

#[tokio::test]
async fn setup_contract_attempt_limits_are_shared_and_do_not_enable_or_audit() {
    let app = app().await;
    let mut browser = app.sign_in(DAVID).await;
    browser.get(API).await;
    for attempt in 0..10 {
        let reply = browser
            .write(
                submission(attempt % 2 == 0, "wrong")
                    .header("x-forwarded-for", &format!("198.18.0.{}", attempt + 1)),
            )
            .await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            reply.text()
        );
    }
    let before = facts(&app).await;
    for json_mode in [false, true] {
        let reply = browser
            .write(submission(json_mode, "123456").header("x-forwarded-for", "198.51.100.1"))
            .await;
        assert_eq!(reply.status, StatusCode::TOO_MANY_REQUESTS);
        assert!(
            reply
                .text()
                .contains("Too many attempts. Try again in a few minutes.")
        );
        assert_eq!(facts(&app).await, before);
    }
}

#[tokio::test]
async fn setup_contract_enrollment_gate_is_typed_for_every_json_entry_and_retained_redirects() {
    let app = app().await;
    let mut browser = app.anonymous();
    browser.get("/session/new").await;
    browser
        .write(Req::new(Method::POST, "/session").form(&[
            ("email_address", "david@37signals.com"),
            ("password", "secret123456"),
        ]))
        .await;
    let before = facts(&app).await;
    for req in [
        Req::new(Method::GET, "/api/v1/settings"),
        Req::new(Method::GET, "/rooms.json"),
        Req::new(Method::GET, "/app/").header("accept", "application/json"),
        Req::new(Method::GET, "/api/v1/sudo"),
    ] {
        let reply = browser.send(req).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.text());
        assert_eq!(
            reply.json(),
            json!({"error":{"_tag":"TwoFactorRequired","message":"Set up two-step sign-in to continue","requirement":{"kind":"setup","location":"http://campfire.test/two_factor_setup"}}})
        );
        assert_eq!(reply.header("cache-control"), Some("no-store"));
        assert_eq!(facts(&app).await, before);
    }
    let reply = browser.get("/app/settings?tab=security").await;
    assert_eq!(
        reply.location(),
        Some("http://campfire.test/two_factor_setup")
    );
    assert_eq!(
        cookie_state(&app, &browser)["return_to_after_authenticating"],
        "http://campfire.test/app/settings?tab=security"
    );
    browser.get(API).await;
    let code = totp::at(&setup(&app).await.1, app.booted.app.clock.now().as_second()).unwrap();
    let reply = browser.write(submission(true, &code)).await;
    assert_eq!(
        reply.json()["continueUrl"],
        "http://campfire.test/app/settings?tab=security"
    );
    assert_eq!(browser.get("/api/v1/settings").await.status, StatusCode::OK);
}

#[tokio::test]
async fn setup_contract_authentication_csrf_and_bad_bodies_do_not_confirm() {
    let app = app().await;
    let mut anonymous = app.anonymous();
    assert_eq!(
        anonymous.get(API).await.json()["location"],
        "http://campfire.test/session/new"
    );
    let mut browser = app.sign_in(DAVID).await;
    browser.get(API).await;
    let before = facts(&app).await;
    let code = totp::at(&setup(&app).await.1, app.booted.app.clock.now().as_second()).unwrap();
    for req in [
        submission(true, &code),
        submission(false, &code),
        submission(true, &code).header("origin", "https://attacker.test"),
    ] {
        assert_eq!(
            browser.send(req).await.status,
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(facts(&app).await, before);
    }
    for body in [json!({}), json!({"code":123456}), json!({"code":null})] {
        let reply = browser.write(request(Method::POST, API, body)).await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(reply.json()["message"], "The request body isn't valid.");
        assert_eq!(facts(&app).await, before);
    }
}

#[tokio::test]
async fn setup_contract_audit_failure_preserves_the_same_completed_enrollment() {
    let mut outcomes = Vec::new();
    for json_mode in [false, true] {
        let app = app().await;
        let mut browser = app.sign_in(DAVID).await;
        browser.get(path(json_mode)).await;
        let (id, secret) = setup(&app).await;
        app.db().write(|tx| {
            tx.conn().execute_batch("CREATE TRIGGER refuse_enable_audit BEFORE INSERT ON audit_logs WHEN NEW.action='two_factor.enable' BEGIN SELECT RAISE(ABORT, 'audit unavailable'); END")?;
            Ok(())
        }).await.unwrap();
        let code = totp::at(&secret, app.booted.app.clock.now().as_second()).unwrap();
        let reply = browser.write(submission(json_mode, &code)).await;
        assert_eq!(
            reply.status,
            StatusCode::INTERNAL_SERVER_ERROR,
            "{}",
            reply.text()
        );
        app.db()
            .read(move |conn| {
                assert!(
                    TwoFactorCredential::for_user(conn, DAVID)?
                        .unwrap()
                        .enabled()
                );
                assert!(Session::find(conn, id)?.two_factor_verified());
                assert_eq!(Session::for_user(conn, DAVID)?.len(), 1);
                Ok(())
            })
            .await
            .unwrap();
        assert_eq!(facts(&app).await["backups"].as_array().unwrap().len(), 10);
        outcomes.push(facts(&app).await);
    }
    assert_eq!(outcomes[0], outcomes[1]);
}

#[tokio::test]
async fn setup_contract_another_session_cannot_confirm_the_displayed_secret() {
    let mut outcomes = Vec::new();
    for json_mode in [false, true] {
        let app = app().await;
        let mut one = app.sign_in(DAVID).await;
        let mut two = app.sign_in(DAVID).await;
        one.get(path(json_mode)).await;
        let (_, first) = setup(&app).await;
        two.get(path(json_mode)).await;
        let (id, _) = setup(&app).await;
        let encryption = ArEncryption::new(&app.booted.app.secrets);
        app.db()
            .write(move |tx| {
                let secret = encryption
                    .encrypt_with_encoding(b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", "ASCII-8BIT");
                tx.conn().execute(
                    "UPDATE two_factor_setup_secrets SET secret=? WHERE session_id=?",
                    rusqlite::params![secret, id],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        two.get(path(json_mode)).await;
        let (_, second) = setup(&app).await;
        assert_ne!(first, second);
        let code = totp::at(&first, app.booted.app.clock.now().as_second()).unwrap();
        let reply = two.write(submission(json_mode, &code)).await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            reply.text()
        );
        assert_eq!(setup(&app).await.1, second);
        assert_eq!(facts(&app).await["backups"], json!([]));
        outcomes.push(facts(&app).await);
    }
    assert_eq!(outcomes[0], outcomes[1]);
}

#[tokio::test]
async fn setup_contract_retained_paths_negotiate_the_same_json_and_share_body_csrf() {
    let app = app().await;
    let mut browser = app.sign_in(DAVID).await;
    let api = browser.get(API).await;
    for req in [
        Req::new(Method::GET, "/two_factor_setup.json"),
        Req::new(Method::GET, "/two_factor_setup").header("accept", "application/json"),
    ] {
        let reply = browser.send(req).await;
        assert_eq!(reply.status, StatusCode::OK);
        assert_eq!(reply.json(), api.json());
    }
    let token = browser.authenticity_token().await;
    let code = totp::at(&setup(&app).await.1, app.booted.app.clock.now().as_second()).unwrap();
    let req = request(
        Method::POST,
        API,
        json!({"code":code,"authenticity_token":token}),
    );
    for origin in ["https://attacker.test", "null"] {
        assert_eq!(
            browser
                .send(req.clone().header("origin", origin))
                .await
                .status,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    assert_eq!(browser.send(req).await.json()["kind"], "recoveryCodes");
}
