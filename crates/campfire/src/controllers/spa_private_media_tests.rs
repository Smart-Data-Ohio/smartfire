use super::api_tests::{app, get, json_body, parse};
use crate::controllers::presenters::test_support::{BENDER_KEY, Browser, DAVID, KEVIN, Req};
use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use serde_json::json;

const DESIGNERS: i64 = 654632876;
const JZ: i64 = 773523953;

#[tokio::test]
async fn scheduled_files_private_media_requires_scheduler_and_current_thread_access() {
    let a = app(true)
        .await
        .expect("restored default seed")
        .without_job_runner()
        .await;
    let mut scheduler = a.sign_in(KEVIN).await;
    let mut reader = a.sign_in(JZ).await;
    let original = upload(&mut scheduler, "scheduled-private.png").await;
    let derivative = upload(&mut scheduler, "scheduled-private-variant.txt").await;
    let verifier = &*a.booted.app.storage.verifier;
    let clock = a.booted.app.clock.now();
    let original_id =
        campfire_storage::paths::verify_signed_blob_id(verifier, &original.signed_id, clock)
            .unwrap();
    let child_id =
        campfire_storage::paths::verify_signed_blob_id(verifier, &derivative.signed_id, clock)
            .unwrap();
    let thread_id = a.db().write(move |tx| {
        let thread = campfire_db::ChannelThread::create(tx, campfire_db::NewChannelThread {
            room_id: DESIGNERS, creator_id: KEVIN, name: Some("Scheduled private files".into()),
            visibility: campfire_db::channel_thread::ThreadVisibility::Private, ..Default::default()
        })?;
        thread.add_access_member(tx, KEVIN, JZ)?;
        tx.conn().execute("INSERT INTO active_storage_variant_records(blob_id,variation_digest) VALUES(?,'scheduled-private-test')", [original_id])?;
        campfire_db::Attachment::create(tx, "ActiveStorage::VariantRecord", tx.conn().last_insert_rowid(), "image", child_id)?;
        Ok(thread.id)
    }).await.unwrap();
    let paths = a
        .db()
        .read(move |conn| {
            [original_id, child_id]
                .into_iter()
                .map(|id| {
                    campfire_storage::Blob::find(conn, id)
                        .map_err(|error| campfire_db::Error::Other(error.to_string()))?
                        .ok_or(campfire_db::Error::RecordNotFound("Blob"))
                })
                .collect::<campfire_db::Result<Vec<_>>>()
        })
        .await
        .unwrap()
        .iter()
        .map(|blob| campfire_storage::paths::blob_proxy_path(verifier, blob, None))
        .collect::<Vec<_>>();
    let response = scheduler.write(json_body(Method::POST, &format!("/api/v1/rooms/{DESIGNERS}/scheduled_messages"),
        &json!({"markdownSource": "Scheduled private file", "sendAt": "2026-03-03T16:00:00Z", "threadId": thread_id, "attachmentSignedIds": [original.signed_id]}))).await;
    assert_eq!(response.status, StatusCode::CREATED, "{}", response.text());
    let scheduled: api::ScheduledMessage = parse(&response);
    for path in &paths {
        assert_eq!(reader.send(get(path)).await.status, StatusCode::NOT_FOUND);
        let allowed = scheduler.send(get(path)).await;
        assert_eq!(allowed.status, StatusCode::OK);
        assert_eq!(allowed.header("cache-control"), Some("private, no-store"));
    }
    let sent = scheduler
        .write(json_body(
            Method::POST,
            &format!("/api/v1/scheduled_messages/{}/send_now", scheduled.id),
            &json!({}),
        ))
        .await;
    assert_eq!(sent.status, StatusCode::OK, "{}", sent.text());
    let sent: api::ScheduledMessage = parse(&sent);
    assert!(sent.sent_message_id.is_some());
    for path in &paths {
        let allowed = reader.send(get(path)).await;
        assert_eq!(allowed.status, StatusCode::OK);
        assert_eq!(allowed.header("cache-control"), Some("private, no-store"));
    }
    a.db().write(move |tx| {
        tx.conn().execute("UPDATE active_storage_blobs SET metadata=json_set(metadata,'$.scheduled_by',?) WHERE id=?", [KEVIN, original_id])?;
        for id in [original_id, child_id] {
            let blob = campfire_storage::Blob::find(tx.conn(), id)
                .map_err(|error| campfire_db::Error::Other(error.to_string()))?
                .ok_or(campfire_db::Error::RecordNotFound("Blob"))?;
            let error = crate::controllers::messages::attachment_blob(tx,
                campfire_runtime::presenters::attachments::Assignment::Existing(blob.clone()),
                crate::controllers::messages::AttachmentPolicy::SignedBlob { actor_id: Some(KEVIN) }, "attachment").unwrap_err();
            assert!(matches!(error, campfire_db::Error::RecordInvalid(errors) if errors.on("attachment") == ["includes a private scheduled upload"]));
            let error = campfire_runtime::presenters::attachments::attach_existing(tx,
                campfire_runtime::presenters::attachments::Record::user(KEVIN), "avatar", blob).unwrap_err();
            assert!(matches!(error, campfire_db::Error::RecordInvalid(errors) if errors.on("attachment") == ["is inaccessible"]));
        }
        Ok(())
    }).await.unwrap();
    for path in &paths {
        assert_eq!(
            reader.send(get(path)).await.status,
            StatusCode::NOT_FOUND,
            "thread access cannot bypass scheduler ownership"
        );
        assert_eq!(scheduler.send(get(path)).await.status, StatusCode::OK);
    }
    a.db()
        .write(move |tx| {
            campfire_db::ChannelThread::find(tx.conn(), thread_id)?
                .remove_access_member(tx, KEVIN, KEVIN)
        })
        .await
        .unwrap();
    for path in &paths {
        assert_eq!(
            scheduler.send(get(path)).await.status,
            StatusCode::NOT_FOUND,
            "scheduler ownership cannot bypass revoked thread access"
        );
    }
    a.db().write(move |tx| {
        tx.conn().execute("UPDATE active_storage_blobs SET metadata=json_remove(metadata,'$.scheduled_by') WHERE id=?", [original_id])?;
        Ok(())
    }).await.unwrap();
    for path in &paths {
        assert_eq!(reader.send(get(path)).await.status, StatusCode::OK);
        assert_eq!(
            scheduler.send(get(path)).await.status,
            StatusCode::NOT_FOUND
        );
    }
}

