//! Ports of the event models at d7c7de92. Scope reconstruction and HTML are separate slices.
use super::*;
use crate::models::calendar_event::{SyncEntryJob, reminders::ReminderPushJob};
use crate::{
    ActivityItem, CalendarEvent, Error, EventAttendance, NewCalendarEvent, Timestamp, User,
};
use jiff::SignedDuration;
use rusqlite::params;

fn frozen() -> TestDb {
    TestDb::with_clock(
        TestClock::frozen_at(Timestamp::parse_db("2026-09-22 12:00:00").unwrap()),
        4,
    )
}
fn attrs(t: &TestDb) -> NewCalendarEvent {
    NewCalendarEvent {
        room_id: id("designers"),
        organizer_id: id("david"),
        title: "Planning session".into(),
        starts_at: Some(t.now().since(SignedDuration::from_mins(10))),
        time_zone: "UTC".into(),
        ..Default::default()
    }
}
fn create(t: &TestDb) -> CalendarEvent {
    let a = attrs(t);
    t.write(move |tx| CalendarEvent::create(tx, a))
}
fn series(t: &TestDb) -> CalendarEvent {
    let mut a = attrs(t);
    a.recurrence_rule = Some("weekly".into());
    a.recurrence_until = Some("2026-10-06".parse().unwrap());
    t.write(move |tx| CalendarEvent::create(tx, a))
}
fn respond(t: &TestDb, event: i64, user: &str, response: &str, future: bool) {
    let user = id(user);
    let response = response.to_string();
    t.write(move |tx| CalendarEvent::respond(tx, event, user, &response, future));
}
fn item(t: &TestDb, event: i64, user: &str) -> Option<ActivityItem> {
    t.read(|c| ActivityItem::find_by_user_and_source(c, id(user), "Event", event))
}
fn dispatch(t: &TestDb, event: i64) -> bool {
    t.write(move |tx| CalendarEvent::dispatch_reminder(tx, event, tx.now()))
}
fn push_count(t: &TestDb) -> usize {
    t.events()
        .iter()
        .filter(|e| matches!(e,Event::Job(j) if j.class=="Event::ReminderPushJob"))
        .count()
}
fn count(t: &TestDb, table: &str) -> i64 {
    t.read(|c| Ok(c.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))?))
}

