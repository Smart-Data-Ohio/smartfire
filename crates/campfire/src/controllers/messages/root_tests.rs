//! Request/row differentials from real Rails root-message requests.
use std::sync::Arc;

use axum::http::{Method, StatusCode};
use askama::Template;
use campfire_db::{Boost, ChannelThread, Message, MessagePin, NewChannelThread, NewMessage, NewSavedItem, SavedItem};
use campfire_kit::clock::FrozenClock;
use serde_json::{Value, json};

use crate::controllers::presenters::test_support::*;

fn oracle() -> Value {
    serde_json::from_str(include_str!("../../../../../vectors/messaging/root.json")).unwrap()
}

async fn fixture() -> (TestApp, Arc<FrozenClock>, Vec<i64>) {
    let clock = Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
    let app = TestApp::boot_with_test_clock(clock.clone()).await.expect("WS8bm requires default seed");
    let data = oracle();
    let inputs = data["inputs"].as_array().unwrap().clone();
    let ids = app.db().write(move |tx| {
        let mut messages = Vec::new();
        for input in inputs {
            messages.push(Message::create(tx, NewMessage {
                room_id: ALL_TALK, creator_id: DAVID,
                body: input["body"].as_str().map(str::to_owned),
                markdown_source: input["markdown_source"].as_str().map(str::to_owned),
                client_message_id: input["client_message_id"].as_str().map(str::to_owned),
                system_note: input["system_note"].as_bool().unwrap_or(false),
                reply_to_message_id: input["reply_to_message_id"].as_i64(),
                reply_notify_author: input["reply_notify_author"].as_bool(),
                ..Default::default()
            })?);
        }
        let thread = ChannelThread::create(tx, NewChannelThread {
            room_id: ALL_TALK, creator_id: JASON, parent_message_id: Some(messages[0].id),
            name: Some("Root thread".into()), ..Default::default()
        })?;
        assert_eq!(thread.id, data["thread_id"].as_i64().unwrap());
        MessagePin::create(tx, &messages[0], ALL_TALK, DAVID)?;
        let saved = SavedItem::create(tx, NewSavedItem { user_id: DAVID, message_id: messages[0].id, ..Default::default() })?;
        assert_eq!(saved.id, data["saved_id"].as_i64().unwrap());
        for booster in [DAVID, JASON, DAVID] { Boost::create(tx, messages[0].id, booster, "👍")?; }
        Ok(messages.iter().map(|message| message.id).collect::<Vec<_>>())
    }).await.unwrap();
    assert_eq!(json!(ids), oracle()["message_ids"]);
    (app, clock, ids)
}

#[tokio::test]
async fn actions_match_rails_for_two_members_without_shared_viewer_state() {
    let (app, _, ids) = fixture().await;
    let mut david = app.david();
    let mut jason = app.sign_in(JASON).await;
    for row in oracle()["actions"].as_array().unwrap() {
        let browser = if row["viewer"] == 0 { &mut david } else { &mut jason };
        let id = ids[row["index"].as_u64().unwrap() as usize];
        let reply = browser.get(&format!("/rooms/{ALL_TALK}/messages/{id}/actions")).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        assert_eq!(reply.header("cache-control"), row["cache_control"].as_str());
        assert_eq!(reply.json(), row["json"]);
        assert_eq!(reply.text(), row["json_text"].as_str().unwrap());
    }
}

#[tokio::test]
async fn actions_require_alive_membership_and_exclude_thread_messages() {
    let (app, _, ids) = fixture().await;
    let path = format!("/rooms/{ALL_TALK}/messages/{}/actions", ids[0]);
    let mut david = app.david();
    assert_eq!(david.get(&path).await.status, StatusCode::OK);
    let parent = ids[0];
    let thread_message = app.db().write(move |tx| {
        let thread = ChannelThread::find_by_parent_message(tx.conn(), parent)?.unwrap();
        Message::create(tx, NewMessage { room_id: ALL_TALK, creator_id: DAVID, thread_id: Some(thread.id),
            markdown_source: Some("Private thread scope".into()), ..Default::default() })
    }).await.unwrap();
    assert_eq!(david.get(&format!("/rooms/{ALL_TALK}/messages/{}/actions", thread_message.id)).await.status, StatusCode::NOT_FOUND);
    let mut kevin = app.sign_in(KEVIN).await;
    assert_eq!(kevin.get(&path).await.status, StatusCode::NOT_FOUND);
    app.db().write(|tx| {
        tx.conn().execute("DELETE FROM memberships WHERE user_id = ? AND room_id = ?", (DAVID, ALL_TALK))?;
        Ok(())
    }).await.unwrap();
    assert_eq!(david.get(&path).await.status, StatusCode::NOT_FOUND);
    let mut jason = app.sign_in(JASON).await;
    app.db().write(|tx| {
        tx.conn().execute("UPDATE rooms SET deleted_at = ? WHERE id = ?", (tx.now(), ALL_TALK))?;
        Ok(())
    }).await.unwrap();
    assert_eq!(jason.get(&path).await.status, StatusCode::NOT_FOUND);
}

