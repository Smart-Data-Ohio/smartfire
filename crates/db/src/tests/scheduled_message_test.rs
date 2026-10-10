//! Rails ScheduledMessageTest (8) and DispatcherTest (13), plus claim/transaction checks.
use super::channel_thread_test::{create_thread, frozen, post_reply, post_root};
use super::*;
use crate::{
    ActivityItem, ChannelThread, Error, Message, NewScheduledMessage, Room, RoomType,
    ScheduledMessage, User,
};

fn schedule(
    t: &TestDb,
    thread: Option<i64>,
    reply: Option<i64>,
    source: &str,
    delay: i64,
) -> ScheduledMessage {
    let attrs = NewScheduledMessage {
        user_id: id("david"),
        room_id: id("watercooler"),
        thread_id: thread,
        reply_to_message_id: reply,
        markdown_source: source.into(),
        send_at: t.now().since(jiff::SignedDuration::from_secs(delay)),
    };
    t.write(move |tx| ScheduledMessage::create(tx, attrs))
}
fn reload(t: &TestDb, row: &ScheduledMessage) -> ScheduledMessage {
    t.read(|c| ScheduledMessage::find(c, row.id))
}
fn send(t: &TestDb, row: &ScheduledMessage, immediate: bool) -> bool {
    let id = row.id;
    t.write(move |tx| ScheduledMessage::dispatch(tx, id, tx.now(), immediate))
}
fn posted(t: &TestDb, row: &ScheduledMessage) -> Message {
    t.read(|c| {
        Message::find(
            c,
            ScheduledMessage::find(c, row.id)?.sent_message_id.unwrap(),
        )
    })
}
fn lose_access(t: &TestDb) {
    t.write(|tx| {
        tx.conn().execute(
            "DELETE FROM memberships WHERE room_id = ? AND user_id = ?",
            rusqlite::params![id("watercooler"), id("david")],
        )?;
        Ok(())
    });
}
fn drop_item(t: &TestDb, row: &ScheduledMessage) -> ActivityItem {
    t.read(|c| ActivityItem::find_by_user_and_source(c, id("david"), "ScheduledMessage", row.id))
        .unwrap()
}
fn count(t: &TestDb) -> i64 {
    t.read(|c| crate::sql::count(c, "SELECT COUNT(*) FROM messages", []))
}

#[test]
fn dispatch_rechecks_scheduled_reply_target_visibility() {
    let t = frozen();
    let target = post_root(&t, "watercooler", "jason", "Target");
    let row = schedule(&t, None, Some(target.id), "Later", 60);
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE messages SET system_note = 1 WHERE id = ?",
            [target.id],
        )?;
        Ok(())
    });
    assert!(!send(&t, &row, true));
    assert!(reload(&t, &row).dropped());
}

#[test]
fn scheduled_replies_dispatch_as_replies_and_deleted_targets_dispatch_as_plain_messages() {
    let t = frozen();
    let target = post_root(&t, "watercooler", "jason", "Target");
    let reply = schedule(&t, None, Some(target.id), "Reply", 60);
    let deleted = schedule(&t, None, Some(target.id), "Plain", 60);
    t.travel(120);
    assert!(send(&t, &reply, false));
    let sent = posted(&t, &reply);
    assert_eq!(sent.reply_to_message_id, Some(target.id));
    assert!(sent.reply_notify_author);
    assert!(
        t.read(|conn| ActivityItem::find_by_user_and_source(conn, id("jason"), "Message", sent.id))
            .is_some_and(|item| item.event_type == "reply")
    );
    t.write(move |tx| target.destroy(tx));
    assert!(send(&t, &deleted, false));
    let sent = posted(&t, &deleted);
    assert_eq!(sent.reply_to_message_id, None);
    assert!(!sent.reply());
}

