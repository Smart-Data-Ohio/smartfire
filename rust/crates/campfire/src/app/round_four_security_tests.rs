//! Executed regressions against 5e2aee1e with exact pinned Rails outcomes and audit rows.
use crate::controllers::presenters::test_support::{Browser, DAVID, Req, TestApp};
use axum::http::{Method, StatusCode};
use base64::{Engine, engine::general_purpose::STANDARD};
use campfire_db::{Account, User};
use serde_json::{Value, json};
use std::sync::Arc;

const CHROME: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
fn vectors() -> Value {
    serde_json::from_str(include_str!("../../../../vectors/round_four_security.json")).unwrap()
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
fn request(method: Method, path: &str) -> Req {
    Req::new(method, path)
        .header("user-agent", CHROME)
        .header("x-forwarded-for", "127.0.0.1")
}
fn json_request(method: Method, path: &str, value: &Value) -> Req {
    request(method, path)
        .header("content-type", "application/json")
        .body(value.to_string())
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
async fn clear_audits(a: &TestApp) {
    a.db()
        .write(|tx| {
            tx.conn().execute("DELETE FROM audit_logs", [])?;
            Ok(())
        })
        .await
        .unwrap();
}
async fn admin_browser(a: &TestApp) -> Browser<'_> {
    let mut b = a.sign_in(DAVID).await;
    b.get("/users/me/profile").await;
    assert_eq!(
        b.write(request(Method::POST, "/sudo").form(&[("password", "secret123456")]))
            .await
            .status,
        StatusCode::FOUND
    );
    b
}
fn logo_request(bytes: &[u8]) -> Req {
    request(Method::PATCH, "/account")
        .multipart(&[], ("account[logo]", "logo.png", "image/png", bytes))
}
async fn prepare_account<'a>(a: &'a TestApp, case: &Value) -> Browser<'a> {
    let styles = case["initial_styles"].as_str().map(str::to_owned);
    a.db().write(move |tx| {
        tx.conn().execute("UPDATE accounts SET name='37signals',custom_styles=?,settings=?", rusqlite::params![styles, "{\"restrict_room_creation_to_administrators\":false}"])?;
        tx.conn().execute("DELETE FROM active_storage_attachments WHERE record_type='Account' AND name='logo'", [])?;
        Ok(())
    }).await.unwrap();
    let mut b = admin_browser(a).await;
    if case["initial_logo"] == true {
        let bytes = STANDARD
            .decode(vectors()["logo_bytes"].as_str().unwrap())
            .unwrap();
        let response = b.write(logo_request(&bytes)).await;
        assert_eq!(
            response.status,
            StatusCode::FOUND,
            "fixture logo: {}",
            response.text()
        );
    }
    clear_audits(a).await;
    b
}
fn account_request(case: &Value) -> Req {
    if case["upload"] == true {
        logo_request(
            &STANDARD
                .decode(vectors()["logo_bytes"].as_str().unwrap())
                .unwrap(),
        )
    } else {
        json_request(
            Method::from_bytes(case["method"].as_str().unwrap().to_uppercase().as_bytes()).unwrap(),
            case["path"].as_str().unwrap(),
            &json!({"account":case["params"]}),
        )
    }
}
async fn account_case(name: &str) {
    let case = &vectors()["account"][name];
    let a = app().await;
    let mut b = prepare_account(&a, case).await;
    let before = a.db().read(Account::first).await.unwrap().unwrap();
    let response = b.write(account_request(case)).await;
    assert_eq!(
        json!(response.status.as_u16()),
        case["status"],
        "{name}: {}",
        response.text()
    );
    assert_eq!(
        json!(
            response
                .headers
                .get("location")
                .map(|v| v.to_str().unwrap())
        ),
        case["location"],
        "{name}"
    );
    assert_eq!(
        audits(&a).await,
        case["audits"],
        "{name}: exact actor, updated Account target, metadata and IP/UA"
    );
    let saved = a.db().read(Account::first).await.unwrap().unwrap();
    assert_eq!(json!(saved.name), case["name"], "{name}");
    assert_eq!(json!(saved.custom_styles), case["styles"], "{name}");
    assert_eq!(
        json!(saved.settings().restrict_room_creation_to_administrators()),
        case["restrict"],
        "{name}"
    );
    assert_eq!(
        json!(saved.join_code != before.join_code),
        case["code_changed"],
        "{name}"
    );
    let attached = a.db().read(|c| Ok(c.query_row("SELECT EXISTS(SELECT 1 FROM active_storage_attachments WHERE record_type='Account' AND name='logo')", [], |r| r.get::<_, bool>(0))?)).await.unwrap();
    assert_eq!(json!(attached), case["logo"], "{name}");
}

