use super::*;
use campfire_db::Message;
use campfire_db::NewMessage;
use campfire_db::Room;
use campfire_db::User;
use crate::integrations::webhook;
use crate::queue::WebhookJob;
use crate::controllers::presenters::test_support::{TestApp, ALL_TALK, BENDER, DAVID};

#[tokio::test]
async fn attachment_processing_root_webhook_recovers_failed_inline_video_like_rails() {
    let test_app = TestApp::boot_frozen().await.unwrap().without_job_runner().await;
    let app = &test_app.booted.app;
    let (_client, server) = crate::controllers::messages::attachment_processing_tests::subscribe(&test_app).await;
    let (room, bot, trigger) = app.db.write(|tx| {
        tx.conn().execute("DELETE FROM background_jobs", [])?;
        Ok((Room::find(tx.conn(), ALL_TALK)?, User::find(tx.conn(), BENDER)?,
            Message::create(tx, NewMessage { room_id: ALL_TALK, creator_id: DAVID,
                body: Some("Root webhook trigger".into()), ..Default::default() })?))
    }).await.unwrap();
    let reply = create_attachment_reply(app, &room, &bot, trigger, webhook::Attachment {
        data: b"corrupt MOV".to_vec(), filename: "attachment.mov".into(), content_type: "video/quicktime".into(),
    }).await.unwrap();
    test_app.publications().take();
    broadcast_create(app, &room, &reply).await.unwrap();
    let id = reply.id;
    let actual = app.db.read(move |c| {
        let blob_id: i64 = c.query_row("SELECT blob_id FROM active_storage_attachments WHERE record_type='Message' AND name='attachment' AND record_id=?", [id], |r| r.get(0))?;
        let token: Option<String> = c.query_row("SELECT message_processing_token FROM active_storage_blobs WHERE id=?", [blob_id], |r| r.get(0))?;
        let preview_attached: bool = c.query_row("SELECT EXISTS(SELECT 1 FROM active_storage_attachments WHERE record_type='ActiveStorage::Blob' AND name='preview_image' AND record_id=?)", [blob_id], |r| r.get(0))?;
        let count: i64 = c.query_row("SELECT count(*) FROM background_jobs WHERE job_class='Message::AttachmentProcessingJob'", [], |r| r.get(0))?;
        Ok(serde_json::json!({"processing_jobs":count,"preview_attached":preview_attached,"token_suffix":token.and_then(|t| t.rsplit(':').next().map(str::to_owned))}))
    }).await.unwrap();
    let mut actual = actual;
    actual["append_without_poster"] = serde_json::json!(test_app.publications().take().iter().any(|(_, bytes)| {
        let payload: serde_json::Value = serde_json::from_str(bytes).unwrap();
        payload.as_str().is_some_and(|html| html.contains("<video") && !html.contains("poster="))
    }));
    let expected: serde_json::Value = serde_json::from_str(include_str!("../../../../../vectors/message_attachment_processing_failures.json")).unwrap();
    assert_eq!(actual, expected["root_webhook"], "rendering the root reply must preserve Rails' recovery request");
    server.abort();
}