#[test]
fn schedule_rejects_invisible_reply_targets_before_persisting() {
    let t = frozen();
    let target = post_root(&t, "watercooler", "jason", "Target");
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE messages SET system_note = 1 WHERE id = ?",
            [target.id],
        )?;
        Ok(())
    });
    let result = t.try_write(move |tx| {
        ScheduledMessage::create(
            tx,
            NewScheduledMessage {
                user_id: id("david"),
                room_id: id("watercooler"),
                thread_id: None,
                reply_to_message_id: Some(target.id),
                markdown_source: "Later".into(),
                send_at: tx.now().since(jiff::SignedDuration::from_hours(1)),
            },
        )
    });
    assert!(matches!(result, Err(Error::RecordInvalid(_))));
}

#[test]
fn schedules_for_the_future() {
    let t = frozen();
    let row = schedule(&t, None, None, "Morning!", 3600);
    assert!(row.pending() && t.read(|c| row.sendable(c)));
    assert_eq!(
        t.read(|c| ScheduledMessage::owned_by(c, id("david"), false)),
        [row]
    );
}

#[test]
fn rejects_past_times_blank_bodies_and_overlong_sources() {
    let t = frozen();
    for (source, delay) in [
        ("Hi".into(), -1),
        (" ".into(), 3600),
        ("é".repeat(50_001), 3600),
    ] {
        let attrs = NewScheduledMessage {
            user_id: id("david"),
            room_id: id("watercooler"),
            thread_id: None,
            reply_to_message_id: None,
            markdown_source: source,
            send_at: t.now().since(jiff::SignedDuration::from_secs(delay)),
        };
        assert!(matches!(
            t.try_write(move |tx| ScheduledMessage::create(tx, attrs)),
            Err(Error::RecordInvalid(_))
        ));
    }
}

#[test]
fn threads_and_replies_must_belong_to_the_scheduled_conversation() {
    let t = frozen();
    let other = create_thread(&t, "designers", "david", None, Some("Other"));
    let thread = create_thread(&t, "watercooler", "david", None, Some("Chat"));
    let target = post_reply(&t, thread.id, "david", "Target");
    for (thread_id, reply_id) in [
        (Some(other.id), None),
        (None, Some(target.id)),
        (Some(thread.id), Some(id("first"))),
    ] {
        let attrs = NewScheduledMessage {
            user_id: id("david"),
            room_id: id("watercooler"),
            thread_id,
            reply_to_message_id: reply_id,
            markdown_source: "Hi".into(),
            send_at: t.now().since(jiff::SignedDuration::from_hours(1)),
        };
        assert!(matches!(
            t.try_write(move |tx| ScheduledMessage::create(tx, attrs)),
            Err(Error::RecordInvalid(_))
        ));
    }
}

#[test]
fn unusable_rows_are_not_sendable() {
    let t = frozen();
    let row = schedule(&t, None, None, "Morning!", 3600);
    lose_access(&t);
    assert!(!t.read(|c| row.sendable(c)));
}

#[test]
fn deleting_a_sent_message_keeps_history_with_its_link_cleared() {
    let t = frozen();
    let row = schedule(&t, None, None, "Morning!", 3600);
    assert!(send(&t, &row, true));
    let message = posted(&t, &row);
    t.write(move |tx| message.destroy(tx));
    assert!(reload(&t, &row).sent());
    assert_eq!(reload(&t, &row).sent_message_id, None);
}

#[test]
fn deleting_a_reply_target_keeps_pending_history_with_its_link_cleared() {
    let t = frozen();
    let target = post_root(&t, "watercooler", "david", "Target");
    let row = schedule(&t, None, Some(target.id), "Reply soon", 3600);
    t.write(move |tx| target.destroy(tx));
    let row = reload(&t, &row);
    assert!(row.pending());
    assert_eq!(row.reply_to_message_id, None);
}

#[test]
fn deleting_a_thread_drops_pending_rows_with_an_inbox_item() {
    let t = frozen();
    let thread = create_thread(&t, "watercooler", "david", None, Some("Chat"));
    let target = post_reply(&t, thread.id, "david", "Target");
    let row = schedule(&t, Some(thread.id), Some(target.id), "Soon", 3600);
    t.write(move |tx| thread.destroy(tx));
    let row = reload(&t, &row);
    assert!(row.dropped());
    assert_eq!(row.drop_reason.as_deref(), Some("its thread was deleted"));
    assert_eq!(row.thread_id, None);
    assert_eq!(drop_item(&t, &row).event_type, "scheduled_message_dropped");
}

