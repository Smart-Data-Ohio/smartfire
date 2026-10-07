use super::*;

#[tokio::test]
async fn membership_removal_calendar_job_is_durable_and_atomic() {
    use campfire_db::fixtures::{Options, identify, load, reference_dir};
    use campfire_db::{Membership, models::google_entry};
    let (booted, _dir) = app().await;
    booted.jobs.shutdown(Duration::from_secs(5)).await;
    let app = booted.app.clone();
    let (room, user, event) = (
        identify("designers"),
        identify("david"),
        identify("launch_party"),
    );
    app.db.write(move |tx| {
        load(tx.conn(), &reference_dir(), &Options { now: tx.now(), bcrypt_cost: 4 })?;
        google_entry::reserve(tx, event, user)?;
        tx.conn().execute_batch("CREATE TRIGGER reject_membership_calendar BEFORE INSERT ON background_jobs WHEN NEW.job_class='Calendar::SyncEntryJob' BEGIN SELECT RAISE(ABORT,'queue unavailable'); END")?;
        Ok(())
    }).await.unwrap();
    assert!(
        app.db
            .write(
                move |tx| Membership::find_by_room_and_user(tx.conn(), room, user)?
                    .unwrap()
                    .destroy(tx)
            )
            .await
            .is_err()
    );
    assert!(
        app.db
            .read(move |c| Ok(Membership::find_by_room_and_user(c, room, user)?.is_some()))
            .await
            .unwrap()
    );
    assert!(jobs(&app).is_empty());
    app.db
        .write(move |tx| {
            tx.conn()
                .execute_batch("DROP TRIGGER reject_membership_calendar")?;
            Membership::find_by_room_and_user(tx.conn(), room, user)?
                .unwrap()
                .destroy(tx)
        })
        .await
        .unwrap();
    assert!(
        !app.db
            .read(move |c| Ok(Membership::find_by_room_and_user(c, room, user)?.is_some()))
            .await
            .unwrap()
    );
    let queued = jobs(&app);
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].class, "Calendar::SyncEntryJob");
    assert_eq!(
        queued[0].arguments,
        serde_json::json!({"event_id":event,"user_id":user})
    );
}

