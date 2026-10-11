use crate::controllers::presenters::test_support::{
    BENDER, Browser, DAVID, JASON, KEVIN, Req, TestApp,
};
use axum::http::{Method, StatusCode};
use campfire_db::TwoFactorCredential;
use campfire_kit::{Crypto, RailsCrypto};
use rails_compat::ar_encryption::ArEncryption;
use serde_json::{Value, json};

fn request(method: Method, path: &str, body: Value) -> Req {
    Req::new(method, path)
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&body).unwrap())
}

fn session(app: &TestApp, browser: &Browser<'_>) -> Value {
    let cookies = browser.cookie_header();
    let raw = cookies
        .split("; ")
        .find_map(|cookie| cookie.strip_prefix("_campfire_session="))
        .unwrap();
    RailsCrypto::new(app.booted.app.secrets.clone())
        .decrypt_cookie(
            "_campfire_session",
            &rails_compat::cookies::unescape(raw),
            app.booted.app.clock.now(),
        )
        .unwrap()
}

async fn facts(app: &TestApp) -> Value {
    app.db().read(|conn| {
        let mut rows = conn.prepare("SELECT action,actor_id,details,ip_address,user_agent,created_at FROM audit_logs WHERE action LIKE 'sudo.confirm.%' ORDER BY id")?;
        let audits = rows.query_map([], |r| Ok(json!([r.get::<_, String>(0)?, r.get::<_, Option<i64>>(1)?, r.get::<_, Option<String>>(2)?, r.get::<_, Option<String>>(3)?, r.get::<_, Option<String>>(4)?, r.get::<_, String>(5)?])))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let factor = TwoFactorCredential::for_user(conn, DAVID)?.unwrap();
        Ok(json!({"audits":audits,"factor":[factor.last_totp_at, factor.consecutive_failures, factor.lockout_count, factor.locked_until.map(|time| time.to_string())]}))
    }).await.unwrap()
}

#[tokio::test]
async fn sudo_review_body_token_matches_retained_confirmation() {
    let mut outcomes = Vec::new();
    for json_mode in [false, true] {
        let app = TestApp::boot_frozen().await.unwrap();
        let mut browser = app.sign_in(DAVID).await;
        let token = browser.authenticity_token().await;
        let before = facts(&app).await;
        let req = if json_mode {
            request(
                Method::POST,
                "/api/v1/sudo",
                json!({"kind":"password","password":"secret123456","authenticity_token":token}),
            )
        } else {
            Req::new(Method::POST, "/sudo")
                .form(&[("password", "secret123456"), ("authenticity_token", &token)])
        };
        for origin in ["https://attacker.test", "null"] {
            let reply = browser.send(req.clone().header("origin", origin)).await;
            assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(facts(&app).await, before);
        }
        let reply = browser.send(req).await;
        assert_eq!(
            reply.status,
            if json_mode {
                StatusCode::OK
            } else {
                StatusCode::FOUND
            },
            "{}",
            reply.text()
        );
        outcomes.push((
            session(&app, &browser)["sudo_verified_at"].clone(),
            facts(&app).await,
        ));
    }
    assert_eq!(outcomes[0], outcomes[1]);
}

