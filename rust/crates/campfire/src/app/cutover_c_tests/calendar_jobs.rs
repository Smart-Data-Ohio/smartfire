use super::*;
use crate::app::google_test_support::{QueueDrain, observe_jobs};
use campfire_db::{
    CalendarEvent, Event, Job, NewCalendarEvent, Room, RoomType,
    models::{
        calendar_event::changes::EventChanges, google_account::GoogleAccount,
        google_calendar::DisconnectCleanupJob, google_entry, room_delete::RemoteDeleteJob,
    },
};
use hyper::Method;
use serde_json::{Value, json};
use std::{sync::Mutex, time::Duration};
async fn setup() -> (TestApp, Arc<Recorded>) {
    let a = app().await;
    let r = Recorded::new(vec![]);
    google::install(&a, r.clone()).await;
    observe_jobs(&a).await;
    (a, r)
}
async fn account(a: &TestApp, expired: bool) {
    google::grant(
        a,
        DAVID,
        Timestamp::from_jiff(a.booted.app.clock.now()).since(jiff::SignedDuration::from_hours(
            if expired { -1 } else { 1 },
        )),
        false,
    )
    .await;
}
async fn snapshot(a: &TestApp) -> (String, i64) {
    let secrets = a.booted.app.secrets.clone();
    let now = a.booted.app.clock.now();
    a.db()
        .read(move |c| {
            let account = GoogleAccount::for_user(c, DAVID)?.unwrap();
            Ok((account.cleanup_snapshot(&secrets, now).unwrap(), account.id))
        })
        .await
        .unwrap()
}
async fn delete_account(a: &TestApp) {
    a.db()
        .write(|tx| {
            tx.conn()
                .execute("DELETE FROM google_accounts WHERE user_id=?", [DAVID])?;
            Ok(())
        })
        .await
        .unwrap();
}
async fn enqueue<J: Job + serde::Serialize + Send + 'static>(a: &TestApp, job: J) {
    a.db()
        .write(move |tx| {
            tx.emit_after_commit(Event::job(&job));
            Ok(())
        })
        .await
        .unwrap();
}
async fn attempt(a: &TestApp, class: &'static str, count: u32) -> Option<Timestamp> {
    let mut drain = QueueDrain::install(a).await;
    let runner = campfire_jobs::start(
        a.db().clone(),
        a.booted.app.jobs.queue.clone(),
        crate::jobs::registry(),
        a.booted.app.clone(),
        crate::jobs::runner_config(&a.booted.app.config),
    );
    let outcome = tokio::time::timeout(Duration::from_secs(5), drain.job_attempt(a, class, count))
        .await
        .expect("registered Calendar job did not commit its outcome");
    runner.shutdown(Duration::from_secs(5)).await;
    outcome
}
async fn emitted(a: &TestApp, class: &'static str) -> Vec<Value> {
    a.db()
        .read(move |c| {
            Ok(
                c.prepare(
                    "SELECT arguments FROM ws14g_emitted_jobs WHERE job_class=? ORDER BY id",
                )?
                .query_map([class], |r| r.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?
                .into_iter()
                .map(|s| serde_json::from_str(&s).unwrap())
                .collect(),
            )
        })
        .await
        .unwrap()
}
async fn emitted_all(a: &TestApp) -> Vec<(String, Value)> {
    a.db()
        .read(|c| {
            Ok(
                c.prepare("SELECT job_class,arguments FROM ws14g_emitted_jobs ORDER BY id")?
                    .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
                    .collect::<rusqlite::Result<Vec<_>>>()?
                    .into_iter()
                    .map(|(class, args)| (class, serde_json::from_str(&args).unwrap()))
                    .collect(),
            )
        })
        .await
        .unwrap()
}
async fn pending(a: &TestApp, class: &'static str) -> Vec<campfire_jobs::inspect::JobRow> {
    a.db()
        .read(move |c| {
            Ok(campfire_jobs::inspect::all(c)?
                .into_iter()
                .filter(|j| j.class == class)
                .collect())
        })
        .await
        .unwrap()
}
const CLEANUP: &str = "Calendar::DisconnectCleanupJob";
const REMOTE: &str = "Calendar::RemoteDeleteJob";
const SYNC: &str = "Calendar::SyncEntryJob";
const ORPHAN: &str = "/calendar/v3/calendars/primary/events/orphan-id";
fn calls(r: &Recorded) -> Vec<Value> {
    r.calls.lock().unwrap().clone()
}
#[tokio::test]
async fn cutover_c_cleanup_expired_snapshot_refreshes_before_delete_with_new_access_token() {
    let (a, r) = setup().await;
    account(&a, true).await;
    let (blob, _) = snapshot(&a).await;
    delete_account(&a).await;
    r.answer(
        200,
        json!({"access_token":"second-access-token","expires_in":3600}),
    );
    r.answer(200, json!({}));
    r.answer(200, json!({}));
    enqueue(
        &a,
        DisconnectCleanupJob((vec!["orphan-id".into()], json!(blob), None)),
    )
    .await;
    assert_eq!(attempt(&a, CLEANUP, 1).await, None);
    let c = calls(&r);
    assert_eq!(
        c.iter()
            .map(|v| (v["method"].as_str().unwrap(), v["path"].as_str().unwrap()))
            .collect::<Vec<_>>(),
        vec![("POST", "/token"), ("DELETE", ORPHAN), ("POST", "/revoke")]
    );
    assert!(
        url::form_urlencoded::parse(c[0]["body"].as_str().unwrap().as_bytes())
            .any(|(key, value)| key == "grant_type" && value == "refresh_token")
    );
    assert_eq!(c[1]["access_token"], "second-access-token");
    assert_eq!(c[2]["method"], "POST");
}
#[tokio::test]
async fn cutover_c_cleanup_invalid_grant_skips_deletes_but_revokes_snapshot_refresh_token() {
    let (a, r) = setup().await;
    account(&a, true).await;
    let (blob, _) = snapshot(&a).await;
    delete_account(&a).await;
    r.answer(400, json!({"error":"invalid_grant"}));
    r.answer(200, json!({}));
    enqueue(
        &a,
        DisconnectCleanupJob((vec!["orphan-id".into()], json!(blob), None)),
    )
    .await;
    assert_eq!(attempt(&a, CLEANUP, 1).await, None);
    let c = calls(&r);
    assert!(!c.iter().any(|v| v["method"] == "DELETE"));
    assert_eq!(
        c.iter()
            .map(|v| (v["method"].as_str().unwrap(), v["path"].as_str().unwrap()))
            .collect::<Vec<_>>(),
        vec![("POST", "/token"), ("POST", "/revoke")]
    );
}
#[tokio::test]
async fn cutover_c_cleanup_refresh_503_retries_without_deletes_or_revoke_then_recovers() {
    let (a, r) = setup().await;
    account(&a, true).await;
    let (blob, _) = snapshot(&a).await;
    delete_account(&a).await;
    let job = DisconnectCleanupJob((vec!["orphan-id".into()], json!(blob), None));
    let args = serde_json::to_value(&job).unwrap();
    r.answer(503, json!({}));
    enqueue(&a, job).await;
    assert!(attempt(&a, CLEANUP, 1).await.is_some());
    let retry = pending(&a, CLEANUP).await;
    assert_eq!(retry.len(), 1);
    assert_eq!(retry[0].arguments, args);
    assert_eq!(
        calls(&r)
            .iter()
            .map(|v| (v["method"].as_str().unwrap(), v["path"].as_str().unwrap()))
            .collect::<Vec<_>>(),
        vec![("POST", "/token")]
    );
    r.calls.lock().unwrap().clear();
    r.answer(
        200,
        json!({"access_token":"recovered-access-token","expires_in":3600}),
    );
    r.answer(200, json!({}));
    r.answer(200, json!({}));
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE background_jobs SET run_at=? WHERE job_class=?",
                rusqlite::params![tx.now(), CLEANUP],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(attempt(&a, CLEANUP, 2).await, None);
    let c = calls(&r);
    assert_eq!(
        (c[1]["method"].as_str(), c[1]["path"].as_str()),
        (Some("DELETE"), Some(ORPHAN))
    );
    assert_eq!(c[1]["access_token"], "recovered-access-token");
    assert_eq!(
        (c[2]["method"].as_str(), c[2]["path"].as_str()),
        (Some("POST"), Some("/revoke"))
    );
}
#[tokio::test]
async fn cutover_c_cleanup_revoke_500_keeps_same_durable_retry_payload() {
    let (a, r) = setup().await;
    account(&a, false).await;
    let (blob, _) = snapshot(&a).await;
    delete_account(&a).await;
    r.answer(200, json!({}));
    r.answer(500, json!({}));
    let job = DisconnectCleanupJob((vec!["orphan-id".into()], json!(blob), None));
    let args = serde_json::to_value(&job).unwrap();
    enqueue(&a, job).await;
    assert!(attempt(&a, CLEANUP, 1).await.is_some());
    let rows = pending(&a, CLEANUP).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].arguments, args);
    let c = calls(&r);
    assert_eq!(
        (c[0]["method"].as_str(), c[0]["path"].as_str()),
        (Some("DELETE"), Some(ORPHAN))
    );
    assert_eq!(
        (c[1]["method"].as_str(), c[1]["path"].as_str()),
        (Some("POST"), Some("/revoke"))
    );
}
#[derive(Clone, Default)]
struct Logs(Arc<Mutex<Vec<u8>>>);
impl std::io::Write for Logs {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Logs {
    type Writer = Self;
    fn make_writer(&'a self) -> Self {
        self.clone()
    }
}
impl Logs {
    fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}
fn log_guard(logs: &Logs) -> tracing::subscriber::DefaultGuard {
    tracing::subscriber::set_default(
        tracing_subscriber::fmt()
            .with_ansi(false)
            .without_time()
            .with_max_level(tracing::Level::DEBUG)
            .with_writer(logs.clone())
            .finish(),
    )
}
#[tokio::test]
async fn cutover_c_cleanup_exhausted_retry_logs_error_and_never_revokes() {
    let (a, r) = setup().await;
    account(&a, false).await;
    let (blob, _) = snapshot(&a).await;
    delete_account(&a).await;
    r.answer(429, json!({}));
    let job = DisconnectCleanupJob((vec!["orphan-id".into()], json!(blob), None));
    let args = serde_json::to_value(&job).unwrap();
    enqueue(&a, job).await;
    // The original Rails job has an exhausted exception_executions group before perform_now.
    // Set the same serialized runner group, leaving arguments and registered handler intact.
    a.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE background_jobs SET attempts=8,arguments=? WHERE job_class=?",
                rusqlite::params![
                    json!({
                        "_campfire_retry_metadata_v1": {
                            "arguments": args,
                            "counts": {"[Google::Client::Unavailable]": 8}
                        }
                    })
                    .to_string(),
                    CLEANUP
                ],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let logs = Logs::default();
    let _guard = log_guard(&logs);
    assert_eq!(attempt(&a, CLEANUP, 9).await, None);
    assert!(pending(&a, CLEANUP).await.is_empty());
    assert!(logs.text().lines().any(|line| line.contains("ERROR")
        && line.contains("Calendar::DisconnectCleanupJob failed after retries")));
    assert!(!calls(&r).iter().any(|c| c["path"] == "/revoke"));
}
#[tokio::test]
async fn cutover_c_cleanup_unreadable_blob_warns_with_account_and_without_tokens_or_google_http() {
    let (a, r) = setup().await;
    account(&a, false).await;
    let (mut blob, account_id) = snapshot(&a).await;
    let replacement = if blob.as_bytes()[10] == b'A' {
        "B"
    } else {
        "A"
    };
    blob.replace_range(10..11, replacement);
    delete_account(&a).await;
    let logs = Logs::default();
    let _guard = log_guard(&logs);
    enqueue(
        &a,
        DisconnectCleanupJob((vec!["orphan-id".into()], json!(blob), Some(account_id))),
    )
    .await;
    assert_eq!(attempt(&a, CLEANUP, 1).await, None);
    let log = logs.text();
    assert!(
        log.lines()
            .any(|line| line.contains("WARN") && line.contains(&format!("account {account_id}")))
    );
    assert!(!log.contains("access-token"));
    assert!(!log.contains("refresh-token"));
    assert!(calls(&r).is_empty());
}
#[tokio::test]
async fn cutover_c_cleanup_encrypted_credentials_and_job_logs_never_contain_plaintext_tokens() {
    let (a, r) = setup().await;
    account(&a, false).await;
    let (blob, _) = snapshot(&a).await;
    delete_account(&a).await;
    assert!(!blob.contains("access-token"));
    assert!(!blob.contains("refresh-token"));
    r.answer(200, json!({}));
    r.answer(200, json!({}));
    let logs = Logs::default();
    let _guard = log_guard(&logs);
    enqueue(
        &a,
        DisconnectCleanupJob((vec!["orphan-id".into()], json!(blob.clone()), None)),
    )
    .await;
    assert_eq!(attempt(&a, CLEANUP, 1).await, None);
    assert!(!logs.text().contains(&blob));
    assert!(!logs.text().contains("access-token"));
    assert!(!logs.text().contains("refresh-token"));
}
async fn remote(a: &TestApp) -> Option<Timestamp> {
    enqueue(a, RemoteDeleteJob((DAVID, "orphan-id".into()))).await;
    attempt(a, REMOTE, 1).await
}
#[tokio::test]
async fn cutover_c_remote_delete_uses_current_account_access_credentials() {
    let (a, r) = setup().await;
    account(&a, false).await;
    r.answer(200, json!({}));
    assert_eq!(remote(&a).await, None);
    assert_eq!(calls(&r).len(), 1);
    assert_eq!(
        (
            calls(&r)[0]["method"].as_str(),
            calls(&r)[0]["path"].as_str()
        ),
        (Some("DELETE"), Some(ORPHAN))
    );
    assert_eq!(calls(&r)[0]["access_token"], "access-token");
}
#[tokio::test]
async fn cutover_c_remote_delete_without_account_makes_no_request() {
    let (a, r) = setup().await;
    assert_eq!(remote(&a).await, None);
    assert!(calls(&r).is_empty());
}
#[tokio::test]
async fn cutover_c_remote_delete_disconnected_account_makes_no_request() {
    let (a, r) = setup().await;
    account(&a, false).await;
    a.db()
        .write(|tx| {
            GoogleAccount::for_user(tx.conn(), DAVID)?
                .unwrap()
                .mark_disconnected(tx, "Google rejected the connection")
        })
        .await
        .unwrap();
    assert_eq!(remote(&a).await, None);
    assert!(calls(&r).is_empty());
}
#[tokio::test]
async fn cutover_c_remote_delete_without_calendar_scope_makes_no_request() {
    let (a, r) = setup().await;
    account(&a, false).await;
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
    assert_eq!(remote(&a).await, None);
    assert!(calls(&r).is_empty());
}
#[tokio::test]
async fn cutover_c_remote_delete_not_found_and_gone_complete_without_retry() {
    let (a, r) = setup().await;
    account(&a, false).await;
    for (status, key) in [(404, "missing-id"), (410, "gone-id")] {
        r.answer(status, json!({}));
        enqueue(&a, RemoteDeleteJob((DAVID, key.into()))).await;
        assert_eq!(attempt(&a, REMOTE, 1).await, None);
    }
    let c = calls(&r);
    assert_eq!(c.len(), 2);
    assert_eq!(
        (c[0]["method"].as_str(), c[0]["path"].as_str()),
        (
            Some("DELETE"),
            Some("/calendar/v3/calendars/primary/events/missing-id")
        )
    );
    assert_eq!(
        (c[1]["method"].as_str(), c[1]["path"].as_str()),
        (
            Some("DELETE"),
            Some("/calendar/v3/calendars/primary/events/gone-id")
        )
    );
}
#[tokio::test]
async fn cutover_c_remote_delete_invalid_grant_disconnects_without_delete_or_retry() {
    let (a, r) = setup().await;
    account(&a, true).await;
    r.answer(400, json!({"error":"invalid_grant"}));
    assert_eq!(remote(&a).await, None);
    assert_eq!(
        a.db()
            .read(|c| GoogleAccount::for_user(c, DAVID))
            .await
            .unwrap()
            .unwrap()
            .disconnected_reason
            .as_deref(),
        Some("Google rejected the connection")
    );
    assert!(!calls(&r).iter().any(|c| c["method"] == "DELETE"));
}
#[tokio::test]
async fn cutover_c_remote_delete_transport_failure_schedules_same_payload_retry() {
    let (a, r) = setup().await;
    account(&a, false).await;
    r.fail_next();
    assert!(remote(&a).await.is_some());
    let rows = pending(&a, REMOTE).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].arguments, json!([DAVID, "orphan-id"]));
}
#[tokio::test]
async fn cutover_c_remote_delete_permanent_google_failure_logs_and_does_not_retry() {
    let (a, r) = setup().await;
    account(&a, false).await;
    r.answer(500, json!({}));
    let logs = Logs::default();
    let _guard = log_guard(&logs);
    assert_eq!(remote(&a).await, None);
    assert!(pending(&a, REMOTE).await.is_empty());
    assert_eq!(emitted(&a, REMOTE).await.len(), 1);
    assert!(logs.text().contains("Calendar::RemoteDeleteJob"));
}
fn event_id() -> i64 {
    id("launch_party")
}
async fn clear_observed(a: &TestApp) {
    a.db()
        .write(|tx| {
            tx.conn().execute("DELETE FROM ws14g_emitted_jobs", [])?;
            Ok(())
        })
        .await
        .unwrap();
}
async fn entries(a: &TestApp, user: i64, event_ids: Vec<i64>) -> Vec<google_entry::Entry> {
    a.db()
        .read(move |c| {
            // Rails: EventCalendarEntry.where(user: @jason, event: occurrences).
            event_ids
                .into_iter()
                .map(|eid| google_entry::find(c, eid, user))
                .collect::<campfire_db::Result<Vec<_>>>()
                .map(|rows| rows.into_iter().flatten().collect())
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn cutover_c_series_head_rsvp_commits_three_sync_jobs_and_three_distinct_google_entries() {
    let (a, r) = setup().await;
    let jason = id("jason");
    google::grant(
        &a,
        jason,
        Timestamp::from_jiff(a.booted.app.clock.now()).since(jiff::SignedDuration::from_hours(1)),
        false,
    )
    .await;
    let head = a
        .db()
        .write(|tx| {
            CalendarEvent::create(
                tx,
                NewCalendarEvent {
                    room_id: id("designers"),
                    organizer_id: DAVID,
                    title: "Daily sync".into(),
                    starts_at: Some(tx.now().since(jiff::SignedDuration::from_hours(48))),
                    time_zone: "UTC".into(),
                    recurrence_rule: Some("daily".into()),
                    recurrence_until: Some(
                        tx.now()
                            .jiff()
                            .to_zoned(jiff::tz::TimeZone::UTC)
                            .date()
                            .checked_add(jiff::Span::new().days(4))
                            .unwrap(),
                    ),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let hid = head.id;
    let occurrences = a
        .db()
        .read(move |c| CalendarEvent::find(c, hid)?.series_events(c))
        .await
        .unwrap();
    assert_eq!(occurrences.len(), 3);
    // Drain creation's organizer work with no Google grant, then measure just RSVP.
    let mut drain = QueueDrain::install(&a).await;
    let runner = campfire_jobs::start(
        a.db().clone(),
        a.booted.app.jobs.queue.clone(),
        crate::jobs::registry(),
        a.booted.app.clone(),
        crate::jobs::runner_config(&a.booted.app.config),
    );
    tokio::time::timeout(Duration::from_secs(5), drain.calendar(&a))
        .await
        .unwrap();
    runner.shutdown(Duration::from_secs(5)).await;
    clear_observed(&a).await;
    for _ in 0..3 {
        r.answer_for(
            Method::POST,
            "/calendar/v3/calendars/primary/events",
            200,
            json!({}),
        );
    }
    let hid = head.id;
    a.db()
        .write(move |tx| CalendarEvent::respond(tx, hid, jason, "going", false))
        .await
        .unwrap();
    assert_eq!(emitted(&a, SYNC).await.len(), 3);
    let mut drain = QueueDrain::install(&a).await;
    let runner = campfire_jobs::start(
        a.db().clone(),
        a.booted.app.jobs.queue.clone(),
        crate::jobs::registry(),
        a.booted.app.clone(),
        crate::jobs::runner_config(&a.booted.app.config),
    );
    tokio::time::timeout(Duration::from_secs(5), drain.calendar(&a))
        .await
        .unwrap();
    runner.shutdown(Duration::from_secs(5)).await;
    assert_eq!(
        calls(&r)
            .iter()
            .filter(
                |c| c["method"] == "POST" && c["path"] == "/calendar/v3/calendars/primary/events"
            )
            .count(),
        3
    );
    let occurrence_ids = occurrences.iter().map(|e| e.id).collect::<Vec<_>>();
    let rows = entries(&a, jason, occurrence_ids.clone()).await;
    assert_eq!(rows.len(), 3);
    assert_eq!(
        rows.iter()
            .map(|e| e.event_id)
            .collect::<std::collections::BTreeSet<_>>(),
        occurrence_ids
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>()
    );
    let ids = rows
        .iter()
        .map(|e| e.google_event_id.clone())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(ids.len(), 3);
}
#[tokio::test]
async fn cutover_c_time_update_enqueues_only_connected_notifying_attendees() {
    let (a, _) = setup().await;
    account(&a, false).await;
    google::grant(
        &a,
        id("jason"),
        Timestamp::from_jiff(a.booted.app.clock.now()).since(jiff::SignedDuration::from_hours(1)),
        false,
    )
    .await;
    a.db()
        .write(|tx| {
            GoogleAccount::for_user(tx.conn(), id("jason"))?
                .unwrap()
                .mark_disconnected(tx, "Google rejected the connection")
        })
        .await
        .unwrap();
    clear_observed(&a).await;
    a.db()
        .write(|tx| {
            let e = CalendarEvent::find(tx.conn(), event_id())?;
            CalendarEvent::update_with_scope(
                tx,
                e.id,
                EventChanges {
                    starts_at: Some(Some(e.starts_at.since(jiff::SignedDuration::from_mins(30)))),
                    ..Default::default()
                },
                "this_event",
                Some(id("jason")),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let jobs = emitted(&a, SYNC).await;
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0], json!({"event_id":event_id(),"user_id":DAVID}));
}
#[tokio::test]
async fn cutover_c_event_sync_is_invisible_to_queue_readers_until_source_transaction_commits() {
    let (a, _) = setup().await;
    account(&a, false).await;
    let reader = a.db().clone();
    a.db()
        .write(move |tx| {
            CalendarEvent::update_with_scope(
                tx,
                event_id(),
                EventChanges {
                    title: Some("Launch party planning v2".into()),
                    ..Default::default()
                },
                "this_event",
                Some(DAVID),
            )?;
            let visible = reader.read_blocking(|c| {
                Ok(c.query_row(
                    "SELECT COUNT(*) FROM background_jobs WHERE job_class=?",
                    [SYNC],
                    |r| r.get::<_, i64>(0),
                )?)
            })?;
            assert_eq!(visible, 0);
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        emitted(&a, SYNC).await,
        vec![json!({"event_id":event_id(),"user_id":DAVID})]
    );
}
#[tokio::test]
async fn cutover_c_setting_and_clearing_venue_each_enqueue_connected_attendee_sync() {
    let (a, _) = setup().await;
    let voice = a
        .db()
        .write(|tx| Room::create_for(tx, RoomType::Voice, Some("Lounge"), DAVID, &[DAVID]))
        .await
        .unwrap();
    account(&a, false).await;
    for venue in [Some(voice.id), None] {
        clear_observed(&a).await;
        a.db()
            .write(move |tx| {
                CalendarEvent::update_with_scope(
                    tx,
                    event_id(),
                    EventChanges {
                        venue_room_id: Some(venue),
                        ..Default::default()
                    },
                    "this_event",
                    Some(DAVID),
                )?;
                Ok(())
            })
            .await
            .unwrap();
        assert_eq!(
            emitted(&a, SYNC).await,
            vec![json!({"event_id":event_id(),"user_id":DAVID})]
        );
    }
}
#[tokio::test]
async fn cutover_c_title_change_enqueues_sync_and_unchanged_save_does_not() {
    let (a, _) = setup().await;
    account(&a, false).await;
    a.db()
        .write(|tx| {
            CalendarEvent::update_with_scope(
                tx,
                event_id(),
                EventChanges {
                    title: Some("Launch party planning v2".into()),
                    ..Default::default()
                },
                "this_event",
                Some(DAVID),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        emitted(&a, SYNC).await,
        vec![json!({"event_id":event_id(),"user_id":DAVID})]
    );
    clear_observed(&a).await;
    a.db()
        .write(|tx| {
            CalendarEvent::update_with_scope(
                tx,
                event_id(),
                EventChanges {
                    title: Some("Launch party planning v2".into()),
                    ..Default::default()
                },
                "this_event",
                Some(DAVID),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    // Rails' unchanged-save assertion has no `only:` job-class filter.
    assert_eq!(emitted_all(&a).await, vec![]);
}
#[tokio::test]
async fn cutover_c_room_destroy_commits_remote_delete_identity_and_real_google_request() {
    let (a, r) = setup().await;
    account(&a, false).await;
    a.db()
        .write(|tx| {
            let entry = google_entry::reserve(tx, event_id(), DAVID)?;
            tx.conn().execute(
                "UPDATE event_calendar_entries SET google_event_id='room-destroy-id' WHERE id=?",
                [entry.id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    r.answer_for(
        Method::DELETE,
        "/calendar/v3/calendars/primary/events/room-destroy-id",
        200,
        json!({}),
    );
    a.db()
        .write(|tx| Room::find(tx.conn(), id("designers"))?.destroy(tx))
        .await
        .unwrap();
    assert_eq!(
        emitted(&a, REMOTE).await,
        vec![json!([DAVID, "room-destroy-id"])]
    );
    assert_eq!(attempt(&a, REMOTE, 1).await, None);
    assert!(calls(&r).iter().any(|c| c["method"] == "DELETE"
        && c["path"] == "/calendar/v3/calendars/primary/events/room-destroy-id"));
}
#[tokio::test]
async fn cutover_c_series_shrink_commits_doomed_remote_identity_and_deletes_google_copy() {
    let (a, r) = setup().await;
    let head = a
        .db()
        .write(|tx| {
            CalendarEvent::create(
                tx,
                NewCalendarEvent {
                    room_id: id("designers"),
                    organizer_id: DAVID,
                    title: "Daily sync".into(),
                    starts_at: Some(tx.now().since(jiff::SignedDuration::from_hours(48))),
                    time_zone: "UTC".into(),
                    recurrence_rule: Some("daily".into()),
                    recurrence_until: Some(
                        tx.now()
                            .jiff()
                            .to_zoned(jiff::tz::TimeZone::UTC)
                            .date()
                            .checked_add(jiff::Span::new().days(4))
                            .unwrap(),
                    ),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let head_id = head.id;
    let doomed = a
        .db()
        .read(move |c| {
            Ok(CalendarEvent::find(c, head_id)?
                .series_events(c)?
                .last()
                .unwrap()
                .id)
        })
        .await
        .unwrap();
    account(&a, false).await;
    a.db()
        .write(move |tx| {
            let entry = google_entry::reserve(tx, doomed, DAVID)?;
            tx.conn().execute(
                "UPDATE event_calendar_entries SET google_event_id='doomed-entry' WHERE id=?",
                [entry.id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    r.answer_for(
        Method::DELETE,
        "/calendar/v3/calendars/primary/events/doomed-entry",
        200,
        json!({}),
    );
    let hid = head.id;
    a.db()
        .write(move |tx| {
            CalendarEvent::update_with_scope(
                tx,
                hid,
                EventChanges {
                    recurrence_until: Some(Some(
                        tx.now()
                            .jiff()
                            .to_zoned(jiff::tz::TimeZone::UTC)
                            .date()
                            .checked_add(jiff::Span::new().days(3))
                            .unwrap(),
                    )),
                    ..Default::default()
                },
                "this_and_following",
                Some(DAVID),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        emitted(&a, REMOTE).await,
        vec![json!([DAVID, "doomed-entry"])]
    ); // Only remote-delete work is selected for the original perform_enqueued_jobs assertion.
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE background_jobs SET run_at=? WHERE job_class!=?",
                rusqlite::params![tx.now().since(jiff::SignedDuration::from_hours(1)), REMOTE],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(attempt(&a, REMOTE, 1).await, None);
    assert!(calls(&r).iter().any(|c| c["method"] == "DELETE"
        && c["path"] == "/calendar/v3/calendars/primary/events/doomed-entry"));
}