#[tokio::test]
async fn ws14e_one_invalid_event_does_not_stop_other_reminders() {
    use campfire_db::fixtures::{Options, identify, load, reference_dir};
    use campfire_db::{CalendarEvent, NewCalendarEvent};
    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    booted.jobs.shutdown(Duration::from_secs(5)).await;
    app.db
        .write(|tx| {
            load(
                tx.conn(),
                &reference_dir(),
                &Options {
                    now: tx.now(),
                    bcrypt_cost: 4,
                },
            )
        })
        .await
        .unwrap();
    let mut ids = Vec::new();
    for who in ["david", "jason"] {
        ids.push(
            app.db
                .write(move |tx| {
                    CalendarEvent::create(
                        tx,
                        NewCalendarEvent {
                            room_id: identify("designers"),
                            organizer_id: identify(who),
                            title: who.into(),
                            time_zone: "UTC".into(),
                            starts_at: Some(tx.now().since(jiff::SignedDuration::from_mins(10))),
                            ..Default::default()
                        },
                    )
                })
                .await
                .unwrap()
                .id,
        );
    }
    app.db
        .write(|tx| {
            tx.conn().execute(
                "DELETE FROM memberships WHERE room_id=? AND user_id=?",
                rusqlite::params![identify("designers"), identify("david")],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    periodic::event_reminders(&app.db).await.unwrap();
    let [bad, good]: [i64; 2] = ids.try_into().unwrap();
    assert!(
        app.db
            .read(move |c| CalendarEvent::find(c, bad))
            .await
            .unwrap()
            .reminded_at
            .is_none()
    );
    assert!(
        app.db
            .read(move |c| CalendarEvent::find(c, good))
            .await
            .unwrap()
            .reminded_at
            .is_some()
    );
    assert!(
        jobs(&app)
            .iter()
            .any(|j| j.class == "Event::ReminderPushJob"
                && j.arguments == serde_json::json!({"event_id":good}))
    );
}

/// Exercise the real app EventSink and background_jobs, including SQLite job-insert failure.
#[tokio::test]
async fn ws14e_event_jobs_commit_with_create_rsvp_and_reminder() {
    use campfire_db::fixtures::{Options, identify, load, reference_dir};
    use campfire_db::{CalendarEvent, NewCalendarEvent};
    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    booted.jobs.shutdown(Duration::from_secs(5)).await;
    app.db
        .write(|tx| {
            load(
                tx.conn(),
                &reference_dir(),
                &Options {
                    now: tx.now(),
                    bcrypt_cost: 4,
                },
            )
        })
        .await
        .unwrap();
    let event = app
        .db
        .write(|tx| {
            CalendarEvent::create(
                tx,
                NewCalendarEvent {
                    room_id: identify("designers"),
                    organizer_id: identify("david"),
                    title: "Standup".into(),
                    time_zone: "UTC".into(),
                    starts_at: Some(tx.now().since(jiff::SignedDuration::from_mins(10))),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let args = serde_json::json!({"event_id":event.id,"user_id":identify("david")});
    assert!(
        jobs(&app)
            .iter()
            .any(|j| j.class == "Calendar::SyncEntryJob" && j.arguments == args)
    );
    app.db
        .write(move |tx| CalendarEvent::respond(tx, event.id, identify("jason"), "going", false))
        .await
        .unwrap();
    app.db.write(|tx|{ tx.conn().execute_batch("CREATE TRIGGER reject_ws14e_reminder BEFORE INSERT ON background_jobs WHEN NEW.job_class='Event::ReminderPushJob' BEGIN SELECT RAISE(ABORT,'queue unavailable'); END")?;Ok(()) }).await.unwrap();
    let before = app
        .db
        .read(move |c| {
            campfire_db::ActivityItem::find_by_user_and_source(
                c,
                identify("jason"),
                "Event",
                event.id,
            )
        })
        .await
        .unwrap();
    assert!(
        app.db
            .write(move |tx| CalendarEvent::dispatch_reminder(tx, event.id, tx.now()))
            .await
            .is_err()
    );
    let after = app
        .db
        .read(move |c| {
            campfire_db::ActivityItem::find_by_user_and_source(
                c,
                identify("jason"),
                "Event",
                event.id,
            )
        })
        .await
        .unwrap();
    assert_eq!(after, before);
    assert!(
        app.db
            .read(move |c| CalendarEvent::find(c, event.id))
            .await
            .unwrap()
            .reminded_at
            .is_none()
    );
    app.db
        .write(|tx| {
            tx.conn()
                .execute_batch("DROP TRIGGER reject_ws14e_reminder")?;
            Ok(())
        })
        .await
        .unwrap();
    periodic::event_reminders(&app.db).await.unwrap();
    assert!(
        jobs(&app)
            .iter()
            .any(|j| j.class == "Event::ReminderPushJob"
                && j.arguments == serde_json::json!({"event_id":event.id}))
    );
    // Rejecting Calendar sync aborts all event materialization and its announcement.
    let before = app
        .db
        .read(|c| Ok(c.query_row::<i64, _, _>("SELECT COUNT(*) FROM events", [], |r| r.get(0))?))
        .await
        .unwrap();
    app.db.write(|tx|{ tx.conn().execute_batch("CREATE TRIGGER reject_ws14e_calendar BEFORE INSERT ON background_jobs WHEN NEW.job_class='Calendar::SyncEntryJob' BEGIN SELECT RAISE(ABORT,'queue unavailable'); END")?;Ok(()) }).await.unwrap();
    assert!(
        app.db
            .write(|tx| CalendarEvent::create(
                tx,
                NewCalendarEvent {
                    room_id: identify("designers"),
                    organizer_id: identify("david"),
                    title: "Rejected series".into(),
                    time_zone: "UTC".into(),
                    starts_at: Some(tx.now()),
                    recurrence_rule: Some("weekly".into()),
                    recurrence_until: Some(
                        tx.now()
                            .jiff()
                            .to_zoned(jiff::tz::TimeZone::UTC)
                            .date()
                            .checked_add(jiff::Span::new().days(14))
                            .unwrap()
                    ),
                    ..Default::default()
                }
            ))
            .await
            .is_err()
    );
    assert_eq!(
        app.db
            .read(
                |c| Ok(c.query_row::<i64, _, _>("SELECT COUNT(*) FROM events", [], |r| r.get(0))?)
            )
            .await
            .unwrap(),
        before
    );
    let before = app
        .db
        .read(move |c| campfire_db::EventAttendance::find_for(c, event.id, identify("jason")))
        .await
        .unwrap();
    assert!(
        app.db
            .write(move |tx| CalendarEvent::respond(
                tx,
                event.id,
                identify("jason"),
                "declined",
                false
            ))
            .await
            .is_err()
    );
    assert_eq!(
        app.db
            .read(move |c| campfire_db::EventAttendance::find_for(c, event.id, identify("jason")))
            .await
            .unwrap(),
        before
    );
}

/// Public WS14g APIs use the real queue; a late follower enqueue failure rolls
/// back every response, and a Meet retry enqueue failure restores the old link.
#[tokio::test]
async fn calendar_api_durable_callbacks_coalesce_and_reject_atomically() {
    use campfire_db::fixtures::{Options, identify, load, reference_dir};
    use campfire_db::{CalendarEvent, EventAttendance, NewCalendarEvent};
    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    booted.jobs.shutdown(Duration::from_secs(5)).await;
    app.db
        .write(|tx| {
            load(
                tx.conn(),
                &reference_dir(),
                &Options {
                    now: tx.now(),
                    bcrypt_cost: 4,
                },
            )
        })
        .await
        .unwrap();
    let head = app
        .db
        .write(|tx| {
            // Keep three weekly occurrences even when the start crosses UTC midnight.
            let starts_at = tx.now().since(jiff::SignedDuration::from_mins(10));
            CalendarEvent::create(
                tx,
                NewCalendarEvent {
                    room_id: identify("designers"),
                    organizer_id: identify("david"),
                    title: "Calendar API".into(),
                    starts_at: Some(starts_at),
                    time_zone: "UTC".into(),
                    recurrence_rule: Some("weekly".into()),
                    recurrence_until: Some(
                        starts_at
                            .jiff()
                            .to_zoned(jiff::tz::TimeZone::UTC)
                            .date()
                            .checked_add(jiff::Span::new().days(14))
                            .unwrap(),
                    ),
                    meet_link_requested: true,
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let hid = head.id;
    let rows = app.db.read(move |c| head.series_events(c)).await.unwrap();
    let ids: Vec<_> = rows.iter().map(|e| e.id).collect();
    app.db
        .write(|tx| {
            tx.conn().execute("DELETE FROM background_jobs", [])?;
            Ok(())
        })
        .await
        .unwrap();
    let user = identify("jason");
    app.db
        .write(move |tx| {
            let first = CalendarEvent::respond(tx, hid, user, "maybe", false)?;
            assert_eq!(first.event_id, hid);
            CalendarEvent::respond(tx, hid, user, "declined", true)?;
            Ok(())
        })
        .await
        .unwrap();
    let queued = jobs(&app);
    assert_eq!(queued.len(), 3);
    assert_eq!(
        queued
            .iter()
            .map(|j| (j.class.as_str(), j.arguments.clone()))
            .collect::<Vec<_>>(),
        ids.iter()
            .map(|eid| (
                "Calendar::SyncEntryJob",
                serde_json::json!({"event_id":eid,"user_id":user})
            ))
            .collect::<Vec<_>>()
    );
    let follower = ids[1];
    app.db.write(move |tx| {
        tx.conn().execute_batch(&format!("CREATE TRIGGER reject_api_follower BEFORE INSERT ON background_jobs WHEN NEW.job_class='Calendar::SyncEntryJob' AND json_extract(NEW.arguments,'$.event_id')={follower} AND json_extract(NEW.arguments,'$.user_id')={user} BEGIN SELECT RAISE(ABORT,'queue unavailable'); END"))?;
        Ok(())
    }).await.unwrap();
    assert!(
        app.db
            .write(move |tx| CalendarEvent::respond(tx, hid, user, "going", false))
            .await
            .is_err()
    );
    assert_eq!(jobs(&app).len(), 3, "no partially inserted callback job");
    for eid in ids {
        let saved = app
            .db
            .read(move |c| EventAttendance::find_for(c, eid, user))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            saved.response, "declined",
            "late follower failure must restore every response"
        );
    }
    app.db
        .write(move |tx| {
            tx.conn()
                .execute_batch("DROP TRIGGER reject_api_follower; DELETE FROM background_jobs;")?;
            CalendarEvent::save_meet_link(
                tx,
                hid,
                Some("https://meet.example.test/fixture".into()),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert!(
        jobs(&app).is_empty(),
        "a populated link schedules no extra sync/provisioning"
    );
    app.db.write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER reject_api_meet BEFORE INSERT ON background_jobs WHEN NEW.job_class='Calendar::MeetLinkJob' BEGIN SELECT RAISE(ABORT,'queue unavailable'); END")?;
        Ok(())
    }).await.unwrap();
    let before = app
        .db
        .read(move |c| CalendarEvent::find(c, hid))
        .await
        .unwrap();
    assert!(
        app.db
            .write(move |tx| CalendarEvent::save_meet_link(tx, hid, None))
            .await
            .is_err()
    );
    assert_eq!(
        app.db
            .read(move |c| CalendarEvent::find(c, hid))
            .await
            .unwrap(),
        before
    );
    assert!(jobs(&app).is_empty());
}

/// A restarted real durable runner consumes WS17's exact ID contract. This adapter
/// records source facts; WS17's own branch supplies Reminder policy and Web Push.
#[tokio::test]
async fn ws14e_reminder_survives_restart_and_rereads_recipients_at_delivery() {
    use campfire_db::fixtures::{Options, identify, load, reference_dir};
    use campfire_db::{CalendarEvent, NewCalendarEvent};
    #[derive(Serialize, Deserialize)]
    #[serde(transparent)]
    struct Consumer(campfire_db::models::calendar_event::reminders::ReminderPushJob);
    impl Job for Consumer {
        const CLASS: &'static str = "Event::ReminderPushJob";
    }
    impl JobKind for Consumer {}
    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    booted.jobs.shutdown(Duration::from_secs(5)).await;
    let event = app
        .db
        .write(|tx| {
            load(
                tx.conn(),
                &reference_dir(),
                &Options {
                    now: tx.now(),
                    bcrypt_cost: 4,
                },
            )?;
            let event = CalendarEvent::create(
                tx,
                NewCalendarEvent {
                    room_id: identify("designers"),
                    organizer_id: identify("david"),
                    title: "Durable reminder".into(),
                    time_zone: "UTC".into(),
                    starts_at: Some(tx.now().since(jiff::SignedDuration::from_mins(10))),
                    ..Default::default()
                },
            )?;
            CalendarEvent::respond(tx, event.id, identify("jason"), "going", false)?;
            Ok(event)
        })
        .await
        .unwrap();
    let event_id = event.id;
    app.db
        .write(move |tx| {
            tx.conn().execute("DELETE FROM background_jobs", [])?;
            CalendarEvent::dispatch_reminder(tx, event_id, tx.now())
        })
        .await
        .unwrap();
    let rows = jobs(&app);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].class, "Event::ReminderPushJob");
    assert_eq!(rows[0].queue, "default");
    assert_eq!(rows[0].arguments, serde_json::json!({"event_id":event_id}));
    app.db
        .write(|tx| {
            tx.conn().execute(
                "DELETE FROM memberships WHERE room_id=? AND user_id=?",
                rusqlite::params![identify("designers"), identify("jason")],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let (delivered, mut receive) = mpsc::unbounded_channel();
    let mut registry = Registry::new();
    registry.register(move |app: App, job: Consumer, _: Execution| {
        let delivered = delivered.clone();
        async move {
            let now = app.db.env().now();
            let source = app
                .db
                .read(move |conn| CalendarEvent::reminder_push_source(conn, job.0.event_id, now))
                .await
                .map_err(discard_missing)?;
            let _ = delivered.send(source);
            Ok(Outcome::Done)
        }
    });
    let config = runner_config(&app.config);
    let queue = JobQueue::new(&registry, &config).unwrap();
    let runner = campfire_jobs::start(app.db.clone(), queue, registry, app.clone(), config);
    let source = tokio::time::timeout(Duration::from_secs(10), receive.recv())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(source.recipient_ids, vec![identify("david")]);
    assert_eq!(
        source.payload.body,
        "Starts in 10 minutes: Durable reminder"
    );
    wait_for(&app, "completed reminder", |rows| rows.is_empty()).await;
    runner.shutdown(Duration::from_secs(5)).await;
}
