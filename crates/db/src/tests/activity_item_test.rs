//! app/models/activity_item.rb: partial writes from stale instances on independent writers.
use super::*;
use crate::{ActivityItem, Timestamp};

fn item(t: &TestDb, read: bool, handled: bool) -> ActivityItem {
    t.write(move |tx| {
        let item =
            ActivityItem::refresh_unread(tx, id("david"), "Message", id("first"), "mention")?;
        tx.conn().execute(
            "UPDATE activity_items SET read_at=?,handled_at=? WHERE id=?",
            rusqlite::params![
                read.then_some(tx.now()),
                handled.then_some(tx.now()),
                item.id
            ],
        )?;
        ActivityItem::find(tx.conn(), item.id)
    })
}

fn frozen_db() -> TestDb {
    TestDb::with_clock(TestClock::frozen_at(Timestamp::from_second(1772467200)), 4)
}

fn assert_saved(
    t: &TestDb,
    saved: &ActivityItem,
    read: Option<Timestamp>,
    handled: Option<Timestamp>,
) {
    assert_eq!(saved.read_at, read);
    assert_eq!(saved.handled_at, handled);
    assert_eq!(saved.updated_at, t.now());
    let id = saved.id;
    assert_eq!(t.read(move |conn| ActivityItem::find(conn, id)), *saved);
    assert_eq!(t.sink.take().len(), 1, "one after-commit state broadcast");
}

#[test]
fn ws12_activity_stale_read_preserves_concurrent_handling() {
    let t = frozen_db();
    let a = item(&t, false, false);
    let b = t.read(|conn| ActivityItem::find(conn, a.id));
    let other = t.another_process();
    t.travel(1);
    let handled = t.write(move |tx| a.mark_handled(tx));
    t.sink.take();
    t.travel(1);
    let read = other.write_blocking(move |tx| b.mark_read(tx)).unwrap();
    assert_saved(&t, &read, Some(t.now()), handled.handled_at);
}

#[test]
fn ws12_activity_stale_unhandle_does_not_restore_concurrently_cleared_read() {
    let t = frozen_db();
    let a = item(&t, true, true);
    let b = t.read(|conn| ActivityItem::find(conn, a.id));
    let other = t.another_process();
    t.travel(1);
    t.write(move |tx| a.mark_unread(tx));
    t.sink.take();
    t.travel(1);
    let unhandled = other
        .write_blocking(move |tx| b.mark_unhandled(tx))
        .unwrap();
    assert_saved(&t, &unhandled, None, None);
}

#[test]
fn ws12_activity_stale_unread_only_clears_previously_read_column() {
    let t = frozen_db();
    let a = item(&t, true, false);
    let b = t.read(|conn| ActivityItem::find(conn, a.id));
    let other = t.another_process();
    t.travel(1);
    let handled = t.write(move |tx| a.mark_handled(tx));
    t.sink.take();
    t.travel(1);
    let unread = other.write_blocking(move |tx| b.mark_unread(tx)).unwrap();
    assert_saved(&t, &unread, None, handled.handled_at);
}

#[test]
fn ws12_activity_stale_unread_only_clears_previously_handled_column() {
    let t = frozen_db();
    let a = item(&t, false, true);
    let b = t.read(|conn| ActivityItem::find(conn, a.id));
    let other = t.another_process();
    t.travel(1);
    let read = t.write(move |tx| a.mark_read(tx));
    t.sink.take();
    t.travel(1);
    let unread = other.write_blocking(move |tx| b.mark_unread(tx)).unwrap();
    assert_saved(&t, &unread, read.read_at, None);
}

#[test]
fn ws12_activity_stale_handle_does_not_restore_concurrently_cleared_read() {
    let t = frozen_db();
    let a = item(&t, true, false);
    let b = t.read(|conn| ActivityItem::find(conn, a.id));
    let other = t.another_process();
    t.travel(1);
    t.write(move |tx| a.mark_unread(tx));
    t.sink.take();
    t.travel(1);
    let handled = other.write_blocking(move |tx| b.mark_handled(tx)).unwrap();
    assert_saved(&t, &handled, None, Some(t.now()));
}