async fn upload(browser: &mut Browser<'_>, name: &str) -> api::DirectUpload {
    let bytes: &[u8] = if name.ends_with(".png") {
        include_bytes!("../../../../fixtures/files/workspace_icons/square_64.png")
    } else {
        b"meeting notes"
    };
    let content_type = if name.ends_with(".png") {
        "image/png"
    } else {
        "text/plain"
    };
    let reply = browser.write(json_body(Method::POST, "/api/v1/uploads", &json!({"filename": name, "byteSize": bytes.len(), "contentType": content_type, "checksum": campfire_storage::key::checksum(bytes)}))).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let upload: api::DirectUpload = parse(&reply);
    let written = browser
        .send(
            Req::new(Method::PUT, &upload.upload_url)
                .header("content-type", content_type)
                .header("content-length", &bytes.len().to_string())
                .body(bytes),
        )
        .await;
    assert_eq!(written.status, StatusCode::NO_CONTENT);
    upload
}

#[tokio::test]
async fn private_thread_media_urls_and_signed_reuse_require_current_access_even_after_purge() {
    let a = app(true)
        .await
        .expect("restored default seed")
        .without_job_runner()
        .await;
    let mut creator = a.sign_in(KEVIN).await;
    let mut outsider = a.sign_in(JZ).await;
    let mut admin = a.sign_in(DAVID).await;
    let mut anonymous = a.anonymous();
    let original = upload(&mut creator, "private.png").await;
    let derivative = upload(&mut creator, "private-variant.txt").await;
    let preview = upload(&mut creator, "private-preview.txt").await;
    let nested = upload(&mut creator, "private-preview-variant.txt").await;
    let verifier = &*a.booted.app.storage.verifier;
    let now = a.booted.app.clock.now();
    let ids: Vec<_> = [&original, &derivative, &preview, &nested]
        .into_iter()
        .map(|upload| {
            campfire_storage::paths::verify_signed_blob_id(verifier, &upload.signed_id, now)
                .unwrap()
        })
        .collect();
    let (original_id, derivative_id, preview_id, nested_id) = (ids[0], ids[1], ids[2], ids[3]);
    let (thread_id, message_id) = a.db().write(move |tx| {
        let thread = campfire_db::ChannelThread::create(tx, campfire_db::NewChannelThread { room_id: DESIGNERS, creator_id: KEVIN, name: Some("Private files".into()), visibility: campfire_db::channel_thread::ThreadVisibility::Private, ..Default::default() })?;
        let message = campfire_db::Message::create(tx, campfire_db::NewMessage { room_id: DESIGNERS, creator_id: KEVIN, thread_id: Some(thread.id), attachment_blob_id: Some(original_id), ..Default::default() })?;
        for (source, child) in [(original_id, derivative_id), (preview_id, nested_id)] {
            tx.conn().execute("INSERT INTO active_storage_variant_records(blob_id,variation_digest) VALUES(?,'private-test')", [source])?;
            let variant = tx.conn().last_insert_rowid();
            campfire_db::Attachment::create(tx, "ActiveStorage::VariantRecord", variant, "image", child)?;
        }
        campfire_db::Attachment::create(tx, "ActiveStorage::Blob", original_id, "preview_image", preview_id)?;
        Ok((thread.id, message.id))
    }).await.unwrap();
    let blobs = a
        .db()
        .read(move |conn| {
            ids.into_iter()
                .map(|id| {
                    campfire_storage::Blob::find(conn, id)
                        .map_err(|error| campfire_db::Error::Other(error.to_string()))?
                        .ok_or(campfire_db::Error::RecordNotFound("Blob"))
                })
                .collect::<campfire_db::Result<Vec<_>>>()
        })
        .await
        .unwrap();
    let mut paths = Vec::new();
    for blob in &blobs {
        paths.push(campfire_storage::paths::blob_proxy_path(
            verifier, blob, None,
        ));
        paths.push(campfire_storage::paths::blob_redirect_path(
            verifier, blob, None,
        ));
        paths.push(a.booted.app.storage.service.url_path(
            verifier,
            &blob.key,
            Some(now + jiff::SignedDuration::from_secs(300)),
            &blob.filename,
            Some("text/plain"),
            "inline",
        ));
    }
    let variation = campfire_storage::Variation::resize_to_limit(32, 32, Some("png"));
    paths.push(campfire_storage::paths::representation_proxy_path(
        verifier, &blobs[0], &variation,
    ));
    paths.push(campfire_storage::paths::representation_redirect_path(
        verifier, &blobs[0], &variation,
    ));
    for path in &paths {
        assert_eq!(
            outsider.send(get(path)).await.status,
            StatusCode::NOT_FOUND,
            "outsider {path}"
        );
        assert_eq!(
            anonymous.send(get(path)).await.status,
            StatusCode::NOT_FOUND,
            "anonymous {path}"
        );
    }
    a.db()
        .write(move |tx| {
            campfire_db::ChannelThread::find(tx.conn(), thread_id)?
                .add_access_member(tx, KEVIN, JZ)?;
            Ok(())
        })
        .await
        .unwrap();
    for path in &paths {
        let reply = outsider.send(get(path)).await;
        assert!(
            matches!(reply.status, StatusCode::OK | StatusCode::FOUND),
            "member {path}: {}",
            reply.text()
        );
        assert_eq!(
            reply.header("cache-control"),
            Some("private, no-store"),
            "{path}"
        );
    }
    a.db()
        .write(move |tx| {
            campfire_db::ChannelThread::find(tx.conn(), thread_id)?
                .remove_access_member(tx, KEVIN, JZ)
        })
        .await
        .unwrap();
    for path in &paths {
        assert_eq!(
            outsider.send(get(path)).await.status,
            StatusCode::NOT_FOUND,
            "removed {path}"
        );
    }
    let reuse = outsider.write(json_body(Method::POST, &format!("/rooms/{DESIGNERS}/messages.json"), &json!({"message":{"attachment": original.signed_id, "markdown_source":"Reuse private file"}}))).await;
    assert_eq!(
        reuse.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "signed reuse: {}",
        reuse.text()
    );
    let avatar = outsider
        .write(
            Req::new(Method::PATCH, &campfire_routes::user_profile())
                .form(&[("user[avatar]", &original.signed_id)]),
        )
        .await;
    assert_eq!(
        avatar.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "avatar reuse: {}",
        avatar.text()
    );
    let logo = admin
        .write(Req::new(Method::PATCH, "/account").form(&[("account[logo]", &original.signed_id)]))
        .await;
    assert_eq!(
        logo.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "logo reuse: {}",
        logo.text()
    );
    let bot = anonymous
        .send(json_body(
            Method::POST,
            &format!("/rooms/486777696/{BENDER_KEY}/messages.json"),
            &json!({"attachment": derivative.signed_id}),
        ))
        .await;
    assert_eq!(
        bot.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "bot signed reuse: {}",
        bot.text()
    );
    let private_path = &paths[0];
    assert_eq!(admin.send(get(private_path)).await.status, StatusCode::OK);
    a.db()
        .write(move |tx| campfire_db::Message::find(tx.conn(), message_id)?.destroy(tx))
        .await
        .unwrap();
    assert_eq!(
        creator.send(get(private_path)).await.status,
        StatusCode::NOT_FOUND,
        "marked orphan denies even original uploader"
    );
    crate::active_storage::purge(&a.booted.app, original_id)
        .await
        .unwrap();
    // The stopped runner leaves variants queued. Their signed paths must stay restricted.
    for path in paths.iter().skip(3) {
        assert_eq!(
            anonymous.send(get(path)).await.status,
            StatusCode::NOT_FOUND,
            "queued derivative {path}"
        );
    }
}

