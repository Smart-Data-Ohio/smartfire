use super::*;
use crate::models::calendar_dispatch::{dispatch_meetings, dispatch_ooo};
use crate::models::user_status_settings::updates::{
    MeetingRefreshJob, StatusBadgeBroadcast,
};
use crate::{Broadcast, MeetingCache, Timestamp, UserStatusSettings};
use rusqlite::{
    params,
    trace::{TraceEvent, TraceEventCodes},
};
use serde_json::{Value, json};
use std::cell::Cell;

fn stamp(s: &str) -> Timestamp {
    Timestamp::from_jiff(s.parse().unwrap())
}
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../../vectors/ws17_calendar_dispatch.json"
    ))
    .unwrap()
}
fn db() -> TestDb {
    TestDb::with_clock(TestClock::frozen_at(stamp("2026-03-02T16:00:00Z")), 4)
}
thread_local! { static UPDATES: Cell<usize> = const { Cell::new(0) }; }
fn trace(event: TraceEvent<'_>) {
    if let TraceEvent::Stmt(_, sql) = event
        && sql.starts_with("UPDATE")
    {
        UPDATES.with(|n| n.set(n.get() + 1));
    }
}

fn setup(t: &TestDb, row: Value) {
    t.write(move|tx|{
  tx.conn().execute("UPDATE users SET meeting_status_enabled=0,ooo_calendar_enabled=0,ooo_until=NULL",[])?;
  tx.conn().execute("DELETE FROM workspace_presence_leases",[])?;
  tx.conn().execute("UPDATE users SET status=0,meeting_status_enabled=0,meeting_dnd_enabled=0,ooo_calendar_enabled=0,ooo_until=NULL,ooo_note=NULL,ooo_broadcast=NULL,ooo_notify_enabled=0,presence_setting='auto',custom_status_text=NULL,custom_status_emoji=NULL,time_zone='UTC',dnd_enabled=0 WHERE id=?",[id("david")])?;
  for(key,value)in row["attrs"].as_object().into_iter().flatten(){
   let value=match value {Value::Bool(b)=>rusqlite::types::Value::Integer(i64::from(*b)),Value::Number(n)=>rusqlite::types::Value::Integer(n.as_i64().unwrap()),Value::String(s) if key=="ooo_until"=>rusqlite::types::Value::Text(stamp(s).to_db()),Value::String(s)=>rusqlite::types::Value::Text(s.clone()),Value::Null=>rusqlite::types::Value::Null,_=>panic!("{value}")};
   tx.conn().execute(&format!("UPDATE users SET {key}=? WHERE id=?"),params![value,id("david")])?;
  }
  if row["missing"]!=true {
   let at=match row.get("fetched_at"){Some(Value::Null)=>None,Some(Value::String(s))=>Some(stamp(s)),None=>Some(tx.now()),_=>panic!("bad timestamp")};
   tx.conn().execute("INSERT INTO calendar_meeting_caches(user_id,busy_intervals,ooo_intervals,fetched_at,in_meeting_broadcast,created_at,updated_at) VALUES (?,?,?,?,?,?,?)",params![id("david"),row.get("busy").unwrap_or(&json!([])).to_string(),row.get("ooo").unwrap_or(&json!([])).to_string(),at,row["claimed"].as_bool(),tx.now(),tx.now()])?;
  }
  let conn=tx.conn();conn.trace_v2(TraceEventCodes::SQLITE_TRACE_STMT,Some(trace));UPDATES.with(|n|n.set(0));
  Ok(())
 });
}

