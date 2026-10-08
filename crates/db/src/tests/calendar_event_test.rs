//! Ports of the event models at d7c7de92. Scope reconstruction and HTML are separate slices.
use super::*;
use crate::models::calendar_event::{SyncEntryJob, reminders::ReminderPushJob};
use crate::{
    ActivityItem, CalendarEvent, Error, EventAttendance, NewCalendarEvent, Room, RoomType,
    Timestamp, User,
};
use jiff::SignedDuration;
use rusqlite::params;

mod calendar_api_test;
mod membership_calendar_test;
mod cutover_event_test;
mod cutover_entry_test;
mod cutover_timeline_test;
mod cutover_recurrence_test;
mod cutover_d_events_test;

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
fn pr174_invitation_failure_preserves_committed_recipients() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "calendar_event_test/invitation-boundaries.json"
    ))
    .unwrap();
    for case in oracle.as_array().unwrap() {
        let t = frozen();
        let rejected = case["rejected"].as_i64().unwrap();
        t.write(move |tx| {
            tx.conn().execute_batch(&format!("CREATE TEMP TRIGGER ws14e_reject_invitation BEFORE INSERT ON activity_items WHEN NEW.source_type='Event' AND NEW.user_id={rejected} BEGIN SELECT RAISE(ABORT, 'ws14e rejected invitation'); END"))?;
            Ok(())
        });
        let a = attrs(&t);
        assert!(t.try_write(move |tx| CalendarEvent::create(tx, a)).is_err());
        let event = t.read(|c| {
            let eid = c.query_row(
                "SELECT id FROM events WHERE title='Planning session'",
                [],
                |r| r.get(0),
            )?;
            CalendarEvent::find(c, eid)
        });
        let invited: Vec<i64> = t.read(|c| {
            let mut s = c.prepare("SELECT user_id FROM activity_items WHERE source_type='Event' AND source_id=? AND event_type='event_invitation' ORDER BY user_id")?;
            Ok(s.query_map([event.id], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?)
        });
        assert_eq!(
            serde_json::to_value(invited).unwrap(),
            case["invited"],
            "recipient position {}",
            case["position"]
        );
        assert!(case["persisted"].as_bool().unwrap());
        assert_eq!(
            t.read(|c| event.response_for(c, Some(id("david")))),
            Some(case["organizer_response"].as_str().unwrap().into())
        );
        assert_eq!(
            t.read(|c| Ok(c.query_row(
                "SELECT COUNT(*) FROM event_references WHERE event_id=?",
                [event.id],
                |r| r.get::<_, i64>(0)
            )?)),
            case["announcements"].as_i64().unwrap()
        );
        // Rails stops its later sync callback (see WS11's pinned failure probe).
        // Our durable-enqueue exception already committed that job, so its queue
        // notification survives even though remaining model callbacks stop.
        assert_eq!(
            t.events()
                .iter()
                .filter(|e| matches!(e, Event::Job(j) if j.class == "Calendar::SyncEntryJob"))
                .count(),
            1
        );
    }
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
            "Scheduled an event: Planning session\nhttp://example.com/rooms/{}/events/{}",
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
    assert!(ids.len() >= 2);
    assert!(
        t.read(|c| event.series_events(c))
            .iter()
            .all(|e| e.meet_link_requested)
    );
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
    t.sink.take();
    let mut singleton = attrs(&t);
    singleton.meet_link_requested = true;
    let singleton = t.write(move |tx| CalendarEvent::create(tx, singleton));
    assert!(
        t.read(|c| CalendarEvent::find(c, singleton.id))
            .meet_link_requested
    );
    let jobs: Vec<_> = t
        .events()
        .into_iter()
        .filter_map(|event| match event {
            Event::Job(job) if job.class == "Calendar::MeetLinkJob" => Some(job),
            _ => None,
        })
        .collect();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].arguments["event_id"], singleton.id);
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