#[tokio::test]
async fn sudo_review_before_action_refusals_decode_and_require_sign_in() {
    for (method, path) in [
        (Method::GET, "/api/v1/sudo"),
        (Method::POST, "/api/v1/sudo"),
        (Method::POST, "/api/v1/sudo/google"),
        (Method::GET, "/api/v1/sudo/continue"),
    ] {
        let mut observed = Vec::new();
        for json_mode in [false, true] {
            let app = TestApp::boot_frozen().await.unwrap();
            let mut browser = app.sign_in(DAVID).await;
            browser.authenticity_token().await;
            let sessions_before = app
                .db()
                .read(|conn| {
                    Ok(conn.query_row(
                        "SELECT count(*) FROM sessions WHERE user_id=?",
                        [DAVID],
                        |r| r.get::<_, i64>(0),
                    )?)
                })
                .await
                .unwrap();
            app.db()
                .write(|tx| {
                    tx.conn().execute(
                        "UPDATE sessions SET two_factor_verified_at=NULL WHERE user_id=?",
                        [DAVID],
                    )?;
                    Ok(())
                })
                .await
                .unwrap();
            let req = if json_mode {
                request(
                    method.clone(),
                    path,
                    json!({"kind":"password","password":"secret123456"}),
                )
            } else {
                Req::new(
                    method.clone(),
                    if method == Method::GET {
                        "/sudo/new"
                    } else if path.ends_with("google") {
                        "/sudo/google"
                    } else {
                        "/sudo"
                    },
                )
                .form(&[("password", "secret123456")])
            };
            let reply = browser.write(req).await;
            let location = if json_mode {
                assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
                assert_eq!(reply.header("cache-control"), Some("no-store"));
                let body: campfire_api_types::SudoResponse =
                    serde_json::from_slice(&reply.body).expect("sudo callback contract");
                let campfire_api_types::SudoResponse::Navigate { location } = body else {
                    panic!("sign-in navigation required")
                };
                location
            } else {
                assert_eq!(reply.status, StatusCode::FOUND);
                reply.location().unwrap().to_owned()
            };
            assert_eq!(location, "http://campfire.test/session/new");
            assert_eq!(
                app.db()
                    .read(|conn| Ok(conn.query_row(
                        "SELECT count(*) FROM sessions WHERE user_id=?",
                        [DAVID],
                        |r| r.get::<_, i64>(0)
                    )?))
                    .await
                    .unwrap(),
                sessions_before - 1
            );
            observed.push(facts(&app).await);
        }
        assert_eq!(observed[0], observed[1], "{method} {path}");
    }
}

#[tokio::test]
async fn sudo_contract_password_and_totp_share_timestamps_audits_and_lockout() {
    for method in ["password", "totp"] {
        let mut observed = Vec::new();
        for json_mode in [false, true] {
            let app = TestApp::boot_frozen().await.unwrap();
            let mut browser = app.sign_in(DAVID).await;
            let encryption = ArEncryption::new(&app.booted.app.secrets);
            let secret = app.db().write(move |tx| {
                let factor = TwoFactorCredential::for_user(tx.conn(), DAVID)?.unwrap();
                tx.conn().execute("UPDATE two_factor_credentials SET last_totp_at=NULL,consecutive_failures=0,lockout_count=0,locked_until=NULL WHERE id=?", [factor.id])?;
                factor.secret(&encryption)
            }).await.unwrap();
            let code =
                rails_compat::totp::at(&secret, app.booted.app.clock.now().as_second()).unwrap();
            for value in [
                "wrong",
                if method == "password" {
                    "secret123456"
                } else {
                    &code
                },
            ] {
                let req = if json_mode {
                    request(
                        Method::POST,
                        "/api/v1/sudo",
                        if method == "password" {
                            json!({"kind":"password","password":value})
                        } else {
                            json!({"kind":"totp","code":value})
                        },
                    )
                } else {
                    Req::new(Method::POST, "/sudo").form(&[
                        ("verifier", method),
                        (
                            if method == "password" {
                                "password"
                            } else {
                                "totp_code"
                            },
                            value,
                        ),
                    ])
                };
                let reply = browser.write(req).await;
                if value == "wrong" {
                    assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{}", reply.text());
                    assert!(reply.text().contains("Confirmation failed. Try again."));
                    assert!(session(&app, &browser).get("sudo_verified_at").is_none());
                } else {
                    assert_eq!(
                        reply.status,
                        if json_mode {
                            StatusCode::OK
                        } else {
                            StatusCode::FOUND
                        },
                        "{}",
                        reply.text()
                    );
                    assert_eq!(
                        session(&app, &browser)["sudo_verified_at"],
                        app.booted.app.clock.now().as_second()
                    );
                }
            }
            observed.push((
                session(&app, &browser)["sudo_verified_at"].clone(),
                facts(&app).await,
            ));
        }
        assert_eq!(observed[0], observed[1], "{method}");
    }
}

