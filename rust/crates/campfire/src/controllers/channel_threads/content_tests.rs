use axum::http::StatusCode;
use campfire_db::{ChannelThread, Message, NewChannelThread, NewMessage, ThreadMembership};
use crate::controllers::presenters::test_support::*;

#[tokio::test]
async fn content_scopes_room_and_anchor_without_joining_and_denies_bots() {
    let app = TestApp::boot().await.unwrap();
    let (thread, foreign) = app.db().write(|tx| {
        let thread = ChannelThread::create(tx, NewChannelThread {room_id: ALL_TALK, creator_id: JASON, name: Some("Pane".into()), ..Default::default()})?;
        let foreign = Message::create(tx, NewMessage {room_id: ALL_TALK, creator_id: JASON, markdown_source: Some("Foreign".into()), ..Default::default()})?;
        Ok((thread.id, foreign.id))
    }).await.unwrap();
    let path = format!("/rooms/{ALL_TALK}/threads/{thread}/content");
    assert_eq!(app.david().get(&format!("{path}?message_id={foreign}")).await.status, StatusCode::NOT_FOUND);
    assert_eq!(app.david().get(&format!("/rooms/{QUIET_CORNER}/threads/{thread}/content")).await.status, StatusCode::NOT_FOUND);
    assert_eq!(app.sign_in(KEVIN).await.get(&path).await.status, StatusCode::NOT_FOUND);
    assert_eq!(app.anonymous().get(&format!("{path}?bot_key={BENDER_KEY}")).await.status, StatusCode::FORBIDDEN);
    assert!(app.db().read(move |conn| ThreadMembership::find_by_thread_and_user(conn, thread, DAVID)).await.unwrap().is_none());
}

use std::sync::Arc;
use campfire_kit::clock::FrozenClock;
use serde_json::Value;
use crate::controllers::presenters::{Presenter, page};
use campfire_db::{Room, Timeline, User};
use campfire_views::helpers::{self as h, request_forgery::{self, AuthenticityTokens, RequestSecrets}};
use askama::Template;

fn oracle() -> Value { serde_json::from_str(include_str!("../../../../../vectors/messaging/thread-content.json")).unwrap() }
struct FixedTokens;
impl AuthenticityTokens for FixedTokens {
    fn global(&self) -> String { "GLOBAL".into() }
    fn for_form(&self, action: &str, method: &str) -> String { format!("{method}:{action}") }
}
async fn fixture() -> (TestApp, i64) {
    let app = TestApp::boot_with_test_clock(Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()))).await.unwrap();
    let id = app.db().write(|tx| {
        let parent = Message::create(tx, NewMessage {room_id: ALL_TALK, creator_id: JASON, markdown_source: Some("Pane parent".into()), client_message_id: Some("pane-parent".into()), ..Default::default()})?;
        let thread = ChannelThread::create(tx, NewChannelThread {room_id: ALL_TALK, creator_id: JASON, parent_message_id: Some(parent.id), name: Some("Pane <&> thread".into()), ..Default::default()})?;
        let other = ChannelThread::create(tx, NewChannelThread {room_id: ALL_TALK, creator_id: JASON, name: Some("Other pane".into()), ..Default::default()})?;
        let foreign = Message::create(tx, NewMessage {room_id: ALL_TALK, creator_id: JASON, thread_id: Some(other.id), markdown_source: Some("Foreign reply".into()), client_message_id: Some("pane-foreign".into()), ..Default::default()})?;
        let ids = (0..45).map(|i| Message::create(tx, NewMessage {room_id: ALL_TALK, creator_id: JASON, thread_id: Some(thread.id), markdown_source: Some(format!("Pane reply {i}")), client_message_id: Some(format!("pane-{i}")), ..Default::default()}).map(|m| m.id)).collect::<campfire_db::Result<Vec<_>>>()?;
        tx.conn().execute("INSERT INTO agent_steps (channel_thread_id, agent_id, name, status, position, input_summary, output_summary, duration_ms, created_at, updated_at) VALUES (?, 0, 'Thread action', 'done', 0, 'Input <&>', 'Output <&>', 1500, ?, ?)", (thread.id, tx.now(), tx.now()))?;
        assert_eq!(parent.id, oracle()["parent_id"]);
        assert_eq!(thread.id, oracle()["thread_id"]);
        assert_eq!(foreign.id, oracle()["foreign_id"]);
        assert_eq!(serde_json::json!(ids), oracle()["message_ids"]);
        Ok(thread.id)
    }).await.unwrap();
    (app, id)
}

