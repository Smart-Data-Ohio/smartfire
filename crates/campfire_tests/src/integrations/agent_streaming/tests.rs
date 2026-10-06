use campfire_db::models::agent_streaming as domain;
use campfire_db::models::agent_streaming::StreamTrailingBroadcastJob;
use crate::controllers::presenters::test_support::{ALL_TALK, BENDER, DAVID, TestApp};
use campfire_db::models::agent_posting::PostResult;
use campfire_db::{ChannelThread, Event, Message, NewChannelThread, NewMessage, Room};
use rusqlite::params;
async fn setup(test: &TestApp) -> (i64, i64) {
    test.db()
        .write(|tx| {
            let agent: i64 = tx.conn().query_row(
                "SELECT id FROM agents WHERE user_id=?",
                [BENDER],
                |r| r.get(0),
            )?;
            tx.conn()
                .execute("DELETE FROM agent_grants WHERE agent_id=?", [agent])?;
            Room::find(tx.conn(), ALL_TALK)?.grant_to(tx, &[BENDER])?;
            let PostResult::Posted(m) = domain::start(
                tx,
                agent,
                NewMessage {
                    room_id: ALL_TALK,
                    markdown_source: Some("A".into()),
                    client_message_id: Some("ws11-app-stream".into()),
                    ..Default::default()
                },
            )?
            else {
                panic!("stream start");
            };
            Ok((agent, m.id))
        })
        .await
        .unwrap()
}
#[tokio::test]
async fn ws11_stream_trailing_is_registered_and_runs_from_the_durable_queue() {
    let test = TestApp::boot().await.expect("default seed");
    let (_, id) = setup(&test).await;
    let old = test
        .db()
        .env()
        .now()
        .ago(jiff::SignedDuration::from_secs(1));
    test.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE messages SET stream_broadcast_at=? WHERE id=?",
                params![old, id],
            )?;
            tx.emit_after_commit(Event::job(&StreamTrailingBroadcastJob {
                message_id: id,
                last_broadcast_at: domain::stamp(old),
            }));
            Ok(())
        })
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let done = test
                .db()
                .read(move |conn| {
                    Ok(Message::find(conn, id)?
                        .stream_broadcast_at
                        .is_some_and(|t| t > old))
                })
                .await
                .unwrap();
            if done {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("registered durable trailing worker ran");
}
#[tokio::test]
async fn ws11_stream_rejected_trailing_job_rolls_back_the_append() {
    let test = TestApp::boot().await.expect("default seed");
    let (agent, id) = setup(&test).await;
    test.db().write(move |tx| {
        tx.conn().execute("UPDATE messages SET stream_broadcast_at=? WHERE id=?",params![tx.now(),id])?;
        tx.conn().execute_batch("CREATE TRIGGER ws11_reject_trailing BEFORE INSERT ON background_jobs WHEN NEW.job_class='Message::StreamTrailingBroadcastJob' BEGIN SELECT RAISE(ABORT,'WS11 rejected trailing job'); END;")?;Ok(())
    }).await.unwrap();
    assert!(
        test.db()
            .write(move |tx| domain::update(tx, agent, id, Some("B"), None))
            .await
            .is_err()
    );
    test.db()
        .read(move |conn| {
            assert_eq!(
                Message::find(conn, id)?.markdown_source.as_deref(),
                Some("A")
            );
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn ws11_stream_sweep_finalizes_root_and_waits_for_locked_threads() {
    let test = TestApp::boot().await.expect("default seed");
    let (_, id) = setup(&test).await;
    let thread_message = test
        .db()
        .write(move |tx| {
            let thread = ChannelThread::create(
                tx,
                NewChannelThread {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    name: Some("Locked sweep".into()),
                    ..Default::default()
                },
            )?;
            let m = Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: BENDER,
                    thread_id: Some(thread.id),
                    markdown_source: Some("Still locked".into()),
                    streaming: true,
                    ..Default::default()
                },
            )?;
            tx.conn().execute(
                "UPDATE channel_threads SET locked_at=? WHERE id=?",
                params![tx.now(), thread.id],
            )?;
            let old = tx.now().ago(jiff::SignedDuration::from_mins(11));
            tx.conn().execute(
                "UPDATE messages SET streaming_updated_at=? WHERE id IN (?,?)",
                params![old, id, m.id],
            )?;
            Ok(m.id)
        })
        .await
        .unwrap();
    crate::jobs::periodic::streaming_messages(test.db())
        .await
        .unwrap();
    test.db()
        .read(move |conn| {
            assert!(!Message::find(conn, id)?.streaming);
            assert!(Message::find(conn, thread_message)?.streaming);
            Ok(())
        })
        .await
        .unwrap();
}