#[test]
fn ws12_activity_stale_handle_writes_its_dirty_read_timestamp() {
    let t = frozen_db();
    let a = item(&t, false, false);
    let b = t.read(|conn| ActivityItem::find(conn, a.id));
    let other = t.another_process();
    t.travel(1);
    t.write(move |tx| a.mark_read(tx));
    t.sink.take();
    t.travel(1);
    let handled = other.write_blocking(move |tx| b.mark_handled(tx)).unwrap();
    assert_saved(&t, &handled, Some(t.now()), Some(t.now()));
}

fn huddle_item(t: &TestDb, handled: bool) -> ActivityItem {
    t.write(move |tx| {
        let membership = crate::Membership::find_by_room_and_user(
            tx.conn(),
            id("designers"),
            id("david"),
        )?
        .unwrap();
        let grant: i64 = tx.conn().query_row(
            "INSERT INTO huddle_grants (identity,room_name,session_id,user_id,membership_id,room_id,created_at,updated_at,last_issued_at) VALUES ('ws12-stale-callback','ws12-stale-callback',?,?,?,?,?,?,?) RETURNING id",
            rusqlite::params![id("david_safari"), id("david"), membership.id, id("designers"), tx.now(), tx.now(), tx.now()],
            |row| row.get(0),
        )?;
        let item = ActivityItem::refresh_unread(tx, id("jason"), "HuddleGrant", grant, "huddle_started")?;
        if handled {
            item.mark_handled(tx)
        } else {
            Ok(item)
        }
    })
}

fn assert_huddle_callback(t: &TestDb, saved: &ActivityItem, persisted_state: &str) {
    assert_eq!(saved.state(), persisted_state);
    let id = saved.id;
    assert_eq!(t.read(move |conn| ActivityItem::find(conn, id)), *saved);
    let events = t.sink.take();
    let frames: Vec<_> = events
        .iter()
        .filter_map(|event| match event.as_broadcast()? {
            crate::broadcasts::Broadcast::Cable { stream, payload } => Some((stream, payload)),
            _ => None,
        })
        .collect();
    assert_eq!(frames.len(), 1, "one committed huddle callback");
    assert_eq!(frames[0].0, format!("user_{}_activity", saved.user_id));
    assert_eq!(frames[0].1["activityItemId"], saved.id);
    assert_eq!(frames[0].1["huddleInvitation"]["state"], "read");
    let rings: Vec<_> = events
        .iter()
        .filter_map(|event| event.as_job::<crate::models::huddle_invitations::RingRequest>())
        .collect();
    assert_eq!(rings.len(), 1);
    assert_eq!(rings[0].invitation["state"], "read");
}

#[test]
fn ws12_activity_stale_huddle_read_broadcast_uses_its_updated_snapshot() {
    let t = frozen_db();
    let a = huddle_item(&t, false);
    let b = t.read(|conn| ActivityItem::find(conn, a.id));
    let other = t.another_process();
    t.travel(1);
    t.write(move |tx| a.mark_handled(tx));
    t.sink.take();
    t.travel(1);
    let read = other.write_blocking(move |tx| b.mark_read(tx)).unwrap();
    assert_huddle_callback(&t, &read, "handled");
}

#[test]
fn ws12_activity_stale_huddle_unhandle_broadcast_uses_its_updated_snapshot() {
    let t = frozen_db();
    let a = huddle_item(&t, true);
    let b = t.read(|conn| ActivityItem::find(conn, a.id));
    let other = t.another_process();
    t.travel(1);
    t.write(move |tx| a.mark_unread(tx));
    t.sink.take();
    t.travel(1);
    let unhandled = other
        .write_blocking(move |tx| b.mark_unhandled(tx))
        .unwrap();
    assert_huddle_callback(&t, &unhandled, "unread");
}

fn unread_snapshot(t: &TestDb, user: i64) -> crate::models::activity_item::ActivityUnread {
    let now = t.now();
    t.read(move |conn| ActivityItem::unread_snapshot(conn, user, now))
}

