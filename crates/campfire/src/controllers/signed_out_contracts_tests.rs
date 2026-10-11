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

fn json_request(method: Method, path: &str, body: Value) -> Req {
    Req::new(method, path)
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&body).unwrap())
}

fn inline_boot(reply: &Reply) -> Value {
    let re =
        regex::Regex::new(r#"(?s)<script type="application/json" id="boot"[^>]*>(.*?)</script>"#)
            .unwrap();
    serde_json::from_str(&re.captures(&reply.text()).expect("boot script")[1]).unwrap()
}

async fn password(browser: &mut Browser<'_>, json: bool, email: &str, password: &str) -> Reply {
    let request = if json {
        json_request(
            Method::POST,
            "/api/v1/session",
            json!({"emailAddress": email, "password": password}),
        )
    } else {
        Req::new(Method::POST, "/session").form(&[("email_address", email), ("password", password)])
    };
    browser.write(request).await
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

fn cookie_contract(reply: &Reply) -> Vec<String> {
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
        Ok(json!({"sessions":sessions,"audits":audits,"devices":devices}))
    }).await.unwrap()
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

fn browser_session(app: &TestApp, browser: &Browser<'_>) -> Value {
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

fn body_token(mut request: Req, json: bool, token: &str) -> Req {
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
        browser.get("/session/new").await;
        let sign_in = password(&mut browser, json, "david@37signals.com", "secret123456").await;
        assert_eq!(destination(&sign_in, json), "http://campfire.test/app/");
        let before = facts(&app).await;
        let path = if json {
            "/api/v1/two_factor/challenge"
        } else {
            "/two_factor_challenge"
        };
        let reply = browser
            .send(Req::new(Method::GET, path).header(
                "accept",
                if json {
                    "application/json"
                } else {
                    "text/html"
                },
            ))
            .await;
        assert_eq!(
            reply.status,
            if json {
                StatusCode::OK
            } else {
                StatusCode::FOUND
            }
        );
        assert_eq!(destination(&reply, json), "http://campfire.test/");
        assert_eq!(
            browser.get("/app/two_factor/challenge").await.location(),
            Some("http://campfire.test/")
        );
        let reply = challenge(&mut browser, json, "123456", false).await;
        assert_eq!(destination(&reply, json), "http://campfire.test/");
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
            browser.get("/session/new").await;
            password(&mut browser, false, "david@37signals.com", "secret123456").await;
            assert_eq!(
                browser
                    .get("/app/settings/security?return=1")
                    .await
                    .location(),
                Some("http://campfire.test/two_factor_setup")
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
            let path = if spa {
                format!("/app{retained}")
            } else {
                retained
            };
            let reply = browser.get(&path).await;
            assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.text());
            assert_eq!(reply.location(), None);
            if spa {
                let boot = inline_boot(&reply);
                assert_eq!(boot["kind"], "signedOut");
                // The help contact and the version joined the five original keys with slice 45.
                assert_eq!(boot.as_object().unwrap().len(), 7);
            } else {
                assert!(reply.text().contains("<form"));
            }
            assert_eq!(
                browser_session(&app, &browser)["return_to_after_authenticating"],
                return_path
            );
            assert_eq!(facts(&app).await, before);
            let reply = browser
                .write(Req::new(
                    Method::PUT,
                    &format!("/session/transfers/{token}"),
                ))
                .await;
            assert_eq!(
                reply.location(),
                Some("http://campfire.test/app/settings/security?return=1")
            );
            outcomes.push(facts(&app).await);
        }
        assert_eq!(outcomes[0], outcomes[1]);
    }
}

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
            let requests = if json {
                vec![
                    json_request(
                        Method::POST,
                        "/api/v1/session",
                        json!({"emailAddress":"david@37signals.com","password":"secret123456"}),
                    ),
                    json_request(
                        Method::POST,
                        "/api/v1/two_factor/challenge",
                        json!({"code":code,"rememberDevice":false}),
                    ),
                    json_request(Method::DELETE, "/api/v1/session", json!({})),
                ]
            } else {
                vec![
                    Req::new(Method::POST, "/session").form(&[
                        ("email_address", "david@37signals.com"),
                        ("password", "secret123456"),
                    ]),
                    Req::new(Method::POST, "/two_factor_challenge")
                        .form(&[("code", &code), ("remember_device", "0")]),
                    Req::new(Method::DELETE, "/session").form(&[]),
                ]
            };
            let mut cookies = Vec::new();
            for (step, request) in requests.into_iter().enumerate() {
                let before = facts(&app).await;
                for rejected in [
                    body_token(request.clone(), json, "invalid"),
                    body_token(request.clone(), json, token)
                        .header("origin", "https://attacker.test"),
                    body_token(request.clone(), json, token).header("origin", "null"),
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
                    body_token(request, json, if valid_body { token } else { "invalid" });
                if let Some(valid) = valid_header {
                    request = request.header("x-csrf-token", if valid { token } else { "invalid" });
                }
                let reply = browser.send(request).await;
                assert_eq!(
                    reply.status,
                    if json {
                        StatusCode::OK
                    } else {
                        StatusCode::FOUND
                    },
                    "step {step}: {}",
                    reply.text()
                );
                if step == 0 {
                    if json {
                        assert_eq!(reply.json()["kind"], "secondFactorRequired");
                    } else {
                        assert_eq!(
                            reply.location(),
                            Some("http://campfire.test/two_factor_challenge")
                        );
                    }
                } else {
                    assert_eq!(
                        destination(&reply, json),
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
                "helpContact",
                "kind",
                "signInMethods",
                "version",
                "workspace"
            ]
        );
        assert_eq!(boot["workspace"].as_object().unwrap().len(), 3);
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
            Some("http://campfire.test/session/new"),
            "{path}"
        );
    }
    assert!(
        browser
            .get("/session/new")
            .await
            .text()
            .contains("name=\"email_address\"")
    );
}

