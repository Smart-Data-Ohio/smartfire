//! `message.notified` on the sync socket: the recipient's `user` topic hears about a new message
//! the push policy notifies them of, and its author never does.

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use serde_json::json;

use super::api_tests::{Sync, app, json_body, parse, serve};
use crate::controllers::presenters::test_support::{DAVID, JASON, KEVIN};

fn notified(message_id: i64) -> impl Fn(&api::SyncEvent) -> bool {
    move |event| {
        matches!(&event.payload, api::SyncPayload::MessageNotified(notified) if notified.message_id == message_id)
    }
}

#[tokio::test]
async fn a_group_message_notifies_its_recipient_on_the_user_topic_and_not_its_author() {
    let Some(a) = app(true).await else { panic!("restored default seed required") };
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let mut davids_sync = Sync::connect(addr, &david.cookie_header(), &[]).await;
    davids_sync.welcome().await;
    let mut kevins_sync = Sync::connect(addr, &kevin.cookie_header(), &[]).await;
    kevins_sync.welcome().await;

    let reply = david
        .write(json_body(Method::POST, "/api/v1/directs", &json!({"userIds": [KEVIN, JASON]})))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let room = parse::<api::SidebarRow>(&reply).room.id;
    let post = |text: &str| json!({"clientMessageId": text, "markdownSource": text});
    let path = format!("/api/v1/rooms/{room}/messages");

    let reply = david.write(json_body(Method::POST, &path, &post("Got a minute?"))).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let davids = parse::<api::MessageDTO>(&reply).id;
    let event = kevins_sync.until(notified(davids), |_| false).await;
    assert_eq!(event.topic, "user");
    assert_eq!(
        event.payload,
        api::SyncPayload::MessageNotified(api::MessageNotified {
            message_id: davids,
            room_id: room,
            thread_id: None,
        })
    );

    let reply = kevin.write(json_body(Method::POST, &path, &post("Sure"))).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let kevins = parse::<api::MessageDTO>(&reply).id;
    davids_sync.until(notified(kevins), notified(davids)).await;
    server.abort();
}
