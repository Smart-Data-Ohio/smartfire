//! The real HTTP -> WS7 publisher -> guard -> socket path versus rendered Rails frames.
use std::sync::Arc;
use axum::http::{Method, StatusCode};
use campfire_db::{ChannelThread, Message, NewChannelThread, NewMessage};
use campfire_kit::clock::FrozenClock;
use serde_json::{Value, json};
use super::{Hub, boot_with_test_clock};
use crate::channels::tests::support::{Client, identifier};
use crate::controllers::presenters::test_support::{ALL_TALK, DAVID, Req, SEED_NOW};

fn oracle() -> Value { serde_json::from_str(include_str!("../../../../../../vectors/messaging/broadcasts.json")).unwrap() }

async fn subscriber(hub: &Hub, thread_id: Option<i64>) -> (Client, Vec<String>) {
    let room = hub.app.db().read(|conn| campfire_db::Room::find(conn, ALL_TALK)).await.unwrap();
    let mut streams = vec![crate::channels::broadcasts::Stream::room_messages(&room).streamables().join(":"), format!("user_{DAVID}_unreads"), format!("user_{DAVID}_unread_threads")];
    if let Some(id) = thread_id { streams.push(crate::channels::broadcasts::Stream::thread_messages(id).streamables().join(":")); }
    let mut client = hub.david().await;
    for stream in &streams {
        let channel = if stream.ends_with(":messages") {
            identifier(json!({"channel": "RoomMessagesChannel", "signed_stream_name": rails_compat::turbo::signed_stream_name(&hub.app.booted.app.secrets, &[stream])}))
        } else { identifier(json!({"channel": if stream.ends_with("unread_threads") { "UnreadThreadsChannel" } else { "UnreadRoomsChannel" }})) };
        client.confirm(&channel).await;
    }
    (client, streams)
}

async fn compare(client: &mut Client, streams: &[String], step: &str) {
    let data = oracle();
    let row = data["steps"].as_array().unwrap().iter().find(|row| row["name"] == step).unwrap();
    for expected in row["frames"].as_array().unwrap().iter().filter(|frame| streams.contains(&frame["stream"].as_str().unwrap().to_owned())) {
        let actual: Value = serde_json::from_str(&client.next_text().await).unwrap();
        if actual["message"] != expected["payload"] {
            crate::controllers::presenters::test_support::rails_mismatch(
                actual["message"].as_str().unwrap_or(&actual.to_string()),
                expected["payload"].as_str().unwrap_or(&expected.to_string()), step);
        }
        if let Some(html) = actual["message"].as_str() {
            assert_eq!(campfire_cable::turbo::session_bound(html), None);
            assert!(!campfire_views::helpers::request_forgery::has_token_slots(html));
        }
    }
    client.assert_silent().await;
}

#[tokio::test]
async fn root_append_replace_and_remove_match_rendered_rails_frames() {
    let clock = Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
    let hub = boot_with_test_clock(clock.clone()).await.unwrap();
    let mut browser = hub.app.david();
    let (mut client, streams) = subscriber(&hub, None).await;
    let response = browser.write(Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/messages.turbo_stream"))
        .header("host", "campfire.test:3443").form(&[("message[markdown_source]", "**Broadcast**"), ("message[client_message_id]", "broadcast-root")])).await;
    assert_eq!(response.status, StatusCode::OK);
    compare(&mut client, &streams, "create").await;
    drop(client);
    let id = oracle()["message_id"].as_i64().unwrap();
    clock.set("2026-03-02T16:00:10Z".parse().unwrap());
    let (mut client, streams) = subscriber(&hub, None).await;
    assert_eq!(browser.write(Req::new(Method::PATCH, &format!("/rooms/{ALL_TALK}/messages/{id}.json"))
        .header("host", "campfire.test:3443").header("content-type", "application/json")
        .body(json!({"message": {"markdown_source": "## Updated", "drive_file_ids": ["abcdefghij"]}}).to_string())).await.status, StatusCode::OK);
    compare(&mut client, &streams, "update").await;
    drop(client);
    let thread_id = hub.app.db().write(move |tx| {
        let reply = Message::create(tx, NewMessage { room_id: ALL_TALK, creator_id: DAVID,
            markdown_source: Some("Reply".into()), reply_to_message_id: Some(id), client_message_id: Some("broadcast-reply".into()), ..Default::default() })?;
        assert_eq!(reply.id, oracle()["reply_id"].as_i64().unwrap());
        let thread = ChannelThread::create(tx, NewChannelThread { room_id: ALL_TALK, creator_id: DAVID,
            parent_message_id: Some(id), name: Some("Broadcast thread".into()), ..Default::default() })?;
        Ok(thread.id)
    }).await.unwrap();
    let (mut client, streams) = subscriber(&hub, Some(thread_id)).await;
    hub.app.db().write(move |tx| {
        let reply = Message::create(tx, NewMessage { room_id: ALL_TALK, thread_id: Some(thread_id), creator_id: DAVID,
            markdown_source: Some("Thread reply".into()), reply_to_message_id: Some(id), client_message_id: Some("broadcast-thread-reply".into()), ..Default::default() })?;
        assert_eq!(reply.id, oracle()["thread_reply_id"].as_i64().unwrap());
        Ok(())
    }).await.unwrap();
    compare(&mut client, &streams, "indicator").await;
    clock.set("2026-03-02T16:00:20Z".parse().unwrap());
    assert_eq!(browser.write(Req::new(Method::DELETE, &format!("/rooms/{ALL_TALK}/messages/{id}.turbo_stream"))
        .header("host", "campfire.test:3443")).await.status, StatusCode::OK);
    compare(&mut client, &streams, "destroy").await;
}

