//! Remaining test/models/event/recurrence_test.rb declarations.
use super::*;
use crate::models::calendar_event::changes::EventChanges;
fn stamp(s: &str) -> Timestamp {
    Timestamp::parse_db(s).unwrap()
}
fn head(t: &TestDb, start: &str, until: &str) -> CalendarEvent {
    let mut a = attrs(t);
    a.starts_at = Some(stamp(start));
    a.ends_at = Some(a.starts_at.unwrap().since(SignedDuration::from_hours(1)));
    a.recurrence_rule = Some("weekly".into());
    a.recurrence_until = Some(until.parse().unwrap());
    t.write(move |tx| CalendarEvent::create(tx, a))
}
fn rows(t: &TestDb, e: &CalendarEvent) -> Vec<CalendarEvent> {
    t.read(|c| e.series_events(c))
}
fn update(t: &TestDb, eid: i64, c: EventChanges, scope: &str) {
    let scope = scope.to_owned();
    t.write(move |tx| CalendarEvent::update_with_scope(tx, eid, c, &scope, Some(id("david"))));
}
#[test]
fn cutover_recurrence_singleton_has_no_series_or_neighbors() {
    let t = frozen();
    let e = super::cutover_event_test::scheduled(&t);
    assert!(!e.series());
    assert!(!e.series_head());
    assert!(t.read(|c| e.next_occurrence(c)).is_none());
    assert!(t.read(|c| e.previous_occurrence(c)).is_none());
    assert!(t.read(|c| e.future_occurrences(c)).is_empty());
}
#[test]
fn cutover_recurrence_head_plain_update_checks_one_year_cap() {
    let t = frozen();
    let h = head(&t, "2026-10-05 09:00:00", "2026-10-19");
    let eid = h.id;
    let result = t.try_write(move |tx| {
        CalendarEvent::update(
            tx,
            eid,
            EventChanges {
                recurrence_until: Some(Some("2029-09-22".parse().unwrap())),
                ..Default::default()
            },
        )
    });
    assert!(matches!(&result, Err(Error::RecordInvalid(_))));
    let err = result.unwrap_err();
    let Error::RecordInvalid(errors) = err else {
        panic!("{err:?}")
    };
    assert!(
        errors
            .on("recurrence_until")
            .iter()
            .any(|e| e.contains("at most one year"))
    );
    assert_eq!(
        t.read(|c| CalendarEvent::find(c, h.id)).recurrence_until,
        h.recurrence_until
    );
}
#[test]
fn cutover_recurrence_head_local_edit_accepts_unchanged_rule_values() {
    let t = frozen();
    let h = head(&t, "2026-09-23 09:00:00", "2026-10-07");
    let before = rows(&t, &h);
    assert_eq!(before.len(), 3);
    update(
        &t,
        h.id,
        EventChanges {
            title: Some("Renamed".into()),
            recurrence_rule: Some(Some("weekly".into())),
            recurrence_until: Some(h.recurrence_until),
            ..Default::default()
        },
        "this_event",
    );
    let after = rows(&t, &h);
    assert_eq!(after[0].title, "Renamed");
    for e in &after[1..] {
        assert_eq!(e.title, "Planning session");
    }
    assert_eq!(
        after.iter().map(|e| e.id).collect::<Vec<_>>(),
        before.iter().map(|e| e.id).collect::<Vec<_>>()
    );
}
#[test]
fn cutover_recurrence_active_series_slots_have_unique_database_constraint() {
    let t = frozen();
    let h = head(&t, "2026-10-05 09:00:00", "2026-10-19");
    let taken = rows(&t, &h)[1].clone();
    use rusqlite::OptionalExtension;
    let index = t.read(|c| Ok(c.query_row("SELECT \"unique\" FROM pragma_index_list('events') WHERE name='index_events_on_series_slot'", [], |r| r.get::<_, bool>(0)).optional()?));
    assert!(index.is_some());
    assert_eq!(index, Some(true));
    let mut a = attrs(&t);
    a.title = "Duplicate slot".into();
    a.starts_at = Some(taken.starts_at);
    a.ends_at = taken.ends_at;
    let result = t.try_write(move |tx| {
        let duplicate = CalendarEvent::create(tx, a)?;
        tx.conn().execute(
            "UPDATE events SET series_id=?,recurrence_rule='weekly',recurrence_until=? WHERE id=?",
            params![
                h.id,
                h.recurrence_until.map(|d| d.to_string()),
                duplicate.id
            ],
        )?;
        Ok(())
    });
    assert!(
        matches!(result,Err(Error::Sqlite(rusqlite::Error::SqliteFailure(e,_))) if e.extended_code==rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE)
    );
}
#[test]
fn cutover_recurrence_equal_time_orders_active_before_cancelled() {
    let t = frozen();
    let h = head(&t, "2027-01-01 09:00:00", "2027-01-15");
    let before = rows(&t, &h);
    super::cutover_event_test::cancel(&t, before[1].id);
    update(
        &t,
        before[2].id,
        EventChanges {
            starts_at: Some(Some(stamp("2027-01-08 09:00:00"))),
            ends_at: Some(Some(stamp("2027-01-08 10:00:00"))),
            ..Default::default()
        },
        "this_event",
    );
    assert_eq!(
        rows(&t, &h).iter().map(|e| e.id).collect::<Vec<_>>(),
        vec![h.id, before[2].id, before[1].id]
    );
    assert_eq!(t.read(|c| h.next_occurrence(c)).unwrap().id, before[2].id);
}
#[test]
fn cutover_recurrence_rule_over_cap_rejects_and_preserves_original_series() {
    let t = frozen();
    let h = head(&t, "2026-10-05 09:00:00", "2026-10-19");
    let before = rows(&t, &h);
    let eid = h.id;
    let result = t.try_write(move |tx| {
        CalendarEvent::update_with_scope(
            tx,
            eid,
            EventChanges {
                recurrence_rule: Some(Some("daily".into())),
                recurrence_until: Some(Some("2026-12-15".parse().unwrap())),
                ..Default::default()
            },
            "this_and_following",
            Some(id("david")),
        )
    });
    assert!(matches!(&result, Err(Error::RecordInvalid(_))));
    let err = result.unwrap_err();
    let Error::RecordInvalid(errors) = err else {
        panic!("{err:?}")
    };
    assert!(
        errors
            .on("recurrence_until")
            .iter()
            .any(|e| e.contains("pick an earlier end date"))
    );
    assert_eq!(
        rows(&t, &h).iter().map(|e| e.id).collect::<Vec<_>>(),
        before.iter().map(|e| e.id).collect::<Vec<_>>()
    );
    assert_eq!(
        t.read(|c| CalendarEvent::find(c, h.id)).recurrence_rule,
        Some("weekly".into())
    );
}
#[test]
fn cutover_recurrence_head_only_series_retimes_through_following() {
    let t = frozen();
    let h = head(&t, "2027-01-01 09:00:00", "2027-01-05");
    assert_eq!(
        rows(&t, &h).iter().map(|e| e.id).collect::<Vec<_>>(),
        vec![h.id]
    );
    update(
        &t,
        h.id,
        EventChanges {
            starts_at: Some(Some(stamp("2027-01-01 10:00:00"))),
            ends_at: Some(Some(stamp("2027-01-01 11:00:00"))),
            ..Default::default()
        },
        "this_and_following",
    );
    let fresh = t.read(|c| CalendarEvent::find(c, h.id));
    assert_eq!(fresh.starts_at, stamp("2027-01-01 10:00:00"));
    assert_eq!(fresh.ends_at, Some(stamp("2027-01-01 11:00:00")));
    assert_eq!(fresh.series_id, Some(h.id));
    assert_eq!(
        rows(&t, &h).iter().map(|e| e.id).collect::<Vec<_>>(),
        vec![h.id]
    );
    // Rails checks @following_reorder is cleared. Exercise the validation it
    // guards, immediately after the scoped operation (event.rb:315-319).
    let eid = h.id;
    let result = t.try_write(move |tx| {
        CalendarEvent::update(
            tx,
            eid,
            EventChanges {
                starts_at: Some(Some(stamp("2027-01-01 12:00:00"))),
                ends_at: Some(Some(stamp("2027-01-01 13:00:00"))),
                ..Default::default()
            },
        )
    });
    assert!(
        matches!(result, Err(Error::RecordInvalid(ref errors)) if errors.on("starts_at") == vec!["moves the whole series: choose This and following or the entire series"])
    );
    assert_eq!(
        t.read(|c| CalendarEvent::find(c, eid)).starts_at,
        fresh.starts_at
    );
}
#[test]
fn cutover_recurrence_head_description_edit_stays_local() {
    let t = frozen();
    let h = head(&t, "2026-09-23 09:00:00", "2026-10-07");
    update(
        &t,
        h.id,
        EventChanges {
            description: Some(Some("Head note".into())),
            ..Default::default()
        },
        "this_event",
    );
    let after = rows(&t, &h);
    assert_eq!(after[0].description, Some("Head note".into()));
    assert!(after[1].description.is_none());
}
