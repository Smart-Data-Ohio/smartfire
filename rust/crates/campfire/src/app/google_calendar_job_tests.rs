//! Consumers execute with recorded Google responses and the real model/queue.
use super::google_api_tests::{self as support, Recorded};
use crate::{
    controllers::presenters::test_support::{DAVID, TestApp},
    integrations::google::{
        api::{Api, Config},
        calendar, meeting_refresh,
    },
};
use campfire_db::{
    Timestamp,
    models::{
        google_calendar::{MeetingRefreshJob, PushChannel},
        google_meeting_cache as cache,
        room_delete::RemoteDeleteJob,
    },
};
use serde_json::{Value, json};
use std::sync::Arc;
async fn app() -> (TestApp, Arc<Recorded>) {
    let a = TestApp::boot_without_periodic()
        .await
        .expect("default seed required");
    let r = Recorded::new(vec![]);
    support::install(&a, r.clone()).await;
    a.db()
        .write(|tx| {
            tx.conn().execute("DELETE FROM google_accounts", [])?;
            tx.conn()
                .execute("DELETE FROM calendar_push_channels", [])?;
            tx.conn()
                .execute("DELETE FROM calendar_meeting_caches", [])?;
            Ok(())
        })
        .await
        .unwrap();
    (a, r)
}
fn now(a: &TestApp) -> Timestamp {
    Timestamp::from_jiff(a.booted.app.clock.now())
}
async fn grant(a: &TestApp) {
    support::grant(
        a,
        DAVID,
        now(a).since(jiff::SignedDuration::from_hours(1)),
        false,
    )
    .await;
}
fn blob(a: &TestApp) -> String {
    rails_compat::calendar_credentials::encrypt(
        &a.booted.app.secrets,
        &rails_compat::calendar_credentials::Snapshot {
            access_token: Some("access-token".into()),
            refresh_token: Some("refresh-token".into()),
            access_token_expires_at: Some(
                (now(a).since(jiff::SignedDuration::from_hours(1))).jiff(),
            ),
        },
        a.booted.app.clock.now(),
    )
}
#[tokio::test]
async fn google_calendar_cleanup_deletes_before_revoke_and_does_not_revoke_after_transient_failure()
{
    let (a, r) = app().await;
    let blob = blob(&a);
    r.answer(404, json!({}));
    r.answer(200, json!({}));
    r.answer(400, json!({}));
    calendar::cleanup(
        &a.booted.app,
        vec!["gone".into(), "copy".into()],
        json!(blob.clone()),
        None,
    )
    .await
    .unwrap();
    let paths = r
        .calls
        .lock()
        .unwrap()
        .iter()
        .map(|c| c["path"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        paths,
        vec![
            "/calendar/v3/calendars/primary/events/gone",
            "/calendar/v3/calendars/primary/events/copy",
            "/revoke"
        ]
    );
    r.calls.lock().unwrap().clear();
    r.answer(429, json!({}));
    let e = calendar::cleanup(&a.booted.app, vec!["copy".into()], json!(blob), None)
        .await
        .unwrap_err();
    assert!(e.unavailable());
    assert_eq!(r.calls.lock().unwrap().len(), 1);
    assert!(r.answers.lock().unwrap().is_empty());
}
#[tokio::test]
async fn google_calendar_cleanup_skips_unreadable_blob_and_supports_legacy_snapshot() {
    let (a, r) = app().await;
    for value in [Value::Null, json!("tampered"), json!({})] {
        calendar::cleanup(&a.booted.app, vec!["copy".into()], value, Some(1))
            .await
            .unwrap();
    }
    assert!(r.calls.lock().unwrap().is_empty());
    r.answer(200, json!({}));
    let legacy = json!({"access_token":"access-token","refresh_token":"refresh-token","access_token_expires_at":now(&a).since(jiff::SignedDuration::from_hours(1)).jiff().to_string()});
    calendar::cleanup(&a.booted.app, vec![], legacy, None)
        .await
        .unwrap();
    assert_eq!(r.calls.lock().unwrap()[0]["path"], "/revoke");
}
#[tokio::test]
async fn google_calendar_watch_starts_before_stop_and_permanent_failure_keeps_old_channel() {
    let (a, r) = app().await;
    grant(&a).await;
    a.booted.app.google.install_api(Api::new(
        Config {
            webhook_url: Some("https://campfire.test/google/calendar/notifications".into()),
            ..support::config()
        },
        r.clone(),
    ));
    a.db()
        .write(|tx| {
            campfire_db::models::google_calendar::replace_watch(
                tx,
                DAVID,
                "old-channel",
                &PushChannel::digest("old-token"),
                Some("old-resource"),
                None,
            )
        })
        .await
        .unwrap();
    r.answer(
        200,
        json!({"resourceId":"new-resource","expiration":"1790880000123"}),
    );
    r.answer(204, Value::Null);
    calendar::watch(&a.booted.app, DAVID).await.unwrap();
    let calls = r.calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 2);
    assert!(
        calls[0]["path"]
            .as_str()
            .unwrap()
            .ends_with("/events/watch")
    );
    assert_eq!(calls[1]["path"], "/calendar/v3/channels/stop");
    let body: Value = serde_json::from_str(calls[0]["body"].as_str().unwrap()).unwrap();
    assert_eq!(body["type"], "web_hook");
    assert_eq!(body["token"].as_str().unwrap().len(), 64);
    let id = body["id"].as_str().unwrap().to_owned();
    let saved_id = id.clone();
    let saved = a
        .db()
        .read(move |c| PushChannel::for_channel(c, &saved_id))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        saved.token_digest,
        PushChannel::digest(body["token"].as_str().unwrap())
    );
    assert_ne!(saved.token_digest, body["token"]);
    r.calls.lock().unwrap().clear();
    r.answer(403, json!({"error":{"errors":[{"reason":"forbidden"}]}}));
    calendar::watch(&a.booted.app, DAVID).await.unwrap();
    assert_eq!(r.calls.lock().unwrap().len(), 1);
    assert!(
        a.db()
            .read(move |c| PushChannel::for_channel(c, &id))
            .await
            .unwrap()
            .is_some()
    );
}
#[tokio::test]
async fn google_calendar_remote_delete_executes_through_registered_durable_runner() {
    let (a, r) = app().await;
    grant(&a).await;
    r.answer(404, json!({}));
    a.db()
        .write(|tx| {
            tx.emit_after_commit(campfire_db::Event::job(&RemoteDeleteJob((
                DAVID,
                "orphan".into(),
            ))));
            Ok(())
        })
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5),async{loop{let done=a.db().read(|c|Ok(!c.query_row("SELECT EXISTS(SELECT 1 FROM background_jobs WHERE job_class='Calendar::RemoteDeleteJob')",[],|r|r.get::<_,bool>(0))?)).await.unwrap();if done && !r.calls.lock().unwrap().is_empty(){break;}tokio::task::yield_now().await;}}).await.unwrap();
    assert_eq!(
        r.calls.lock().unwrap()[0]["path"],
        "/calendar/v3/calendars/primary/events/orphan"
    );
}
#[tokio::test]
async fn google_calendar_refresh_persists_opted_in_sets_and_throttled_followup_is_atomic() {
    let (a, r) = app().await;
    grant(&a).await;
    a.db().write(|tx|{tx.conn().execute("UPDATE users SET meeting_status_enabled=1,ooo_calendar_enabled=1,time_zone='America/New_York' WHERE id=?",[DAVID])?;Ok(())}).await.unwrap();
    let clock = now(&a);
    let items = serde_json::from_str::<Value>(include_str!(
        "../../../../vectors/google_meeting_intervals.json"
    ))
    .unwrap();
    let busy = items["cases"][0]["items"].clone();
    r.answer(200, json!({"items":busy}));
    assert_eq!(
        meeting_refresh::refresh(&a.booted.app, DAVID, clock)
            .await
            .unwrap(),
        meeting_refresh::Result::Ok
    );
    let saved = a
        .db()
        .read(|c| cache::find(c, DAVID))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved.busy, items["cases"][0]["busy"]);
    assert_eq!(saved.ooo, json!([]));
    assert!(saved.fetch_error.is_none());
    let calls = r.calls.lock().unwrap().len();
    assert_eq!(
        meeting_refresh::refresh(&a.booted.app, DAVID, clock)
            .await
            .unwrap(),
        meeting_refresh::Result::Fresh
    );
    assert_eq!(
        meeting_refresh::refresh(&a.booted.app, DAVID, clock)
            .await
            .unwrap(),
        meeting_refresh::Result::Fresh
    );
    assert_eq!(r.calls.lock().unwrap().len(), calls);
    assert_eq!(a.db().read(|c|Ok(c.query_row("SELECT count(*) FROM background_jobs WHERE job_class='Calendar::MeetingRefreshJob'",[],|r|r.get::<_,i64>(0))?)).await.unwrap(),1);
    a.db().write(|tx|{tx.conn().execute("UPDATE calendar_meeting_caches SET refresh_pending_at=NULL WHERE user_id=?",[DAVID])?;tx.conn().execute_batch("CREATE TRIGGER reject_followup BEFORE INSERT ON background_jobs BEGIN SELECT RAISE(ABORT,'fixture rollback'); END;")?;Ok(())}).await.unwrap();
    assert!(
        meeting_refresh::refresh(&a.booted.app, DAVID, clock)
            .await
            .is_err()
    );
    assert!(
        a.db()
            .read(|c| cache::find(c, DAVID))
            .await
            .unwrap()
            .unwrap()
            .refresh_pending_at
            .is_none()
    );
}
#[tokio::test]
async fn google_calendar_refresh_transient_error_keeps_intervals_and_dead_grant_clears_them() {
    let (a, r) = app().await;
    grant(&a).await;
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE users SET meeting_status_enabled=1 WHERE id=?",
                [DAVID],
            )?;
            cache::complete(
                tx,
                DAVID,
                Some(json!([["2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z"]])),
                Some(json!([])),
                None,
                tx.now().ago(jiff::SignedDuration::from_secs(120)),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let clock = now(&a);
    r.answer(500, json!({}));
    assert_eq!(
        meeting_refresh::refresh(&a.booted.app, DAVID, clock)
            .await
            .unwrap(),
        meeting_refresh::Result::Error
    );
    let row = a
        .db()
        .read(|c| cache::find(c, DAVID))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.busy.as_array().unwrap().len(), 1);
    assert_eq!(
        row.fetch_error.as_deref(),
        Some(meeting_refresh::UNREACHABLE)
    );
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE google_accounts SET disconnected_reason='rejected' WHERE user_id=?",
                [DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    meeting_refresh::refresh(&a.booted.app, DAVID, clock)
        .await
        .unwrap();
    let row = a
        .db()
        .read(|c| cache::find(c, DAVID))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.busy, json!([]));
    assert_eq!(
        row.fetch_error.as_deref(),
        Some(meeting_refresh::NOT_CONNECTED)
    );
    assert!(row.refresh_pending_at.is_none());
}