#[tokio::test]
async fn domain_message_descriptions_render_through_the_guard_and_publisher() {
    use campfire_db::broadcasts::{Broadcast, Partial, room_messages, message_dom_id, room_dom_id};
    let clock = Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
    let hub = boot_with_test_clock(clock.clone()).await.unwrap();
    let mut message = hub.app.db().write(|tx| Message::create(tx, NewMessage { room_id: ALL_TALK, creator_id: DAVID,
        markdown_source: Some("**Broadcast**".into()), client_message_id: Some("broadcast-root".into()), ..Default::default() })).await.unwrap();
    clock.set("2026-03-02T16:00:10Z".parse().unwrap());
    message = hub.app.db().write(move |tx| {
        message.edit(tx, campfire_db::MessageChanges { markdown_source: Some("## Updated".into()), drive_file_ids: Some(vec!["abcdefghij".into()]), ..Default::default() })?;
        Message::find(tx.conn(), message.id)
    }).await.unwrap();
    let room = hub.app.db().read(|conn| campfire_db::Room::find(conn, ALL_TALK)).await.unwrap();
    let (mut client, streams) = subscriber(&hub, None).await;
    for (name, description) in [
        ("domain_append", Broadcast::append(room_messages(&room), room_dom_id(&room, Some("messages")), Partial::Message { message_id: message.id })),
        ("domain_replace", Broadcast::replace(room_messages(&room), message_dom_id(&message, None), Partial::MessageReplace { message_id: message.id })),
    ] {
        hub.app.db().write(move |tx| { tx.emit_after_commit(campfire_db::Event::broadcast(&description)); Ok(()) }).await.unwrap();
        // The direct append description does not also describe unread delivery; that is a
        // separate domain event. Compare its rendered Turbo frame alone.
        compare(&mut client, &streams[..1], name).await;
    }
}

#[tokio::test]
async fn all_owned_message_states_publish_the_actual_rails_append_replace_remove_bytes() {
    use crate::controllers::messages::state_tests::{oracle, seed, create_state};
    use campfire_db::broadcasts::{Broadcast, Partial, room_messages, message_dom_id, room_dom_id};
    let hub = boot_with_test_clock(Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()))).await.unwrap();
    hub.app.db().write(seed).await.unwrap();
    let room = hub.app.db().read(|conn| campfire_db::Room::find(conn, ALL_TALK)).await.unwrap();
    for row in oracle()["rows"].as_array().unwrap() {
        let fixture = row.clone();
        // Create before subscribing so only the three explicit publisher operations are read.
        let message = hub.app.db().write(move |tx| create_state(tx, &fixture)).await.unwrap();
        let (mut client, streams) = subscriber(&hub, None).await;
        let descriptions = [
            Broadcast::append(room_messages(&room), room_dom_id(&room, Some("messages")), Partial::Message { message_id: message.id }),
            Broadcast::replace(room_messages(&room), message_dom_id(&message, None), Partial::MessageReplace { message_id: message.id }),
            Broadcast::remove(room_messages(&room), message_dom_id(&message, None)),
        ];
        for (description, expected) in descriptions.into_iter().zip(row["frames"].as_array().unwrap()) {
            assert_eq!(description.stream_name(), expected["stream"]);
            assert_eq!(expected["stream"], streams[0]);
            hub.app.db().write(move |tx| { tx.emit_after_commit(campfire_db::Event::broadcast(&description)); Ok(()) }).await.unwrap();
            let actual: Value = serde_json::from_str(&client.next_text().await).unwrap();
            let html = actual["message"].as_str().unwrap();
            let expected = expected["payload"].as_str().unwrap();
            if html != expected { crate::controllers::presenters::test_support::rails_mismatch(html, expected, row["name"].as_str().unwrap()); }
            assert_eq!(campfire_cable::turbo::session_bound(html), None);
        }
        client.assert_silent().await;
    }
}

