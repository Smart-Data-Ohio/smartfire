//! Membership endpoint/row parity and parent-room authorization, over the real seeded router.
use std::sync::Arc;
use axum::http::{Method, StatusCode};
use campfire_db::{ChannelThread, Message, NewChannelThread, NewMessage, ThreadMembership};
use campfire_kit::clock::FrozenClock;
use serde_json::{Value, json};
use crate::controllers::presenters::test_support::*;

fn oracle() -> Value { serde_json::from_str(include_str!("../../../../../vectors/messaging/thread-memberships.json")).unwrap() }

async fn fixture() -> (TestApp, Arc<FrozenClock>, i64) {
    let clock = Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
    let app = TestApp::boot_with_test_clock(clock.clone()).await.unwrap();
    let id = app.db().write(|tx| {
        let parent = Message::create(tx, NewMessage { room_id: ALL_TALK, creator_id: DAVID,
            markdown_source: Some("Membership parent".into()), client_message_id: Some("membership-parent".into()), ..Default::default() })?;
        assert_eq!(parent.id, oracle()["parent_id"].as_i64().unwrap());
        Ok(ChannelThread::create(tx, NewChannelThread { room_id: ALL_TALK, creator_id: JASON,
            parent_message_id: Some(parent.id), name: Some("Membership thread".into()), ..Default::default() })?.id)
    }).await.unwrap();
    assert_eq!(id, oracle()["thread_id"].as_i64().unwrap());
    (app, clock, id)
}

#[tokio::test]
async fn joins_reads_and_leaves_match_rails_json_redirects_and_rows() {
    let (app, clock, id) = fixture().await;
    let mut david = app.david();
    for row in oracle()["steps"].as_array().unwrap() {
        clock.set(row["time"].as_str().unwrap().parse().unwrap());
        if row["name"] == "read" {
            app.db().write(move |tx| { tx.conn().execute("UPDATE thread_memberships SET unread_at = ? WHERE thread_id = ? AND user_id = ?", (tx.now(), id, DAVID))?; Ok(()) }).await.unwrap();
        }
        let method = match row["verb"].as_str().unwrap() { "delete" => Method::DELETE, "patch" => Method::PATCH, _ => Method::POST };
        let response = david.write(Req::new(method, &format!("/rooms/{ALL_TALK}/threads/{id}/{}.{}", row["action"].as_str().unwrap(), row["format"].as_str().unwrap()))
            .header("content-type", "application/json").body(row["input"].to_string())).await;
        assert_eq!(response.status.as_u16(), row["status"].as_u64().unwrap() as u16, "{}: {}", row["name"], response.text());
        assert_eq!(response.header("cache-control"), row["cache_control"].as_str(), "{}", row["name"]);
        assert_eq!(response.location(), row["location"].as_str(), "{}", row["name"]);
        if let Some(text) = row["json_text"].as_str() { assert_eq!(response.text(), text, "{}", row["name"]); }
        let member = app.db().read(move |conn| Ok(ThreadMembership::find_by_thread_and_user(conn, id, DAVID)?.map(|member| json!({
            "id": member.id, "involvement": member.involvement.name(),
            "joined_at": campfire_presentation::messages::support::json_time(member.joined_at.jiff()),
            "unread_at": member.unread_at.map(|time| campfire_presentation::messages::support::json_time(time.jiff())),
            "updated_at": campfire_presentation::messages::support::json_time(member.updated_at.jiff()),
        })))).await.unwrap();
        assert_eq!(json!(member), row["membership"], "{}", row["name"]);
    }
}

