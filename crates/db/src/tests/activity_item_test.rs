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
