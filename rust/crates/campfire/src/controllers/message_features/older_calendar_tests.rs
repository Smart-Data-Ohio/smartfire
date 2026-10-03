//! Real Event callbacks on both old-message streams; pinned Rails bytes and total SQL.
use super::quote_integration_tests::{app_rows, stream};
use crate::controllers::presenters::test_support::*;
use campfire_db::{CalendarEvent, models::calendar_event::changes::EventChanges};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/older_calendar_callbacks.json"
    ))
    .unwrap()
}
fn title(tx: &mut campfire_db::Tx<'_>, id: i64, text: &str) -> campfire_db::Result<()> {
    CalendarEvent::update(
        tx,
        id,
        EventChanges {
            title: Some(text.into()),
            ..Default::default()
        },
    )
    .map(|_| ())
}
async fn callbacks(check_reads: bool) {
    let mut counts = HashMap::new();
    let mut frame_count = 0;
    for group in oracle()["groups"].as_array().unwrap() {
        let app = app_rows(group["rows"].clone()).await;
        for path in [
            format!("/rooms/{QUIET_CORNER}/messages"),
            format!(
                "/rooms/{QUIET_CORNER}/threads/{}/messages",
                group["thread_id"]
            ),
        ] {
            let response = app.david().get(&path).await;
            assert_eq!(response.status, axum::http::StatusCode::OK);
            for id in group["old_ids"].as_array().unwrap() {
                assert!(
                    !response
                        .text()
                        .contains(&format!("data-message-id=\"{id}\"")),
                    "old reference in {path}"
                );
            }
        }
        let (mut client, server) = stream(&app).await;
        let gid = campfire_views::helpers::gid_param(
            "ChannelThread",
            group["thread_id"].as_i64().unwrap(),
        );
        let signed =
            rails_compat::turbo::signed_stream_name(&app.booted.app.secrets, &[&gid, "messages"]);
        client
            .confirm(&crate::channels::tests::support::identifier(
                json!({"channel":"RoomMessagesChannel","signed_stream_name":signed}),
            ))
            .await;
        let id = group["event_id"].as_i64().unwrap();
        for step in group["steps"].as_array().unwrap() {
            let name = step["name"].as_str().unwrap().to_owned();
            if check_reads && name == "removed_reference" {
                continue;
            }
            let operation = name.clone();
            let removed = group["old_ids"][0].as_i64().unwrap();
            let queries = app.db().capture_queries();
            app.db()
                .write(move |tx| match operation.as_str() {
                    "title" => title(tx, id, "Updated <&> title"),
                    "twice" => {
                        title(tx, id, "Intermediate")?;
                        title(tx, id, "Final <&> title")
                    }
                    "meet" => CalendarEvent::save_meet_link(
                        tx,
                        id,
                        Some("https://meet.example.test/fixture".into()),
                    )
                    .map(|_| ()),
                    "cancel" => CalendarEvent::cancel_with_scope(tx, id, "this_event", Some(DAVID))
                        .map(|_| ()),
                    "removed_reference" => {
                        title(tx, id, "Reference removed")?;
                        tx.conn().execute(
                            "DELETE FROM event_references WHERE message_id=? AND event_id=?",
                            [removed, id],
                        )?;
                        Ok(())
                    }
                    _ => unreachable!(),
                })
                .await
                .unwrap();
            app.db().stop_capturing_queries();
            let reads = queries
                .lock()
                .unwrap()
                .iter()
                .filter(|sql| {
                    sql.trim_start().starts_with("SELECT") || sql.trim_start().starts_with("WITH")
                })
                .count();
            if name == "title" {
                let key = group["populated"].as_bool().unwrap();
                let size = group["size"].as_u64().unwrap();
                println!(
                    "WS8bm2 older-calendar Rust reads populated={key} size={size}: {reads}; Rails={}",
                    step["reads"]
                );
                if check_reads && let Some(before) = counts.insert(key, reads) {
                    assert_eq!(
                        reads, before,
                        "callback total reads grow within a Rails batch"
                    );
                }
            }
            frame_count += step["frames"].as_array().unwrap().len();
            super::comparison_support::published_frames(&app, &mut client, &step["frames"], &name).await;
            client.assert_silent().await;
        }
        let result: campfire_db::Result<()> = app
            .db()
            .write(move |tx| {
                title(tx, id, "Rolled back secret")?;
                Err(campfire_db::Error::Other("fixture rollback".into()))
            })
            .await;
        assert!(result.is_err());
        client.assert_silent().await;
        app.db()
            .read(move |c| {
                assert_eq!(
                    CalendarEvent::find(c, id)?.title,
                    if check_reads {
                        "Final <&> title"
                    } else {
                        "Reference removed"
                    }
                );
                Ok(())
            })
            .await
            .unwrap();
        server.abort();
    }
    println!(
        "WS8bm2 older-calendar Rust: 4 groups; {frame_count} exact Rails frames; 4 silent rollbacks; post-commit reference snapshots"
    );
}
#[tokio::test]
async fn older_calendar_callback_bytes_match_rails_after_reference_removal() {
    callbacks(false).await;
}
#[tokio::test]
async fn older_calendar_callback_total_reads_are_flat() {
    callbacks(true).await;
}
#[tokio::test]
async fn older_calendar_callbacks_bound_association_preloads() {
    for case in oracle()["boundaries"].as_array().unwrap() {
        let app = app_rows(case["rows"].clone()).await;
        let count = case["count"].as_i64().unwrap();
        let base = case["base_id"].as_i64().unwrap();
        let id = case["event_id"].as_i64().unwrap();
        app.db().write(move |tx| {
            tx.conn().execute("WITH RECURSIVE ids(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM ids WHERE n<?1) INSERT INTO messages (id,room_id,creator_id,client_message_id,embeds_suppressed,created_at,updated_at) SELECT ?2+n,?3,?4,'bounded-event-'||n,1,'2026-03-01 16:00:00','2026-03-01 16:00:00' FROM ids",rusqlite::params![count,base,QUIET_CORNER,DAVID])?;
            tx.conn().execute("INSERT INTO event_references (message_id,event_id,created_at,updated_at) SELECT id,?1,'2026-03-01 16:00:00','2026-03-01 16:00:00' FROM messages WHERE id>?2",[id,base])?;Ok(())
        }).await.unwrap();
        let (mut client, server) = stream(&app).await;
        let expected = case.clone();
        let receive = tokio::spawn(async move {
            let mut digest = Sha256::new();
            for n in 1..=count {
                let actual: Value = serde_json::from_str(&client.next_text().await).unwrap();
                let html = actual["message"].as_str().unwrap();
                assert!(
                    html.contains(&format!("bounded-event-{n}\"")),
                    "frame order {n}"
                );
                if n == 1 {
                    assert_eq!(html, expected["first"].as_str().unwrap());
                }
                if n == count {
                    assert_eq!(html, expected["last"].as_str().unwrap());
                }
                digest.update(html.as_bytes());
                digest.update(b"\n");
            }
            assert_eq!(
                format!("{:x}", digest.finalize()),
                expected["sha256"].as_str().unwrap()
            );
        });
        let result = app.db().write(move |tx| title(tx, id, "After <&>")).await;
        if let Err(error) = result {
            receive.abort();
            server.abort();
            panic!("bounded event callback failed: {error}");
        }
        receive.await.unwrap();
        server.abort();
        println!("WS8bm2 older-calendar Rust boundary: {count} ordered frames; exact Rails SHA256");
    }
}

