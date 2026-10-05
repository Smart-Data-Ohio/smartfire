//! Exact remaining sudo and Google model assertions from the pinned Rails declarations.
use super::cutover_c_tests;
use crate::controllers::presenters::test_support::{DAVID, Req, TestApp};
use crate::integrations::google::{api, meeting_refresh};
use axum::http::{Method, StatusCode};
use campfire_db::{
    Timestamp, TwoFactorCredential,
    models::google_account::{ConnectionGrant, GoogleAccount},
};
use campfire_kit::FrozenClock;
use jiff::SignedDuration;
use rails_compat::ar_encryption::ArEncryption;
use serde_json::{Value, json};
use std::sync::Arc;

#[tokio::test]
async fn cutover_d_totp_is_registered_unsupported_and_unavailable_without_enrollment() {
    let a = cutover_c_tests::app().await;
    assert!(
        a.booted
            .app
            .sudo
            .extra_verifiers()
            .iter()
            .any(|name| name == "totp")
    );
    let app = a.booted.app.clone();
    let verification = a
        .db()
        .write(move |tx| {
            if let Some(credential) = TwoFactorCredential::for_user(tx.conn(), DAVID)? {
                credential.destroy(tx)?;
            }
            app.sudo.verify_totp(tx, DAVID, &app.secrets, "123456")
        })
        .await
        .unwrap();
    assert_eq!(verification, None);
    let app = a.booted.app.clone();
    let available = a
        .db()
        .read(move |conn| app.sudo.verifier_available(conn, "totp", DAVID))
        .await
        .unwrap();
    assert!(!available);
}