#[tokio::test]
async fn sudo_contract_ten_attempts_are_shared_across_html_json_and_google() {
    let app = TestApp::boot_frozen().await.unwrap();
    let mut browser = app.sign_in(DAVID).await;
    for attempt in 0..10 {
        let reply = match attempt % 3 {
            0 => {
                browser
                    .write(Req::new(Method::POST, "/sudo").form(&[("password", "wrong")]))
                    .await
            }
            1 => {
                browser
                    .write(request(
                        Method::POST,
                        "/api/v1/sudo",
                        json!({"kind":"password","password":"wrong"}),
                    ))
                    .await
            }
            _ => {
                browser
                    .write(request(Method::POST, "/api/v1/sudo/google", json!({})))
                    .await
            }
        };
        assert_eq!(
            reply.status,
            if attempt % 3 == 2 {
                StatusCode::UNPROCESSABLE_ENTITY
            } else {
                StatusCode::UNAUTHORIZED
            },
            "{}",
            reply.text()
        );
    }
    for path in ["/api/v1/sudo", "/api/v1/sudo/google"] {
        let reply = browser
            .write(request(
                Method::POST,
                path,
                json!({"kind":"password","password":"secret123456"}),
            ))
            .await;
        assert_eq!(
            reply.status,
            StatusCode::TOO_MANY_REQUESTS,
            "{}",
            reply.text()
        );
        assert_eq!(
            reply.json(),
            json!({"kind":"error","message":"Too many confirmation attempts. Try again in a few minutes."})
        );
    }
    assert_eq!(
        browser
            .write(Req::new(Method::POST, "/sudo").form(&[("password", "secret123456")]))
            .await
            .status,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert!(session(&app, &browser).get("sudo_verified_at").is_none());
}

#[tokio::test]
async fn sudo_contract_expired_confirmation_does_not_resume_and_bad_bodies_do_not_audit() {
    let app = TestApp::boot_frozen().await.unwrap();
    let mut browser = app.sign_in(DAVID).await;
    browser
        .write(request(
            Method::POST,
            "/api/v1/admin/workspace/join_code",
            json!({}),
        ))
        .await;
    browser
        .write(request(
            Method::POST,
            "/api/v1/sudo",
            json!({"kind":"password","password":"secret123456"}),
        ))
        .await;
    let mut state = session(&app, &browser);
    state["sudo_verified_at"] = json!(app.booted.app.clock.now().as_second() - 900);
    let crypto = RailsCrypto::new(app.booted.app.secrets.clone());
    browser.absorb_cookie_header(&format!(
        "_campfire_session={}",
        campfire_kit::cookies::escape(&crypto.encrypt_cookie("_campfire_session", &state, None))
    ));
    let reply = browser.get("/api/v1/sudo/continue").await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.text());
    assert_eq!(reply.json()["kind"], "ready");
    let before = facts(&app).await;
    for body in [
        json!({"kind":"recoveryCode","code":"secret"}),
        json!({"kind":"password"}),
    ] {
        let reply = browser
            .write(request(Method::POST, "/api/v1/sudo", body))
            .await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            reply.text()
        );
    }
    assert_eq!(facts(&app).await, before);
}

#[tokio::test]
async fn sudo_contract_totp_lockout_and_recovery_code_refusal_match_html() {
    let mut outcomes = Vec::new();
    for json_mode in [false, true] {
        let app = TestApp::boot_frozen().await.unwrap();
        let mut browser = app.sign_in(DAVID).await;
        let encryption = ArEncryption::new(&app.booted.app.secrets);
        let (secret, recovery) = app.db().write(move |tx| {
            let factor = TwoFactorCredential::for_user(tx.conn(), DAVID)?.unwrap();
            tx.conn().execute("UPDATE two_factor_credentials SET last_totp_at=NULL,consecutive_failures=0,lockout_count=0,locked_until=NULL WHERE id=?", [factor.id])?;
            Ok((factor.secret(&encryption)?, campfire_db::TwoFactorBackupCode::regenerate_set(tx, factor.id)?))
        }).await.unwrap();
        let code = rails_compat::totp::at(&secret, app.booted.app.clock.now().as_second()).unwrap();
        for value in [&recovery[0], "wrong", "wrong", "wrong", "wrong", &code] {
            let req = if json_mode {
                request(
                    Method::POST,
                    "/api/v1/sudo",
                    json!({"kind":"totp","code":value}),
                )
            } else {
                Req::new(Method::POST, "/sudo").form(&[("verifier", "totp"), ("totp_code", value)])
            };
            let reply = browser.write(req).await;
            assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{}", reply.text());
            assert!(reply.text().contains("Confirmation failed. Try again."));
        }
        assert!(session(&app, &browser).get("sudo_verified_at").is_none());
        let now = campfire_db::Timestamp::from_jiff(app.booted.app.clock.now());
        assert_eq!(
            app.db()
                .read(move |conn| {
                    let factor = TwoFactorCredential::for_user(conn, DAVID)?.unwrap();
                    assert!(factor.locked_out(now));
                    Ok(
                        campfire_db::TwoFactorBackupCode::for_credential(conn, factor.id)?
                            .iter()
                            .filter(|code| code.used_at.is_none())
                            .count(),
                    )
                })
                .await
                .unwrap(),
            10
        );
        outcomes.push(facts(&app).await);
    }
    assert_eq!(outcomes[0], outcomes[1]);
}

