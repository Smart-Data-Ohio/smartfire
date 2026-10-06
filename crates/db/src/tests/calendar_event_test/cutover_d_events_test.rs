//! Exact outstanding Rails model declarations at parity/reference.sha.
use super::*;
use crate::models::calendar_event::changes::EventChanges;
use serde_json::{Value, json};

fn stamp(s: &str) -> Timestamp {
    Timestamp::parse_db(s).unwrap()
}
fn weekly(t: &TestDb, start: &str, until: &str) -> CalendarEvent {
    let mut a = attrs(t);
    a.starts_at = Some(stamp(start));
    a.ends_at = Some(a.starts_at.unwrap().since(SignedDuration::from_hours(1)));
    a.recurrence_rule = Some("weekly".into());
    a.recurrence_until = Some(until.parse().unwrap());
    t.write(move |tx| CalendarEvent::create(tx, a))
}
fn rows(t: &TestDb, head: i64) -> Vec<CalendarEvent> {
    t.read(|c| CalendarEvent::find(c, head)?.series_events(c))
}
fn update(t: &TestDb, eid: i64, changes: EventChanges, scope: &str) {
    let scope = scope.to_owned();
    t.write(move |tx| {
        CalendarEvent::update_with_scope(tx, eid, changes, &scope, Some(id("david")))
    });
}
fn cancel(t: &TestDb, eid: i64) -> bool {
    t.write(move |tx| CalendarEvent::cancel_with_scope(tx, eid, "this_event", Some(id("david"))))
}
fn jobs(t: &TestDb) -> Vec<(String, Value)> {
    t.events()
        .into_iter()
        .filter_map(|e| match e {
            Event::Job(job) => Some((job.class.to_owned(), job.arguments)),
            _ => None,
        })
        .collect()
}
fn due(t: &TestDb) {
    for eid in t.read(|c| CalendarEvent::due_reminder_ids(c, t.now())) {
        dispatch(t, eid);
    }
}
fn standup(t: &TestDb) -> CalendarEvent {
    let mut a = attrs(t);
    a.title = "Standup".into();
    let e = t.write(move |tx| CalendarEvent::create(tx, a));
    for (name, response) in [("jason", "going"), ("jz", "maybe"), ("kevin", "declined")] {
        let eid = e.id;
        let user = id(name);
        t.write(move |tx| EventAttendance::create(tx, eid, user, response));
    }
    e
}

#[test]
fn cutover_d_attendance_member_holds_one_response_and_duplicate_creation_is_invalid() {
    let t = frozen();
    let attendance = t
        .read(|c| EventAttendance::find_for(c, id("launch_party"), id("jason")))
        .unwrap();
    // Rails 11
    assert_eq!(attendance.response, "maybe");
    t.write(move |tx| EventAttendance::update(tx, attendance.id, Some("going"), None));
    // Rails 13
    assert_eq!(
        t.read(|c| EventAttendance::find_for(c, id("launch_party"), id("jason")))
            .unwrap()
            .response,
        "going"
    );
    // Rails 15
    assert!(matches!(
        t.try_write(|tx| EventAttendance::create(tx, id("launch_party"), id("jason"), "maybe")),
        Err(Error::RecordInvalid(_))
    ));
    assert_eq!(
        t.read(|c| Ok(c.query_row(
            "SELECT COUNT(*) FROM event_attendances WHERE event_id=? AND user_id=?",
            params![id("launch_party"), id("jason")],
            |r| r.get::<_, i64>(0)
        )?)),
        1
    );
}

#[test]
fn cutover_d_recurrence_response_copy_skips_api_cancelled_occurrence() {
    let t = frozen();
    let h = weekly(&t, "2026-09-23 09:00:00", "2026-10-07");
    let occurrences = rows(&t, h.id);
    respond(&t, h.id, "jason", "going", false);
    respond(&t, occurrences[1].id, "jason", "declined", false);
    cancel(&t, occurrences[2].id);
    respond(&t, h.id, "jason", "maybe", false);
    // Rails 308
    assert_eq!(
        t.read(|c| occurrences[0].response_for(c, Some(id("jason")))),
        Some("maybe".into())
    );
    // Rails 309
    assert_eq!(
        t.read(|c| occurrences[1].response_for(c, Some(id("jason")))),
        Some("maybe".into())
    );
    // Rails 310
    assert_eq!(
        t.read(|c| occurrences[2].response_for(c, Some(id("jason")))),
        Some("going".into())
    );
}

