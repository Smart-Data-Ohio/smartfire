use std::sync::Arc;
use axum::http::{Method, StatusCode};
use campfire_db::{ChannelThread, Message, NewChannelThread, NewMessage};
use campfire_kit::clock::FrozenClock;
use serde_json::Value;
use crate::controllers::presenters::test_support::*;

fn oracle() -> Value { serde_json::from_str(include_str!("../../../../../vectors/messaging/thread-message-reads.json")).unwrap() }

async fn fixture() -> (TestApp, i64, i64, Vec<i64>) {
    let app = TestApp::boot_with_test_clock(Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()))).await.unwrap();
    let (parent, thread, messages) = app.db().write(|tx| {
        let parent = Message::create(tx, NewMessage { room_id: ALL_TALK, creator_id: DAVID, markdown_source: Some("Thread parent".into()), client_message_id: Some("reads-parent".into()), ..Default::default() })?;
        let thread = ChannelThread::create(tx, NewChannelThread { room_id: ALL_TALK, creator_id: DAVID, parent_message_id: Some(parent.id), name: Some("Read thread".into()), ..Default::default() })?;
        let empty = ChannelThread::create(tx, NewChannelThread { room_id: ALL_TALK, creator_id: DAVID, name: Some("Empty thread".into()), ..Default::default() })?;
        assert_eq!(empty.id, oracle()["empty_id"].as_i64().unwrap());
        let messages = (0..45).map(|index| Message::create(tx, NewMessage { room_id: ALL_TALK, creator_id: DAVID, thread_id: Some(thread.id),
            markdown_source: Some(format!("Thread {index}")), client_message_id: Some(format!("reads-{index}")), ..Default::default() }).map(|message| message.id)).collect::<campfire_db::Result<Vec<_>>>()?;
        Ok((parent.id, thread.id, messages))
    }).await.unwrap();
    assert_eq!(parent, oracle()["parent_id"].as_i64().unwrap());
    assert_eq!(thread, oracle()["thread_id"].as_i64().unwrap());
    assert_eq!(serde_json::json!(messages), oracle()["message_ids"]);
    (app, parent, thread, messages)
}

#[tokio::test]
async fn nested_reads_require_alive_membership_and_both_thread_and_message_scope() {
    let (app, parent, thread, messages) = fixture().await;
    let mut browser = app.david();
    for room in [QUIET_CORNER, DIRECT_KEVIN_BENDER] {
        for suffix in [".json".to_owned(), format!("/{}.json", messages[0]), format!("/{}/actions.json", messages[0])] {
            assert_eq!(browser.get(&format!("/rooms/{room}/threads/{thread}/messages{suffix}")).await.status, StatusCode::NOT_FOUND);
        }
    }
    let base = format!("/rooms/{ALL_TALK}/threads/{thread}/messages");
    for path in [format!("{base}/{parent}.json"), format!("{base}/{parent}/actions.json"), format!("{base}.json?before={parent}"), format!("{base}.json?after={parent}"),
        format!("/rooms/{ALL_TALK}/threads/{}/messages/{}.json", oracle()["empty_id"], messages[0])] {
        assert_eq!(browser.get(&path).await.status, StatusCode::NOT_FOUND, "{path}");
    }
    for deletion in [false, true] {
        let (app, _, thread, _) = fixture().await;
        app.db().write(move |tx| {
            if deletion { tx.conn().execute("UPDATE rooms SET deleted_at = ? WHERE id = ?", (tx.now(), ALL_TALK))?; }
            else { tx.conn().execute("DELETE FROM memberships WHERE room_id = ? AND user_id = ?", (ALL_TALK, DAVID))?; }
            Ok(())
        }).await.unwrap();
        assert_eq!(app.david().get(&format!("/rooms/{ALL_TALK}/threads/{thread}/messages.json")).await.status, StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
async fn thread_reads_deny_bot_keys_and_do_not_join_a_browsing_human() {
    let (app, _, thread, messages) = fixture().await;
    let base = format!("/rooms/{ALL_TALK}/threads/{thread}/messages");
    let mut anonymous = app.anonymous();
    for path in [format!("{base}.json"), format!("{base}/{}.json", messages[0]), format!("{base}/{}/actions.json", messages[0])] {
        assert_eq!(anonymous.send(Req::new(Method::GET, &path).header("authorization", &format!("Bearer {BENDER_KEY}"))).await.status, StatusCode::UNAUTHORIZED);
        assert_eq!(anonymous.get(&format!("{path}?bot_key={BENDER_KEY}")).await.status, StatusCode::FORBIDDEN);
    }
    assert_eq!(app.david().get(&format!("{base}.json")).await.status, StatusCode::OK);
    let memberships: i64 = app.db().read(move |conn| Ok(conn.query_row("SELECT COUNT(*) FROM thread_memberships WHERE thread_id = ? AND user_id = ?", (thread, DAVID), |row| row.get(0))?)).await.unwrap();
    assert_eq!(memberships, 0, "a read must not create a thread membership");
}

#[tokio::test]
async fn nested_pages_show_actions_formats_and_locked_reads_match_rails_bytes() {
    let (app, _, thread, _) = fixture().await;
    let mut browser = app.david();
    for row in oracle()["rows"].as_array().unwrap() {
        if row["name"] == "locked_actions" {
            app.db().write(move |tx| { tx.conn().execute("UPDATE channel_threads SET locked_at = ? WHERE id = ?", (tx.now(), thread))?; Ok(()) }).await.unwrap();
        }
        let mut request = Req::new(Method::GET, row["path"].as_str().unwrap());
        if let Some(frame) = row["frame"].as_str() { request = request.header("turbo-frame", frame); }
        let response = browser.send(request).await;
        let name = row["name"].as_str().unwrap();
        assert_eq!(response.status.as_u16(), row["status"].as_u64().unwrap() as u16, "{name}: {}", response.text());
        assert_eq!(response.header("cache-control"), row["cache_control"].as_str(), "{name}");
        if response.status.is_success() || response.status.is_redirection() {
            assert_eq!(response.content_type(), row["content_type"].as_str(), "{name}");
            assert_eq!(response.location(), row["location"].as_str(), "{name}");
            let expected = row["body"].as_str().unwrap();
            let body = response.text();
            let content = if row["html"] == true {
                let (start, end) = if row["frame"].is_null() { ("<main id=\"main-content\">\n      ", "\n\n      <footer id=\"footer\">") }
                    else { ("<body>\n    ", "\n  </body>") };
                let (_, content) = body.split_once(start).expect("Rails' application/frame layout");
                content.split_once(end).expect("layout closes after index").0
            } else { &body };
            if content != expected { rails_mismatch(content, expected, name); }
        }
    }
}