#[tokio::test]
async fn sudo_contract_methods_unavailable_totp_and_authentication_csrf_keep_html_contracts() {
    let app = TestApp::boot_frozen().await.unwrap();
    app.db()
        .write(|tx| {
            TwoFactorCredential::for_user(tx.conn(), DAVID)?
                .unwrap()
                .destroy(tx)?;
            Ok(())
        })
        .await
        .unwrap();
    let mut browser = app.sign_in(DAVID).await;
    let ready = browser.get("/api/v1/sudo").await;
    assert_eq!(
        ready.json(),
        json!({"kind":"ready","reauthentication":{"methods":["password"],"retry":null}})
    );
    assert_eq!(ready.header("cache-control"), Some("no-store"));
    for json_mode in [false, true] {
        let req = if json_mode {
            request(
                Method::POST,
                "/api/v1/sudo",
                json!({"kind":"totp","code":"123456"}),
            )
        } else {
            Req::new(Method::POST, "/sudo").form(&[("verifier", "totp"), ("totp_code", "123456")])
        };
        let reply = browser.write(req).await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            reply.text()
        );
        assert!(
            reply
                .text()
                .contains("That confirmation method is not available.")
        );
    }
    assert_eq!(
        app.db()
            .read(|conn| Ok(conn.query_row(
                "SELECT count(*) FROM audit_logs WHERE action LIKE 'sudo.confirm.%'",
                [],
                |r| r.get::<_, i64>(0)
            )?))
            .await
            .unwrap(),
        0
    );
    let rejected = browser
        .send(request(
            Method::POST,
            "/api/v1/sudo",
            json!({"kind":"password","password":"secret123456"}),
        ))
        .await;
    assert_eq!(rejected.status, StatusCode::UNPROCESSABLE_ENTITY);
    let mut anonymous = app.anonymous();
    for path in ["/api/v1/sudo", "/api/v1/sudo/continue"] {
        let reply = anonymous.get(path).await;
        assert_eq!(reply.json()["kind"], "navigate");
        assert_eq!(reply.json()["location"], "http://campfire.test/session/new");
    }
}

#[tokio::test]
async fn sudo_contract_typed_gate_and_retry_do_not_execute_the_write_early() {
    let app = TestApp::boot_frozen().await.unwrap();
    let mut browser = app.sign_in(DAVID).await;
    let before = app
        .db()
        .read(campfire_db::Account::first)
        .await
        .unwrap()
        .unwrap()
        .join_code;
    let path = "/api/v1/admin/workspace/join_code";
    let gated = browser
        .write(
            request(Method::POST, path, json!({}))
                .header("referer", "http://campfire.test/app/admin?tab=workspace"),
        )
        .await;
    assert_eq!(gated.status, StatusCode::FORBIDDEN);
    assert_eq!(
        gated.json(),
        json!({"error":{"_tag":"SudoRequired","message":"Confirm your password to continue","reauthentication":{"methods":["password","totp"],"retry":{"method":"POST","path":path,"returnTo":"/app/admin"}}}})
    );
    assert_eq!(
        app.db()
            .read(campfire_db::Account::first)
            .await
            .unwrap()
            .unwrap()
            .join_code,
        before
    );
    let confirmed = browser
        .write(request(
            Method::POST,
            "/api/v1/sudo",
            json!({"kind":"password","password":"secret123456"}),
        ))
        .await;
    assert_eq!(confirmed.status, StatusCode::OK, "{}", confirmed.text());
    assert_eq!(confirmed.json()["kind"], "confirmed");
    assert_eq!(confirmed.json()["retry"]["path"], path);
    assert!(
        session(&app, &browser)
            .get("sudo_pending_request")
            .is_none()
    );
    assert_eq!(
        browser
            .write(request(Method::POST, path, json!({})))
            .await
            .status,
        StatusCode::OK
    );
    assert_ne!(
        app.db()
            .read(campfire_db::Account::first)
            .await
            .unwrap()
            .unwrap()
            .join_code,
        before
    );
}

