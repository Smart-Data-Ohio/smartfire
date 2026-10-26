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

/// The browser's cookie session; empty before any response has set one.
fn session(app: &TestApp, browser: &Browser<'_>) -> Value {
    let cookies = browser.cookie_header();
    let Some(raw) = cookies
        .split("; ")
        .find_map(|cookie| cookie.strip_prefix("_campfire_session="))
    else {
        return json!({});
    };
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
        let factor = TwoFactorCredential::for_user(conn, DAVID)?.map(|factor| json!([factor.last_totp_at, factor.consecutive_failures, factor.lockout_count, factor.locked_until.map(|time| time.to_string())]));
        Ok(json!({"audits":audits,"factor":factor}))
    }).await.unwrap()
}

/// The retained form's POST, which now goes to the SPA's page before anything is read or checked.
async fn assert_retained_confirms_nothing(app: &TestApp, browser: &mut Browser<'_>, req: Req) {
    let before = facts(app).await;
    let reply = browser.send(req).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.text());
    assert_eq!(reply.location(), Some("http://campfire.test/app/sudo/new"));
    assert!(session(app, browser).get("sudo_verified_at").is_none());
    assert_eq!(facts(app).await, before);
}

#[tokio::test]
async fn sudo_review_body_token_is_accepted_and_the_retained_form_confirms_nothing() {
    let app = TestApp::boot_frozen().await.unwrap();
    let mut browser = app.sign_in(DAVID).await;
    let token = browser.authenticity_token().await;
    let before = facts(&app).await;
    let req = request(
        Method::POST,
        "/api/v1/sudo",
        json!({"kind":"password","password":"secret123456","authenticity_token":token}),
    );
    for origin in ["https://attacker.test", "null"] {
        let reply = browser.send(req.clone().header("origin", origin)).await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(facts(&app).await, before);
        let retained = Req::new(Method::POST, "/sudo")
            .form(&[("password", "secret123456"), ("authenticity_token", &token)])
            .header("origin", origin);
        assert_retained_confirms_nothing(&app, &mut browser, retained).await;
    }
    let retained = Req::new(Method::POST, "/sudo")
        .form(&[("password", "secret123456"), ("authenticity_token", &token)]);
    assert_retained_confirms_nothing(&app, &mut browser, retained).await;
    let reply = browser.send(req).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert_eq!(
        session(&app, &browser)["sudo_verified_at"],
        app.booted.app.clock.now().as_second()
    );
    let audits = facts(&app).await["audits"].clone();
    assert_eq!(audits.as_array().unwrap().len(), 1);
    assert_eq!(audits[0][0], "sudo.confirm.success");
    assert_eq!(audits[0][2], "{\"verifier\":\"password\"}");
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
            if !json_mode && method != Method::GET {
                // The retained forms go to the SPA's page before any before-action runs.
                assert_eq!(reply.status, StatusCode::SEE_OTHER);
                assert_eq!(reply.location(), Some("http://campfire.test/app/sudo/new"));
                assert_eq!(
                    app.db()
                        .read(|conn| Ok(conn.query_row(
                            "SELECT count(*) FROM sessions WHERE user_id=?",
                            [DAVID],
                            |r| r.get::<_, i64>(0)
                        )?))
                        .await
                        .unwrap(),
                    sessions_before
                );
                observed.push(facts(&app).await);
                continue;
            }
            let location = if json_mode {
                assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
                assert_eq!(reply.header("cache-control"), Some("no-store"));
                let body: campfire_api_types::SudoResponse =
                    serde_json::from_slice(&reply.body).expect("sudo callback contract");
                let campfire_api_types::SudoResponse::Navigate { location, .. } = body else {
                    panic!("sign-in navigation required")
                };
                location
            } else {
                assert_eq!(reply.status, StatusCode::FOUND);
                reply.location().unwrap().to_owned()
            };
            assert_eq!(location, "http://campfire.test/app/session/new");
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
        let app = TestApp::boot_frozen().await.unwrap();
        let mut browser = app.sign_in(DAVID).await;
        let encryption = ArEncryption::new(&app.booted.app.secrets);
        let secret = app.db().write(move |tx| {
            let factor = TwoFactorCredential::for_user(tx.conn(), DAVID)?.unwrap();
            tx.conn().execute("UPDATE two_factor_credentials SET last_totp_at=NULL,consecutive_failures=0,lockout_count=0,locked_until=NULL WHERE id=?", [factor.id])?;
            factor.secret(&encryption)
        }).await.unwrap();
        let code = rails_compat::totp::at(&secret, app.booted.app.clock.now().as_second()).unwrap();
        let key = if method == "password" {
            "password"
        } else {
            "totp_code"
        };
        for value in [
            "wrong",
            if method == "password" {
                "secret123456"
            } else {
                &code
            },
        ] {
            let retained = Req::new(Method::POST, "/sudo")
                .form(&[("verifier", method), (key, value)])
                .header(
                    campfire_kit::csrf::HEADER,
                    &browser.authenticity_token().await,
                );
            assert_retained_confirms_nothing(&app, &mut browser, retained).await;
            let req = request(
                Method::POST,
                "/api/v1/sudo",
                if method == "password" {
                    json!({"kind":"password","password":value})
                } else {
                    json!({"kind":"totp","code":value})
                },
            );
            let reply = browser.write(req).await;
            if value == "wrong" {
                assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{}", reply.text());
                assert!(reply.text().contains("Confirmation failed. Try again."));
                assert!(session(&app, &browser).get("sudo_verified_at").is_none());
            } else {
                assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
                assert_eq!(
                    session(&app, &browser)["sudo_verified_at"],
                    app.booted.app.clock.now().as_second()
                );
            }
        }
        let facts = facts(&app).await;
        let audits = facts["audits"].as_array().unwrap();
        assert_eq!(
            audits
                .iter()
                .map(|audit| (audit[0].clone(), audit[2].clone()))
                .collect::<Vec<_>>(),
            [
                (
                    json!("sudo.confirm.failure"),
                    json!(format!("{{\"verifier\":\"{method}\"}}"))
                ),
                (
                    json!("sudo.confirm.success"),
                    json!(format!("{{\"verifier\":\"{method}\"}}"))
                ),
            ],
            "{method}"
        );
    }
}

