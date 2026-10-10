//! Actual owner registrations and durable retries, not replacement test handlers.
//! Producer controls: check_calendar_retry_mutants.py (queue budget, payload,
//! persisted failure write and unexpected real broadcast).
use super::{
    comparison_support,
    quote_integration_tests::{insert_rows},
};
use crate::{
    app::google_api_tests::{self as support, Recorded},
    controllers::presenters::test_support::*,
    integrations::google::{calendar_sync, entry_sync},
};
use campfire_db::{
    Event, Timestamp,
    models::{
        google_account::{CALENDAR_SCOPE, ConnectionGrant, GoogleAccount},
        google_calendar::{InboundSyncJob, SyncEntryJob},
    },
};
use campfire_jobs::{JobQueue, QueueConfig, Registry, RunnerConfig};
use serde_json::{Value, json};
use std::{collections::HashMap, time::Duration};
const TOKEN: &str = "fixture-calendar-access";

/// The triggers observe the runner's committed outcomes, including deleted successes.
/// Neither an expected counter nor a replacement consumer constructs these observations.
async fn observe(app: &TestApp) -> tokio::sync::mpsc::UnboundedReceiver<()> {
    let (send, received) = tokio::sync::mpsc::unbounded_channel();
    app.db().write(move |tx| {
        tx.conn().execute_batch("CREATE TABLE ws8_retry_outcomes(id INTEGER PRIMARY KEY, status TEXT, attempts INTEGER, arguments TEXT, run_at TEXT, updated_at TEXT, last_error TEXT); CREATE TRIGGER ws8_retry_status AFTER UPDATE OF status ON background_jobs WHEN OLD.status='running' AND NEW.status IN ('ready','failed') BEGIN INSERT INTO ws8_retry_outcomes(status,attempts,arguments,run_at,updated_at,last_error) VALUES(NEW.status,NEW.attempts,NEW.arguments,NEW.run_at,NEW.updated_at,NEW.last_error); END; CREATE TRIGGER ws8_retry_done BEFORE DELETE ON background_jobs BEGIN INSERT INTO ws8_retry_outcomes(status,attempts,arguments,run_at,updated_at,last_error) VALUES('done',OLD.attempts,OLD.arguments,OLD.run_at,OLD.updated_at,OLD.last_error); END;")?;
        tx.conn().update_hook(Some(move |_, _: &str, table: &str, _| { if table=="ws8_retry_outcomes" { let _=send.send(()); } }));
        Ok(())
    }).await.unwrap();
    received
}
async fn outcome(
    app: &TestApp,
    changed: &mut tokio::sync::mpsc::UnboundedReceiver<()>,
    after: i64,
) -> Value {
    loop {
        while changed.try_recv().is_ok() {}
        let actual = app
            .db()
            .write(move |tx| {
                let id: i64 = tx.conn().query_row(
                    "SELECT COALESCE(MAX(id),0) FROM ws8_retry_outcomes",
                    [],
                    |r| r.get(0),
                )?;
                Ok(if id > after {
                    comparison_support::row(tx.conn(), "ws8_retry_outcomes", id)?
                } else {
                    Value::Null
                })
            })
            .await
            .unwrap();
        if !actual.is_null() {
            return actual;
        }
        changed
            .recv()
            .await
            .expect("actual queue outcome notification");
    }
}
fn consumer_reads(sql: &[String]) -> usize {
    sql.iter()
        .filter(|s| {
            let s = s.trim_start();
            (s.starts_with("SELECT") || s.starts_with("WITH"))
                && !s.contains("background_jobs")
                && !s.contains("ws8_retry_outcomes")
        })
        .count()
}
#[tokio::test]
async fn calendar_retry_recovery_and_exhaustion_match_real_rails_jobs_with_flat_reads() {
    let vector: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/calendar_retry_consumers.json"
    ))
    .unwrap();
    let mut counts = HashMap::new();
    let mut executions = 0;
    for group in vector["groups"].as_array().unwrap() {
        for case in group["cases"].as_array().unwrap() {
            let app = TestApp::boot_frozen_with_env(&[("APP_URL", "http://campfire.test")])
                .await
                .unwrap()
                .without_job_runner()
                .await;
            insert_rows(&app, group["rows"].clone()).await;
            let id = group["event_id"].as_i64().unwrap();
            let entry = group["entry_id"].as_i64().unwrap();
            let action = case["action"].as_str().unwrap().to_owned();
            let remove = action == "remove";
            let crypto = rails_compat::ar_encryption::ArEncryption::new(&app.booted.app.secrets);
            app.db().write(move|tx|{
                tx.conn().execute("DELETE FROM event_calendar_entries WHERE id!=?",[entry])?;
                GoogleAccount::save_connection(tx,&crypto,ConnectionGrant{user_id:DAVID,email:"fixture@calendar.test".into(),access_token:Some(TOKEN.into()),refresh_token:Some("fixture-calendar-refresh".into()),access_token_expires_at:Some(Timestamp::parse_db("2026-03-02 17:00:00").unwrap()),scopes:Some(CALENDAR_SCOPE.into())})?;
                if remove {tx.conn().execute("UPDATE event_attendances SET response='declined' WHERE event_id=? AND user_id=?",[id,DAVID])?;}
                tx.conn().execute("DELETE FROM background_jobs",[])?;Ok(())
            }).await.unwrap();
            let recorded = Recorded::new(vec![]);
            for step in case["steps"].as_array().unwrap() {
                let r = &step["route"];
                let method = r["method"].as_str().unwrap().parse().unwrap();
                let path = r["path"].as_str().unwrap();
                if let Some(failure) = r["failure"].as_str() {
                    use crate::net::http::HttpError;
                    recorded.fail_for_error(
                        method,
                        path,
                        match failure {
                            "open_timeout" => HttpError::OpenTimeout,
                            "read_timeout" => HttpError::ReadTimeout,
                            "write_timeout" => HttpError::WriteTimeout,
                            _ => panic!("unlisted transport failure"),
                        }
                        .into(),
                    );
                } else {
                    recorded.answer_for(
                        method,
                        path,
                        r["status"].as_u64().unwrap() as u16,
                        r["body"].clone(),
                    );
                }
            }
            support::install(&app, recorded.clone()).await;


            let mut changed = observe(&app).await;
            let mut registry = Registry::new();
            calendar_sync::register(&mut registry);
            entry_sync::register(&mut registry);
            let config = RunnerConfig::new(vec![QueueConfig::new("default", 1)]);
            let queue = JobQueue::new(&registry, &config).unwrap();
            let inbound = action == "inbound";
            let class = if inbound {
                "Calendar::InboundSyncJob"
            } else {
                "Calendar::SyncEntryJob"
            };
            let mut queries = app.db().capture_queries();
            app.db()
                .write(move |tx| {
                    if inbound {
                        tx.emit_after_commit(Event::job(&InboundSyncJob((DAVID,))));
                    } else {
                        tx.emit_after_commit(Event::job(&SyncEntryJob {
                            event_id: id,
                            user_id: DAVID,
                        }));
                    }
                    Ok(())
                })
                .await
                .unwrap();
            let runner = campfire_jobs::start(
                app.db().clone(),
                queue.clone(),
                registry,
                app.booted.app.clone(),
                config,
            );
            let mut previous = 0;
            for (index, step) in case["steps"].as_array().unwrap().iter().enumerate() {
                let actual = outcome(&app, &mut changed, previous).await;
                previous = actual["id"].as_i64().unwrap();
                app.db().stop_capturing_queries();
                let reads = consumer_reads(&queries.lock().unwrap());
                let arguments: Value =
                    serde_json::from_str(actual["arguments"].as_str().unwrap()).unwrap();
                let metadata = &arguments["_campfire_retry_metadata_v1"];
                let counters = metadata.get("counts").cloned().unwrap_or_else(|| json!({}));
                let queue_state = json!({"retry":actual["status"]=="ready","executions":actual["attempts"],"counts":counters});
                assert_eq!(
                    queue_state,
                    json!({"retry":step["queue"]["retry"],"executions":step["queue"]["executions"],"counts":step["queue"]["counts"]}),
                    "Calendar queue state {} step {index}",
                    case["name"]
                );
                assert_eq!(
                    actual["status"] == "failed",
                    !step["error"].is_null() && step["queue"]["retry"] == false,
                    "Calendar terminal queue outcome"
                );
                if !step["error"].is_null() {
                    assert_eq!(
                        actual["last_error"], step["error"]["message"],
                        "Calendar persisted queue error"
                    );
                }
                if actual["status"] == "ready" {
                    let delay = Timestamp::parse_db(actual["run_at"].as_str().unwrap())
                        .unwrap()
                        .as_microsecond()
                        - Timestamp::parse_db(actual["updated_at"].as_str().unwrap())
                            .unwrap()
                            .as_microsecond();
                    let minimum = step["queue"]["wait"].as_f64().unwrap();
                    let delay = delay as f64 / 1_000_000.0;
                    assert!(
                        delay >= minimum && delay < minimum + (minimum - 2.0) * 0.15,
                        "production Rails polynomial jitter: {delay} >= {minimum}"
                    );
                }
                let calls=recorded.calls.lock().unwrap().drain(..).map(|c|{assert_eq!(c["access_token"],TOKEN);let body=c["body"].as_str().unwrap();json!({"method":c["method"],"path":c["path"],"body":if body.is_empty(){Value::Null}else{serde_json::from_str::<Value>(body).unwrap()}})}).collect::<Vec<_>>();
                assert_eq!(json!(calls), step["calls"], "Calendar owner exchanges");
                let data = step.clone();
                app.db()
                    .read(move |c| {
                        comparison_support::same_row(
                            &comparison_support::row(c, "event_calendar_entries", entry)?,
                            &data["entry"],
                            "Calendar persisted entry",
                        );
                        comparison_support::same_row(
                            &comparison_support::row(c, "events", id)?,
                            &data["event"],
                            "Calendar persisted event",
                        );
                        let actual = c
                            .prepare(
                                "SELECT id FROM event_attendances WHERE event_id=? ORDER BY id",
                            )?
                            .query_map([id], |r| r.get::<_, i64>(0))?
                            .collect::<rusqlite::Result<Vec<_>>>()?
                            .into_iter()
                            .map(|a| comparison_support::row(c, "event_attendances", a))
                            .collect::<campfire_db::Result<Vec<_>>>()?;
                        assert_eq!(actual.len(), data["attendance"].as_array().unwrap().len());
                        for (row, expected) in
                            actual.iter().zip(data["attendance"].as_array().unwrap())
                        {
                            comparison_support::same_row(
                                row,
                                expected,
                                "Calendar persisted attendance",
                            );
                        }
                        Ok(())
                    })
                    .await
                    .unwrap();

                let key = format!("{} step {index}", case["name"].as_str().unwrap());
                println!(
                    "WS8bm2 Calendar retry {key} size={}: Rust {reads}; Rails {}",
                    group["size"], step["reads"]
                );
                if let Some(before) = counts.insert(key, reads) {
                    assert_eq!(reads, before, "Calendar retry physical read growth");
                }
                executions += 1;
                if index + 1 < case["steps"].as_array().unwrap().len() {
                    queries = app.db().capture_queries();
                    app.db()
                        .write(|tx| {
                            tx.conn().execute(
                                "UPDATE background_jobs SET run_at=? WHERE status='ready'",
                                [tx.now()],
                            )?;
                            Ok(())
                        })
                        .await
                        .unwrap();
                    // Explicitly dispatch the actual durable retry, without sleeping for backoff.
                    queue.wake(class);
                }
            }
            runner.shutdown(Duration::from_secs(1)).await;

        }
    }
    println!(
        "WS8bm2 Calendar retry Rust: {executions} real registered job executions; exact queue counters, owner exchanges, complete rows and broadcasts; flat reads"
    );
}
