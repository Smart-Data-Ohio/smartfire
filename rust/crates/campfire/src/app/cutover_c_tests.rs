//! Remaining Rails HTTP assertions, using fresh Rails fixtures and the real router.
use super::google_api_tests::{self as google, Recorded};
use crate::controllers::presenters::test_support::{DAVID, KEVIN, Req, TestApp};
use axum::http::Method;
use campfire_db::{Message, NewMessage, Timestamp, fixtures};
use campfire_richtext::dom::{Dom, NodeId};
use std::sync::Arc;

pub(crate) async fn app() -> TestApp {
    app_with_env(&[]).await
}
async fn app_with_env(env: &[(&str, &str)]) -> TestApp {
    let app = TestApp::boot_frozen_with_env(env)
        .await
        .expect("pinned default seed")
        .without_job_runner()
        .await;
    app.db().write(|tx| {
        tx.conn().execute_batch("PRAGMA defer_foreign_keys=ON")?;
        let tables = tx.conn().prepare(
            "SELECT name FROM pragma_table_list WHERE schema='main' AND type='table' AND name NOT LIKE 'sqlite_%' AND name NOT IN ('schema_migrations','ar_internal_metadata')",
        )?.query_map([], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for table in tables {
            tx.conn().execute(&format!("DELETE FROM \"{}\"", table.replace('"', "\"\"")), [])?;
        }
        fixtures::load(tx.conn(), &fixtures::reference_dir(), &fixtures::Options {
            now: tx.now(), bcrypt_cost: 4,
        })
    }).await.unwrap();
    google::install(&app, Recorded::new(vec![])).await;
    app
}
fn id(name: &str) -> i64 {
    fixtures::identify(name)
}
fn dom(html: &str) -> (Dom, NodeId) {
    let mut d = Dom::new();
    let root = d.parse_fragment(html).unwrap();
    (d, root)
}
fn class(d: &Dom, n: NodeId, name: &str) -> bool {
    d.attr(n, "class")
        .is_some_and(|c| c.split_whitespace().any(|t| t == name))
}
fn nodes(d: &Dom, root: NodeId, filter: impl Fn(&Dom, NodeId) -> bool) -> Vec<NodeId> {
    d.descendants(root)
        .into_iter()
        .filter(|&n| filter(d, n))
        .collect()
}
async fn grant(app: &TestApp, scope: &str) {
    google::grant(
        app,
        DAVID,
        Timestamp::from_jiff(app.booted.app.clock.now()).since(jiff::SignedDuration::from_hours(1)),
        false,
    )
    .await;
    let scope = scope.to_owned();
    app.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE google_accounts SET scopes=? WHERE user_id=?",
                rusqlite::params![scope, DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
}
const DRIVE: &str = "https://www.googleapis.com/auth/drive.file";
const FILE_A: &str = "1AbcDefGhIjKlMnOpQrSt";
const FILE_B: &str = "2BcdEfgHiJkLmNoPqRsTu";
async fn message(app: &TestApp, files: Vec<String>) -> Message {
    app.db()
        .write(move |tx| {
            Message::create(
                tx,
                NewMessage {
                    room_id: id("watercooler"),
                    creator_id: DAVID,
                    markdown_source: Some("shared".into()),
                    client_message_id: Some("cutover-c-drive".into()),
                    drive_file_ids: files,
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap()
}
#[tokio::test]
async fn cutover_c_drive_chip_markup_is_identical_with_and_without_viewer_consent() {
    let a = app().await;
    let m = message(&a, vec![FILE_A.into()]).await;
    let path = format!("/rooms/{}/messages/{}", m.room_id, m.id);
    let mut b = a.sign_in(DAVID).await;
    let without = b.get(&path).await;
    assert_eq!(without.status, 200);
    let (d, root) = dom(&without.text());
    let chips = nodes(&d, root, |d, n| class(d, n, "drive-attachments"));
    assert_eq!(chips.len(), 1);
    let first = d.to_html(chips[0]);
    grant(&a, DRIVE).await;
    let with = b.get(&path).await;
    assert_eq!(with.status, 200);
    let (d, root) = dom(&with.text());
    let chips = nodes(&d, root, |d, n| class(d, n, "drive-attachments"));
    assert_eq!(chips.len(), 1);
    let second = d.to_html(chips[0]);
    assert_eq!(first, second);
    assert!(second.contains("Google Drive file"));
}
#[tokio::test]
async fn cutover_c_drive_edit_form_has_two_removable_chips_and_exact_hidden_sentinels() {
    let a = app().await;
    let m = message(&a, vec![FILE_A.into(), FILE_B.into()]).await;
    let response = a
        .sign_in(DAVID)
        .await
        .get(&format!("/rooms/{}/messages/{}/edit", m.room_id, m.id))
        .await;
    assert_eq!(response.status, 200);
    let (d, root) = dom(&response.text());
    assert_eq!(
        nodes(&d, root, |d, n| class(d, n, "drive-attachment-chip")).len(),
        2
    );
    for expected in [FILE_A, FILE_B, ""] {
        assert_eq!(
            nodes(&d, root, |d, n| d.local_name(n) == Some("input")
                && d.attr(n, "type") == Some("hidden")
                && d.attr(n, "name") == Some("message[drive_file_ids][]")
                && d.attr(n, "value") == Some(expected))
            .len(),
            1
        );
    }
    assert_eq!(
        nodes(&d, root, |d, n| class(
            d,
            n,
            "drive-attachment-chip__remove"
        ))
        .len(),
        2
    );
}
async fn audit_success(a: &TestApp) -> i64 {
    a.db()
        .read(|c| {
            Ok(c.query_row(
                "SELECT COUNT(*) FROM audit_logs WHERE action='sudo.confirm.success'",
                [],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap()
}
#[tokio::test]
async fn cutover_c_sudo_password_confirmation_adds_one_success_audit_and_verifies() {
    let a = app().await;
    let mut b = a.sign_in(DAVID).await;
    let before = audit_success(&a).await;
    let response = b
        .write(Req::new(Method::POST, "/sudo").form(&[("password", "secret123456")]))
        .await;
    assert_eq!(audit_success(&a).await - before, 1);
    assert_eq!(
        (response.status.as_u16(), response.location()),
        (302, Some("http://campfire.test/"))
    );
    let continued = b.write(Req::new(Method::POST, "/account/join_code")).await;
    assert_eq!(
        (continued.status.as_u16(), continued.location()),
        (302, Some("http://campfire.test/account/edit"))
    );
}
#[tokio::test]
async fn cutover_c_sudo_registered_extra_verifier_is_retained_once_and_unsupported_without_an_implementation()
 {
    let a = app().await;
    a.booted.app.sudo.register_verifier("passkey");
    a.booted.app.sudo.register_verifier("passkey");
    assert!(
        a.booted
            .app
            .sudo
            .extra_verifiers()
            .contains(&"passkey".into())
    );
    assert_eq!(
        a.booted
            .app
            .sudo
            .extra_verifiers()
            .iter()
            .filter(|n| *n == "passkey")
            .count(),
        1
    );
    assert_eq!(
        a.sign_in(DAVID)
            .await
            .write(Req::new(Method::POST, "/sudo").form(&[("verifier", "passkey")]))
            .await
            .status,
        422
    );
}
#[tokio::test]
async fn cutover_c_sudo_enrolled_user_can_still_confirm_with_password() {
    let a = app().await;
    let crypto = a.booted.app.ar_encryption.clone();
    a.db()
        .write(move |tx| {
            let credential =
                campfire_db::TwoFactorCredential::create(tx, &crypto, DAVID, "JBSWY3DPEHPK3PXP")?;
            tx.conn().execute(
                "UPDATE two_factor_credentials SET confirmed_at=? WHERE id=?",
                rusqlite::params![tx.now(), credential.id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let response = a
        .sign_in(DAVID)
        .await
        .write(Req::new(Method::POST, "/sudo").form(&[("password", "secret123456")]))
        .await;
    assert_eq!(
        (response.status.as_u16(), response.location()),
        (302, Some("http://campfire.test/"))
    );
}
#[tokio::test]
async fn cutover_c_sudo_audit_log_read_requires_no_confirmation() {
    let a = app().await;
    assert_eq!(
        a.sign_in(DAVID)
            .await
            .get("/account/audit_log")
            .await
            .status,
        200
    );
}
#[tokio::test]
async fn cutover_c_sudo_signing_in_again_drops_previous_confirmation() {
    let a = app().await;
    let mut b = a.sign_in(DAVID).await;
    b.grant_sudo().await;
    let r = b
        .write(Req::new(Method::POST, "/session").form(&[
            ("email_address", "david@37signals.com"),
            ("password", "secret123456"),
        ]))
        .await;
    assert_eq!(r.status, 302);
    // Satisfy the second factor on the newly created session, as Rails' helper does;
    // this never grants sudo access and preserves the actual re-sign-in cookie.
    a.db()
        .write(|tx| {
            let session_id = tx.conn().query_row(
                "SELECT id FROM sessions WHERE user_id=? ORDER BY id DESC LIMIT 1",
                [DAVID],
                |r| r.get(0),
            )?;
            campfire_db::Session::find(tx.conn(), session_id)?.mark_two_factor_verified(tx)?;
            Ok(())
        })
        .await
        .unwrap();
    let gated = b.write(Req::new(Method::POST, "/account/join_code")).await;
    assert_eq!(
        (gated.status.as_u16(), gated.location()),
        (302, Some("http://campfire.test/sudo/new"))
    );
}
#[tokio::test]
async fn cutover_c_sudo_replay_form_rebuilds_nested_user_role() {
    let a = app().await;
    let mut b = a.sign_in(DAVID).await;
    let path = format!("/account/users/{KEVIN}");
    let gated = b
        .write(Req::new(Method::PATCH, &path).form(&[("user[role]", "administrator")]))
        .await;
    assert_eq!(
        (gated.status.as_u16(), gated.location()),
        (302, Some("http://campfire.test/sudo/new"))
    );
    let r = b
        .write(Req::new(Method::POST, "/sudo").form(&[("password", "secret123456")]))
        .await;
    assert_eq!(r.status, 200);
    let (d, root) = dom(&r.text());
    let forms = nodes(&d, root, |d, n| {
        d.local_name(n) == Some("form") && d.attr(n, "action") == Some(path.as_str())
    });
    assert_eq!(forms.len(), 1);
    assert_eq!(
        nodes(&d, forms[0], |d, n| d.local_name(n) == Some("input")
            && d.attr(n, "name") == Some("user[role]")
            && d.attr(n, "value") == Some("administrator"))
        .len(),
        1
    );
}
// Reaches the shared store through the same Ctx used by the production router.
async fn clear_rate_limit(c: &mut campfire_kit::Ctx) -> campfire_kit::Result {
    c.kit().rate_limits().clear();
    Ok(c.html("cleared"))
}
#[tokio::test]
async fn cutover_c_sudo_confirmation_limit_is_reset_by_clearing_the_shared_store() {
    use campfire_kit::{Kit, KitConfig, RailsCrypto};
    use tower::ServiceExt;
    let a = app().await;
    let kit = Kit::new(
        KitConfig::production(true),
        Arc::new(RailsCrypto::new(a.booted.app.secrets.clone())),
        a.booted.app.clock.clone(),
        a.booted.app.clone(),
    );
    let router = campfire_kit::app(
        axum::Router::new()
            .route(
                "/sudo",
                campfire_kit::post(crate::controllers::sudos::create),
            )
            .route("/clear", campfire_kit::get(clear_rate_limit)),
        kit,
    );
    // Route requests with real signed session/csrf cookies from TestApp's browser.
    let mut b = a.sign_in(DAVID).await;
    let csrf = b.authenticity_token().await;
    let cookie = b.cookie_header();
    let send = |path: &str| {
        axum::http::Request::builder()
            .method(if path == "/clear" { "GET" } else { "POST" })
            .uri(path)
            .header("host", "campfire.test")
            .header("cookie", &cookie)
            .header("content-type", "application/x-www-form-urlencoded")
            .body(axum::body::Body::from(if path == "/clear" {
                String::new()
            } else {
                format!(
                    "authenticity_token={}&password=wrong",
                    crate::controllers::presenters::test_support::encode(&csrf)
                )
            }))
            .unwrap()
    };
    for _ in 0..10 {
        assert_eq!(
            router
                .clone()
                .oneshot(send("/sudo"))
                .await
                .unwrap()
                .status(),
            401
        );
    }
    assert_eq!(
        router
            .clone()
            .oneshot(send("/sudo"))
            .await
            .unwrap()
            .status(),
        429
    );
    assert_eq!(
        router
            .clone()
            .oneshot(send("/clear"))
            .await
            .unwrap()
            .status(),
        200
    );
    assert_eq!(router.oneshot(send("/sudo")).await.unwrap().status(), 401);
}
const PICKER_ENV: &[(&str, &str)] = &[
    ("GOOGLE_CLIENT_ID", "test-client-id"),
    ("GOOGLE_PICKER_API_KEY", "test-picker-key"),
    ("GOOGLE_CLOUD_PROJECT_NUMBER", "123456789012"),
];
async fn room(a: &TestApp) -> crate::controllers::presenters::test_support::Reply {
    a.sign_in(DAVID)
        .await
        .get(&format!("/rooms/{}", id("watercooler")))
        .await
}
fn controller_count(d: &Dom, root: NodeId, name: &str) -> usize {
    nodes(d, root, |d, n| d.attr(n, "data-controller") == Some(name)).len()
}
#[tokio::test]
async fn cutover_c_picker_without_drive_consent_omits_legacy_menu_even_with_calendar_grant() {
    let a = app().await;
    for calendar in [false, true] {
        if calendar {
            grant(&a, "https://www.googleapis.com/auth/calendar.events").await;
        }
        let r = room(&a).await;
        assert_eq!(r.status, 200);
        assert!(!r.text().contains("google-drive-previews"));
        let (d, root) = dom(&r.text());
        assert_eq!(controller_count(&d, root, "drive-picker"), 0);
        assert_eq!(nodes(&d, root, |d, n| class(d, n, "attach-menu")).len(), 0);
        assert_eq!(
            nodes(&d, root, |d, n| d.local_name(n) == Some("button")
                && class(d, n, "composer__attachment-btn")
                && d.has_attr(n, "aria-haspopup"))
            .len(),
            0
        );
    }
}
#[tokio::test]
async fn cutover_c_picker_drive_scope_renders_legacy_menu_buttons_and_dialog() {
    let a = app().await;
    grant(&a, DRIVE).await;
    let r = room(&a).await;
    assert_eq!(r.status, 200);
    assert!(
        r.text()
            .contains("<meta name=\"google-drive-previews\" content=\"enabled\">")
    );
    let (d, root) = dom(&r.text());
    assert_eq!(controller_count(&d, root, "drive-picker"), 1);
    assert_eq!(
        nodes(&d, root, |d, n| d.local_name(n) == Some("button")
            && class(d, n, "composer__attachment-btn")
            && d.attr(n, "aria-haspopup") == Some("menu"))
        .len(),
        1
    );
    let menus = nodes(&d, root, |d, n| class(d, n, "attach-menu"));
    assert_eq!(menus.len(), 1);
    let items = nodes(&d, menus[0], |d, n| d.attr(n, "role") == Some("menuitem"));
    assert_eq!(items.len(), 2);
    assert!(
        items
            .iter()
            .any(|&n| d.text_content(n).trim() == "From this device")
    );
    assert!(
        items
            .iter()
            .any(|&n| d.text_content(n).trim() == "From Google Drive")
    );
    assert_eq!(
        nodes(&d, root, |d, n| class(d, n, "drive-picker__panel")
            && d.attr(n, "role") == Some("dialog")
            && d.attr(n, "aria-label") == Some("Find a Drive file"))
        .len(),
        1
    );
}
#[tokio::test]
async fn cutover_c_picker_configured_sharing_renders_single_enhanced_menu_and_public_metas() {
    let a = app_with_env(PICKER_ENV).await;
    let r = room(&a).await;
    assert_eq!(r.status, 200);
    for meta in [
        "<meta name=\"google-drive-share\" content=\"enabled\">",
        "<meta name=\"google-picker-client-id\" content=\"test-client-id\">",
        "<meta name=\"google-picker-api-key\" content=\"test-picker-key\">",
        "<meta name=\"google-cloud-project-number\" content=\"123456789012\">",
    ] {
        assert!(r.text().contains(meta));
    }
    let (d, root) = dom(&r.text());
    assert_eq!(controller_count(&d, root, "drive-share"), 1);
    assert_eq!(
        nodes(&d, root, |d, n| d.local_name(n) == Some("button")
            && class(d, n, "composer__attachment-btn")
            && d.attr(n, "aria-haspopup") == Some("menu"))
        .len(),
        1
    );
    let menus = nodes(&d, root, |d, n| class(d, n, "attach-menu"));
    assert_eq!(menus.len(), 1);
    assert_eq!(
        nodes(&d, menus[0], |d, n| d.attr(n, "role") == Some("menuitem")
            && d.text_content(n).trim() == "From Google Drive")
        .len(),
        1
    );
    assert_eq!(controller_count(&d, root, "drive-picker"), 0);
}
#[tokio::test]
async fn cutover_c_picker_enhanced_menu_precedes_legacy_picker_with_existing_consent() {
    let a = app_with_env(PICKER_ENV).await;
    grant(&a, DRIVE).await;
    let r = room(&a).await;
    assert_eq!(r.status, 200);
    let (d, root) = dom(&r.text());
    assert_eq!(controller_count(&d, root, "drive-share"), 1);
    assert_eq!(controller_count(&d, root, "drive-picker"), 0);
    assert!(
        r.text()
            .contains("<meta name=\"google-drive-previews\" content=\"enabled\">")
    );
}
#[tokio::test]
async fn cutover_c_picker_missing_sharing_configuration_falls_back_to_legacy() {
    let a = app().await;
    grant(&a, DRIVE).await;
    let r = room(&a).await;
    assert_eq!(r.status, 200);
    assert!(!r.text().contains("google-drive-share"));
    let (d, root) = dom(&r.text());
    assert_eq!(controller_count(&d, root, "drive-share"), 0);
    assert_eq!(controller_count(&d, root, "drive-picker"), 1);
}
#[tokio::test]
async fn cutover_c_picker_unconfigured_without_drive_grant_has_no_menu() {
    let a = app().await;
    let r = room(&a).await;
    assert_eq!(r.status, 200);
    assert!(!r.text().contains("google-drive-share"));
    let (d, root) = dom(&r.text());
    assert_eq!(controller_count(&d, root, "drive-share"), 0);
    assert_eq!(controller_count(&d, root, "drive-picker"), 0);
    assert_eq!(nodes(&d, root, |d, n| class(d, n, "attach-menu")).len(), 0);
}
#[tokio::test]
async fn cutover_c_picker_signed_out_visitors_are_redirected_before_sharing_markup() {
    let a = app_with_env(PICKER_ENV).await;
    let mut b = a.sign_in(DAVID).await;
    b.write(Req::new(Method::DELETE, "/session")).await;
    let r = b.get(&format!("/rooms/{}", id("watercooler"))).await;
    assert_eq!(
        (r.status.as_u16(), r.location()),
        (302, Some("http://campfire.test/session/new"))
    );
}
mod calendar_jobs;

#[tokio::test]
async fn cutover_c_google_account_connected_usable_and_expiry_follow_persisted_changes() {
    use campfire_db::models::google_account::GoogleAccount;
    let a = app().await;
    grant(&a, "https://www.googleapis.com/auth/calendar.events").await;
    let crypto = a.booted.app.ar_encryption.clone();
    let (connected, usable, expired) = a
        .db()
        .write(move |tx| {
            let mut account = GoogleAccount::for_user(tx.conn(), DAVID)?.unwrap();
            Ok((
                account.connected(),
                account.usable(tx, &crypto)?,
                account.access_token_expired(&crypto, tx.now()).unwrap(),
            ))
        })
        .await
        .unwrap();
    assert!(connected);
    assert!(usable);
    assert!(!expired);
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE google_accounts SET access_token_expires_at=? WHERE user_id=?",
                rusqlite::params![tx.now().ago(jiff::SignedDuration::from_mins(1)), DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let crypto = a.booted.app.ar_encryption.clone();
    let now = Timestamp::from_jiff(a.booted.app.clock.now());
    assert!(
        a.db()
            .read(move |c| Ok(GoogleAccount::for_user(c, DAVID)?
                .unwrap()
                .access_token_expired(&crypto, now)
                .unwrap()))
            .await
            .unwrap()
    );
    a.db()
        .write(|tx| {
            GoogleAccount::for_user(tx.conn(), DAVID)?
                .unwrap()
                .mark_disconnected(tx, "Google rejected the connection")
        })
        .await
        .unwrap();
    let crypto = a.booted.app.ar_encryption.clone();
    let (connected, usable) = a
        .db()
        .write(move |tx| {
            let mut account = GoogleAccount::for_user(tx.conn(), DAVID)?.unwrap();
            Ok((account.connected(), account.usable(tx, &crypto)?))
        })
        .await
        .unwrap();
    assert!(!connected);
    assert!(!usable);
}
#[tokio::test]
async fn cutover_c_google_account_calendar_grant_accepts_blank_scopes_and_requires_events_scope_otherwise()
 {
    use campfire_db::models::google_account::GoogleAccount;
    let a = app().await;
    grant(&a, "").await;
    assert!(
        a.db()
            .read(|c| Ok(GoogleAccount::for_user(c, DAVID)?.unwrap().calendar()))
            .await
            .unwrap()
    );
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE google_accounts SET scopes='openid email' WHERE user_id=?",
                [DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert!(
        !a.db()
            .read(|c| Ok(GoogleAccount::for_user(c, DAVID)?.unwrap().calendar()))
            .await
            .unwrap()
    );
    grant(&a, "https://www.googleapis.com/auth/calendar.events").await;
    assert!(
        a.db()
            .read(|c| Ok(GoogleAccount::for_user(c, DAVID)?.unwrap().calendar()))
            .await
            .unwrap()
    );
}
