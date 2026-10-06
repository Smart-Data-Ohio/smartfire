//! Remaining test/models/event/channel_timeline_test.rb declarations.
use super::cutover_event_test::cancel;
use super::*;
use crate::{Message, NewMessage};
fn timeline_db() -> TestDb {
    TestDb::with_clock_and_origin(
        TestClock::frozen_at(Timestamp::parse_db("2026-09-22 12:00:00").unwrap()),
        4,
        "",
    )
}
fn scheduled(t: &TestDb) -> CalendarEvent {
    let mut a = attrs(t);
    a.starts_at = Some(t.now().since(SignedDuration::from_hours(48)));
    t.write(move |tx| CalendarEvent::create(tx, a))
}
fn roots(t: &TestDb) -> i64 {
    t.read(|c| {
        Ok(c.query_row(
            "SELECT COUNT(*) FROM messages WHERE room_id=? AND thread_id IS NULL",
            [id("designers")],
            |r| r.get(0),
        )?)
    })
}
fn latest(t: &TestDb) -> Message {
    t.read(|c|{let mid=c.query_row("SELECT id FROM messages WHERE room_id=? AND thread_id IS NULL ORDER BY created_at DESC,id DESC LIMIT 1",[id("designers")],|r|r.get(0))?;Message::find(c,mid)})
}
#[test]
fn cutover_timeline_singleton_has_one_organizer_announcement_with_title_url_and_reference() {
    let t = timeline_db();
    let before = roots(&t);
    let e = scheduled(&t);
    assert_eq!(roots(&t), before + 1);
    let m = latest(&t);
    assert_eq!(m.creator_id, id("david"));
    assert_eq!(
        m.markdown_source,
        Some(format!(
            "Scheduled an event: Planning session\n/rooms/{}/events/{}",
            e.room_id, e.id
        ))
    );
    let refs: Vec<i64> = t.read(|c| {
        Ok(
            c.prepare("SELECT event_id FROM event_references WHERE message_id=?")?
                .query_map([m.id], |r| r.get(0))?
                .collect::<rusqlite::Result<_>>()?,
        )
    });
    assert_eq!(refs, vec![e.id]);
    assert_eq!(
        t.read(|c| Ok(CalendarEvent::for_message_ids(c, &[m.id])?
            .remove(&m.id)
            .unwrap_or_default()
            .into_iter()
            .map(|e| e.id)
            .collect::<Vec<_>>())),
        vec![e.id]
    );
}
#[test]
fn cutover_timeline_edits_and_cancels_post_no_messages() {
    let t = timeline_db();
    let e = scheduled(&t);
    let before = count(&t, "messages");
    let eid = e.id;
    t.write(move |tx| {
        CalendarEvent::update_with_scope(
            tx,
            eid,
            crate::models::calendar_event::changes::EventChanges {
                title: Some("Renamed".into()),
                starts_at: Some(Some(tx.now().since(SignedDuration::from_hours(72)))),
                ..Default::default()
            },
            "this_event",
            Some(id("david")),
        )
    });
    cancel(&t, e.id);
    assert_eq!(count(&t, "messages"), before);
}
#[test]
fn cutover_timeline_announcement_has_no_inbox_items() {
    let t = timeline_db();
    scheduled(&t);
    let m = latest(&t);
    assert_eq!(
        t.read(|c| Ok(c.query_row(
            "SELECT COUNT(*) FROM activity_items WHERE source_type='Message' AND source_id=?",
            [m.id],
            |r| r.get::<_, i64>(0)
        )?)),
        0
    );
}
#[test]
fn cutover_timeline_event_destroy_removes_references_but_preserves_message() {
    let t = timeline_db();
    let e = scheduled(&t);
    let m = t.write(move |tx| {
        Message::create(
            tx,
            NewMessage {
                room_id: e.room_id,
                creator_id: id("jason"),
                markdown_source: Some(format!("see /rooms/{}/events/{}", e.room_id, e.id)),
                client_message_id: Some("evt-ref-delete".into()),
                ..Default::default()
            },
        )
    });
    let refs = || {
        t.read(|c| {
            Ok(c.query_row(
                "SELECT COUNT(*) FROM event_references WHERE message_id=? AND event_id=?",
                params![m.id, e.id],
                |r| r.get::<_, i64>(0),
            )?)
        })
    };
    assert_eq!(refs(), 1);
    let eid = e.id;
    t.write(move |tx| CalendarEvent::find(tx.conn(), eid)?.destroy(tx));
    assert_eq!(refs(), 0);
    assert!(t.read(|c| Message::find(c, m.id)).id == m.id);
}
