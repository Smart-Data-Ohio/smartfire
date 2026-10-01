//! Full seeded app: committed event writes cross the real socket publisher.
use super::support::{Client, identifier};
use crate::controllers::presenters::test_support::{ALL_TALK, DAVID, JASON, KEVIN, TestApp};
use campfire_db::{
    CalendarEvent, NewCalendarEvent, Timestamp, models::calendar_event::changes::EventChanges,
};
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

async fn listen(app: &TestApp) -> (String, String) {
    let mut listener = None;
    for port in 53200..=53299 {
        match tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await {
            Ok(l) => {
                listener = Some(l);
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => (),
            Err(e) => panic!("bind WS14e socket: {e}"),
        }
    }
    let listener = listener.expect("a free WS14e test port");
    let addr = listener.local_addr().unwrap();
    let router = app.booted.router.clone();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    (format!("ws://{addr}/cable"), format!("http://{addr}"))
}
async fn connect(app: &TestApp, url: &str, origin: &str, user: i64) -> Client {
    let browser = app.sign_in(user).await;
    let mut req = url.into_client_request().unwrap();
    req.headers_mut()
        .insert("cookie", browser.cookie_header().parse().unwrap());
    req.headers_mut().insert("origin", origin.parse().unwrap());
    req.headers_mut().insert(
        "sec-websocket-protocol",
        "actioncable-v1-json".parse().unwrap(),
    );
    let (socket, _) = tokio_tungstenite::connect_async(req).await.unwrap();
    let mut c = Client { socket };
    assert_eq!(c.next_text().await, r#"{"type":"welcome"}"#);
    c
}
fn payload(frame: String) -> Value {
    serde_json::from_str::<Value>(&frame).unwrap()["message"].clone()
}
async fn compare_stage(client: &mut Client, state: &Value) -> Vec<Value> {
    eprintln!("socket stage: {}", state["kind"]);
    let mut actual = Vec::new();
    for f in state["frames"].as_array().unwrap() {
        let p = payload(client.next_text().await);
        assert_eq!(p, f["payload"], "{}", state["kind"]);
        actual.push(p);
    }
    actual
}

async fn creation_app() -> TestApp {
    let clock = std::sync::Arc::new(campfire_kit::FrozenClock::new(
        "2026-03-02T16:00:00Z".parse().unwrap(),
    ));
    // sockets.rb configures Rails' route defaults explicitly; reproduce that
    // input instead of relying on the unrelated mail URL fallback.
    let app = TestApp::boot_with_clock_and_env(clock, &[("APP_URL", "http://example.com")])
        .await
        .expect("pinned default seed");
    // Match Rails' before_create fixture inputs before Message::create reloads the row.
    app.db().write(|tx| {
        tx.conn().execute_batch("UPDATE sqlite_sequence SET seq=8000000000 WHERE name='events'; UPDATE sqlite_sequence SET seq=9000000000 WHERE name='messages'; UPDATE sqlite_sequence SET seq=7000000000 WHERE name='activity_items';
            CREATE TRIGGER ws14e_socket_message_id AFTER INSERT ON messages
            WHEN NEW.markdown_source LIKE 'Scheduled an event: Socket planning%' OR NEW.markdown_source LIKE 'Scheduled an event: Two records%'
            BEGIN UPDATE messages SET client_message_id=CASE WHEN NEW.markdown_source LIKE 'Scheduled an event: Socket planning%' THEN 'ws14e-announcement' ELSE 'ws14e-series' END WHERE id=NEW.id; END;")?;
        Ok(())
    }).await.unwrap();
    app
}

#[tokio::test]
async fn pr174_event_creation_appends_announcement_to_connected_members() {
    let app = creation_app().await;
    let oracle: Value = serde_json::from_str(include_str!("golden/event-sockets.json")).unwrap();
    let (url, origin) = listen(&app).await;
    let room = app
        .db()
        .read(|c| campfire_db::Room::find(c, ALL_TALK))
        .await
        .unwrap();
    let signed = rails_compat::turbo::signed_stream_name(
        &app.booted.app.secrets,
        &[&crate::channels::room_gid(&room).to_param(), "messages"],
    );
    let channel = identifier(json!({"channel":"RoomMessagesChannel", "signed_stream_name":signed}));
    let mut member = connect(&app, &url, &origin, JASON).await;
    member.confirm(&channel).await;
    let mut organizer = connect(&app, &url, &origin, DAVID).await;
    organizer.confirm(&channel).await;
    let mut outsider = connect(&app, &url, &origin, KEVIN).await;
    outsider.reject(&channel).await;
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE users SET updated_at='2026-02-10 12:00:00' WHERE id=?",
                [DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    app.db()
        .write(|tx| {
            CalendarEvent::create(
                tx,
                NewCalendarEvent {
                    room_id: ALL_TALK,
                    organizer_id: DAVID,
                    title: "Socket planning".into(),
                    starts_at: Timestamp::parse_db("2026-03-02 16:10:00"),
                    time_zone: "UTC".into(),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let expected = oracle[0]["frames"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| {
            f["payload"]
                .as_str()
                .is_some_and(|p| p.contains("action=\"append\""))
        })
        .expect("Rails creation append")["payload"]
        .clone();
    assert_eq!(payload(member.next_text().await), expected);
    assert_eq!(payload(organizer.next_text().await), expected);
    assert!(campfire_cable::turbo::session_bound(expected.as_str().unwrap()).is_none());
    outsider.assert_silent().await;
    member.assert_silent().await;
    organizer.assert_silent().await;
}

#[tokio::test]
async fn event_cards_and_activity_match_rails_over_real_sockets() {
    let app = creation_app().await;
    let oracle: Value = serde_json::from_str(include_str!("golden/event-sockets.json")).unwrap();
    let (url, origin) = listen(&app).await;
    let room = app
        .db()
        .read(|c| campfire_db::Room::find(c, ALL_TALK))
        .await
        .unwrap();
    let signed = rails_compat::turbo::signed_stream_name(
        &app.booted.app.secrets,
        &[&crate::channels::room_gid(&room).to_param(), "messages"],
    );
    let room_channel =
        identifier(json!({"channel":"RoomMessagesChannel","signed_stream_name":signed}));
    let activity = identifier(json!({"channel":"ActivityChannel"}));
    let mut member = connect(&app, &url, &origin, JASON).await;
    member.confirm(&room_channel).await;
    member.confirm(&activity).await;
    let mut organizer = connect(&app, &url, &origin, DAVID).await;
    organizer.confirm(&room_channel).await;
    let mut outsider = connect(&app, &url, &origin, KEVIN).await;
    outsider.reject(&room_channel).await;
    outsider.confirm(&activity).await;
    // Signing in touches the seeded user; restore the pinned fixture input before rendering.
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE users SET updated_at='2026-02-10 12:00:00' WHERE id=?",
                [DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    app.db().write(|tx|{tx.conn().execute_batch("UPDATE sqlite_sequence SET seq=8000000000 WHERE name='events'; UPDATE sqlite_sequence SET seq=9000000000 WHERE name='messages'; UPDATE sqlite_sequence SET seq=7000000000 WHERE name='activity_items';")?;Ok(())}).await.unwrap();
    let event = app
        .db()
        .write(|tx| {
            CalendarEvent::create(
                tx,
                NewCalendarEvent {
                    room_id: ALL_TALK,
                    organizer_id: DAVID,
                    title: "Socket planning".into(),
                    starts_at: Timestamp::parse_db("2026-03-02 16:10:00"),
                    time_zone: "UTC".into(),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let id = event.id;
    assert_eq!(id, 8000000001);
    let created = compare_stage(&mut member, &oracle[0]).await;
    assert_eq!(
        payload(organizer.next_text().await),
        *created.iter().find(|p| p.is_string()).unwrap()
    );
    assert!(
        app.db()
            .write(move |tx| {
                CalendarEvent::update(
                    tx,
                    id,
                    EventChanges {
                        title: Some("Rolled back".into()),
                        ..Default::default()
                    },
                )?;
                Err::<(), _>(campfire_db::Error::Other(
                    "roll back after registering card callback".into(),
                ))
            })
            .await
            .is_err()
    );
    member.assert_silent().await;
    organizer.assert_silent().await;
    app.db()
        .write(move |tx| {
            CalendarEvent::update(
                tx,
                id,
                EventChanges {
                    title: Some("Changed <&>".into()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let edited = compare_stage(&mut member, &oracle[1]).await;
    assert_eq!(payload(organizer.next_text().await), edited[0]);
    assert!(campfire_cable::turbo::session_bound(edited[0].as_str().unwrap()).is_none());
    app.db()
        .write(move |tx| CalendarEvent::respond(tx, id, JASON, "maybe", false))
        .await
        .unwrap();
    assert!(compare_stage(&mut member, &oracle[2]).await.is_empty());
    member.assert_silent().await;
    organizer.assert_silent().await;
    let now = app.db().env().now();
    assert!(
        app.db()
            .write(move |tx| CalendarEvent::dispatch_reminder(tx, id, now))
            .await
            .unwrap()
    );
    let reminder = compare_stage(&mut member, &oracle[3]).await;
    assert_eq!(
        payload(organizer.next_text().await),
        *reminder.iter().find(|p| p.is_string()).unwrap()
    );
    app.db()
        .write(move |tx| CalendarEvent::cancel_with_scope(tx, id, "this_event", Some(DAVID)))
        .await
        .unwrap();
    let cancelled = compare_stage(&mut member, &oracle[4]).await;
    assert_eq!(
        payload(organizer.next_text().await),
        *cancelled.iter().find(|p| p.is_string()).unwrap()
    );
    let series = app
        .db()
        .write(|tx| {
            CalendarEvent::create(
                tx,
                NewCalendarEvent {
                    room_id: ALL_TALK,
                    organizer_id: DAVID,
                    title: "Two records".into(),
                    starts_at: Timestamp::parse_db("2026-03-04 09:00:00"),
                    time_zone: "UTC".into(),
                    recurrence_rule: Some("weekly".into()),
                    recurrence_until: Some("2026-03-11".parse().unwrap()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let created = compare_stage(&mut member, &oracle[5]).await;
    assert_eq!(
        payload(organizer.next_text().await),
        *created.iter().find(|p| p.is_string()).unwrap()
    );
    let sid = series.id;
    app.db().write(move|tx| {
        tx.conn().execute("UPDATE messages SET client_message_id='ws14e-series' WHERE id IN (SELECT message_id FROM event_references WHERE event_id=?)",[sid])?;
        let ids=CalendarEvent::find(tx.conn(),sid)?.series_events(tx.conn())?.into_iter().map(|e|format!("http://example.com/rooms/{ALL_TALK}/events/{}",e.id)).collect::<Vec<_>>();
        campfire_db::Message::create(tx,campfire_db::NewMessage{room_id:ALL_TALK,creator_id:DAVID,client_message_id:Some("ws14e-shared".into()),markdown_source:Some(ids.join("\n")),..Default::default()})?;Ok(())
    }).await.unwrap();
    app.db()
        .write(move |tx| {
            CalendarEvent::update_with_scope(
                tx,
                sid,
                EventChanges {
                    title: Some("Shared changed".into()),
                    ..Default::default()
                },
                "this_and_following",
                Some(DAVID),
            )
        })
        .await
        .unwrap();
    let two_records = compare_stage(&mut member, &oracle[6]).await;
    for expected in two_records {
        assert_eq!(payload(organizer.next_text().await), expected);
    }
    app.db()
        .write(move |tx| {
            CalendarEvent::update(
                tx,
                sid,
                EventChanges {
                    description: Some(Some("First".into())),
                    ..Default::default()
                },
            )?;
            CalendarEvent::update(
                tx,
                sid,
                EventChanges {
                    description: Some(Some("Second".into())),
                    ..Default::default()
                },
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let coalesced = compare_stage(&mut member, &oracle[7]).await;
    for expected in coalesced {
        assert_eq!(payload(organizer.next_text().await), expected);
    }
    app.db()
        .write(move |tx| {
            let e = CalendarEvent::update(
                tx,
                sid,
                EventChanges {
                    title: Some("Never delivered".into()),
                    ..Default::default()
                },
            )?;
            e.destroy(tx)
        })
        .await
        .unwrap();
    assert!(compare_stage(&mut member, &oracle[8]).await.is_empty());
    app.db()
        .write(move |tx| {
            CalendarEvent::save_meet_link(tx, id, Some("https://meet.example.test/socket".into()))
        })
        .await
        .unwrap();
    let meet_link = compare_stage(&mut member, &oracle[9]).await;
    assert_eq!(payload(organizer.next_text().await), meet_link[0]);
    assert!(campfire_cable::turbo::session_bound(meet_link[0].as_str().unwrap()).is_none());
    outsider.assert_silent().await;
    member.assert_silent().await;
    organizer.assert_silent().await;
}