#[tokio::test]
async fn cutover_d_wrong_totp_audits_failure_and_initial_session_stays_gated() {
    let a = cutover_c_tests::app().await;
    let crypto = a.booted.app.ar_encryption.clone();
    a.db()
        .write(move |tx| {
            let credential = TwoFactorCredential::create(tx, &crypto, DAVID, "JBSWY3DPEHPK3PXP")?;
            tx.conn().execute(
                "UPDATE two_factor_credentials SET confirmed_at=? WHERE id=?",
                rusqlite::params![tx.now(), credential.id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut b = a.sign_in_for_tests(DAVID).await;
    let before = a
        .db()
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT count(*) FROM audit_logs WHERE action='sudo.confirm.failure'",
                [],
                |r| r.get::<_, i64>(0),
            )?)
        })
        .await
        .unwrap();
    let rejected = b
        .write(
            Req::new(Method::POST, "/sudo").form(&[("verifier", "totp"), ("totp_code", "000000")]),
        )
        .await;
    let after = a
        .db()
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT count(*) FROM audit_logs WHERE action='sudo.confirm.failure'",
                [],
                |r| r.get::<_, i64>(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(after, before + 1);
    assert_eq!(rejected.status, StatusCode::UNAUTHORIZED);
    let verifier = a.db().read(|conn| Ok(conn.query_row(
        "SELECT json_extract(details,'$.verifier') FROM audit_logs WHERE action='sudo.confirm.failure' ORDER BY id DESC LIMIT 1", [], |r| r.get::<_,String>(0))?)).await.unwrap();
    assert_eq!(verifier, "totp");
    let gated = b.write(Req::new(Method::POST, "/account/join_code")).await;
    assert_eq!(gated.status, StatusCode::FOUND);
    assert_eq!(gated.location(), Some("http://campfire.test/sudo/new"));
}

#[tokio::test]
async fn cutover_d_gated_audit_csv_get_continues_and_returns_csv() {
    let a = cutover_c_tests::app().await;
    let mut b = a.sign_in_for_tests(DAVID).await;
    let gated = b.get("/account/audit_log.csv").await;
    assert_eq!(gated.status, StatusCode::FOUND);
    assert_eq!(gated.location(), Some("http://campfire.test/sudo/new"));
    let confirmed = b
        .write(Req::new(Method::POST, "/sudo").form(&[("password", "secret123456")]))
        .await;
    assert_eq!(confirmed.status, StatusCode::FOUND);
    assert_eq!(
        confirmed.location(),
        Some("http://campfire.test/account/audit_log.csv")
    );
    let csv = b.get(confirmed.location().unwrap()).await;
    assert_eq!(csv.status, StatusCode::OK);
    assert_eq!(
        csv.headers
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap(),
        "text/csv"
    );
}

async fn sudo_at_minutes(minutes: i64) -> crate::controllers::presenters::test_support::Reply {
    let clock = Arc::new(FrozenClock::new("2026-03-02T16:00:00Z".parse().unwrap()));
    let a = TestApp::boot_with_test_clock(clock.clone())
        .await
        .unwrap()
        .without_job_runner()
        .await;
    load_fixtures(&a).await;
    let mut b = a.sign_in_for_tests(DAVID).await;
    let confirmed = b
        .write(Req::new(Method::POST, "/sudo").form(&[("password", "secret123456")]))
        .await;
    assert_eq!(confirmed.status, StatusCode::FOUND);
    assert_eq!(confirmed.location(), Some("http://campfire.test/"));
    clock.advance(SignedDuration::from_mins(minutes));
    b.write(Req::new(Method::POST, "/account/join_code")).await
}
#[tokio::test]
async fn cutover_d_fourteen_minute_confirmation_allows_real_join_code_post() {
    let response = sudo_at_minutes(14).await;
    assert_eq!(response.status, StatusCode::FOUND);
    assert_eq!(
        response.location(),
        Some("http://campfire.test/account/edit")
    );
}
#[tokio::test]
async fn cutover_d_sixteen_minute_confirmation_gates_real_join_code_post() {
    let response = sudo_at_minutes(16).await;
    assert_eq!(response.status, StatusCode::FOUND);
    assert_eq!(response.location(), Some("http://campfire.test/sudo/new"));
}

#[tokio::test]
async fn cutover_d_duplicate_google_account_is_invalid_before_persistence() {
    let a = cutover_c_tests::app().await;
    connect_google(&a).await;
    let duplicate = ConnectionGrant {
        user_id: DAVID,
        email: "other@gmail.test".into(),
        access_token: None,
        access_token_expires_at: None,
        refresh_token: None,
        scopes: None,
    };
    let errors = a
        .db()
        .read(move |conn| GoogleAccount::validate_new(conn, &duplicate))
        .await
        .unwrap();
    assert!(!errors.is_empty());
    assert_eq!(errors.on("user_id"), ["has already been taken"]);
    let app = a.booted.app.clone();
    let creation = a
        .db()
        .write(move |tx| {
            GoogleAccount::create(
                tx,
                &ArEncryption::new(&app.secrets),
                ConnectionGrant {
                    user_id: DAVID,
                    email: "other@gmail.test".into(),
                    access_token: None,
                    access_token_expires_at: None,
                    refresh_token: None,
                    scopes: None,
                },
            )
        })
        .await;
    assert!(matches!(
        creation,
        Err(campfire_db::Error::RecordInvalid(_))
    ));
    assert_eq!(
        a.db()
            .read(|conn| Ok(conn.query_row(
                "SELECT count(*) FROM google_accounts WHERE user_id=?",
                [DAVID],
                |r| r.get::<_, i64>(0)
            )?))
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        a.db()
            .read(|conn| Ok(GoogleAccount::for_user(conn, DAVID)?.unwrap().email))
            .await
            .unwrap(),
        "david@gmail.test"
    );
}

struct EscapingJsonParser;
impl meeting_refresh::EventLister for EscapingJsonParser {
    fn list<'a>(
        &'a self,
        _app: &'a crate::app::App,
        _user_id: i64,
        _start: Timestamp,
        _end: Timestamp,
        _now: Timestamp,
    ) -> crate::integrations::net::BoxFuture<'a, api::Result<Value>> {
        Box::pin(async {
            Err(api::Error::JsonParser(
                <serde_json::Error as serde::de::Error>::custom("unexpected token"),
            ))
        })
    }
}
#[tokio::test]
async fn cutover_d_escaping_json_parser_keeps_existing_busy_intervals_and_records_error() {
    let a = cutover_c_tests::app().await;
    let now = Timestamp::from_jiff(a.booted.app.clock.now());
    connect_google(&a).await;
    let busy = json!([["2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z"]]);
    let persisted = busy.clone();
    a.db().write(move |tx| {
        tx.conn().execute("UPDATE users SET meeting_status_enabled=1 WHERE id=?", [DAVID])?;
        tx.conn().execute("INSERT INTO calendar_meeting_caches(user_id,busy_intervals,ooo_intervals,created_at,updated_at) VALUES(?,?,'[]',?,?)", rusqlite::params![DAVID,persisted.to_string(),tx.now(),tx.now()])?;
        Ok(())
    }).await.unwrap();
    let result =
        meeting_refresh::refresh_with_client(&a.booted.app, DAVID, now, &EscapingJsonParser)
            .await
            .unwrap();
    assert_eq!(result, meeting_refresh::Result::Error);
    let cache = a
        .db()
        .read(|conn| campfire_db::models::google_meeting_cache::find(conn, DAVID))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(cache.busy, busy);
    assert_eq!(
        cache.fetch_error.as_deref(),
        Some(meeting_refresh::UNREACHABLE)
    );
}

async fn connect_google(a: &TestApp) {
    let crypto = a.booted.app.ar_encryption.clone();
    a.db()
        .write(move |tx| {
            GoogleAccount::create(
                tx,
                &crypto,
                ConnectionGrant {
                    user_id: DAVID,
                    email: "david@gmail.test".into(),
                    access_token: Some(format!("access-token-{DAVID}")),
                    refresh_token: Some(format!("refresh-token-{DAVID}")),
                    access_token_expires_at: Some(tx.now().since(SignedDuration::from_hours(1))),
                    scopes: None,
                },
            )?;
            Ok(())
        })
        .await
        .unwrap();
}
async fn load_fixtures(a: &TestApp) {
    a.db().write(|tx| {
        tx.conn().execute_batch("PRAGMA defer_foreign_keys=ON")?;
        let tables = tx.conn().prepare("SELECT name FROM pragma_table_list WHERE schema='main' AND type='table' AND name NOT LIKE 'sqlite_%' AND name NOT IN ('schema_migrations','ar_internal_metadata')")?
            .query_map([], |r| r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        for table in tables { tx.conn().execute(&format!("DELETE FROM \"{}\"",table.replace('"',"\"\"")), [])?; }
        campfire_db::fixtures::load(tx.conn(), &campfire_db::fixtures::reference_dir(), &campfire_db::fixtures::Options { now: tx.now(), bcrypt_cost: 4 })
    }).await.unwrap();
}