#[tokio::test]
async fn private_poll_media_cannot_be_reattached_after_deletion_or_while_purge_is_pending() {
    let a = app(true)
        .await
        .expect("restored default seed")
        .without_job_runner()
        .await;
    let mut creator = a.sign_in(KEVIN).await;
    let mut outsider = a.sign_in(JZ).await;
    let original = upload(&mut creator, "private-poll.png").await;
    let derivative = upload(&mut creator, "private-poll-variant.txt").await;
    let verifier = &*a.booted.app.storage.verifier;
    let now = a.booted.app.clock.now();
    let original_id =
        campfire_storage::paths::verify_signed_blob_id(verifier, &original.signed_id, now).unwrap();
    let child_id =
        campfire_storage::paths::verify_signed_blob_id(verifier, &derivative.signed_id, now)
            .unwrap();
    let (thread_id, message_id) = a.db().write(move |tx| {
        let thread = campfire_db::ChannelThread::create(tx, campfire_db::NewChannelThread {
            room_id: DESIGNERS, creator_id: KEVIN, visibility: campfire_db::channel_thread::ThreadVisibility::Private, ..Default::default()
        })?;
        let message = campfire_db::Message::create(tx, campfire_db::NewMessage {
            room_id: DESIGNERS, creator_id: KEVIN, thread_id: Some(thread.id), markdown_source: Some("Private question".into()), ..Default::default()
        })?;
        let poll = campfire_db::Poll::create_for_message(tx, &message, campfire_db::NewPoll {
            labels: vec!["Image".into(), "Other".into()], ..Default::default()
        })?;
        tx.conn().execute("UPDATE active_storage_blobs SET metadata=json_set(metadata,'$.poll_media',json('true')) WHERE id=?", [original_id])?;
        poll.options(tx.conn())?[0].attach_media(tx, original_id)?;
        tx.conn().execute("INSERT INTO active_storage_variant_records(blob_id,variation_digest) VALUES(?,'private-poll-test')", [original_id])?;
        campfire_db::Attachment::create(tx, "ActiveStorage::VariantRecord", tx.conn().last_insert_rowid(), "image", child_id)?;
        Ok((thread.id, message.id))
    }).await.unwrap();
    let blobs = a
        .db()
        .read(move |conn| {
            [original_id, child_id]
                .into_iter()
                .map(|id| {
                    campfire_storage::Blob::find(conn, id)
                        .map_err(|error| campfire_db::Error::Other(error.to_string()))?
                        .ok_or(campfire_db::Error::RecordNotFound("Blob"))
                })
                .collect::<campfire_db::Result<Vec<_>>>()
        })
        .await
        .unwrap();
    let paths = blobs
        .iter()
        .map(|blob| campfire_storage::paths::blob_proxy_path(verifier, blob, None))
        .collect::<Vec<_>>();
    for path in &paths {
        assert_eq!(outsider.send(get(path)).await.status, StatusCode::NOT_FOUND);
    }
    a.db()
        .write(move |tx| {
            campfire_db::ChannelThread::find(tx.conn(), thread_id)?
                .add_access_member(tx, KEVIN, JZ)?;
            Ok(())
        })
        .await
        .unwrap();
    for path in &paths {
        assert_eq!(outsider.send(get(path)).await.status, StatusCode::OK);
    }
    a.db()
        .write(move |tx| {
            campfire_db::ChannelThread::find(tx.conn(), thread_id)?
                .remove_access_member(tx, KEVIN, JZ)
        })
        .await
        .unwrap();
    a.db()
        .write(move |tx| campfire_db::Message::find(tx.conn(), message_id)?.destroy(tx))
        .await
        .unwrap();
    for signed in [&original.signed_id, &derivative.signed_id] {
        let response = outsider.write(json_body(Method::POST, &format!("/rooms/{DESIGNERS}/messages.json"),
            &json!({"message":{"attachment":signed,"markdown_source":"Reuse deleted private poll"}}))).await;
        assert_eq!(
            response.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            response.text()
        );
    }
    let poll_reuse = creator
        .write(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{DESIGNERS}/polls"),
            &json!({"clientMessageId":"private-poll-orphan-reuse","question":"Public question",
            "options":["Image","Other"],"multiple":false,"anonymous":false,"closesAt":null,
            "optionMedia":[{"signedId":original.signed_id},null]}),
        ))
        .await;
    assert_eq!(
        poll_reuse.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        poll_reuse.text()
    );
    crate::active_storage::purge(&a.booted.app, original_id)
        .await
        .unwrap();
    let response = outsider.write(json_body(Method::POST, &format!("/rooms/{DESIGNERS}/messages.json"),
        &json!({"message":{"attachment":derivative.signed_id,"markdown_source":"Reuse queued private poll derivative"}}))).await;
    assert_eq!(
        response.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        response.text()
    );
    assert_eq!(
        outsider.send(get(&paths[1])).await.status,
        StatusCode::NOT_FOUND
    );
}
