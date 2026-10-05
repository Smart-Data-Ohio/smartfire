//! test/models/event/reminder_dispatcher_test.rb: due scan and per-occurrence claims.
use super::*;
use crate::models::calendar_event::changes::EventChanges;
fn due(t: &TestDb) {
    let now = t.now();
    let ids = t.read(|c| CalendarEvent::due_reminder_ids(c, now));
    for eid in ids {
        t.write(move |tx| CalendarEvent::dispatch_reminder(tx, eid, now));
    }
}
#[test]
fn cutover_reminder_consecutive_occurrences_are_claimed_once_at_own_times() {
    let t = frozen();
    let mut a = attrs(&t);
    a.title = "Daily sync".into();
    a.recurrence_rule = Some("daily".into());
    a.recurrence_until = Some("2026-09-23".parse().unwrap());
    let head = t.write(move |tx| CalendarEvent::create(tx, a));
    let rows = t.read(|c| head.series_events(c));
    assert_eq!(rows.len(), 2);
    let first = rows[0].id;
    let second = rows[1].id;
    let starts = t.now().since(SignedDuration::from_mins(70));
    t.write(move |tx| {
        CalendarEvent::update(
            tx,
            second,
            EventChanges {
                starts_at: Some(Some(starts)),
                ..Default::default()
            },
        )
    });
    for e in &rows {
        respond(&t, e.id, "jason", "going", false);
    }
    due(&t);
    assert_eq!(
        t.read(|c| CalendarEvent::find(c, first)).reminded_at,
        Some(t.now())
    );
    assert!(
        t.read(|c| CalendarEvent::find(c, second))
            .reminded_at
            .is_none()
    );
    assert_eq!(
        item(&t, first, "jason").unwrap().event_type,
        "event_reminder"
    );
    let first_stamp = t.now();
    t.travel(3600);
    due(&t);
    assert_eq!(
        t.read(|c| CalendarEvent::find(c, second)).reminded_at,
        Some(t.now())
    );
    assert_eq!(
        item(&t, second, "jason").unwrap().event_type,
        "event_reminder"
    );
    let before = push_count(&t);
    due(&t);
    assert_eq!(push_count(&t), before);
    assert_eq!(
        t.read(|c| CalendarEvent::find(c, first)).reminded_at,
        Some(first_stamp)
    );
}
#[test]
fn cutover_reminder_start_crossing_midnight_builds_two_occurrences() {
    let t = TestDb::with_clock(
        TestClock::frozen_at(Timestamp::parse_db("2026-09-22 23:50:00").unwrap()),
        4,
    );
    let mut a = attrs(&t);
    a.title = "Midnight sync".into();
    a.recurrence_rule = Some("daily".into());
    a.recurrence_until = Some("2026-09-24".parse().unwrap());
    let head = t.write(move |tx| CalendarEvent::create(tx, a));
    let rows = t.read(|c| head.series_events(c));
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[0].starts_at,
        Timestamp::parse_db("2026-09-23 00:00:00").unwrap()
    );
    assert_eq!(
        rows[1].starts_at,
        Timestamp::parse_db("2026-09-24 00:00:00").unwrap()
    );
}
