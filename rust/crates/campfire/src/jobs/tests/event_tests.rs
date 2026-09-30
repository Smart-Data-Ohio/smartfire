use super::*;

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
