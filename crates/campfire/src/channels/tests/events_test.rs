//! Committed calendar and digest changes cross the JSON socket without HTML channels.
use crate::controllers::presenters::test_support::{ALL_TALK, DAVID, JASON};
use crate::controllers::spa::api_tests::{Sync, app, serve};
use campfire_api_types as api;
use campfire_db::{CalendarEvent, NewCalendarEvent, Timestamp};

#[path = "board_digest_review_test.rs"]
mod board_digest_review;

#[tokio::test]
async fn event_creation_and_card_updates_reach_connected_members() {
    let Some(a) = app(true).await else { return };
    let a = a.without_job_runner().await;
    let (addr, server) = serve(&a).await;
    let browser = a.sign_in(JASON).await;
    let mut sync = Sync::connect(addr, &browser.cookie_header(), &[format!("room:{ALL_TALK}")]).await;
    sync.welcome().await;
    let event = a.db().write(|tx| CalendarEvent::create(tx, NewCalendarEvent {
        room_id: ALL_TALK, organizer_id: DAVID, title: "Socket planning".into(),
        starts_at: Timestamp::parse_db("2026-10-10 16:10:00"), time_zone: "UTC".into(),
        ..Default::default()
    })).await.unwrap();
    let event_id = event.id;
    let created = sync.until(|event| matches!(&event.payload, api::SyncPayload::MessageCreated(message) if message.cards.iter().any(|card| matches!(card, api::MessageCard::Event(card) if card.event_id == event_id))), |_| false).await;
    let api::SyncPayload::MessageCreated(message) = created.payload else { unreachable!() };
    assert!(message.body_html.contains("Socket planning"));
    let id = event.id;
    a.db().write(move |tx| {
        tx.emit_after_commit(campfire_db::Event::broadcast(&campfire_db::models::calendar_event::CardUpdate { event_id: id }));
        Ok(())
    }).await.unwrap();
    sync.until(|event| matches!(&event.payload, api::SyncPayload::MessageCards(cards) if cards.message_id == message.id), |_| false).await;
    server.abort();
}