#[test]
fn cutover_d_recurrence_strictly_before_previous_rejects_both_scopes_and_preserves_series() {
    let t = frozen();
    let h = weekly(&t, "2027-01-01 09:00:00", "2027-01-22");
    let before = rows(&t, h.id);
    let eid = before[1].id;
    for scope in ["this_event", "this_and_following"] {
        let result = t.try_write(move |tx| {
            CalendarEvent::update_with_scope(
                tx,
                eid,
                EventChanges {
                    starts_at: Some(Some(stamp("2026-12-31 09:00:00"))),
                    ends_at: Some(Some(stamp("2026-12-31 10:00:00"))),
                    ..Default::default()
                },
                scope,
                Some(id("david")),
            )
        });
        // Rails 424
        assert!(matches!(&result, Err(Error::RecordInvalid(_))));
        let Error::RecordInvalid(errors) = result.unwrap_err() else {
            unreachable!()
        };
        // Rails 430
        assert_eq!(
            errors.on("starts_at"),
            vec!["must stay between the neighbouring occurrences in its series"]
        );
    }
    // Rails 433
    assert_eq!(
        rows(&t, h.id)
            .iter()
            .map(|e| e.starts_at)
            .collect::<Vec<_>>(),
        before.iter().map(|e| e.starts_at).collect::<Vec<_>>()
    );
}

#[test]
fn cutover_d_recurrence_strictly_past_next_rejects_local_and_shifts_complete_following_series() {
    let t = frozen();
    let h = weekly(&t, "2027-01-01 09:00:00", "2027-01-22");
    let eid = rows(&t, h.id)[1].id;
    let changes = EventChanges {
        starts_at: Some(Some(stamp("2027-01-16 09:00:00"))),
        ends_at: Some(Some(stamp("2027-01-16 10:00:00"))),
        ..Default::default()
    };
    let rejected = changes.clone();
    let result = t.try_write(move |tx| {
        CalendarEvent::update_with_scope(tx, eid, rejected, "this_event", Some(id("david")))
    });
    // Rails 442
    assert!(matches!(&result, Err(Error::RecordInvalid(_))));
    let Error::RecordInvalid(errors) = result.unwrap_err() else {
        unreachable!()
    };
    // Rails 448
    assert_eq!(
        errors.on("starts_at"),
        vec!["must stay between the neighbouring occurrences in its series"]
    );
    // Rails 449
    assert_eq!(
        t.read(|c| CalendarEvent::find(c, eid)).starts_at,
        stamp("2027-01-08 09:00:00")
    );
    update(&t, eid, changes, "this_and_following");
    // Rails 456
    assert_eq!(
        rows(&t, h.id)
            .iter()
            .map(|e| e.starts_at)
            .collect::<Vec<_>>(),
        [
            "2027-01-01 09:00:00",
            "2027-01-16 09:00:00",
            "2027-01-23 09:00:00",
            "2027-01-30 09:00:00"
        ]
        .map(stamp)
    );
}

