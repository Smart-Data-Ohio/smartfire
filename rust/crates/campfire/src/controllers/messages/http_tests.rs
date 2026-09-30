//! Fork-specific request contracts; the Rails-built parity seed is mandatory here.
use axum::http::{Method, StatusCode};
use campfire_db::{ChannelThread, Message, NewChannelThread, NewMessage};

use crate::controllers::presenters::test_support::*;

async fn boot() -> TestApp {
    TestApp::boot().await.expect("WS8bm HTTP tests require parity/bin/seed build default")
}

async fn create(app: &TestApp, creator_id: i64, system_note: bool) -> Message {
    app.db().write(move |tx| Message::create(tx, NewMessage {
        room_id: ALL_TALK, creator_id, system_note,
        markdown_source: Some("original".into()), ..Default::default()
    })).await.unwrap()
}

#[tokio::test]
async fn author_only_edits_even_for_an_administrator() {
    let app = boot().await;
    let message = create(&app, JASON, false).await;
    let path = format!("/rooms/{ALL_TALK}/messages/{}", message.id);
    let mut david = app.david();
    assert_eq!(david.get(&format!("{path}/edit")).await.status, StatusCode::FORBIDDEN);
    assert_eq!(david.write(Req::new(Method::PATCH, &path).form(&[("message[body]", "changed")])).await.status, StatusCode::FORBIDDEN);
    let original = app.db().read(move |conn| Message::find(conn, message.id)?.body_html(conn)).await.unwrap();
    assert_eq!(original.as_deref(), Some("<p>original</p>"));
}

#[tokio::test]
async fn system_notes_are_immutable_for_author_and_administrator() {
    let app = boot().await;
    let mut david = app.david();
    for creator in [DAVID, JASON] {
        let note = create(&app, creator, true).await;
        let path = format!("/rooms/{ALL_TALK}/messages/{}", note.id);
        assert_eq!(david.get(&format!("{path}/edit")).await.status, StatusCode::FORBIDDEN);
        assert_eq!(david.write(Req::new(Method::PATCH, &path).form(&[("message[body]", "changed")])).await.status, StatusCode::FORBIDDEN);
        assert_eq!(david.write(Req::new(Method::DELETE, &path).header("accept", "text/vnd.turbo-stream.html")).await.status, StatusCode::FORBIDDEN);
        assert!(app.db().read(move |conn| Message::find_by_id(conn, note.id)).await.unwrap().is_some());
    }
}

#[tokio::test]
async fn non_author_non_admin_cannot_edit_or_delete() {
    let app = boot().await;
    app.db().write(|tx| {
        tx.conn().execute("UPDATE users SET role = 0 WHERE id = ?", [JASON])?;
        Ok(())
    }).await.unwrap();
    let message = create(&app, DAVID, false).await;
    let path = format!("/rooms/{ALL_TALK}/messages/{}", message.id);
    let mut jason = app.sign_in(JASON).await;
    assert_eq!(jason.get(&format!("{path}/edit")).await.status, StatusCode::FORBIDDEN);
    assert_eq!(jason.write(Req::new(Method::DELETE, &path).header("accept", "text/vnd.turbo-stream.html")).await.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn preview_matches_real_rails_http_without_writing() {
    let app = boot().await;
    let oracle: serde_json::Value = serde_json::from_str(include_str!("../../../../../vectors/messaging/preview.json")).unwrap();
    let before = app.db().read(|conn| Ok(conn.query_row("SELECT count(*) FROM messages", [], |r| r.get::<_, i64>(0))?)).await.unwrap();
    let mut david = app.david();
    for row in oracle["previews"].as_array().unwrap() {
        let body = serde_json::to_string(&serde_json::json!({"message": {"markdown_source": row["source"]}})).unwrap();
        let reply = david.write(Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/messages/preview"))
            .header("content-type", "application/json").header("accept", "application/json").body(body)).await;
        assert_eq!(reply.status.as_u16(), row["status"].as_u64().unwrap() as u16, "{}", reply.text());
        assert_eq!(reply.json(), row["json"], "source prefix: {:?}", row["source"].as_str().unwrap().chars().take(40).collect::<String>());
    }
    let after = app.db().read(|conn| Ok(conn.query_row("SELECT count(*) FROM messages", [], |r| r.get::<_, i64>(0))?)).await.unwrap();
    assert_eq!(before, after);
}

#[tokio::test]
async fn preview_requires_membership_and_a_valid_source_parameter() {
    let app = boot().await;
    let mut david = app.david();
    let path = format!("/rooms/{ALL_TALK}/messages/preview");
    for body in ["{}", r#"{"message":{"body":"wrong"}}"#, r#"{"message":{"markdown_source":{}}}"#] {
        assert_eq!(david.write(Req::new(Method::POST, &path).header("content-type", "application/json").body(body)).await.status, StatusCode::BAD_REQUEST);
    }
    let mut kevin = app.sign_in(KEVIN).await;
    assert_eq!(kevin.write(Req::new(Method::POST, &path).form(&[("message[markdown_source]", "private")])).await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn deleted_room_is_inaccessible_even_with_a_lingering_membership() {
    let app = boot().await;
    let message = create(&app, DAVID, false).await;
    app.db().write(|tx| {
        tx.conn().execute("UPDATE rooms SET deleted_at = ? WHERE id = ?", (tx.now(), ALL_TALK))?;
        Ok(())
    }).await.unwrap();
    let mut david = app.david();
    for path in [format!("/rooms/{ALL_TALK}/messages"), format!("/rooms/{ALL_TALK}/messages/{}", message.id)] {
        assert_eq!(david.get(&path).await.status, StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
async fn removed_and_non_members_cannot_read_or_edit_messages() {
    let app = boot().await;
    let message = create(&app, DAVID, false).await;
    let path = format!("/rooms/{ALL_TALK}/messages/{}", message.id);
    let mut kevin = app.sign_in(KEVIN).await;
    assert_eq!(kevin.get(&path).await.status, StatusCode::NOT_FOUND);
    app.db().write(|tx| {
        tx.conn().execute("DELETE FROM memberships WHERE room_id = ? AND user_id = ?", (ALL_TALK, DAVID))?;
        Ok(())
    }).await.unwrap();
    let mut david = app.david();
    assert_eq!(david.get(&path).await.status, StatusCode::NOT_FOUND);
    assert_eq!(david.write(Req::new(Method::PATCH, &path).form(&[("message[body]", "changed")])).await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn root_endpoint_cannot_read_edit_or_delete_thread_messages() {
    let app = boot().await;
    let message = create(&app, DAVID, false).await;
    let message = app.db().write(move |tx| {
        let thread = ChannelThread::create(tx, NewChannelThread {
            room_id: ALL_TALK, creator_id: DAVID, parent_message_id: Some(message.id), ..Default::default()
        })?;
        Message::create(tx, NewMessage {
            room_id: ALL_TALK, creator_id: DAVID, thread_id: Some(thread.id),
            markdown_source: Some("thread reply".into()), ..Default::default()
        })
    }).await.unwrap();
    let path = format!("/rooms/{ALL_TALK}/messages/{}", message.id);
    let mut david = app.david();
    assert_eq!(david.get(&path).await.status, StatusCode::NOT_FOUND);
    assert_eq!(david.write(Req::new(Method::PATCH, &path).form(&[("message[body]", "changed")])).await.status, StatusCode::NOT_FOUND);
    assert_eq!(david.write(Req::new(Method::DELETE, &path).header("accept", "text/vnd.turbo-stream.html")).await.status, StatusCode::NOT_FOUND);
}