#[test]
fn activity_unread_revision_orders_equal_time_state_writes_and_raw_deletion() {
    let t = frozen_db();
    let user = id("david");
    let other = id("jason");
    t.write(move |tx| {
        tx.conn()
            .execute("DELETE FROM activity_items WHERE user_id=?", [user])?;
        Ok(())
    });
    let start = unread_snapshot(&t, user);
    let isolated = unread_snapshot(&t, other);
    assert_eq!(start.count, 0);
    let created = t
        .write(move |tx| ActivityItem::refresh_unread(tx, user, "Message", id("first"), "mention"));
    let snapshot = unread_snapshot(&t, user);
    assert_eq!((snapshot.count, snapshot.revision), (1, start.revision + 1));
    let read = t.write(move |tx| created.mark_read(tx));
    assert_eq!(read.updated_at, t.now());
    let snapshot = unread_snapshot(&t, user);
    assert_eq!((snapshot.count, snapshot.revision), (0, start.revision + 2));
    let read_again = t.write(move |tx| read.mark_read(tx));
    assert_eq!(unread_snapshot(&t, user), snapshot);
    let handled = t.write(move |tx| read_again.mark_handled(tx));
    let snapshot = unread_snapshot(&t, user);
    assert_eq!((snapshot.count, snapshot.revision), (0, start.revision + 3));
    let unread = t.write(move |tx| handled.mark_unread(tx));
    assert_eq!(unread.updated_at, t.now());
    let snapshot = unread_snapshot(&t, user);
    assert_eq!((snapshot.count, snapshot.revision), (1, start.revision + 4));
    let handled = t.write(move |tx| unread.mark_handled(tx));
    let snapshot = unread_snapshot(&t, user);
    assert_eq!((snapshot.count, snapshot.revision), (0, start.revision + 5));
    let unread = t.write(move |tx| handled.mark_unread(tx));
    let snapshot = unread_snapshot(&t, user);
    assert_eq!((snapshot.count, snapshot.revision), (1, start.revision + 6));
    t.write(move |tx| {
        tx.conn()
            .execute("DELETE FROM activity_items WHERE id=?", [unread.id])?;
        Ok(())
    });
    let snapshot = unread_snapshot(&t, user);
    assert_eq!((snapshot.count, snapshot.revision), (0, start.revision + 7));
    assert_eq!(unread_snapshot(&t, other), isolated);
}

#[test]
fn activity_unread_revision_tracks_live_membership_source_and_user_access() {
    let t = frozen_db();
    let user = id("david");
    t.write(move |tx| {
        tx.conn()
            .execute("DELETE FROM activity_items WHERE user_id=?", [user])?;
        ActivityItem::refresh_unread(tx, user, "Message", id("first"), "mention")?;
        Ok(())
    });
    let start = unread_snapshot(&t, user);
    assert_eq!(start.count, 1);
    let membership = t.read(move |conn| {
        Ok(conn.query_row(
            "SELECT memberships.id FROM memberships JOIN messages ON messages.room_id=memberships.room_id WHERE messages.id=? AND memberships.user_id=?",
            rusqlite::params![id("first"), user], |row| row.get::<_, i64>(0),
        )?)
    });
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE memberships SET unread_at=? WHERE id=?",
            rusqlite::params![tx.now(), membership],
        )?;
        Ok(())
    });
    assert_eq!(unread_snapshot(&t, user), start);
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE memberships SET room_id=? WHERE id=?",
            rusqlite::params![id("bender_and_kevin"), membership],
        )?;
        Ok(())
    });
    let lost = unread_snapshot(&t, user);
    assert_eq!((lost.count, lost.revision), (0, start.revision + 1));
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE memberships SET room_id=(SELECT room_id FROM messages WHERE id=?) WHERE id=?",
            rusqlite::params![id("first"), membership],
        )?;
        Ok(())
    });
    let restored = unread_snapshot(&t, user);
    assert_eq!((restored.count, restored.revision), (1, start.revision + 2));
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE activity_items SET source_id=-1 WHERE user_id=?",
            [user],
        )?;
        Ok(())
    });
    let missing = unread_snapshot(&t, user);
    assert_eq!((missing.count, missing.revision), (0, start.revision + 3));
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE activity_items SET source_id=? WHERE user_id=?",
            rusqlite::params![id("first"), user],
        )?;
        tx.conn()
            .execute("UPDATE users SET status=1 WHERE id=?", [user])?;
        Ok(())
    });
    let inactive = unread_snapshot(&t, user);
    assert_eq!((inactive.count, inactive.revision), (0, start.revision + 5));
    t.write(move |tx| {
        tx.conn()
            .execute("UPDATE users SET status=0 WHERE id=?", [user])?;
        tx.conn().execute(
            "UPDATE messages SET room_id=? WHERE id=?",
            rusqlite::params![id("bender_and_kevin"), id("first")],
        )?;
        Ok(())
    });
    let moved = unread_snapshot(&t, user);
    assert_eq!((moved.count, moved.revision), (0, start.revision + 7));
}