#[test]
fn ws11_legacy_webhook_retry_policy_keeps_transient_sources() {
    use campfire_jobs::JobKind;
    use crate::net::http::HttpError;
    use crate::integrations::webhook::WebhookError;
    let policy = WebhookJob::retry_policy();
    let timeout = anyhow::Error::new(WebhookError::Http(HttpError::Io(std::io::Error::new(std::io::ErrorKind::TimedOut, "write timed out"))));
    assert!((policy.retry_on)(&timeout));
    let refused = anyhow::Error::new(WebhookError::Http(HttpError::Io(std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused"))));
    assert!(!(policy.retry_on)(&refused));
    assert!(!(policy.retry_on)(&anyhow::Error::new(WebhookError::InvalidUrl("bad URI".into()))));
    let delays: Vec<_> = (1..=5).map(|attempt| policy.retry_delay(attempt, None, 0.0).map(|delay| delay.as_secs())).collect();
    assert_eq!(delays, vec![Some(3), Some(18), Some(83), Some(258), None]);
}

#[tokio::test]
async fn ws11_sync_replies_use_root_links_threads_boards_and_locking() {
    let app = TestApp::boot().await.expect("build the default parity seed");
    app.db().write(|tx| {
        let attrs = || NewMessage { room_id: ALL_TALK, creator_id: DAVID, body: Some("Trigger".into()), ..Default::default() };
        let root = Message::create(tx, attrs())?;
        let root_reply = create_reply(tx, Some(&root), NewMessage { room_id: ALL_TALK, creator_id: BENDER, body: Some("Answer".into()), ..Default::default() })?;
        assert_eq!(root_reply.reply_to_message_id, Some(root.id));
        assert_eq!(root_reply.thread_id, None);
        let timeout = create_reply(tx, None, NewMessage { room_id: ALL_TALK, creator_id: BENDER, body: Some("Failed to respond within 7 seconds".into()), ..Default::default() })?;
        assert_eq!(timeout.reply_to_message_id, None);
        let mut thread = campfire_db::ChannelThread::create(tx, campfire_db::NewChannelThread { room_id: ALL_TALK, creator_id: DAVID, name: Some("Replies".into()), ..Default::default() })?;
        let trigger = thread.post_message(tx, DAVID, attrs())?;
        let reply = create_reply(tx, Some(&trigger), NewMessage { room_id: ALL_TALK, creator_id: BENDER, body: Some("Thread answer".into()), ..Default::default() })?;
        assert_eq!(reply.thread_id, Some(thread.id));
        assert_eq!(reply.reply_to_message_id, Some(trigger.id));
        assert_eq!(campfire_db::ChannelThread::find(tx.conn(), thread.id)?.messages_count, 2);
        tx.conn().execute("UPDATE rooms SET type='Rooms::Board' WHERE id=?", [ALL_TALK])?;
        tx.conn().execute("UPDATE channel_threads SET work_status='in_progress' WHERE id=?", [thread.id])?;
        thread = campfire_db::ChannelThread::find(tx.conn(), thread.id)?;
        let board_reply = create_reply(tx, Some(&trigger), NewMessage { room_id: ALL_TALK, creator_id: BENDER, body: Some("Board answer".into()), ..Default::default() })?;
        assert_eq!(board_reply.thread_id, Some(thread.id));
        assert_eq!(board_reply.reply_to_message_id, Some(trigger.id));
        thread.lock_conversation(tx)?;
        assert!(matches!(create_reply(tx, Some(&trigger), NewMessage { room_id: ALL_TALK, creator_id: BENDER, body: Some("Locked".into()), ..Default::default() }), Err(campfire_db::Error::Other(error)) if error == campfire_db::models::channel_thread::LOCKED_MESSAGE));
        Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn ws11_sync_attachment_reply_is_stored_in_the_trigger_thread() {
    let test_app = TestApp::boot().await.expect("build the default parity seed");
    let app = &test_app.booted.app;
    let (room, bot, trigger) = app.db.write(|tx| {
        let room = Room::find(tx.conn(), ALL_TALK)?;
        let bot = User::find(tx.conn(), BENDER)?;
        let mut thread = campfire_db::ChannelThread::create(tx, campfire_db::NewChannelThread { room_id: ALL_TALK, creator_id: DAVID, name: Some("Attachment reply".into()), ..Default::default() })?;
        let trigger = thread.post_message(tx, DAVID, NewMessage { body: Some("Attach".into()), ..Default::default() })?;
        Ok((room, bot, trigger))
    }).await.unwrap();
    let reply = create_attachment_reply(app, &room, &bot, trigger.clone(), webhook::Attachment { data: b"a,b\n1,2\n".to_vec(), filename: "attachment.csv".into(), content_type: "text/csv".into() }).await.unwrap();
    assert_eq!(reply.thread_id, trigger.thread_id);
    assert_eq!(reply.reply_to_message_id, Some(trigger.id));
    let (_, blob) = app.db.read(move |conn| reply.attachment(conn)).await.unwrap().unwrap();
    assert_eq!(blob.filename, "attachment.csv");
    assert_eq!(blob.byte_size, 8);
}
