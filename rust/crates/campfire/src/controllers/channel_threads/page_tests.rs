use std::sync::Arc;
use axum::http::{Method, StatusCode};
use campfire_db::{ChannelThread, Message, NewChannelThread, NewMessage, ThreadMembership};
use campfire_kit::clock::FrozenClock;
use serde_json::Value;
use crate::controllers::presenters::test_support::*;

fn oracle() -> Value { serde_json::from_str(include_str!("../../../../../vectors/messaging/thread-pages.json")).unwrap() }

async fn fixture() -> (TestApp, i64, Vec<i64>) {
    let app = TestApp::boot_with_test_clock(Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()))).await.unwrap();
    let (parent, threads) = app.db().write(|tx| {
        let parent = Message::create(tx, NewMessage { room_id: ALL_TALK, creator_id: DAVID,
            markdown_source: Some("Page starter".into()), client_message_id: Some("pages-parent".into()), ..Default::default() })?;
        let names = ["Pages <&> thread", "Empty thread", "Stale thread", "Closed thread", "Locked thread", "Work thread"];
        let threads = names.iter().enumerate().map(|(index, name)| ChannelThread::create(tx, NewChannelThread {
            room_id: ALL_TALK, creator_id: JASON, name: Some((*name).into()), parent_message_id: (index == 0).then_some(parent.id),
            auto_archive_after_minutes: (index == 2).then_some(60), work_status: (index == 5).then_some("planned".into()), ..Default::default()
        })).collect::<campfire_db::Result<Vec<_>>>()?;
        let stale = tx.now().since(jiff::SignedDuration::from_hours(-2));
        tx.conn().execute("UPDATE channel_threads SET last_activity_at = ? WHERE id = ?", (stale, threads[2].id))?;
        tx.conn().execute("UPDATE channel_threads SET closed_at = ? WHERE id = ?", (tx.now(), threads[3].id))?;
        tx.conn().execute("UPDATE channel_threads SET closed_at = ?, locked_at = ? WHERE id = ?", (tx.now(), tx.now(), threads[4].id))?;
        tx.conn().execute("UPDATE channel_threads SET work_owner_id = ? WHERE id = ?", (DAVID, threads[5].id))?;
        let ids = (0..45).map(|index| Message::create(tx, NewMessage { room_id: ALL_TALK, creator_id: JASON,
            thread_id: Some(threads[0].id), markdown_source: Some(format!("Page reply {index}")),
            client_message_id: Some(format!("pages-{index}")), ..Default::default() }).map(|message| message.id)).collect::<campfire_db::Result<Vec<_>>>()?;
        assert_eq!(serde_json::json!(ids), oracle()["message_ids"]);
        tx.conn().execute("UPDATE channel_threads SET last_activity_at = ?, closed_at = NULL WHERE id = ?", (stale, threads[2].id))?;
        ThreadMembership::join(tx, threads[0].id, JASON)?;
        tx.conn().execute("INSERT INTO work_thread_events (id, channel_thread_id, actor_id, event_type, from_status, to_status, to_owner_id, to_owner_name, metadata, created_at, updated_at) VALUES (?, ?, ?, 'work_assignment', 'planned', 'planned', ?, 'David', ?, ?, ?)",
            (oracle()["work_event_id"].as_i64().unwrap(), threads[5].id, DAVID, DAVID, r#"{"note":"Assigned <&>"}"#, tx.now(), tx.now()))?;
        Ok((parent.id, threads.into_iter().map(|thread| thread.id).collect::<Vec<_>>()))
    }).await.unwrap();
    assert_eq!(parent, oracle()["parent_id"].as_i64().unwrap());
    assert_eq!(serde_json::json!(threads), oracle()["thread_ids"]);
    (app, parent, threads)
}

#[tokio::test]
async fn thread_state_lists_and_standalone_reads_match_rails_bytes() {
    let (app, parent, threads) = fixture().await;
    let mut browser = app.david();
    for row in oracle()["rows"].as_array().unwrap() {
        let name = row["name"].as_str().unwrap();
        if name == "show_deleted_parent" {
            app.db().write(move |tx| Message::find(tx.conn(), parent)?.destroy(tx)).await.unwrap();
        }
        let mut request = Req::new(Method::GET, row["path"].as_str().unwrap());
        if let Some(frame) = row["frame"].as_str() { request = request.header("turbo-frame", frame); }
        let response = browser.send(request).await;
        assert_eq!(response.status.as_u16(), row["status"].as_u64().unwrap() as u16, "{name}: {}", response.text());
        assert_eq!(response.header("cache-control"), row["cache_control"].as_str(), "{name}");
        if response.status.is_success() {
            assert_eq!(response.header("content-type"), row["content_type"].as_str(), "{name}");
            let expected = row["body"].as_str().unwrap();
            if row["html"] == true {
                if !response.text().contains(expected) { rails_mismatch(&response.text(), expected, name); }
                if row["frame"].is_string() { assert!(!response.text().contains("<title>Smartfire</title>")); }
                else {
                    assert!(response.text().contains("<!DOCTYPE html>"));
                    assert!(response.text().contains(row["title"].as_str().unwrap()), "{name}: page title missing");
                }
            } else if response.text() != expected { rails_mismatch(&response.text(), expected, name); }
        }
    }
    assert!(app.db().read(move |conn| ThreadMembership::find_by_thread_and_user(conn, threads[0], DAVID)).await.unwrap().is_none());
}

#[tokio::test]
async fn stale_listing_reads_do_not_persist_closure() {
    let (app, _, threads) = fixture().await;
    let mut browser = app.david();
    for state in ["active", "closed", "all"] {
        assert_eq!(browser.get(&format!("/rooms/{ALL_TALK}/threads.json?state={state}")).await.status, StatusCode::OK);
    }
    assert!(app.db().read(move |conn| ChannelThread::find(conn, threads[2])).await.unwrap().closed_at.is_none());
}