fn vector_changes(a: &serde_json::Value) -> crate::models::calendar_event::changes::EventChanges {
    use crate::models::calendar_event::changes::EventChanges;
    EventChanges {
        title: a.get("title").map(|v| v.as_str().unwrap().into()),
        description: a.get("description").map(|v| v.as_str().map(str::to_string)),
        starts_at: a
            .get("starts_at")
            .map(|v| v.as_str().map(|s| Timestamp::parse_db(s).unwrap())),
        ends_at: a
            .get("ends_at")
            .map(|v| v.as_str().map(|s| Timestamp::parse_db(s).unwrap())),
        time_zone: a.get("time_zone").map(|v| v.as_str().unwrap().into()),
        venue_room_id: a.get("venue_room_id").map(|v| v.as_i64()),
        recurrence_rule: a
            .get("recurrence_rule")
            .map(|v| v.as_str().map(str::to_string)),
        recurrence_until: a
            .get("recurrence_until")
            .map(|v| v.as_str().map(|s| s.parse().unwrap())),
        meet_link_requested: a.get("meet_link_requested").map(|v| v.as_bool().unwrap()),
    }
}

#[test]
fn event_scoped_operations_match_rails_vectors() {
    use serde_json::{Value, json};
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("../models/calendar_event/scoped.json")).unwrap();
    for case in cases {
        let t = frozen();
        let input = &case["input"];
        let mut a = attrs(&t);
        a.starts_at = input["starts_at"]
            .as_str()
            .map(|s| Timestamp::parse_db(s).unwrap());
        a.ends_at = input["ends_at"]
            .as_str()
            .map(|s| Timestamp::parse_db(s).unwrap());
        a.recurrence_rule = input["recurrence_rule"].as_str().map(str::to_string);
        a.meet_link_requested = input["meet_link_requested"].as_bool().unwrap_or(false);
        a.recurrence_until = Some(input["recurrence_until"].as_str().unwrap().parse().unwrap());
        let head = t.write(move |tx| CalendarEvent::create(tx, a));
        let mut ids = t
            .read(|c| head.series_events(c))
            .into_iter()
            .map(|e| e.id)
            .collect::<Vec<_>>();
        t.sink.take();
        for (step, expected) in case["steps"]
            .as_array()
            .unwrap()
            .iter()
            .zip(case["results"].as_array().unwrap())
        {
            let event = ids[step["index"].as_u64().unwrap() as usize];
            let scope = step["scope"].as_str().unwrap_or("").to_string();
            let changes = vector_changes(&step["attrs"]);
            let result = match step["kind"].as_str().unwrap() {
                "rsvp" => {
                    let user = id(step["user"].as_str().unwrap());
                    let response = step["response"].as_str().unwrap().to_string();
                    t.try_write(move |tx| {
                        CalendarEvent::respond(tx, event, user, &response, false)
                            .map(|_| Value::Null)
                    })
                }
                "update" => t.try_write(move |tx| {
                    CalendarEvent::update_with_scope(tx, event, changes, &scope, Some(id("david")))
                        .map(Value::Bool)
                }),
                "cancel" => t.try_write(move |tx| {
                    CalendarEvent::cancel_with_scope(tx, event, &scope, Some(id("david")))
                        .map(Value::Bool)
                }),
                _ => unreachable!(),
            };
            let actual = match result {
                Ok(value) => json!({"value":value}),
                Err(Error::RecordInvalid(errors)) => json!({"errors":errors.0}),
                Err(error) => panic!("{}: {error}", case["name"]),
            };
            assert_eq!(actual, *expected, "{} step {step}", case["name"]);
        }
        let current = t.read(|c| head.series_events(c));
        for e in &current {
            if !ids.contains(&e.id) {
                ids.push(e.id);
            }
        }
        let stamp = |ts: Timestamp| ts.jiff().strftime("%Y-%m-%d %H:%M:%S.%6f").to_string();
        let rows=ids.iter().map(|event|{
            t.read(|c|match CalendarEvent::find(c,*event) {
                Ok(e)=>{
                    let mut responses=EventAttendance::for_event(c,e.id)?;
                    responses.sort_by_key(|a|a.user_id);
                    Ok(json!({"starts_at":stamp(e.starts_at),"ends_at":e.ends_at.map(stamp),"title":e.title,"description":e.description,"rule":e.recurrence_rule,"until":e.recurrence_until.unwrap().to_string(),"cancelled":e.cancelled(),"responses":responses.iter().map(|a|json!([a.user_id,a.response])).collect::<Vec<_>>()}))
                },
                Err(Error::RecordNotFound(_))=>Ok(Value::Null),
                Err(error)=>Err(error),
            })
        }).collect::<Vec<_>>();
        assert_eq!(json!(rows), case["rows"], "{} rows", case["name"]);
        let items=t.read(|c|{
            let mut stmt=c.prepare("SELECT user_id,source_id,event_type,read_at IS NULL,handled_at IS NULL FROM activity_items WHERE source_type='Event' ORDER BY user_id,source_id")?;
            let values=stmt.query_map([],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?,r.get::<_,String>(2)?,r.get::<_,bool>(3)?,r.get::<_,bool>(4)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(values.into_iter().filter_map(|(u,s,e,r,h)|ids.iter().position(|id|*id==s).map(|index|json!([u,index,e,r,h]))).collect::<Vec<_>>())
        });
        assert_eq!(json!(items), case["items"], "{} items", case["name"]);
        let jobs = t
            .events()
            .iter()
            .filter_map(|event| match event {
                Event::Job(job) if job.class.starts_with("Calendar::") => {
                    let event = job.arguments["event_id"].as_i64().unwrap();
                    let args = if job.class == "Calendar::MeetLinkJob" {
                        json!([ids.iter().position(|id| *id == event)])
                    } else {
                        json!([
                            ids.iter().position(|id| *id == event),
                            job.arguments["user_id"]
                        ])
                    };
                    Some(json!([job.class, args]))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(json!(jobs), case["jobs"], "{} jobs", case["name"]);
    }
}

#[test]
fn event_scoped_placement_failure_rolls_back_parking_and_jobs() {
    use crate::models::calendar_event::changes::EventChanges;
    let t = frozen();
    let head = series(&t);
    let before = t.read(|c| head.series_events(c));
    let victim = before[2].id;
    let expected = before.clone();
    t.write(move |tx|{
        tx.conn().execute_batch(&format!("CREATE TRIGGER reject_event_placement BEFORE UPDATE ON events WHEN OLD.id={victim} AND OLD.series_id IS NULL AND NEW.series_id IS NOT NULL BEGIN SELECT RAISE(ABORT,'placement failed'); END"))?;
        Ok(())
    });
    t.sink.take();
    let head_id = head.id;
    assert!(
        t.try_write(move |tx| CalendarEvent::update_with_scope(
            tx,
            head_id,
            EventChanges {
                starts_at: Some(Some(
                    before[0]
                        .starts_at
                        .since(SignedDuration::from_hours(24 * 7))
                )),
                ends_at: Some(Some(
                    before[0]
                        .starts_at
                        .since(SignedDuration::from_hours(24 * 7))
                        .since(SignedDuration::from_hours(1))
                )),
                ..Default::default()
            },
            "this_and_following",
            Some(id("david"))
        ))
        .is_err()
    );
    assert_eq!(t.read(|c| head.series_events(c)), expected);
    assert!(t.events().is_empty());
}

#[test]
fn scoped_recurrence_guards_do_not_escape_the_operation() {
    use crate::models::calendar_event::changes::EventChanges;
    let t = frozen();
    let head = series(&t);
    let event = head.id;
    t.write(move |tx| {
        CalendarEvent::update_with_scope(
            tx,
            event,
            EventChanges {
                recurrence_until: Some(Some("2026-10-13".parse().unwrap())),
                ..Default::default()
            },
            "this_and_following",
            Some(id("david")),
        )
    });
    let result = t.try_write(move |tx| {
        CalendarEvent::update(
            tx,
            event,
            EventChanges {
                recurrence_rule: Some(Some("daily".into())),
                ..Default::default()
            },
        )
    });
    assert!(
        matches!(result,Err(Error::RecordInvalid(errors)) if errors.on("recurrence_rule")==["can only be changed from the first event in the series using This and following"])
    );
    let result = t.try_write(move |tx| {
        CalendarEvent::update(
            tx,
            event,
            EventChanges {
                starts_at: Some(Some(head.starts_at.since(SignedDuration::from_mins(5)))),
                ..Default::default()
            },
        )
    });
    assert!(
        matches!(result,Err(Error::RecordInvalid(errors)) if errors.on("starts_at")==["moves the whole series: choose This and following or the entire series"])
    );
}

#[test]
fn series_notifications_replace_read_updates_and_rearm_every_reminder() {
    use crate::models::calendar_event::changes::EventChanges;
    let t = frozen();
    let head = series(&t);
    let events = t.read(|c| head.series_events(c));
    respond(&t, head.id, "jason", "going", false);
    let future = events[1].id;
    t.write(move |tx| {
        let item = ActivityItem::refresh_unread(tx, id("jason"), "Event", future, "event_update")?;
        tx.conn().execute(
            "UPDATE activity_items SET read_at=? WHERE id=?",
            params![tx.now(), item.id],
        )?;
        tx.conn().execute(
            "UPDATE memberships SET involvement='nothing' WHERE room_id=? AND user_id=?",
            params![id("designers"), id("jason")],
        )?;
        tx.conn().execute(
            "UPDATE events SET reminded_at=? WHERE series_id=?",
            params![tx.now(), head.id],
        )?;
        CalendarEvent::update_with_scope(
            tx,
            head.id,
            EventChanges {
                starts_at: Some(Some(head.starts_at.since(SignedDuration::from_mins(5)))),
                ..Default::default()
            },
            "this_and_following",
            Some(id("david")),
        )?;
        Ok(())
    });
    assert!(item(&t, future, "jason").unwrap().handled_at.is_some());
    assert_eq!(
        item(&t, head.id, "jason").unwrap().event_type,
        "event_update"
    );
    assert!(
        t.read(|c| head.series_events(c))
            .iter()
            .all(|e| e.reminded_at.is_none())
    );
}

#[test]
fn rematerialization_failure_restores_original_series_rows() {
    use crate::models::calendar_event::changes::EventChanges;
    let t = frozen();
    let mut a = attrs(&t);
    a.starts_at = Timestamp::parse_db("2027-01-31 10:00:00");
    a.ends_at = Timestamp::parse_db("2027-01-31 11:00:00");
    a.recurrence_rule = Some("weekly".into());
    a.recurrence_until = Some("2027-03-07".parse().unwrap());
    let head = t.write(move |tx| CalendarEvent::create(tx, a));
    let original = t.read(|c| head.series_events(c));
    respond(&t, head.id, "jason", "going", false);
    respond(&t, original.last().unwrap().id, "jason", "declined", false);
    let victim = original[4].id;
    t.write(move |tx|{
        tx.conn().execute_batch(&format!("CREATE TRIGGER reject_rematerialization BEFORE UPDATE ON events WHEN OLD.id={victim} AND OLD.series_id IS NULL AND NEW.series_id IS NOT NULL BEGIN SELECT RAISE(ABORT,'rematerialization failed'); END"))?;
        Ok(())
    });
    t.sink.take();
    let event = head.id;
    assert!(
        t.try_write(move |tx| CalendarEvent::update_with_scope(
            tx,
            event,
            EventChanges {
                recurrence_rule: Some(Some("monthly".into())),
                recurrence_until: Some(Some("2027-07-31".parse().unwrap())),
                ..Default::default()
            },
            "this_and_following",
            Some(id("david"))
        ))
        .is_err()
    );
    assert_eq!(t.read(|c| head.series_events(c)), original);
    assert!(t.events().is_empty());
}

#[test]
fn reminder_push_source_rechecks_membership_and_ignores_inbox_preferences() {
    let t = frozen();
    let event = create(&t);
    respond(&t, event.id, "jason", "maybe", false);
    let event_id = event.id;
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE memberships SET involvement='nothing' WHERE room_id=? AND user_id=?",
            params![id("designers"), id("jason")],
        )?;
        tx.conn().execute(
            "UPDATE users SET inbox_preferences=? WHERE id=?",
            params![r#"{"event_reminders":false}"#, id("jason")],
        )?;
        Ok(())
    });
    let source = t
        .read(|c| CalendarEvent::reminder_push_source(c, event_id, t.now()))
        .unwrap();
    assert_eq!(source.recipient_ids, vec![id("david"), id("jason")]);
    assert_eq!(source.payload.title, "Designers");
    assert_eq!(
        source.payload.body,
        "Starts in 10 minutes: Planning session"
    );
    assert_eq!(
        source.payload.path,
        format!("/rooms/{}/events/{event_id}", id("designers"))
    );
    assert_eq!(source.payload.tag, format!("event-{event_id}"));
    t.write(move |tx| {
        tx.conn().execute(
            "DELETE FROM memberships WHERE room_id=? AND user_id=?",
            params![id("designers"), id("jason")],
        )?;
        Ok(())
    });
    assert_eq!(
        t.read(|c| CalendarEvent::reminder_push_source(c, event_id, t.now()))
            .unwrap()
            .recipient_ids,
        vec![id("david")]
    );
    assert!(
        t.read(|c| CalendarEvent::reminder_push_source(
            c,
            event_id,
            t.now().since(SignedDuration::from_mins(16))
        ))
        .is_none()
    );
}

#[test]
fn reminder_push_payload_and_staleness_match_rails_vectors() {
    use serde_json::{Value, json};
    let vectors: Vec<Value> =
        serde_json::from_str(include_str!("../models/calendar_event/pusher.json")).unwrap();
    for vector in vectors {
        let t = frozen();
        let mut a = attrs(&t);
        a.room_id = id(vector["room"].as_str().unwrap());
        a.starts_at = Some(t.now().since(SignedDuration::from_secs(
            vector["offset"].as_i64().unwrap(),
        )));
        let event = t.write(move |tx| CalendarEvent::create(tx, a));
        let source = t.read(|c| CalendarEvent::reminder_push_source(c, event.id, t.now()));
        let actual = source
            .map(|source| {
                let mut payload = source.payload;
                payload.path = format!("/rooms/{}/events/EVENT", event.room_id);
                payload.tag = "event-EVENT".into();
                json!(payload)
            })
            .unwrap_or(Value::Null);
        assert_eq!(actual, vector["payload"], "{vector}");
    }
}

#[test]
fn scoped_calendar_sync_and_cancel_preserve_ws14g_argument_contracts() {
    use crate::models::calendar_event::changes::EventChanges;
    let t = frozen();
    let head = series(&t);
    respond(&t, head.id, "jason", "maybe", false);
    let rows = t.read(|c| head.series_events(c));
    let head_id = head.id;
    t.write(move |tx|{
        for user in [id("david"),id("jason")] {
            tx.conn().execute("INSERT INTO google_accounts (user_id,email,created_at,updated_at) VALUES (?,?,?,?)",params![user,format!("{user}@example.test"),tx.now(),tx.now()])?;
        }
        for e in &rows {tx.conn().execute("INSERT INTO event_calendar_entries (event_id,user_id,google_event_id,created_at,updated_at) VALUES (?,?,?,?,?)",params![e.id,id("jason"),format!("remote-{}",e.id),tx.now(),tx.now()])?;}
        Ok(())
    });
    t.sink.take();
    t.write(move |tx| {
        CalendarEvent::update_with_scope(
            tx,
            head_id,
            EventChanges {
                title: Some("Updated series".into()),
                ..Default::default()
            },
            "this_and_following",
            Some(id("david")),
        )
    });
    let requests = t
        .events()
        .iter()
        .filter_map(|e| match e {
            Event::Job(j) if j.class == "Calendar::SyncEntryJob" => {
                j.decode::<SyncEntryJob>().unwrap().ok()
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let rows = t.read(|c| head.series_events(c));
    assert_eq!(
        requests
            .iter()
            .map(|j| (j.event_id, j.user_id))
            .collect::<Vec<_>>(),
        rows.iter()
            .flat_map(|e| [(e.id, id("david")), (e.id, id("jason"))])
            .collect::<Vec<_>>()
    );
    t.sink.take();
    t.write(move |tx| {
        CalendarEvent::cancel_with_scope(tx, head_id, "this_and_following", Some(id("david")))
    });
    let requests = t
        .events()
        .iter()
        .filter_map(|e| match e {
            Event::Job(j) if j.class == "Calendar::SyncEntryJob" => {
                j.decode::<SyncEntryJob>().unwrap().ok()
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        requests
            .iter()
            .map(|j| (j.event_id, j.user_id))
            .collect::<Vec<_>>(),
        rows.iter().map(|e| (e.id, id("jason"))).collect::<Vec<_>>()
    );
    t.sink.take();
    t.write(move |tx| CalendarEvent::find(tx.conn(), head_id)?.destroy(tx));
    let remote = t
        .events()
        .iter()
        .filter_map(|e| match e {
            Event::Job(j) if j.class == "Calendar::RemoteDeleteJob" => Some(j.arguments.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        remote,
        vec![serde_json::json!([
            id("jason"),
            format!("remote-{head_id}")
        ])]
    );
}

fn edge_vectors() -> serde_json::Value {
    serde_json::from_str(include_str!("../models/calendar_event/edges.json")).unwrap()
}
#[test]
fn event_after_commit_rejection_keeps_rails_rows() {
    let t = frozen();
    let room = t.write(|tx| {
        Room::create_for(
            tx,
            RoomType::Board,
            Some("Rejected event board"),
            id("david"),
            &[id("david"), id("jason")],
        )
    });
    let tables = ["events", "event_attendances", "activity_items", "messages"];
    let before = tables.map(|table| count(&t, table));
    let mut a = attrs(&t);
    a.room_id = room.id;
    assert!(t.try_write(move |tx| CalendarEvent::create(tx, a)).is_err());
    let actual: Vec<i64> = tables
        .into_iter()
        .zip(before)
        .map(|(table, n)| count(&t, table) - n)
        .collect();
    let expected: Vec<i64> = edge_vectors()["rejected"]["delta"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_i64().unwrap())
        .collect();
    assert_eq!(actual, expected);
}
#[test]
fn event_announcement_uses_rails_configured_origin() {
    for vector in edge_vectors()["urls"].as_array().unwrap() {
        let origin = vector["origin"].as_str().unwrap();
        let t = TestDb::with_clock_and_origin(
            TestClock::frozen_at(Timestamp::parse_db("2026-09-22 12:00:00").unwrap()),
            4,
            origin,
        );
        let mut a = attrs(&t);
        a.title = "Origin <&>".into();
        let event = t.write(move |tx| CalendarEvent::create(tx, a));
        let actual = t.read(|c| {
            Ok(c.query_row::<String, _, _>(
                "SELECT markdown_source FROM messages ORDER BY id DESC LIMIT 1",
                [],
                |r| r.get(0),
            )?)
        });
        let expected = vector["announcement"].as_str().unwrap().replace(
            vector["suffix"].as_str().unwrap(),
            &format!("/rooms/{}/events/{}", event.room_id, event.id),
        );
        assert_eq!(actual, expected);
    }
}
#[test]
fn event_nil_series_start_matches_rails_failure_without_writes() {
    use crate::models::calendar_event::changes::EventChanges;
    for vector in edge_vectors()["nil_starts"].as_array().unwrap() {
        assert_eq!(vector["exception"], "NoMethodError");
        assert_eq!(vector["unchanged"], true);
        let t = frozen();
        let head = series(&t);
        let before = t.read(|c| head.series_events(c));
        let event = before[vector["index"].as_u64().unwrap() as usize].id;
        let scope = vector["scope"].as_str().unwrap().to_owned();
        let before_events = t.events().len();
        let error = t
            .try_write(move |tx| {
                CalendarEvent::update_with_scope(
                    tx,
                    event,
                    EventChanges {
                        starts_at: Some(None),
                        title: Some("Must roll back".into()),
                        ..Default::default()
                    },
                    &scope,
                    Some(id("david")),
                )
            })
            .unwrap_err();
        assert!(matches!(error, Error::Other(_)), "{error:?}");
        assert_eq!(t.read(|c| head.series_events(c)), before);
        assert_eq!(t.events().len(), before_events);
    }
}

// test/models/event_test.rb:138: a dispatched singleton reminder is rearmed by
// the same announcement-producing update used by the event controller.
#[test]
fn singleton_time_change_rearms_a_dispatched_reminder() {
    use crate::models::calendar_event::changes::EventChanges;
    let t = frozen();
    let event = create(&t);
    assert!(dispatch(&t, event.id));
    assert!(
        t.read(|c| CalendarEvent::find(c, event.id))
            .reminded_at
            .is_some()
    );
    let starts_at = t.now().since(SignedDuration::from_hours(48));
    t.write(move |tx| {
        CalendarEvent::update_with_scope(
            tx,
            event.id,
            EventChanges {
                starts_at: Some(Some(starts_at)),
                ..Default::default()
            },
            "this_event",
            Some(id("david")),
        )
    });
    assert!(
        t.read(|c| CalendarEvent::find(c, event.id))
            .reminded_at
            .is_none()
    );
}

// test/models/event_test.rb:176: a later second cancellation preserves the
// original cancellation timestamp and every existing notification row.
#[test]
fn cancelling_a_singleton_twice_preserves_timestamp_and_activity_rows() {
    let t = frozen();
    let event = create(&t);
    respond(&t, event.id, "jason", "going", false);
    assert!(t.write(move |tx| {
        CalendarEvent::cancel_with_scope(tx, event.id, "this_event", Some(id("david")))
    }));
    let cancelled_at = t.read(|c| CalendarEvent::find(c, event.id)).cancelled_at;
    let items = ["david", "jason", "jz", "kevin"].map(|user| item(&t, event.id, user));
    let before = count(&t, "activity_items");
    t.travel(60);
    assert!(!t.write(move |tx| {
        CalendarEvent::cancel_with_scope(tx, event.id, "this_event", Some(id("david")))
    }));
    assert_eq!(
        t.read(|c| CalendarEvent::find(c, event.id)).cancelled_at,
        cancelled_at
    );
    assert_eq!(count(&t, "activity_items"), before);
    assert_eq!(
        ["david", "jason", "jz", "kevin"].map(|user| item(&t, event.id, user)),
        items
    );
}

/// The rooms the writes since the last `take` told their open calendar screens about, in order.
fn rooms_told(t: &TestDb) -> Vec<i64> {
    t.events()
        .iter()
        .filter_map(|e| match e {
            Event::Broadcast(request) => request
                .decode::<crate::models::calendar_event::EventsChanged>()
                .map(|change| change.unwrap().room_id),
            _ => None,
        })
        .collect()
}

// Later occurrences of a series have no message linking them, so the card update can't reach
// the single-page app's calendar screens for them: every write to an event tells its room,
// once per transaction, including the occurrences a shortened recurrence destroys.
#[test]
fn every_event_write_tells_its_room_once() {
    use crate::models::calendar_event::changes::EventChanges;
    let t = frozen();
    let room = id("designers");
    t.sink.take();
    let head = series(&t);
    assert_eq!(rooms_told(&t), [room], "scheduling a series");
    let ids: Vec<i64> = t
        .read(|c| head.series_events(c))
        .iter()
        .map(|e| e.id)
        .collect();
    assert_eq!(ids.len(), 3);

    t.sink.take();
    let later = ids[1];
    t.write(move |tx| {
        CalendarEvent::update_with_scope(
            tx,
            later,
            EventChanges {
                title: Some("Planning, moved".into()),
                ..Default::default()
            },
            "this_event",
            Some(id("david")),
        )
    });
    assert_eq!(rooms_told(&t), [room], "editing a later occurrence");

    t.sink.take();
    let (head_id, last) = (head.id, ids[2]);
    t.write(move |tx| {
        CalendarEvent::update_with_scope(
            tx,
            head_id,
            EventChanges {
                recurrence_until: Some(Some("2026-09-29".parse().unwrap())),
                ..Default::default()
            },
            "this_and_following",
            Some(id("david")),
        )
    });
    assert_eq!(rooms_told(&t), [room], "shortening the recurrence");
    assert!(
        t.read(|c| Ok(CalendarEvent::find(c, last).is_err())),
        "the occurrence past the new end is gone"
    );

    t.sink.take();
    assert!(t.write(move |tx| {
        CalendarEvent::cancel_with_scope(tx, later, "this_event", Some(id("david")))
    }));
    assert_eq!(rooms_told(&t), [room], "cancelling a later occurrence");

    let single = create(&t);
    t.sink.take();
    t.write(move |tx| single.destroy(tx));
    assert_eq!(rooms_told(&t), [room], "destroying an event");
}

mod cutover_reference_test;
mod cutover_reminder_test;
mod cutover_venue_test;
