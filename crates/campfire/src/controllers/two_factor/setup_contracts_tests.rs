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

/// A code submitted as the SPA's setup screen does: JSON to the API, or with a JSON body to the
/// retained route, which still enrolls a JSON request.
fn submission(path: &str, code: &str) -> Req {
    request(Method::POST, path, json!({"code":code}))
}

fn show() -> Req {
    Req::new(Method::GET, API).header("accept", "application/json")
}

/// The retained setup page as a browser still reaches it: a page load goes to the SPA's setup
/// screen (or wherever its before-actions send it), keeping the query, and a form post goes,
/// unread, to the security settings. Neither changes a row or the cookie session.
async fn assert_retained_page_redirects(
    app: &TestApp,
    browser: &mut Browser<'_>,
    code: &str,
    page: &str,
) {
    // The form needs the session's token, which also gives the cookie session something to compare.
    browser.authenticity_token().await;
    let before = (facts(app).await, cookie_state(app, browser));
    let shown = browser.get("/two_factor_setup?from=bookmark").await;
    assert_eq!(shown.status, StatusCode::FOUND);
    assert_eq!(shown.location(), Some(page));
    let posted = browser
        .write(Req::new(Method::POST, "/two_factor_setup").form(&[("code", code)]))
        .await;
    assert_eq!(posted.status, StatusCode::SEE_OTHER);
    assert_eq!(
        posted.location(),
        Some("http://campfire.test/app/settings/security")
    );
    assert!(!posted.text().contains("abcd-0000-efgh"));
    assert_eq!((facts(app).await, cookie_state(app, browser)), before);
}

const SCREEN: &str = "http://campfire.test/app/two_factor/setup?from=bookmark";
const PROFILE: &str = "http://campfire.test/users/me/profile";

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

async fn enabled(app: &TestApp) -> bool {
    app.db()
        .read(|conn| User::find(conn, DAVID)?.two_factor_enabled(conn))
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
    assert_eq!(
        browser
            .password_sign_in("david@37signals.com", "secret123456")
            .await
            .json()["kind"],
        "signedIn"
    );
    let first = browser.send(show()).await;
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
    assert_eq!(
        first.json(),
        json!({"kind":"ready","setup":{"secret":secret,"manualKey":key,"otpauthUri":uri,"qrSvg":qr}})
    );
    assert!(
        cookie_state(&app, &browser)
            .get("sudo_verified_at")
            .is_none()
    );
    let code = totp::at(&secret, app.booted.app.clock.now().as_second()).unwrap();
    // The retained page neither shows nor confirms the live secret, even with the right code.
    assert_retained_page_redirects(&app, &mut browser, &code, SCREEN).await;
    let again = browser.send(show()).await;
    assert_eq!(again.status, StatusCode::OK);
    assert_eq!(again.json(), first.json());
    assert_eq!(setup(&app).await.1, secret);
    let wrong = browser.write(submission(API, "wrong")).await;
    assert_eq!(wrong.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(wrong.json()["message"], WRONG);
    assert_eq!(wrong.json()["setup"], first.json()["setup"]);
    assert_eq!(setup(&app).await.1, secret);
    assert!(!enabled(&app).await);
    assert_retained_page_redirects(&app, &mut browser, &code, SCREEN).await;
    let confirmed = browser.write(submission(API, &code)).await;
    assert_eq!(confirmed.status, StatusCode::OK, "{}", confirmed.text());
    assert_eq!(confirmed.header("cache-control"), Some("no-store"));
    assert_eq!(confirmed.json()["kind"], "recoveryCodes");
    assert_eq!(confirmed.json()["continueUrl"], "http://campfire.test/app/");
    assert_eq!(confirmed.json()["signedOut"], 2);
    let codes: Vec<String> = serde_json::from_value(confirmed.json()["codes"].clone()).unwrap();
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
    let audits = facts(&app).await["audits"].clone();
    let enabled = audits
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| {
            row.as_array()
                .unwrap()
                .contains(&json!("two_factor.enable"))
        })
        .count();
    assert_eq!(enabled, 1, "{audits}");
    let after = (facts(&app).await, cookie_state(&app, &browser));
    // Enrolled, the setup state and a replayed code navigate to the profile, never showing the
    // codes again; the retained page sends its load to the profile too.
    for reply in [
        browser.send(show()).await,
        browser.write(submission(API, &code)).await,
    ] {
        assert_eq!(reply.json(), json!({"kind":"navigate","location":PROFILE}));
        assert!(!reply.text().contains("abcd-0000-efgh"));
    }
    assert_eq!((facts(&app).await, cookie_state(&app, &browser)), after);
    assert_retained_page_redirects(&app, &mut browser, &code, PROFILE).await;
}