#[tokio::test]
async fn sudo_contract_all_retained_callers_negotiate_json_before_mutation() {
    let app = TestApp::boot_with_github_network_and_env(crate::net::Network::system(), &[])
        .await
        .unwrap();
    crate::app::google_api_tests::install(
        &app,
        crate::app::google_api_tests::Recorded::new(vec![]),
    )
    .await;
    for (method, path, pairs) in [
        (Method::POST, "/account/join_code".into(), vec![]),
        (
            Method::PATCH,
            "/account/custom_styles".into(),
            vec![("account[custom_styles]", "body{}")],
        ),
        (
            Method::POST,
            "/account/bots".into(),
            vec![("user[name]", "New bot")],
        ),
        (
            Method::PATCH,
            format!("/account/bots/{BENDER}"),
            vec![("user[webhook_url]", "https://example.test/changed")],
        ),
        (Method::PUT, format!("/account/bots/{BENDER}/key"), vec![]),
        (
            Method::POST,
            format!("/account/bots/{BENDER}/webhook_secret"),
            vec![],
        ),
        (
            Method::POST,
            format!("/account/bots/{BENDER}/credentials"),
            vec![],
        ),
        (
            Method::DELETE,
            format!("/account/bots/{BENDER}/credentials/1"),
            vec![],
        ),
        (
            Method::POST,
            format!("/account/bots/{BENDER}/grants"),
            vec![],
        ),
        (
            Method::DELETE,
            format!("/account/bots/{BENDER}/grants/1"),
            vec![],
        ),
        (
            Method::POST,
            format!("/account/bots/{BENDER}/github_connection"),
            vec![],
        ),
        (
            Method::DELETE,
            format!("/account/bots/{BENDER}/github_connection"),
            vec![],
        ),
        (
            Method::PATCH,
            format!("/account/users/{JASON}"),
            vec![("user[role]", "member")],
        ),
        (Method::DELETE, format!("/account/users/{JASON}"), vec![]),
        (Method::POST, format!("/users/{KEVIN}/ban"), vec![]),
        (Method::DELETE, format!("/users/{KEVIN}/ban"), vec![]),
        (Method::GET, "/account/audit_log.csv".into(), vec![]),
        (
            Method::POST,
            "/fizzy/connection".into(),
            vec![("access_token", "secret")],
        ),
        (Method::DELETE, "/fizzy/connection".into(), vec![]),
        (Method::POST, "/github/connection".into(), vec![]),
        (Method::DELETE, "/github/connection".into(), vec![]),
        (Method::GET, "/github/app/connect".into(), vec![]),
        (Method::POST, "/google/connect".into(), vec![]),
        (Method::DELETE, "/google/connection".into(), vec![]),
        (Method::GET, "/slack/oauth/start".into(), vec![]),
        (Method::DELETE, "/slack/connection".into(), vec![]),
        (Method::PATCH, "/account/slack_import".into(), vec![]),
        (Method::DELETE, "/account/slack_import".into(), vec![]),
    ] {
        for json_mode in [false, true] {
            let mut browser = app.sign_in(DAVID).await;
            let mut req = Req::new(method.clone(), &path).form(&pairs);
            if json_mode {
                req = req.header("accept", "application/json");
            }
            let reply = browser.write(req).await;
            assert_eq!(
                reply.status,
                if json_mode {
                    StatusCode::FORBIDDEN
                } else {
                    StatusCode::FOUND
                },
                "{method} {path}: {}",
                reply.text()
            );
            if json_mode {
                assert_eq!(reply.json()["error"]["_tag"], "SudoRequired");
                assert_eq!(
                    reply.json()["error"]["reauthentication"]["retry"]["path"],
                    path
                );
                assert!(session(&app, &browser)["sudo_pending_request"]["params"].is_null());
            } else {
                assert_eq!(reply.location(), Some("http://campfire.test/sudo/new"));
            }
        }
    }
}