#[tokio::test]
async fn password_success_creates_the_same_session_cookies_and_audit() {
    let mut outcomes = Vec::new();
    for json in [false, true] {
        let app = app().await;
        reset_factor(&app, false).await;
        let mut browser = app.anonymous();
        browser.get("/session/new").await;
        let reply = password(&mut browser, json, "david@37signals.com", "secret123456").await;
        assert_eq!(destination(&reply, json), "http://campfire.test/app/");
        if json {
            assert_eq!(reply.json()["kind"], "signedIn");
        }
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
        outcomes.push((cookie_contract(&reply), facts(&app).await));
    }
    assert_eq!(outcomes[0], outcomes[1]);
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
        let mut outcomes = Vec::new();
        for json in [false, true] {
            let app = app().await;
            let mut browser = app.anonymous();
            browser.get("/session/new").await;
            let mut reply = self::password(&mut browser, json, email, password).await;
            for _ in 1..attempts {
                reply = self::password(&mut browser, json, email, password).await;
            }
            assert_eq!(reply.status, status);
            if json {
                assert_eq!(
                    reply.json(),
                    json!({"kind":"error","fieldErrors":{"base":["Too many requests or unauthorized."]}})
                );
            } else {
                assert!(reply.text().contains("Too many requests or unauthorized."));
            }
            assert!(!browser.cookie_header().contains("session_token="));
            outcomes.push(facts(&app).await);
        }
        assert_eq!(outcomes[0], outcomes[1]);
    }
}

async fn challenge(browser: &mut Browser<'_>, json: bool, code: &str, remember: bool) -> Reply {
    let req = if json {
        json_request(
            Method::POST,
            "/api/v1/two_factor/challenge",
            json!({"code":code,"rememberDevice":remember}),
        )
    } else {
        Req::new(Method::POST, "/two_factor_challenge").form(&[
            ("code", code),
            ("remember_device", if remember { "1" } else { "0" }),
        ])
    };
    let req = if remember {
        req.header("x-forwarded-proto", "https")
    } else {
        req
    };
    browser.write(req).await
}

