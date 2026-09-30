use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{CalendarEvent, NewCalendarEvent, Timestamp};

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
    assert_eq!(kevin.get(&path).await.status, StatusCode::NOT_FOUND);
    let mut bot = app.sign_in(BENDER).await;
    assert_eq!(bot.get(&path).await.status, StatusCode::FORBIDDEN);
    let mut david = app.david();
    let wrong = format!("/rooms/{DIRECT_DAVID_JASON}/events/{}/attendance", event.id);
    assert_eq!(david.get(&wrong).await.status, StatusCode::NOT_FOUND);
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
    assert_eq!(david.get(&path).await.status, StatusCode::NOT_FOUND);
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
    let shown = david
        .send(
            Req::new(Method::GET, &format!("{path}?message_id=601")).header("turbo-frame", &frame),
        )
        .await;
    assert_eq!(shown.status, StatusCode::OK);
    assert!(shown.text().contains(&format!("id=\"{frame}\"")));
    assert!(shown.text().contains("Currently: <strong>Going</strong>"));
    assert!(shown.text().contains("Apply to all future occurrences"));
    let saved = david
        .write(
            Req::new(Method::PATCH, &path)
                .header("turbo-frame", &frame)
                .form(&[("response", "maybe"), ("message_id", "601")]),
        )
        .await;
    assert_eq!(saved.status, StatusCode::OK);
    assert_eq!(saved.location(), None);
    assert!(saved.text().contains("Currently: <strong>Maybe</strong>"));
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
    assert!(invalid.text().contains("Choose going, maybe, or declined."));
    assert!(invalid.text().contains("Currently: <strong>Maybe</strong>"));
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
    assert!(
        closed
            .text()
            .contains("This event is no longer open for responses.")
    );
    assert!(
        closed
            .text()
            .contains("Responses are closed because this event was cancelled.")
    );
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
async fn event_cards_refresh_after_an_event_edit_through_the_message_cache() {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    let event = event(&app).await;
    let mut david = app.david();
    let before = david.get(&format!("/rooms/{ALL_TALK}")).await;
    assert_eq!(before.status, StatusCode::OK);
    assert!(before.text().contains("event-card__title"));
    assert!(before.text().contains("response_for_message_"));
    let id = event.id;
    app.db()
        .write(move |tx| {
            CalendarEvent::update(
                tx,
                id,
                campfire_db::models::calendar_event::changes::EventChanges {
                    title: Some("Changed & escaped <title>".into()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let after = david.get(&format!("/rooms/{ALL_TALK}")).await;
    assert_eq!(after.status, StatusCode::OK);
    assert!(after.text().contains("Changed &amp; escaped &lt;title&gt;"));
    assert!(
        !after.text().contains("event-card__current"),
        "The card must lazy-load viewer state"
    );
    let (html,message)=app.db().read(move |conn|{
        let message=conn.query_row("SELECT message_id FROM event_references WHERE event_id=? ORDER BY message_id LIMIT 1",[id],|r|r.get::<_,i64>(0))?;
        Ok((crate::controllers::presenters::events::cards(conn,message)?,campfire_db::Message::find(conn,message)?))
    }).await.unwrap();
    assert!(html.contains(&format!(
        "id=\"event_cards_message_{}\"",
        message.client_message_id
    )));
    assert!(html.contains(&format!("response_for_message_{}_event_{id}", message.id)));
    assert!(campfire_cable::turbo::session_bound(&html).is_none());
}

#[tokio::test]
async fn event_pages_scope_members_bots_and_the_series_index() {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    let head = event(&app).await;
    let path = format!("/rooms/{ALL_TALK}/events/{}", head.id);
    let mut kevin = app.sign_in(KEVIN).await;
    assert_eq!(kevin.get(&path).await.status, StatusCode::NOT_FOUND);
    let mut bot = app.sign_in(BENDER).await;
    assert_eq!(bot.get(&path).await.status, StatusCode::FORBIDDEN);
    let mut david = app.david();
    let shown = david.get(&path).await;
    assert_eq!(shown.status, StatusCode::OK);
    assert!(shown.text().contains("Part of a series: repeats weekly"));
    let indexed = david.get(&format!("/rooms/{ALL_TALK}/events")).await;
    assert_eq!(indexed.status, StatusCode::OK);
    assert_eq!(indexed.text().matches("3 occurrences remaining").count(), 1);
}