#[test]
fn event_validations_match_rails_messages() {
    let t = frozen();
    type ValidationCase = (fn(&mut NewCalendarEvent), &'static str, &'static str);
    let cases: Vec<ValidationCase> = vec![
        (|a| a.title = "\u{a0}".into(), "title", "can't be blank"),
        (|a| a.starts_at = None, "starts_at", "can't be blank"),
        (|a| a.time_zone = "".into(), "time_zone", "can't be blank"),
        (
            |a| a.time_zone = "Mars/Olympus".into(),
            "time_zone",
            "is invalid",
        ),
        (
            |a| a.ends_at = a.starts_at,
            "ends_at",
            "must be after the start time",
        ),
        (
            |a| a.recurrence_rule = Some("bogus".into()),
            "recurrence_rule",
            "is not included in the list",
        ),
        (
            |a| a.organizer_id = id("bender"),
            "organizer",
            "must be an active human member of the room",
        ),
        (
            |a| a.venue_room_id = Some(id("designers")),
            "venue",
            "must be a voice or Stage channel you belong to",
        ),
    ];
    for (change, key, message) in cases {
        let mut a = attrs(&t);
        change(&mut a);
        assert!(
            t.read(|c| CalendarEvent::validate(c, &a))
                .on(key)
                .contains(&message)
        );
        assert!(matches!(
            t.try_write(move |tx| CalendarEvent::create(tx, a)),
            Err(Error::RecordInvalid(_))
        ));
    }
}
#[test]
fn recurrence_range_cap_and_blank_rule_match_rails() {
    let t = frozen();
    for (until, message) in [
        (None, "can't be blank"),
        (Some("2026-09-22"), "must be after the start date"),
        (
            Some("2027-09-23"),
            "must be at most one year after the start date",
        ),
        (
            Some("2026-11-13"),
            "would create 53 occurrences (maximum 52); pick an earlier end date",
        ),
    ] {
        let mut a = attrs(&t);
        a.recurrence_rule = Some("daily".into());
        a.recurrence_until = until.map(|s| s.parse().unwrap());
        assert!(
            t.read(|c| CalendarEvent::validate(c, &a))
                .on("recurrence_until")
                .contains(&message)
        );
    }
    let mut a = attrs(&t);
    a.recurrence_rule = Some(" ".into());
    assert_eq!(
        t.write(move |tx| CalendarEvent::create(tx, a))
            .recurrence_rule,
        None
    );
    let mut a = attrs(&t);
    a.recurrence_rule = Some("daily".into());
    a.recurrence_until = Some("2026-11-12".parse().unwrap());
    let head = t.write(move |tx| CalendarEvent::create(tx, a));
    assert_eq!(t.read(|c| head.series_events(c)).len(), 52);
}
#[test]
fn series_materializes_organizer_attendance_one_invitation_and_one_announcement() {
    let t = frozen();
    let before = count(&t, "messages");
    let head = series(&t);
    let rows = t.read(|c| head.series_events(c));
    assert_eq!(rows.len(), 3);
    for e in &rows {
        assert_eq!(e.series_id, Some(head.id));
        assert_eq!(
            t.read(|c| EventAttendance::find_for(c, e.id, id("david")))
                .unwrap()
                .response,
            "going"
        );
    }
    assert_eq!(count(&t, "messages"), before + 1);
    for who in ["jason", "jz", "kevin"] {
        assert_eq!(
            item(&t, head.id, who).unwrap().event_type,
            "event_invitation"
        );
        assert!(item(&t, rows[1].id, who).is_none());
    }
    assert!(item(&t, head.id, "david").is_none());
    let message = t.read(|c| {
        Ok(c.query_row(
            "SELECT markdown_source FROM messages ORDER BY id DESC LIMIT 1",
            [],
            |r| r.get::<_, String>(0),
        )?)
    });
    assert_eq!(
        message,
        format!(
            "Scheduled an event: Planning session\n/rooms/{}/events/{}",
            head.room_id, head.id
        )
    );
    assert_eq!(
        t.read(|c| Ok(c.query_row(
            "SELECT COUNT(*) FROM event_references WHERE event_id=?",
            [head.id],
            |r| r.get::<_, i64>(0)
        )?)),
        1
    );
    let jobs: Vec<_> = t
        .events()
        .into_iter()
        .filter_map(|e| if let Event::Job(j) = e { Some(j) } else { None })
        .filter(|j| j.class == "Calendar::SyncEntryJob")
        .collect();
    assert_eq!(jobs.len(), 3);
    assert_eq!(
        jobs[0].decode::<SyncEntryJob>().unwrap().unwrap().user_id,
        id("david")
    );
}
#[test]
fn head_response_copies_and_follower_response_stays_local_until_requested() {
    let t = frozen();
    let head = series(&t);
    let rows = t.read(|c| head.series_events(c));
    respond(&t, head.id, "jason", "maybe", false);
    for e in &rows {
        assert_eq!(
            t.read(|c| EventAttendance::find_for(c, e.id, id("jason")))
                .unwrap()
                .response,
            "maybe"
        );
    }
    respond(&t, rows[1].id, "jason", "declined", false);
    assert_eq!(
        t.read(|c| EventAttendance::find_for(c, rows[2].id, id("jason")))
            .unwrap()
            .response,
        "maybe"
    );
    respond(&t, rows[1].id, "jason", "going", true);
    assert_eq!(
        t.read(|c| EventAttendance::find_for(c, rows[2].id, id("jason")))
            .unwrap()
            .response,
        "going"
    );
    assert_eq!(
        t.read(|c| EventAttendance::find_for(c, head.id, id("jason")))
            .unwrap()
            .response,
        "maybe"
    );
}
#[test]
fn unchanged_attendance_does_not_enqueue_sync_or_touch_timestamp() {
    let t = frozen();
    let e = create(&t);
    respond(&t, e.id, "jason", "going", false);
    let before = t
        .read(|c| EventAttendance::find_for(c, e.id, id("jason")))
        .unwrap();
    t.sink.take();
    t.travel(60);
    respond(&t, e.id, "jason", "going", false);
    assert_eq!(
        t.read(|c| EventAttendance::find_for(c, e.id, id("jason")))
            .unwrap(),
        before
    );
    assert!(t.events().is_empty());
}