#[test]
fn cutover_d_recurrence_following_time_change_replaces_going_and_maybe_updates() {
    let t = frozen();
    let h = weekly(&t, "2026-09-23 09:00:00", "2026-10-07");
    let occurrences = rows(&t, h.id);
    respond(&t, h.id, "jason", "going", false);
    respond(&t, h.id, "jz", "maybe", false);
    update(
        &t,
        occurrences[1].id,
        EventChanges {
            starts_at: Some(Some(
                occurrences[1]
                    .starts_at
                    .since(SignedDuration::from_hours(1)),
            )),
            ends_at: Some(
                occurrences[1]
                    .ends_at
                    .map(|s| s.since(SignedDuration::from_hours(1))),
            ),
            ..Default::default()
        },
        "this_event",
    );
    let earlier = item(&t, occurrences[1].id, "jz").unwrap();
    // Rails 573
    assert_eq!(earlier.event_type, "event_update");
    update(
        &t,
        h.id,
        EventChanges {
            starts_at: Some(Some(h.starts_at.since(SignedDuration::from_hours(2)))),
            ends_at: Some(h.ends_at.map(|s| s.since(SignedDuration::from_hours(2)))),
            ..Default::default()
        },
        "this_and_following",
    );
    let sources = occurrences.iter().map(|e| e.id).collect::<Vec<_>>();
    for user in [id("jason"), id("jz")] {
        let unread:Vec<(String,i64)>=t.read(|c|{Ok(c.prepare("SELECT event_type,source_id FROM activity_items WHERE user_id=? AND source_type='Event' AND source_id IN (SELECT value FROM json_each(?)) AND read_at IS NULL AND handled_at IS NULL")?.query_map(params![user,json!(sources).to_string()],|r|Ok((r.get(0)?,r.get(1)?)))?.collect::<rusqlite::Result<_>>()?)});
        // Rails 585
        assert_eq!(unread.len(), 1);
        // Rails 586
        assert_eq!(unread[0].0, "event_update");
        // Rails 587
        assert_eq!(unread[0].1, h.id);
    }
    let handled = t.read(|c| ActivityItem::find(c, earlier.id));
    // Rails 589
    assert!(handled.handled_at.is_some());
    // Rails 590
    assert_eq!(t.read(|c|Ok(c.query_row("SELECT COUNT(*) FROM activity_items WHERE user_id=? AND source_type='Event' AND source_id IN (SELECT value FROM json_each(?))",params![id("david"),json!(sources).to_string()],|r|r.get::<_,i64>(0))?)),0);
}

#[test]
fn cutover_d_recurrence_shortening_notifies_going_but_not_declined_on_same_excess_event() {
    let t = frozen();
    let h = weekly(&t, "2026-10-05 09:00:00", "2026-10-26");
    let occurrences = rows(&t, h.id);
    // Rails 744
    assert_eq!(occurrences.len(), 4);
    respond(&t, h.id, "jason", "going", false);
    respond(&t, occurrences[1].id, "jason", "declined", false);
    respond(&t, occurrences[2].id, "jz", "going", false);
    respond(&t, occurrences[2].id, "kevin", "declined", false);
    update(
        &t,
        h.id,
        EventChanges {
            recurrence_rule: Some(Some("daily".into())),
            recurrence_until: Some(Some("2026-10-06".parse().unwrap())),
            ..Default::default()
        },
        "this_and_following",
    );
    let excess = t.read(|c| CalendarEvent::find(c, occurrences[2].id));
    // Rails 756
    assert!(excess.cancelled());
    // Rails 757
    assert_eq!(
        item(&t, excess.id, "jz").unwrap().event_type,
        "event_cancelled"
    );
    // Rails 758
    assert_eq!(t.read(|c|Ok(c.query_row("SELECT COUNT(*) FROM activity_items WHERE user_id=? AND source_type='Event' AND source_id=? AND event_type='event_cancelled'",params![id("kevin"),excess.id],|r|r.get::<_,i64>(0))?)),0);
}

#[test]
fn cutover_d_recurrence_compound_time_and_until_update_moves_distinct_kept_occurrence() {
    let t = frozen();
    let h = weekly(&t, "2026-10-05 09:00:00", "2026-10-19");
    let occurrences = rows(&t, h.id);
    respond(&t, h.id, "jason", "going", false);
    respond(&t, occurrences[1].id, "jason", "declined", false);
    update(
        &t,
        h.id,
        EventChanges {
            starts_at: Some(Some(stamp("2026-10-05 11:00:00"))),
            ends_at: Some(Some(stamp("2026-10-05 12:00:00"))),
            recurrence_until: Some(Some("2026-10-20".parse().unwrap())),
            ..Default::default()
        },
        "this_and_following",
    );
    let kept = t.read(|c| CalendarEvent::find(c, occurrences[1].id));
    // Rails 779
    assert_eq!(kept.starts_at, stamp("2026-10-12 11:00:00"));
    // Rails 780
    assert_eq!(
        t.read(|c| kept.response_for(c, Some(id("jason")))),
        Some("declined".into())
    );
}

