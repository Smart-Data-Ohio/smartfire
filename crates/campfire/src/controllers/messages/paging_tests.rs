//! Actual Rails HTTP page/format oracle, including same-timestamp page edges.
use std::sync::Arc;
use axum::http::{Method, StatusCode};
use campfire_db::{Message, NewMessage};
use campfire_kit::clock::FrozenClock;
use serde_json::Value;
use crate::controllers::presenters::test_support::*;

fn oracle() -> Value {
    serde_json::from_str(include_str!("../../../../../vectors/messaging/paging.json")).unwrap()
}

async fn fixture() -> (TestApp, Vec<i64>) {
    let app = TestApp::boot_with_test_clock(Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()))).await.unwrap();
    let ids = app.db().write(|tx| (0..85).map(|index| Message::create(tx, NewMessage {
        room_id: ALL_TALK, creator_id: DAVID, markdown_source: Some(format!("Paging {index}")),
        client_message_id: Some(format!("paging-{index}")), ..Default::default()
    }).map(|message| message.id)).collect::<campfire_db::Result<Vec<_>>>()).await.unwrap();
    assert_eq!(serde_json::json!(ids), oracle()["message_ids"]);
    (app, ids)
}

#[tokio::test]
async fn pages_match_rails_tuple_edges_formats_and_etag_bytes() {
    let (app, _) = fixture().await;
    let mut david = app.david();
    let rows = oracle();
    let etag = rows["pages"][0]["etag"].as_str().unwrap();
    for row in rows["pages"].as_array().unwrap().iter().filter(|row| row["path"].as_str().unwrap().contains(".json")) {
        let mut req = Req::new(Method::GET, row["path"].as_str().unwrap());
        match row["name"].as_str().unwrap() {
            "conditional" => req = req.header("if-none-match", etag),
            "modified_since" => req = req.header("if-modified-since", "Mon, 02 Mar 2026 16:00:00 GMT"),
            "frame" => req = req.header("turbo-frame", "messages"),
            _ => {}
        }
        let reply = david.send(req).await;
        assert_eq!(reply.status.as_u16(), row["status"].as_u64().unwrap() as u16, "{}", row["name"]);
        assert_eq!(reply.header("last-modified"), row["last_modified"].as_str(), "{}", row["name"]);
        if let Some(expected) = row["etag"].as_str() { assert_eq!(reply.header("etag"), Some(expected), "{}", row["name"]); }
        if let Some(expected) = row["cache_control"].as_str() { assert_eq!(reply.header("cache-control"), Some(expected), "{}", row["name"]); }
        if let Some(html) = row["html"].as_str() {
            let actual = reply.text();
            if actual != html {
                rails_mismatch(&actual, html, row["name"].as_str().unwrap());
            }
        }
    }
}

#[tokio::test]
async fn root_formats_and_destroy_side_effects_match_rails() {
    let (app, ids) = fixture().await;
    let mut david = app.david();
    for row in oracle()["formats"].as_array().unwrap().iter().filter(|row| row["action"] == "create" || row["action"] == "destroy" || row["format"] == "json") {
        let format = row["format"].as_str().unwrap();
        let action = row["action"].as_str().unwrap();
        let base = format!("/rooms/{ALL_TALK}/messages");
        let reply = match action {
            "show" => david.get(&format!("{base}/{}.{}", ids[0], format)).await,
            "edit" => david.get(&format!("{base}/{}/edit.{}", ids[0], format)).await,
            "create" => david.write(Req::new(Method::POST, &format!("{base}.{format}")).form(&[("message[markdown_source]", &format!("format {format}")), ("message[client_message_id]", &format!("format-{format}"))])).await,
            "destroy" => {
                let source = format.to_owned();
                let message = app.db().write(move |tx| Message::create(tx, NewMessage { room_id: ALL_TALK, creator_id: DAVID,
                    markdown_source: Some(format!("destroy {source}")), client_message_id: Some(format!("destroy-{source}")), ..Default::default() })).await.unwrap();
                let reply = david.write(Req::new(Method::DELETE, &format!("{base}/{}.{format}", message.id))).await;
                assert!(app.db().read(move |conn| Message::find_by_id(conn, message.id)).await.unwrap().is_none());
                assert!(reply.text().is_empty());
                reply
            }
            _ => unreachable!()
        };
        let expected = match action { "create" => 201, "destroy" => 204, _ => row["status"].as_u64().unwrap() as u16 };
        assert_eq!(reply.status.as_u16(), expected, "{action}.{format}: {}", reply.text());
    }
}

#[tokio::test]
async fn page_anchors_require_alive_membership_and_a_root_message() {
    let (app, ids) = fixture().await;
    let parent = ids[0];
    let child = app.db().write(move |tx| {
        let thread = campfire_db::ChannelThread::create(tx, campfire_db::NewChannelThread { room_id: ALL_TALK, creator_id: DAVID,
            parent_message_id: Some(parent), name: Some("Paging thread".into()), ..Default::default() })?;
        Message::create(tx, NewMessage { room_id: ALL_TALK, creator_id: DAVID, thread_id: Some(thread.id),
            markdown_source: Some("Hidden child".into()), ..Default::default() })
    }).await.unwrap();
    let mut david = app.david();
    for direction in ["before", "after"] {
        assert_eq!(david.get(&format!("/rooms/{ALL_TALK}/messages?{direction}={}", child.id)).await.status, StatusCode::NOT_FOUND);
    }
    let mut kevin = app.sign_in(KEVIN).await;
    let path = format!("/rooms/{ALL_TALK}/messages?before={parent}");
    assert_eq!(kevin.get(&path).await.status, StatusCode::NOT_FOUND);
    app.db().write(|tx| { tx.conn().execute("DELETE FROM memberships WHERE user_id = ? AND room_id = ?", (DAVID, ALL_TALK))?; Ok(()) }).await.unwrap();
    assert_eq!(david.get(&path).await.status, StatusCode::NOT_FOUND);
    let mut jason = app.sign_in(JASON).await;
    app.db().write(|tx| { tx.conn().execute("UPDATE rooms SET deleted_at = ? WHERE id = ?", (tx.now(), ALL_TALK))?; Ok(()) }).await.unwrap();
    assert_eq!(jason.get(&path).await.status, StatusCode::NOT_FOUND);
}
