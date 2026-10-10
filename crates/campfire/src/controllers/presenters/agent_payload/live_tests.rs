//! WS8's shared reader through the production presenter; rendered bytes come from Rails.
use super::super::test_support::{ALL_TALK, BENDER, DAVID, SEED_NOW, TestApp};
use campfire_db::Message;
use campfire_runtime::presenters::Presenter;
use campfire_db::{ChannelThread, NewChannelThread, NewMessage, ThreadMembership};
use serde_json::Value;
use std::sync::Arc;
fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../../vectors/agents_message_presenter_contract.json"
    ))
    .unwrap()
}
async fn capture(app: &crate::app::App, case: &str, id: i64, viewer: i64, base: &str) {
    let app2 = app.clone();
    let name = case.to_owned();
    let base = base.to_owned();
    app.db
        .read(move |conn| {
            let mut p = Presenter::new(conn, &app2, None);
            p.current_user_id = Some(viewer);
            p.cache_base_url = Some(base.clone());
            p.request_host = Some(base.trim_start_matches("https://").to_owned());
            let payload = p.agent_message_payload(&Message::find(conn, id)?)?;
            assert_eq!(
                payload,
                oracle()["cases"][&name],
                "Rails payload {name}; rendered HTML bytes included"
            );
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn ws11_live_message_presenter_matches_rails_rendered_bytes_and_request_permissions() {
    let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
        SEED_NOW.parse().unwrap(),
    ));
    let (app, _dir) = TestApp::boot_with_clock(clock)
        .await
        .expect("default seed")
        .stop_jobs()
        .await;
    capture(&app, "rich_text", 136976342, DAVID, "https://payload.test").await;
    let (mid, rid) = app
        .db
        .write(|tx| {
            let m = Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    markdown_source: Some(
                        "Hello **world** & <x>\n\n```rust\nlet x = 1;\n```".into(),
                    ),
                    client_message_id: Some("ws11-presenter".into()),
                    ..Default::default()
                },
            )?;
            let reply = Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: BENDER,
                    markdown_source: Some("Answer".into()),
                client_message_id: Some("ws11-presenter-reply".into()),
                    reply_to_message_id: Some(m.id),
                    reply_notify_author: Some(true),
                    forwarded_from_message_id: Some(m.id),
                    forwarded_at: Some(tx.now()),
                    forward_note: Some("From elsewhere".into()),
                    drive_file_ids: vec!["ws11-public-drive-file".into()],
                    ..Default::default()
                },
            )?;
            Ok((m.id, reply.id))
        })
        .await
        .unwrap();
    capture(&app, "markdown", mid, DAVID, "https://payload.test").await;
    capture(
        &app,
        "reply_forward_drive",
        rid,
        DAVID,
        "https://payload.test",
    )
    .await;
    app.db
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE messages SET reply_to_message_id=NULL,reply_target_deleted_at=? WHERE id=?",
                rusqlite::params![tx.now(), rid],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    capture(&app, "deleted_reply", rid, DAVID, "https://payload.test").await;
    let tid = app
        .db
        .write(move |tx| {
            let thread = ChannelThread::create(
                tx,
                NewChannelThread {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    parent_message_id: Some(mid),
                    name: Some("Presenter chat".into()),
                    ..Default::default()
                },
            )?;
            ThreadMembership::join(tx, thread.id, DAVID)?;
            Ok(thread.id)
        })
        .await
        .unwrap();
    capture(&app, "root_human", mid, DAVID, "https://payload.test").await;
    capture(&app, "root_bot", mid, BENDER, "https://payload.test").await;
    let threaded = app
        .db
        .write(move |tx| {
            Ok(Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: BENDER,
                    thread_id: Some(tid),
                    markdown_source: Some("Thread reply".into()),
                client_message_id: Some("ws11-presenter-thread".into()),
                    streaming: true,
                    ..Default::default()
                },
            )?
            .id)
        })
        .await
        .unwrap();
    capture(&app, "thread_bot", threaded, BENDER, "https://payload.test").await;
    capture(&app, "thread_human", threaded, DAVID, "https://other.test").await;
}