#[tokio::test]
async fn thread_writes_publish_rails_bytes_on_the_correct_streams_without_retry_frames() {
    use crate::controllers::channel_thread_messages::write_tests::{oracle, seed, prepare, request};
    let clock = Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
    let hub = boot_with_test_clock(clock.clone()).await.unwrap();
    hub.app.db().write(seed).await.unwrap();
    let thread = oracle()["thread_id"].as_i64().unwrap();
    let mut browser = hub.app.david();
    for row in oracle()["rows"].as_array().unwrap() {
        let name = row["name"].as_str().unwrap().to_owned();
        hub.app.db().write(move |tx| prepare(tx, &name)).await.unwrap();
        clock.set(row["time"].as_str().unwrap().parse().unwrap());
        let (mut client, streams) = subscriber(&hub, Some(thread)).await;
        let response = browser.write(request(row)).await;
        assert_eq!(response.status.as_u16(), row["status"].as_u64().unwrap() as u16, "{}: {}", row["name"], response.text());
        for expected in row["frames"].as_array().unwrap().iter().filter(|frame| streams.iter().any(|stream| frame["stream"] == *stream)) {
            let stream = expected["stream"].as_str().unwrap();
            let expected_channel = if stream.ends_with(":messages") {
                identifier(json!({"channel": "RoomMessagesChannel", "signed_stream_name": rails_compat::turbo::signed_stream_name(&hub.app.booted.app.secrets, &[stream])}))
            } else { identifier(json!({"channel": if stream.ends_with("unread_threads") { "UnreadThreadsChannel" } else { "UnreadRoomsChannel" }})) };
            let actual: Value = serde_json::from_str(&client.next_text().await).unwrap();
            assert_eq!(actual["identifier"], expected_channel, "{}/stream", row["name"]);
            if actual["message"] != expected["payload"] {
                crate::controllers::presenters::test_support::rails_mismatch(
                    actual["message"].as_str().unwrap_or(&actual["message"].to_string()),
                    expected["payload"].as_str().unwrap_or(&expected["payload"].to_string()), row["name"].as_str().unwrap());
            }
            if let Some(html) = actual["message"].as_str() { assert_eq!(campfire_cable::turbo::session_bound(html), None); }
        }
        client.assert_silent().await;
    }
}

#[tokio::test]
async fn thread_lifecycle_publishes_only_rails_delete_indicator_frames() {
    use crate::controllers::channel_threads::write_tests::{oracle, seed, prepare, request};
    let clock = Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
    let hub = boot_with_test_clock(clock.clone()).await.unwrap();
    hub.app.db().write(seed).await.unwrap();
    let mut david = hub.app.david();
    let mut creator = hub.app.sign_in(crate::controllers::presenters::test_support::JASON).await;
    for row in oracle()["rows"].as_array().unwrap() {
        let name = row["name"].as_str().unwrap().to_owned();
        hub.app.db().write(move |tx| prepare(tx, &name)).await.unwrap();
        clock.set(row["time"].as_str().unwrap().parse().unwrap());
        let (mut client, streams) = subscriber(&hub, None).await;
        let browser = if row["viewer"] == 0 { &mut david } else { &mut creator };
        let response = browser.write(request(row)).await;
        assert_eq!(response.status.as_u16(), row["status"].as_u64().unwrap() as u16, "{}: {}", row["name"], response.text());
        for expected in row["frames"].as_array().unwrap().iter().filter(|frame| streams.iter().any(|stream| frame["stream"] == *stream)) {
            let actual: Value = serde_json::from_str(&client.next_text().await).unwrap();
            if actual["message"] != expected["payload"] {
                crate::controllers::presenters::test_support::rails_mismatch(actual["message"].as_str().unwrap(), expected["payload"].as_str().unwrap(), row["name"].as_str().unwrap());
            }
            assert_eq!(campfire_cable::turbo::session_bound(actual["message"].as_str().unwrap()), None);
        }
        client.assert_silent().await;
    }
}
