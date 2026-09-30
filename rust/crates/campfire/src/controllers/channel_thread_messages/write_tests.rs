use std::sync::Arc;
use axum::http::{Method, StatusCode};
use campfire_db::{ChannelThread, Message, NewChannelThread, NewMessage, ThreadMembership, Tx};
use campfire_kit::clock::FrozenClock;
use serde_json::{Value, json};
use crate::controllers::presenters::test_support::*;

pub(crate) fn oracle() -> Value { serde_json::from_str(include_str!("../../../../../vectors/messaging/thread-message-writes.json")).unwrap() }

pub(crate) fn seed(tx: &mut Tx<'_>) -> campfire_db::Result<()> {
    let parent = Message::create(tx, NewMessage { room_id: ALL_TALK, creator_id: DAVID, markdown_source: Some("Write parent".into()), client_message_id: Some("writes-parent".into()), ..Default::default() })?;
    let mut thread = ChannelThread::create(tx, NewChannelThread { room_id: ALL_TALK, creator_id: JASON, parent_message_id: Some(parent.id), name: Some("Write thread".into()), ..Default::default() })?;
    let initial = thread.post_message(tx, JASON, NewMessage { markdown_source: Some("Original".into()), client_message_id: Some("writes-original".into()), ..Default::default() })?;
    thread.close(tx)?;
    assert_eq!(parent.id, oracle()["parent_id"]);
    assert_eq!(thread.id, oracle()["thread_id"]);
    assert_eq!(initial.id, oracle()["initial_id"]);
    Ok(())
}

pub(crate) fn prepare(tx: &mut Tx<'_>, name: &str) -> campfire_db::Result<()> {
    let thread = oracle()["thread_id"].as_i64().unwrap();
    match name {
        "destroy" => {
            let reply = Message::create(tx, NewMessage { room_id: ALL_TALK, creator_id: DAVID, thread_id: Some(thread), markdown_source: Some("Reply".into()),
                reply_to_message_id: oracle()["posted_id"].as_i64(), client_message_id: Some("writes-reply".into()), ..Default::default() })?;
            assert_eq!(reply.id, oracle()["reply_id"]);
        },
        "locked_post" => ChannelThread::find(tx.conn(), thread)?.lock_conversation(tx)?,
        "note_update" => {
            let note = Message::create(tx, NewMessage { room_id: ALL_TALK, creator_id: DAVID, thread_id: Some(thread), system_note: true,
                markdown_source: Some("Immutable".into()), client_message_id: Some("writes-note".into()), ..Default::default() })?;
            assert_eq!(note.id, oracle()["note_id"]);
        },
        _ => {},
    }
    Ok(())
}

pub(crate) fn request(row: &Value) -> Req {
    let mut request = Req::new(Method::from_bytes(row["method"].as_str().unwrap().to_uppercase().as_bytes()).unwrap(), row["path"].as_str().unwrap());
    if row["input"].is_object() { request = request.header("content-type", "application/json").body(json!({"message": row["input"]}).to_string()); }
    request.header("accept", "application/json")
}

#[tokio::test]
async fn thread_writes_match_rails_rows_retries_drive_sets_locks_and_response_bytes() {
    let clock = Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
    let app = TestApp::boot_with_test_clock(clock.clone()).await.unwrap();
    app.db().write(seed).await.unwrap();
    let mut browser = app.david();
    for row in oracle()["rows"].as_array().unwrap() {
        let name = row["name"].as_str().unwrap().to_owned();
        app.db().write(move |tx| prepare(tx, &name)).await.unwrap();
        clock.set(row["time"].as_str().unwrap().parse().unwrap());
        let response = browser.write(request(row)).await;
        let name = row["name"].as_str().unwrap();
        assert_eq!(response.status.as_u16(), row["status"].as_u64().unwrap() as u16, "{name}: {}", response.text());
        assert_eq!(response.header("cache-control"), row["cache_control"].as_str(), "{name}");
        assert_eq!(response.content_type(), row["content_type"].as_str(), "{name}");
        assert_eq!(response.location(), row["location"].as_str(), "{name}");
        if response.text() != row["body"].as_str().unwrap() { rails_mismatch(&response.text(), row["body"].as_str().unwrap(), name); }
        let now = campfire_db::Timestamp::from_jiff(row["time"].as_str().unwrap().parse().unwrap());
        let state = app.db().read(move |conn| {
            let thread = ChannelThread::find(conn, oracle()["thread_id"].as_i64().unwrap())?;
            let records = Message::in_thread(conn, thread.id)?.iter().map(|message| Ok(json!({
                "id": message.id, "client_message_id": message.client_message_id, "creator_id": message.creator_id,
                "markdown_source": message.markdown_source, "body": message.body_html(conn)?, "edited_at": message.edited_at.map(|time| campfire_views::messages::support::json_time(time.jiff())),
                "reply_to_message_id": message.reply_to_message_id, "reply_target_deleted_at": message.reply_target_deleted_at.map(|time| campfire_views::messages::support::json_time(time.jiff())),
                "drive_file_ids": message.drive_file_ids(conn)?
            }))).collect::<campfire_db::Result<Vec<_>>>()?;
            Ok(json!({"records": records, "closed": thread.status(conn, now)?.name() == "closed", "locked": thread.locked_at.is_some(),
                "joined": ThreadMembership::find_by_thread_and_user(conn, thread.id, DAVID)?.is_some(), "message_count": thread.message_count(conn)?}))
        }).await.unwrap();
        for key in ["records", "closed", "locked", "joined", "message_count"] { assert_eq!(state[key], row[key], "{name}/{key}"); }
    }
}