#[test]
fn activity_unread_snapshot_keeps_count_and_revision_in_the_same_sqlite_snapshot() {
    let t = frozen_db();
    let user = id("david");
    let item = t.write(move |tx| {
        tx.conn()
            .execute("DELETE FROM activity_items WHERE user_id=?", [user])?;
        ActivityItem::refresh_unread(tx, user, "Message", id("first"), "mention")
    });
    let queries = t.db.capture_read_queries();
    let before = unread_snapshot(&t, user);
    t.db.stop_capturing_read_queries();
    assert_eq!(before.count, 1);
    let queries = queries.lock().unwrap();
    assert_eq!(
        queries.len(),
        1,
        "count and revision must be one SQLite statement"
    );
    assert!(queries[0].starts_with("SELECT "));
    drop(queries);
    let db = t.db.clone();
    let now = t.now();
    let during = t.read(move |conn| {
        let snapshot = conn.unchecked_transaction()?;
        let first = ActivityItem::unread_snapshot(&snapshot, user, now)?;
        assert_eq!(first, before);
        db.write_blocking(move |tx| {
            tx.conn()
                .execute("DELETE FROM activity_items WHERE id=?", [item.id])?;
            Ok(())
        })?;
        ActivityItem::unread_snapshot(&snapshot, user, now)
    });
    assert_eq!(during, before);
    let after = unread_snapshot(&t, user);
    assert_eq!((after.count, after.revision), (0, before.revision + 1));
}

