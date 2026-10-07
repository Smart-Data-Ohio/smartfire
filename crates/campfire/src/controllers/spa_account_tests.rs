//! The classic profile's account panels and writes, through both transports on one frozen seed.
//! Whole database, audit rows and cable publications use the S7 integration parity comparison.

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use campfire_db::{
    Session, TwoFactorBackupCode, TwoFactorCredential, TwoFactorRememberedDevice, User,
};
use campfire_kit::Crypto;
use rails_compat::{ar_encryption::ArEncryption, totp};
use serde_json::{Value, json};

use super::admin_tests::{
    PASSWORD, app, assert_parity_with_app, audits, dump, error, get, json_body, parse, settle,
    write,
};
use crate::controllers::presenters::{
    accounts,
    test_support::{BENDER, BENDER_KEY, Browser, DAVID, JASON, Reply, Req, TestApp},
};

const REFUSED: &str = "Enter your authenticator code or password to continue.";
const GOOGLE_REFUSED: &str = "Enter your authenticator code or confirm with Google to continue.";
const RATE: &str = "Too many attempts. Try again in a few minutes.";
const DISABLED: &str = "Two-step sign-in is off. Set it up again to keep signing in.";
const FORGOTTEN: &str = "Device forgotten. It will ask for a code at next sign-in.";
const ALL_FORGOTTEN: &str =
    "All devices forgotten. Every browser will ask for a code at next sign-in.";

async fn stopped_app() -> Option<TestApp> {
    Some(app().await?.without_job_runner().await)
}

fn session_state(a: &TestApp, b: &Browser<'_>) -> Value {
    let key = campfire_kit::session::SESSION_KEY;
    let cookies = b.cookie_header();
    let cookie = campfire_kit::cookies::parse_cookie_header(&cookies)
        .into_iter()
        .find(|(name, _)| name == key)
        .unwrap()
        .1;
    campfire_kit::RailsCrypto::new(a.booted.app.secrets.clone())
        .decrypt_cookie(key, &cookie, a.booted.app.clock.now())
        .unwrap()
}

/// The same completed-Google session flag used by the classic challenge suite.
fn google_confirm(a: &TestApp, b: &mut Browser<'_>, expired: bool) {
    let key = campfire_kit::session::SESSION_KEY;
    let crypto = campfire_kit::RailsCrypto::new(a.booted.app.secrets.clone());
    let mut state = session_state(a, b);
    state[crate::concerns::session_keys::REAUTH_KEY] =
        json!(a.booted.app.clock.now().as_second() - if expired { 601 } else { 0 });
    b.absorb_cookie_header(&format!(
        "{key}={}",
        campfire_kit::cookies::escape(&crypto.encrypt_cookie(key, &state, None))
    ));
}

async fn prepare(a: &TestApp, b: &mut Browser<'_>, case: &str) -> Value {
    let encryption = ArEncryption::new(&a.booted.app.secrets);
    let now = a.booted.app.clock.now();
    let mut context = a
        .db()
        .write(move |tx| {
            tx.conn().execute_batch(
                "DELETE FROM audit_logs; DELETE FROM background_jobs;
            DELETE FROM two_factor_remembered_devices;",
            )?;
            let credential = TwoFactorCredential::for_user(tx.conn(), DAVID)?.unwrap();
            tx.conn().execute(
                "UPDATE two_factor_credentials SET last_totp_at=NULL WHERE user_id=?",
                [DAVID],
            )?;
            tx.conn().execute(
                "DELETE FROM two_factor_backup_codes WHERE two_factor_credential_id=?",
                [credential.id],
            )?;
            let mut old = Vec::new();
            for n in 0..10 {
                let digest = TwoFactorBackupCode::digest(&format!("old-backup-{n}"));
                TwoFactorBackupCode::create(tx, credential.id, &digest)?;
                old.push(digest);
            }
            let own = TwoFactorRememberedDevice::create_for(
                tx,
                DAVID,
                Some("Remembered Firefox"),
                Some("203.0.113.4"),
            )?
            .0;
            let other = TwoFactorRememberedDevice::create_for(
                tx,
                JASON,
                Some("Someone else's browser"),
                None,
            )?
            .0;
            TwoFactorRememberedDevice::create_for(tx, DAVID, Some(" \t"), None)?;
            let expired =
                TwoFactorRememberedDevice::create_for(tx, DAVID, Some("Expired browser"), None)?.0;
            tx.conn().execute(
                "UPDATE two_factor_remembered_devices SET expires_at=? WHERE id=?",
                rusqlite::params![tx.now(), expired.id],
            )?;
            let code = totp::at(&credential.secret(&encryption)?, now.as_second()).unwrap();
            Ok(json!({"own": own.id, "other": other.id, "code": code, "old": old}))
        })
        .await
        .unwrap();
    if case == "not_enabled" {
        a.db()
            .write(|tx| User::find(tx.conn(), DAVID)?.reset_two_factor(tx))
            .await
            .unwrap();
    }
    if matches!(
        case,
        "no_password"
            | "google"
            | "google_wrong_then_empty"
            | "google_whitespace"
            | "google_expired"
    ) {
        a.db()
            .write(|tx| {
                tx.conn()
                    .execute("UPDATE users SET password_digest=NULL WHERE id=?", [DAVID])?;
                Ok(())
            })
            .await
            .unwrap();
    }
    if case.starts_with("google") {
        google_confirm(a, b, case == "google_expired");
    }
    b.absorb_cookie_header("two_factor_remember=remembered-before-disable");
    context["session"] = session_state(a, b);
    if case == "session_refresh" {
        a.db()
            .write(|tx| {
                tx.conn().execute(
                    "UPDATE sessions SET last_active_at=? WHERE user_id=?",
                    rusqlite::params![tx.now().ago(jiff::SignedDuration::from_hours(2)), DAVID],
                )?;
                Ok(())
            })
            .await
            .unwrap();
    }
    context
}