#[test]
fn deleting_a_thread_keeps_sent_history_with_the_thread_link_cleared() {
    let t = frozen();
    let thread = create_thread(&t, "watercooler", "david", None, Some("Chat"));
    let row = schedule(&t, Some(thread.id), None, "Hi", 3600);
    assert!(send(&t, &row, true));
    t.write(move |tx| thread.destroy(tx));
    let row = reload(&t, &row);
    assert!(row.sent());
    assert_eq!(row.thread_id, None);
}

#[test]
fn dispatch_due_posts_due_rows_as_the_author() {
    let t = frozen();
    let row = schedule(&t, None, None, "Morning!", 60);
    let future = schedule(&t, None, None, "Later", 7200);
    t.travel(120);
    let before = count(&t);
    assert_eq!(
        ScheduledMessage::dispatch_due(&t.db, t.now()).unwrap(),
        [row.id]
    );
    assert_eq!(count(&t), before + 1);
    let message = posted(&t, &row);
    assert_eq!(
        t.read(|c| message.plain_text_body(c, &BasicRichText)),
        "Morning!"
    );
    assert_eq!(message.creator_id, id("david"));
    assert!(reload(&t, &future).pending());
}

#[test]
fn each_row_sends_exactly_once_across_repeated_runs() {
    let t = frozen();
    let row = schedule(&t, None, None, "Once", 60);
    t.travel(120);
    assert!(send(&t, &row, false));
    let before = count(&t);
    assert!(!send(&t, &row, false));
    assert_eq!(count(&t), before);
}

#[test]
fn a_moved_send_time_fires_at_the_new_time() {
    let t = frozen();
    let mut row = schedule(&t, None, None, "Moved", 60);
    t.travel(120);
    let later = t.now().since(jiff::SignedDuration::from_hours(2));
    let saved = row.clone();
    t.write(move |tx| row.update(tx, "Moved", later, row.reply_to_message_id));
    assert!(!send(&t, &saved, false));
    assert!(reload(&t, &saved).pending());
    assert_eq!(reload(&t, &saved).claimed_at, None);
    t.travel(7201);
    assert!(send(&t, &saved, false));
}

#[test]
fn rows_whose_author_lost_access_drop_with_an_inbox_item() {
    let t = frozen();
    let row = schedule(&t, None, None, "Stranded", 60);
    lose_access(&t);
    t.travel(120);
    let before = count(&t);
    assert!(!send(&t, &row, false));
    assert!(reload(&t, &row).dropped());
    assert_eq!(count(&t), before);
    assert_eq!(drop_item(&t, &row).event_type, "scheduled_message_dropped");
}

#[test]
fn drop_items_are_private_to_the_author() {
    let t = frozen();
    let row = schedule(&t, None, None, "Stranded", 60);
    lose_access(&t);
    assert!(!send(&t, &row, true));
    assert_eq!(drop_item(&t, &row).user_id, id("david"));
    assert!(
        t.read(|c| ActivityItem::find_by_user_and_source(
            c,
            id("jason"),
            "ScheduledMessage",
            row.id
        ))
        .is_none()
    );
}

#[test]
fn rows_in_soft_deleted_rooms_are_dropped() {
    let t = frozen();
    let row = schedule(&t, None, None, "Doomed", 60);
    t.write(|tx| {
        tx.conn().execute(
            "UPDATE rooms SET deleted_at = ? WHERE id = ?",
            rusqlite::params![tx.now(), id("watercooler")],
        )?;
        Ok(())
    });
    t.travel(120);
    assert_eq!(
        t.read(|c| ScheduledMessage::due_candidate_ids(c, t.now())),
        [row.id]
    );
    assert!(!send(&t, &row, false));
    assert_eq!(
        reload(&t, &row).drop_reason.as_deref(),
        Some("its room was deleted")
    );
    assert_eq!(drop_item(&t, &row).user_id, id("david"));
}