#[tokio::test]
async fn setup_contract_expired_and_abandoned_setups_match_html() {
    for abandoned in [false, true] {
        let app = app().await;
        let mut browser = app.sign_in(DAVID).await;
        browser.send(show()).await;
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
        // The retained page's form neither confirms nor replaces the old setup.
        assert_retained_page_redirects(&app, &mut browser, &code, SCREEN).await;
        let reply = browser.write(submission(API, &code)).await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            reply.text()
        );
        assert_eq!(reply.json()["message"], WRONG);
        // The screen gets a live setup in place of the stale one (the fixture's secret is fixed).
        let (_, live) = setup(&app).await;
        assert_eq!(reply.json()["setup"]["secret"], live);
        assert!(!enabled(&app).await);
        assert_eq!(facts(&app).await["backups"], json!([]));
    }
}

#[tokio::test]
async fn setup_contract_attempt_limits_are_shared_and_do_not_enable_or_audit() {
    // The API and the retained route's JSON form count against the same limits.
    let paths = [API, "/two_factor_setup"];
    let app = app().await;
    let mut browser = app.sign_in(DAVID).await;
    browser.send(show()).await;
    for attempt in 0..10 {
        let reply = browser
            .write(
                submission(paths[attempt % 2], "wrong")
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
    let code = totp::at(&setup(&app).await.1, app.booted.app.clock.now().as_second()).unwrap();
    for path in paths {
        let reply = browser
            .write(submission(path, &code).header("x-forwarded-for", "198.51.100.1"))
            .await;
        assert_eq!(reply.status, StatusCode::TOO_MANY_REQUESTS);
        assert!(
            reply
                .text()
                .contains("Too many attempts. Try again in a few minutes.")
        );
        assert_eq!(facts(&app).await, before);
    }
    assert_retained_page_redirects(&app, &mut browser, &code, SCREEN).await;
}

#[tokio::test]
async fn setup_contract_enrollment_gate_is_typed_for_every_json_entry_and_retained_redirects() {
    let app = app().await;
    let mut browser = app.anonymous();
    assert_eq!(
        browser
            .password_sign_in("david@37signals.com", "secret123456")
            .await
            .json()["kind"],
        "signedIn"
    );
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
            json!({"error":{"_tag":"TwoFactorRequired","message":"Set up two-step sign-in to continue","requirement":{"kind":"setup","location":"http://campfire.test/app/two_factor/setup"}}})
        );
        assert_eq!(reply.header("cache-control"), Some("no-store"));
        assert_eq!(facts(&app).await, before);
    }
    let reply = browser.get("/app/settings?tab=security").await;
    assert_eq!(
        reply.location(),
        Some("http://campfire.test/app/two_factor/setup?required=true")
    );
    assert_eq!(
        cookie_state(&app, &browser)["return_to_after_authenticating"],
        "http://campfire.test/app/settings?tab=security"
    );
    browser.send(show()).await;
    let code = totp::at(&setup(&app).await.1, app.booted.app.clock.now().as_second()).unwrap();
    let reply = browser.write(submission(API, &code)).await;
    assert_eq!(
        reply.json()["continueUrl"],
        "http://campfire.test/app/settings?tab=security"
    );
    assert_eq!(browser.get("/api/v1/settings").await.status, StatusCode::OK);
}