#[test]
fn cutover_d_reminder_preference_optout_preserves_invitation_updates_and_followup() {
    let t = frozen();
    let e = standup(&t);
    t.write(|tx| {
        tx.conn().execute(
            "UPDATE users SET inbox_preferences=? WHERE id=?",
            params![json!({"event_reminders":false}).to_string(), id("jason")],
        )?;
        Ok(())
    });
    t.sink.take();
    due(&t);
    // Rails 42
    assert_eq!(
        jobs(&t),
        vec![("Event::ReminderPushJob".into(), json!({"event_id":e.id}))]
    );
    // Rails 46
    assert_eq!(
        item(&t, e.id, "jason").unwrap().event_type,
        "event_invitation"
    );
    // Rails 47
    assert_eq!(item(&t, e.id, "jz").unwrap().event_type, "event_reminder");
    update(
        &t,
        e.id,
        EventChanges {
            starts_at: Some(Some(t.now().since(SignedDuration::from_hours(48)))),
            ..Default::default()
        },
        "this_event",
    );
    // Rails 50
    assert_eq!(item(&t, e.id, "jason").unwrap().event_type, "event_update");
    let mut a = attrs(&t);
    a.title = "Follow-up".into();
    a.starts_at = Some(t.now().since(SignedDuration::from_hours(48)));
    let followup = t.write(move |tx| CalendarEvent::create(tx, a));
    // Rails 55
    assert_eq!(
        item(&t, followup.id, "jason").unwrap().event_type,
        "event_invitation"
    );
}

#[test]
fn cutover_d_reminder_api_cancelled_due_event_is_skipped_without_changing_cancelled_item() {
    let t = frozen();
    let e = standup(&t);
    cancel(&t, e.id);
    t.sink.take();
    due(&t);
    // Rails 69
    assert!(
        jobs(&t)
            .iter()
            .all(|(class, _)| class != "Event::ReminderPushJob")
    );
    // Rails 73
    assert!(
        t.read(|c| CalendarEvent::find(c, e.id))
            .reminded_at
            .is_none()
    );
    // Rails 74
    assert_eq!(
        item(&t, e.id, "jason").unwrap().event_type,
        "event_cancelled"
    );
}

#[test]
fn cutover_d_calendar_direct_attendance_creation_enqueues_exact_event_and_kevin() {
    let t = frozen();
    t.sink.take();
    t.write(|tx| EventAttendance::create(tx, id("launch_party"), id("kevin"), "going"));
    // Rails 467
    assert_eq!(
        jobs(&t),
        vec![(
            "Calendar::SyncEntryJob".into(),
            json!({"event_id":id("launch_party"),"user_id":id("kevin")})
        )]
    );
}

#[test]
fn cutover_d_calendar_direct_response_update_enqueues_but_timestamp_only_save_does_not() {
    let t = frozen();
    let a = t
        .read(|c| EventAttendance::find_for(c, id("launch_party"), id("jason")))
        .unwrap();
    t.sink.take();
    t.write(move |tx| EventAttendance::update(tx, a.id, Some("declined"), None));
    // Rails 475
    assert_eq!(
        jobs(&t),
        vec![(
            "Calendar::SyncEntryJob".into(),
            json!({"event_id":id("launch_party"),"user_id":id("jason")})
        )]
    );
    t.sink.take();
    let changed = t.write(move |tx| {
        EventAttendance::update(
            tx,
            a.id,
            None,
            Some(tx.now().ago(SignedDuration::from_hours(24))),
        )
    });
    // Rails 479
    assert!(jobs(&t).is_empty());
    assert_eq!(
        changed.updated_at,
        t.now().ago(SignedDuration::from_hours(24))
    );
}

#[test]
fn cutover_d_calendar_singleton_cancel_enqueues_each_of_two_distinct_users_entries() {
    let t = frozen();
    t.write(|tx| {
        crate::models::google_entry::reserve(tx, id("launch_party"), id("david"))?;
        crate::models::google_entry::reserve(tx, id("launch_party"), id("jason"))?;
        Ok(())
    });
    t.sink.take();
    cancel(&t, id("launch_party"));
    // Rails 537
    assert_eq!(
        jobs(&t),
        vec![
            (
                "Calendar::SyncEntryJob".into(),
                json!({"event_id":id("launch_party"),"user_id":id("david")})
            ),
            (
                "Calendar::SyncEntryJob".into(),
                json!({"event_id":id("launch_party"),"user_id":id("jason")})
            )
        ]
    );
}
