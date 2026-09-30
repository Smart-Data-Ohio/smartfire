//! WS8 writes and WS3's real durable queue share one SQLite transaction.
use super::*;
use campfire_db::{
    CachedStatements, ChannelThread, Message, NewChannelThread, NewMessage, NewSavedItem,
    NewScheduledMessage, NewUser, Room, RoomType, SavedItem, ScheduledMessage, User,
};

fn messaging_harness() -> (Harness, i64, i64, i64) {
    let h = harness(&Registry::<()>::new(), &config());
    let (user, room, thread) =
        h.db.write_blocking(|tx| {
            let user = User::create(
                tx,
                NewUser {
                    name: "Sender".into(),
                    ..Default::default()
                },
            )?;
            let room = Room::create_for(
                tx,
                RoomType::Closed,
                Some("Queue checks"),
                user.id,
                &[user.id],
            )?;
            let thread = ChannelThread::create(
                tx,
                NewChannelThread {
                    room_id: room.id,
                    creator_id: user.id,
                    name: Some("Thread".into()),
                    ..Default::default()
                },
            )?;
            Ok((user.id, room.id, thread.id))
        })
        .unwrap();
    (h, user, room, thread)
}

fn reject_jobs(h: &Harness) {
    h.db.write_blocking(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER reject_ws8_job BEFORE INSERT ON background_jobs BEGIN SELECT RAISE(ABORT, 'queue unavailable'); END")?;
        Ok(())
    }).unwrap();
}

fn restore_jobs(h: &Harness) {
    h.db.write_blocking(|tx| {
        tx.conn().execute_batch("DROP TRIGGER reject_ws8_job")?;
        Ok(())
    })
    .unwrap();
}

fn message_count(h: &Harness) -> i64 {
    h.db.read_blocking(
        |conn| Ok(conn.query_row("SELECT count(*) FROM messages", [], |r| r.get(0))?),
    )
    .unwrap()
}

#[test]
fn thread_post_rolls_back_with_a_failed_durable_enqueue() {
    let (h, user, _, thread) = messaging_harness();
    let before =
        h.db.read_blocking(|conn| ChannelThread::find(conn, thread))
            .unwrap();
    let memberships: i64 =
        h.db.read_blocking(|conn| {
            Ok(conn.query_row("SELECT count(*) FROM thread_memberships", [], |r| r.get(0))?)
        })
        .unwrap();
    let post = move |tx: &mut Tx<'_>| {
        ChannelThread::find(tx.conn(), thread)?.post_message(
            tx,
            user,
            NewMessage {
                markdown_source: Some("Durable thread post".into()),
                ..Default::default()
            },
        )
    };
    reject_jobs(&h);
    assert!(h.db.write_blocking(post).is_err());
    assert_eq!(message_count(&h), 0);
    assert_eq!(
        h.db.read_blocking(|conn| ChannelThread::find(conn, thread))
            .unwrap(),
        before
    );
    assert_eq!(
        h.db.read_blocking(|conn| Ok(conn.query_row::<i64, _, _>(
            "SELECT count(*) FROM thread_memberships",
            [],
            |r| r.get(0)
        )?))
        .unwrap(),
        memberships
    );
    assert!(h.jobs().is_empty());
    restore_jobs(&h);
    let message = h.db.write_blocking(post).unwrap();
    let jobs = h.jobs();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].class, "ChannelThread::PushMessageJob");
    assert_eq!(
        jobs[0].arguments,
        serde_json::json!({"thread_id": thread, "message_id": message.id})
    );
    assert_eq!(jobs[0].status, READY);
}