#[tokio::test]
async fn membership_actions_require_alive_parent_membership_and_thread_scope() {
    let (app, _, id) = fixture().await;
    let mut kevin = app.sign_in(KEVIN).await;
    for (verb, action) in [(Method::POST, "join"), (Method::PATCH, "read"), (Method::DELETE, "leave")] {
        assert_eq!(kevin.write(Req::new(verb, &format!("/rooms/{ALL_TALK}/threads/{id}/{action}.json"))).await.status, StatusCode::NOT_FOUND);
    }
    assert!(app.db().read(move |conn| ThreadMembership::find_by_thread_and_user(conn, id, KEVIN)).await.unwrap().is_none());
    let mut david = app.david();
    assert_eq!(david.write(Req::new(Method::POST, &format!("/rooms/{QUIET_CORNER}/threads/{id}/join.json"))).await.status, StatusCode::NOT_FOUND);
    app.db().write(|tx| { tx.conn().execute("DELETE FROM memberships WHERE user_id = ? AND room_id = ?", (DAVID, ALL_TALK))?; Ok(()) }).await.unwrap();
    assert_eq!(david.write(Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/threads/{id}/join.json"))).await.status, StatusCode::NOT_FOUND);
    let mut jason = app.sign_in(JASON).await;
    app.db().write(|tx| { tx.conn().execute("UPDATE rooms SET deleted_at = ? WHERE id = ?", (tx.now(), ALL_TALK))?; Ok(()) }).await.unwrap();
    assert_eq!(jason.write(Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/threads/{id}/join.json"))).await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn thread_membership_writes_reject_bot_keys_and_forged_csrf_without_rows() {
    let (app, _, id) = fixture().await;
    let mut david = app.david();
    let path = format!("/rooms/{ALL_TALK}/threads/{id}/join.json");
    assert_eq!(david.send(Req::new(Method::POST, &path).header("origin", "https://forged.example")).await.status, StatusCode::UNPROCESSABLE_ENTITY);
    let mut bot = app.anonymous();
    assert_eq!(bot.send(Req::new(Method::POST, &format!("{path}?bot_key={BENDER_KEY}"))).await.status, StatusCode::FORBIDDEN);
    assert!(app.db().read(move |conn| ThreadMembership::find_by_thread_and_user(conn, id, DAVID)).await.unwrap().is_none());
    assert!(app.db().read(move |conn| ThreadMembership::find_by_thread_and_user(conn, id, BENDER)).await.unwrap().is_none());
}

#[tokio::test]
async fn thread_pages_require_alive_parent_membership_and_nested_room_scope() {
    let (app, _, id) = fixture().await;
    let mut david = app.david();
    assert_eq!(david.get(&format!("/rooms/{QUIET_CORNER}/threads/{id}.json")).await.status, StatusCode::NOT_FOUND);
    let mut kevin = app.sign_in(KEVIN).await;
    for path in [format!("/rooms/{ALL_TALK}/threads.json"), format!("/rooms/{ALL_TALK}/threads/{id}.json")] {
        assert_eq!(kevin.get(&path).await.status, StatusCode::NOT_FOUND);
    }
    app.db().write(|tx| { tx.conn().execute("DELETE FROM memberships WHERE user_id = ? AND room_id = ?", (DAVID, ALL_TALK))?; Ok(()) }).await.unwrap();
    assert_eq!(david.get(&format!("/rooms/{ALL_TALK}/threads.json")).await.status, StatusCode::NOT_FOUND);
    let mut jason = app.sign_in(JASON).await;
    app.db().write(|tx| { tx.conn().execute("UPDATE rooms SET deleted_at = ? WHERE id = ?", (tx.now(), ALL_TALK))?; Ok(()) }).await.unwrap();
    assert_eq!(jason.get(&format!("/rooms/{ALL_TALK}/threads/{id}.json")).await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn thread_pages_deny_bot_keys_without_joining_the_browser() {
    let (app, _, id) = fixture().await;
    let mut bot = app.anonymous();
    for path in [format!("/rooms/{ALL_TALK}/threads.json"), format!("/rooms/{ALL_TALK}/threads/{id}.json")] {
        assert_eq!(bot.get(&format!("{path}?bot_key={BENDER_KEY}")).await.status, StatusCode::FORBIDDEN);
        assert_eq!(app.david().get(&path).await.status, StatusCode::OK);
    }
    assert!(app.db().read(move |conn| ThreadMembership::find_by_thread_and_user(conn, id, DAVID)).await.unwrap().is_none());
}
