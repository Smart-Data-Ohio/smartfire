//! The profile badges of test/system/meeting_status_test.rb:7 (WS14g-269) and
//! out_of_office_test.rb:4 (WS14g-271), over the SPA's profile and presence APIs,
//! with Google's events list answered by the payload their WebMock stub returned.
use super::google_api_tests::{self as google, Recorded};
use crate::{
    controllers::presenters::test_support::{DAVID, DIRECT_DAVID_JASON, JASON, Req, SEED_NOW, TestApp},
    integrations::google::meeting_refresh,
};
use axum::http::Method;
use campfire_db::{Timestamp, models::calendar_dispatch::dispatch_meetings};
use campfire_kit::FrozenClock;
use jiff::SignedDuration;
use serde_json::json;
use std::sync::Arc;

/// `timed_calendar_item` in google_calendar_test_helper.rb.
fn item(start: jiff::Timestamp, end: jiff::Timestamp) -> serde_json::Value {
    json!({
        "status": "confirmed",
        "start": {"dateTime": start.strftime("%Y-%m-%dT%H:%M:%SZ").to_string()},
        "end": {"dateTime": end.strftime("%Y-%m-%dT%H:%M:%SZ").to_string()},
    })
}

/// The Rails system tests' database: the test fixtures, at the frozen instant, on the seeded app.
async fn boot(clock: Arc<FrozenClock>) -> TestApp {
    let app = TestApp::boot_without_periodic_with_clock(clock)
        .await
        .expect("seed required")
        // `perform_enqueued_jobs only:` and the dispatcher run explicitly, as in the Ruby test.
        .without_job_runner()
        .await;
    app.db().write(|tx| {
        tx.conn().execute_batch("PRAGMA defer_foreign_keys=ON")?;
        let tables = tx.conn().prepare("SELECT name FROM pragma_table_list WHERE schema='main' AND type='table' AND name NOT LIKE 'sqlite_%' AND name NOT IN ('schema_migrations','ar_internal_metadata')")?
            .query_map([], |row| row.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        for table in tables {
            tx.conn().execute(&format!("DELETE FROM \"{table}\""), [])?;
        }
        campfire_db::fixtures::load(tx.conn(), &campfire_db::fixtures::reference_dir(), &campfire_db::fixtures::Options {
            now: tx.now(),
            bcrypt_cost: 4,
        })
    }).await.unwrap();
    app
}

async fn refresh_jobs(app: &TestApp) -> i64 {
    app.db()
        .read(|c| {
            Ok(c.query_row(
                "SELECT COUNT(*) FROM background_jobs WHERE job_class='Calendar::MeetingRefreshJob' AND arguments=?",
                [json!({"user_id": DAVID}).to_string()],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn ws14g_269_opting_in_shows_in_a_meeting_for_a_busy_interval_then_clears_after_it_ends() {
    let now: jiff::Timestamp = SEED_NOW.parse().unwrap();
    let clock = Arc::new(FrozenClock::new(now));
    let app = boot(clock.clone()).await;
    let recorded = Recorded::new(vec![]);
    google::install(&app, recorded.clone()).await;
    google::grant(&app, DAVID, Timestamp::from_jiff(now).since(SignedDuration::from_hours(1)), false).await;
    let minutes = |m: i64| now + SignedDuration::from_mins(m);
    let mut transparent = item(minutes(-6), minutes(-4));
    transparent["transparency"] = json!("transparent");
    recorded.answer(200, json!({"items": [item(minutes(-5), minutes(5)), transparent]}));

    let mut david = app.sign_in(DAVID).await;
    let reply = david
        .write(Req::new(Method::PATCH, "/users/me/status").form(&[("user[meeting_status_enabled]", "1")]))
        .await;
    assert!(reply.status.is_redirection() || reply.status.is_success(), "{}", reply.status);
    let enabled: bool = app
        .db()
        .read(|c| Ok(c.query_row("SELECT meeting_status_enabled FROM users WHERE id=?", [DAVID], |r| r.get(0))?))
        .await
        .unwrap();
    assert!(enabled, "meeting status was not enabled");

    assert!(refresh_jobs(&app).await > 0, "opting in enqueues the refresh the test performs");
    let result = meeting_refresh::refresh(&app.booted.app, DAVID, Timestamp::from_jiff(now)).await.unwrap();
    assert!(matches!(result, meeting_refresh::Result::Ok), "the refresh fetched and cached");
    let calls = recorded.calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 1, "one events.list call: {calls:?}");
    assert!(calls[0]["path"].as_str().unwrap().contains("singleEvents=true"), "{calls:?}");
    dispatch_meetings(app.db(), Timestamp::from_jiff(now)).await.unwrap();

    let profile = david.get(&format!("/api/v1/people/{DAVID}")).await;
    assert_eq!(profile.status, 200);
    assert_eq!(profile.json()["status"]["statusText"], "📅 In a meeting");

    // travel_to 6.minutes.from_now
    clock.advance(SignedDuration::from_mins(6));
    dispatch_meetings(app.db(), Timestamp::from_jiff(minutes(6))).await.unwrap();
    let profile = david.get(&format!("/api/v1/people/{DAVID}")).await;
    assert_eq!(profile.status, 200);
    assert_eq!(
        profile.json()["status"]["statusText"],
        serde_json::Value::Null
    );
}

#[tokio::test]
async fn ws14g_271_out_of_office_badge_and_dm_notice_show_for_another_user_then_clear() {
    let now: jiff::Timestamp = SEED_NOW.parse().unwrap();
    let app = boot(Arc::new(FrozenClock::new(now))).await;
    let manual_ooo = || async {
        let settings = app.db().read(|c| campfire_db::UserStatusSettings::find(c, DAVID)).await.unwrap();
        settings.manual_ooo_active(Timestamp::from_jiff(now))
    };
    let mut david = app.sign_in(DAVID).await;
    david
        .write(Req::new(Method::PATCH, "/users/me/status").form(&[
            ("user[ooo_preset]", "tomorrow"),
            ("user[ooo_note]", "Back soon"),
        ]))
        .await;
    assert!(manual_ooo().await, "OOO was not set");

    let profile = david.get(&format!("/api/v1/people/{DAVID}")).await;
    assert_eq!(profile.status, 200);
    assert_eq!(
        profile.json()["status"]["statusText"],
        "🌴 Out of office until March 03, 2026 — Back soon"
    );
    let mut jason = app.sign_in(JASON).await;
    let room = jason
        .get(&format!("/api/v1/rooms/{DIRECT_DAVID_JASON}"))
        .await;
    assert_eq!(room.status, 200);
    assert_eq!(room.json()["directMemberIds"], json!([DAVID]));
    let presence = jason.get(&format!("/api/v1/presence?ids={DAVID}")).await;
    assert_eq!(presence.status, 200);
    assert_eq!(presence.json()["presences"][0]["userId"], DAVID);
    assert_eq!(
        presence.json()["presences"][0]["statusText"],
        profile.json()["status"]["statusText"]
    );

    david
        .write(Req::new(Method::PATCH, "/users/me/status").form(&[("user[clear_ooo]", "1")]))
        .await;
    assert!(!manual_ooo().await, "OOO was not cleared");
    let profile = david.get(&format!("/api/v1/people/{DAVID}")).await;
    assert_eq!(profile.status, 200);
    assert_eq!(
        profile.json()["status"]["statusText"],
        serde_json::Value::Null
    );
    let presence = jason.get(&format!("/api/v1/presence?ids={DAVID}")).await;
    assert_eq!(presence.status, 200);
    assert_eq!(
        presence.json()["presences"][0]["statusText"],
        serde_json::Value::Null
    );
}