#[test]
fn thread_rows_post_inside_the_thread() {
    let t = frozen();
    let thread = create_thread(&t, "watercooler", "david", None, Some("Chat"));
    let row = schedule(&t, Some(thread.id), None, "Hi", 60);
    assert!(send(&t, &row, true));
    assert_eq!(posted(&t, &row).thread_id, Some(thread.id));
    assert_eq!(
        t.read(|c| ChannelThread::find(c, thread.id)).messages_count,
        1
    );
}

#[test]
fn thread_posts_never_fan_out_to_legacy_bots_and_root_posts_do() {
    let t = frozen();
    let legacy = t.write(|tx| {
        let bot = User::create_bot(tx, "Legacy Note", Some("https://example.test/legacy-note"))?;
        Room::find(tx.conn(), id("watercooler"))?.grant_to(tx, &[bot.id])?;
        Ok(bot)
    });
    let thread = create_thread(&t, "watercooler", "david", None, Some("Chat"));
    let row = schedule(&t, Some(thread.id), None, "Hey @[Legacy Note]", 60);
    let from = t.events().len();
    assert!(send(&t, &row, true));
    assert!(
        !t.events()[from..]
            .iter()
            .any(|e| matches!(e, Event::DeliverWebhook { .. }))
    );
    let root = schedule(&t, None, None, "Hey @[Legacy Note]", 60);
    let from = t.events().len();
    assert!(send(&t, &root, true));
    assert!(
        t.events()[from..]
            .iter()
            .any(|e| matches!(e, Event::DeliverWebhook { bot_id, .. } if *bot_id == legacy.id))
    );
}

#[test]
fn locked_threads_retry_instead_of_dropping() {
    let t = frozen();
    let thread = create_thread(&t, "watercooler", "david", None, Some("Chat"));
    let row = schedule(&t, Some(thread.id), None, "Hi", 60);
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE channel_threads SET locked_at = ? WHERE id = ?",
            rusqlite::params![tx.now(), thread.id],
        )?;
        Ok(())
    });
    assert!(!send(&t, &row, true));
    let saved = reload(&t, &row);
    assert!(saved.pending());
    assert_eq!(saved.claimed_at, None);
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE channel_threads SET locked_at = NULL WHERE id = ?",
            [thread.id],
        )?;
        Ok(())
    });
    assert!(send(&t, &row, true));
}

#[test]
fn send_time_validation_failures_drop_the_row_with_the_reason() {
    let t = frozen();
    let row = t.write(|tx| {
        let board = Room::create_for(
            tx,
            RoomType::Board,
            Some("Launch"),
            id("david"),
            &[id("david")],
        )?;
        ScheduledMessage::create(
            tx,
            NewScheduledMessage {
                user_id: id("david"),
                room_id: board.id,
                thread_id: None,
                reply_to_message_id: None,
                markdown_source: "Root post".into(),
                send_at: tx.now().since(jiff::SignedDuration::from_hours(1)),
            },
        )
    });
    let before = count(&t);
    assert!(!send(&t, &row, true));
    assert!(reload(&t, &row).drop_reason.unwrap().contains("board"));
    assert!(!send(&t, &row, true));
    assert_eq!(count(&t), before);
    assert_eq!(drop_item(&t, &row).event_type, "scheduled_message_dropped");
}

#[test]
fn a_row_dropped_after_a_claim_posts_nothing() {
    let t = frozen();
    let thread = create_thread(&t, "watercooler", "david", None, Some("Chat"));
    let row = schedule(&t, Some(thread.id), None, "Hi", 60);
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE scheduled_messages SET claimed_at = ? WHERE id = ?",
            rusqlite::params![tx.now(), row.id],
        )?;
        thread.destroy(tx)
    });
    let before = count(&t);
    assert!(!send(&t, &row, true));
    assert_eq!(count(&t), before);
    assert!(reload(&t, &row).dropped());
}

