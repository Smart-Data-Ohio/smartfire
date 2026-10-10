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
    let reply = b
        .write(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{HQ}/messages"),
            &json!({"clientMessageId": "unowned", "attachmentSignedIds": [file.signed_id]}),
        ))
        .await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.text()
    );
    assert_eq!(write_counts(&a).await, before);
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
