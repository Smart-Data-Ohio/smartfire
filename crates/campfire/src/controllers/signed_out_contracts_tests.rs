//! Retained forms and signed-out JSON contracts over the same frozen SQLite seed.
use crate::controllers::presenters::test_support::{Browser, DAVID, Reply, Req, TestApp};
use axum::http::{Method, StatusCode};
use campfire_db::{Session, TwoFactorBackupCode, TwoFactorCredential, User};
use campfire_kit::Crypto;
use rails_compat::{ar_encryption::ArEncryption, totp};
use serde_json::{Value, json};

async fn app() -> TestApp {
    let app = TestApp::boot_frozen().await.expect("restored default seed");
    app.db()
        .write(|tx| {
            tx.conn().execute("DELETE FROM audit_logs", [])?;
            Ok(())
        })
        .await
        .unwrap();
    app
}

pub(crate) fn json_request(method: Method, path: &str, body: Value) -> Req {
    Req::new(method, path)
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&body).unwrap())
}

pub(crate) fn inline_boot(reply: &Reply) -> Value {
    let re =
        regex::Regex::new(r#"(?s)<script type="application/json" id="boot"[^>]*>(.*?)</script>"#)
            .unwrap();
    serde_json::from_str(&re.captures(&reply.text()).expect("boot script")[1]).unwrap()
}

async fn password(browser: &mut Browser<'_>, email: &str, password: &str) -> Reply {
    browser.password_sign_in(email, password).await
}

fn destination(reply: &Reply, json: bool) -> String {
    if json {
        reply.json()["location"]
            .as_str()
            .expect("next location")
            .into()
    } else {
        reply.location().expect("HTML redirect").into()
    }
}

pub(crate) fn cookie_contract(reply: &Reply) -> Vec<String> {
    let mut cookies = reply
        .headers
        .get_all("set-cookie")
        .iter()
        .map(|value| {
            let value = value.to_str().unwrap();
            let (pair, attributes) = value.split_once(';').unwrap();
            let (name, token) = pair.split_once('=').unwrap();
            format!(
                "{name}={} ;{attributes}",
                if token.is_empty() { "deleted" } else { "set" }
            )
        })
        .collect::<Vec<_>>();
    cookies.sort();
    cookies
}

async fn facts(app: &TestApp) -> Value {
    app.db().read(|conn| {
        let mut stmt = conn.prepare("SELECT user_id,user_agent,ip_address,two_factor_verified_at FROM sessions ORDER BY id")?;
        let sessions = stmt.query_map([], |r| Ok(json!([r.get::<_, i64>(0)?,r.get::<_, Option<String>>(1)?,r.get::<_, Option<String>>(2)?,r.get::<_, Option<String>>(3)?])))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let mut stmt = conn.prepare("SELECT action,actor_id,actor_label,target_id,target_type,target_label,details,ip_address,user_agent FROM audit_logs ORDER BY id")?;
        let audits = stmt.query_map([], |r| Ok(json!([r.get::<_, String>(0)?,r.get::<_, Option<i64>>(1)?,r.get::<_, Option<String>>(2)?,r.get::<_, Option<i64>>(3)?,r.get::<_, Option<String>>(4)?,r.get::<_, Option<String>>(5)?,r.get::<_, Option<String>>(6)?,r.get::<_, Option<String>>(7)?,r.get::<_, Option<String>>(8)?])))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let mut stmt = conn.prepare("SELECT user_id,user_agent,ip_address,expires_at,last_used_at FROM two_factor_remembered_devices ORDER BY id")?;
        let devices = stmt.query_map([], |r| Ok(json!([r.get::<_, i64>(0)?,r.get::<_, Option<String>>(1)?,r.get::<_, Option<String>>(2)?,r.get::<_, String>(3)?,r.get::<_, Option<String>>(4)?])))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let mut stmt = conn.prepare("SELECT user_id,last_totp_at,consecutive_failures,lockout_count,locked_until FROM two_factor_credentials ORDER BY id")?;
        let credentials = stmt.query_map([], |r| Ok(json!([r.get::<_, i64>(0)?,r.get::<_, Option<i64>>(1)?,r.get::<_, i64>(2)?,r.get::<_, i64>(3)?,r.get::<_, Option<String>>(4)?])))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let mut stmt = conn.prepare("SELECT used_at FROM two_factor_backup_codes ORDER BY id")?;
        let backup_codes = stmt.query_map([], |r| r.get::<_, Option<String>>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(json!({"sessions":sessions,"audits":audits,"devices":devices,"credentials":credentials,"backupCodes":backup_codes}))
    }).await.unwrap()
}

/// A retained page or form URL (a bookmark, an old tab, a form loaded before the cutover) sends
/// the browser, unread, to the SPA page at `location`: a `GET` gets a 302 and any other method a
/// 303, before CSRF and rate limits, with nothing signed in, spent or recorded.
async fn assert_retained_redirect(
    app: &TestApp,
    browser: &mut Browser<'_>,
    request: Req,
    location: &str,
) {
    let get = request.method == Method::GET;
    let path = request.path.clone();
    let before = facts(app).await;
    let reply = browser.send(request).await;
    assert_eq!(
        reply.status,
        if get {
            StatusCode::FOUND
        } else {
            StatusCode::SEE_OTHER
        },
        "{path}: {}",
        reply.text()
    );
    assert_eq!(
        reply.location(),
        Some(format!("http://campfire.test{location}").as_str()),
        "{path}"
    );
    assert!(
        !reply.headers.get_all("set-cookie").iter().any(|cookie| {
            let cookie = cookie.to_str().unwrap();
            cookie.starts_with("session_token=") || cookie.starts_with("two_factor_remember=")
        }),
        "{path}"
    );
    assert_eq!(facts(app).await, before, "{path}");
}

async fn reset_factor(app: &TestApp, enabled: bool) -> (String, Vec<String>) {
    let enc = ArEncryption::new(&app.booted.app.secrets);
    app.db().write(move |tx| {
        if !enabled {
            User::find(tx.conn(), DAVID)?.reset_two_factor(tx)?;
            return Ok((String::new(), vec![]));
        }
        let credential = TwoFactorCredential::for_user(tx.conn(), DAVID)?.unwrap();
        tx.conn().execute("UPDATE two_factor_credentials SET last_totp_at=NULL,consecutive_failures=0,lockout_count=0,locked_until=NULL WHERE id=?", [credential.id])?;
        Ok((credential.secret(&enc)?, TwoFactorBackupCode::regenerate_set(tx, credential.id)?))
    }).await.unwrap()
}

pub(crate) fn browser_session(app: &TestApp, browser: &Browser<'_>) -> Value {
    let cookies = browser.cookie_header();
    let raw = cookies
        .split("; ")
        .find_map(|pair| pair.strip_prefix("_campfire_session="))
        .unwrap();
    campfire_kit::RailsCrypto::new(app.booted.app.secrets.clone())
        .decrypt_cookie(
            "_campfire_session",
            &rails_compat::cookies::unescape(raw),
            app.booted.app.clock.now(),
        )
        .unwrap()
}

pub(crate) fn body_token(mut request: Req, json: bool, token: &str) -> Req {
    if json {
        let mut body: Value = serde_json::from_slice(&request.body).unwrap();
        body["authenticity_token"] = json!(token);
        request.body = serde_json::to_vec(&body).unwrap();
    } else {
        if !request.body.is_empty() {
            request.body.push(b'&');
        }
        request.body.extend_from_slice(
            format!(
                "authenticity_token={}",
                crate::controllers::presenters::test_support::encode(token)
            )
            .as_bytes(),
        );
    }
    request
}

#[tokio::test]
async fn unenrolled_logout_and_challenge_match_retained_exemptions() {
    let mut outcomes = Vec::new();
    for json in [false, true] {
        let app = app().await;
        reset_factor(&app, false).await;
        let mut browser = app.anonymous();
        let sign_in = browser
            .password_sign_in("david@37signals.com", "secret123456")
            .await;
        assert_eq!(destination(&sign_in, true), "http://campfire.test/app/");
        let before = facts(&app).await;
        if json {
            let reply = browser
                .send(
                    Req::new(Method::GET, "/api/v1/two_factor/challenge")
                        .header("accept", "application/json"),
                )
                .await;
            assert_eq!(reply.status, StatusCode::OK);
            assert_eq!(destination(&reply, json), "http://campfire.test/");
            let reply = challenge(&mut browser, "123456", false).await;
            assert_eq!(destination(&reply, json), "http://campfire.test/");
        } else {
            // The retained challenge page and form go to the SPA's, which sends this session home.
            assert_retained_redirect(
                &app,
                &mut browser,
                Req::new(Method::GET, "/two_factor_challenge"),
                "/app/two_factor/challenge",
            )
            .await;
            assert_retained_redirect(
                &app,
                &mut browser,
                Req::new(Method::POST, "/two_factor_challenge")
                    .form(&[("code", "123456"), ("remember_device", "0")]),
                "/app/two_factor/challenge",
            )
            .await;
        }
        assert_eq!(
            browser.get("/app/two_factor/challenge").await.location(),
            Some("http://campfire.test/")
        );
        assert_eq!(facts(&app).await, before);
        let request = if json {
            json_request(Method::DELETE, "/api/v1/session", json!({}))
        } else {
            Req::new(Method::DELETE, "/session")
        };
        let reply = browser.write(request).await;
        assert_eq!(destination(&reply, json), "http://campfire.test/");
        assert!(
            !browser
                .cookie_header()
                .split("; ")
                .any(|pair| pair.starts_with("session_token="))
        );
        let after = facts(&app).await;
        assert_eq!(
            before["sessions"].as_array().unwrap().len(),
            after["sessions"].as_array().unwrap().len() + 1
        );
        outcomes.push((cookie_contract(&reply), after));
    }
    assert_eq!(outcomes[0], outcomes[1]);
}

#[tokio::test]
async fn unenrolled_public_shells_match_retained_forms_and_preserve_return_path() {
    for transfer in [false, true] {
        let mut outcomes = Vec::new();
        for spa in [false, true] {
            let app = app().await;
            reset_factor(&app, false).await;
            let mut browser = app.anonymous();
            browser
                .password_sign_in("david@37signals.com", "secret123456")
                .await;
            assert_eq!(
                browser
                    .get("/app/settings/security?return=1")
                    .await
                    .location(),
                Some("http://campfire.test/app/two_factor/setup?required=true")
            );
            let before = facts(&app).await;
            let session = browser_session(&app, &browser);
            let return_path = session["return_to_after_authenticating"].clone();
            assert_eq!(
                return_path,
                "http://campfire.test/app/settings/security?return=1"
            );
            let token = crate::controllers::presenters::accounts::transfer_id(
                &app.booted.app.secrets,
                DAVID,
                app.booted.app.clock.now(),
            );
            let retained = if transfer {
                format!("/session/transfers/{token}")
            } else {
                "/session/new".into()
            };
            let path = format!("/app{retained}");
            if !spa {
                // The retained page sends this session to the SPA's copy of it.
                assert_retained_redirect(
                    &app,
                    &mut browser,
                    Req::new(Method::GET, &retained),
                    &path,
                )
                .await;
            }
            let reply = browser.get(&path).await;
            assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.text());
            assert_eq!(reply.location(), None);
            let boot = inline_boot(&reply);
            assert_eq!(boot["kind"], "signedOut");
            // The help contact and the version joined the five original keys with slice 45,
            // and a redirect's flash with the retained pages' cutover.
            assert_eq!(boot.as_object().unwrap().len(), 8);
            assert_eq!(
                browser_session(&app, &browser)["return_to_after_authenticating"],
                return_path
            );
            assert_eq!(facts(&app).await, before);
            if !spa {
                // The retained transfer form spends nothing; the SPA's page signs in with the link.
                assert_retained_redirect(
                    &app,
                    &mut browser,
                    Req::new(Method::PUT, &format!("/session/transfers/{token}")),
                    &format!("/app/session/transfers/{token}"),
                )
                .await;
            }
            let reply = browser
                .write(json_request(
                    Method::PUT,
                    &format!("/api/v1/session/transfers/{token}"),
                    json!({}),
                ))
                .await;
            assert_eq!(
                destination(&reply, true),
                "http://campfire.test/app/settings/security?return=1"
            );
            outcomes.push(facts(&app).await);
        }
        assert_eq!(outcomes[0], outcomes[1]);
    }
}

/// The policy pages, and their retained URLs that now send it there, answer an unenrolled session
/// without enforcing enrollment or touching its saved return destination.
#[tokio::test]
async fn unenrolled_sessions_read_public_pages_without_enrollment_or_a_new_return_path() {
    let app = app().await;
    reset_factor(&app, false).await;
    let mut browser = app.anonymous();
    browser
        .password_sign_in("david@37signals.com", "secret123456")
        .await;
    assert_eq!(
        browser.get("/app/settings/security?return=1").await.location(),
        Some("http://campfire.test/app/two_factor/setup?required=true")
    );
    let before = facts(&app).await;
    let return_path = browser_session(&app, &browser)["return_to_after_authenticating"].clone();
    assert_eq!(return_path, "http://campfire.test/app/settings/security?return=1");
    for page in ["about", "privacy", "terms"] {
        // The retained page has moved for good to the SPA's, for this session as for anyone.
        let retained = browser.get(&format!("/{page}")).await;
        assert_eq!(retained.status, StatusCode::MOVED_PERMANENTLY, "/{page}");
        assert_eq!(
            retained.location(),
            Some(format!("http://campfire.test/app/{page}").as_str())
        );
        assert_eq!(
            retained.location(),
            app.anonymous().get(&format!("/{page}")).await.location()
        );

        let shell = browser.get(&format!("/app/{page}")).await;
        assert_eq!(shell.status, StatusCode::OK, "/app/{page}: {}", shell.text());
        assert_eq!(shell.location(), None);
        // The page a visitor gets, as the retained page shows this session what it shows anyone.
        assert_eq!(inline_boot(&shell)["kind"], "signedOut", "/app/{page}");

        let contract = browser
            .send(Req::new(Method::GET, &format!("/api/v1/public_pages/{page}")).header("accept", "application/json"))
            .await;
        assert_eq!(contract.status, StatusCode::OK, "{page}");
        assert_eq!(contract.json()["page"], page);
        assert_eq!(
            contract.body,
            app.anonymous()
                .send(Req::new(Method::GET, &format!("/api/v1/public_pages/{page}")).header("accept", "application/json"))
                .await
                .body
        );

        assert_eq!(
            browser_session(&app, &browser)["return_to_after_authenticating"],
            return_path,
            "{page}"
        );
        assert_eq!(facts(&app).await, before, "{page}");
    }
}

/// The JSON writes check a body token as the retained forms did. Of those forms only sign-out
/// still acts, and its checks match; the retained sign-in and challenge forms go to the SPA's
/// pages unread, whatever token they carry.
#[tokio::test]
async fn body_authenticity_tokens_match_retained_validation() {
    for (valid_body, valid_header) in [(true, None), (true, Some(false)), (false, Some(true))] {
        let mut outcomes = Vec::new();
        for json in [false, true] {
            let app = app().await;
            let (secret, _) = reset_factor(&app, true).await;
            let mut browser = app.anonymous();
            let boot = browser.get("/api/v1/session/boot").await.json();
            let token = boot["csrfToken"].as_str().unwrap();
            let code = totp::at(&secret, app.booted.app.clock.now().as_second()).unwrap();
            let requests = [
                (
                    true,
                    json_request(
                        Method::POST,
                        "/api/v1/session",
                        json!({"emailAddress":"david@37signals.com","password":"secret123456"}),
                    ),
                    Req::new(Method::POST, "/session").form(&[
                        ("email_address", "david@37signals.com"),
                        ("password", "secret123456"),
                    ]),
                    "/app/session/new",
                ),
                (
                    true,
                    json_request(
                        Method::POST,
                        "/api/v1/two_factor/challenge",
                        json!({"code":code,"rememberDevice":false}),
                    ),
                    Req::new(Method::POST, "/two_factor_challenge")
                        .form(&[("code", &code), ("remember_device", "0")]),
                    "/app/two_factor/challenge",
                ),
                (
                    json,
                    json_request(Method::DELETE, "/api/v1/session", json!({})),
                    Req::new(Method::DELETE, "/session").form(&[]),
                    "",
                ),
            ];
            let mut cookies = Vec::new();
            for (step, (step_json, api, retained, spa)) in
                requests.into_iter().enumerate()
            {
                if !json && !spa.is_empty() {
                    for forged in [
                        body_token(retained.clone(), false, "invalid"),
                        body_token(retained.clone(), false, token)
                            .header("origin", "https://attacker.test"),
                        body_token(retained.clone(), false, token),
                    ] {
                        assert_retained_redirect(&app, &mut browser, forged, spa).await;
                    }
                }
                let request = if step_json { api } else { retained };
                let before = facts(&app).await;
                for rejected in [
                    body_token(request.clone(), step_json, "invalid"),
                    body_token(request.clone(), step_json, token)
                        .header("origin", "https://attacker.test"),
                    body_token(request.clone(), step_json, token).header("origin", "null"),
                ] {
                    let reply = browser.send(rejected).await;
                    assert_eq!(
                        reply.status,
                        StatusCode::UNPROCESSABLE_ENTITY,
                        "step {step}"
                    );
                    assert_eq!(facts(&app).await, before);
                }
                let mut request =
                    body_token(request, step_json, if valid_body { token } else { "invalid" });
                if let Some(valid) = valid_header {
                    request = request.header("x-csrf-token", if valid { token } else { "invalid" });
                }
                let reply = browser.send(request).await;
                assert_eq!(
                    reply.status,
                    if step_json {
                        StatusCode::OK
                    } else {
                        StatusCode::FOUND
                    },
                    "step {step}: {}",
                    reply.text()
                );
                if step == 0 {
                    assert_eq!(reply.json()["kind"], "secondFactorRequired");
                } else {
                    assert_eq!(
                        destination(&reply, step_json),
                        if step == 1 {
                            "http://campfire.test/app/"
                        } else {
                            "http://campfire.test/"
                        }
                    );
                }
                cookies.push(cookie_contract(&reply));
            }
            outcomes.push((cookies, facts(&app).await));
        }
        assert_eq!(outcomes[0], outcomes[1]);
    }
}

#[tokio::test]
async fn signed_out_boot_is_allow_listed_and_contains_only_public_auth_inputs() {
    let app = app().await;
    let mut browser = app.anonymous();
    for path in [
        "/app/session/new",
        "/app/session/transfers/example",
        "/app/two_factor/challenge",
    ] {
        let reply = browser.get(path).await;
        assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.text());
        assert_eq!(reply.header("cache-control"), Some("no-store"));
        let boot = inline_boot(&reply);
        assert_eq!(boot["kind"], "signedOut");
        assert_eq!(
            boot["signInMethods"],
            json!({"password":true,"google":false,"googleDomains":[]})
        );
        assert_eq!(boot["firstRunPending"], false);
        // The retained sign-in page's help line: the first administrator and the version.
        assert_eq!(
            boot["helpContact"].as_object().unwrap().keys().collect::<Vec<_>>(),
            ["name", "emailAddress"]
        );
        assert!(boot["version"].is_string());
        assert_eq!(boot["workspace"]["description"], "");
        assert!(boot["workspace"]["name"].is_string());
        assert!(boot["workspace"].get("logoUrl").is_some());
        assert!(browser.real_authenticity_token().unwrap().is_valid(
            boot["csrfToken"].as_str().unwrap(),
            "/api/v1/session",
            "POST"
        ));
        let mut keys = boot
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>();
        keys.sort();
        assert_eq!(
            keys,
            [
                "csrfToken",
                "firstRunPending",
                "flash",
                "helpContact",
                "kind",
                "signInMethods",
                "version",
                "workspace"
            ]
        );
        let mut workspace_keys = boot["workspace"].as_object().unwrap().keys().map(String::as_str).collect::<Vec<_>>();
        workspace_keys.sort();
        assert_eq!(workspace_keys, ["description", "inviteSplashUrl", "logoUrl", "name"]);
    }
    for path in [
        "/app/",
        "/app/rooms/1",
        "/app/session/new/extra",
        "/app/session/transfers/",
        "/app/session/transfers/a/b",
        "/app/two_factor/challenge/extra",
    ] {
        assert_eq!(
            browser.get(path).await.location(),
            Some("http://campfire.test/app/session/new"),
            "{path}"
        );
    }
    // The retained sign-in page is this one now, with the query it was given.
    assert_retained_redirect(
        &app,
        &mut browser,
        Req::new(Method::GET, "/session/new?email_address=david%4037signals.com"),
        "/app/session/new?email_address=david%4037signals.com",
    )
    .await;
}