#[test]
fn meet_requests_and_attendance_reads_preserve_persisted_state() {
    let t = frozen();
    let mut a = attrs(&t);
    a.meet_link_requested = true;
    let e = t.write(move |tx| CalendarEvent::create(tx, a));
    assert!(e.needs_meet_link());
    assert!(
        t.events()
            .iter()
            .any(|e| matches!(e, Event::Job(j) if j.class == "Calendar::MeetLinkJob"))
    );
    assert_eq!(
        t.read(|c| e.response_for(c, Some(id("david")))),
        Some("going".into())
    );
    assert_eq!(t.read(|c| e.response_for(c, None)), None);
    respond(&t, e.id, "jason", "maybe", false);
    assert_eq!(
        t.read(|c| e.attendance_counts(c)),
        std::collections::BTreeMap::from([("going".into(), 1), ("maybe".into(), 1)])
    );
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE events SET meet_link=? WHERE id=?",
            params!["https://meet.example.test/fixture", e.id],
        )?;
        Ok(())
    });
    assert!(!t.read(|c| CalendarEvent::find(c, e.id)).needs_meet_link());
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE events SET meet_link=NULL,cancelled_at=? WHERE id=?",
            params![tx.now(), e.id],
        )?;
        Ok(())
    });
    assert!(!t.read(|c| CalendarEvent::find(c, e.id)).needs_meet_link());
}

