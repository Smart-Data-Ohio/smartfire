//! test/integration/event_cards_test.rb: foreign/missing links and whole-request query growth.
use super::super::*;
use super::support::*;
use campfire_db::{Message, NewMessage};
async fn message(app: &TestApp, text: String, key: String) -> i64 {
    app.db()
        .write(move |tx| {
            Message::create(
                tx,
                NewMessage {
                    room_id: id("designers"),
                    creator_id: DAVID,
                    markdown_source: Some(text),
                    client_message_id: Some(key),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap()
        .id
}
#[tokio::test]
async fn cutover_cards_foreign_event_link_is_plain_for_members_and_nonmembers() {
    let app = app().await;
    let e = app
        .db()
        .write(|tx| {
            CalendarEvent::create(
                tx,
                NewCalendarEvent {
                    room_id: id("pets"),
                    organizer_id: DAVID,
                    title: "Secret planning".into(),
                    starts_at: Some(tx.now().since(jiff::SignedDuration::from_hours(48))),
                    time_zone: "UTC".into(),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let url = format!("http://example.test/rooms/{}/events/{}", id("pets"), e.id);
    let mid = message(&app, format!("see {url}"), "evt-card-foreign".into()).await;
    assert!(
        app.db()
            .read(move |c| CalendarEvent::for_message_ids(c, &[mid]))
            .await
            .unwrap()
            .get(&mid)
            .is_none_or(Vec::is_empty)
    );
    let room = format!("/rooms/{}", id("designers"));
    let mut david = app.sign_in(DAVID).await;
    let reply = david.get(&room).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(cards(&reply.text()).is_empty());
    assert!(reply.text().contains(&format!("href=\"{url}\"")));
    assert!(!reply.text().contains("Secret planning"));
    let mut kevin = app.sign_in(KEVIN).await;
    let reply = kevin.get(&room).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(cards(&reply.text()).is_empty());
    assert!(!reply.text().contains("Secret planning"));
}
#[tokio::test]
async fn cutover_cards_message_without_event_link_has_no_card() {
    let app = app().await;
    message(&app, "just chatting".into(), "evt-card-none".into()).await;
    let mut david = app.sign_in(DAVID).await;
    let reply = david.get(&format!("/rooms/{}", id("designers"))).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(cards(&reply.text()).is_empty());
}
async fn create_messages(app: &TestApp, count: i64, offset: i64) {
    for n in 0..count {
        let e = app
            .db()
            .write(move |tx| {
                CalendarEvent::create(
                    tx,
                    NewCalendarEvent {
                        room_id: id("designers"),
                        organizer_id: DAVID,
                        title: format!("Query event {}", offset + n),
                        starts_at: Some(tx.now().since(jiff::SignedDuration::from_hours(48))),
                        time_zone: "UTC".into(),
                        ..Default::default()
                    },
                )
            })
            .await
            .unwrap();
        message(
            app,
            format!("see /rooms/{}/events/{}", id("designers"), e.id),
            format!("evt-card-query-{}", offset + n),
        )
        .await;
    }
}
#[tokio::test]
async fn cutover_cards_whole_room_query_count_is_independent_of_event_link_count() {
    let app = app().await;
    create_messages(&app, 2, 0).await;
    let mut david = app.sign_in(DAVID).await;
    let url = format!("/rooms/{}", id("designers"));
    assert_eq!(david.get(&url).await.status, StatusCode::OK);
    let small = query_count(&app, &mut david, &url).await;
    create_messages(&app, 4, 10).await;
    let large = query_count(&app, &mut david, &url).await;
    assert_eq!(
        small, large,
        "room page must have O(1) queries per event-link message"
    );
}