#[tokio::test]
async fn signed_out_boot_carries_a_redirects_flash_for_one_load() {
    use crate::integrations::google::sign_in::{Config, SignIn};
    let app = app().await;
    app.booted.app.google.install(SignIn::new(
        Config {
            client_id: "fixture-client".into(),
            client_secret: "fixture-secret".into(),
            domains: vec!["smartdata.net".into()],
        },
        crate::net::Network::system(),
    ));
    let alert = json!({
        "kind": "alert",
        "message": "Google sign-in expired. Try again or sign in with email and password."
    });
    let mut browser = app.anonymous();
    // A callback with no flow in progress alerts and sends the visitor to sign in.
    let callback = "/session/google/callback?state=stale&code=fixture";
    let reply = browser.get(callback).await;
    assert_eq!(
        reply.location(),
        Some("http://campfire.test/app/session/new"),
        "{}",
        reply.text()
    );
    // The public pages show no flash, so they leave it for the page that does.
    for path in ["/app/privacy", "/api/v1/public_pages/privacy", "/app/about", "/app/terms"] {
        let reply = browser.get(path).await;
        assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.text());
        if path.starts_with("/app/") {
            assert_eq!(inline_boot(&reply)["flash"], Value::Null, "{path}");
        }
    }
    let page = browser.get("/app/session/new").await;
    assert_eq!(page.status, StatusCode::OK, "{}", page.text());
    assert_eq!(inline_boot(&page)["flash"], alert);
    let boot = browser.get("/api/v1/session/boot").await;
    assert_eq!(boot.json()["flash"], Value::Null);
    assert_eq!(inline_boot(&browser.get("/app/session/new").await)["flash"], Value::Null);
    // The JSON boot reads it the same way, once.
    browser.get(callback).await;
    let boot = browser.get("/api/v1/session/boot").await;
    assert_eq!(boot.json()["flash"], alert);
    assert_eq!(inline_boot(&browser.get("/app/session/new").await)["flash"], Value::Null);
    assert_eq!(browser.get("/api/v1/session/boot").await.json()["flash"], Value::Null);
}

