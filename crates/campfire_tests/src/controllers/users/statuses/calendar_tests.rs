use super::*;
use campfire_db::models::calendar_dispatch::{dispatch_meetings, dispatch_ooo};
use rusqlite::trace::{TraceEvent, TraceEventCodes};
use std::cell::RefCell;

thread_local! {
    static UPDATE_STATEMENTS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn observe_updates(event: TraceEvent<'_>) {
    // Like Rails' sql.active_record subscriber, count statements even when the
    // conditional UPDATE matches zero rows and still acquires the writer lock.
    if let TraceEvent::Stmt(_, sql) = event
        && sql.trim_start().starts_with("UPDATE")
    {
        UPDATE_STATEMENTS.with(|statements| statements.borrow_mut().push(sql.to_owned()));
    }
}

fn vectors() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../../vectors/ws17_calendar_dispatch.json"
    ))
    .unwrap()
}
fn stamp(s: &str) -> Timestamp {
    Timestamp::from_jiff(s.parse().unwrap())
}
async fn setup(app: &TestApp, row: Value) {
    app.db().write(move|tx|{
  tx.conn().execute("UPDATE users SET meeting_status_enabled=0,ooo_calendar_enabled=0,ooo_until=NULL",[])?;
  tx.conn().execute("DELETE FROM workspace_presence_leases",[])?;
  tx.conn().execute("DELETE FROM calendar_meeting_caches WHERE user_id=?",[DAVID])?;
  tx.conn().execute("UPDATE users SET status=0,meeting_status_enabled=0,meeting_dnd_enabled=0,ooo_calendar_enabled=0,ooo_until=NULL,ooo_note=NULL,ooo_broadcast=NULL,ooo_notify_enabled=0,presence_setting='auto',custom_status_text=NULL,custom_status_emoji=NULL,time_zone='UTC',dnd_enabled=0 WHERE id=?",[DAVID])?;
  for(key,value)in row["attrs"].as_object().into_iter().flatten(){
   let value=match value {Value::Bool(b)=>SqlValue::Integer(i64::from(*b)),Value::Number(n)=>SqlValue::Integer(n.as_i64().unwrap()),Value::String(s) if key=="ooo_until"=>SqlValue::Text(stamp(s).to_db()),Value::String(s)=>SqlValue::Text(s.clone()),Value::Null=>SqlValue::Null,_=>panic!("{value}")};
   tx.conn().execute(&format!("UPDATE users SET {key}=? WHERE id=?"),rusqlite::params![value,DAVID])?;
  }
  if row["missing"]!=true {
   let at=match row.get("fetched_at"){Some(Value::Null)=>None,Some(Value::String(s))=>Some(stamp(s)),None=>Some(tx.now()),_=>panic!("bad timestamp")};
   tx.conn().execute("INSERT INTO calendar_meeting_caches(user_id,busy_intervals,ooo_intervals,fetched_at,in_meeting_broadcast,created_at,updated_at) VALUES (?,?,?,?,?,?,?)",rusqlite::params![DAVID,row.get("busy").unwrap_or(&json!([])).to_string(),row.get("ooo").unwrap_or(&json!([])).to_string(),at,row["claimed"].as_bool(),tx.now(),tx.now()])?;
  }
  Ok(())
 }).await.unwrap();
}
async fn assert_frames(app: &TestApp, socket: &mut Client, frames: &[Value], name: &str) {
    let mut expected =
        std::collections::BTreeMap::<String, std::collections::VecDeque<&Value>>::new();
    for frame in frames {
        let stream = frame["stream"]
            .as_str()
            .unwrap()
            .split(':')
            .collect::<Vec<_>>();
        expected
            .entry(rails_compat::turbo::signed_stream_name(
                &app.booted.app.secrets,
                &stream,
            ))
            .or_default()
            .push_back(frame);
    }
    for _ in frames {
        let actual: Value = serde_json::from_str(&socket.next_text().await).unwrap();
        let channel: Value = serde_json::from_str(actual["identifier"].as_str().unwrap()).unwrap();
        assert_eq!(channel["channel"], "Turbo::StreamsChannel");
        let expected = expected
            .get_mut(channel["signed_stream_name"].as_str().unwrap())
            .expect("exact Rails stream")
            .pop_front()
            .expect("no duplicate frames");
        assert_eq!(
            actual["message"], expected["html"],
            "{name}: complete frame"
        );
    }
    assert!(expected.values().all(|queue| queue.is_empty()));
    socket.assert_silent().await;
}
async fn replay(names: &[&str]) {
    let golden = vectors();
    let now = stamp(golden["now"].as_str().unwrap());
    let app = boot().await.without_job_runner().await;
    let (_server, mut socket) = subscribe(&app).await;
    for name in names {
        let row = golden["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == *name)
            .unwrap()
            .clone();
        setup(&app, row.clone()).await;
        for run in row["runs"].as_array().unwrap() {
            app.db()
                .write(|tx| {
                    tx.conn().execute(
                        "DELETE FROM background_jobs WHERE job_class='Calendar::MeetingRefreshJob'",
                        [],
                    )?;
                    UPDATE_STATEMENTS.with(|statements| statements.borrow_mut().clear());
                    tx.conn()
                        .trace_v2(TraceEventCodes::SQLITE_TRACE_STMT, Some(observe_updates));
                    Ok(())
                })
                .await
                .unwrap();
            if matches!(row["kind"].as_str(), Some("meeting" | "both")) {
                assert!(
                    dispatch_meetings(app.db(), now)
                        .await
                        .unwrap()
                        .failed_user_ids
                        .is_empty()
                );
            }
            if matches!(row["kind"].as_str(), Some("ooo" | "both")) {
                assert!(
                    dispatch_ooo(app.db(), now)
                        .await
                        .unwrap()
                        .failed_user_ids
                        .is_empty()
                );
            }
            let updates = app
                .db()
                .write(|tx| {
                    tx.conn().trace_v2(TraceEventCodes::empty(), None);
                    Ok(UPDATE_STATEMENTS.with(|statements| statements.take()))
                })
                .await
                .unwrap();
            assert_eq!(
                json!(updates.len()),
                run["updates"],
                "{name} UPDATE statements: {updates:?}"
            );
            let current = settings(&app).await;
            assert_eq!(
                json!(current.ooo_broadcast),
                run["stored"]["ooo_broadcast"],
                "{name}"
            );
            assert_eq!(json!(current.ooo_note), run["stored"]["ooo_note"], "{name}");
            assert_eq!(
                current.ooo_until,
                run["stored"]["ooo_until"].as_str().map(stamp),
                "{name}"
            );
            assert_eq!(
                json!(current.meeting_cache.and_then(|c| c.in_meeting_broadcast)),
                run["meeting_claim"],
                "{name}"
            );
            let jobs:Vec<Value>=app.db().read(|conn|{
    let mut q=conn.prepare("SELECT arguments FROM background_jobs WHERE job_class='Calendar::MeetingRefreshJob' ORDER BY id")?;
    Ok(q.query_map([],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?.iter().map(|s|json!({"class":"Calendar::MeetingRefreshJob","args":[serde_json::from_str::<Value>(s).unwrap()["user_id"]]})).collect())
   }).await.unwrap();
            assert_eq!(json!(jobs), run["jobs"], "{name} durable jobs");
            assert_frames(&app, &mut socket, run["frames"].as_array().unwrap(), name).await;
        }
    }
}
macro_rules! calendar_scenario {($name:ident,$($case:literal),+) => {#[tokio::test]async fn $name(){replay(&[$($case),+]).await;}};}
calendar_scenario!(ws17_meeting_start_broadcasts_badge, "meeting_start");
calendar_scenario!(ws17_meeting_rerun_broadcasts_nothing, "meeting_steady");
calendar_scenario!(ws17_meeting_steady_tick_has_no_claim_write, "meeting_start");
calendar_scenario!(ws17_meeting_end_broadcasts_badge, "meeting_end");
calendar_scenario!(ws17_meeting_broadcast_carries_label, "meeting_start");
calendar_scenario!(
    ws17_meeting_stale_cache_enqueues_refresh,
    "meeting_stale",
    "meeting_stale_boundary",
    "meeting_unfetched"
);
calendar_scenario!(
    ws17_meeting_missing_cache_refreshes_without_broadcast,
    "meeting_missing"
);
calendar_scenario!(ws17_meeting_fresh_cache_does_not_refresh, "meeting_fresh");
calendar_scenario!(ws17_meeting_opted_out_members_ignored, "meeting_off");
calendar_scenario!(ws17_meeting_deactivated_members_ignored, "meeting_inactive");
calendar_scenario!(
    ws17_manual_ooo_start_broadcasts_badge_and_notice,
    "ooo_manual"
);
calendar_scenario!(ws17_ooo_rerun_broadcasts_nothing, "ooo_steady");
calendar_scenario!(ws17_ooo_steady_tick_has_no_claim_write, "ooo_manual");
calendar_scenario!(
    ws17_ooo_end_broadcasts_and_clears_columns,
    "ooo_end",
    "ooo_expired_already_false"
);
#[tokio::test]
async fn ws17_ooo_broadcast_contains_label_note_return_date() {
    // test/models/calendar/ooo_dispatcher_test.rb:52-72 uses this note/date
    // through dispatch_due!, rather than a detached badge or notice presenter.
    let now = stamp("2026-09-20T12:00:00Z");
    let app = TestApp::boot_without_periodic_with_clock(std::sync::Arc::new(
        campfire_kit::FrozenClock::new(now.jiff()),
    ))
    .await
    .unwrap()
    .without_job_runner()
    .await;
    setup(
        &app,
        json!({"missing":true,"attrs":{"ooo_until":"2026-09-24T12:00:00Z","ooo_note":"Back soon"}}),
    )
    .await;
    let browser = app.sign_in(DAVID).await;
    let (_server, mut socket) = subscribe_with_cookie(&app, &browser.cookie_header()).await;
    assert_eq!(dispatch_ooo(app.db(), now).await.unwrap().flipped, 1);
    let mut frames = std::collections::BTreeMap::new();
    for _ in 0..2 {
        let actual: Value = serde_json::from_str(&socket.next_text().await).unwrap();
        let channel: Value = serde_json::from_str(actual["identifier"].as_str().unwrap()).unwrap();
        assert_eq!(channel["channel"], "Turbo::StreamsChannel");
        assert!(
            frames
                .insert(
                    channel["signed_stream_name"].as_str().unwrap().to_owned(),
                    actual["message"].as_str().unwrap().to_owned(),
                )
                .is_none(),
            "one frame per stream"
        );
    }
    let stream = |suffix| {
        rails_compat::turbo::signed_stream_name(
            &app.booted.app.secrets,
            &[&user_gid(DAVID).to_param(), suffix],
        )
    };
    let badge = &frames[&stream("status")];
    assert!(badge.contains("Out of office"));
    assert!(badge.contains("Back soon"));
    let notice = &frames[&stream("ooo_notice")];
    assert!(notice.contains("is out of office until September 24, 2026"));
    assert!(notice.contains("Back soon"));
    socket.assert_silent().await;
    replay(&["ooo_manual", "ooo_invisible"]).await;
}
calendar_scenario!(
    ws17_calendar_ooo_start_broadcasts_badge_and_notice,
    "ooo_calendar"
);
calendar_scenario!(
    ws17_stale_ooo_only_cache_enqueues_refresh,
    "ooo_stale",
    "ooo_unfetched"
);
calendar_scenario!(
    ws17_ooo_missing_cache_refreshes_without_broadcast,
    "ooo_missing",
    "ooo_missing_previous_true"
);
calendar_scenario!(ws17_ooo_members_without_manual_or_optin_ignored, "ooo_off");
calendar_scenario!(ws17_ooo_deactivated_members_ignored, "ooo_inactive");
calendar_scenario!(
    ws17_calendar_dispatch_malformed_intervals_ignored,
    "meeting_malformed",
    "ooo_calendar_malformed"
);

#[tokio::test]
async fn ws17_both_optins_refresh_through_meeting_dispatcher_only() {
    let app = boot().await.without_job_runner().await;
    let golden = vectors();
    let row = golden["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "ooo_both")
        .unwrap()
        .clone();
    setup(&app, row).await;
    let now = stamp(golden["now"].as_str().unwrap());
    assert_eq!(dispatch_ooo(app.db(), now).await.unwrap().refreshed, 0);
    let count = app.db().read(|conn| {
        Ok(conn.query_row("SELECT count(*) FROM background_jobs WHERE job_class='Calendar::MeetingRefreshJob'", [], |r| r.get::<_,i64>(0))?)
    }).await.unwrap();
    assert_eq!(count, 0);
    assert_eq!(dispatch_meetings(app.db(), now).await.unwrap().refreshed, 1);
    let jobs = app.db().read(|conn| {
        Ok(conn.prepare("SELECT arguments FROM background_jobs WHERE job_class='Calendar::MeetingRefreshJob' ORDER BY id")?
            .query_map([], |r| r.get::<_,String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?
            .iter().map(|args| serde_json::from_str::<Value>(args).unwrap()).collect::<Vec<_>>())
    }).await.unwrap();
    assert_eq!(jobs, vec![json!({"user_id":DAVID})]);
}

async fn failed_member(kind: &str) {
    use crate::controllers::presenters::test_support::JASON;
    let app = boot().await.without_job_runner().await;
    let golden = vectors();
    let row = golden["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| {
            r["name"]
                == if kind == "meeting" {
                    "meeting_stale"
                } else {
                    "ooo_stale"
                }
        })
        .unwrap()
        .clone();
    setup(&app, row).await;
    let meeting = kind == "meeting";
    let now = stamp(golden["now"].as_str().unwrap());
    app.db().write(move|tx|{
  let jason:i64=tx.conn().query_row("SELECT id FROM users WHERE email_address='jason@37signals.com'",[],|r|r.get(0))?;assert_eq!(jason,JASON);
  tx.conn().execute("DELETE FROM calendar_meeting_caches WHERE user_id=?",[JASON])?;
  if meeting {
   tx.conn().execute("UPDATE users SET meeting_status_enabled=1,ooo_broadcast=NULL WHERE id=?",[JASON])?;
   tx.conn().execute("INSERT INTO calendar_meeting_caches(user_id,fetched_at,busy_intervals,created_at,updated_at) VALUES (?,?,'[[\"2026-03-02T15:55:00Z\",\"2026-03-02T16:55:00Z\"]]',?,?)",rusqlite::params![JASON,now,now,now])?;
   tx.conn().execute("UPDATE calendar_meeting_caches SET busy_intervals='[[\"2026-03-02T15:55:00Z\",\"2026-03-02T16:55:00Z\"]]' WHERE user_id=?",[DAVID])?;
  } else {
   tx.conn().execute("UPDATE users SET ooo_until=?,ooo_broadcast=NULL WHERE id=?",rusqlite::params![now.since(jiff::SignedDuration::from_hours(1)),JASON])?;
   tx.conn().execute("UPDATE calendar_meeting_caches SET ooo_intervals='[[\"2026-03-02T15:55:00Z\",\"2026-03-04T16:00:00Z\"]]' WHERE user_id=?",[DAVID])?;
  }
  tx.conn().execute_batch(&format!("CREATE TRIGGER ws17_reject_calendar_refresh BEFORE INSERT ON background_jobs WHEN NEW.job_class='Calendar::MeetingRefreshJob' AND json_extract(NEW.arguments,'$.user_id')={DAVID} BEGIN SELECT RAISE(ABORT,'WS17 refresh rejected'); END"))?;Ok(())
 }).await.unwrap();
    let jason = app
        .db()
        .read(|conn| UserStatusSettings::find(conn, JASON))
        .await
        .unwrap();
    assert_eq!(jason.user.status, campfire_db::Status::Active);
    let (_server, mut socket) = subscribe(&app).await;
    let signed = rails_compat::turbo::signed_stream_name(
        &app.booted.app.secrets,
        &[&user_gid(JASON).to_param(), "status"],
    );
    socket
        .confirm(&identifier(
            json!({"channel":"Turbo::StreamsChannel","signed_stream_name":signed}),
        ))
        .await;
    if !meeting {
        let signed = rails_compat::turbo::signed_stream_name(
            &app.booted.app.secrets,
            &[&user_gid(JASON).to_param(), "ooo_notice"],
        );
        socket
            .confirm(&identifier(
                json!({"channel":"Turbo::StreamsChannel","signed_stream_name":signed}),
            ))
            .await;
    }
    let stats = if meeting {
        dispatch_meetings(app.db(), now).await
    } else {
        dispatch_ooo(app.db(), now).await
    }
    .unwrap();
    assert_eq!(stats.failed_user_ids, vec![DAVID]);
    assert_eq!(stats.flipped, 1, "{stats:?}");
    assert_eq!(stats.refreshed, 0);
    let mut expected_streams = if meeting {
        vec!["status"]
    } else {
        vec!["status", "ooo_notice"]
    }
    .into_iter()
    .map(|suffix| {
        rails_compat::turbo::signed_stream_name(
            &app.booted.app.secrets,
            &[&user_gid(JASON).to_param(), suffix],
        )
    })
    .collect::<std::collections::BTreeSet<_>>();
    for _ in 0..if meeting { 1 } else { 2 } {
        let actual: Value = serde_json::from_str(&socket.next_text().await).unwrap();
        let channel: Value = serde_json::from_str(actual["identifier"].as_str().unwrap()).unwrap();
        assert_eq!(channel["channel"], "Turbo::StreamsChannel");
        assert!(expected_streams.remove(channel["signed_stream_name"].as_str().unwrap()));
        let html = actual["message"].as_str().unwrap();
        assert!(html.contains(&format!("user_{JASON}")));
        assert!(
            html.contains(if meeting {
                "In a meeting"
            } else {
                "out of office"
            }) || html.contains("Out of office")
        );
    }
    assert!(expected_streams.is_empty());
    socket.assert_silent().await;
    let david = settings(&app).await;
    assert_eq!(david.ooo_broadcast, None);
    assert_eq!(david.meeting_cache.unwrap().in_meeting_broadcast, None);
    let jobs=app.db().read(|conn|Ok(conn.query_row("SELECT count(*) FROM background_jobs WHERE job_class='Calendar::MeetingRefreshJob'",[],|r|r.get::<_,i64>(0))?)).await.unwrap();
    assert_eq!(jobs, 0);
}
#[tokio::test]
async fn ws17_meeting_failing_member_does_not_stop_sweep_and_rolls_back_claim() {
    failed_member("meeting").await;
}
#[tokio::test]
async fn ws17_ooo_failing_member_does_not_stop_sweep_and_rolls_back_claim() {
    failed_member("ooo").await;
}

// Exact DM broadcast setup: active one-hour OOO, claimed true then clock advanced two hours.
async fn dm_notice_broadcast(name: &str) {
    let clock = std::sync::Arc::new(campfire_kit::FrozenClock::new(
        crate::controllers::presenters::test_support::SEED_NOW
            .parse()
            .unwrap(),
    ));
    let app = TestApp::boot_with_clock(clock.clone())
        .await
        .expect("parity seed");
    let (_server, mut socket) = subscribe(&app).await;
    let invisible = name == "invisible_flip";
    app.db().write(move |tx| {
        tx.conn().execute("UPDATE users SET meeting_status_enabled=0,ooo_calendar_enabled=0,ooo_until=NULL,ooo_broadcast=NULL",[])?;
        tx.conn().execute("DELETE FROM calendar_meeting_caches",[])?;
        tx.conn().execute("UPDATE users SET ooo_until='2026-03-02 17:00:00.000000',presence_setting=?,custom_status_text=NULL,custom_status_emoji=NULL,ooo_note=NULL,time_zone='UTC' WHERE id=?",rusqlite::params![if invisible {"invisible"} else {"auto"},DAVID])?;
        if !invisible { let user=UserStatusSettings::find(tx.conn(),DAVID)?; assert!(user.claim_ooo_broadcast(tx,true,tx.now())?); }
        Ok(())
    }).await.unwrap();
    let data: Value = serde_json::from_str(include_str!(
        "../../../../../../crates/views/tests/golden/ws17-dm-profile.json"
    ))
    .unwrap();
    let row = data["broadcasts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == name)
        .unwrap();
    clock.set(row["now"].as_str().unwrap().parse().unwrap());
    let stats = dispatch_ooo(app.db(), stamp(row["now"].as_str().unwrap()))
        .await
        .unwrap();
    assert!(stats.failed_user_ids.is_empty());
    assert_frames(&app, &mut socket, row["frames"].as_array().unwrap(), name).await;
    let notices = row["frames"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["stream"].as_str().unwrap().ends_with(":ooo_notice"))
        .collect::<Vec<_>>();
    assert_eq!(notices.len(), 1);
    assert!(
        !notices[0]["html"]
            .as_str()
            .unwrap()
            .contains("is out of office")
    );
}
#[tokio::test]
async fn ws17_invisible_ooo_flip_broadcasts_emptied_dm_notice() {
    dm_notice_broadcast("invisible_flip").await;
}
#[tokio::test]
async fn ws17_ooo_end_broadcasts_emptied_dm_notice() {
    dm_notice_broadcast("ooo_end").await;
}
