use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{CalendarEvent, NewCalendarEvent, Timestamp};

mod cutover;

async fn event(app: &TestApp) -> CalendarEvent {
    app.db()
        .write(|tx| {
            CalendarEvent::create(
                tx,
                NewCalendarEvent {
                    room_id: ALL_TALK,
                    organizer_id: DAVID,
                    title: "Controller planning".into(),
                    starts_at: Timestamp::parse_db("2026-03-03 09:00:00"),
                    time_zone: "UTC".into(),
                    recurrence_rule: Some("weekly".into()),
                    recurrence_until: Some("2026-03-17".parse().unwrap()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn pr174_attendance_parameter_shapes_match_pinned_rails() {
    let app = TestApp::boot().await.expect("pinned default seed");
    let app = app.without_job_runner().await;
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("event-review-regressions.json")).unwrap();
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE sqlite_sequence SET seq=8000000000 WHERE name='events'",
                [],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut david = app.david();
    for case in oracle["attendance"].as_array().unwrap().iter().filter(|case| case["show"] != true) {
        let head = event(&app).await;
        let rows = app.db().read(move |c| head.series_events(c)).await.unwrap();
        let path = format!("/rooms/{ALL_TALK}/events/{}/attendance", rows[1].id);
        let before = event_snapshot(&app).await;
        let mut req = Req::new(
            if case["show"] == true {
                Method::GET
            } else {
                Method::PATCH
            },
            &path,
        );
        if case["frame"] == true {
            req = req.header("turbo-frame", "response_for_message_601");
        }
        req = if let Some(form) = case["form"].as_str() {
            req.header("content-type", "application/x-www-form-urlencoded")
                .body(form.as_bytes().to_vec())
        } else {
            req.header("content-type", "application/json")
                .body(serde_json::to_vec(&case["params"]).unwrap())
        };
        let response = if case["show"] == true {
            david.send(req).await
        } else {
            david.write(req).await
        };
        let label = format!("{} frame={}", case["name"], case["frame"]);
        if response.status.as_u16() as u64 != case["status"].as_u64().unwrap() {
            let probe_rows = rows.clone();
            let current = app
                .db()
                .read(move |c| {
                    probe_rows
                        .iter()
                        .map(|e| e.response_for(c, Some(DAVID)))
                        .collect::<campfire_db::Result<Vec<_>>>()
                })
                .await
                .unwrap();
            eprintln!(
                "{label}: actual status={}, unchanged={}, responses={current:?}",
                response.status,
                before == event_snapshot(&app).await
            );
        }
        assert_eq!(
            response.status.as_u16() as u64,
            case["status"].as_u64().unwrap(),
            "{label}"
        );
        if response.status == StatusCode::INTERNAL_SERVER_ERROR {
            assert_eq!(
                response.text(),
                oracle["production_500"].as_str().unwrap(),
                "{label}"
            );
        }
        let after = event_snapshot(&app).await;
        assert_eq!(
            before == after,
            case["unchanged"].as_bool().unwrap(),
            "{label}: persisted writes"
        );
        let actual = app
            .db()
            .read(move |c| {
                rows.iter()
                    .map(|e| e.response_for(c, Some(DAVID)))
                    .collect::<campfire_db::Result<Vec<_>>>()
            })
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            case["responses"],
            "{label}: series responses"
        );

    }
}
#[tokio::test]
async fn attendance_controller_security_blocks_nonmembers_and_bots() {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    let event = event(&app).await;
    let path = format!(
        "/rooms/{ALL_TALK}/events/{}/attendance?message_id=601",
        event.id
    );
    let mut kevin = app.sign_in(KEVIN).await;
    assert_eq!(kevin.classic_page(&path).await.status, StatusCode::NOT_FOUND);
    let mut bot = app.sign_in(BENDER).await;
    assert_eq!(bot.classic_page(&path).await.status, StatusCode::FORBIDDEN);
    let mut david = app.david();
    let wrong = format!("/rooms/{DIRECT_DAVID_JASON}/events/{}/attendance", event.id);
    assert_eq!(david.classic_page(&wrong).await.status, StatusCode::NOT_FOUND);
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE rooms SET deleted_at=? WHERE id=?",
                rusqlite::params![tx.now(), ALL_TALK],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(david.classic_page(&path).await.status, StatusCode::NOT_FOUND);
}
#[tokio::test]
async fn attendance_controller_renders_and_updates_the_requested_frame() {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    let event = event(&app).await;
    let path = format!("/rooms/{ALL_TALK}/events/{}/attendance", event.id);
    let frame = format!("response_for_message_601_event_{}", event.id);
    let mut david = app.david();
    let saved = david
        .write(
            Req::new(Method::PATCH, &path)
                .header("turbo-frame", &frame)
                .form(&[("response", "maybe"), ("message_id", "601")]),
        )
        .await;
    assert_eq!(saved.status, StatusCode::OK);
    assert_eq!(saved.location(), None);

    let invalid = david
        .write(
            Req::new(Method::PATCH, &path)
                .header("turbo-frame", &frame)
                .form(&[
                    ("attendance[response]", "invalid"),
                    ("attendance[message_id]", "601"),
                ]),
        )
        .await;
    assert_eq!(invalid.status, StatusCode::OK);


    let id = event.id;
    app.db()
        .write(move |tx| CalendarEvent::cancel_with_scope(tx, id, "this_event", Some(DAVID)))
        .await
        .unwrap();
    let closed = david
        .write(
            Req::new(Method::PATCH, &path)
                .header("turbo-frame", &frame)
                .form(&[("response", "going"), ("message_id", "601")]),
        )
        .await;
    assert_eq!(closed.status, StatusCode::OK);


}
#[tokio::test]
async fn attendance_controller_redirects_and_copies_future_responses() {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    let head = event(&app).await;
    let rows = app.db().read(move |c| head.series_events(c)).await.unwrap();
    let path = format!("/rooms/{ALL_TALK}/events/{}/attendance", rows[1].id);
    let mut david = app.david();
    let saved = david
        .write(Req::new(Method::PATCH, &path).form(&[
            ("attendance[response]", "declined"),
            ("attendance[apply_to_future]", "1"),
        ]))
        .await;
    assert_eq!(saved.status, StatusCode::FOUND);
    assert_eq!(
        saved.location(),
        Some(
            format!(
                "http://campfire.test/rooms/{ALL_TALK}/events/{}",
                rows[1].id
            )
            .as_str()
        )
    );
    assert_eq!(
        app.db()
            .read(move |c| rows
                .iter()
                .map(|e| e.response_for(c, Some(DAVID)))
                .collect::<campfire_db::Result<Vec<_>>>())
            .await
            .unwrap(),
        vec![
            Some("going".into()),
            Some("declined".into()),
            Some("declined".into())
        ]
    );
}

#[tokio::test]
async fn event_write_controller_security_and_validation() {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    let head = event(&app).await;
    let path = format!("/rooms/{ALL_TALK}/events/{}", head.id);
    let mut outsider = app.sign_in(KEVIN).await;
    assert_eq!(
        outsider.classic_page(&format!("{path}/edit")).await.status,
        StatusCode::NOT_FOUND
    );
    app.db()
        .write(|tx| {
            tx.conn()
                .execute("UPDATE users SET role=0 WHERE id=?", [JASON])?;
            Ok(())
        })
        .await
        .unwrap();
    let mut jason = app.sign_in(JASON).await;
    assert_eq!(
        jason.classic_page(&format!("{path}/edit")).await.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        jason
            .write(Req::new(Method::PATCH, &path).form(&[("event[title]", "Stolen")]))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    let event_id = head.id;
    assert_eq!(
        app.db()
            .read(move |c| Ok(CalendarEvent::find(c, event_id)?.title))
            .await
            .unwrap(),
        "Controller planning"
    );
    assert_eq!(
        jason
            .write(Req::new(Method::PATCH, &format!("{path}/cancel")))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    app.db()
        .write(|tx| {
            tx.conn()
                .execute("UPDATE users SET role=1 WHERE id=?", [JASON])?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        jason.classic_page(&format!("{path}/edit")).await.status,
        StatusCode::FOUND
    );
    let mut david = app.david();
    assert_eq!(
        david.classic_page(&format!("{path}/edit")).await.status,
        StatusCode::FOUND
    );
    let invalid = david
        .write(
            Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/events"))
                .form(&[("event[title]", ""), ("event[time_zone]", "UTC")]),
        )
        .await;
    assert_eq!(invalid.status, StatusCode::UNPROCESSABLE_ENTITY);


    let missing = david
        .write(Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/events")))
        .await;
    assert_eq!(missing.status, StatusCode::BAD_REQUEST);
}
#[tokio::test]
async fn persisted_series_nil_start_returns_rails_public_500_and_writes_nothing() {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    let app = app.without_job_runner().await;
    let head = event(&app).await;
    let rows = app.db().read(move |c| head.series_events(c)).await.unwrap();
    let expected: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../db/src/models/calendar_event/edges.json"
    )))
    .unwrap();
    let before = event_snapshot(&app).await;
    let mut david = app.david();
    for e in rows {
        for scope in ["this_event", "this_and_following", "all"] {
            let response = david
                .write(
                    Req::new(Method::PATCH, &format!("/rooms/{ALL_TALK}/events/{}", e.id))
                        .form(&[("event[starts_at]", ""), ("update_scope", scope)]),
                )
                .await;
            assert_eq!(
                response.status,
                StatusCode::INTERNAL_SERVER_ERROR,
                "{} {scope}",
                e.id
            );
            assert_eq!(
                response.text(),
                expected["production_500"].as_str().unwrap()
            );
            let after = event_snapshot(&app).await;
            for (i, (actual, expected)) in after.iter().zip(&before).enumerate() {
                assert!(actual == expected, "event snapshot table {i} changed");
            }
        }
    }
}
async fn event_snapshot(app: &TestApp) -> Vec<Vec<String>> {
    app.db().read(|c| {
        ["events","event_attendances","event_references","activity_items","messages","background_jobs"].into_iter().map(|table| {
            let sql=if table=="background_jobs" {"SELECT id,arguments,job_class FROM background_jobs WHERE job_class LIKE 'Calendar::%' OR job_class LIKE 'Event::%' ORDER BY id".to_owned()}else{format!("SELECT * FROM {table} ORDER BY id")};
            let mut s=c.prepare(&sql)?;
            let n=s.column_count();let rows=s.query_map([],|r|Ok((0..n).map(|i|format!("{:?}",r.get_ref(i).unwrap())).collect::<Vec<_>>().join("|")))?.collect::<rusqlite::Result<Vec<_>>>()?;Ok(rows)
        }).collect::<campfire_db::Result<Vec<_>>>()
    }).await.unwrap()
}

#[tokio::test]
async fn event_create_update_cancel_keep_zone_and_calendar_jobs() {
    let Some(app) = TestApp::boot_with_clock_and_env(
        seed_clock(),
        &[("APP_URL", "https://calendar.smartfire.test:8443")],
    )
    .await
    else {
        return;
    };
    let app = app.without_job_runner().await;
    let mut david = app.david();
    let collection = format!("/rooms/{ALL_TALK}/events");
    let created = david
        .write(Req::new(Method::POST, &collection).form(&[
            ("event[title]", "From form"),
            ("event[starts_at]", "2026-10-05T09:00"),
            ("event[ends_at]", "2026-10-05T10:00"),
            ("event[time_zone]", "Eastern Time (US & Canada)"),
            ("event[meet_link_requested]", "1"),
        ]))
        .await;
    assert_eq!(created.status, StatusCode::FOUND);
    let path = created
        .location()
        .unwrap()
        .strip_prefix("http://campfire.test")
        .unwrap()
        .to_string();
    let id = path.rsplit('/').next().unwrap().parse::<i64>().unwrap();
    let e = app
        .db()
        .read(move |conn| CalendarEvent::find(conn, id))
        .await
        .unwrap();
    assert_eq!(
        e.starts_at,
        Timestamp::parse_db("2026-10-05 13:00:00").unwrap()
    );
    let (body,jobs)=app.db().read(move|conn|{
        let body=conn.query_row("SELECT body FROM action_text_rich_texts WHERE record_type='Message' AND record_id IN (SELECT message_id FROM event_references WHERE event_id=?)",[id],|r|r.get::<_,String>(0))?;
        let mut s=conn.prepare("SELECT job_class,arguments FROM background_jobs WHERE job_class LIKE 'Calendar::%' ORDER BY id")?;
        let jobs=s.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;Ok((body,jobs))
    }).await.unwrap();
    assert!(body.contains(&format!(
        "https://calendar.smartfire.test:8443/rooms/{ALL_TALK}/events/{id}"
    )));
    assert!(
        jobs.iter()
            .any(|(class, args)| class == "Calendar::MeetLinkJob"
                && serde_json::from_str::<serde_json::Value>(args).unwrap()
                    == serde_json::json!({"event_id":id}))
    );
    assert!(
        jobs.iter()
            .any(|(class, args)| class == "Calendar::SyncEntryJob"
                && serde_json::from_str::<serde_json::Value>(args).unwrap()
                    == serde_json::json!({"event_id":id,"user_id":DAVID}))
    );
    let updated = david
        .write(Req::new(Method::PATCH, &path).form(&[
            ("event[title]", "Updated"),
            ("event[starts_at]", "2026-10-05T09:00"),
            ("event[ends_at]", "2026-10-05T10:00"),
            ("event[time_zone]", "Hawaii"),
        ]))
        .await;
    assert_eq!(updated.status, StatusCode::FOUND);
    let saved = app
        .db()
        .read(move |conn| CalendarEvent::find(conn, id))
        .await
        .unwrap();
    assert_eq!(saved.starts_at, e.starts_at);
    assert_eq!(saved.time_zone, e.time_zone);
    let cancelled = david
        .write(Req::new(Method::PATCH, &format!("{path}/cancel")))
        .await;
    assert_eq!(cancelled.status, StatusCode::FOUND);
    assert_eq!(
        david.classic_page(&format!("{path}/edit")).await.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        david
            .write(Req::new(Method::PATCH, &format!("{path}/cancel")))
            .await
            .status,
        StatusCode::FOUND
    );
}

#[tokio::test]
async fn event_create_keeps_commit_after_announcement_failure_and_jobs_reject_atomically() {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE rooms SET type='Rooms::Board' WHERE id=?",
                [ALL_TALK],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut david = app.david();
    let collection = format!("/rooms/{ALL_TALK}/events");
    let before = app
        .db()
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM events WHERE room_id=?",
                [ALL_TALK],
                |r| r.get::<_, i64>(0),
            )?)
        })
        .await
        .unwrap();
    let rejected = david
        .write(Req::new(Method::POST, &collection).form(&[
            ("event[title]", "Committed despite board"),
            ("event[starts_at]", "2026-10-05T09:00"),
            ("event[time_zone]", "UTC"),
        ]))
        .await;
    assert_eq!(rejected.status, StatusCode::INTERNAL_SERVER_ERROR);
    let oracle: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../db/src/models/calendar_event/edges.json"
    )))
    .unwrap();
    assert_eq!(rejected.text(), oracle["production_500"].as_str().unwrap());
    let (events, attendances, invitations) = app
        .db()
        .read(|conn| {
            let id = conn.query_row(
                "SELECT id FROM events WHERE title='Committed despite board'",
                [],
                |r| r.get::<_, i64>(0),
            )?;
            Ok((
                conn.query_row(
                    "SELECT COUNT(*) FROM events WHERE room_id=?",
                    [ALL_TALK],
                    |r| r.get::<_, i64>(0),
                )?,
                conn.query_row(
                    "SELECT COUNT(*) FROM event_attendances WHERE event_id=?",
                    [id],
                    |r| r.get::<_, i64>(0),
                )?,
                conn.query_row(
                    "SELECT COUNT(*) FROM activity_items WHERE source_type='Event' AND source_id=?",
                    [id],
                    |r| r.get::<_, i64>(0),
                )?,
            ))
        })
        .await
        .unwrap();
    assert_eq!(events, before + 1);
    assert_eq!(attendances, 1);
    assert_eq!(invitations, 1);
    app.db().write(|tx|{tx.conn().execute_batch("CREATE TRIGGER ws14e_reject_calendar_job BEFORE INSERT ON background_jobs WHEN NEW.job_class='Calendar::SyncEntryJob' BEGIN SELECT RAISE(ABORT,'reject durable calendar job'); END;")?;Ok(())}).await.unwrap();
    let rejected = david
        .write(Req::new(Method::POST, &collection).form(&[
            ("event[title]", "Rolled back durable job"),
            ("event[starts_at]", "2026-10-05T09:00"),
            ("event[time_zone]", "UTC"),
        ]))
        .await;
    assert_eq!(rejected.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        app.db()
            .read(|conn| Ok(conn.query_row(
                "SELECT COUNT(*) FROM events WHERE title='Rolled back durable job'",
                [],
                |r| r.get::<_, i64>(0)
            )?))
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn rescued_not_found_matches_rails_empty_bodies_and_headers() {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("http_errors.json")).unwrap();
    for v in vectors.as_array().unwrap() {
        let mut browser = app.sign_in(v["user_id"].as_i64().unwrap()).await;
        let response = browser
            .send(
                Req::new(Method::GET, v["path"].as_str().unwrap())
                    .header("accept", v["accept"].as_str().unwrap()).header("x-requested-with", "XMLHttpRequest"),
            )
            .await;
        assert_eq!(
            response.status.as_u16(),
            v["status"].as_u64().unwrap() as u16
        );
        assert_eq!(
            response.text(),
            v["body"].as_str().unwrap(),
            "{} {}",
            v["path"],
            v["accept"]
        );
        assert_eq!(
            response
                .headers
                .get("content-type")
                .unwrap()
                .to_str()
                .unwrap(),
            v["content_type"].as_str().unwrap()
        );
    }
}