#[tokio::test]
async fn conversation_and_room_composer_match_fixed_secret_rails_bytes() {
    let (app, id) = fixture().await;
    for row in oracle()["rows"].as_array().unwrap().iter().filter(|row| row["status"] == 200) {
        let runtime = app.booted.app.clone();
        let row_owned = row.clone();
        let html = app.db().read(move |conn| {
            let p = Presenter::new(conn, &runtime, None);
            let room = Room::find(conn, ALL_TALK)?;
            let thread = ChannelThread::find(conn, id)?;
            let viewer = User::find(conn, DAVID)?;
            let anchor = row_owned["anchor"].as_i64();
            let records = if let Some(anchor) = anchor { Message::page_around(conn, Timeline::Thread(id), &Message::find(conn, anchor)?)? } else {Message::last_page(conn, Timeline::Thread(id))?};
            assert_eq!(serde_json::json!(records.iter().map(|message| message.id).collect::<Vec<_>>()), row_owned["selected_ids"]);
            let messages = p.messages(&records)?;
            let user = p.user_view(DAVID)?;
            let steps = p.thread_steps(id)?;
            let composer = p.composer_facts(&room, &viewer, Some(&thread), p.composer_drive_flow(&viewer, false)?)?;
            assert_eq!(serde_json::json!(composer.slash_commands), oracle()["slash_names"]);
            // The schedule child is an explicit owner input, produced by the actual Rails child.
            let scheduled_control = h::raw(row_owned["composer_button"].as_str().unwrap());
            page::render_detached_at(&runtime, None, "http://campfire.test", |ctx| request_forgery::rendering_with(RequestSecrets {tokens: Box::new(FixedTokens), csp_nonce: None}, || {
                campfire_views::channel_threads::Conversation {ctx, thread_id: id, room_updated_at: room.updated_at.jiff(), anchor,
                    messages: &messages, user: &user, steps: &steps, composer: &composer, scheduled_control: &scheduled_control}.render()
            })).map_err(|e| campfire_db::Error::Other(e.to_string()))
        }).await.unwrap();
        let expected = row["body"].as_str().unwrap();
        if html != expected {rails_mismatch(&html, expected, row["name"].as_str().unwrap());}
    }
    let runtime = app.booted.app.clone();
    let (html,footer,pending) = app.db().read(move |conn| {
        let p = Presenter::new(conn, &runtime, None);
        let composer = p.composer_facts(&Room::find(conn, ALL_TALK)?, &User::find(conn, DAVID)?, None, p.composer_drive_flow(&User::find(conn, DAVID)?, false)?)?;
        let scheduled_control = h::raw(oracle()["room_schedule"].as_str().unwrap());
        page::render_detached_at(&runtime, None, "http://campfire.test", |ctx| request_forgery::rendering_with(RequestSecrets {tokens: Box::new(FixedTokens), csp_nonce: None}, || {
            Ok::<_,askama::Error>((campfire_views::messages::composer::Composer {ctx, facts: &composer, scheduled_control: &scheduled_control}.render()?,
                campfire_views::messages::composer::FooterComposer {ctx, facts: &composer, scheduled_control: &scheduled_control}.render()?,
                campfire_views::channel_threads::PendingTemplate {ctx,user: &p.user_view(DAVID).unwrap()}.render()?))
        })).map_err(|e| campfire_db::Error::Other(e.to_string()))
    }).await.unwrap();
    if html != oracle()["room_composer"].as_str().unwrap() {rails_mismatch(&html, oracle()["room_composer"].as_str().unwrap(), "room composer");}
    if footer != oracle()["room_footer"].as_str().unwrap() {rails_mismatch(&footer,oracle()["room_footer"].as_str().unwrap(),"room footer");}
    if pending != oracle()["pending_template"].as_str().unwrap() {rails_mismatch(&pending,oracle()["pending_template"].as_str().unwrap(),"pending template");}
}

#[tokio::test]
async fn thread_content_selects_rails_windows_and_sets_headers_without_joining() {
    let (app, id) = fixture().await;
    let mut browser = app.david();
    for row in oracle()["rows"].as_array().unwrap() {
        let query = row["params"]["message_id"].as_i64().map(|id| format!("?message_id={id}")).unwrap_or_default();
        let response = browser.get(&format!("{}{query}", row["path"].as_str().unwrap())).await;
        assert_eq!(response.status.as_u16(), row["status"].as_u64().unwrap() as u16, "{}: {}", row["name"], response.text());
        if response.status.is_success() {
            assert_eq!(response.header("cache-control"), row["cache_control"].as_str());
            assert_eq!(response.header("x-thread-content-at-latest"), row["at_latest"].as_str());
            assert_eq!(response.content_type(), row["content_type"].as_str());
            let text = response.text();
            let ids = oracle()["message_ids"].as_array().unwrap().iter().enumerate().filter_map(|(index, id)| text.contains(&format!("id=\"message_pane-{index}\"")).then_some(id.clone())).collect::<Vec<_>>();
            assert_eq!(serde_json::json!(ids), row["selected_ids"]);
            assert!(text.contains("name=\"authenticity_token\""));
            let token = text.split("name=\"authenticity_token\" value=\"").nth(1).unwrap().split('"').next().unwrap();
            assert!(browser.real_authenticity_token().unwrap().is_valid(token, &format!("/rooms/{ALL_TALK}/threads/{id}/messages"), "post"));
            assert!(!text.contains("<!DOCTYPE html>"));
            assert!(text.contains(&format!("id=\"composer_channel_thread_{id}\"")));
            assert_eq!(text.matches("role=\"log\"").count(), 1);
        }
    }
    assert!(app.db().read(move |conn| ThreadMembership::find_by_thread_and_user(conn, id, DAVID)).await.unwrap().is_none());
}
