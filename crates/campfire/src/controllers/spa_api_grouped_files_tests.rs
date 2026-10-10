use super::*;

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
    let first = upload(&mut b, "first.txt").await;
    let second = upload(&mut b, "second.txt").await;
    let files = json!([first.signed_id, second.signed_id]);
    for (path, body) in [
        (
            "/api/v1/rooms/654632876/threads",
            json!({"parentMessageId": 935962057, "message": {"clientMessageId": "grouped-opener", "attachmentSignedIds": files}}),
        ),
        (
            "/api/v1/rooms/699448332/posts",
            json!({"name": "Files", "status": "planned", "tags": [], "message": {"clientMessageId": "grouped-board", "attachmentSignedIds": files}}),
        ),
    ] {
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
    locked_body["clientMessageId"] = json!("locked-files");
    let locked = david
        .write(json_body(Method::POST, path, &locked_body))
        .await;
    assert_eq!(locked.status, StatusCode::FORBIDDEN);
}