#[test]
fn activity_unread_revision_tracks_source_insertion_deletion_and_agent_access() {
    let t = frozen_db();
    let user = id("david");
    let session = 9_800_000_001_i64;
    t.write(move |tx| {
        tx.conn().execute("DELETE FROM activity_items WHERE user_id=?", [user])?;
        tx.conn().execute(
            "INSERT INTO activity_items(user_id,source_type,source_id,event_type,created_at,updated_at) VALUES (?,'Session',?,'new_sign_in',?,?)",
            rusqlite::params![user, session, tx.now(), tx.now()],
        )?;
        Ok(())
    });
    let missing = unread_snapshot(&t, user);
    assert_eq!(missing.count, 0);
    t.write(move |tx| {
        tx.conn().execute(
            "INSERT INTO sessions(id,user_id,token,created_at,updated_at,last_active_at) VALUES (?,?, 'revision-test',?,?,?)",
            rusqlite::params![session, user, tx.now(), tx.now(), tx.now()],
        )?;
        Ok(())
    });
    let inserted = unread_snapshot(&t, user);
    assert_eq!(
        (inserted.count, inserted.revision),
        (1, missing.revision + 1)
    );
    t.write(move |tx| {
        tx.conn()
            .execute("DELETE FROM sessions WHERE id=?", [session])?;
        tx.conn()
            .execute("UPDATE users SET role=0 WHERE id=?", [user])?;
        Ok(())
    });
    let deleted = unread_snapshot(&t, user);
    assert_eq!(
        (deleted.count, deleted.revision),
        (0, inserted.revision + 2)
    );
    let approval = t.write(move |tx| {
        let approval = tx.conn().query_row(
            "INSERT INTO agent_approvals(agent_id,action,summary,created_at,updated_at,expires_at) VALUES (?,'test','Revision test',?,?,?) RETURNING id",
            rusqlite::params![id("bender_agent"), tx.now(), tx.now(), tx.now()],
            |row| row.get::<_, i64>(0),
        )?;
        ActivityItem::refresh_unread(tx, user, "AgentApproval", approval, "agent_approval_request")
    });
    let owned = unread_snapshot(&t, user);
    assert_eq!((owned.count, owned.revision), (1, deleted.revision + 1));
    let isolated = unread_snapshot(&t, id("kevin"));
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE agents SET owner_id=? WHERE id=?",
            rusqlite::params![id("jason"), id("bender_agent")],
        )?;
        Ok(())
    });
    let transferred = unread_snapshot(&t, user);
    assert_eq!(
        (transferred.count, transferred.revision),
        (0, owned.revision + 1)
    );
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE agents SET owner_id=owner_id,user_id=user_id WHERE id=?",
            [id("bender_agent")],
        )?;
        tx.conn().execute(
            "UPDATE users SET status=status,role=role WHERE id=?",
            [user],
        )?;
        Ok(())
    });
    assert_eq!(unread_snapshot(&t, user), transferred);
    t.write(move |tx| {
        tx.conn()
            .execute("UPDATE users SET role=1 WHERE id=?", [user])?;
        Ok(())
    });
    let administrator = unread_snapshot(&t, user);
    assert_eq!(
        (administrator.count, administrator.revision),
        (1, transferred.revision + 1)
    );
    t.write(move |tx| {
        tx.conn()
            .execute("UPDATE users SET status=1 WHERE id=?", [id("bender")])?;
        Ok(())
    });
    let bot_inactive = unread_snapshot(&t, user);
    assert_eq!(
        (bot_inactive.count, bot_inactive.revision),
        (0, administrator.revision + 1)
    );
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE agents SET user_id=? WHERE id=?",
            rusqlite::params![id("kevin"), id("bender_agent")],
        )?;
        Ok(())
    });
    let reassigned = unread_snapshot(&t, user);
    assert_eq!(
        (reassigned.count, reassigned.revision),
        (1, bot_inactive.revision + 1)
    );
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE agents SET user_id=? WHERE id=?",
            rusqlite::params![id("bender"), id("bender_agent")],
        )?;
        Ok(())
    });
    let returned = unread_snapshot(&t, user);
    assert_eq!(
        (returned.count, returned.revision),
        (0, reassigned.revision + 1)
    );
    t.write(move |tx| {
        tx.conn()
            .execute("UPDATE users SET status=0 WHERE id=?", [id("bender")])?;
        tx.conn().execute(
            "DELETE FROM agent_approvals WHERE id=?",
            [approval.source_id],
        )?;
        Ok(())
    });
    let removed = unread_snapshot(&t, user);
    assert_eq!(
        (removed.count, removed.revision),
        (0, returned.revision + 2)
    );
    assert_eq!(unread_snapshot(&t, id("kevin")), isolated);
}