#[tokio::test]
async fn sudo_contract_every_spa_sensitive_write_returns_methods_and_retry() {
    let app = TestApp::boot_frozen().await.unwrap();
    crate::app::google_api_tests::install(
        &app,
        crate::app::google_api_tests::Recorded::new(vec![]),
    )
    .await;
    for (method, path, body) in [
        (
            Method::POST,
            "/api/v1/admin/workspace/join_code".into(),
            json!({}),
        ),
        (
            Method::PATCH,
            "/api/v1/admin/custom_styles".into(),
            json!({}),
        ),
        (
            Method::PATCH,
            format!("/api/v1/admin/people/{JASON}"),
            json!({}),
        ),
        (
            Method::DELETE,
            format!("/api/v1/admin/people/{JASON}"),
            json!({}),
        ),
        (
            Method::POST,
            format!("/api/v1/people/{KEVIN}/ban"),
            json!({}),
        ),
        (
            Method::DELETE,
            format!("/api/v1/people/{KEVIN}/ban"),
            json!({}),
        ),
        (Method::POST, "/api/v1/admin/bots".into(), json!({})),
        (
            Method::PATCH,
            format!("/api/v1/admin/bots/{BENDER}"),
            json!({"webhookUrl":"https://example.test/new"}),
        ),
        (
            Method::PUT,
            format!("/api/v1/admin/bots/{BENDER}/key"),
            json!({}),
        ),
        (
            Method::POST,
            format!("/api/v1/admin/bots/{BENDER}/webhook_secret"),
            json!({}),
        ),
        (
            Method::PUT,
            format!("/api/v1/admin/bots/{BENDER}/github_connection"),
            json!({}),
        ),
        (
            Method::DELETE,
            format!("/api/v1/admin/bots/{BENDER}/github_connection"),
            json!({}),
        ),
        (
            Method::POST,
            format!("/api/v1/admin/bots/{BENDER}/credentials"),
            json!({}),
        ),
        (
            Method::DELETE,
            format!("/api/v1/admin/bots/{BENDER}/credentials/1"),
            json!({}),
        ),
        (
            Method::POST,
            format!("/api/v1/admin/bots/{BENDER}/grants"),
            json!({}),
        ),
        (
            Method::DELETE,
            format!("/api/v1/admin/bots/{BENDER}/grants/1"),
            json!({}),
        ),
        (
            Method::PUT,
            "/api/v1/settings/github_connection".into(),
            json!({"accessToken":"secret"}),
        ),
        (
            Method::DELETE,
            "/api/v1/settings/github_connection".into(),
            json!({}),
        ),
        (
            Method::PUT,
            "/api/v1/settings/fizzy_connection".into(),
            json!({"accessToken":"secret"}),
        ),
        (
            Method::DELETE,
            "/api/v1/settings/fizzy_connection".into(),
            json!({}),
        ),
        (
            Method::DELETE,
            "/api/v1/settings/google_connection".into(),
            json!({}),
        ),
        (Method::PUT, "/api/v1/admin/slack".into(), json!({})),
        (Method::DELETE, "/api/v1/admin/slack".into(), json!({})),
        (Method::DELETE, "/api/v1/slack/connection".into(), json!({})),
    ] {
        let mut browser = app.sign_in(DAVID).await;
        let reply = browser.write(request(method.clone(), &path, body)).await;
        assert_eq!(
            reply.status,
            StatusCode::FORBIDDEN,
            "{method} {path}: {}",
            reply.text()
        );
        let error = reply.json();
        assert_eq!(error["error"]["_tag"], "SudoRequired", "{method} {path}");
        assert_eq!(
            error["error"]["reauthentication"]["methods"],
            json!(["password", "totp"])
        );
        assert_eq!(
            error["error"]["reauthentication"]["retry"],
            json!({"method":method.as_str(),"path":path,"returnTo":"/app/"})
        );
        assert!(session(&app, &browser)["sudo_pending_request"]["params"].is_null());
    }
}