#[test]
fn saved_reminder_rolls_back_claim_and_activity_with_a_failed_enqueue() {
    let (h, user, room, _) = messaging_harness();
    let saved =
        h.db.write_blocking(move |tx| {
            let message = Message::create(
                tx,
                NewMessage {
                    room_id: room,
                    creator_id: user,
                    markdown_source: Some("Remember this".into()),
                    ..Default::default()
                },
            )?;
            let item = SavedItem::create(
                tx,
                NewSavedItem {
                    user_id: user,
                    message_id: message.id,
                    remind_at: Some(tx.now().since(jiff::SignedDuration::from_secs(1))),
                    ..Default::default()
                },
            )?;
            Ok(item.id)
        })
        .unwrap();
    h.db.write_blocking(|tx| {
        tx.conn()
            .execute_cached("DELETE FROM background_jobs", [])?;
        Ok(())
    })
    .unwrap();
    h.travel(2);
    reject_jobs(&h);
    let dispatch = move |tx: &mut Tx<'_>| SavedItem::dispatch_reminder(tx, saved, tx.now());
    assert!(h.db.write_blocking(dispatch).is_err());
    assert!(
        h.db.read_blocking(|conn| SavedItem::find(conn, saved))
            .unwrap()
            .reminded_at
            .is_none()
    );
    assert_eq!(
        h.db.read_blocking(|conn| Ok(conn.query_row::<i64, _, _>(
            "SELECT count(*) FROM activity_items",
            [],
            |r| r.get(0)
        )?))
        .unwrap(),
        0
    );
    assert!(h.jobs().is_empty());
    restore_jobs(&h);
    assert!(h.db.write_blocking(dispatch).unwrap());
    assert!(!h.db.write_blocking(dispatch).unwrap());
    let jobs = h.jobs();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].class, "SavedItem::ReminderPushJob");
    assert_eq!(
        jobs[0].arguments,
        serde_json::json!({"saved_item_id": saved})
    );
    assert_eq!(jobs[0].status, READY);
}

#[test]
fn scheduled_thread_post_rolls_back_claim_and_history_with_a_failed_enqueue() {
    let (h, user, room, thread) = messaging_harness();
    let scheduled =
        h.db.write_blocking(move |tx| {
            ScheduledMessage::create(
                tx,
                NewScheduledMessage {
                    user_id: user,
                    room_id: room,
                    thread_id: Some(thread),
                    reply_to_message_id: None,
                    markdown_source: "Scheduled durable post".into(),
                    send_at: tx.now().since(jiff::SignedDuration::from_secs(1)),
                },
            )
        })
        .unwrap()
        .id;
    reject_jobs(&h);
    let dispatch = move |tx: &mut Tx<'_>| ScheduledMessage::dispatch(tx, scheduled, tx.now(), true);
    assert!(h.db.write_blocking(dispatch).is_err());
    let row =
        h.db.read_blocking(|conn| ScheduledMessage::find(conn, scheduled))
            .unwrap();
    assert!(row.claimed_at.is_none() && row.sent_at.is_none() && row.sent_message_id.is_none());
    assert_eq!(message_count(&h), 0);
    assert!(h.jobs().is_empty());
    restore_jobs(&h);
    assert!(h.db.write_blocking(dispatch).unwrap());
    assert!(!h.db.write_blocking(dispatch).unwrap());
    let row =
        h.db.read_blocking(|conn| ScheduledMessage::find(conn, scheduled))
            .unwrap();
    let jobs = h.jobs();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].class, "ChannelThread::PushMessageJob");
    assert_eq!(
        jobs[0].arguments,
        serde_json::json!({"thread_id": thread, "message_id": row.sent_message_id.unwrap()})
    );
}

#[test]
fn source_edit_rolls_back_with_a_failed_quote_refresh_enqueue() {
    let (h, user, room, _) = messaging_harness();
    let source =
        h.db.write_blocking(move |tx| {
            Message::create(
                tx,
                NewMessage {
                    room_id: room,
                    creator_id: user,
                    body: Some("original source".into()),
                    ..Default::default()
                },
            )
        })
        .unwrap()
        .id;
    h.db.write_blocking(|tx| {
        tx.conn()
            .execute_cached("DELETE FROM background_jobs", [])?;
        Ok(())
    })
    .unwrap();
    reject_jobs(&h);
    let edit = move |tx: &mut Tx<'_>| {
        let mut message = Message::find(tx.conn(), source)?;
        message.edit(
            tx,
            campfire_db::MessageChanges {
                body: Some("changed source".into()),
                ..Default::default()
            },
        )
    };
    assert!(h.db.write_blocking(edit).is_err());
    assert_eq!(
        h.db.read_blocking(|conn| Message::find(conn, source)?.body_html(conn))
            .unwrap()
            .as_deref(),
        Some("original source")
    );
    assert!(h.jobs().is_empty());
    restore_jobs(&h);
    h.db.write_blocking(edit).unwrap();
    let jobs = h.jobs();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].class, "Message::QuoteCardsRefreshJob");
    assert_eq!(
        jobs[0].arguments,
        serde_json::json!({"source_message_id":source})
    );
}