#[tokio::test]
async fn password_success_creates_the_same_session_cookies_and_audit() {
    let app = app().await;
    reset_factor(&app, false).await;
    let mut browser = app.anonymous();
    browser.get("/app/session/new").await;
    // The retained form signs nobody in: it sends the browser to the SPA's sign-in page.
    assert_retained_redirect(
        &app,
        &mut browser,
        Req::new(Method::POST, "/session")
            .form(&[("email_address", "david@37signals.com"), ("password", "secret123456")]),
        "/app/session/new",
    )
    .await;
    let before = facts(&app).await;
    let reply = password(&mut browser, "david@37signals.com", "secret123456").await;
    assert_eq!(destination(&reply, true), "http://campfire.test/app/");
    assert_eq!(reply.json()["kind"], "signedIn");
    let cookie = browser.cookie_header();
    let token = cookie
        .split("; ")
        .find_map(|p| p.strip_prefix("session_token="))
        .unwrap();
    let crypto = campfire_kit::RailsCrypto::new(app.booted.app.secrets.clone());
    let device = cookie
        .split("; ")
        .find_map(|p| p.strip_prefix("device_id="))
        .unwrap();
    let device = crypto
        .verify_signed_cookie(
            "device_id",
            &rails_compat::cookies::unescape(device),
            app.booted.app.clock.now(),
        )
        .unwrap();
    let token = crypto
        .verify_signed_cookie(
            "session_token",
            &rails_compat::cookies::unescape(token),
            app.booted.app.clock.now(),
        )
        .unwrap();
    app.db()
        .read(move |conn| {
            let session = Session::find_by_token(conn, &token)?
                .expect("cookie resolves to the issued session");
            assert_eq!(session.user_id, DAVID);
            assert_eq!(session.device_id.as_deref(), Some(device.as_str()));
            Ok(())
        })
        .await
        .unwrap();
    let contract = cookie_contract(&reply);
    for name in ["device_id=set", "session_token=set"] {
        assert!(contract.iter().any(|c| c.starts_with(name)), "{contract:?}");
    }
    let after = facts(&app).await;
    assert_eq!(
        after["sessions"].as_array().unwrap().len(),
        before["sessions"].as_array().unwrap().len() + 1
    );
    assert_eq!(after["sessions"].as_array().unwrap().last().unwrap()[0], DAVID);
    assert_eq!(before["audits"], json!([]));
    assert_eq!(
        after["audits"],
        json!([["session.sign_in.success",DAVID,"David <david@37signals.com>",null,null,null,"{\"method\":\"password\"}","",null]])
    );
}