#[tokio::test]
async fn calendar_api_meet_link_is_not_a_user_parameter() {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    let e = event(&app).await;
    let id = e.id;
    let path = format!("/rooms/{ALL_TALK}/events/{id}");
    let mut david = app.david();
    let response = david
        .write(Req::new(Method::PATCH, &path).form(&[
            ("event[title]", "User edit"),
            ("event[starts_at]", "2026-03-03T09:00"),
            ("event[meet_link]", "https://meet.example.test/injected"),
        ]))
        .await;
    assert_eq!(response.status, StatusCode::FOUND);
    let saved = app
        .db()
        .read(move |c| CalendarEvent::find(c, id))
        .await
        .unwrap();
    assert_eq!(saved.title, "User edit");
    assert_eq!(saved.meet_link, e.meet_link);
    app.db()
        .write(move |tx| {
            CalendarEvent::save_meet_link(tx, id, Some("https://meet.example.test/internal".into()))
        })
        .await
        .unwrap();
    let response = david
        .write(Req::new(Method::PATCH, &path).form(&[
            ("event[starts_at]", "2026-03-03T09:00"),
            ("event[meet_link]", "https://meet.example.test/injected"),
        ]))
        .await;
    assert_eq!(response.status, StatusCode::FOUND);
    assert_eq!(
        app.db()
            .read(move |c| CalendarEvent::find(c, id))
            .await
            .unwrap()
            .meet_link
            .as_deref(),
        Some("https://meet.example.test/internal")
    );
}

