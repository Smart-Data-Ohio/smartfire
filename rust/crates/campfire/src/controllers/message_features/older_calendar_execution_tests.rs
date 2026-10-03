//! Execute the real Calendar consumers and their durable children against pinned Rails exchanges.
use super::quote_integration_tests::{insert_rows, stream};
use crate::{
    app::{
        google_api_tests::{self as support, Recorded},
        google_test_support::QueueDrain,
    },
    controllers::presenters::test_support::*,
    integrations::google::{api, calendar, calendar_sync, entry_sync},
};
use campfire_db::{
    CalendarEvent, Event, Timestamp,
    models::{
        google_account::{CALENDAR_SCOPE, ConnectionGrant, GoogleAccount},
        google_calendar::{InboundSyncJob, SyncEntryJob},
    },
};
use campfire_jobs::{
    Execution, JobKind, JobQueue, QueueConfig, Registry, RetryPolicy, RunnerConfig,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};
const FIXTURE_TOKEN: &str = "fixture-calendar-access";
macro_rules! observed_job {
    ($name:ident,$args:ty,$class:literal) => {
        #[derive(Serialize, Deserialize)]
        #[serde(transparent)]
        struct $name($args);
        impl campfire_db::Job for $name {
            const CLASS: &'static str = $class;
        }
        impl JobKind for $name {
            fn retry_policy() -> RetryPolicy {
                RetryPolicy::application_job()
                    .attempts(8)
                    .retry_on(|error| {
                        error
                            .downcast_ref::<api::Error>()
                            .is_some_and(api::Error::unavailable)
                    })
            }
        }
    };
}
observed_job!(Inbound, InboundSyncJob, "Calendar::InboundSyncJob");
observed_job!(Sync, SyncEntryJob, "Calendar::SyncEntryJob");
fn reads(sql: &[String]) -> usize {
    sql.iter()
        .filter(|s| !s.starts_with("SELECT job_class,status,attempts,last_error FROM background_jobs WHERE job_class IN (") && (s.trim_start().starts_with("SELECT") || s.trim_start().starts_with("WITH")))
        .count()
}
#[tokio::test]
async fn older_calendar_inbound_and_sync_jobs_execute_queued_children_like_rails_with_flat_reads() {
    run_matrix(false).await;
}
#[tokio::test]
async fn calendar_fatal_writes_reach_queue_outcomes() {
    run_matrix(true).await;
}
async fn run_matrix(fatal_only: bool) {
    let vector: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/older_calendar_execution.json"
    ))
    .unwrap();
    let mut counts = HashMap::new();
    let mut children = 0;
    let mut parents = 0;
    for group in vector["groups"].as_array().unwrap() {
        for case in group["cases"].as_array().unwrap() {
            if case["error"].is_null() == fatal_only {
                continue;
            }
            parents += 1;
            let app = TestApp::boot_frozen_with_env(&[("APP_URL", "http://campfire.test")])
                .await
                .unwrap()
                .without_job_runner()
                .await;
            insert_rows(&app, group["rows"].clone()).await;
            let id = group["event_id"].as_i64().unwrap();
            let entry = group["entry_id"].as_i64().unwrap();
            let name = case["name"].as_str().unwrap().to_owned();
            let setup = name.clone();
            let (mut client, server) = stream(&app).await;
            let gid = campfire_views::helpers::gid_param(
                "ChannelThread",
                group["thread_id"].as_i64().unwrap(),
            );
            let signed = rails_compat::turbo::signed_stream_name(
                &app.booted.app.secrets,
                &[&gid, "messages"],
            );
            client
                .confirm(&crate::channels::tests::support::identifier(
                    json!({"channel":"RoomMessagesChannel","signed_stream_name":signed}),
                ))
                .await;
            let crypto = rails_compat::ar_encryption::ArEncryption::new(&app.booted.app.secrets);
            app.db().write(move|tx|{
   tx.conn().execute("DELETE FROM event_calendar_entries WHERE id!=?",[entry])?;
   GoogleAccount::save_connection(tx,&crypto,ConnectionGrant{user_id:DAVID,email:"fixture@calendar.test".into(),access_token:Some(FIXTURE_TOKEN.into()),refresh_token:Some("fixture-calendar-refresh".into()),access_token_expires_at:Some(Timestamp::parse_db("2026-03-02 17:00:00").unwrap()),scopes:Some(CALENDAR_SCOPE.into())})?;
   match setup.as_str(){
    "inbound_no_account"=>{tx.conn().execute("DELETE FROM google_accounts WHERE user_id=?",[DAVID])?;},
    "inbound_disconnected"=>{tx.conn().execute("UPDATE google_accounts SET disconnected_reason='fixture-disconnected' WHERE user_id=?",[DAVID])?;},
    "inbound_departed"=>{tx.conn().execute("DELETE FROM memberships WHERE user_id=? AND room_id=?",[DAVID,QUIET_CORNER])?;},
    "inbound_local_declined"=>{tx.conn().execute("UPDATE event_attendances SET response='declined' WHERE event_id=? AND user_id=?",[id,DAVID])?;},
    "sync_conflict" | "sync_write_failed"=>{tx.conn().execute("UPDATE event_calendar_entries SET synced_at=NULL,last_error='fixture prior calendar failure' WHERE id=?",[entry])?;},
    "sync_delete_failed"=>{tx.conn().execute("UPDATE event_attendances SET response='declined' WHERE event_id=? AND user_id=?",[id,DAVID])?;},
    "sync_deleted"=>CalendarEvent::find(tx.conn(),id)?.destroy(tx)?,_=>(),
   }
   if setup.ends_with("_failed") {
    let operation=if setup=="sync_delete_failed" {"DELETE"}else{"UPDATE"};
    tx.conn().execute_batch(&format!("CREATE TRIGGER ws8_calendar_failure BEFORE {operation} ON event_calendar_entries WHEN OLD.id={entry} BEGIN SELECT RAISE(ABORT,'fixture calendar writer failure'); END"))?;
   }
   tx.conn().execute("DELETE FROM background_jobs",[])?;Ok(())
  }).await.unwrap();
            let recorded = Recorded::new(vec![]);
            for route in case["routes"].as_array().unwrap() {
                recorded.answer_for(
                    route["method"].as_str().unwrap().parse().unwrap(),
                    route["path"].as_str().unwrap(),
                    route["status"].as_u64().unwrap() as u16,
                    route["body"].clone(),
                );
            }
            support::install(&app, recorded.clone()).await;
            let metrics = Arc::new(Mutex::new(Vec::new()));
            let mut registry = Registry::new();
            // Only instrumentation wraps the owner consumers; their result/retry policies are unchanged.
            let observed = metrics.clone();
            registry.register(
                move |app: crate::app::App, job: Inbound, execution: Execution| {
                    let observed = observed.clone();
                    async move {
                        let queries = app.db.capture_queries();
                        let result = calendar_sync::inbound(&app, job.0.0.0).await;
                        app.db.stop_capturing_queries();
                        observed
                            .lock()
                            .unwrap()
                            .push(("inbound", reads(&queries.lock().unwrap())));
                        calendar::job_result(result, &execution)
                    }
                },
            );
            let observed = metrics.clone();
            let (send, received) = tokio::sync::oneshot::channel();
            let send = Arc::new(Mutex::new(Some(send)));
            registry.register(
                move |app: crate::app::App, job: Sync, execution: Execution| {
                    let observed = observed.clone();
                    let send = send.clone();
                    async move {
                        let queries = app.db.capture_queries();
                        let result = entry_sync::sync(&app, job.0.event_id, job.0.user_id).await;
                        if let Some(send) = send.lock().unwrap().take() {
                            let _ =
                                send.send(result.as_ref().map(|_| ()).map_err(ToString::to_string));
                        }
                        app.db.stop_capturing_queries();
                        observed
                            .lock()
                            .unwrap()
                            .push(("sync", reads(&queries.lock().unwrap())));
                        calendar::job_result(result, &execution)
                    }
                },
            );
            let mut drain = QueueDrain::install(&app).await;
            let config = RunnerConfig::new(vec![QueueConfig::new("default", 1)]);
            let queue = JobQueue::new(&registry, &config).unwrap();
            let runner = campfire_jobs::start(
                app.db().clone(),
                queue,
                registry,
                app.booted.app.clone(),
                config,
            );
            let inbound = name.starts_with("inbound");
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
            assert!(!case["retry"].as_bool().unwrap());
            if !case["error"].is_null() {
                let error = received
                    .await
                    .unwrap()
                    .expect_err("Calendar write failure reaches the consumer");
                assert!(error.contains("fixture calendar writer failure"), "{error}");
            } else {
                drain.calendar(&app).await;
            }
            runner.shutdown(Duration::from_secs(1)).await;
            if !case["error"].is_null() {
                app.db()
                    .read(|c| {
                        let jobs = c
                            .prepare("SELECT status,attempts,last_error FROM background_jobs")?
                            .query_map([], |r| {
                                Ok((
                                    r.get::<_, String>(0)?,
                                    r.get::<_, i64>(1)?,
                                    r.get::<_, String>(2)?,
                                ))
                            })?
                            .collect::<rusqlite::Result<Vec<_>>>()?;
                        assert_eq!(jobs.len(), 1);
                        assert_eq!(jobs[0].0, "failed");
                        assert_eq!(jobs[0].1, 1);
                        assert!(jobs[0].2.contains("fixture calendar writer failure"));
                        Ok(())
                    })
                    .await
                    .unwrap();
            }
            client.assert_silent().await;
            assert!(case["frames"].as_array().unwrap().is_empty());
            let calls=recorded.calls.lock().unwrap().iter().map(|c|{assert_eq!(c["access_token"],FIXTURE_TOKEN);let body=c["body"].as_str().unwrap();json!({"method":c["method"],"path":c["path"],"body":if body.is_empty(){Value::Null}else{serde_json::from_str::<Value>(body).unwrap()}})}).collect::<Vec<_>>();
            assert_eq!(
                json!(calls),
                case["calls"],
                "{name}: Google owner exchanges"
            );
            let actual = app
                .db()
                .read(move |c| {
                    use rusqlite::OptionalExtension;
                    Ok((
                        c.query_row(
                            "SELECT response FROM event_attendances WHERE event_id=? AND user_id=?",
                            [id, DAVID],
                            |r| r.get::<_, String>(0),
                        )
                        .optional()?,
                        super::comparison_support::row(c, "event_calendar_entries", entry)?,
                    ))
                })
                .await
                .unwrap();
            assert_eq!(json!(actual.0), case["response"], "{name}");
            assert_eq!(json!(!actual.1.is_null()), case["entry_present"], "{name}");
            super::comparison_support::same_row(&actual.1, &case["entry_row"], &name);
            let metrics = metrics.lock().unwrap();
            let expected_children = usize::from(matches!(
                name.as_str(),
                "inbound_cancelled" | "inbound_deleted"
            ));
            assert_eq!(
                metrics.len(),
                1 + expected_children,
                "{name}: real consumers executed"
            );
            children += expected_children;
            let count = metrics.iter().map(|(_, n)| n).sum::<usize>();
            println!(
                "WS8bm2 Calendar execution Rust {name} size={}: {count} consumer reads; Rails={}",
                group["size"], case["reads"]
            );
            if let Some(previous) = counts.insert(name.clone(), count) {
                assert_eq!(count, previous, "{name}: read growth");
            }
            server.abort();
        }
    }
    println!(
        "WS8bm2 Calendar execution Rust: {parents} parent jobs; {children} queued SyncEntry children executed; exact exchanges, complete persisted entries, states and silent old-window streams; flat consumer reads"
    );
}