#[test]
fn dispatch_now_sends_immediately_and_reports_drops() {
    let t = frozen();
    let row = schedule(&t, None, None, "Now", 7200);
    assert!(send(&t, &row, true));
    let stranded = schedule(&t, None, None, "Stranded", 7200);
    lose_access(&t);
    assert!(!send(&t, &stranded, true));
    assert!(reload(&t, &stranded).dropped());
}

#[test]
fn dispatched_messages_never_start_a_stream() {
    let t = frozen();
    let row = schedule(&t, None, None, "Morning!", 60);
    assert!(send(&t, &row, true));
    let message = posted(&t, &row);
    assert!(!message.streaming);
    assert!(message.streaming_updated_at.is_none());
}

#[test]
fn a_live_claim_blocks_dispatch_and_a_stale_claim_is_recovered() {
    let t = frozen();
    let row = schedule(&t, None, None, "Once", 60);
    t.travel(120);
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE scheduled_messages SET claimed_at = ? WHERE id = ?",
            rusqlite::params![tx.now(), row.id],
        )?;
        Ok(())
    });
    assert!(!send(&t, &row, false));
    t.travel(300);
    assert!(reload(&t, &row).claimed(t.now()));
    assert!(!send(&t, &row, true));
    t.travel(1);
    assert!(!reload(&t, &row).claimed(t.now()));
    assert!(send(&t, &row, false));
}

#[test]
fn concurrent_dispatchers_post_once_on_real_sqlite() {
    let t = frozen();
    let row = schedule(&t, None, None, "Once", 60);
    t.travel(120);
    let now = t.now();
    let handles: Vec<_> = (0..4).map(|_| t.another_process()).collect();
    let barrier = Arc::new(std::sync::Barrier::new(4));
    let before = count(&t);
    let wins: Vec<_> = std::thread::scope(|scope| {
        let tasks: Vec<_> = handles
            .iter()
            .map(|db| {
                let barrier = barrier.clone();
                scope.spawn(move || {
                    barrier.wait();
                    db.write_blocking(move |tx| ScheduledMessage::dispatch(tx, row.id, now, false))
                        .unwrap()
                })
            })
            .collect();
        tasks.into_iter().map(|t| t.join().unwrap()).collect()
    });
    assert_eq!(wins.iter().filter(|win| **win).count(), 1);
    assert_eq!(count(&t), before + 1);
    assert!(reload(&t, &row).sent());
}

#[test]
fn posting_failure_rolls_back_the_claim_and_can_retry() {
    let t = frozen();
    let row = schedule(&t, None, None, "Retry", 60);
    t.write(|tx| { tx.conn().execute_batch("CREATE TRIGGER reject_scheduled_post BEFORE INSERT ON messages BEGIN SELECT RAISE(ABORT, 'injected post failure'); END")?; Ok(()) });
    let before = count(&t);
    let from = t.events().len();
    assert!(
        t.try_write(move |tx| ScheduledMessage::dispatch(tx, row.id, tx.now(), true))
            .is_err()
    );
    assert_eq!(reload(&t, &row).claimed_at, None);
    assert_eq!(count(&t), before);
    assert_eq!(t.events().len(), from);
    t.write(|tx| {
        tx.conn()
            .execute_batch("DROP TRIGGER reject_scheduled_post")?;
        Ok(())
    });
    assert!(send(&t, &row, true));
}

#[test]
fn deactivated_authors_are_not_due_and_history_orders_newest_first() {
    let t = frozen();
    let first = schedule(&t, None, None, "First", 60);
    let second = schedule(&t, None, None, "Second", 120);
    assert!(send(&t, &first, true));
    assert!(send(&t, &second, true));
    assert_eq!(
        t.read(|c| ScheduledMessage::owned_by(c, id("david"), true))
            .iter()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        [second.id, first.id]
    );
    schedule(&t, None, None, "Inactive", 60);
    t.write(|tx| User::find(tx.conn(), id("david"))?.deactivate(tx));
    t.travel(120);
    assert!(
        t.read(|c| ScheduledMessage::due_candidate_ids(c, t.now()))
            .is_empty()
    );
}