async fn edit_html(app: &TestApp, id: i64) -> String {
    use crate::controllers::presenters::{Presenter, page};
    let runtime = app.booted.app.clone();
    app.db().read(move |conn| {
        let presenter = Presenter::new(conn, &runtime, None);
        let message = Message::find(conn, id)?;
        let edit = campfire_views::messages::EditView {
            editable_body_html: presenter.editable_markdown_source(&message)?, message: presenter.message(&message)?,
        };
        let account = campfire_db::Account::first(conn)?;
        page::render_detached_at(&runtime, account.as_ref(), "http://campfire.test", |ctx| campfire_views::messages::Edit { ctx, edit: &edit }.render())
            .map_err(|error| campfire_db::Error::Other(error.to_string()))
    }).await.unwrap()
}

#[tokio::test]
async fn standalone_message_wrapper_matches_rails_bytes() {
    use crate::controllers::presenters::{Presenter, page};
    let (app, _, ids) = fixture().await;
    for row in oracle()["shows"].as_array().unwrap() {
        let id = ids[row["index"].as_u64().unwrap() as usize];
        let runtime = app.booted.app.clone();
        let html = app.db().read(move |conn| {
            let message = Presenter::new(conn, &runtime, None).message(&Message::find(conn, id)?)?;
            let account = campfire_db::Account::first(conn)?;
            page::render_detached_at(&runtime, account.as_ref(), "http://campfire.test", |ctx| {
                campfire_views::messages::Show { ctx, message: &message }.render()
                    .map_err(|error| campfire_db::Error::Other(error.to_string()))
            })
        }).await.unwrap();
        let expected = row["html"].as_str().unwrap();
        if html != expected {
            rails_mismatch(&html, expected, "standalone message");
        }
        assert_eq!(campfire_cable::turbo::session_bound(&html), None);
    }
}

#[tokio::test]
async fn edit_forms_and_actions_menu_match_rails_bytes() {
    use crate::controllers::presenters::{Presenter, page};
    let (app, _, ids) = fixture().await;
    for row in oracle()["edits"].as_array().unwrap() {
        assert_eq!(edit_html(&app, ids[row["index"].as_u64().unwrap() as usize]).await, row["html"].as_str().unwrap());
        let id = ids[row["index"].as_u64().unwrap() as usize];
        let runtime = app.booted.app.clone();
        let body = app.db().read(move |conn| Presenter::new(conn, &runtime, None).rendered_body_html(&Message::find(conn, id)?)).await.unwrap();
        assert_eq!(body, row["rendered_body"].as_str().unwrap());
    }
    let runtime = app.booted.app.clone();
    let html = app.db().read(move |conn| {
        let account = campfire_db::Account::first(conn)?;
        page::render_detached_at(&runtime, account.as_ref(), "http://campfire.test", |ctx| campfire_views::messages::ActionsMenu { ctx }.render())
            .map_err(|error| campfire_db::Error::Other(error.to_string()))
    }).await.unwrap();
    assert_eq!(html, oracle()["actions_menu"].as_str().unwrap());
}

#[tokio::test]
async fn edit_http_uses_markdown_source_and_update_requires_csrf() {
    let (app, _, ids) = fixture().await;
    let mut david = app.david();
    let path = format!("/rooms/{ALL_TALK}/messages/{}", ids[0]);
    let reply = david.get(&format!("{path}/edit")).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(reply.text().contains("id=\"message_markdown_source\">\n**Before**\n\n@[David]</textarea>"));
    assert!(reply.text().contains("name=\"authenticity_token\""));
    let id = ids[0];
    let original = app.db().read(move |conn| Message::find(conn, id)).await.unwrap();
    let reply = david.send(Req::new(Method::PATCH, &format!("{path}.json"))
        .header("origin", "https://other.example").form(&[("message[markdown_source]", "forged")])).await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(app.db().read(move |conn| Message::find(conn, id)).await.unwrap(), original);
}