// test/controllers/rooms/events_controller_test.rb:401 uses bot-key create and
// a bot session on index, two different authentication paths.
#[tokio::test]
async fn event_http_denies_bot_key_create_and_bot_session_index() {
    let app = TestApp::boot().await.expect("default seed required");
    let before = app
        .db()
        .read(|c| Ok(c.query_row("SELECT count(*) FROM events", [], |r| r.get::<_, i64>(0))?))
        .await
        .unwrap();
    let response = app
        .anonymous()
        .send(
            Req::new(
                Method::POST,
                &format!("/rooms/{ALL_TALK}/events?bot_key={BENDER_KEY}"),
            )
            .form(&[
                ("event[title]", "Bot party"),
                ("event[starts_at]", "2026-09-25T15:30"),
                ("event[time_zone]", "UTC"),
            ]),
        )
        .await;
    assert_eq!(response.status, StatusCode::FORBIDDEN);
    assert_eq!(
        app.db()
            .read(|c| {
                Ok(c.query_row("SELECT count(*) FROM events", [], |r| r.get::<_, i64>(0))?)
            })
            .await
            .unwrap(),
        before
    );
    assert_eq!(
        app.sign_in(BENDER)
            .await
            .get(&format!("/rooms/{ALL_TALK}/events"))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
}