#[test]
fn activity_unread_revision_tracks_saved_message_links_and_source_deletion() {
    let t = frozen_db();
    let user = id("david");
    let saved = t.write(move |tx| {
        tx.conn().execute("DELETE FROM activity_items WHERE user_id=?", [user])?;
        let saved = tx.conn().query_row(
            "INSERT INTO saved_items(user_id,message_id,created_at,updated_at) VALUES (?,?,?,?) RETURNING id",
            rusqlite::params![user, id("first"), tx.now(), tx.now()],
            |row| row.get::<_, i64>(0),
        )?;
        ActivityItem::refresh_unread(tx, user, "SavedItem", saved, "message_reminder")?;
        Ok(saved)
    });
    let before = unread_snapshot(&t, user);
    let isolated = unread_snapshot(&t, id("jason"));
    assert_eq!(before.count, 1);
    let inaccessible_message = t.write(move |tx| {
        Ok(tx.conn().query_row(
            "INSERT INTO messages(room_id,creator_id,client_message_id,created_at,updated_at) VALUES (?,?,'revision-message',?,?) RETURNING id",
            rusqlite::params![id("bender_and_kevin"), user, tx.now(), tx.now()],
            |row| row.get::<_, i64>(0),
        )?)
    });
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE saved_items SET message_id=? WHERE id=?",
            rusqlite::params![inaccessible_message, saved],
        )?;
        Ok(())
    });
    let lost = unread_snapshot(&t, user);
    assert_eq!((lost.count, lost.revision), (0, before.revision + 1));
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE saved_items SET message_id=? WHERE id=?",
            rusqlite::params![id("first"), saved],
        )?;
        Ok(())
    });
    let restored = unread_snapshot(&t, user);
    assert_eq!((restored.count, restored.revision), (1, lost.revision + 1));
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE messages SET room_id=? WHERE id=?",
            rusqlite::params![id("bender_and_kevin"), id("first")],
        )?;
        Ok(())
    });
    let moved = unread_snapshot(&t, user);
    assert_eq!((moved.count, moved.revision), (0, restored.revision + 1));
    t.write(move |tx| {
        tx.conn()
            .execute("DELETE FROM saved_items WHERE id=?", [saved])?;
        Ok(())
    });
    let removed = unread_snapshot(&t, user);
    assert_eq!((removed.count, removed.revision), (0, moved.revision + 1));
    assert_eq!(unread_snapshot(&t, id("jason")), isolated);
}

#[test]
fn activity_unread_revision_tracks_work_event_and_thread_access_links() {
    let t = frozen_db();
    let user = id("david");
    let (accessible, inaccessible, event) = t.write(move |tx| {
        tx.conn().execute("DELETE FROM activity_items WHERE user_id=?", [user])?;
        let mut threads = Vec::new();
        for room in [id("designers"), id("bender_and_kevin")] {
            threads.push(tx.conn().query_row(
                "INSERT INTO channel_threads(room_id,creator_id,name,created_at,updated_at,last_activity_at) VALUES (?,?,'Revision test',?,?,?) RETURNING id",
                rusqlite::params![room, user, tx.now(), tx.now(), tx.now()],
                |row| row.get::<_, i64>(0),
            )?);
        }
        let event = tx.conn().query_row(
            "INSERT INTO work_thread_events(channel_thread_id,event_type,created_at,updated_at) VALUES (?,'status_changed',?,?) RETURNING id",
            rusqlite::params![threads[0], tx.now(), tx.now()],
            |row| row.get::<_, i64>(0),
        )?;
        ActivityItem::refresh_unread(tx, user, "WorkThreadEvent", event, "work_update")?;
        Ok((threads[0], threads[1], event))
    });
    let before = unread_snapshot(&t, user);
    let isolated = unread_snapshot(&t, id("jason"));
    assert_eq!(before.count, 1);
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE work_thread_events SET channel_thread_id=? WHERE id=?",
            rusqlite::params![inaccessible, event],
        )?;
        Ok(())
    });
    let lost = unread_snapshot(&t, user);
    assert_eq!((lost.count, lost.revision), (0, before.revision + 1));
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE work_thread_events SET channel_thread_id=? WHERE id=?",
            rusqlite::params![accessible, event],
        )?;
        tx.conn().execute(
            "UPDATE channel_threads SET room_id=? WHERE id=?",
            rusqlite::params![id("bender_and_kevin"), accessible],
        )?;
        Ok(())
    });
    let moved = unread_snapshot(&t, user);
    assert_eq!((moved.count, moved.revision), (0, lost.revision + 2));
    t.write(move |tx| {
        tx.conn()
            .execute("DELETE FROM channel_threads WHERE id=?", [accessible])?;
        Ok(())
    });
    let removed = unread_snapshot(&t, user);
    assert_eq!((removed.count, removed.revision), (0, moved.revision + 2));
    assert_eq!(unread_snapshot(&t, id("jason")), isolated);
}