#[tokio::test]
async fn challenge_totp_recovery_and_remembered_device_have_identical_effects() {
    for recovery in [false, true] {
        let mut outcomes = Vec::new();
        for json in [false, true] {
            let app = app().await;
            let (secret, codes) = reset_factor(&app, true).await;
            let mut browser = app.anonymous();
            browser.get("/app/settings/security?tab=two_factor").await;
            browser.get("/session/new").await;
            let first = password(&mut browser, json, "david@37signals.com", "secret123456").await;
            assert!(!browser.cookie_header().contains("session_token="));
            if json {
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
            } else {
                assert_eq!(
                    first.location(),
                    Some("http://campfire.test/two_factor_challenge")
                );
            }
            let wrong = challenge(&mut browser, json, "invalid", false).await;
            assert_eq!(wrong.status, StatusCode::UNPROCESSABLE_ENTITY);
            let message =
                "That code didn't work. Check your authenticator app or try a backup code.";
            if json {
                assert_eq!(wrong.json()["fieldErrors"]["code"], json!([message]));
            } else {
                crate::form_contracts::assert_text(&wrong.text(), message);
            }
            let code = if recovery {
                codes[0].clone()
            } else {
                totp::at(&secret, app.booted.app.clock.now().as_second()).unwrap()
            };
            let reply = challenge(&mut browser, json, &code, true).await;
            assert_eq!(
                destination(&reply, json),
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
            outcomes.push((cookie_contract(&reply), facts(&app).await));
            // The remembered device must skip the next challenge and verify the new session.
            browser.write(Req::new(Method::DELETE, "/session")).await;
            let reply = password(&mut browser, json, "david@37signals.com", "secret123456").await;
            assert_eq!(destination(&reply, json), "http://campfire.test/app/");
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
        assert_eq!(outcomes[0], outcomes[1]);
    }
}

#[tokio::test]
async fn challenge_recovery_replay_lockout_and_rate_limit_match_html() {
    let mut outcomes = Vec::new();
    for json in [false, true] {
        let app = app().await;
        let (_, codes) = reset_factor(&app, true).await;
        let mut browser = app.anonymous();
        password(&mut browser, json, "david@37signals.com", "secret123456").await;
        let accepted = challenge(&mut browser, json, &codes[0], false).await;
        assert_eq!(destination(&accepted, json), "http://campfire.test/app/");
        browser.write(Req::new(Method::DELETE, "/session")).await;
        password(&mut browser, json, "david@37signals.com", "secret123456").await;
        for attempt in 1..=10 {
            let reply = challenge(&mut browser, json, &codes[0], false).await;
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
                if json {
                    assert_eq!(reply.json()["fieldErrors"]["code"], json!([message]));
                } else {
                    assert!(reply.text().contains(message));
                }
            }
            if attempt == 10 {
                let message = "Too many attempts. Try again in a few minutes.";
                if json {
                    assert_eq!(reply.json()["fieldErrors"]["code"], json!([message]));
                } else {
                    assert!(reply.text().contains(message));
                }
            }
        }
        outcomes.push(facts(&app).await);
    }
    assert_eq!(outcomes[0], outcomes[1]);
}

#[tokio::test]
async fn challenge_absent_expired_and_deactivated_pending_state_navigates_back_to_sign_in() {
    for expired in [false, true] {
        let mut outcomes = Vec::new();
        for json in [false, true] {
            let app = app().await;
            let mut browser = app.anonymous();
            if expired {
                reset_factor(&app, true).await;
                password(&mut browser, json, "david@37signals.com", "secret123456").await;
                replace_session_value(
                    &app,
                    &mut browser,
                    "two_factor_pending_expires_at",
                    json!(0),
                );
            }
            let reply = challenge(&mut browser, json, "123456", false).await;
            assert_eq!(
                destination(&reply, json),
                "http://campfire.test/session/new"
            );
            outcomes.push(facts(&app).await);
        }
        assert_eq!(outcomes[0], outcomes[1]);
    }
    let app = app().await;
    let mut browser = app.anonymous();
    reset_factor(&app, true).await;
    password(&mut browser, true, "david@37signals.com", "secret123456").await;
    app.db()
        .write(|tx| {
            tx.conn()
                .execute("UPDATE users SET status=1 WHERE id=?", [DAVID])?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        destination(&challenge(&mut browser, true, "123456", false).await, true),
        "http://campfire.test/session/new"
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
        let mut outcomes = Vec::new();
        for json in [false, true] {
            let app = app().await;
            reset_factor(&app, false).await;
            let mut browser = app.anonymous();
            browser.get(start).await;
            browser.get("/session/new").await;
            let reply = password(&mut browser, json, "david@37signals.com", "secret123456").await;
            assert_eq!(destination(&reply, json), expected);
            outcomes.push(facts(&app).await);
        }
        assert_eq!(outcomes[0], outcomes[1]);
    }
}

#[tokio::test]
async fn transfer_valid_reused_expired_and_invalid_links_match_retained_behavior() {
    for mode in ["valid", "expired", "invalid", "inactive"] {
        let mut outcomes = Vec::new();
        for json in [false, true] {
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
            let path = format!(
                "{}/{token}",
                if json {
                    "/api/v1/session/transfers"
                } else {
                    "/session/transfers"
                }
            );
            let mut browser = app.anonymous();
            browser.get("/app/settings/profile?transfer=1").await;
            browser.get("/session/new").await;
            for repeat in 0..2 {
                let req = if json {
                    json_request(Method::PUT, &path, json!({}))
                } else {
                    Req::new(Method::PUT, &path)
                };
                let reply = browser.write(req).await;
                if mode == "valid" {
                    assert_eq!(
                        destination(&reply, json),
                        if repeat == 0 {
                            "http://campfire.test/app/settings/profile?transfer=1"
                        } else {
                            "http://campfire.test/app/"
                        }
                    );
                    if repeat == 0 {
                        browser = app.anonymous();
                        browser.get("/session/new").await;
                    }
                } else {
                    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
                    if json {
                        assert_eq!(reply.json()["kind"], "error");
                    }
                }
            }
            outcomes.push(facts(&app).await);
        }
        assert_eq!(outcomes[0], outcomes[1]);
    }
}

#[tokio::test]
async fn transfer_two_factor_uses_the_same_pending_session_and_audit() {
    let mut outcomes = Vec::new();
    for json in [false, true] {
        let app = app().await;
        let (secret, _) = reset_factor(&app, true).await;
        let token = crate::controllers::presenters::accounts::transfer_id(
            &app.booted.app.secrets,
            DAVID,
            app.booted.app.clock.now(),
        );
        let mut browser = app.anonymous();
        let path = format!(
            "{}/{token}",
            if json {
                "/api/v1/session/transfers"
            } else {
                "/session/transfers"
            }
        );
        let reply = browser
            .write(Req::new(Method::PUT, &path).header(
                "accept",
                if json {
                    "application/json"
                } else {
                    "text/html"
                },
            ))
            .await;
        if json {
            assert_eq!(reply.json()["kind"], "secondFactorRequired");
        } else {
            assert_eq!(
                reply.location(),
                Some("http://campfire.test/two_factor_challenge")
            );
        }
        let code = totp::at(&secret, app.booted.app.clock.now().as_second()).unwrap();
        let reply = challenge(&mut browser, json, &code, false).await;
        assert_eq!(destination(&reply, json), "http://campfire.test/app/");
        outcomes.push(facts(&app).await);
    }
    assert_eq!(outcomes[0], outcomes[1]);
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

#[tokio::test]
async fn signed_out_first_run_boot_and_authenticated_routes_keep_retained_pages() {
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
        Some("http://campfire.test/first_run")
    );
    let app = self::app().await;
    let mut browser = app.sign_in(DAVID).await;
    for (retained, spa) in [
        ("/session/new", "/app/session/new"),
        (
            "/session/transfers/example",
            "/app/session/transfers/example",
        ),
        ("/two_factor_challenge", "/app/two_factor/challenge"),
    ] {
        let html = browser.get(retained).await;
        let reply = browser.get(spa).await;
        assert_eq!(reply.status, html.status, "{spa}");
        assert_eq!(reply.location(), html.location(), "{spa}");
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
        browser.get("/session/new").await;
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
        json!({"kind":"navigate","location":"http://campfire.test/session/new"})
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