#[tokio::test]
async fn password_wrong_unknown_and_rate_limited_match_retained_errors_and_audits() {
    for (email, password, attempts, status) in [
        ("david@37signals.com", "wrong", 1, StatusCode::UNAUTHORIZED),
        ("unknown@example.com", "wrong", 1, StatusCode::UNAUTHORIZED),
        (
            "unknown@example.com",
            "wrong",
            11,
            StatusCode::TOO_MANY_REQUESTS,
        ),
    ] {
        let app = app().await;
        let mut browser = app.anonymous();
        browser.get("/app/session/new").await;
        // The retained form is neither checked, audited nor counted against the limit.
        for _ in 0..attempts {
            assert_retained_redirect(
                &app,
                &mut browser,
                Req::new(Method::POST, "/session")
                    .form(&[("email_address", email), ("password", password)]),
                "/app/session/new",
            )
            .await;
        }
        let mut reply = self::password(&mut browser, email, password).await;
        for _ in 1..attempts {
            reply = self::password(&mut browser, email, password).await;
        }
        assert_eq!(reply.status, status);
        assert_eq!(
            reply.json(),
            json!({"kind":"error","fieldErrors":{"base":["Too many requests or unauthorized."]}})
        );
        assert!(!browser.cookie_header().contains("session_token="));
        let audits = facts(&app).await["audits"].clone();
        assert_eq!(audits.as_array().unwrap().len(), attempts, "{audits}");
        assert!(
            audits
                .as_array()
                .unwrap()
                .iter()
                .all(|audit| audit[0] == "session.sign_in.failure"),
            "{audits}"
        );
    }
}