#[tokio::test]
async fn setup_shell_opens_for_an_unenrolled_session_and_keeps_its_return_path() {
    const SHELL: &str = "/app/two_factor/setup";
    let gated = json!({"error":{"_tag":"TwoFactorRequired","message":"Set up two-step sign-in to continue","requirement":{"kind":"setup","location":"http://campfire.test/app/two_factor/setup"}}});
    let app = app().await;
    // Signed out, the screen asks for sign-in as the retained page does.
    let retained = app.anonymous().get("/two_factor_setup").await;
    let shell = app.anonymous().get(SHELL).await;
    assert_eq!(retained.status, StatusCode::FOUND);
    assert_eq!(shell.status, retained.status);
    assert_eq!(shell.location(), retained.location());

    let mut browser = app.anonymous();
    assert_eq!(
        browser
            .password_sign_in("david@37signals.com", "secret123456")
            .await
            .json()["kind"],
        "signedIn"
    );
    // Only the screen's page is let through: the rest of /app, a nested path and the JSON API stay
    // gated.
    for path in [
        "/app/",
        "/app/two_factor/setup/extra",
        "/app/settings/security",
    ] {
        assert_eq!(
            browser.get(path).await.location(),
            Some("http://campfire.test/app/two_factor/setup?required=true"),
            "{path}"
        );
    }
    let json = browser
        .send(Req::new(Method::GET, "/api/v1/settings"))
        .await;
    assert_eq!(json.status, StatusCode::FORBIDDEN);
    assert_eq!(json.json(), gated);
    // The gate lets the screen through only as a page, so the gate's redirect can't loop; a JSON
    // request for it gets the gate's typed refusal, uncached, and issues nothing.
    let before = facts(&app).await;
    let json = browser
        .send(Req::new(Method::GET, SHELL).header("accept", "application/json"))
        .await;
    assert_eq!(json.status, StatusCode::FORBIDDEN);
    assert_eq!(json.json(), gated);
    assert_eq!(json.header("cache-control"), Some("no-store"));
    assert_eq!(facts(&app).await, before);

    assert_eq!(
        browser.get("/app/settings?tab=security").await.location(),
        Some("http://campfire.test/app/two_factor/setup?required=true")
    );
    let saved = cookie_state(&app, &browser)["return_to_after_authenticating"].clone();
    assert_eq!(saved, "http://campfire.test/app/settings?tab=security");
    let before = facts(&app).await;

    // A full load of the SPA's setup screen is the shell, not the gate's redirect.
    let page = browser
        .get("/app/two_factor/setup?return_to=%2Fapp%2Fsettings%3Ftab%3Dsecurity&required=true")
        .await;
    assert_eq!(page.status, StatusCode::OK, "{}", page.text());
    assert_eq!(page.location(), None);
    assert!(
        page.text()
            .contains(r#"<script type="application/json" id="boot""#)
    );
    assert_eq!(page.header("cache-control"), Some("no-store"));
    assert_eq!(
        cookie_state(&app, &browser)["return_to_after_authenticating"],
        saved
    );
    assert_eq!(facts(&app).await, before);

    // Setting up from the screen returns to the saved path.
    browser.send(show()).await;
    let code = totp::at(&setup(&app).await.1, app.booted.app.clock.now().as_second()).unwrap();
    let reply = browser.write(submission(API, &code)).await;
    assert_eq!(reply.json()["continueUrl"], saved);

    // Enrolled, the screen sends the account to its profile, as the retained page does.
    let retained = browser.get("/two_factor_setup").await;
    let shell = browser.get(SHELL).await;
    assert_eq!(
        retained.location(),
        Some("http://campfire.test/users/me/profile")
    );
    assert_eq!(shell.status, retained.status);
    assert_eq!(shell.location(), retained.location());
}

#[tokio::test]
async fn setup_contract_authentication_csrf_and_bad_bodies_do_not_confirm() {
    let app = app().await;
    let mut anonymous = app.anonymous();
    assert_eq!(
        anonymous.get(API).await.json()["location"],
        "http://campfire.test/app/session/new"
    );
    let mut browser = app.sign_in(DAVID).await;
    browser.send(show()).await;
    let before = facts(&app).await;
    let code = totp::at(&setup(&app).await.1, app.booted.app.clock.now().as_second()).unwrap();
    for req in [
        submission(API, &code),
        submission("/two_factor_setup", &code),
        submission(API, &code).header("origin", "https://attacker.test"),
    ] {
        assert_eq!(
            browser.send(req).await.status,
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(facts(&app).await, before);
    }
    // A form post to the retained page without a token is sent on before CSRF is checked, and
    // confirms nothing either.
    let posted = browser
        .send(Req::new(Method::POST, "/two_factor_setup").form(&[("code", &code)]))
        .await;
    assert_eq!(posted.status, StatusCode::SEE_OTHER);
    assert_eq!(
        posted.location(),
        Some("http://campfire.test/app/settings/security")
    );
    assert_eq!(facts(&app).await, before);
    for body in [json!({}), json!({"code":123456}), json!({"code":null})] {
        let reply = browser.write(request(Method::POST, API, body)).await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(reply.json()["message"], "The request body isn't valid.");
        assert_eq!(facts(&app).await, before);
    }
}

#[tokio::test]
async fn setup_contract_audit_failure_preserves_the_same_completed_enrollment() {
    let app = app().await;
    let mut browser = app.sign_in(DAVID).await;
    browser.send(show()).await;
    let (id, secret) = setup(&app).await;
    app.db().write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER refuse_enable_audit BEFORE INSERT ON audit_logs WHEN NEW.action='two_factor.enable' BEGIN SELECT RAISE(ABORT, 'audit unavailable'); END")?;
        Ok(())
    }).await.unwrap();
    let code = totp::at(&secret, app.booted.app.clock.now().as_second()).unwrap();
    let audits = facts(&app).await["audits"].clone();
    let reply = browser.write(submission(API, &code)).await;
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
            assert_eq!(
                conn.query_row(
                    "SELECT count(*) FROM two_factor_setup_secrets WHERE session_id=?",
                    [id],
                    |r| r.get::<_, i64>(0)
                )?,
                0
            );
            Ok(())
        })
        .await
        .unwrap();
    let after = facts(&app).await;
    assert_eq!(after["backups"].as_array().unwrap().len(), 10);
    assert_eq!(after["audits"], audits);
}

#[tokio::test]
async fn setup_contract_another_session_cannot_confirm_the_displayed_secret() {
    let app = app().await;
    let mut one = app.sign_in(DAVID).await;
    let mut two = app.sign_in(DAVID).await;
    one.send(show()).await;
    let (_, first) = setup(&app).await;
    two.send(show()).await;
    let (id, _) = setup(&app).await;
    let encryption = ArEncryption::new(&app.booted.app.secrets);
    app.db()
        .write(move |tx| {
            let secret =
                encryption.encrypt_with_encoding(b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", "ASCII-8BIT");
            tx.conn().execute(
                "UPDATE two_factor_setup_secrets SET secret=? WHERE session_id=?",
                rusqlite::params![secret, id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    two.send(show()).await;
    let (_, second) = setup(&app).await;
    assert_ne!(first, second);
    let code = totp::at(&first, app.booted.app.clock.now().as_second()).unwrap();
    let reply = two.write(submission(API, &code)).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.text()
    );
    assert_eq!(reply.json()["message"], WRONG);
    assert_eq!(setup(&app).await.1, second);
    assert!(!enabled(&app).await);
    assert_eq!(facts(&app).await["backups"], json!([]));
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