#[test]
fn ws17_calendar_two_ticks_match_rails_jobs_claims_update_counts_and_emission_order() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let golden = fixture();
    let now = stamp(golden["now"].as_str().unwrap());
    for row in golden["rows"].as_array().unwrap() {
        let t = db();
        setup(&t, row.clone());
        for expected in row["runs"].as_array().unwrap() {
            let before = t.events().len();
            t.write(|_| {
                UPDATES.with(|n| n.set(0));
                Ok(())
            });
            if matches!(row["kind"].as_str(), Some("meeting" | "both")) {
                assert!(
                    rt.block_on(dispatch_meetings(&t.db, now))
                        .unwrap()
                        .failed_user_ids
                        .is_empty()
                );
            }
            if matches!(row["kind"].as_str(), Some("ooo" | "both")) {
                assert!(
                    rt.block_on(dispatch_ooo(&t.db, now))
                        .unwrap()
                        .failed_user_ids
                        .is_empty()
                );
            }
            let updates = t.write(|_| Ok(UPDATES.with(Cell::get)));
            assert_eq!(
                json!(updates),
                expected["updates"],
                "{} UPDATE count",
                row["name"]
            );
            let events = t.events().into_iter().skip(before).collect::<Vec<_>>();
            let jobs = events
                .iter()
                .filter_map(|e| e.as_job::<MeetingRefreshJob>())
                .map(|j| json!({"class":"Calendar::MeetingRefreshJob","args":[j.user_id]}))
                .collect::<Vec<_>>();
            assert_eq!(json!(jobs), expected["jobs"], "{} jobs", row["name"]);
            let streams = events
                .iter()
                .filter_map(|e| match e {
                    Event::Broadcast(b) if b.kind == StatusBadgeBroadcast::KIND => Some("status"),
                    Event::Broadcast(b) => panic!("unexpected {b:?}"),
                    _ => None,
                })
                .collect::<Vec<_>>();
            let expected_streams = expected["frames"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|f| { let stream = f["stream"].as_str().unwrap().rsplit(':').next().unwrap(); (stream == "status").then_some(stream) })
                .collect::<Vec<_>>();
            assert_eq!(streams, expected_streams, "{} emission order", row["name"]);
            let settings = t.read(|c| UserStatusSettings::find(c, id("david")));
            assert_eq!(
                json!(settings.ooo_broadcast),
                expected["stored"]["ooo_broadcast"],
                "{} OOO claim",
                row["name"]
            );
            assert_eq!(
                json!(settings.ooo_note),
                expected["stored"]["ooo_note"],
                "{} note",
                row["name"]
            );
            assert_eq!(
                settings.ooo_until,
                expected["stored"]["ooo_until"].as_str().map(stamp),
                "{} until",
                row["name"]
            );
            assert_eq!(
                json!(settings.meeting_cache.and_then(|c| c.in_meeting_broadcast)),
                expected["meeting_claim"],
                "{} meeting claim",
                row["name"]
            );
        }
    }
}

#[test]
fn ws17_concurrent_dispatchers_broadcast_each_boundary_once() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    for kind in ["meeting", "ooo"] {
        let t = db();
        let row = fixture()["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| {
                r["name"]
                    == format!(
                        "{kind}_{}",
                        if kind == "meeting" { "start" } else { "manual" }
                    )
            })
            .unwrap()
            .clone();
        setup(&t, row);
        let second = t.another_process();
        let now = t.now();
        let (a, b) = rt.block_on(async {
            if kind == "meeting" {
                tokio::join!(
                    dispatch_meetings(&t.db, now),
                    dispatch_meetings(&second, now)
                )
            } else {
                tokio::join!(dispatch_ooo(&t.db, now), dispatch_ooo(&second, now))
            }
        });
        assert_eq!(a.unwrap().flipped + b.unwrap().flipped, 1, "{kind}");
        assert_eq!(
            t.events()
                .iter()
                .filter(|e| matches!(e, Event::Broadcast(_)))
                .count(),
            1
        );
    }
}