#[tokio::test]
async fn profile_zone_case_and_alias_validation_matches_rails_without_partial_writes() {
    let snapshots = vectors();
    for name in [
        "america/new_york",
        "AMERICA/NEW_YORK",
        "America/new_York",
        "utc",
        "UTC",
        "America/New_York",
        "US/Eastern",
        "GMT",
        "Etc/GMT+5",
        "Factory",
        "Mars/Olympus",
        "Eastern Time (US & Canada)",
        "eastern time (us & canada)",
        " America/New_York",
        "America/New_York ",
        "",
        " \t",
    ] {
        let case = &snapshots["profile"][name];
        let a = app().await;
        a.db()
            .write(|tx| {
                tx.conn().execute(
                    "UPDATE users SET time_zone='UTC',time_zone_explicit=0 WHERE id=?",
                    [DAVID],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let before = a.db().read(|c| User::find(c, DAVID)).await.unwrap();
        let mut b = admin_browser(&a).await;
        clear_audits(&a).await;
        let response = b
            .write(json_request(
                Method::PATCH,
                "/users/me/profile",
                &json!({"user":{"time_zone":name}}),
            ))
            .await;
        if name == "america/new_york"
            && let Ok(path) = std::env::var("WS9_ZONE_ROLLBACK_DB")
        {
            let source: String = a
                .db()
                .read(|c| Ok(c.query_row("PRAGMA database_list", [], |r| r.get(2))?))
                .await
                .unwrap();
            assert!(std::process::Command::new("python3").args(["-c", "import sqlite3,sys; src=sqlite3.connect(sys.argv[1]); dst=sqlite3.connect(sys.argv[2]); src.backup(dst); dst.close(); src.close()", &source, &path]).status().unwrap().success());
        }
        assert_eq!(json!(response.status.as_u16()), case["status"], "{name}");
        assert_eq!(
            json!(
                response
                    .headers
                    .get("location")
                    .map(|v| v.to_str().unwrap())
            ),
            case["location"],
            "{name}"
        );
        let (zone, explicit): (Option<String>, bool) = a
            .db()
            .read(|c| {
                Ok(c.query_row(
                    "SELECT time_zone,time_zone_explicit FROM users WHERE id=?",
                    [DAVID],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )?)
            })
            .await
            .unwrap();
        assert_eq!(json!(zone), case["zone"], "{name}");
        assert_eq!(json!(explicit), case["explicit"], "{name}");
        assert_eq!(audits(&a).await, case["audits"], "{name}");
        if case["status"] == 422 {
            assert_eq!(
                a.db().read(|c| User::find(c, DAVID)).await.unwrap(),
                before,
                "{name}"
            );
        }
    }
}
#[tokio::test]
async fn settings_audit_has_only_changed_values_and_updated_account_label() {
    for name in [
        "settings",
        "settings_noop",
        "settings_name",
        "settings_bool",
        "settings_array_bool",
        "settings_object_bool",
        "settings_clear_logo",
        "settings_add_logo",
        "settings_replace_logo",
    ] {
        account_case(name).await;
    }
}
#[tokio::test]
async fn join_code_reset_audits_without_disclosing_the_credential() {
    account_case("join_reset").await;
}
#[tokio::test]
async fn styles_audit_has_only_utf8_sizes_and_digest_prefixes_on_actual_changes() {
    for name in [
        "styles",
        "styles_unicode",
        "styles_empty",
        "styles_noop",
        "styles_clear",
        "styles_nil",
        "styles_boolean",
        "styles_array",
    ] {
        account_case(name).await;
    }
}
#[tokio::test]
async fn logo_destroy_audits_even_when_the_attachment_is_already_absent() {
    account_case("logo_destroy").await;
    account_case("logo_absent").await;
}
#[tokio::test]
async fn account_audit_failure_rolls_back_settings_code_styles_and_logo() {
    for name in [
        "settings",
        "join_reset",
        "styles",
        "logo_destroy",
        "settings_add_logo",
    ] {
        let case = &vectors()["account"][name];
        let a = app().await;
        let mut b = prepare_account(&a, case).await;
        let before = a.db().read(Account::first).await.unwrap();
        let attachments = |c: &campfire_db::Connection| -> campfire_db::Result<Vec<i64>> {
            Ok(c.prepare("SELECT id FROM active_storage_attachments WHERE record_type='Account' AND name='logo' ORDER BY id")?.query_map([], |r| r.get(0))?.collect::<rusqlite::Result<Vec<_>>>()?)
        };
        let related = a.db().read(attachments).await.unwrap();
        a.db().write(|tx| { tx.conn().execute_batch("CREATE TRIGGER reject_account_audit BEFORE INSERT ON audit_logs BEGIN SELECT RAISE(ABORT,'review audit failure'); END;")?; Ok(()) }).await.unwrap();
        let response = b.write(account_request(case)).await;
        assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR, "{name}");
        assert_eq!(a.db().read(Account::first).await.unwrap(), before, "{name}");
        assert_eq!(a.db().read(attachments).await.unwrap(), related, "{name}");
        assert_eq!(audits(&a).await, json!([]), "{name}");
    }
}