#[tokio::test]
async fn google_calendar_meeting_refresh_executes_through_registered_durable_runner() {
    let (a, r) = app().await;
    grant(&a).await;
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE users SET meeting_status_enabled=1 WHERE id=?",
                [DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    r.answer(200, json!({"items":[]}));
    a.db()
        .write(|tx| {
            tx.emit_after_commit(campfire_db::Event::job(&MeetingRefreshJob {
                user_id: DAVID,
            }));
            Ok(())
        })
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let row = a.db().read(|c| cache::find(c, DAVID)).await.unwrap();
            if row.is_some_and(|c| c.fetched_at.is_some()) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(r.calls.lock().unwrap().len(), 1);
    assert!(
        r.calls.lock().unwrap()[0]["path"]
            .as_str()
            .unwrap()
            .contains("fields=items")
    );
}
async fn event(a: &TestApp) -> i64 {
    a.db().write(|tx|{tx.conn().execute("INSERT INTO events(room_id,organizer_id,title,starts_at,time_zone,created_at,updated_at) VALUES(? ,? ,'Party',?,'UTC',?,?)",rusqlite::params![crate::controllers::presenters::test_support::ALL_TALK,DAVID,tx.now().since(jiff::SignedDuration::from_hours(1)),tx.now(),tx.now()])?;let id=tx.conn().last_insert_rowid();tx.conn().execute("INSERT INTO event_attendances(event_id,user_id,response,created_at,updated_at) VALUES(?,?,'going',?,?)",rusqlite::params![id,DAVID,tx.now(),tx.now()])?;Ok(id)}).await.unwrap()
}
#[tokio::test]
async fn google_calendar_entry_sync_reserves_deterministic_id_and_converges_conflict_missing_and_decline()
 {
    use crate::integrations::google::entry_sync;
    use campfire_db::models::google_entry;
    let (a, r) = app().await;
    grant(&a).await;
    let event_id = event(&a).await;
    r.answer(409, json!({}));
    r.answer(200, json!({}));
    entry_sync::sync(&a.booted.app, event_id, DAVID)
        .await
        .unwrap();
    let entry = a
        .db()
        .read(move |c| google_entry::find(c, event_id, DAVID))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        entry.google_event_id,
        google_entry::google_id(event_id, DAVID)
    );
    assert!(entry.synced_at.is_some());
    let calls = r.calls.lock().unwrap().clone();
    assert_eq!(calls[0]["method"], "POST");
    assert_eq!(calls[1]["method"], "PUT");
    let body: Value = serde_json::from_str(calls[1]["body"].as_str().unwrap()).unwrap();
    assert_eq!(body["status"], "confirmed");
    r.calls.lock().unwrap().clear();
    r.answer(404, json!({}));
    r.answer(200, json!({}));
    entry_sync::sync(&a.booted.app, event_id, DAVID)
        .await
        .unwrap();
    assert_eq!(r.calls.lock().unwrap()[0]["method"], "PUT");
    assert_eq!(r.calls.lock().unwrap()[1]["method"], "POST");
    a.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE event_attendances SET response='declined' WHERE event_id=?",
                [event_id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    r.answer(410, json!({}));
    entry_sync::sync(&a.booted.app, event_id, DAVID)
        .await
        .unwrap();
    assert!(
        a.db()
            .read(move |c| google_entry::find(c, event_id, DAVID))
            .await
            .unwrap()
            .is_none()
    );
}
#[tokio::test]
async fn google_calendar_entry_sync_records_transient_failure_then_drops_unreachable_copy() {
    use crate::integrations::google::entry_sync;
    use campfire_db::models::google_entry;
    let (a, r) = app().await;
    grant(&a).await;
    let event_id = event(&a).await;
    r.answer(429, json!({}));
    assert!(
        entry_sync::sync(&a.booted.app, event_id, DAVID)
            .await
            .unwrap_err()
            .unavailable()
    );
    let row = a
        .db()
        .read(move |c| google_entry::find(c, event_id, DAVID))
        .await
        .unwrap()
        .unwrap();
    assert!(row.synced_at.is_none());
    let last: String = a
        .db()
        .read(move |c| {
            Ok(c.query_row(
                "SELECT last_error FROM event_calendar_entries WHERE event_id=? AND user_id=?",
                [event_id, DAVID],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(
        last,
        "RateLimited: Google Calendar request rate limited (429)"
    );
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE google_accounts SET disconnected_reason='rejected' WHERE user_id=?",
                [DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let calls = r.calls.lock().unwrap().len();
    entry_sync::sync(&a.booted.app, event_id, DAVID)
        .await
        .unwrap();
    assert!(
        a.db()
            .read(move |c| google_entry::find(c, event_id, DAVID))
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(r.calls.lock().unwrap().len(), calls);
}