#[tokio::test]
async fn older_calendar_meet_jobs_match_rails_frames_and_retry_or_noop_outcomes() {
    use crate::app::{
        google_api_tests::{self as support, Recorded},
        google_test_support::QueueDrain,
    };
    use campfire_db::{
        Event, Timestamp,
        models::{
            google_account::{CALENDAR_SCOPE, ConnectionGrant, GoogleAccount},
            google_calendar::MeetLinkJob,
        },
    };
    use campfire_jobs::{JobQueue, QueueConfig, RunnerConfig};
    use std::time::Duration;
    const FIXTURE_TOKEN: &str = "fixture-calendar-access";
    let vector: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/older_calendar_jobs.json"
    ))
    .unwrap();
    for group in vector["groups"].as_array().unwrap() {
        for case in group["cases"].as_array().unwrap() {
            let app = app_rows(group["rows"].clone()).await;
            let id = group["event_id"].as_i64().unwrap();
            let name = case["name"].as_str().unwrap().to_owned();
            let setup = name.clone();
            let crypto = rails_compat::ar_encryption::ArEncryption::new(&app.booted.app.secrets);
            app.db().write(move |tx| {
                GoogleAccount::save_connection(tx,&crypto,ConnectionGrant{user_id:DAVID,email:"fixture@calendar.test".into(),access_token:Some(FIXTURE_TOKEN.into()),refresh_token:Some("fixture-calendar-refresh".into()),access_token_expires_at:Some(Timestamp::parse_db("2026-03-02 17:00:00").unwrap()),scopes:Some(CALENDAR_SCOPE.into())})?;
                match setup.as_str() {
                    "no_account"=>{tx.conn().execute("DELETE FROM google_accounts WHERE user_id=?",[DAVID])?;},
                    "disconnected"=>{tx.conn().execute("UPDATE google_accounts SET disconnected_reason='fixture-disconnected' WHERE user_id=?",[DAVID])?;},
                    "cancelled"=>{tx.conn().execute("UPDATE events SET cancelled_at=? WHERE id=?",rusqlite::params![tx.now(),id])?;},
                    "existing_link"=>{tx.conn().execute("UPDATE events SET meet_link='https://meet.example.test/existing' WHERE id=?",[id])?;},
                    "deleted"=>CalendarEvent::find(tx.conn(),id)?.destroy(tx)?,
                    _=>(),
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
            let mut drain = QueueDrain::install(&app).await;
            let mut registry = crate::jobs::Registry::new();
            crate::integrations::google::calendar_sync::register(&mut registry);
            let config = RunnerConfig::new(vec![QueueConfig::new("default", 1)]);
            let queue = JobQueue::new(&registry, &config).unwrap();
            let runner = campfire_jobs::start(
                app.db().clone(),
                queue,
                registry,
                app.booted.app.clone(),
                config,
            );
            app.db()
                .write(move |tx| {
                    tx.emit_after_commit(Event::job(&MeetLinkJob { event_id: id }));
                    Ok(())
                })
                .await
                .unwrap();
            if case["retry"].as_bool().unwrap() {
                assert!(
                    drain
                        .job_attempt(&app, "Calendar::MeetLinkJob", 1)
                        .await
                        .is_some()
                );
            } else {
                drain.calendar(&app).await;
            }
            super::comparison_support::published_frames(
                &app,
                &mut client,
                &case["frames"],
                &format!("job {name}"),
            )
            .await;
            client.assert_silent().await;
            runner.shutdown(Duration::from_secs(1)).await;
            let calls=recorded.calls.lock().unwrap().iter().map(|c| {assert_eq!(c["access_token"],FIXTURE_TOKEN);json!({"method":c["method"],"path":c["path"],"body":serde_json::from_str::<Value>(c["body"].as_str().unwrap()).unwrap()})}).collect::<Vec<_>>();
            assert_eq!(json!(calls), case["calls"], "job {name} owner API requests");
            assert!(recorded.answers.lock().unwrap().is_empty());
            let link = app
                .db()
                .read(move |c| match CalendarEvent::find(c, id) {
                    Ok(e) => Ok(e.meet_link),
                    Err(campfire_db::Error::RecordNotFound(_)) => Ok(None),
                    Err(e) => Err(e),
                })
                .await
                .unwrap();
            assert_eq!(json!(link), case["meet_link"], "job {name}");
            server.abort();
        }
    }
    println!(
        "WS8bm2 older-calendar jobs Rust: 16 real registered jobs; 20 exact Rails frames; 2 pending retries; 10 guarded no-ops; exact owner API requests"
    );
}
