//! Real persisted cache facts, complete Rails status-panel bytes and the profile HTTP route.
use crate::controllers::presenters::{self, test_support::*};
use askama::Template;
use campfire_views::users;
use serde_json::Value;

async fn check_case(name: &str) {
    let now: jiff::Timestamp = SEED_NOW.parse().unwrap();
    let app = TestApp::boot_with_clock_and_env(
        std::sync::Arc::new(campfire_kit::FrozenClock::new(now)),
        &[
            ("GOOGLE_CLIENT_ID", "parity-client"),
            ("GOOGLE_CLIENT_SECRET", "parity-secret"),
        ],
    )
    .await
    .expect("seed required");
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/users_profile_effective_ooo.json"
    ))
    .unwrap();
    let case = vectors["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == name)
        .unwrap();
    let setup = case.clone();
    app.db().write(move |tx| {
        for (key, value) in setup["attributes"].as_object().unwrap() {
            let value = match value {
                Value::Null => rusqlite::types::Value::Null,
                Value::Bool(v) => rusqlite::types::Value::Integer(i64::from(*v)),
                Value::String(v) if key == "ooo_until" => rusqlite::types::Value::Text(campfire_db::Timestamp::from_jiff(v.parse().unwrap()).to_db()),
                Value::String(v) => rusqlite::types::Value::Text(v.clone()),
                _ => panic!("unexpected fixture field"),
            };
            tx.conn().execute(&format!("UPDATE users SET {key}=? WHERE id=?"), rusqlite::params![value,DAVID])?;
        }
        tx.conn().execute("DELETE FROM calendar_meeting_caches WHERE user_id=?",[DAVID])?;
        tx.conn().execute("DELETE FROM google_accounts WHERE user_id=?",[DAVID])?;
        tx.conn().execute("INSERT INTO google_accounts(user_id,email,created_at,updated_at) VALUES (?,?,?,?)",rusqlite::params![DAVID,"fixture<&>@example.test",tx.now(),tx.now()])?;
        tx.conn().execute("INSERT INTO calendar_meeting_caches(user_id,ooo_intervals,created_at,updated_at) VALUES (?,?,?,?)",rusqlite::params![DAVID,setup["intervals"].to_string(),tx.now(),tx.now()])?;
        Ok(())
    }).await.unwrap();
    let mut sections = app
        .db()
        .read(move |conn| presenters::profile_sections::load(conn, DAVID, now))
        .await
        .unwrap();
    sections.google.calendar_configured = app.booted.app.config.profile_google_calendar_configured;
    assert_eq!(
        sections.status.manual_ooo,
        case["manual_ooo"].as_bool().unwrap(),
        "{name}: manual state"
    );
    assert_eq!(
        sections.status.ooo_return.as_deref(),
        case["return_date"].as_str(),
        "{name}: effective return date"
    );
    let actual = super::people_tests::render(&app, |ctx| {
        users::StatusPanel { ctx, sections }.render().unwrap()
    });
    assert_eq!(
        actual,
        case["html"].as_str().unwrap(),
        "{name}: complete status panel bytes"
    );
    let page = app.david().get("/users/me/profile").await;
    assert_eq!(page.status, axum::http::StatusCode::OK);
    // The live request retains its real CSRF tokens. This paragraph is session-independent.
    let body = page.text();
    if let Some(date) = case["return_date"].as_str() {
        let label = if case["manual_ooo"] == true {
            format!("Out of office until {date}.")
        } else {
            format!("Out of office until {date} from your Google Calendar.")
        };
        assert!(body.contains(&label), "{name}: missing HTTP return date");
    } else {
        assert!(
            !body.contains("Out of office until "),
            "{name}: inactive OOO must have no return date"
        );
    }
}
macro_rules! cases {
    ($($case:ident),+ $(,)?) => { $(#[tokio::test] async fn $case() { check_case(stringify!($case)).await; })+ };
}
cases!(
    calendar_current,
    calendar_future,
    calendar_start_boundary,
    calendar_end_boundary,
    calendar_disabled,
    calendar_later_end,
    manual_later_end,
    expired_manual_with_calendar,
    member_zone_return_date,
    overlapping_calendar_latest_end,
    fractional_calendar_end,
    calendar_note_without_manual
);