#[test]
fn ws17_ooo_emits_all_badges_before_all_notices_in_primary_key_order() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let t = db();
    let until = t.now().since(jiff::SignedDuration::from_hours(1));
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE users SET ooo_until=?,ooo_broadcast=NULL,time_zone='UTC' WHERE id IN (?,?)",
            params![until, id("david"), id("jason")],
        )?;
        Ok(())
    });
    assert_eq!(
        rt.block_on(dispatch_ooo(&t.db, t.now())).unwrap().flipped,
        2
    );
    let mut ids = [id("david"), id("jason")];
    ids.sort();
    let actual = t
        .events()
        .iter()
        .filter_map(|event| {
            if let Event::Broadcast(b) = event {
                Some((b.kind, b.arguments["user_id"].as_i64().unwrap()))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(
        actual,
        vec![
            (StatusBadgeBroadcast::KIND, ids[0]),
            (StatusBadgeBroadcast::KIND, ids[1]),
        ]
    );
}

fn cache(t: &TestDb, busy: Value, ooo: Value) -> MeetingCache {
    t.write(move|tx|{tx.conn().execute("INSERT INTO calendar_meeting_caches(user_id,busy_intervals,ooo_intervals,created_at,updated_at) VALUES (?,?,?,?,?)",params![id("david"),busy.to_string(),ooo.to_string(),tx.now(),tx.now()])?;Ok(())});
    t.read(|c| {
        Ok(UserStatusSettings::find(c, id("david"))?
            .meeting_cache
            .unwrap())
    })
}
#[test]
fn ws17_meeting_cache_named_readers_cover_boundaries_malformed_and_epochs() {
    let t = db();
    let start = "2026-09-23T10:00:00Z";
    let end = "2026-09-23T11:00:00Z";
    let pairs = json!([[start, end], null, "nope", ["not-a-time", end]]);
    let c = cache(&t, pairs.clone(), pairs);
    for (time, covered) in [
        ("2026-09-23T09:59:59Z", false),
        (start, true),
        ("2026-09-23T10:30:00Z", true),
        (end, false),
    ] {
        assert_eq!(c.in_meeting(stamp(time)), covered);
        assert_eq!(c.in_ooo(stamp(time)), covered);
    }
    let epochs = vec![(
        stamp(start).jiff().as_second(),
        stamp(end).jiff().as_second(),
    )];
    assert_eq!(c.quiet_window_epochs(t.now()), epochs);
    assert_eq!(c.ooo_window_epochs(t.now()), epochs);
    let mut c = c;
    c.busy_intervals = json!([]);
    assert!(!c.in_meeting(stamp(start)));
    c.busy_intervals = json!([null, "nope", ["bad", end]]);
    assert!(!c.in_meeting(stamp(start)));
    c.ooo_intervals = json!([]);
    c.busy_intervals = json!([[start, end]]);
    assert!(c.in_meeting(stamp(start)));
    assert!(!c.in_ooo(stamp(start)));
    assert!(c.ooo_end_covering(stamp(start)).is_none());
    c.ooo_intervals = json!([
        ["2026-09-23T08:00:00Z", end],
        ["2026-09-23T10:00:00Z", "2026-09-23T13:00:00Z"],
        ["2026-09-23T06:00:00Z", "2026-09-23T07:00:00Z"]
    ]);
    assert_eq!(
        c.ooo_end_covering(stamp(start)),
        Some(stamp("2026-09-23T13:00:00Z"))
    );
    assert!(c.ooo_end_covering(stamp("2026-09-23T14:00:00Z")).is_none());
}
#[test]
fn ws17_meeting_cache_claims_flip_once_across_loaded_records_and_processes() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let t = db();
    let c = cache(&t, json!([]), json!([]));
    let first = c.clone();
    let second = c.clone();
    let other = t.another_process();
    let (a, b) = rt.block_on(async {
        tokio::join!(
            t.db.write(move |tx| first.claim_broadcast(tx, true)),
            other.write(move |tx| second.claim_broadcast(tx, true))
        )
    });
    assert_eq!(usize::from(a.unwrap()) + usize::from(b.unwrap()), 1);
    for (active, expected) in [(true, false), (false, true), (false, false), (true, true)] {
        let c = c.clone();
        assert_eq!(t.write(move |tx| c.claim_broadcast(tx, active)), expected);
    }
}
#[test]
fn ws17_meeting_cache_foreign_key_cascades_and_followup_window_claims() {
    let t = db();
    let c = cache(&t, json!([]), json!([]));
    let first = c.clone();
    assert!(t.write(move |tx| first.claim_refresh_followup(
        tx,
        tx.now(),
        jiff::SignedDuration::from_secs(60)
    )));
    let second = c.clone();
    assert!(!t.write(move |tx| second.claim_refresh_followup(
        tx,
        tx.now(),
        jiff::SignedDuration::from_secs(60)
    )));
    t.travel(60);
    assert!(t.write(move |tx| c.claim_refresh_followup(
        tx,
        tx.now(),
        jiff::SignedDuration::from_secs(60)
    )));
    let fk = t.read(|conn| {
        crate::sql::query_all(
            conn,
            "PRAGMA foreign_key_list(calendar_meeting_caches)",
            [],
            |r| {
                Ok((
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(6)?,
                ))
            },
        )
    });
    assert!(fk.contains(&("users".into(), "user_id".into(), "CASCADE".into())));
}

#[test]
fn ws17_corrupt_cache_row_isolated_from_other_members_in_both_sweeps() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    for meeting in [true, false] {
        let t = db();
        let now = t.now();
        t.write(move|tx| {
            tx.conn().execute("UPDATE users SET meeting_status_enabled=?,ooo_calendar_enabled=? WHERE id IN (?,?)",params![meeting,!meeting,id("david"),id("jason")])?;
            for user in [id("david"),id("jason")] {
                tx.conn().execute("INSERT INTO calendar_meeting_caches(user_id,busy_intervals,ooo_intervals,fetched_at,created_at,updated_at) VALUES (?,'[[\"2026-03-02T15:55:00Z\",\"2026-03-02T16:55:00Z\"]]','[[\"2026-03-02T15:55:00Z\",\"2026-03-02T16:55:00Z\"]]',?,?,?)",params![user,now,now,now])?;
            }
            tx.conn().execute("UPDATE calendar_meeting_caches SET fetched_at='broken timestamp' WHERE user_id=?",[id("david")])?;
            Ok(())
        });
        let stats = rt
            .block_on(async {
                if meeting {
                    dispatch_meetings(&t.db, now).await
                } else {
                    dispatch_ooo(&t.db, now).await
                }
            })
            .unwrap();
        assert_eq!(stats.failed_user_ids, vec![id("david")]);
        assert_eq!(stats.flipped, 1);
        for event in t.events() {
            if let Event::Broadcast(b) = event {
                assert_eq!(b.arguments["user_id"], id("jason"));
            }
        }
    }
}