#[test]
fn series_create_calendar_callbacks_match_pinned_rails_order_and_arguments() {
    let t = frozen();
    let mut a = attrs(&t);
    a.meet_link_requested = true;
    a.recurrence_rule = Some("weekly".into());
    a.recurrence_until = Some("2026-10-06".parse().unwrap());
    let event = t.write(move |tx| CalendarEvent::create(tx, a));
    let ids: Vec<i64> = t
        .read(|c| event.series_events(c))
        .into_iter()
        .map(|e| e.id)
        .collect();
    let actual: Vec<_> = t
        .events()
        .into_iter()
        .filter_map(|e| if let Event::Job(j) = e { Some(j) } else { None })
        .filter(|j| j.class.starts_with("Calendar::"))
        .map(|j| {
            let index = ids
                .iter()
                .position(|id| Some(*id) == j.arguments["event_id"].as_i64())
                .unwrap();
            let mut args = vec![serde_json::json!(format!("event-{index}"))];
            if let Some(user_id) = j.arguments.get("user_id") {
                args.push(user_id.clone());
            }
            serde_json::json!([j.class, args])
        })
        .collect();
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "../models/calendar_event/create_callbacks.json"
    ))
    .unwrap();
    assert_eq!(serde_json::json!(actual), expected);
}
#[test]
fn ws14e_security_nonmember_cannot_view_an_event() {
    let t = frozen();
    let e = create(&t);
    t.write(|tx| {
        tx.conn().execute(
            "DELETE FROM memberships WHERE room_id=? AND user_id=?",
            params![id("designers"), id("jason")],
        )?;
        Ok(())
    });
    assert!(
        t.db.read_blocking(|c| CalendarEvent::find_visible(c, e.room_id, e.id, id("jason")))
            .is_err()
    );
    assert!(
        t.db.read_blocking(|c| CalendarEvent::find_visible(
            c,
            id("watercooler"),
            e.id,
            id("david")
        ))
        .is_err()
    );
}
#[test]
fn ws14e_security_only_organizer_or_active_human_admin_can_manage() {
    let t = frozen();
    let e = create(&t);
    assert!(!e.manageable_by(Some(&t.read(|c| User::find(c, id("jz"))))));
    assert!(!e.manageable_by(Some(&t.read(|c| User::find(c, id("bender"))))));
    assert!(e.manageable_by(Some(&t.read(|c| User::find(c, id("david"))))));
}
#[test]
fn ws14e_security_rsvp_rejects_nonmembers_bots_and_cancelled_events() {
    let t = frozen();
    let e = create(&t);
    t.write(|tx| {
        tx.conn().execute(
            "DELETE FROM memberships WHERE room_id=? AND user_id=?",
            params![id("designers"), id("jason")],
        )?;
        Ok(())
    });
    assert!(
        t.try_write(move |tx| CalendarEvent::respond(tx, e.id, id("jason"), "going", false))
            .is_err()
    );
    assert!(
        t.try_write(move |tx| CalendarEvent::respond(tx, e.id, id("bender"), "going", false))
            .is_err()
    );
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE events SET cancelled_at=? WHERE id=?",
            params![tx.now(), e.id],
        )?;
        Ok(())
    });
    assert!(
        t.try_write(move |tx| CalendarEvent::respond(tx, e.id, id("kevin"), "going", false))
            .is_err()
    );
}
#[test]
fn ws14e_security_reminder_excludes_a_recipient_who_left_the_room() {
    let t = frozen();
    let e = create(&t);
    respond(&t, e.id, "jason", "going", false);
    let before = item(&t, e.id, "jason");
    assert!(before.is_some());
    t.write(|tx| {
        tx.conn().execute(
            "DELETE FROM memberships WHERE room_id=? AND user_id=?",
            params![id("designers"), id("jason")],
        )?;
        Ok(())
    });
    assert!(dispatch(&t, e.id));
    assert_eq!(item(&t, e.id, "jason"), before);
}
#[test]
fn reminders_notify_going_and_maybe_once_refreshing_existing_items() {
    let t = frozen();
    let e = create(&t);
    respond(&t, e.id, "jason", "going", false);
    respond(&t, e.id, "jz", "maybe", false);
    respond(&t, e.id, "kevin", "declined", false);
    t.sink.take();
    assert!(dispatch(&t, e.id));
    assert_eq!(push_count(&t), 1);
    for who in ["david", "jason", "jz"] {
        let item = item(&t, e.id, who).unwrap();
        assert_eq!(item.event_type, "event_reminder");
        assert!(item.unread());
    }
    assert_eq!(
        item(&t, e.id, "kevin").unwrap().event_type,
        "event_invitation"
    );
    assert!(!dispatch(&t, e.id));
    assert_eq!(push_count(&t), 1);
    assert!(
        t.events()
            .iter()
            .any(|e| matches!(e,Event::Job(j) if j.decode::<ReminderPushJob>().is_some()))
    );
}
#[test]
fn room_involvement_and_inbox_preferences_control_reminder_items() {
    let t = frozen();
    let e = create(&t);
    respond(&t, e.id, "jason", "going", false);
    respond(&t, e.id, "jz", "maybe", false);
    t.write(|tx| {
        tx.conn().execute(
            "UPDATE memberships SET involvement='nothing' WHERE room_id=? AND user_id=?",
            params![id("designers"), id("jason")],
        )?;
        tx.conn().execute(
            "UPDATE users SET inbox_preferences=? WHERE id=?",
            params![r#"{"event_reminders":false}"#, id("jz")],
        )?;
        Ok(())
    });
    assert!(dispatch(&t, e.id));
    assert_eq!(
        item(&t, e.id, "jason").unwrap().event_type,
        "event_invitation"
    );
    assert_eq!(item(&t, e.id, "jz").unwrap().event_type, "event_invitation");
    assert_eq!(push_count(&t), 1);
}
#[test]
fn due_window_is_inclusive_and_stale_claims_are_silent() {
    let t = frozen();
    #[derive(serde::Deserialize)]
    struct Case {
        offset: i64,
        due: bool,
        pushed: bool,
    }
    let cases: Vec<Case> = serde_json::from_str(include_str!(
        "../models/calendar_event/reminder_window.json"
    ))
    .unwrap();
    for Case {
        offset,
        due,
        pushed,
    } in cases
    {
        let mut a = attrs(&t);
        a.starts_at = Some(t.now().since(SignedDuration::from_secs(offset)));
        let e = t.write(move |tx| CalendarEvent::create(tx, a));
        assert_eq!(
            t.read(|c| CalendarEvent::due_reminder_ids(c, t.now()))
                .contains(&e.id),
            due,
            "{offset}"
        );
        if due {
            let before = push_count(&t);
            assert_eq!(dispatch(&t, e.id), pushed);
            assert_eq!(push_count(&t) - before, usize::from(pushed));
        }
    }
}
#[test]
fn ended_cancelled_and_soft_deleted_rooms_stay_silent() {
    let t = frozen();
    let mut a = attrs(&t);
    a.starts_at = Some(t.now().ago(SignedDuration::from_mins(3)));
    a.ends_at = Some(t.now());
    let e = t.write(move |tx| CalendarEvent::create(tx, a));
    assert!(!dispatch(&t, e.id));
    assert_eq!(push_count(&t), 0);
    let e = create(&t);
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE events SET cancelled_at=? WHERE id=?",
            params![tx.now(), e.id],
        )?;
        Ok(())
    });
    assert!(
        !t.read(|c| CalendarEvent::due_reminder_ids(c, t.now()))
            .contains(&e.id)
    );
    let e = create(&t);
    t.write(|tx| {
        tx.conn().execute(
            "UPDATE rooms SET deleted_at=? WHERE id=?",
            params![tx.now(), id("designers")],
        )?;
        Ok(())
    });
    assert!(
        !t.read(|c| CalendarEvent::due_reminder_ids(c, t.now()))
            .contains(&e.id)
    );
    assert!(!dispatch(&t, e.id));
}
#[test]
fn concurrent_database_handles_claim_one_reminder() {
    let t = frozen();
    let e = create(&t);
    let db2 = t.another_process();
    t.sink.take();
    let db1 = t.db.clone();
    let now = t.now();
    let a = std::thread::spawn(move || {
        db1.write_blocking(move |tx| CalendarEvent::dispatch_reminder(tx, e.id, now))
            .unwrap()
    });
    let b = std::thread::spawn(move || {
        db2.write_blocking(move |tx| CalendarEvent::dispatch_reminder(tx, e.id, now))
            .unwrap()
    });
    assert_ne!(a.join().unwrap(), b.join().unwrap());
    assert_eq!(push_count(&t), 1);
}
#[test]
fn references_follow_message_edits_and_reject_other_rooms() {
    let t = frozen();
    let e = create(&t);
    let source = format!(
        "https://another.example.test/rooms/999/events/{}?x=1#y and /rooms/1/events/{}",
        e.id, e.id
    );
    let m = t.write(move |tx| {
        crate::Message::create(
            tx,
            crate::NewMessage {
                room_id: e.room_id,
                creator_id: id("david"),
                markdown_source: Some(source),
                ..Default::default()
            },
        )
    });
    assert_eq!(
        t.read(|c| Ok(c.query_row(
            "SELECT COUNT(*) FROM event_references WHERE message_id=?",
            [m.id],
            |r| r.get::<_, i64>(0)
        )?)),
        1
    );
    let message_id = m.id;
    t.write(move |tx| {
        let mut m = m;
        m.edit(
            tx,
            crate::MessageChanges {
                markdown_source: Some("never mind".into()),
                ..Default::default()
            },
        )
    });
    assert_eq!(
        t.read(|c| Ok(c.query_row(
            "SELECT COUNT(*) FROM event_references WHERE message_id=?",
            [message_id],
            |r| r.get::<_, i64>(0)
        )?)),
        0
    );
    let source = format!("/rooms/1/events/{}", e.id);
    let m = t.write(move |tx| {
        crate::Message::create(
            tx,
            crate::NewMessage {
                room_id: id("watercooler"),
                creator_id: id("david"),
                markdown_source: Some(source),
                ..Default::default()
            },
        )
    });
    assert_eq!(
        t.read(|c| Ok(c.query_row(
            "SELECT COUNT(*) FROM event_references WHERE message_id=?",
            [m.id],
            |r| r.get::<_, i64>(0)
        )?)),
        0
    );
}