async fn challenge(browser: &mut Browser<'_>, code: &str, remember: bool) -> Reply {
    let req = json_request(
        Method::POST,
        "/api/v1/two_factor/challenge",
        json!({"code":code,"rememberDevice":remember}),
    );
    let req = if remember {
        req.header("x-forwarded-proto", "https")
    } else {
        req
    };
    browser.write(req).await
}

#[tokio::test]
async fn challenge_totp_recovery_and_remembered_device_have_identical_effects() {
    let mut outcomes = Vec::new();
    for recovery in [false, true] {
        let app = app().await;
        let (secret, codes) = reset_factor(&app, true).await;
        let mut browser = app.anonymous();
        browser.get("/app/settings/security?tab=two_factor").await;
        browser.get("/app/session/new").await;
        let first = password(&mut browser, "david@37signals.com", "secret123456").await;
        assert!(!browser.cookie_header().contains("session_token="));
        assert_eq!(
            first.json(),
            json!({"kind":"secondFactorRequired","challenge":{"methods":["totp","recoveryCode"],"rememberDevice":true}})
        );
        assert_eq!(
            browser
                .send(
                    Req::new(Method::GET, "/api/v1/two_factor/challenge")
                        .header("accept", "application/json")
                )
                .await
                .json(),
            first.json()
        );
        let wrong = challenge(&mut browser, "invalid", false).await;
        assert_eq!(wrong.status, StatusCode::UNPROCESSABLE_ENTITY);
        let message =
            "That code didn't work. Check your authenticator app or try a backup code.";
        assert_eq!(wrong.json()["fieldErrors"]["code"], json!([message]));
        let code = if recovery {
            codes[0].clone()
        } else {
            totp::at(&secret, app.booted.app.clock.now().as_second()).unwrap()
        };
        // The retained page and form leave the pending sign-in as it was and spend no code.
        assert_retained_redirect(
            &app,
            &mut browser,
            Req::new(Method::GET, "/two_factor_challenge"),
            "/app/two_factor/challenge",
        )
        .await;
        assert_retained_redirect(
            &app,
            &mut browser,
            Req::new(Method::POST, "/two_factor_challenge")
                .form(&[("code", &code), ("remember_device", "1")]),
            "/app/two_factor/challenge",
        )
        .await;
        let reply = challenge(&mut browser, &code, true).await;
        assert_eq!(
            destination(&reply, true),
            "http://campfire.test/app/settings/security?tab=two_factor"
        );
        assert!(browser.cookie_header().contains("two_factor_remember="));
        let used = app
            .db()
            .read(|conn| {
                let credential = TwoFactorCredential::for_user(conn, DAVID)?.unwrap();
                Ok(TwoFactorBackupCode::for_credential(conn, credential.id)?
                    .iter()
                    .filter(|c| c.used_at.is_some())
                    .count())
            })
            .await
            .unwrap();
        assert_eq!(used, usize::from(recovery));
        let facts = facts(&app).await;
        outcomes.push((cookie_contract(&reply), facts["sessions"].clone(), facts["devices"].clone()));
        // The remembered device must skip the next challenge and verify the new session.
        browser.write(Req::new(Method::DELETE, "/session")).await;
        let reply = password(&mut browser, "david@37signals.com", "secret123456").await;
        assert_eq!(destination(&reply, true), "http://campfire.test/app/");
        let last = app
            .db()
            .read(|conn| {
                Ok(Session::for_user(conn, DAVID)?
                    .into_iter()
                    .max_by_key(|s| s.id)
                    .unwrap()
                    .two_factor_verified())
            })
            .await
            .unwrap();
        assert!(last);
    }
    // A code from the app and a backup code start the same session and remember the same device.
    assert_eq!(outcomes[0], outcomes[1]);
}