#[tokio::test]
async fn valid_bot_keys_are_forbidden_on_root_actions_and_updates() {
    let (app, _, ids) = fixture().await;
    let mut bot = app.anonymous();
    let path = format!("/rooms/{ALL_TALK}/messages/{}", ids[0]);
    assert_eq!(bot.get(&format!("{path}/actions?bot_key={BENDER_KEY}")).await.status, StatusCode::FORBIDDEN);
    let message = app.db().write(|tx| Message::create(tx, NewMessage { room_id: ALL_TALK, creator_id: BENDER,
        markdown_source: Some("Bot-owned message".into()), ..Default::default() })).await.unwrap();
    assert_eq!(bot.send(Req::new(Method::PATCH, &format!("/rooms/{ALL_TALK}/messages/{}.json?bot_key={BENDER_KEY}", message.id))
        .form(&[("message[markdown_source]", "bot edit")])).await.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn updates_match_rails_json_and_saved_rows_including_legacy_conversion() {
    let (app, clock, ids) = fixture().await;
    let mut david = app.david();
    for step in oracle()["updates"].as_array().unwrap() {
        clock.set(step["time"].as_str().unwrap().parse().unwrap());
        let id = ids[step["index"].as_u64().unwrap() as usize];
        let reply = david.write(Req::new(Method::PATCH, &format!("/rooms/{ALL_TALK}/messages/{id}.json"))
            .header("content-type", "application/json").body(json!({"message": step["input"]}).to_string())).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        assert_eq!(reply.json(), step["json"]);
        assert_eq!(reply.text(), step["json_text"].as_str().unwrap());
        let runtime = app.booted.app.clone();
        let row = app.db().read(move |conn| {
            let message = Message::find(conn, id)?;
            Ok(json!({
                "markdown_source": message.markdown_source,
                "body": message.body_html(conn)?.unwrap_or_default(),
                "plain_text": message.plain_text_body(conn, &*runtime.db.env().rich_text)?,
                "client_message_id": message.client_message_id,
                "edited_at": message.edited_at.map(|time| campfire_views::messages::support::json_time(time.jiff())),
                "reply_to_message_id": message.reply_to_message_id,
                "reply_notify_author": message.reply_notify_author,
                "drive_file_ids": message.drive_file_ids(conn)?
            }))
        }).await.unwrap();
        assert_eq!(row, step["row"]);
        assert_eq!(edit_html(&app, id).await, step["edit_html"].as_str().unwrap());
    }
}

#[tokio::test]
async fn invalid_updates_match_rails_and_roll_back_rows() {
    let (app, _, ids) = fixture().await;
    let id = ids[0];
    let original = app.db().read(move |conn| Message::find(conn, id)).await.unwrap();
    let mut david = app.david();
    for row in oracle()["invalid_updates"].as_array().unwrap() {
        let reply = david.write(Req::new(Method::PATCH, &format!("/rooms/{ALL_TALK}/messages/{id}.json"))
            .header("content-type", "application/json").body(json!({"message": row["input"]}).to_string())).await;
        assert_eq!(reply.status.as_u16(), row["status"].as_u64().unwrap() as u16);
        assert_eq!(reply.json(), row["json"]);
        assert_eq!(reply.text(), row["json_text"].as_str().unwrap());
        assert_eq!(app.db().read(move |conn| Message::find(conn, id)).await.unwrap(), original);
    }
}

#[tokio::test]
async fn update_rolls_back_text_and_drive_changes_when_atomic_job_insert_fails() {
    let (app, _, ids) = fixture().await;
    let id = ids[0];
    let original = app.db().read(move |conn| Message::find(conn, id)).await.unwrap();
    app.db().write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER ws8bm_reject_update_job BEFORE INSERT ON background_jobs BEGIN SELECT RAISE(ABORT, 'WS8bm update enqueue rejection'); END;")?;
        Ok(())
    }).await.unwrap();
    let mut david = app.david();
    let reply = david.write(Req::new(Method::PATCH, &format!("/rooms/{ALL_TALK}/messages/{id}.json"))
        .header("content-type", "application/json")
        .body(json!({"message": {"markdown_source": "rejected", "drive_file_ids": ["abcdefghij"]}}).to_string())).await;
    assert_eq!(reply.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(app.db().read(move |conn| Message::find(conn, id)).await.unwrap(), original);
    assert!(app.db().read(move |conn| Message::find(conn, id)?.drive_file_ids(conn)).await.unwrap().is_empty());
}

use campfire_web::controllers::presenters::{Rendering};
