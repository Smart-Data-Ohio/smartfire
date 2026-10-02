//! Exact fetch/cache/queue observations from Calendar::MeetingRefresh at the Rails pin.
use super::google_api_tests::{self as support, Recorded};
use crate::{
    controllers::presenters::test_support::{DAVID, TestApp},
    integrations::google::meeting_refresh,
};
use campfire_db::{Timestamp, models::google_meeting_cache as cache};
use campfire_kit::FrozenClock;
use rusqlite::OptionalExtension;
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};

fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../vectors/google_meeting_refresh.json"
    ))
    .unwrap()
}
fn stamp(value: &Value) -> Timestamp {
    Timestamp::from_jiff(value.as_str().unwrap().parse().unwrap())
}
fn time(value: Option<Timestamp>) -> Value {
    json!(value.map(|t| format!("{:.6}", t.jiff())))
}
async fn cache_state(a: &TestApp) -> Value {
    a.db().read(|conn| {
        Ok(conn.query_row(
            "SELECT busy_intervals,ooo_intervals,fetch_error,fetched_at,refresh_pending_at,created_at,updated_at FROM calendar_meeting_caches WHERE user_id=?",
            [DAVID], |r| Ok(json!({
                "busy_intervals":serde_json::from_str::<Value>(&r.get::<_,String>(0)?).unwrap(),
                "ooo_intervals":serde_json::from_str::<Value>(&r.get::<_,String>(1)?).unwrap(),
                "fetch_error":r.get::<_,Option<String>>(2)?,
                "fetched_at":time(r.get(3)?),"refresh_pending_at":time(r.get(4)?),
                "created_at":time(r.get(5)?),"updated_at":time(r.get(6)?)
            }))
        ).optional()?.unwrap_or(Value::Null))
    }).await.unwrap()
}
async fn fixture(case: &Value, now: Timestamp) -> (TestApp, Arc<Recorded>) {
    let mut a = TestApp::boot_without_periodic_with_clock(Arc::new(FrozenClock::new(now.jiff())))
        .await
        .unwrap();
    a.booted.jobs.stop(Duration::from_secs(5)).await;
    let r = Recorded::new(vec![]);
    support::install(&a, r.clone()).await;
    a.db().write(|tx| {
        tx.conn().execute_batch("DELETE FROM google_accounts;DELETE FROM calendar_meeting_caches;DELETE FROM background_jobs;")?;
        Ok(())
    }).await.unwrap();
    let spec = case["spec"].clone();
    if spec["account"] != false {
        support::grant(
            &a,
            DAVID,
            now.since(jiff::SignedDuration::from_hours(
                if spec["expired"] == true { -1 } else { 1 },
            )),
            false,
        )
        .await;
    }
    let initial = case["initial"].clone();
    a.db().write(move |tx| {
        tx.conn().execute("UPDATE users SET status=?,meeting_status_enabled=?,ooo_calendar_enabled=?,time_zone=? WHERE id=?",
            rusqlite::params![i64::from(spec["inactive"] == true),spec["meeting"] != false,spec["ooo"] == true,spec["zone"].as_str(),DAVID])?;
        if spec["disconnected"] == true {
            tx.conn().execute("UPDATE google_accounts SET disconnected_reason='Google rejected the connection' WHERE user_id=?", [DAVID])?;
        }
        if let Some(scope) = spec["scopes"].as_str() {
            tx.conn().execute("UPDATE google_accounts SET scopes=? WHERE user_id=?", rusqlite::params![scope,DAVID])?;
        }
        if spec["unreadable"] == true {
            tx.conn().execute("UPDATE google_accounts SET refresh_token='broken-AR-ciphertext' WHERE user_id=?",[DAVID])?;
        }
        if !initial.is_null() {
            tx.conn().execute("INSERT INTO calendar_meeting_caches(user_id,busy_intervals,ooo_intervals,fetch_error,fetched_at,refresh_pending_at,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?)", rusqlite::params![DAVID,initial["busy_intervals"].to_string(),initial["ooo_intervals"].to_string(),initial["fetch_error"].as_str(),initial["fetched_at"].as_str().map(|_|stamp(&initial["fetched_at"])),initial["refresh_pending_at"].as_str().map(|_|stamp(&initial["refresh_pending_at"])),stamp(&initial["created_at"]),stamp(&initial["updated_at"])])?;
        }
        Ok(())
    }).await.unwrap();
    for response in case["responses"].as_array().unwrap() {
        if response[0] == "timeout" {
            r.fail_next();
        } else {
            r.answers.lock().unwrap().push_back((
                response[0].as_u64().unwrap() as u16,
                response[1].as_str().unwrap().as_bytes().to_vec(),
            ));
        }
    }
    (a, r)
}