#[tokio::test]
async fn challenge_recovery_replay_lockout_and_rate_limit_match_html() {
    let app = app().await;
    let (_, codes) = reset_factor(&app, true).await;
    let mut browser = app.anonymous();
    password(&mut browser, "david@37signals.com", "secret123456").await;
    let accepted = challenge(&mut browser, &codes[0], false).await;
    assert_eq!(destination(&accepted, true), "http://campfire.test/app/");
    browser.write(Req::new(Method::DELETE, "/session")).await;
    password(&mut browser, "david@37signals.com", "secret123456").await;
    for attempt in 1..=10 {
        // A replay through the retained form counts no attempt and locks nothing.
        assert_retained_redirect(
            &app,
            &mut browser,
            Req::new(Method::POST, "/two_factor_challenge")
                .form(&[("code", &codes[0]), ("remember_device", "0")]),
            "/app/two_factor/challenge",
        )
        .await;
        let reply = challenge(&mut browser, &codes[0], false).await;
        assert_eq!(
            reply.status,
            if attempt < 5 {
                StatusCode::UNPROCESSABLE_ENTITY
            } else {
                StatusCode::TOO_MANY_REQUESTS
            }
        );
        if attempt == 5 {
            let message = "Too many wrong codes. Try again in 1 minute.";
            assert_eq!(reply.json()["fieldErrors"]["code"], json!([message]));
        }
        if attempt == 10 {
            let message = "Too many attempts. Try again in a few minutes.";
            assert_eq!(reply.json()["fieldErrors"]["code"], json!([message]));
        }
    }
    let credentials = facts(&app).await["credentials"].clone();
    let credential = credentials
        .as_array()
        .unwrap()
        .iter()
        .find(|credential| credential[0] == DAVID)
        .unwrap();
    assert_eq!(credential[3], 1, "one lockout: {credential}");
}

