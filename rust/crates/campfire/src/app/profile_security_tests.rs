//! Review regressions exercise real requests, CSRF, and persisted security state.
use crate::controllers::presenters::test_support::{DAVID, Req, TestApp};
use axum::http::{Method, StatusCode};
use campfire_db::{TwoFactorRememberedDevice, User};
use serde_json::{Value, json};
use std::sync::Arc;
const CHROME: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
fn vectors() -> Value {
    serde_json::from_str(include_str!("../../../../vectors/profile_security.json")).unwrap()
}

async fn app() -> TestApp {
    TestApp::boot_with_clock(Arc::new(campfire_kit::FrozenClock::new(
        crate::controllers::presenters::test_support::SEED_NOW
            .parse()
            .unwrap(),
    )))
    .await
    .expect("build the pinned WS19 default seed")
}
async fn failures(a: &TestApp) -> i64 {
    a.db()
        .read(|c| {
            Ok(c.query_row(
                "SELECT count(*) FROM audit_logs WHERE action='session.sign_in.failure'",
                [],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn email_and_new_password_require_the_existing_password_before_any_write() {
    let a = app().await;
    let mut b = a.sign_in(DAVID).await;
    b.get("/users/me/profile").await;
    let before = a.db().read(|c| User::find(c, DAVID)).await.unwrap();
    let response = b
        .write(Req::new(Method::PATCH, "/users/me/profile").form(&[
            ("user[email_address]", "ws9-reviewed@example.test"),
            ("user[password]", "proposed-password"),
            ("user[name]", "Submitted name"),
            ("user[bio]", "Submitted bio"),
        ]))
        .await;
    assert_eq!(
        response.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        response.text()
    );
    assert_eq!(a.db().read(|c| User::find(c, DAVID)).await.unwrap(), before);
    assert!(
        response
            .text()
            .contains("Current password is required to change your email address.")
    );
    assert!(response.text().contains("Submitted name"));
    assert!(response.text().contains("ws9-reviewed@example.test"));
    assert!(!response.text().contains("proposed-password"));
}

#[tokio::test]
async fn self_changed_email_persists_marker_and_audit_for_rails_rollback() {
    let a = app().await;
    let mut b = a.sign_in(DAVID).await;
    b.get("/users/me/profile").await;
    let response = b
        .write(Req::new(Method::PATCH, "/users/me/profile").form(&[
            ("user[email_address]", "ws9-reviewed@example.test"),
            ("user[current_password]", "secret123456"),
        ]))
        .await;
    assert_eq!(response.status, StatusCode::FOUND);
    if let Ok(path) = std::env::var("WS9_PROFILE_ROLLBACK_DB") {
        let source: String = a
            .db()
            .read(|c| Ok(c.query_row("PRAGMA database_list", [], |r| r.get(2))?))
            .await
            .unwrap();
        assert!(std::process::Command::new("python3").args([
            "-c", "import sqlite3,sys; src=sqlite3.connect(sys.argv[1]); dst=sqlite3.connect(sys.argv[2]); src.backup(dst); dst.close(); src.close()",
            &source, &path,
        ]).status().unwrap().success());
    }
    let (marker, allowed, audit): (Option<String>, bool, i64) = a.db().read(|c| Ok(c.query_row(
        "SELECT email_self_changed_at,google_email_link_allowed,(SELECT count(*) FROM audit_logs WHERE action='user.email.change' AND target_id=users.id) FROM users WHERE id=?", [DAVID],
        |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?)).await.unwrap();
    assert!(
        marker.is_some(),
        "email_self_changed_at={marker:?}; google_email_link_allowed={allowed}; email audit rows={audit}"
    );
    assert!(
        allowed,
        "Rails retains this flag; the marker blocks linking"
    );
    assert_eq!(audit, 1);
}

#[tokio::test]
async fn unauthorized_password_request_writes_a_filtered_failure_audit() {
    let a = app().await;
    let mut b = a.anonymous();
    b.get("/session/new").await;
    let before = failures(&a).await;
    let response = b
        .write(Req::new(Method::POST, "/session").form(&[
            ("email_address", "david@37signals.com"),
            ("password", "wrong"),
        ]))
        .await;
    assert_eq!(response.status, StatusCode::UNAUTHORIZED);
    assert_eq!(failures(&a).await, before + 1);
}

#[tokio::test]
async fn rate_limited_password_request_writes_a_failure_audit() {
    let a = app().await;
    let mut b = a.anonymous();
    b.get("/session/new").await;
    for _ in 0..10 {
        assert_eq!(
            b.write(Req::new(Method::POST, "/session").form(&[
                ("email_address", "david@37signals.com"),
                ("password", "wrong")
            ]))
            .await
            .status,
            StatusCode::UNAUTHORIZED
        );
    }
    let before = failures(&a).await;
    let response = b
        .write(Req::new(Method::POST, "/session").form(&[
            ("email_address", "another@example.test"),
            ("password", "wrong"),
        ]))
        .await;
    assert_eq!(response.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(failures(&a).await, before + 1);
}

#[tokio::test]
async fn rejected_transfer_writes_a_filtered_failure_audit() {
    let a = app().await;
    let mut b = a.anonymous();
    b.get("/session/transfers/invalid-transfer").await;
    let before = failures(&a).await;
    let response = b
        .write(Req::new(Method::PUT, "/session/transfers/invalid-transfer"))
        .await;
    assert_eq!(response.status, StatusCode::BAD_REQUEST);
    assert_eq!(failures(&a).await, before + 1);
}

async fn audits(a: &TestApp) -> Value {
    a.db()
        .read(|c| {
            let ids = c
                .prepare("SELECT id FROM audit_logs ORDER BY id")?
                .query_map([], |r| r.get::<_, i64>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(Value::Array(
                ids.into_iter()
                    .map(|id| {
                        campfire_db::models::audit_log::AuditLog::find(c, id)
                            .map(|row| row.snapshot())
                    })
                    .collect::<campfire_db::Result<Vec<_>>>()?,
            ))
        })
        .await
        .unwrap()
}
#[tokio::test]
async fn profile_guard_fields_errors_and_security_writes_match_pinned_rails() {
    for (name, case) in vectors()["profile"].as_object().unwrap() {
        let a = app().await;
        a.db()
            .write(|tx| {
                tx.conn().execute("DELETE FROM audit_logs", [])?;
                TwoFactorRememberedDevice::revoke_all(tx, DAVID)?;
                TwoFactorRememberedDevice::create_for(tx, DAVID, Some(CHROME), Some("127.0.0.1"))?;
                Ok(())
            })
            .await
            .unwrap();
        if name == "passwordless" || name == "unicode_case_only" {
            let passwordless = name == "passwordless";
            a.db()
                .write(move |tx| {
                    if passwordless {
                        tx.conn()
                            .execute("UPDATE users SET password_digest=NULL WHERE id=?", [DAVID])?;
                    } else {
                        tx.conn().execute(
                            "UPDATE users SET email_address='STRASSE@example.test' WHERE id=?",
                            [DAVID],
                        )?;
                    }
                    Ok(())
                })
                .await
                .unwrap();
        }
        let before = a.db().read(|c| User::find(c, DAVID)).await.unwrap();
        let mut b = a.sign_in(DAVID).await;
        let page = b
            .send(
                Req::new(Method::GET, "/users/me/profile")
                    .header("user-agent", CHROME)
                    .header("x-forwarded-for", "127.0.0.1"),
            )
            .await;
        assert_eq!(
            page.text().contains("id=\"user_current_password\""),
            name != "passwordless",
            "{name}"
        );
        let mut request = Req::new(Method::PATCH, "/users/me/profile")
            .header("user-agent", CHROME)
            .header("x-forwarded-for", "127.0.0.1");
        if case["params"]
            .as_object()
            .unwrap()
            .values()
            .any(|v| !v.is_string())
        {
            request = request
                .header("content-type", "application/json")
                .body(json!({"user": case["params"]}).to_string());
        } else {
            let fields: Vec<(String, String)> = case["params"]
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (format!("user[{k}]"), v.as_str().unwrap().into()))
                .collect();
            let pairs: Vec<_> = fields
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();
            request = request.form(&pairs);
        }
        let response = b.write(request).await;
        assert_eq!(
            response.status.as_u16(),
            case["status"].as_u64().unwrap() as u16,
            "{name}: {}",
            response.text()
        );
        if response.status == StatusCode::FOUND {
            assert_eq!(response.location(), Some("http://campfire.test/users/me/profile"), "{name}");
        }
        let saved = a.db().read(|c| User::find(c, DAVID)).await.unwrap();
        assert_eq!(json!(saved.email_address), case["email"], "{name}");
        assert_eq!(json!(saved.name), case["name"], "{name}");
        assert_eq!(json!(saved.bio), case["bio"], "{name}");
        assert_eq!(
            json!(saved.password_digest != before.password_digest),
            case["password_changed"],
            "{name}"
        );
        let (marker, allowed): (Option<campfire_db::Timestamp>, bool) = a
            .db()
            .read(|c| {
                Ok(c.query_row(
                    "SELECT email_self_changed_at,google_email_link_allowed FROM users WHERE id=?",
                    [DAVID],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )?)
            })
            .await
            .unwrap();
        assert_eq!(
            marker.map(|t| t.jiff()),
            case["marker"]
                .as_str()
                .map(|v| v.parse::<jiff::Timestamp>().unwrap()),
            "{name}"
        );
        assert_eq!(json!(allowed), case["allowed"], "{name}");
        assert_eq!(audits(&a).await, case["audits"], "{name}");
        let device_count = a
            .db()
            .read(|c| Ok(TwoFactorRememberedDevice::for_user(c, DAVID)?.len()))
            .await
            .unwrap();
        assert_eq!(
            device_count,
            if case["password_changed"] == true {
                0
            } else {
                1
            },
            "{name}"
        );
        for key in ["current_password_input", "error_html"] {
            if let Some(html) = case[key].as_str() {
                assert!(
                    response.text().contains(html),
                    "{name}: missing {key}: {html}\n{}",
                    response.text()
                );
            }
        }
    }
}
#[tokio::test]
async fn password_failure_rows_collapse_filter_and_cap_exactly_like_rails() {
    let a = app().await;
    a.db()
        .write(|tx| {
            tx.conn().execute("DELETE FROM audit_logs", [])?;
            Ok(())
        })
        .await
        .unwrap();
    let mut b = a.anonymous();
    b.get("/session/new").await;
    for case in vectors()["failure"].as_array().unwrap() {
        let response = b
            .write(
                Req::new(Method::POST, "/session")
                    .header("user-agent", CHROME)
                    .header("x-forwarded-for", "127.0.0.1")
                    .form(&[
                        ("email_address", case["email"].as_str().unwrap()),
                        ("password", "wrong"),
                    ]),
            )
            .await;
        assert_eq!(
            response.status.as_u16(),
            case["status"].as_u64().unwrap() as u16
        );
        assert_eq!(audits(&a).await, case["audits"]);
    }
    let a = app().await;
    a.db()
        .write(|tx| {
            tx.conn().execute("DELETE FROM audit_logs", [])?;
            Ok(())
        })
        .await
        .unwrap();
    let mut b = a.anonymous();
    b.get("/session/new").await;
    let response = b
        .write(
            Req::new(Method::PUT, "/session/transfers/invalid-transfer")
                .header("user-agent", CHROME)
                .header("x-forwarded-for", "127.0.0.1"),
        )
        .await;
    assert_eq!(
        response.status.as_u16(),
        vectors()["transfer"]["status"].as_u64().unwrap() as u16
    );
    assert_eq!(audits(&a).await, vectors()["transfer"]["audits"]);
    let clock = Arc::new(campfire_kit::FrozenClock::new(
        crate::controllers::presenters::test_support::SEED_NOW
            .parse()
            .unwrap(),
    ));
    let a = TestApp::boot_with_clock(clock.clone())
        .await
        .expect("build seed");
    a.db()
        .write(|tx| {
            tx.conn().execute("DELETE FROM audit_logs", [])?;
            Ok(())
        })
        .await
        .unwrap();
    let start: jiff::Timestamp = crate::controllers::presenters::test_support::SEED_NOW
        .parse()
        .unwrap();
    let mut b = a.anonymous();
    b.get("/session/new").await;
    for case in vectors()["failure_windows"].as_array().unwrap() {
        clock.set(
            start
                .checked_add(jiff::SignedDuration::from_secs(
                    case["offset"].as_i64().unwrap(),
                ))
                .unwrap(),
        );
        let response = b
            .write(
                Req::new(Method::POST, "/session")
                    .header("user-agent", CHROME)
                    .header("x-forwarded-for", case["ip"].as_str().unwrap())
                    .form(&[
                        ("email_address", case["email"].as_str().unwrap()),
                        ("password", "wrong"),
                    ]),
            )
            .await;
        assert_eq!(
            response.status.as_u16(),
            case["status"].as_u64().unwrap() as u16
        );
        assert_eq!(audits(&a).await, case["audits"]);
    }
}
#[tokio::test]
async fn rejected_profile_security_audit_rolls_back_every_change_and_marker() {
    for action in ["user.email.change", "user.password.change"] {
        let a = app().await;
        a.db()
            .write(|tx| {
                TwoFactorRememberedDevice::revoke_all(tx, DAVID)?;
                TwoFactorRememberedDevice::create_for(tx, DAVID, Some(CHROME), Some("127.0.0.1"))?;
                Ok(())
            })
            .await
            .unwrap();
        let before = a.db().read(|c| User::find(c, DAVID)).await.unwrap();
        a.db().write(move |tx| { tx.conn().execute_batch(&format!("CREATE TRIGGER refuse_profile_audit BEFORE INSERT ON audit_logs WHEN NEW.action='{action}' BEGIN SELECT RAISE(ABORT, 'audit unavailable'); END"))?; Ok(()) }).await.unwrap();
        let mut b = a.sign_in(DAVID).await;
        b.get("/users/me/profile").await;
        let response = b
            .write(Req::new(Method::PATCH, "/users/me/profile").form(&[
                ("user[email_address]", "ws9-reviewed@example.test"),
                ("user[current_password]", "secret123456"),
                ("user[password]", "proposed-password"),
                ("user[name]", "Submitted name"),
                ("user[bio]", "Submitted bio"),
            ]))
            .await;
        assert_eq!(
            response.status,
            StatusCode::INTERNAL_SERVER_ERROR,
            "{action}"
        );
        assert_eq!(a.db().read(|c| User::find(c, DAVID)).await.unwrap(), before);
        a.db()
            .read(|c| {
                assert!(
                    c.query_row(
                        "SELECT email_self_changed_at FROM users WHERE id=?",
                        [DAVID],
                        |r| r.get::<_, Option<String>>(0)
                    )?
                    .is_none()
                );
                assert_eq!(TwoFactorRememberedDevice::for_user(c, DAVID)?.len(), 1);
                assert_eq!(
                    c.query_row(
                        "SELECT count(*) FROM audit_logs WHERE action LIKE 'user.%.change'",
                        [],
                        |r| r.get::<_, i64>(0)
                    )?,
                    0
                );
                Ok(())
            })
            .await
            .unwrap();
    }
}