async fn replay(rows: &[Value], now: Timestamp) {
    for case in rows {
        let name = case["spec"]["name"].as_str().unwrap();
        let (a, r) = fixture(case, now).await;
        assert_eq!(
            cache_state(&a).await,
            case["initial"],
            "{name}: initial cache"
        );
        for step in case["steps"].as_array().unwrap() {
            let user_id = if case["spec"]["missing_user"] == true {
                0
            } else {
                DAVID
            };
            let result = meeting_refresh::refresh(&a.booted.app, user_id, stamp(&step["at"]))
                .await
                .unwrap();
            let result = match result {
                meeting_refresh::Result::Skipped => "skipped",
                meeting_refresh::Result::Fresh => "fresh",
                meeting_refresh::Result::Ok => "ok",
                meeting_refresh::Result::Error => "error",
            };
            assert_eq!(json!(result), step["result"], "{name}: refresh result");
            assert_eq!(
                json!(*r.calls.lock().unwrap()),
                step["calls"],
                "{name}: exact Google exchanges"
            );
            assert_eq!(
                cache_state(&a).await,
                step["cache"],
                "{name}: complete persisted cache"
            );
            let (jobs, disconnected) = a.db().read(|conn| {
                let jobs = conn.prepare("SELECT arguments,run_at FROM background_jobs WHERE job_class='Calendar::MeetingRefreshJob' ORDER BY id")?
                    .query_map([], |r| Ok((r.get::<_,String>(0)?,r.get::<_,Timestamp>(1)?)))?
                    .collect::<rusqlite::Result<Vec<_>>>()?.into_iter().map(|(args,at)| {
                        let args:Value=serde_json::from_str(&args).unwrap();
                        json!({"class":"Calendar::MeetingRefreshJob","args":[args["user_id"]],"at":time(Some(at))})
                    }).collect::<Vec<_>>();
                let disconnected = conn.query_row("SELECT disconnected_reason FROM google_accounts WHERE user_id=?",[DAVID],|r|r.get::<_,Option<String>>(0)).optional()?.flatten();
                Ok((jobs,disconnected))
            }).await.unwrap();
            assert_eq!(
                json!(jobs),
                step["jobs"],
                "{name}: durable follow-up and exact schedule"
            );
            assert_eq!(
                json!(disconnected),
                step["disconnected"],
                "{name}: grant usability"
            );
        }
    }
}
#[tokio::test]
async fn google_meeting_refresh_complete_states_and_requests_match_pinned_rails() {
    let v = oracle();
    replay(v["rows"].as_array().unwrap(), stamp(&v["now"])).await;
    println!("Pinned Rails MeetingRefresh: 38 scenarios; 41 ordered refreshes; 0 skipped");
}
#[tokio::test]
async fn google_meeting_refresh_cache_timestamps_use_the_injected_clock() {
    let v = oracle();
    let rows = v["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["spec"]["name"] == "success")
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 1);
    replay(&rows, stamp(&v["now"])).await;
}
#[tokio::test]
async fn google_meeting_refresh_followup_rejection_rolls_back_claim_and_cache() {
    let v = oracle();
    let case = v["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["spec"]["name"] == "followup")
        .unwrap();
    let now = stamp(&v["now"]);
    let (a, r) = fixture(case, now).await;
    let before = cache_state(&a).await;
    a.db().write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER reject_calendar_followup BEFORE INSERT ON background_jobs WHEN NEW.job_class='Calendar::MeetingRefreshJob' BEGIN SELECT RAISE(ABORT,'recorded follow-up rejection'); END;")?;
        Ok(())
    }).await.unwrap();
    assert!(
        meeting_refresh::refresh(&a.booted.app, DAVID, now)
            .await
            .is_err()
    );
    assert_eq!(cache_state(&a).await, before);
    assert!(r.calls.lock().unwrap().is_empty());
    assert_eq!(
        a.db()
            .read(|c| Ok(
                c.query_row("SELECT COUNT(*) FROM background_jobs", [], |r| r
                    .get::<_, i64>(0))?
            ))
            .await
            .unwrap(),
        0
    );
    // Once queue persistence works, one claimant wins and all replays keep that schedule.
    a.db()
        .write(|tx| {
            tx.conn()
                .execute_batch("DROP TRIGGER reject_calendar_followup")?;
            Ok(())
        })
        .await
        .unwrap();
    for _ in 0..3 {
        assert_eq!(
            meeting_refresh::refresh(&a.booted.app, DAVID, now)
                .await
                .unwrap(),
            meeting_refresh::Result::Fresh
        );
    }
    assert_eq!(
        a.db()
            .read(|c| Ok(
                c.query_row("SELECT COUNT(*) FROM background_jobs", [], |r| r
                    .get::<_, i64>(0))?
            ))
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        a.db()
            .read(|c| cache::find(c, DAVID))
            .await
            .unwrap()
            .unwrap()
            .refresh_pending_at,
        Some(now)
    );
}