#[tokio::test]
async fn challenge_absent_expired_and_deactivated_pending_state_navigates_back_to_sign_in() {
    for expired in [false, true] {
        let app = app().await;
        let mut browser = app.anonymous();
        if expired {
            reset_factor(&app, true).await;
            password(&mut browser, "david@37signals.com", "secret123456").await;
            replace_session_value(
                &app,
                &mut browser,
                "two_factor_pending_expires_at",
                json!(0),
            );
        }
        // The retained form goes to the SPA's challenge page, which asks the same question.
        assert_retained_redirect(
            &app,
            &mut browser,
            Req::new(Method::POST, "/two_factor_challenge")
                .form(&[("code", "123456"), ("remember_device", "0")]),
            "/app/two_factor/challenge",
        )
        .await;
        let before = facts(&app).await;
        let reply = challenge(&mut browser, "123456", false).await;
        assert_eq!(
            destination(&reply, true),
            "http://campfire.test/app/session/new"
        );
        assert_eq!(facts(&app).await, before);
    }
    let app = app().await;
    let mut browser = app.anonymous();
    reset_factor(&app, true).await;
    password(&mut browser, "david@37signals.com", "secret123456").await;
    app.db()
        .write(|tx| {
            tx.conn()
                .execute("UPDATE users SET status=1 WHERE id=?", [DAVID])?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        destination(&challenge(&mut browser, "123456", false).await, true),
        "http://campfire.test/app/session/new"
    );
}

fn replace_session_value(app: &TestApp, browser: &mut Browser<'_>, key: &str, value: Value) {
    let cookie = browser.cookie_header();
    let raw = cookie
        .split("; ")
        .find_map(|p| p.strip_prefix("_campfire_session="))
        .unwrap();
    let crypto = campfire_kit::RailsCrypto::new(app.booted.app.secrets.clone());
    let mut session = crypto
        .decrypt_cookie(
            "_campfire_session",
            &rails_compat::cookies::unescape(raw),
            app.booted.app.clock.now(),
        )
        .unwrap();
    session[key] = value;
    let raw = crypto.encrypt_cookie("_campfire_session", &session, None);
    browser.absorb_cookie_header(&format!(
        "_campfire_session={}",
        campfire_kit::cookies::escape(&raw)
    ));
}

#[tokio::test]
async fn password_returns_to_saved_spa_path_and_ignores_off_site_request_parameter() {
    for (start, expected) in [
        (
            "/app/settings/profile?tab=contact",
            "http://campfire.test/app/settings/profile?tab=contact",
        ),
        (
            "/session/new?return_to=https%3A%2F%2Fevil.example%2Fsteal",
            "http://campfire.test/app/",
        ),
        (
            "/session/new?return_to=%2F%2Fevil.example%2Fsteal",
            "http://campfire.test/app/",
        ),
    ] {
        let app = app().await;
        reset_factor(&app, false).await;
        let mut browser = app.anonymous();
        if let Some(query) = start.strip_prefix("/session/new") {
            // The retained page hands its query to the SPA's sign-in page as it came.
            assert_retained_redirect(
                &app,
                &mut browser,
                Req::new(Method::GET, start),
                &format!("/app/session/new{query}"),
            )
            .await;
            browser.get(&format!("/app/session/new{query}")).await;
        } else {
            browser.get(start).await;
        }
        browser.get("/app/session/new").await;
        let reply = password(&mut browser, "david@37signals.com", "secret123456").await;
        assert_eq!(destination(&reply, true), expected);
    }
}

#[tokio::test]
async fn transfer_valid_reused_expired_and_invalid_links_match_retained_behavior() {
    for mode in ["valid", "expired", "invalid", "inactive"] {
        let app = app().await;
        reset_factor(&app, false).await;
        if mode == "inactive" {
            app.db()
                .write(|tx| {
                    tx.conn()
                        .execute("UPDATE users SET status=1 WHERE id=?", [DAVID])?;
                    Ok(())
                })
                .await
                .unwrap();
        }
        let now = app.booted.app.clock.now();
        let token = if mode == "invalid" {
            "invalid".into()
        } else {
            crate::controllers::presenters::accounts::transfer_id(
                &app.booted.app.secrets,
                DAVID,
                if mode == "expired" {
                    now - jiff::SignedDuration::from_hours(5)
                } else {
                    now
                },
            )
        };
        let path = format!("/api/v1/session/transfers/{token}");
        let mut browser = app.anonymous();
        browser.get("/app/settings/profile?transfer=1").await;
        browser.get("/app/session/new").await;
        // The retained link and its form leave the link unused for the SPA's page.
        for method in [Method::GET, Method::PUT, Method::PATCH] {
            assert_retained_redirect(
                &app,
                &mut browser,
                Req::new(method, &format!("/session/transfers/{token}")),
                &format!("/app/session/transfers/{token}"),
            )
            .await;
        }
        for repeat in 0..2 {
            let reply = browser
                .write(json_request(Method::PUT, &path, json!({})))
                .await;
            if mode == "valid" {
                assert_eq!(
                    destination(&reply, true),
                    if repeat == 0 {
                        "http://campfire.test/app/settings/profile?transfer=1"
                    } else {
                        "http://campfire.test/app/"
                    }
                );
                if repeat == 0 {
                    browser = app.anonymous();
                    browser.get("/app/session/new").await;
                }
            } else {
                assert_eq!(reply.status, StatusCode::BAD_REQUEST);
                assert_eq!(reply.json()["kind"], "error");
            }
        }
    }
}

#[tokio::test]
async fn transfer_two_factor_uses_the_same_pending_session_and_audit() {
    let app = app().await;
    let (secret, _) = reset_factor(&app, true).await;
    let token = crate::controllers::presenters::accounts::transfer_id(
        &app.booted.app.secrets,
        DAVID,
        app.booted.app.clock.now(),
    );
    let mut browser = app.anonymous();
    assert_retained_redirect(
        &app,
        &mut browser,
        Req::new(Method::PUT, &format!("/session/transfers/{token}")),
        &format!("/app/session/transfers/{token}"),
    )
    .await;
    let reply = browser
        .write(
            Req::new(Method::PUT, &format!("/api/v1/session/transfers/{token}"))
                .header("accept", "application/json"),
        )
        .await;
    assert_eq!(reply.json()["kind"], "secondFactorRequired");
    let code = totp::at(&secret, app.booted.app.clock.now().as_second()).unwrap();
    let reply = challenge(&mut browser, &code, false).await;
    assert_eq!(destination(&reply, true), "http://campfire.test/app/");
    let facts = facts(&app).await;
    assert_eq!(facts["sessions"].as_array().unwrap().last().unwrap()[0], DAVID);
    assert_eq!(
        facts["audits"],
        json!([["session.sign_in.success",DAVID,"David <david@37signals.com>",null,null,null,"{\"method\":\"transfer\",\"two_factor\":\"totp\"}","",null]])
    );
}

#[tokio::test]
async fn sign_out_revokes_the_same_session_cookie_and_push_subscription() {
    let mut outcomes = Vec::new();
    for json in [false, true] {
        let app = app().await;
        let mut browser = app.sign_in(DAVID).await;
        app.db()
            .write(|tx| {
                for user_id in [DAVID, crate::controllers::presenters::test_support::JASON] {
                    let subscription = campfire_db::PushSubscription::new(
                        user_id,
                        Some("https://fcm.googleapis.com/logout-test"),
                        Some("p256dh"),
                        Some("auth"),
                        None,
                    );
                    campfire_db::PushSubscription::create(tx, &subscription, &|_| {
                        Some("198.51.100.1".into())
                    })?;
                }
                Ok(())
            })
            .await
            .unwrap();
        let req = if json {
            json_request(
                Method::DELETE,
                "/api/v1/session",
                json!({"pushSubscriptionEndpoint":"https://fcm.googleapis.com/logout-test"}),
            )
        } else {
            Req::new(Method::DELETE, "/session").form(&[(
                "push_subscription_endpoint",
                "https://fcm.googleapis.com/logout-test",
            )])
        };
        let reply = browser.write(req).await;
        assert_eq!(destination(&reply, json), "http://campfire.test/");
        assert!(
            reply
                .headers
                .get_all("set-cookie")
                .iter()
                .any(|c| c.to_str().unwrap().starts_with("session_token=;"))
        );
        app.db()
            .read(|conn| {
                assert!(
                    !campfire_db::PushSubscription::for_user(conn, DAVID)?
                        .iter()
                        .any(|s| s.endpoint.as_deref()
                            == Some("https://fcm.googleapis.com/logout-test"))
                );
                assert!(
                    campfire_db::PushSubscription::for_user(
                        conn,
                        crate::controllers::presenters::test_support::JASON
                    )?
                    .iter()
                    .any(
                        |s| s.endpoint.as_deref() == Some("https://fcm.googleapis.com/logout-test")
                    )
                );
                Ok(())
            })
            .await
            .unwrap();
        outcomes.push((cookie_contract(&reply), facts(&app).await));
    }
    assert_eq!(outcomes[0], outcomes[1]);
}

#[tokio::test]
async fn signed_out_json_writes_require_real_csrf_and_preserve_rows_on_rejection() {
    let app = app().await;
    let mut browser = app.anonymous();
    let before = facts(&app).await;
    for (method, path, body) in [
        (
            Method::POST,
            "/api/v1/session",
            json!({"emailAddress":"david@37signals.com","password":"secret123456"}),
        ),
        (Method::POST, "/api/v1/session/google", json!({})),
        (
            Method::POST,
            "/api/v1/two_factor/challenge",
            json!({"code":"123456","rememberDevice":false}),
        ),
        (Method::PUT, "/api/v1/session/transfers/invalid", json!({})),
    ] {
        let reply = browser.send(json_request(method, path, body)).await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{path}");
    }
    assert_eq!(facts(&app).await, before);
}

/// The retained signed-out pages no longer answer for themselves: anyone, signed in or not, is
/// sent to the SPA's page with the query as it came, and that page decides what to show.
#[tokio::test]
async fn signed_out_first_run_boot_and_retained_routes_redirect_to_the_spa_pages() {
    let app = TestApp::boot_seed_with_env(
        "first_run",
        crate::controllers::presenters::test_support::seed_clock(),
        &[],
    )
    .await
    .unwrap();
    let mut browser = app.anonymous();
    let reply = browser.get("/app/session/new").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(inline_boot(&reply)["firstRunPending"], true);
    assert_eq!(
        browser.get("/session/new").await.location(),
        Some("http://campfire.test/app/first_run")
    );
    let app = self::app().await;
    let mut anonymous = app.anonymous();
    let mut signed_in = app.sign_in(DAVID).await;
    for (retained, spa) in [
        ("/session/new", "/app/session/new"),
        (
            "/session/transfers/example",
            "/app/session/transfers/example",
        ),
        ("/two_factor_challenge", "/app/two_factor/challenge"),
    ] {
        for browser in [&mut anonymous, &mut signed_in] {
            assert_retained_redirect(
                &app,
                browser,
                Req::new(Method::GET, &format!("{retained}?from=bookmark")),
                &format!("{spa}?from=bookmark"),
            )
            .await;
        }
        assert_eq!(anonymous.get(spa).await.status, StatusCode::OK, "{spa}");
        // As the retained pages did, only the challenge sends a signed-in person home.
        let reply = signed_in.get(spa).await;
        if retained == "/two_factor_challenge" {
            assert_eq!(reply.location(), Some("http://campfire.test/"), "{spa}");
        } else {
            assert_eq!(reply.status, StatusCode::OK, "{spa}");
        }
    }
}

#[tokio::test]
async fn google_start_shares_the_authorization_url_flow_cookie_and_return_path() {
    use crate::integrations::google::sign_in::{Config, SignIn};
    let mut outcomes = Vec::new();
    for json in [false, true] {
        let app = app().await;
        app.booted.app.google.install(SignIn::new(
            Config {
                client_id: "fixture-client".into(),
                client_secret: "fixture-secret".into(),
                domains: vec!["smartdata.net".into()],
            },
            crate::net::Network::system(),
        ));
        let mut browser = app.anonymous();
        browser.get("/app/settings/security?google=1").await;
        browser.get("/app/session/new").await;
        let req = Req::new(
            Method::POST,
            if json {
                "/api/v1/session/google"
            } else {
                "/session/google"
            },
        )
        .header(
            "accept",
            if json {
                "application/json"
            } else {
                "text/html"
            },
        );
        let reply = crate::test_support::with_oauth_entropy([7; 64], browser.write(req)).await;
        let url = destination(&reply, json);
        let parsed = url::Url::parse(&url).unwrap();
        assert_eq!(parsed.host_str(), Some("accounts.google.com"));
        let query = parsed
            .query_pairs()
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(
            query.get("redirect_uri").unwrap(),
            "http://campfire.test/session/google/callback"
        );
        let crypto = campfire_kit::RailsCrypto::new(app.booted.app.secrets.clone());
        let cookie = browser.cookie_header();
        let raw = cookie
            .split("; ")
            .find_map(|p| p.strip_prefix("_campfire_session="))
            .unwrap();
        let session = crypto
            .decrypt_cookie(
                "_campfire_session",
                &rails_compat::cookies::unescape(raw),
                app.booted.app.clock.now(),
            )
            .unwrap();
        assert_eq!(
            session["return_to_after_authenticating"],
            "http://campfire.test/app/settings/security?google=1"
        );
        let boot = browser.get("/api/v1/session/boot").await;
        if json {
            assert_eq!(boot.json()["signInMethods"]["google"], true);
            assert_eq!(
                boot.json()["signInMethods"]["googleDomains"],
                json!(["smartdata.net"])
            );
        }
        outcomes.push((
            url,
            session[crate::integrations::google::sign_in::FLOW_SESSION_KEY].clone(),
            cookie_contract(&reply),
            facts(&app).await,
        ));
    }
    assert_eq!(outcomes[0], outcomes[1]);
}

#[tokio::test]
async fn google_unconfigured_and_authenticated_and_signed_out_logout_answer_json_next_actions() {
    let app = app().await;
    let mut browser = app.anonymous();
    let disabled = browser
        .write(
            Req::new(Method::POST, "/api/v1/session/google").header("accept", "application/json"),
        )
        .await;
    assert_eq!(disabled.status, StatusCode::NOT_FOUND);
    assert_eq!(disabled.json(), json!({"kind":"error","fieldErrors":{}}));
    let logout = browser
        .write(json_request(
            Method::DELETE,
            "/api/v1/session",
            json!({"pushSubscriptionEndpoint":null}),
        ))
        .await;
    assert_eq!(
        logout.json(),
        json!({"kind":"navigate","location":"http://campfire.test/app/session/new"})
    );
    let mut browser = app.sign_in(DAVID).await;
    let html = browser
        .write(Req::new(Method::POST, "/session/google"))
        .await;
    let reply = browser
        .write(
            Req::new(Method::POST, "/api/v1/session/google").header("accept", "application/json"),
        )
        .await;
    assert_eq!(destination(&reply, true), html.location().unwrap());
}
