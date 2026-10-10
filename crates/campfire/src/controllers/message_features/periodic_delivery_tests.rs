//! Actual production task objects executed by Periodic::tick, with complete state.
//! Producer controls in check_rendering_mutants.py remove the reminder claim,
//! change durable push arguments and skip scheduled delivery; each witness is real.
use super::{
    comparison_support,
    quote_integration_tests::{insert_rows},
};
use crate::controllers::presenters::test_support::*;
use serde_json::{Value, json};
use std::{collections::HashMap, sync::Arc, time::Duration};
#[tokio::test]
async fn periodic_wide_and_due_delivery_match_rails_full_rows_jobs_and_frames() {
    let vector: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/periodic_delivery.json"
    ))
    .unwrap();
    let mut counts = HashMap::new();
    let mut ticks = 0;
    for group in vector["groups"].as_array().unwrap() {
        let clock = Arc::new(campfire_kit::FrozenClock::new(SEED_NOW.parse().unwrap()));
        let inputs = campfire_db::database::FixtureInputs {
            message_uuid: Arc::new(|| "fixture-periodic-message".into()),
            sqlite_now: campfire_db::Timestamp::parse_db(SEED_NOW).unwrap(),
        };
        let app = crate::test_support::with_message_inputs(
            inputs,
            TestApp::boot_with_test_clock(clock.clone()),
        )
        .await
        .unwrap()
        .without_job_runner()
        .await;
        insert_rows(&app, group["rows"].clone()).await;
        app.db()
            .write(|tx| {
                tx.conn().execute(
                    "UPDATE users SET time_zone='America/New_York' WHERE id=?",
                    [DAVID],
                )?;
                Ok(())
            })
            .await
            .unwrap();

        let mut periodic =
            crate::jobs::periodic::periodic(crate::jobs::periodic::PeriodicIntervals {
                reminders: Duration::from_secs(30),
                retention: Duration::from_secs(86_400),
            });
        periodic.retain_tasks(&["saved item reminders", "scheduled messages"]);
        for (index, step) in group["steps"].as_array().unwrap().iter().enumerate() {
            let elapsed = step["elapsed"].as_i64().unwrap();
            clock.set(
                SEED_NOW.parse::<jiff::Timestamp>().unwrap()
                    + jiff::SignedDuration::from_secs(elapsed),
            );
            app.db()
                .write(|tx| {
                    tx.conn().execute("DELETE FROM background_jobs", [])?;
                    Ok(())
                })
                .await
                .unwrap();
            let query_log = app.db().capture_queries();
            let ran = periodic
                .tick(
                    app.booted.app.clone(),
                    campfire_db::Timestamp::from_jiff(clock_time(&clock)),
                )
                .await;
            app.db().stop_capturing_queries();
            assert_eq!(
                json!(ran),
                step["ran"],
                "periodic actual registered ticks differ from Rails"
            );
            let expected = comparison_support::expected_tables(group, step);
            let actual_jobs = app
                .db()
                .read(move |conn| {
                    for (table, expected) in &expected {
                        let rows = conn
                            .prepare(&format!("SELECT id FROM {table} ORDER BY id"))?
                            .query_map([], |r| r.get::<_, i64>(0))?
                            .collect::<rusqlite::Result<Vec<_>>>()?;
                        assert_eq!(
                            rows.len(),
                            expected.as_array().unwrap().len(),
                            "periodic actual persisted row count: {table}"
                        );
                        for (id, row) in rows.into_iter().zip(expected.as_array().unwrap()) {
                            comparison_support::same_row(
                                &comparison_support::row(conn, table, id)?,
                                row,
                                "periodic actual persisted rows",
                            );
                        }
                    }
                    let mut stmt = conn
                        .prepare("SELECT job_class,arguments FROM background_jobs ORDER BY id")?;
                    let jobs = stmt
                        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
                        .collect::<rusqlite::Result<Vec<_>>>()?;
                    Ok(jobs
                        .into_iter()
                        .map(|(class, args)| {
                            let args: Value = serde_json::from_str(&args).unwrap();
                            let ids = match class.as_str() {
                                "SavedItem::ReminderPushJob" => vec![args["saved_item_id"].clone()],
                                "Room::PushMessageJob" => {
                                    vec![args["room_id"].clone(), args["message_id"].clone()]
                                }
                                _ => panic!("unexpected actual periodic job {class}: {args}"),
                            };
                            json!({"class":class,"record_ids":ids})
                        })
                        .collect::<Vec<_>>())
                })
                .await
                .unwrap();
            assert_eq!(
                json!(actual_jobs),
                step["jobs"],
                "periodic actual durable jobs differ from Rails"
            );

            let reads = query_log.lock().unwrap().len();
            let key = format!("{}/{index}", group["kind"]);
            if let Some(previous) = counts.insert(key, reads) {
                assert_eq!(previous, reads, "periodic physical read growth");
            }
            println!(
                "WS8bm2 periodic size={} kind={} tick={index}: Rust {reads}; Rails {}",
                group["size"], group["kind"], step["reads"]
            );
            ticks += 1;
        }

    }
    assert_eq!(ticks, 24);
    println!(
        "WS8bm2 periodic Rust: {ticks} actual registered ticks; complete rows, jobs and byte-identical ordered publications; flat reads"
    );
}
fn clock_time(clock: &campfire_kit::FrozenClock) -> jiff::Timestamp {
    campfire_kit::Clock::now(clock)
}