#[tokio::test]
async fn sudo_contract_ten_attempts_are_shared_across_json_and_google_and_the_retained_form_counts_none()
 {
    let app = TestApp::boot_frozen().await.unwrap();
    let mut browser = app.sign_in(DAVID).await;
    let token = browser.authenticity_token().await;
    for _ in 0..3 {
        let retained = Req::new(Method::POST, "/sudo")
            .form(&[("password", "wrong")])
            .header(campfire_kit::csrf::HEADER, &token);
        assert_retained_confirms_nothing(&app, &mut browser, retained).await;
    }
    for attempt in 0..10 {
        let reply = if attempt % 2 == 0 {
            browser
                .write(request(
                    Method::POST,
                    "/api/v1/sudo",
                    json!({"kind":"password","password":"wrong"}),
                ))
                .await
        } else {
            browser
                .write(request(Method::POST, "/api/v1/sudo/google", json!({})))
                .await
        };
        assert_eq!(
            reply.status,
            if attempt % 2 == 1 {
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
    let retained = Req::new(Method::POST, "/sudo")
        .form(&[("password", "secret123456")])
        .header(campfire_kit::csrf::HEADER, &token);
    assert_retained_confirms_nothing(&app, &mut browser, retained).await;
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
async fn sudo_contract_totp_lockout_and_recovery_code_refusal() {
    let app = TestApp::boot_frozen().await.unwrap();
    let mut browser = app.sign_in(DAVID).await;
    let encryption = ArEncryption::new(&app.booted.app.secrets);
    let (secret, recovery) = app.db().write(move |tx| {
        let factor = TwoFactorCredential::for_user(tx.conn(), DAVID)?.unwrap();
        tx.conn().execute("UPDATE two_factor_credentials SET last_totp_at=NULL,consecutive_failures=0,lockout_count=0,locked_until=NULL WHERE id=?", [factor.id])?;
        Ok((factor.secret(&encryption)?, campfire_db::TwoFactorBackupCode::regenerate_set(tx, factor.id)?))
    }).await.unwrap();
    let code = rails_compat::totp::at(&secret, app.booted.app.clock.now().as_second()).unwrap();
    let retained =
        Req::new(Method::POST, "/sudo").form(&[("verifier", "totp"), ("totp_code", &recovery[0])]);
    assert_retained_confirms_nothing(&app, &mut browser, retained).await;
    for value in [&recovery[0], "wrong", "wrong", "wrong", "wrong", &code] {
        let reply = browser
            .write(request(
                Method::POST,
                "/api/v1/sudo",
                json!({"kind":"totp","code":value}),
            ))
            .await;
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
    let audits = facts(&app).await["audits"].clone();
    assert_eq!(audits.as_array().unwrap().len(), 6);
    assert!(
        audits
            .as_array()
            .unwrap()
            .iter()
            .all(|audit| audit[0] == "sudo.confirm.failure")
    );
}

#[tokio::test]
async fn sudo_contract_methods_unavailable_totp_and_authentication_csrf() {
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
        json!({"kind":"ready","reauthentication":{"methods":["password"],"retry":null},"session":ready.json()["session"].as_i64().unwrap()})
    );
    assert_eq!(ready.header("cache-control"), Some("no-store"));
    let retained =
        Req::new(Method::POST, "/sudo").form(&[("verifier", "totp"), ("totp_code", "123456")]);
    assert_retained_confirms_nothing(&app, &mut browser, retained).await;
    let reply = browser
        .write(request(
            Method::POST,
            "/api/v1/sudo",
            json!({"kind":"totp","code":"123456"}),
        ))
        .await;
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
        assert_eq!(
            reply.json()["location"],
            "http://campfire.test/app/session/new"
        );
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
                // A form goes to the SPA's confirmation; the CSV download, not a page, to the
                // retained route.
                let page = if path.ends_with(".csv") {
                    "/sudo/new"
                } else {
                    "/app/sudo/new"
                };
                assert_eq!(
                    reply.location(),
                    Some(format!("http://campfire.test{page}").as_str())
                );
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

/// Rewrites `browser`'s cookie session with `change`, as the browser would carry it.
fn edit_session(app: &TestApp, browser: &mut Browser<'_>, change: impl FnOnce(&mut Value)) {
    let mut state = session(app, browser);
    change(&mut state);
    let crypto = RailsCrypto::new(app.booted.app.secrets.clone());
    browser.absorb_cookie_header(&format!(
        "_campfire_session={}",
        campfire_kit::cookies::escape(&crypto.encrypt_cookie("_campfire_session", &state, None))
    ));
}

const GUARDED: &str = "/api/v1/admin/workspace/join_code";

/// A guarded write the gate holds, sent from the SPA's admin page.
fn guarded() -> Req {
    request(Method::POST, GUARDED, json!({}))
        .header("referer", "http://campfire.test/app/admin?tab=workspace")
}

#[tokio::test]
async fn sudo_contract_status_reports_a_fresh_confirmation_and_only_peeks_at_the_held_request() {
    let app = TestApp::boot_frozen().await.unwrap();
    let mut browser = app.sign_in(DAVID).await;
    let ready = browser.get("/api/v1/sudo").await;
    let session_id = ready.json()["session"].as_i64().unwrap();
    assert_eq!(ready.status, StatusCode::OK);
    assert_eq!(
        ready.json(),
        json!({"kind":"ready","reauthentication":{"methods":["password","totp"],"retry":null},"session":session_id})
    );
    assert_eq!(browser.write(guarded()).await.status, StatusCode::FORBIDDEN);
    let retry = json!({"method":"POST","path":GUARDED,"returnTo":"/app/admin"});
    // Not fresh: `ready`, naming what is held, as often as it's asked.
    for _ in 0..2 {
        let ready = browser.get("/api/v1/sudo").await;
        assert_eq!(ready.status, StatusCode::OK);
        assert_eq!(
            ready.json(),
            json!({"kind":"ready","reauthentication":{"methods":["password","totp"],"retry":retry},"session":session_id})
        );
    }
    // Fresh, with a Google attempt in flight: `confirmed`, and nothing it reads is let go.
    browser.grant_sudo().await;
    edit_session(&app, &mut browser, |state| {
        state["sudo_google_attempt"] = json!({"id":"attempt-in-flight-0001","user_id":DAVID,"session_id":session_id,"at":app.booted.app.clock.now().as_second(),"retry":null});
        state["google_sign_in_request"] =
            json!({"purpose":"sudo","google_json":true,"session_id":session_id});
    });
    let before = session(&app, &browser);
    for _ in 0..2 {
        let fresh = browser.get("/api/v1/sudo").await;
        assert_eq!(fresh.status, StatusCode::OK, "{}", fresh.text());
        assert_eq!(fresh.header("cache-control"), Some("no-store"));
        assert_eq!(
            fresh.json(),
            json!({"kind":"confirmed","retry":retry,"session":session_id})
        );
        assert_eq!(session(&app, &browser), before);
    }
    // The continuation still hands it over, once.
    let continued = browser.get("/api/v1/sudo/continue").await;
    assert_eq!(continued.status, StatusCode::OK, "{}", continued.text());
    assert_eq!(continued.json()["kind"], "confirmed");
    assert_eq!(continued.json()["retry"], retry);
    assert_eq!(
        browser.get("/api/v1/sudo").await.json(),
        json!({"kind":"confirmed","retry":null,"session":session_id})
    );
    assert_eq!(
        session(&app, &browser)["sudo_google_attempt"],
        before["sudo_google_attempt"]
    );
}

#[tokio::test]
async fn sudo_contract_another_sessions_held_request_is_never_named() {
    let app = TestApp::boot_frozen().await.unwrap();
    let mut first = app.sign_in(DAVID).await;
    assert_eq!(first.write(guarded()).await.status, StatusCode::FORBIDDEN);
    // Another signed-in session carries the first one's cookie session, held request and all.
    let mut second = app.sign_in(DAVID).await;
    let carried = first
        .cookie_header()
        .split("; ")
        .find(|cookie| cookie.starts_with("_campfire_session="))
        .unwrap()
        .to_owned();
    second.absorb_cookie_header(&carried);
    assert!(session(&app, &second).get("sudo_pending_request").is_some());
    let ready = second.get("/api/v1/sudo").await;
    assert_eq!(ready.json()["kind"], "ready");
    assert_eq!(ready.json()["reauthentication"]["retry"], Value::Null);
    second.grant_sudo().await;
    assert_eq!(
        second.get("/api/v1/sudo").await.json()["retry"],
        Value::Null
    );
    assert_eq!(
        second.get("/api/v1/sudo/continue").await.json()["retry"],
        Value::Null
    );
    // Its own session still names it.
    assert_eq!(
        first.get("/api/v1/sudo").await.json()["reauthentication"]["retry"]["path"],
        GUARDED
    );
}

#[tokio::test]
async fn sudo_contract_status_signed_out_asks_to_sign_in() {
    // No session at all: the same sign-in hand-off every sudo JSON read gives.
    let app = TestApp::boot_frozen().await.unwrap();
    for path in ["/api/v1/sudo", "/api/v1/sudo/continue"] {
        let reply = app.anonymous().get(path).await;
        assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.text());
        assert_eq!(
            reply.json(),
            json!({"kind":"navigate","location":"http://campfire.test/app/session/new"}),
            "{path}"
        );
    }

    // A session that no longer counts as signed in gets the usual 401.
    let mut browser = app.sign_in(DAVID).await;
    browser.authenticity_token().await;
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
    let reply = browser
        .write(request(Method::GET, "/api/v1/sudo", json!({})))
        .await;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{}", reply.text());
    assert_eq!(reply.header("cache-control"), Some("no-store"));
    assert_eq!(
        reply.json(),
        json!({"kind":"navigate","location":"http://campfire.test/app/session/new"})
    );
}