#[tokio::test]
async fn nested_writes_enforce_author_admin_notes_scope_and_csrf_before_changes() {
    let app = TestApp::boot().await.unwrap();
    app.db().write(seed).await.unwrap();
    let data = oracle();
    let (thread, initial, parent) = (data["thread_id"].as_i64().unwrap(), data["initial_id"].as_i64().unwrap(), data["parent_id"].as_i64().unwrap());
    let base = format!("/rooms/{ALL_TALK}/threads/{thread}/messages");
    let mut david = app.david();
    assert_eq!(david.write(Req::new(Method::PATCH, &format!("{base}/{initial}.json")).form(&[("message[markdown_source]", "Admin cannot edit")])).await.status, StatusCode::FORBIDDEN);
    assert_eq!(david.write(Req::new(Method::DELETE, &format!("{base}/{parent}.json"))).await.status, StatusCode::NOT_FOUND);
    assert_eq!(david.write(Req::new(Method::POST, &format!("/rooms/{QUIET_CORNER}/threads/{thread}/messages.json")).form(&[("message[markdown_source]", "Wrong room")])).await.status, StatusCode::NOT_FOUND);
    assert_eq!(david.send(Req::new(Method::POST, &format!("{base}.json")).header("origin", "https://other.test").form(&[("message[markdown_source]", "Forged")])).await.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(app.anonymous().send(Req::new(Method::POST, &format!("{base}.json?bot_key={BENDER_KEY}")).form(&[("message[markdown_source]", "Bot")])).await.status, StatusCode::FORBIDDEN);
    app.db().write(|tx| { tx.conn().execute("UPDATE users SET role = 0 WHERE id = ?", [JASON])?; Ok(()) }).await.unwrap();
    let own = app.db().write(move |tx| Message::create(tx, NewMessage { room_id: ALL_TALK, thread_id: Some(thread), creator_id: DAVID, markdown_source: Some("David".into()), ..Default::default() })).await.unwrap();
    let mut jason = app.sign_in(JASON).await;
    assert_eq!(jason.write(Req::new(Method::DELETE, &format!("{base}/{}.json", own.id))).await.status, StatusCode::FORBIDDEN);
    assert!(app.db().read(move |conn| Message::find_by_id(conn, own.id)).await.unwrap().is_some());
}

#[tokio::test]
async fn rejected_thread_enqueue_rolls_back_post_join_and_reopening() {
    let app = TestApp::boot().await.unwrap();
    app.db().write(seed).await.unwrap();
    app.db().write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER ws8bm_reject_thread_job BEFORE INSERT ON background_jobs BEGIN SELECT RAISE(ABORT, 'WS8bm thread enqueue rejection'); END;")?;
        Ok(())
    }).await.unwrap();
    let thread = oracle()["thread_id"].as_i64().unwrap();
    let response = app.david().write(Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/threads/{thread}/messages.json"))
        .form(&[("message[markdown_source]", "Rejected"), ("message[client_message_id]", "rejected-thread")])).await;
    assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
    app.db().read(move |conn| {
        let thread = ChannelThread::find(conn, thread)?;
        assert!(thread.closed_at.is_some());
        assert_eq!(thread.message_count(conn)?, 1);
        assert!(ThreadMembership::find_by_thread_and_user(conn, thread.id, DAVID)?.is_none());
        Ok(())
    }).await.unwrap();
}
