use super::*;

fn blob_id(a: &TestApp, file: &api::DirectUpload) -> i64 {
    campfire_storage::paths::verify_signed_blob_id(
        &*a.booted.app.storage.verifier,
        &file.signed_id,
        a.booted.app.clock.now(),
    )
    .unwrap()
}

async fn legacy_attachment_edit(thread: bool, grouped: bool) {
    let a = app(true).await.expect("restored default seed").without_job_runner().await;
    let mut david = a.sign_in(DAVID).await;
    let file = upload(&mut david, "current.txt").await;
    let file_id = blob_id(&a, &file);
    let message = a.db().write(move |tx| {
        let thread_id = if thread {
            Some(campfire_db::ChannelThread::create(tx, campfire_db::NewChannelThread {
                room_id: ALL_TALK, creator_id: DAVID, ..Default::default()
            })?.id)
        } else { None };
        campfire_db::Message::create(tx, campfire_db::NewMessage {
            room_id: ALL_TALK, creator_id: DAVID, thread_id,
            markdown_source: Some("Original".into()),
            attachment_blob_id: (!grouped).then_some(file_id),
            attachment_blob_ids: if grouped { vec![file_id] } else { Vec::new() },
            ..Default::default()
        })
    }).await.unwrap();
    let base = message.thread_id.map_or_else(
        || format!("/rooms/{ALL_TALK}/messages"),
        |id| format!("/rooms/{ALL_TALK}/threads/{id}/messages"),
    );
    let path = format!("{base}/{}.json", message.id);
    let reply = david.write(json_body(Method::PATCH, &path,
        &json!({"message": {"attachment": file.signed_id, "markdown_source": "Changed"}}))).await;
    assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.text());
    a.db().read(move |conn| {
        let saved = campfire_db::Message::find(conn, message.id)?;
        assert_eq!(saved.markdown_source.as_deref(), Some("Changed"));
        assert_eq!(saved.attachments(conn)?.len(), 1);
        assert!(saved.attachments(conn)?.iter().all(|(_, blob)| blob.id == file_id));
        Ok(())
    }).await.unwrap();

    a.db().write(move |tx| {
        tx.conn().execute("UPDATE active_storage_blobs SET metadata=json_set(metadata,'$.uploader_id',?) WHERE id=?", [KEVIN, file_id])?;
        Ok(())
    }).await.unwrap();
    let reply = david.write(json_body(Method::PATCH, &path,
        &json!({"message": {"attachment": file.signed_id, "markdown_source": "Changed"}}))).await;
    assert_eq!(reply.status, StatusCode::OK, "a current attachment keeps its original uploader: {}", reply.text());

    a.db().write(move |tx| {
        campfire_db::Message::create(tx, campfire_db::NewMessage {
            room_id: ALL_TALK, creator_id: DAVID, attachment_blob_id: Some(file_id),
            ..Default::default()
        })?;
        Ok(())
    }).await.unwrap();
    let before = write_counts(&a).await;
    let reply = david.write(json_body(Method::PATCH, &path,
        &json!({"message": {"attachment": file.signed_id, "markdown_source": "Rejected"}}))).await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", reply.text());
    assert_eq!(write_counts(&a).await, before);
    a.db().read(move |conn| {
        assert_eq!(campfire_db::Message::find(conn, message.id)?.markdown_source.as_deref(), Some("Changed"));
        Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn grouped_files_round4_legacy_room_edit_keeps_current_attachment() {
    legacy_attachment_edit(false, false).await;
}

#[tokio::test]
async fn grouped_files_round4_legacy_thread_edit_keeps_current_attachment() {
    legacy_attachment_edit(true, false).await;
}

#[tokio::test]
async fn grouped_files_round4_legacy_edit_accepts_current_grouped_attachment() {
    legacy_attachment_edit(false, true).await;
}

#[tokio::test]
async fn grouped_files_round5_legacy_edit_retains_ten_grouped_files_and_text() {
    let a = app(true).await.expect("restored default seed").without_job_runner().await;
    let mut david = a.sign_in(DAVID).await;
    let mut files = Vec::new();
    for index in 0..10 {
        files.push(upload(&mut david, &format!("retain-{index}.txt")).await.signed_id);
    }
    let created = david.write(json_body(Method::POST,
        &format!("/api/v1/rooms/{ALL_TALK}/messages"),
        &json!({"clientMessageId": "retain-ten-grouped", "markdownSource": "Original", "attachmentSignedIds": files}))).await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.text());
    let message: api::MessageDTO = parse(&created);
    let id = message.id;
    let original = a.db().read(move |conn| campfire_db::Message::find(conn, id)?.attachments(conn)).await.unwrap();
    let before = write_counts(&a).await;
    let reply = david.write(json_body(Method::PATCH,
        &format!("/rooms/{ALL_TALK}/messages/{id}.json"),
        &json!({"message": {"attachment": files[0], "markdown_source": "Changed"}}))).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert_eq!(write_counts(&a).await, before);
    let read: api::MessageRead = parse(&david.send(get(&format!("/api/v1/messages/{id}"))).await);
    assert_eq!(read.message.markdown_source.as_deref(), Some("Changed"));
    assert_eq!(read.message.attachments, message.attachments);
    a.db().read(move |conn| {
        let saved = campfire_db::Message::find(conn, id)?;
        assert_eq!(saved.markdown_source.as_deref(), Some("Changed"));
        assert!(saved.edited_at.is_some());
        assert_eq!(saved.attachments(conn)?, original);
        Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn grouped_files_round4_bot_edit_keeps_current_attachment() {
    use crate::controllers::presenters::test_support::{BENDER, BENDER_KEY};
    let a = app(true).await.expect("restored default seed").without_job_runner().await;
    let file = upload(&mut a.sign_in(DAVID).await, "bot-current.txt").await;
    let file_id = blob_id(&a, &file);
    let id = a.db().write(move |tx| {
        tx.conn().execute("UPDATE active_storage_blobs SET metadata=json_set(metadata,'$.uploader_id',?) WHERE id=?", [BENDER, file_id])?;
        Ok(campfire_db::Message::create(tx, campfire_db::NewMessage {
            room_id: ALL_TALK, creator_id: BENDER, attachment_blob_id: Some(file_id),
            ..Default::default()
        })?.id)
    }).await.unwrap();
    let reply = a.anonymous().send(json_body(Method::PATCH,
        &format!("/rooms/{ALL_TALK}/{BENDER_KEY}/messages/{id}"),
        &json!({"attachment": file.signed_id}))).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    a.db().read(move |conn| {
        let files = campfire_db::Message::find(conn, id)?.attachments(conn)?;
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].1.id, file_id);
        Ok(())
    }).await.unwrap();
}

async fn legacy_attachment_edit_limit(thread: bool, bot: bool) {
    use crate::controllers::presenters::test_support::{BENDER, BENDER_KEY};
    let a = app(true).await.expect("restored default seed").without_job_runner().await;
    let mut david = a.sign_in(DAVID).await;
    let mut files = Vec::new();
    for index in 0..10 {
        files.push(upload(&mut david, &format!("{index}.txt")).await.signed_id);
    }
    let extra = upload(&mut david, "eleventh.txt").await;
    let extra_id = blob_id(&a, &extra);
    let creator = if bot { BENDER } else { DAVID };
    let thread_id = if thread {
        Some(a.db().write(|tx| Ok(campfire_db::ChannelThread::create(tx, campfire_db::NewChannelThread {
            room_id: ALL_TALK, creator_id: DAVID, ..Default::default()
        })?.id)).await.unwrap())
    } else { None };
    let create_path = thread_id.map_or_else(
        || format!("/api/v1/rooms/{ALL_TALK}/messages"),
        |id| format!("/api/v1/threads/{id}/messages"),
    );
    let created = david.write(json_body(Method::POST, &create_path,
        &json!({"clientMessageId": "ten-files-edit", "markdownSource": "Original", "attachmentSignedIds": files}))).await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.text());
    let id = parse::<api::MessageDTO>(&created).id;
    if bot {
        a.db().write(move |tx| {
            tx.conn().execute("UPDATE messages SET creator_id=? WHERE id=?", [creator, id])?;
            tx.conn().execute("UPDATE active_storage_blobs SET metadata=json_set(metadata,'$.uploader_id',?) WHERE id=?", [creator, extra_id])?;
            Ok(())
        }).await.unwrap();
    }
    let path = if bot { format!("/rooms/{ALL_TALK}/{BENDER_KEY}/messages/{id}") }
        else if let Some(thread_id) = thread_id { format!("/rooms/{ALL_TALK}/threads/{thread_id}/messages/{id}.json") }
        else { format!("/rooms/{ALL_TALK}/messages/{id}.json") };
    let before = write_counts(&a).await;
    let input = if bot { json!({"attachment": extra.signed_id}) }
        else { json!({"message": {"attachment": extra.signed_id, "markdown_source": "Rejected"}}) };
    let request = json_body(Method::PATCH, &path, &input);
    let reply = if bot { a.anonymous().send(request).await } else { david.write(request).await };
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{path}: {}", reply.text());
    assert!(reply.text().contains("10"), "{}", reply.text());
    assert_eq!(write_counts(&a).await, before);
    let read = david.send(get(&format!("/api/v1/messages/{id}"))).await;
    assert_eq!(read.status, StatusCode::OK, "{}", read.text());
    let read: Value = parse(&read);
    assert_eq!(read["message"]["attachments"].as_array().unwrap().len(), 10);
    assert_eq!(read["message"]["markdownSource"], "Original");
    a.db().read(move |conn| {
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM active_storage_attachments WHERE blob_id=?", [extra_id], |row| row.get::<_, i64>(0))?, 0);
        Ok(())
    }).await.unwrap();
    a.db().write(move |tx| {
        let files = campfire_db::Message::find(tx.conn(), id)?.attachments(tx.conn())?;
        files.last().unwrap().0.delete(tx)
    }).await.unwrap();
    for _ in 0..2 {
        let request = json_body(Method::PATCH, &path, &input);
        let reply = if bot { a.anonymous().send(request).await } else { david.write(request).await };
        assert_eq!(reply.status, StatusCode::OK, "nine grouped files plus the legacy file: {}", reply.text());
        a.db().read(move |conn| {
            let files = campfire_db::Message::find(conn, id)?.attachments(conn)?;
            assert_eq!(files.len(), 10);
            assert_eq!(files.last().unwrap().1.id, extra_id);
            Ok(())
        }).await.unwrap();
    }
}

#[tokio::test]
async fn grouped_files_round4_legacy_room_edit_enforces_total_limit() {
    legacy_attachment_edit_limit(false, false).await;
}

#[tokio::test]
async fn grouped_files_round4_legacy_thread_edit_enforces_total_limit() {
    legacy_attachment_edit_limit(true, false).await;
}

#[tokio::test]
async fn grouped_files_round4_bot_edit_enforces_total_limit() {
    legacy_attachment_edit_limit(false, true).await;
}

async fn legacy_attachment_claims(thread: bool, edit: bool) {
    let a = app(true).await.expect("restored default seed").without_job_runner().await;
    let mut david = a.sign_in(DAVID).await;
    let foreign = upload(&mut a.sign_in(KEVIN).await, "foreign.txt").await;
    let used = upload(&mut david, "used.txt").await;
    let owned = upload(&mut david, "owned.txt").await;
    let used_id = blob_id(&a, &used);
    let (thread_id, message_id) = a.db().write(move |tx| {
        campfire_db::Message::create(tx, campfire_db::NewMessage {
            room_id: ALL_TALK, creator_id: DAVID, attachment_blob_id: Some(used_id),
            ..Default::default()
        })?;
        let thread_id = if thread {
            Some(campfire_db::ChannelThread::create(tx, campfire_db::NewChannelThread {
                room_id: ALL_TALK, creator_id: DAVID, ..Default::default()
            })?.id)
        } else { None };
        let message = campfire_db::Message::create(tx, campfire_db::NewMessage {
            room_id: ALL_TALK, creator_id: DAVID, thread_id,
            markdown_source: Some("Keep me".into()), ..Default::default()
        })?;
        Ok((thread_id, message.id))
    }).await.unwrap();
    let base = thread_id.map_or_else(
        || format!("/rooms/{ALL_TALK}/messages"),
        |id| format!("/rooms/{ALL_TALK}/threads/{id}/messages"),
    );
    let path = if edit { format!("{base}/{message_id}") } else { base };
    let method = if edit { Method::PATCH } else { Method::POST };
    let before = write_counts(&a).await;
    for file in [&foreign, &used] {
        let reply = david.write(Req::new(method.clone(), &path)
            .header("accept", if !edit && !thread { "text/vnd.turbo-stream.html" } else { "application/json" })
            .header("content-type", "application/json")
            .body(json!({"message": {"attachment": file.signed_id, "markdown_source": "Changed"}}).to_string())).await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{path}: {}", reply.text());
        assert_eq!(write_counts(&a).await, before);
        a.db().read(move |conn| {
            let message = campfire_db::Message::find(conn, message_id)?;
            assert_eq!(message.markdown_source.as_deref(), Some("Keep me"));
            assert!(message.attachments(conn)?.is_empty());
            Ok(())
        }).await.unwrap();
    }
    let reply = david.write(Req::new(method, &path)
        .header("accept", if !edit && !thread { "text/vnd.turbo-stream.html" } else { "application/json" })
        .header("content-type", "application/json")
        .body(json!({"message": {"attachment": owned.signed_id, "markdown_source": "Owned"}}).to_string())).await;
    assert!(reply.status.is_success(), "{}", reply.text());
    let owned_id = blob_id(&a, &owned);
    assert_eq!(a.db().read(move |conn| Ok(conn.query_row(
        "SELECT COUNT(*) FROM active_storage_attachments WHERE blob_id=?", [owned_id], |row| row.get::<_, i64>(0)
    )?)).await.unwrap(), 1);
}

#[tokio::test]
async fn grouped_files_round3_legacy_room_posts_claim_uploads() {
    legacy_attachment_claims(false, false).await;
}

#[tokio::test]
async fn grouped_files_round3_legacy_thread_posts_claim_uploads() {
    legacy_attachment_claims(true, false).await;
}

#[tokio::test]
async fn grouped_files_round3_legacy_room_edits_claim_uploads() {
    legacy_attachment_claims(false, true).await;
}

#[tokio::test]
async fn grouped_files_round3_legacy_thread_edits_claim_uploads() {
    legacy_attachment_claims(true, true).await;
}

async fn agent_attachment_claims(bot_key: bool, thread: bool) {
    use crate::controllers::agent_http_tests::{AGENT, SECRET, initialize};
    use crate::controllers::presenters::test_support::{BENDER, BENDER_KEY};
    let a = app(true).await.expect("restored default seed").without_job_runner().await;
    initialize(&a).await;
    a.db().write(|tx| {
        tx.conn().execute("INSERT INTO agent_grants(agent_id,capability,granted_by_id,created_at,updated_at) VALUES(?,'post_messages',?,?,?)",
            rusqlite::params![AGENT, DAVID, tx.now(), tx.now()])?;
        Ok(())
    }).await.unwrap();
    let mut david = a.sign_in(DAVID).await;
    let foreign = upload(&mut david, "foreign.txt").await;
    let owned = upload(&mut david, "agent-owned.txt").await;
    let owned_id = blob_id(&a, &owned);
    a.db().write(move |tx| {
        tx.conn().execute("UPDATE active_storage_blobs SET metadata=json_set(metadata,'$.uploader_id',?) WHERE id=?",
            [BENDER, owned_id])?;
        Ok(())
    }).await.unwrap();
    let path = if bot_key { format!("/rooms/{ALL_TALK}/{BENDER_KEY}/messages") }
        else { format!("/rooms/{ALL_TALK}/agents/messages") };
    let thread_id = if thread {
        Some(a.db().write(|tx| Ok(campfire_db::ChannelThread::create(tx, campfire_db::NewChannelThread {
            room_id: ALL_TALK, creator_id: DAVID, ..Default::default()
        })?.id)).await.unwrap())
    } else { None };
    let mut client = a.anonymous();
    for (file, status) in [(&foreign, StatusCode::UNPROCESSABLE_ENTITY), (&owned, StatusCode::CREATED), (&owned, StatusCode::UNPROCESSABLE_ENTITY)] {
        let before = write_counts(&a).await;
        let body = if bot_key { json!({"attachment": file.signed_id}) }
            else { json!({"thread_id": thread_id, "message": {"attachment": file.signed_id}}) };
        let mut request = json_body(Method::POST, &path, &body);
        if !bot_key { request = request.header("authorization", &format!("Bearer {SECRET}")); }
        let reply = client.send(request).await;
        assert_eq!(reply.status, status, "{path}: {}", reply.text());
        if !status.is_success() { assert_eq!(write_counts(&a).await, before); }
    }
}

#[tokio::test]
async fn grouped_files_round3_bot_posts_claim_uploads() {
    agent_attachment_claims(true, false).await;
}

#[tokio::test]
async fn grouped_files_round3_agent_token_posts_claim_uploads() {
    agent_attachment_claims(false, false).await;
}

#[tokio::test]
async fn grouped_files_round3_agent_token_thread_posts_claim_uploads() {
    agent_attachment_claims(false, true).await;
}

#[tokio::test]
async fn grouped_files_round3_bot_edits_claim_uploads() {
    use crate::controllers::presenters::test_support::{BENDER, BENDER_KEY};
    let a = app(true).await.expect("restored default seed").without_job_runner().await;
    let mut david = a.sign_in(DAVID).await;
    let file = upload(&mut david, "foreign.txt").await;
    let message = a.db().write(|tx| campfire_db::Message::create(tx, campfire_db::NewMessage {
        room_id: ALL_TALK, creator_id: BENDER, body: Some("Keep me".into()), ..Default::default()
    })).await.unwrap();
    let before = write_counts(&a).await;
    let reply = a.anonymous().send(json_body(Method::PATCH,
        &format!("/rooms/{ALL_TALK}/{BENDER_KEY}/messages/{}", message.id),
        &json!({"attachment": file.signed_id}))).await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", reply.text());
    assert_eq!(write_counts(&a).await, before);
    a.db().read(move |conn| {
        assert!(campfire_db::Message::find(conn, message.id)?.attachments(conn)?.is_empty());
        Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn grouped_files_round3_legacy_thread_openers_claim_uploads() {
    let a = app(true).await.expect("restored default seed").without_job_runner().await;
    let file = upload(&mut a.sign_in(KEVIN).await, "foreign.txt").await;
    let before = write_counts(&a).await;
    let reply = a.sign_in(DAVID).await.write(json_body(Method::POST,
        &format!("/rooms/{ALL_TALK}/threads"),
        &json!({"channel_thread": {"name": "Files"}, "message": {"attachment": file.signed_id}}))).await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", reply.text());
    assert_eq!(write_counts(&a).await, before, "the rejected opener must roll back its thread");
}

#[tokio::test]
async fn grouped_files_round3_forwards_copy_visible_foreign_owned_files() {
    let a = app(true).await.expect("restored default seed").without_job_runner().await;
    let mut kevin = a.sign_in(KEVIN).await;
    let file = upload(&mut kevin, "kevin.txt").await;
    let source = kevin.write(json_body(Method::POST, &format!("/api/v1/rooms/{HQ}/messages"),
        &json!({"clientMessageId": "foreign-forward-source", "attachmentSignedIds": [file.signed_id]}))).await;
    assert_eq!(source.status, StatusCode::CREATED, "{}", source.text());
    let source_id = parse::<api::MessageDTO>(&source).id;
    let mut david = a.sign_in(DAVID).await;
    let path = format!("/api/v1/messages/{source_id}/forwards");
    let body = json!({"destinations": [{"roomId": ALL_TALK}]});
    let forward = david.write(json_body(Method::POST, &path, &body)).await;
    assert_eq!(forward.status, StatusCode::CREATED, "{}", forward.text());
    let copy_id = parse::<api::ForwardResult>(&forward).forwards[0].id;
    a.db().read(move |conn| {
        let source = campfire_db::Message::find(conn, source_id)?.attachments(conn)?;
        let copied = campfire_db::Message::find(conn, copy_id)?.attachments(conn)?;
        assert_ne!(source[0].1.id, copied[0].1.id);
        assert_ne!(source[0].1.key, copied[0].1.key);
        assert_eq!(copied[0].1.filename, source[0].1.filename);
        Ok(())
    }).await.unwrap();
    a.db().write(|tx| {
        tx.conn().execute("DELETE FROM memberships WHERE room_id=? AND user_id=?", [HQ, DAVID])?;
        Ok(())
    }).await.unwrap();
    let before = write_counts(&a).await;
    let denied = david.write(json_body(Method::POST, &path, &body)).await;
    assert_eq!(denied.status, StatusCode::NOT_FOUND);
    assert_eq!(write_counts(&a).await, before);
}

#[tokio::test]
async fn grouped_files_round3_schedules_do_not_claim_or_dispatch_signed_uploads() {
    let a = app(true).await.expect("restored default seed").without_job_runner().await;
    let mut david = a.sign_in(DAVID).await;
    let file = upload(&mut a.sign_in(KEVIN).await, "foreign.txt").await;
    let file_id = blob_id(&a, &file);
    let scheduled = david.write(json_body(Method::POST, &format!("/api/v1/rooms/{ALL_TALK}/scheduled_messages"),
        &json!({"markdownSource": "Text only", "sendAt": "2026-03-03T16:00:00Z",
            "attachmentSignedId": file.signed_id, "attachmentSignedIds": [file.signed_id]}))).await;
    assert_eq!(scheduled.status, StatusCode::CREATED, "{}", scheduled.text());
    let id = parse::<api::ScheduledMessage>(&scheduled).id;
    let edit = david.write(json_body(Method::PATCH, &format!("/api/v1/scheduled_messages/{id}"),
        &json!({"markdownSource": "Still text", "attachmentSignedId": file.signed_id,
            "attachmentSignedIds": [file.signed_id]}))).await;
    assert_eq!(edit.status, StatusCode::OK, "{}", edit.text());
    let sent = david.write(json_body(Method::POST, &format!("/api/v1/scheduled_messages/{id}/send_now"), &json!({}))).await;
    assert_eq!(sent.status, StatusCode::OK, "{}", sent.text());
    a.db().read(move |conn| {
        let scheduled = campfire_db::ScheduledMessage::find(conn, id)?;
        let message = campfire_db::Message::find(conn, scheduled.sent_message_id.unwrap())?;
        assert_eq!(message.markdown_source.as_deref(), Some("Still text"));
        assert!(message.attachments(conn)?.is_empty());
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM active_storage_attachments WHERE blob_id=?", [file_id], |row| row.get::<_, i64>(0))?, 0);
        Ok(())
    }).await.unwrap();
}

async fn upload(b: &mut Browser<'_>, filename: &str) -> api::DirectUpload {
    let reply = b
        .write(json_body(
            Method::POST,
            "/api/v1/uploads",
            &json!({
                "filename": filename, "byteSize": 5, "checksum": "XUFAKrxLKna5cZ2REBfFkg==",
                "contentType": "text/plain",
            }),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let upload: api::DirectUpload = parse(&reply);
    let reply = b
        .send(
            Req::new(Method::PUT, &upload.upload_url)
                .header("content-type", "text/plain")
                .header("content-length", "5")
                .body("hello"),
        )
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.text());
    upload
}

fn file_posts(client: &str, files: Value) -> [(String, Value); 3] {
    let mut message = files;
    message["clientMessageId"] = json!(client);
    [
        (
            format!("/api/v1/rooms/{ALL_TALK}/messages"),
            message.clone(),
        ),
        (
            "/api/v1/rooms/654632876/threads".into(),
            json!({"parentMessageId": 935962057, "message": message}),
        ),
        (
            "/api/v1/rooms/699448332/posts".into(),
            json!({"name": "Files", "status": "planned", "tags": [], "message": message}),
        ),
    ]
}

async fn write_counts(a: &TestApp) -> (i64, i64, i64) {
    a.db().read(|conn| {
        Ok(conn.query_row(
            "SELECT (SELECT COUNT(*) FROM messages), (SELECT COUNT(*) FROM channel_threads), (SELECT COUNT(*) FROM active_storage_attachments)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?)
    }).await.unwrap()
}

#[tokio::test]
async fn grouped_files_round6_enforce_current_per_file_limit_before_any_write() {
    const MB: usize = 1024 * 1024;
    let a = app(true).await.expect("restored default seed").without_job_runner().await;
    let mut david = a.sign_in(DAVID).await;
    let small = upload(&mut david, "small.txt").await;
    let bytes = vec![0; 2 * MB];
    let created = david.write(json_body(Method::POST, "/api/v1/uploads", &json!({
        "filename": "large.bin", "byteSize": bytes.len(),
        "checksum": campfire_storage::key::checksum(&bytes),
        "contentType": "application/octet-stream",
    }))).await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.text());
    let large: api::DirectUpload = parse(&created);
    let received = david.send(Req::new(Method::PUT, &large.upload_url)
        .header("content-type", "application/octet-stream")
        .header("content-length", &bytes.len().to_string())
        .body(bytes)).await;
    assert_eq!(received.status, StatusCode::NO_CONTENT, "{}", received.text());
    let lowered = david.write(json_body(Method::PATCH, "/api/v1/admin/workspace",
        &json!({"uploadLimitBytes": MB}))).await;
    assert_eq!(lowered.status, StatusCode::OK, "{}", lowered.text());
    let before = write_counts(&a).await;
    let path = format!("/api/v1/rooms/{ALL_TALK}/messages");
    let single = david.write(json_body(Method::POST, &path, &json!({
        "clientMessageId": "size-limit-single", "attachmentSignedId": large.signed_id,
    }))).await;
    assert_eq!(single.status, StatusCode::PAYLOAD_TOO_LARGE, "{}", single.text());
    assert_eq!(write_counts(&a).await, before);
    for (path, body) in file_posts("size-limit-grouped", json!({
        "attachmentSignedIds": [small.signed_id, large.signed_id],
    })) {
        let reply = david.write(json_body(Method::POST, &path, &body)).await;
        assert_eq!(reply.status, StatusCode::PAYLOAD_TOO_LARGE, "{path}: {}", reply.text());
        assert_eq!(write_counts(&a).await, before, "{path} must not claim either upload");
    }
    let raised = david.write(json_body(Method::PATCH, "/api/v1/admin/workspace",
        &json!({"uploadLimitBytes": 2 * MB}))).await;
    assert_eq!(raised.status, StatusCode::OK, "{}", raised.text());
    let accepted = david.write(json_body(Method::POST, &path, &json!({
        "clientMessageId": "size-limit-grouped",
        "attachmentSignedIds": [small.signed_id, large.signed_id],
    }))).await;
    assert_eq!(accepted.status, StatusCode::CREATED, "{}", accepted.text());
    assert_eq!(parse::<api::MessageDTO>(&accepted).attachments.unwrap().len(), 2);
}

#[tokio::test]
async fn grouped_files_reject_other_uploaders_without_claiming_owned_files() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let foreign = upload(&mut kevin, "foreign.txt").await;
    let owned = upload(&mut david, "owned.txt").await;
    let before = write_counts(&a).await;
    for files in [
        json!({"attachmentSignedId": foreign.signed_id}),
        json!({"attachmentSignedIds": [owned.signed_id, foreign.signed_id]}),
    ] {
        for (path, body) in file_posts("foreign-upload", files.clone()) {
            let reply = david.write(json_body(Method::POST, &path, &body)).await;
            assert_eq!(
                reply.status,
                StatusCode::UNPROCESSABLE_ENTITY,
                "{path}: {}",
                reply.text()
            );
            assert_eq!(tag(&reply), "Validation");
            let field = if files.get("attachmentSignedId").is_some() { "attachmentSignedId" } else { "attachmentSignedIds" };
            assert!(parse::<Value>(&reply)["error"]["fields"][field].is_array());
            assert_eq!(write_counts(&a).await, before, "{path} must roll back");
        }
    }
    for (b, signed_id, client) in [
        (&mut david, owned.signed_id, "owned-upload"),
        (&mut kevin, foreign.signed_id, "foreign-owner"),
    ] {
        let reply = b
            .write(json_body(
                Method::POST,
                &format!("/api/v1/rooms/{HQ}/messages"),
                &json!({"clientMessageId": client, "attachmentSignedIds": [signed_id]}),
            ))
            .await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    }
}

#[tokio::test]
async fn grouped_files_reject_already_attached_uploads_in_both_slots() {
    let Some(a) = app(true).await else { return };
    let mut b = a.sign_in(DAVID).await;
    for slot in ["attachmentSignedId", "attachmentSignedIds"] {
        let used = upload(&mut b, "used.txt").await;
        let fresh = upload(&mut b, "fresh.txt").await;
        let mut body = json!({"clientMessageId": format!("first-{slot}")});
        body[slot] = if slot == "attachmentSignedId" {
            json!(used.signed_id)
        } else {
            json!([used.signed_id])
        };
        let reply = b
            .write(json_body(
                Method::POST,
                &format!("/api/v1/rooms/{HQ}/messages"),
                &body,
            ))
            .await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
        let before = write_counts(&a).await;
        for files in [
            json!({"attachmentSignedId": used.signed_id}),
            json!({"attachmentSignedIds": [fresh.signed_id, used.signed_id]}),
        ] {
            for (path, body) in file_posts(&format!("reuse-{slot}"), files.clone()) {
                let reply = b.write(json_body(Method::POST, &path, &body)).await;
                assert_eq!(
                    reply.status,
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "{path}: {}",
                    reply.text()
                );
                assert_eq!(tag(&reply), "Validation");
                assert!(reply.text().contains("already attached"), "{}", reply.text());
                assert_eq!(write_counts(&a).await, before, "{path} must roll back");
            }
        }
        let reply = b.write(json_body(Method::POST, &format!("/api/v1/rooms/{HQ}/messages"),
            &json!({"clientMessageId": format!("fresh-{slot}"), "attachmentSignedIds": [fresh.signed_id]}))).await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    }
}

#[tokio::test]
async fn grouped_files_round7_legacy_signed_blobs_remain_shareable() {
    let a = app(true).await.expect("restored default seed").without_job_runner().await;
    let mut david = a.sign_in(DAVID).await;
    let file = upload(&mut david, "legacy.txt").await;
    let file_id = blob_id(&a, &file);
    let original_id = a.db().write(move |tx| {
        tx.conn().execute("UPDATE active_storage_blobs SET metadata='{}' WHERE id=?", [file_id])?;
        let message = campfire_db::Message::create(tx, campfire_db::NewMessage {
            room_id: ALL_TALK, creator_id: DAVID, attachment_blob_id: Some(file_id),
            ..Default::default()
        })?;
        Ok(message.attachments(tx.conn())?[0].0.id)
    }).await.unwrap();
    let reply = david.write(Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/messages"))
        .header("accept", "text/vnd.turbo-stream.html")
        .header("content-type", "application/json")
        .body(json!({"message": {"attachment": file.signed_id}}).to_string())).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    a.db().read(move |conn| {
        let mut query = conn.prepare("SELECT id FROM active_storage_attachments WHERE blob_id=? ORDER BY id")?;
        let ids = query.query_map([file_id], |row| row.get::<_, i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0], original_id);
        Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn grouped_files_require_recorded_upload_ownership() {
    let Some(a) = app(true).await else { return };
    let mut b = a.sign_in(DAVID).await;
    let file = upload(&mut b, "unowned.txt").await;
    let id = campfire_storage::paths::verify_signed_blob_id(
        &*a.booted.app.storage.verifier,
        &file.signed_id,
        a.booted.app.clock.now(),
    )
    .unwrap();
    a.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE active_storage_blobs SET metadata='{}' WHERE id=?",
                [id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let before = write_counts(&a).await;
    for files in [
        json!({"attachmentSignedId": file.signed_id}),
        json!({"attachmentSignedIds": [file.signed_id]}),
    ] {
        for (path, body) in file_posts("unowned", files.clone()) {
            let reply = b.write(json_body(Method::POST, &path, &body)).await;
            assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{path}: {}", reply.text());
            assert_eq!(write_counts(&a).await, before);
        }
    }
}

#[tokio::test]
async fn grouped_files_record_authenticated_upload_owner_over_client_metadata() {
    let Some(a) = app(true).await else { return };
    let mut b = a.sign_in(KEVIN).await;
    let reply = b.write(json_body(Method::POST, "/rails/active_storage/direct_uploads", &json!({
        "blob": {"filename": "owned.txt", "byte_size": 5, "checksum": "XUFAKrxLKna5cZ2REBfFkg==",
            "content_type": "text/plain", "metadata": {"uploader_id": DAVID}},
    }))).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let file: Value = parse(&reply);
    let id = file["id"].as_i64().unwrap();
    let owner = a
        .db()
        .read(move |conn| {
            Ok(conn.query_row(
                "SELECT json_extract(metadata, '$.uploader_id') FROM active_storage_blobs WHERE id=?",
                [id],
                |row| row.get::<_, Option<i64>>(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(owner, Some(KEVIN));
    let reply = b.send(Req::new(Method::PUT, file["direct_upload"]["url"].as_str().unwrap())
        .header("content-type", "text/plain").header("content-length", "5").body("hello")).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.text());
    let body = json!({"clientMessageId": "metadata-owner", "attachmentSignedIds": [file["signed_id"]]});
    let path = format!("/api/v1/rooms/{HQ}/messages");
    let mut david = a.sign_in(DAVID).await;
    let reply = david.write(json_body(Method::POST, &path, &body)).await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", reply.text());
    let reply = b.write(json_body(Method::POST, &path, &body)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
}

#[tokio::test]
async fn grouped_files_claim_one_upload_once_when_distinct_posts_race() {
    let Some(a) = app(true).await else { return };
    let a = a.without_job_runner().await;
    let mut first = a.sign_in(DAVID).await;
    let mut second = a.sign_in(DAVID).await;
    first.authenticity_token().await;
    second.authenticity_token().await;
    let file = upload(&mut first, "raced.txt").await;
    let before = write_counts(&a).await;
    let (entered, ready) = tokio::sync::oneshot::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let db = a.db().clone();
    let blocker = tokio::spawn(async move {
        db.write(move |_| {
            entered.send(()).unwrap();
            wait.recv_timeout(Duration::from_secs(10)).unwrap();
            Ok(())
        })
        .await
        .unwrap();
    });
    ready.await.unwrap();
    let path = format!("/api/v1/rooms/{HQ}/messages");
    let (one, two, ()) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(
            first.write(json_body(Method::POST, &path,
                &json!({"clientMessageId": "upload-claim-race-1", "attachmentSignedId": file.signed_id}))),
            second.write(json_body(Method::POST, &path,
                &json!({"clientMessageId": "upload-claim-race-2", "attachmentSignedIds": [file.signed_id]}))),
            async {
                // Both requests have staged their files and queued their message writes.
                tokio::time::timeout(Duration::from_secs(5), async {
                    while a.db().queued_writes() < 2 {
                        tokio::task::yield_now().await;
                    }
                }).await.unwrap();
                assert_eq!(write_counts(&a).await, before);
                release.send(()).unwrap();
                blocker.await.unwrap();
            },
        )
    }).await.unwrap();
    let mut statuses = [one.status, two.status];
    statuses.sort();
    assert_eq!(
        statuses,
        [StatusCode::CREATED, StatusCode::UNPROCESSABLE_ENTITY],
        "{} / {}",
        one.text(),
        two.text()
    );
    assert_eq!(
        write_counts(&a).await,
        (before.0 + 1, before.1, before.2 + 1)
    );
}

#[tokio::test]
async fn grouped_files_retry_race_replays_the_claimed_upload() {
    let Some(a) = app(true).await else { return };
    let mut first = a.sign_in(DAVID).await;
    let mut second = a.sign_in(DAVID).await;
    first.authenticity_token().await;
    second.authenticity_token().await;
    let file = upload(&mut first, "retry.txt").await;
    campfire_api::test_hooks::hold_after_duplicate_check("upload-retry-race", 2);
    let before = write_counts(&a).await;
    let path = format!("/api/v1/rooms/{HQ}/messages");
    let body =
        json!({"clientMessageId": "upload-retry-race", "attachmentSignedIds": [file.signed_id]});
    let (one, two) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(
            first.write(json_body(Method::POST, &path, &body)),
            second.write(json_body(Method::POST, &path, &body)),
        )
    })
    .await
    .unwrap();
    let mut statuses = [one.status, two.status];
    statuses.sort();
    assert_eq!(
        statuses,
        [StatusCode::OK, StatusCode::CREATED],
        "{} / {}",
        one.text(),
        two.text()
    );
    assert_eq!(parse::<Value>(&one)["id"], parse::<Value>(&two)["id"]);
    assert_eq!(
        write_counts(&a).await,
        (before.0 + 1, before.1, before.2 + 1)
    );
}

#[tokio::test]
async fn grouped_files_post_read_retry_edit_and_delete() {
    let Some(a) = app(true).await else { return };
    let mut b = a.sign_in(DAVID).await;
    let (addr, server) = serve(&a).await;
    let mut sync = Sync::connect(addr, &b.cookie_header(), &[format!("room:{ALL_TALK}")]).await;
    sync.welcome().await;
    let first = upload(&mut b, "first.txt").await;
    let second = upload(&mut b, "second.txt").await;
    let path = format!("/api/v1/rooms/{ALL_TALK}/messages");
    let body = json!({"clientMessageId": "grouped", "markdownSource": "Two files",
        "attachmentSignedIds": [second.signed_id, first.signed_id]});
    let reply = b.write(json_body(Method::POST, &path, &body)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let posted: Value = parse(&reply);
    assert_eq!(posted["attachments"][0]["filename"], "second.txt");
    assert_eq!(posted["attachments"][1]["filename"], "first.txt");
    assert_eq!(posted["attachment"], posted["attachments"][0]);
    let event = sync.until(created_in(ALL_TALK), |_| false).await;
    let api::SyncPayload::MessageCreated(live) = event.payload else {
        unreachable!()
    };
    assert_eq!(
        serde_json::to_value(live).unwrap()["attachments"],
        posted["attachments"]
    );
    let id = posted["id"].as_i64().unwrap();
    let retry = b.write(json_body(Method::POST, &path, &body)).await;
    assert_eq!(retry.status, StatusCode::OK);
    assert_eq!(parse::<Value>(&retry)["attachments"], posted["attachments"]);
    let read: Value = parse(&b.send(get(&format!("/api/v1/messages/{id}"))).await);
    assert_eq!(read["message"]["attachments"], posted["attachments"]);
    let page: Value = parse(&b.send(get(&path)).await);
    assert_eq!(
        page["messages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["id"] == id)
            .unwrap()["attachments"],
        posted["attachments"]
    );
    let edit = b
        .write(json_body(
            Method::PATCH,
            &format!("/api/v1/messages/{id}"),
            &json!({"markdownSource": ""}),
        ))
        .await;
    assert_eq!(edit.status, StatusCode::OK, "{}", edit.text());
    assert_eq!(parse::<Value>(&edit)["attachments"], posted["attachments"]);
    let forward = b
        .write(json_body(
            Method::POST,
            &format!("/api/v1/messages/{id}/forwards"),
            &json!({"destinations": [{"roomId": HQ, "threadId": null}]}),
        ))
        .await;
    assert_eq!(forward.status, StatusCode::CREATED, "{}", forward.text());
    let forwarded: api::ForwardResult = parse(&forward);
    let forwarded_id = forwarded.forwards[0].id;
    let read: Value = parse(
        &b.send(get(&format!("/api/v1/messages/{forwarded_id}")))
            .await,
    );
    assert_eq!(read["message"]["attachments"][0]["filename"], "second.txt");
    assert_eq!(read["message"]["attachments"][1]["filename"], "first.txt");
    let deleted = b
        .write(json_body(
            Method::DELETE,
            &format!("/api/v1/messages/{id}"),
            &json!({}),
        ))
        .await;
    assert_eq!(deleted.status, StatusCode::NO_CONTENT, "{}", deleted.text());
    assert_eq!(a.db().read(move |conn| Ok(conn.query_row("SELECT COUNT(*) FROM active_storage_attachments WHERE record_type='Message' AND record_id=?", [id], |row| row.get::<_, i64>(0))?)).await.unwrap(), 0);
    let read: Value = parse(
        &b.send(get(&format!("/api/v1/messages/{forwarded_id}")))
            .await,
    );
    for file in read["message"]["attachments"].as_array().unwrap() {
        let reply = b.send(get(file["url"].as_str().unwrap())).await;
        assert!(reply.status.is_redirection(), "{}", reply.text());
    }
    server.abort();
}

#[tokio::test]
async fn grouped_files_are_kept_by_thread_and_board_openers() {
    let Some(a) = app(true).await else { return };
    let mut b = a.sign_in(DAVID).await;
    for (path, body) in [
        (
            "/api/v1/rooms/654632876/threads",
            json!({"parentMessageId": 935962057, "message": {"clientMessageId": "grouped-opener"}}),
        ),
        (
            "/api/v1/rooms/699448332/posts",
            json!({"name": "Files", "status": "planned", "tags": [], "message": {"clientMessageId": "grouped-board"}}),
        ),
    ] {
        let first = upload(&mut b, "first.txt").await;
        let second = upload(&mut b, "second.txt").await;
        let mut body = body;
        body["message"]["attachmentSignedIds"] = json!([first.signed_id, second.signed_id]);
        let reply = b.write(json_body(Method::POST, path, &body)).await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
        let thread_id = if path.ends_with("/threads") {
            parse::<api::ThreadCreated>(&reply).detail.thread.id
        } else {
            parse::<api::ThreadDetail>(&reply).thread.id
        };
        let page: Value = parse(
            &b.send(get(&format!("/api/v1/threads/{thread_id}/messages")))
                .await,
        );
        assert_eq!(
            page["messages"][0]["attachments"][0]["filename"],
            "first.txt"
        );
        assert_eq!(
            page["messages"][0]["attachments"][1]["filename"],
            "second.txt"
        );
    }
}

#[tokio::test]
async fn grouped_files_read_existing_slots_in_attachment_id_order_and_keep_shared_blobs() {
    let Some(a) = app(true).await else { return };
    let a = a.without_job_runner().await;
    let mut b = a.sign_in(DAVID).await;
    let first = upload(&mut b, "first.txt").await;
    let second = upload(&mut b, "second.txt").await;
    let verifier = &*a.booted.app.storage.verifier;
    let now = a.booted.app.clock.now();
    let first_id =
        campfire_storage::paths::verify_signed_blob_id(verifier, &first.signed_id, now).unwrap();
    let second_id =
        campfire_storage::paths::verify_signed_blob_id(verifier, &second.signed_id, now).unwrap();
    let (id, shared_id) = a
        .db()
        .write(move |tx| {
            let message = campfire_db::Message::create(
                tx,
                campfire_db::NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    markdown_source: Some("Files".into()),
                    ..Default::default()
                },
            )?;
            campfire_db::Attachment::create(tx, "Message", message.id, "attachments", second_id)?;
            campfire_db::Attachment::create(tx, "Message", message.id, "attachment", first_id)?;
            let shared = campfire_db::Message::create(
                tx,
                campfire_db::NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    attachment_blob_id: Some(second_id),
                    ..Default::default()
                },
            )?;
            assert!(campfire_db::models::message_attachment_processing::owns(
                tx.conn(),
                message.id,
                second_id
            )?);
            campfire_db::models::message_attachment_processing::schedule_message(tx, &message)?;
            Ok((message.id, shared.id))
        })
        .await
        .unwrap();
    let read: Value = parse(&b.send(get(&format!("/api/v1/messages/{id}"))).await);
    assert_eq!(read["message"]["attachments"][0]["filename"], "second.txt");
    assert_eq!(read["message"]["attachments"][1]["filename"], "first.txt");
    let files: Value = parse(
        &b.send(get(&format!(
            "/api/v1/rooms/{ALL_TALK}/files?filename=first.txt"
        )))
        .await,
    );
    assert_eq!(files["files"][0]["attachment"]["filename"], "first.txt");
    let removed = b
        .write(json_body(
            Method::DELETE,
            &format!("/api/v1/messages/{id}"),
            &json!({}),
        ))
        .await;
    assert_eq!(removed.status, StatusCode::NO_CONTENT);
    crate::active_storage::purge(&a.booted.app, first_id)
        .await
        .unwrap();
    crate::active_storage::purge(&a.booted.app, second_id)
        .await
        .unwrap();
    assert_eq!(
        a.db()
            .read(move |conn| Ok((
                campfire_storage::Blob::find(conn, first_id)
                    .unwrap()
                    .is_none(),
                campfire_storage::Blob::find(conn, second_id)
                    .unwrap()
                    .is_some()
            )))
            .await
            .unwrap(),
        (true, true)
    );
    let shared: api::MessageRead =
        parse(&b.send(get(&format!("/api/v1/messages/{shared_id}"))).await);
    assert_eq!(shared.message.attachment.unwrap().filename, "second.txt");
}

#[tokio::test]
async fn grouped_files_validate_every_file_and_the_ten_file_cap_atomically() {
    let Some(a) = app(true).await else { return };
    let mut b = a.sign_in(DAVID).await;
    let upload = upload(&mut b, "valid.txt").await;
    let path = format!("/api/v1/rooms/{ALL_TALK}/messages");
    for (client, ids, single) in [
        ("over-cap", vec![upload.signed_id.clone(); 11], None),
        (
            "invalid",
            vec![upload.signed_id.clone(), "forged".into()],
            None,
        ),
        ("duplicate", vec![upload.signed_id.clone(); 2], None),
        (
            "mixed",
            vec![upload.signed_id.clone()],
            Some(upload.signed_id.clone()),
        ),
    ] {
        let reply = b
            .write(json_body(
                Method::POST,
                &path,
                &json!({
                    "clientMessageId": client, "markdownSource": "Files",
                    "attachmentSignedIds": ids, "attachmentSignedId": single,
                }),
            ))
            .await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{client}: {}",
            reply.text()
        );
        let error: Value = parse(&reply);
        assert!(
            error["error"]["fields"]["attachmentSignedIds"].is_array(),
            "{error}"
        );
        if client == "over-cap" {
            assert!(reply.text().contains("10"));
        }
        let client = client.to_string();
        assert!(
            a.db()
                .read(move |conn| campfire_db::Message::find_duplicate(
                    conn, ALL_TALK, DAVID, &client
                ))
                .await
                .unwrap()
                .is_none()
        );
    }
    let reply = b
        .write(json_body(
            Method::POST,
            &path,
            &json!({
                "clientMessageId": "no-text", "attachmentSignedIds": [upload.signed_id],
            }),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let mut ten = Vec::new();
    for index in 0..10 {
        ten.push(
            self::upload(&mut b, &format!("{index}.txt"))
                .await
                .signed_id,
        );
    }
    let reply = b
        .write(json_body(
            Method::POST,
            &path,
            &json!({"clientMessageId": "ten-files", "attachmentSignedIds": ten}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let posted: Value = parse(&reply);
    assert_eq!(posted["attachments"].as_array().unwrap().len(), 10);
    let unfinished: api::DirectUpload = parse(&b.write(json_body(Method::POST, "/api/v1/uploads", &json!({"filename": "pending.txt", "byteSize": 5, "checksum": "XUFAKrxLKna5cZ2REBfFkg==", "contentType": "text/plain"}))).await);
    let reply = b.write(json_body(Method::POST, &path, &json!({"clientMessageId": "unfinished", "markdownSource": "Files", "attachmentSignedIds": [unfinished.signed_id]}))).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.text()
    );
    assert!(reply.text().contains("finished"));
}

#[tokio::test]
async fn grouped_files_keep_room_thread_and_delete_permissions() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let upload = upload(&mut david, "thread.txt").await;
    let body = json!({"clientMessageId": "grouped-thread", "markdownSource": "Files", "attachmentSignedIds": [upload.signed_id]});
    let denied = kevin
        .write(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{ALL_PETS}/messages"),
            &body,
        ))
        .await;
    assert_eq!(denied.status, StatusCode::NOT_FOUND);
    let private = david
        .write(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{ALL_PETS}/messages"),
            &body,
        ))
        .await;
    assert_eq!(private.status, StatusCode::CREATED, "{}", private.text());
    let private: Value = parse(&private);
    let private_id = private["id"].as_i64().unwrap();
    let denied = kevin
        .send(get(&format!("/api/v1/messages/{private_id}")))
        .await;
    assert_eq!(denied.status, StatusCode::NOT_FOUND);
    let thread_upload = self::upload(&mut david, "thread.txt").await;
    let mut body = body;
    body["attachmentSignedIds"] = json!([thread_upload.signed_id]);
    let path = "/api/v1/threads/1/messages";
    let reply = david.write(json_body(Method::POST, path, &body)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let posted: Value = parse(&reply);
    assert_eq!(posted["attachments"][0]["filename"], "thread.txt");
    let id = posted["id"].as_i64().unwrap();
    let denied = kevin
        .write(json_body(
            Method::DELETE,
            &format!("/api/v1/messages/{id}"),
            &json!({}),
        ))
        .await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE channel_threads SET locked_at=? WHERE id=1",
                [tx.now()],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut locked_body = body;
    let locked_upload = self::upload(&mut david, "locked.txt").await;
    locked_body["clientMessageId"] = json!("locked-files");
    locked_body["attachmentSignedIds"] = json!([locked_upload.signed_id]);
    let locked = david
        .write(json_body(Method::POST, path, &locked_body))
        .await;
    assert_eq!(locked.status, StatusCode::FORBIDDEN);
}