fn confirmation<'a>(case: &str, context: &'a Value) -> &'a str {
    match case {
        "code" | "code_reused" => context["code"].as_str().unwrap(),
        "blank" | "google" | "google_expired" => "",
        "whitespace" | "google_whitespace" => " \t\n ",
        "wrong" | "no_password" => "wrong",
        "backup" => "old-backup-0",
        _ => PASSWORD,
    }
}

fn classic_codes(reply: &Reply) -> Vec<String> {
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert_eq!(reply.header("cache-control"), Some("no-store"));
    assert_eq!(reply.header("pragma"), Some("no-cache"));
    regex::Regex::new(r#"<li><code class="txt-large">([^<]+)</code></li>"#)
        .unwrap()
        .captures_iter(&reply.text())
        .map(|capture| capture[1].to_owned())
        .collect()
}

async fn check_codes(a: &TestApp, codes: &[String], old: &Value) {
    assert_eq!(codes.len(), 10);
    assert_eq!(
        codes
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        10
    );
    for code in codes {
        assert_eq!(code.len(), 10);
        assert!(
            code.bytes()
                .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
        );
    }
    let expected: std::collections::BTreeSet<_> = codes
        .iter()
        .map(|code| TwoFactorBackupCode::digest(code))
        .collect();
    let stored = a
        .db()
        .read(|conn| {
            let credential = TwoFactorCredential::for_user(conn, DAVID)?.unwrap();
            TwoFactorBackupCode::for_credential(conn, credential.id)
        })
        .await
        .unwrap();
    assert_eq!(stored.len(), 10);
    let digests: std::collections::BTreeSet<_> =
        stored.iter().map(|row| row.code_digest.clone()).collect();
    assert_eq!(
        digests, expected,
        "the once-shown codes must be the stored set"
    );
    for row in stored {
        assert!(row.used_at.is_none());
        assert_eq!(row.code_digest.len(), 64);
        assert!(row.code_digest.bytes().all(|ch| ch.is_ascii_hexdigit()));
        assert!(
            !old.as_array().unwrap().contains(&json!(row.code_digest)),
            "the old set was replaced"
        );
    }
}

fn classic_alert(b: &Browser<'_>, reply: &Reply, alert: &str) {
    assert_eq!(reply.status, StatusCode::FOUND, "{}", reply.text());
    assert_eq!(
        reply.location(),
        Some("http://campfire.test/users/me/profile")
    );
    assert_eq!(b.flash(), json!({"alert": alert}));
}

fn api_alert(reply: &Reply, tag: &str, status: StatusCode, message: &str) {
    assert_eq!(reply.status, status, "{}", reply.text());
    assert_eq!(
        error(reply),
        match tag {
            "Validation" => json!({"_tag": tag, "message": message, "fields": {}}),
            "RateLimited" => json!({"_tag": tag, "message": message, "retryAfter": 0}),
            _ => json!({"_tag": tag, "message": message}),
        }
    );
}

#[tokio::test]
async fn backup_codes_match_accepted_refused_not_enabled_rate_and_google_branches() {
    for case in [
        "code",
        "password",
        "blank",
        "whitespace",
        "wrong",
        "backup",
        "no_password",
        "not_enabled",
        "rate",
        "google",
        "google_wrong_then_empty",
        "google_whitespace",
        "google_expired",
        "code_reused",
    ] {
        let classic_path = "/two_factor_backup_codes";
        let api_path = "/api/v1/settings/two_factor/backup_codes";
        let accepted = matches!(
            case,
            "code"
                | "password"
                | "google"
                | "google_wrong_then_empty"
                | "google_whitespace"
                | "code_reused"
        );
        let refused = if matches!(case, "no_password" | "google_expired") {
            GOOGLE_REFUSED
        } else {
            REFUSED
        };
        let Some(outcome) = assert_parity_with_app(
            stopped_app,
            async |a, b| {
                let context = prepare(a, b, case).await;
                if case == "rate" {
                    for _ in 0..10 {
                        let reply = b
                            .write(
                                Req::new(Method::POST, classic_path).form(&[("reauth", "wrong")]),
                            )
                            .await;
                        classic_alert(b, &reply, REFUSED);
                    }
                }
                context
            },
            async |b, context| {
                if case == "google_wrong_then_empty" {
                    let wrong = "wrong";
                    let reply = b
                        .write(Req::new(Method::POST, classic_path).form(&[("reauth", wrong)]))
                        .await;
                    classic_alert(b, &reply, GOOGLE_REFUSED);
                }
                let reauth = if case.ends_with("then_empty") {
                    ""
                } else {
                    confirmation(case, &context)
                };
                let reply = b
                    .write(Req::new(Method::POST, classic_path).form(&[("reauth", reauth)]))
                    .await;
                if accepted {
                    check_codes(b.app(), &classic_codes(&reply), &context["old"]).await;
                    if case.starts_with("google") || case == "code_reused" {
                        let reply = b
                            .write(Req::new(Method::POST, classic_path).form(&[("reauth", reauth)]))
                            .await;
                        classic_alert(
                            b,
                            &reply,
                            if case.starts_with("google") {
                                GOOGLE_REFUSED
                            } else {
                                REFUSED
                            },
                        );
                    }
                } else if case == "not_enabled" {
                    assert_eq!(
                        reply.location(),
                        Some("http://campfire.test/two_factor_setup")
                    );
                } else {
                    classic_alert(b, &reply, if case == "rate" { RATE } else { refused });
                }
            },
            async |b, context| {
                if case == "google_wrong_then_empty" {
                    let wrong = "wrong";
                    let reply = write(b, Method::POST, api_path, json!({"reauth": wrong})).await;
                    api_alert(
                        &reply,
                        "Validation",
                        StatusCode::UNPROCESSABLE_ENTITY,
                        GOOGLE_REFUSED,
                    );
                }
                let reauth = if case.ends_with("then_empty") {
                    ""
                } else {
                    confirmation(case, &context)
                };
                let reply = write(b, Method::POST, api_path, json!({"reauth": reauth})).await;
                if case != "rate" {
                    assert_eq!(reply.header("cache-control"), Some("no-store"));
                    assert_eq!(reply.header("pragma"), Some("no-cache"));
                }
                if accepted {
                    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
                    let codes: api::BackupCodes = parse(&reply);
                    assert_eq!(reply.json(), json!({"codes": codes.codes}));
                    check_codes(b.app(), &codes.codes, &context["old"]).await;
                    if case.starts_with("google") || case == "code_reused" {
                        let reply =
                            write(b, Method::POST, api_path, json!({"reauth": reauth})).await;
                        api_alert(
                            &reply,
                            "Validation",
                            StatusCode::UNPROCESSABLE_ENTITY,
                            if case.starts_with("google") {
                                GOOGLE_REFUSED
                            } else {
                                REFUSED
                            },
                        );
                    }
                } else if case == "not_enabled" {
                    api_alert(
                        &reply,
                        "Conflict",
                        StatusCode::CONFLICT,
                        "Set up two-step sign-in first.",
                    );
                } else if case == "rate" {
                    api_alert(&reply, "RateLimited", StatusCode::TOO_MANY_REQUESTS, RATE);
                } else {
                    api_alert(
                        &reply,
                        "Validation",
                        StatusCode::UNPROCESSABLE_ENTITY,
                        refused,
                    );
                }
            },
        )
        .await
        else {
            return;
        };
        assert_eq!(
            audits(&outcome).contains("two_factor.backup_codes.regenerate"),
            accepted,
            "{case}"
        );
        if !accepted {
            assert_eq!(
                outcome.rows, outcome.before,
                "{case}: rejected change leaves all rows alone"
            );
            assert!(outcome.frames.is_empty());
        } else {
            assert!(
                !outcome.frames.is_empty(),
                "accepted regeneration resets remote connections"
            );
        }
    }
}

fn wire_time(at: jiff::Timestamp) -> String {
    at.strftime("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

async fn check_panel(a: &TestApp, user_id: i64, panel: &api::TwoFactorSettings) {
    let now = a.booted.app.clock.now();
    let google = a.booted.app.two_factor.google().is_some();
    let (user, data) = a
        .db()
        .read(move |conn| {
            Ok((
                User::find(conn, user_id)?,
                accounts::profile_two_factor(conn, user_id, now, google)?,
            ))
        })
        .await
        .unwrap();
    assert_eq!(panel.confirmed_at, data.confirmed_at.map(wire_time));
    assert_eq!(panel.google, data.google);
    assert_eq!(panel.has_password, accounts::profile_has_password(&user));
    assert_eq!(panel.devices.len(), data.devices.len());
    for (actual, expected) in panel.devices.iter().zip(data.devices) {
        assert_eq!(actual.id, expected.id);
        assert_eq!(
            actual.description,
            campfire_views::two_factor::device_description(&expected)
        );
        assert_eq!(actual.ip_address, expected.ip_address);
        assert_eq!(actual.last_used_at, expected.last_used_at.map(wire_time));
    }
}

/// Cookie attributes and authenticated identity match; flash is intentionally only classic.
/// The encrypted session/CSRF payload and per-run authentication token are random.
fn cookies(a: &TestApp, b: &Browser<'_>, reply: &Reply) -> Vec<String> {
    let crypto = campfire_kit::RailsCrypto::new(a.booted.app.secrets.clone());
    let mut out: Vec<_> = reply
        .headers
        .get_all("set-cookie")
        .iter()
        .filter(|header| !header.to_str().unwrap().starts_with("_campfire_session="))
        .map(|header| {
            let header = header.to_str().unwrap();
            let (pair, attributes) = header.split_once(';').unwrap();
            let (name, value) = pair.split_once('=').unwrap();
            let payload: String = if name == "session_token" && !value.is_empty() {
                let token = crypto
                    .verify_signed_cookie(
                        name,
                        &rails_compat::cookies::unescape(value),
                        a.booted.app.clock.now(),
                    )
                    .unwrap();
                assert!(!token.is_empty());
                "<authentication token>".into()
            } else {
                value.into()
            };
            format!("{name}={payload};{attributes}")
        })
        .collect();
    let mut state = session_state(a, b);
    state.as_object_mut().unwrap().remove("flash");
    for (key, marker) in [("_csrf_token", "<csrf>"), ("session_id", "<session id>")] {
        assert!(state[key].is_string());
        state[key] = json!(marker);
    }
    out.push(format!("effective cookie session={state}"));
    out.sort();
    out
}

#[tokio::test]
async fn disable_matches_notice_refusals_conflict_rate_and_response_cookies() {
    for case in [
        "password",
        "session_refresh",
        "code",
        "blank",
        "wrong",
        "no_password",
        "not_enabled",
        "rate",
        "google",
    ] {
        let accepted = matches!(case, "password" | "session_refresh" | "code" | "google");
        let expected_cookies = std::sync::Mutex::new(None);
        let Some(outcome) = assert_parity_with_app(
            stopped_app,
            async |a, b| {
                let context = prepare(a, b, case).await;
                if case == "rate" {
                    for _ in 0..10 {
                        b.write(
                            Req::new(Method::DELETE, "/two_factor_setup")
                                .form(&[("reauth", "wrong")]),
                        )
                        .await;
                    }
                }
                context
            },
            async |b, context| {
                let before_token = authentication_token(b.app(), b);
                let reply = b
                    .write(
                        Req::new(Method::DELETE, "/two_factor_setup")
                            .form(&[("reauth", confirmation(case, &context))]),
                    )
                    .await;
                if accepted {
                    if case == "session_refresh" {
                        assert!(reply.headers.get_all("set-cookie").iter().any(|value| {
                            value.to_str().unwrap().starts_with("session_token=")
                        }));
                    }
                    assert_eq!(reply.status, StatusCode::FOUND);
                    assert_eq!(
                        reply.location(),
                        Some("http://campfire.test/two_factor_setup")
                    );
                    assert_eq!(b.flash(), json!({"notice": DISABLED}));
                    *expected_cookies.lock().unwrap() = Some(cookies(b.app(), b, &reply));
                    assert_disabled(b, &reply, &before_token, &context).await;
                } else if case == "not_enabled" {
                    assert_eq!(
                        reply.location(),
                        Some("http://campfire.test/two_factor_setup")
                    );
                } else {
                    classic_alert(
                        b,
                        &reply,
                        if case == "rate" {
                            RATE
                        } else if case == "no_password" {
                            GOOGLE_REFUSED
                        } else {
                            REFUSED
                        },
                    );
                }
            },
            async |b, context| {
                let before_token = authentication_token(b.app(), b);
                let reply = write(
                    b,
                    Method::DELETE,
                    "/api/v1/settings/two_factor",
                    json!({"reauth": confirmation(case, &context)}),
                )
                .await;
                if accepted {
                    if case == "session_refresh" {
                        assert!(reply.headers.get_all("set-cookie").iter().any(|value| {
                            value.to_str().unwrap().starts_with("session_token=")
                        }));
                    }
                    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
                    let change: api::TwoFactorChange = parse(&reply);
                    assert_eq!(change.notice, DISABLED);
                    assert!(change.two_factor.confirmed_at.is_none());
                    assert!(change.two_factor.devices.is_empty());
                    check_panel(b.app(), DAVID, &change.two_factor).await;
                    assert_eq!(
                        cookies(b.app(), b, &reply),
                        expected_cookies.lock().unwrap().clone().unwrap()
                    );
                    assert_disabled(b, &reply, &before_token, &context).await;
                } else if case == "not_enabled" {
                    api_alert(
                        &reply,
                        "Conflict",
                        StatusCode::CONFLICT,
                        "Set up two-step sign-in first.",
                    );
                } else if case == "rate" {
                    api_alert(&reply, "RateLimited", StatusCode::TOO_MANY_REQUESTS, RATE);
                } else {
                    api_alert(
                        &reply,
                        "Validation",
                        StatusCode::UNPROCESSABLE_ENTITY,
                        if case == "no_password" {
                            GOOGLE_REFUSED
                        } else {
                            REFUSED
                        },
                    );
                }
            },
        )
        .await
        else {
            return;
        };
        assert_eq!(
            audits(&outcome).contains("two_factor.disable"),
            accepted,
            "{case}"
        );
        if !accepted {
            assert_eq!(outcome.rows, outcome.before, "{case}");
            assert!(outcome.frames.is_empty());
        } else {
            assert!(!outcome.frames.is_empty());
        }
    }
}

fn authentication_token(a: &TestApp, b: &Browser<'_>) -> String {
    let token = campfire_kit::cookies::parse_cookie_header(&b.cookie_header())
        .into_iter()
        .find(|(name, _)| name == "session_token")
        .unwrap()
        .1;
    campfire_kit::RailsCrypto::new(a.booted.app.secrets.clone())
        .verify_signed_cookie("session_token", &token, a.booted.app.clock.now())
        .unwrap()
}

async fn assert_disabled(b: &mut Browser<'_>, reply: &Reply, before_token: &str, context: &Value) {
    let after = session_state(b.app(), b);
    for key in ["session_id", "_csrf_token"] {
        assert_eq!(after[key], context["session"][key], "disable retains {key}");
    }
    assert!(
        reply.headers.get_all("set-cookie").iter().any(|header| {
            header
                .to_str()
                .unwrap()
                .starts_with("two_factor_remember=;")
        }),
        "the remember cookie is deleted on this response"
    );
    assert!(!b.cookie_header().contains("two_factor_remember="));
    assert_eq!(
        authentication_token(b.app(), b),
        before_token,
        "disable retains the classic authentication token"
    );
    b.app()
        .db()
        .read(|conn| {
            assert!(TwoFactorCredential::for_user(conn, DAVID)?.is_none());
            for session in Session::for_user(conn, DAVID)? {
                assert!(!session.two_factor_verified());
            }
            Ok(())
        })
        .await
        .unwrap();
    // Classic enrollment enforcement permits setup after disabling. It intentionally gates
    // other human screens until enrollment; the authentication cookie still restores the user.
    let reply = b.send(get("/api/v1/settings/account")).await;
    api_alert(
        &reply,
        "TwoFactorRequired",
        StatusCode::FORBIDDEN,
        "Set up two-step sign-in to continue",
    );
}

#[tokio::test]
async fn forgetting_one_or_all_devices_matches_ownership_refusals_and_rate_limits() {
    for all in [false, true] {
        for case in [
            "password",
            "code",
            "blank",
            "wrong",
            "no_password",
            "other",
            "missing",
            "rate",
            "google",
            "not_enabled",
        ] {
            let notice = if all { ALL_FORGOTTEN } else { FORGOTTEN };
            let accepted = matches!(case, "password" | "code" | "other" | "missing" | "google");
            // The response cookies, the session they leave and the remember cookie the browser
            // still holds, from the classic side, for the API side to match.
            let expected_cookies = std::sync::Mutex::new(None);
            let path = |context: &Value, api: bool| {
                let prefix = if api {
                    "/api/v1/settings/two_factor/devices"
                } else {
                    "/two_factor_remembered_devices"
                };
                if all {
                    prefix.to_owned()
                } else {
                    let id = if case == "other" {
                        context["other"].as_i64().unwrap()
                    } else if case == "missing" {
                        -1
                    } else {
                        context["own"].as_i64().unwrap()
                    };
                    format!("{prefix}/{id}")
                }
            };
            let Some(outcome) = assert_parity_with_app(
                stopped_app,
                async |a, b| {
                    let context = prepare(a, b, case).await;
                    if case == "rate" {
                        for _ in 0..10 {
                            b.write(
                                Req::new(Method::DELETE, &path(&context, false))
                                    .form(&[("reauth", "wrong")]),
                            )
                            .await;
                        }
                    }
                    context
                },
                async |b, context| {
                    let reply = b
                        .write(
                            Req::new(Method::DELETE, &path(&context, false))
                                .form(&[("reauth", confirmation(case, &context))]),
                        )
                        .await;
                    if accepted {
                        assert_eq!(reply.status, StatusCode::FOUND);
                        assert_eq!(
                            reply.location(),
                            Some("http://campfire.test/users/me/profile")
                        );
                        assert_eq!(b.flash(), json!({"notice": notice}));
                        check_devices(b.app(), &context, case, all).await;
                    } else if case == "not_enabled" {
                        // Forgetting has no enrollment check in classic: with two-step sign-in
                        // off, the password still confirms it and the notice comes back.
                        assert_eq!(reply.status, StatusCode::FOUND);
                        assert_eq!(
                            reply.location(),
                            Some("http://campfire.test/users/me/profile")
                        );
                        assert_eq!(b.flash(), json!({"notice": notice}));
                    } else {
                        classic_alert(
                            b,
                            &reply,
                            if case == "rate" {
                                RATE
                            } else if case == "no_password" {
                                GOOGLE_REFUSED
                            } else {
                                REFUSED
                            },
                        );
                    }
                    *expected_cookies.lock().unwrap() =
                        Some((cookies(b.app(), b, &reply), remember_cookie(b)));
                },
                async |b, context| {
                    let reply = write(
                        b,
                        Method::DELETE,
                        &path(&context, true),
                        json!({"reauth": confirmation(case, &context)}),
                    )
                    .await;
                    if accepted {
                        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
                        let change: api::TwoFactorChange = parse(&reply);
                        assert_eq!(change.notice, notice);
                        assert!(change.two_factor.confirmed_at.is_some());
                        check_panel(b.app(), DAVID, &change.two_factor).await;
                        check_devices(b.app(), &context, case, all).await;
                    } else if case == "not_enabled" {
                        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
                        let change: api::TwoFactorChange = parse(&reply);
                        assert_eq!(change.notice, notice);
                        assert!(change.two_factor.confirmed_at.is_none());
                        assert!(change.two_factor.devices.is_empty());
                    } else {
                        api_alert(
                            &reply,
                            if case == "rate" {
                                "RateLimited"
                            } else {
                                "Validation"
                            },
                            if case == "rate" {
                                StatusCode::TOO_MANY_REQUESTS
                            } else {
                                StatusCode::UNPROCESSABLE_ENTITY
                            },
                            if case == "rate" {
                                RATE
                            } else if case == "no_password" {
                                GOOGLE_REFUSED
                            } else {
                                REFUSED
                            },
                        );
                    }
                    assert_eq!(
                        (cookies(b.app(), b, &reply), remember_cookie(b)),
                        expected_cookies.lock().unwrap().clone().unwrap(),
                        "{case} all={all}: response cookies, session and remember cookie"
                    );
                },
            )
            .await
            else {
                return;
            };
            // With two-step sign-in off, classic still confirms and forgets (and audits it).
            let confirmed = accepted || case == "not_enabled";
            assert_eq!(
                audits(&outcome).contains("two_factor.devices.revoke_all"),
                all && confirmed
            );
            if !confirmed || (!all && matches!(case, "other" | "missing")) {
                assert_eq!(outcome.rows, outcome.before, "{case} all={all}");
            }
            assert!(outcome.frames.is_empty());
        }
    }
}

/// The `two_factor_remember` cookie the browser holds after a reply, if any.
fn remember_cookie(b: &Browser<'_>) -> Option<String> {
    campfire_kit::cookies::parse_cookie_header(&b.cookie_header())
        .into_iter()
        .find(|(name, _)| name == "two_factor_remember")
        .map(|(_, value)| value)
}

async fn check_devices(a: &TestApp, context: &Value, case: &'static str, all: bool) {
    let own_id = context["own"].as_i64().unwrap();
    let other_id = context["other"].as_i64().unwrap();
    a.db()
        .read(move |conn| {
            let own = TwoFactorRememberedDevice::for_user(conn, DAVID)?;
            assert_eq!(
                own.len(),
                if all {
                    0
                } else if matches!(case, "other" | "missing") {
                    3
                } else {
                    2
                }
            );
            assert_eq!(
                own.iter().any(|device| device.id == own_id),
                !all && matches!(case, "other" | "missing")
            );
            let other = TwoFactorRememberedDevice::for_user(conn, JASON)?;
            assert_eq!(other.len(), 1);
            assert_eq!(other[0].id, other_id);
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn test_push_matches_owned_job_and_refuses_another_users_or_missing_subscription() {
    for owner in [DAVID, JASON, -1] {
        let Some(outcome) = assert_parity_with_app(
            stopped_app,
            async |a, _| {
                a.db()
                    .write(|tx| {
                        tx.conn().execute_batch(
                            "DELETE FROM background_jobs; DELETE FROM audit_logs;",
                        )?;
                        Ok(())
                    })
                    .await
                    .unwrap();
                if owner == -1 {
                    json!(-1)
                } else {
                    json!(
                        a.db()
                            .read(move |conn| Ok(campfire_db::PushSubscription::for_user(
                                conn, owner
                            )?[0]
                                .id))
                            .await
                            .unwrap()
                    )
                }
            },
            async |b, id| {
                let reply = b
                    .write(Req::new(
                        Method::POST,
                        &format!("/users/me/push_subscriptions/{id}/test_notifications"),
                    ))
                    .await;
                if owner == DAVID {
                    assert_eq!(reply.status, StatusCode::FOUND);
                    assert_eq!(
                        reply.location(),
                        Some("http://campfire.test/users/me/push_subscriptions")
                    );
                    check_job(b.app(), id.as_i64().unwrap()).await;
                } else {
                    assert_eq!(reply.status, StatusCode::NOT_FOUND);
                }
            },
            async |b, id| {
                let reply = b
                    .write(
                        Req::new(
                            Method::POST,
                            &format!("/api/v1/settings/push_subscriptions/{id}/test"),
                        )
                        .header("accept", "application/json"),
                    )
                    .await;
                if owner == DAVID {
                    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.text());
                    assert!(reply.body.is_empty());
                    check_job(b.app(), id.as_i64().unwrap()).await;
                } else {
                    api_alert(&reply, "NotFound", StatusCode::NOT_FOUND, "Not found");
                }
            },
        )
        .await
        else {
            return;
        };
        assert!(outcome.frames.is_empty());
        assert!(outcome.rows["audit_logs"].as_array().unwrap().is_empty());
        if owner != DAVID {
            assert_eq!(outcome.rows, outcome.before);
        }
    }
}

async fn check_job(a: &TestApp, id: i64) {
    a.db()
        .read(move |conn| {
            let jobs: Vec<(String, String)> = conn
                .prepare("SELECT job_class, arguments FROM background_jobs")?
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<rusqlite::Result<_>>()?;
            assert_eq!(jobs.len(), 1);
            assert_eq!(jobs[0].0, "Push::Subscription::TestNotificationJob");
            let args: campfire_db::models::push_subscription::TestNotificationJob =
                serde_json::from_str(&jobs[0].1).unwrap();
            assert_eq!(args.user_id, DAVID);
            assert_eq!(args.subscription_id, id);
            assert_eq!(
                args.path,
                "http://campfire.test/users/me/push_subscriptions"
            );
            assert_eq!(
                uuid::Uuid::parse_str(&args.body).unwrap().get_version_num(),
                4
            );
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn account_read_uses_classic_membership_order_device_rows_transfer_and_null_levels() {
    let Some(a) = stopped_app().await else { return };
    let mut b = a.sign_in(DAVID).await;
    b.grant_sudo().await;
    prepare(&a, &mut b, "password").await;
    // Include both dated and undated rows, whitespace agents, and an expired device.
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE two_factor_remembered_devices SET last_used_at=NULL WHERE user_agent=?",
                [" \t"],
            )?;
            // A membership with no level stored, as old rows have: classic labels it with nothing.
            tx.conn().execute(
                "UPDATE memberships SET involvement=NULL WHERE id=(SELECT m.id FROM memberships m
                 JOIN rooms r ON r.id=m.room_id WHERE m.user_id=? AND r.type!='Rooms::Direct'
                 ORDER BY m.id LIMIT 1)",
                [DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    {
        let user_id = DAVID;
        let mut browser = a.sign_in(user_id).await;
        browser.authenticity_token().await;
        let before = dump(&a).await;
        let reply = browser.send(get("/api/v1/settings/account")).await;
        assert_eq!(dump(&a).await, before, "reading the panel changes no rows");
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        assert_eq!(reply.header("cache-control"), Some("no-store"));
        assert_eq!(reply.header("pragma"), Some("no-cache"));
        let account: api::AccountSettings = parse(&reply);
        let (direct, shared) = a
            .db()
            .read(move |conn| accounts::profile_memberships(conn, &User::find(conn, user_id)?))
            .await
            .unwrap();
        for (actual, expected) in [
            (&account.shared_rooms, shared),
            (&account.direct_rooms, direct),
        ] {
            assert_eq!(actual.len(), expected.len());
            for (row, classic) in actual.iter().zip(expected) {
                assert_eq!(row.room_id, classic.room_id);
                assert_eq!(row.name, classic.room_display_name);
                // NULL (an empty classic label) stays null: it is not `mentions`.
                let classic_involvement = if classic.involvement.is_empty() {
                    Value::Null
                } else {
                    json!(classic.involvement)
                };
                assert_eq!(
                    serde_json::to_value(row.involvement).unwrap(),
                    classic_involvement
                );
                assert_eq!(row.direct, classic.direct);
            }
        }
        let unset = account
            .shared_rooms
            .iter()
            .find(|row| row.involvement.is_none())
            .expect("the NULL membership reads as no level")
            .room_id;
        // Choosing Mentions from no level is a real change: it is stored and reads back.
        let reply = browser
            .write(
                Req::new(Method::PUT, &format!("/rooms/{unset}/involvement.json"))
                    .form(&[("involvement", "mentions")]),
            )
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        let after: api::AccountSettings =
            parse(&browser.send(get("/api/v1/settings/account")).await);
        assert_eq!(
            after
                .shared_rooms
                .iter()
                .find(|row| row.room_id == unset)
                .unwrap()
                .involvement,
            Some(api::Involvement::Mentions)
        );
        {
            assert!(!account.shared_rooms.is_empty());
            assert!(!account.direct_rooms.is_empty());
            assert!(account.shared_rooms.iter().all(|row| !row.direct));
            assert!(account.direct_rooms.iter().all(|row| row.direct));
            let panel = &account.two_factor;
            check_panel(&a, user_id, panel).await;
            assert!(panel.confirmed_at.is_some());
            assert_eq!(panel.devices.len(), 2);
            assert!(
                panel
                    .devices
                    .iter()
                    .any(|device| device.description == "Unknown browser"
                        && device.last_used_at.is_none())
            );
            assert!(
                !panel
                    .devices
                    .iter()
                    .any(|device| device.description == "Expired browser")
            );
        }
        let transfer =
            accounts::transfer_id(&a.booted.app.secrets, user_id, a.booted.app.clock.now());
        assert_eq!(
            account.transfer_url,
            format!(
                "http://campfire.test{}",
                campfire_routes::session_transfer(&transfer)
            )
        );
        assert_eq!(
            accounts::user_id_from_transfer_id(
                &a.booted.app.secrets,
                &transfer,
                a.booted.app.clock.now()
            ),
            Some(user_id)
        );
        assert_eq!(
            Some(account.transfer_qr_svg),
            campfire_people::controllers::qr_code::transfer_svg(&account.transfer_url),
            "the QR code is the classic image of the same link, drawn in place"
        );
    }
}

fn actions() -> [(Method, &'static str); 6] {
    [
        (Method::GET, "/api/v1/settings/account"),
        (Method::POST, "/api/v1/settings/two_factor/backup_codes"),
        (Method::DELETE, "/api/v1/settings/two_factor"),
        (Method::DELETE, "/api/v1/settings/two_factor/devices/1"),
        (Method::DELETE, "/api/v1/settings/two_factor/devices"),
        (Method::POST, "/api/v1/settings/push_subscriptions/1/test"),
    ]
}

#[tokio::test]
async fn every_account_route_refuses_signed_out_bot_keys_and_warmed_agent_tokens() {
    use crate::controllers::agent_http_tests::{SECRET, initialize};
    let Some(a) = stopped_app().await else { return };
    initialize(&a).await;
    let authorization = format!("Bearer {SECRET}");
    let mut anonymous = a.anonymous();
    anonymous
        .send(get("/api/v1/me").header("authorization", &authorization))
        .await;
    let before = dump(&a).await;
    let capture = a.booted.app.cable.capture_every_publication();
    for (method, path) in actions() {
        for (kind, status, tag) in [
            ("anonymous", StatusCode::UNAUTHORIZED, "Unauthorized"),
            ("bot", StatusCode::FORBIDDEN, "Forbidden"),
            ("agent", StatusCode::FORBIDDEN, "Forbidden"),
        ] {
            let path = if kind == "bot" {
                format!("{path}?bot_key={BENDER_KEY}")
            } else {
                path.into()
            };
            let mut request = json_body(method.clone(), &path, &json!({"reauth": PASSWORD}));
            if kind == "agent" {
                request = request.header("authorization", &authorization);
            }
            let reply = anonymous.send(request).await;
            assert_eq!(reply.status, status, "{kind} {path}: {}", reply.text());
            assert_eq!(error(&reply)["_tag"], tag);
            assert_eq!(dump(&a).await, before, "{kind} {path}");
        }
    }
    assert!(settle(&capture).await.is_empty());
}

#[tokio::test]
async fn every_account_write_requires_csrf_and_typed_reauthentication() {
    let Some(a) = stopped_app().await else { return };
    let mut b = a.sign_in(DAVID).await;
    b.authenticity_token().await;
    let before = dump(&a).await;
    for (method, path) in actions()
        .into_iter()
        .filter(|(method, _)| *method != Method::GET)
    {
        let reply = b
            .send(json_body(
                method.clone(),
                path,
                &json!({"reauth": PASSWORD}),
            ))
            .await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{path}: {}",
            reply.text()
        );
        assert_eq!(error(&reply)["_tag"], "InvalidAuthenticityToken");
        if !path.ends_with("/test") {
            for body in [
                json!({}),
                json!({"reauth": 123456}),
                json!({"reauth": null}),
            ] {
                let reply = write(&mut b, method.clone(), path, body).await;
                assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{path}");
                assert_eq!(error(&reply)["_tag"], "Validation");
            }
        }
        assert_eq!(dump(&a).await, before);
    }
}

#[tokio::test]
async fn every_account_route_refuses_a_bot_session() {
    let Some(a) = stopped_app().await else { return };
    let mut b = a.sign_in(BENDER).await;
    let reply = b
        .write(Req::new(Method::DELETE, "/two_factor_setup").form(&[("reauth", PASSWORD)]))
        .await;
    assert_eq!(reply.location(), Some("http://campfire.test/"));
    let before = dump(&a).await;
    let capture = a.booted.app.cable.capture_every_publication();
    // No sign-in link, codes, device changes or test push for a bot's own session.
    for (method, path) in actions() {
        let reply = if method == Method::GET {
            b.send(get(path)).await
        } else {
            write(&mut b, method, path, json!({"reauth": PASSWORD})).await
        };
        api_alert(&reply, "Forbidden", StatusCode::FORBIDDEN, "Not allowed");
        assert!(!reply.text().contains("session/transfers"), "{path}");
        assert_eq!(dump(&a).await, before, "{path}");
    }
    assert!(settle(&capture).await.is_empty());
}

struct ConfiguredGoogle;
impl crate::state::two_factor::GoogleReauthentication for ConfiguredGoogle {
    fn configured(&self) -> bool {
        true
    }

    fn start(
        &self,
        _: &mut campfire_kit::Ctx,
        _: i64,
    ) -> campfire_kit::Result<campfire_kit::Response> {
        panic!("the account reader must never start OAuth")
    }
}

#[tokio::test]
async fn account_google_option_requires_both_configuration_and_the_viewers_link() {
    let Some(a) = stopped_app().await else { return };
    a.booted
        .app
        .two_factor
        .install_google(std::sync::Arc::new(ConfiguredGoogle));
    let mut b = a.sign_in(DAVID).await;
    for linked in [false, true] {
        a.db().write(move |tx| {
            tx.conn().execute("DELETE FROM google_identities WHERE user_id=?", [DAVID])?;
            if linked {
                tx.conn().execute(
                    "INSERT INTO google_identities(user_id,subject,email,created_at,updated_at) VALUES (?,'account-subject','david@example.test',?,?)",
                    rusqlite::params![DAVID, tx.now(), tx.now()],
                )?;
            }
            Ok(())
        }).await.unwrap();
        let account: api::AccountSettings = parse(&b.send(get("/api/v1/settings/account")).await);
        assert_eq!(account.two_factor.google, linked);
        check_panel(&a, DAVID, &account.two_factor).await;
    }
}

#[tokio::test]
async fn account_routes_do_not_require_sudo_for_the_signed_in_person() {
    let Some(a) = stopped_app().await else { return };
    let mut b = a.sign_in(DAVID).await;
    // Deliberately leave sudo unconfirmed. Wrong confirmation reaches the classic refusal.
    for (method, path) in actions()
        .into_iter()
        .filter(|(_, path)| !path.ends_with("/test"))
    {
        let reply = if method == Method::GET {
            b.send(get(path)).await
        } else {
            write(&mut b, method, path, json!({"reauth": "wrong"})).await
        };
        if path.ends_with("/account") {
            assert_eq!(reply.status, StatusCode::OK);
        } else {
            api_alert(
                &reply,
                "Validation",
                StatusCode::UNPROCESSABLE_ENTITY,
                REFUSED,
            );
        }
    }
}
