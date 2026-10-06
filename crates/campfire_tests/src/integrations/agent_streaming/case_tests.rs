//! Pinned MessageStreamingTest sweep comparisons through the production periodic callback.
use crate::controllers::presenters::test_support::{ALL_TALK, BENDER, DAVID, TestApp};
use campfire_db::{ChannelThread, Message, NewChannelThread, NewMessage, Room};
use rusqlite::params;
async fn setup() -> TestApp {
    let t = TestApp::boot().await.expect("default seed");
    t.db()
        .write(|tx| {
            tx.conn().execute("DELETE FROM agent_grants", [])?;
            tx.conn().execute("DELETE FROM agent_events", [])?;
            Room::find(tx.conn(), ALL_TALK)?.grant_to(tx, &[BENDER])?;
            Ok(())
        })
        .await
        .unwrap();
    t
}
fn create(
    tx: &mut campfire_db::Tx<'_>,
    text: &str,
    thread: Option<i64>,
) -> campfire_db::Result<i64> {
    Ok(Message::create(
        tx,
        NewMessage {
            room_id: ALL_TALK,
            creator_id: BENDER,
            markdown_source: Some(text.into()),
            thread_id: thread,
            streaming: true,
            ..Default::default()
        },
    )?
    .id)
}
fn age(tx: &campfire_db::Tx<'_>, mid: i64, created: i64, idle: i64) -> campfire_db::Result<()> {
    tx.conn().execute(
        "UPDATE messages SET created_at=?,streaming_updated_at=? WHERE id=?",
        params![
            tx.now().ago(jiff::SignedDuration::from_mins(created)),
            tx.now().ago(jiff::SignedDuration::from_mins(idle)),
            mid
        ],
    )?;
    Ok(())
}
#[tokio::test]
async fn ws11_stream_case_sweep_finalizes_overdue_not_fresh() {
    let t = setup().await;
    let (fresh, old) = t
        .db()
        .write(|tx| {
            let fresh = create(tx, "Fresh", None)?;
            let old = create(tx, "Old hovercraft", None)?;
            age(tx, old, 11, 11)?;
            Ok((fresh, old))
        })
        .await
        .unwrap();
    crate::jobs::periodic::streaming_messages(t.db())
        .await
        .unwrap();
    t.db()
        .read(move |c| {
            assert!(Message::find(c, fresh)?.streaming);
            assert!(!Message::find(c, old)?.streaming);
            assert_eq!(
                c.query_row(
                    "SELECT COUNT(*) FROM message_search_index WHERE rowid=?",
                    [old],
                    |r| r.get::<_, i64>(0)
                )?,
                1
            );
            assert_eq!(
                c.query_row(
                    "SELECT COUNT(*) FROM agent_events WHERE event_type='posted' AND message_id=?",
                    [old],
                    |r| r.get::<_, i64>(0)
                )?,
                1
            );
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn ws11_stream_case_sweep_keys_on_inactivity_not_creation() {
    let t = setup().await;
    let (active, idle) = t
        .db()
        .write(|tx| {
            let active = create(tx, "Still going", None)?;
            let idle = create(tx, "Gone quiet", None)?;
            age(tx, active, 20, 1)?;
            age(tx, idle, 20, 11)?;
            Ok((active, idle))
        })
        .await
        .unwrap();
    crate::jobs::periodic::streaming_messages(t.db())
        .await
        .unwrap();
    t.db()
        .read(move |c| {
            assert!(Message::find(c, active)?.streaming);
            assert!(!Message::find(c, idle)?.streaming);
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn ws11_stream_case_sweep_skips_locked_thread_until_unlock() {
    let t = setup().await;
    let (mid, thread) = t
        .db()
        .write(|tx| {
            let thread = ChannelThread::create(
                tx,
                NewChannelThread {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    name: Some("Locked stream".into()),
                    ..Default::default()
                },
            )?;
            let mid = create(tx, "Waiting", Some(thread.id))?;
            age(tx, mid, 11, 11)?;
            tx.conn().execute(
                "UPDATE channel_threads SET locked_at=? WHERE id=?",
                params![tx.now(), thread.id],
            )?;
            Ok((mid, thread.id))
        })
        .await
        .unwrap();
    crate::jobs::periodic::streaming_messages(t.db())
        .await
        .unwrap();
    t.db()
        .read(move |c| {
            assert!(Message::find(c, mid)?.streaming);
            assert_eq!(
                c.query_row(
                    "SELECT COUNT(*) FROM agent_events WHERE message_id=?",
                    [mid],
                    |r| r.get::<_, i64>(0)
                )?,
                0
            );
            Ok(())
        })
        .await
        .unwrap();
    t.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE channel_threads SET locked_at=NULL WHERE id=?",
                [thread],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    crate::jobs::periodic::streaming_messages(t.db())
        .await
        .unwrap();
    t.db()
        .read(move |c| {
            assert!(!Message::find(c, mid)?.streaming);
            assert_eq!(
                c.query_row(
                    "SELECT COUNT(*) FROM agent_events WHERE event_type='posted' AND message_id=?",
                    [mid],
                    |r| r.get::<_, i64>(0)
                )?,
                1
            );
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn ws11_stream_case_sweep_suspended_agent_quiet() {
    let t = setup().await;
    t.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE agents SET suspended_at=? WHERE user_id=?",
                params![tx.now(), BENDER],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mid = t
        .db()
        .write(|tx| {
            let mid = create(tx, "Gone quiet hovercraft", None)?;
            age(tx, mid, 11, 11)?;
            Ok(mid)
        })
        .await
        .unwrap();
    // A queued side effect for this message must fail the originating claim,
    // even if a worker would immediately drain it before our subsequent read.
    t.db().write(move|tx|{tx.conn().execute_batch(&format!("CREATE TRIGGER ws11_quiet_sweep_job BEFORE INSERT ON background_jobs WHEN instr(NEW.arguments, '{mid}') > 0 BEGIN SELECT RAISE(ABORT, 'quiet sweep enqueued a message job'); END;"))?;Ok(())}).await.unwrap();
    crate::jobs::periodic::streaming_messages(t.db())
        .await
        .unwrap();
    t.db()
        .read(move |c| {
            assert!(!Message::find(c, mid)?.streaming);
            assert_eq!(
                c.query_row(
                    "SELECT COUNT(*) FROM agent_events WHERE message_id=?",
                    [mid],
                    |r| r.get::<_, i64>(0)
                )?,
                0
            );
            assert_eq!(
                c.query_row(
                    "SELECT COUNT(*) FROM message_search_index WHERE rowid=?",
                    [mid],
                    |r| r.get::<_, i64>(0)
                )?,
                0
            );
            Ok(())
        })
        .await
        .unwrap();
}
