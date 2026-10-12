use super::grouped_files_tests::upload;
use super::*;

async fn schedule_files(browser: &mut Browser<'_>, ids: &[String]) -> api::ScheduledMessage {
    let reply = browser.write(json_body(Method::POST,
        &format!("/api/v1/rooms/{ALL_TALK}/scheduled_messages"),
        &json!({"markdownSource": "", "sendAt": "2026-03-03T16:00:00Z", "attachmentSignedIds": ids}),
    )).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    parse(&reply)
}

#[tokio::test]
async fn scheduled_files_are_private_on_original_thumbnail_proxy_and_disk_urls() {
    let a = app(true)
        .await
        .expect("restored default seed")
        .without_job_runner()
        .await;
    a.db().write(|tx| {
        tx.conn().execute("INSERT OR IGNORE INTO memberships (room_id,user_id,created_at,updated_at) VALUES (?,?,?,?)", rusqlite::params![ALL_TALK, KEVIN, tx.now(), tx.now()])?;
        Ok(())
    }).await.unwrap();
    let staged = a
        .booted
        .app
        .storage
        .stage_bytes(
            include_bytes!("../../../../fixtures/files/workspace_icons/animated.gif"),
            campfire_storage::Filename::new("private.gif"),
            Some("image/gif"),
        )
        .unwrap();
    let blob = a
        .db()
        .write(move |tx| {
            let mut blob = crate::controllers::messages::save_staged(tx, staged)?;
            blob.metadata
                .set("uploader_id", campfire_storage::Json::Int(DAVID));
            blob.update_metadata(tx.conn(), blob.metadata.clone())
                .unwrap();
            Ok(blob)
        })
        .await
        .unwrap();
    let signed =
        campfire_storage::paths::signed_blob_id(&*a.booted.app.storage.verifier, blob.id, None);
    let mut author = a.sign_in(DAVID).await;
    let mut other = a.sign_in(KEVIN).await;
    let mut anonymous = a.anonymous();
    let scheduled = schedule_files(&mut author, std::slice::from_ref(&signed)).await;
    let file = &scheduled.attachments[0].attachment;
    let mut urls = Vec::new();
    for url in [
        &file.url,
        file.thumbnail_url.as_ref().expect("image thumbnail"),
    ] {
        let response = author.send(get(url)).await;
        assert_eq!(response.status, StatusCode::FOUND, "{}", response.text());
        assert_eq!(response.header("cache-control"), Some("private, no-store"));
        let disk = response.location().unwrap().to_owned();
        let proxy = url.replace("/redirect/", "/proxy/");
        for path in [url.clone(), disk, proxy] {
            let allowed = author.send(get(&path)).await;
            assert!(
                allowed.status.is_success() || allowed.status.is_redirection(),
                "{path}: {}",
                allowed.text()
            );
            assert_eq!(allowed.header("cache-control"), Some("private, no-store"));
            for browser in [&mut other, &mut anonymous] {
                assert_eq!(
                    browser.send(get(&path)).await.status,
                    StatusCode::NOT_FOUND,
                    "{path}"
                );
            }
            urls.push(path);
        }
    }
    let list: api::ScheduledMessageList =
        parse(&other.send(get("/api/v1/scheduled_messages")).await);
    assert!(
        !list
            .scheduled_messages
            .iter()
            .any(|row| row.id == scheduled.id)
    );
    let path = format!("/api/v1/scheduled_messages/{}", scheduled.id);
    for (method, body) in [
        (Method::PATCH, json!({"attachmentSignedIds": []})),
        (Method::DELETE, json!({})),
    ] {
        assert_eq!(
            other.write(json_body(method, &path, &body)).await.status,
            StatusCode::NOT_FOUND
        );
    }
    let stolen = other
        .write(
            Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/messages"))
                .header("accept", "text/vnd.turbo-stream.html")
                .header("content-type", "application/json")
                .body(json!({"message": {"attachment": signed}}).to_string()),
        )
        .await;
    assert_eq!(
        stolen.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        stolen.text()
    );
    let cancelled = author
        .write(json_body(Method::DELETE, &path, &json!({})))
        .await;
    assert_eq!(cancelled.status, StatusCode::NO_CONTENT);
    let reuse = author
        .write(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{ALL_TALK}/messages"),
            &json!({"clientMessageId": "cancelled-upload", "attachmentSignedIds": [signed]}),
        ))
        .await;
    assert_eq!(
        reuse.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reuse.text()
    );
    // Purge is queued; detached originals and their variants stay private during that interval.
    for url in &urls {
        assert_eq!(other.send(get(url)).await.status, StatusCode::NOT_FOUND);
        assert_eq!(anonymous.send(get(url)).await.status, StatusCode::NOT_FOUND);
    }
    crate::active_storage::purge(&a.booted.app, blob.id)
        .await
        .unwrap();
    for url in urls {
        assert_eq!(other.send(get(&url)).await.status, StatusCode::NOT_FOUND);
    }
    assert!(!a.booted.app.storage.service.path_for(&blob.key).exists());
}

#[tokio::test]
async fn scheduled_files_revalidation_drops_without_sending_any_part_of_the_group() {
    for failure in ["missing", "blob", "size", "owner", "limit"] {
        let a = app(true)
            .await
            .expect("restored default seed")
            .without_job_runner()
            .await;
        let mut author = a.sign_in(DAVID).await;
        let first = upload(&mut author, "first.txt").await;
        let second = upload(&mut author, "second.txt").await;
        let scheduled =
            schedule_files(&mut author, &[first.signed_id, second.signed_id.clone()]).await;
        let blob_id = campfire_storage::paths::verify_signed_blob_id(
            &*a.booted.app.storage.verifier,
            &second.signed_id,
            a.booted.app.clock.now(),
        )
        .unwrap();
        let blob = a
            .db()
            .read(move |conn| {
                Ok(campfire_storage::Blob::find(conn, blob_id)
                    .unwrap()
                    .unwrap())
            })
            .await
            .unwrap();
        match failure {
            "blob" => {
                let conn = rusqlite::Connection::open(a.db().path()).unwrap();
                conn.execute("PRAGMA foreign_keys=OFF", []).unwrap();
                conn.execute("DELETE FROM active_storage_blobs WHERE id=?", [blob_id]).unwrap();
            }
            "missing" => std::fs::remove_file(a.booted.app.storage.service.path_for(&blob.key)).unwrap(),
            "size" => std::fs::write(a.booted.app.storage.service.path_for(&blob.key), "different").unwrap(),
            "owner" => a.db().write(move |tx| {
                tx.conn().execute("UPDATE active_storage_blobs SET metadata=json_set(metadata,'$.uploader_id',?) WHERE id=?", [KEVIN, blob_id])?;
                Ok(())
            }).await.unwrap(),
            "limit" => a.db().write(|tx| {
                    tx.conn().execute("UPDATE accounts SET settings=json_set(COALESCE(settings,'{}'),'$.upload_limit_bytes',4)", [])?;
                Ok(())
            }).await.unwrap(),
            _ => unreachable!(),
        }
        let before = a
            .db()
            .read(|conn| {
                Ok(conn.query_row("SELECT COUNT(*) FROM messages", [], |row| {
                    row.get::<_, i64>(0)
                })?)
            })
            .await
            .unwrap();
        let response = author
            .write(json_body(
                Method::POST,
                &format!("/api/v1/scheduled_messages/{}/send_now", scheduled.id),
                &json!({}),
            ))
            .await;
        assert_eq!(
            response.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{failure}: {}",
            response.text()
        );
        let id = scheduled.id;
        a.db()
            .read(move |conn| {
                let row = campfire_db::ScheduledMessage::find(conn, id)?;
                assert!(row.dropped(), "{failure}");
                assert_eq!(row.sent_message_id, None);
            assert_eq!(conn.query_row("SELECT COUNT(*) FROM active_storage_attachments WHERE record_type='ScheduledMessage' AND record_id=?", [id], |row| row.get::<_, i64>(0))?, 2);
                assert!(row.drop_reason.unwrap().contains("attachment"));
                assert_eq!(
                    conn.query_row("SELECT COUNT(*) FROM messages", [], |row| row
                        .get::<_, i64>(0))?,
                    before
                );
                assert_eq!(
                    campfire_db::ActivityItem::find_by_user_and_source(
                        conn,
                        DAVID,
                        "ScheduledMessage",
                        id
                    )?
                    .unwrap()
                    .event_type,
                    "scheduled_message_dropped"
                );
                Ok(())
            })
            .await
            .unwrap();
        let past = author
            .send(get("/api/v1/scheduled_messages?status=past"))
            .await;
        assert_eq!(past.status, StatusCode::OK, "{failure}: {}", past.text());
    }
}

#[tokio::test]
async fn scheduled_files_accept_ten_files_and_reject_removing_the_last_content_atomically() {
    let a = app(true)
        .await
        .expect("restored default seed")
        .without_job_runner()
        .await;
    let mut author = a.sign_in(DAVID).await;
    let mut ids = Vec::new();
    for index in 0..10 {
        ids.push(
            upload(&mut author, &format!("file-{index}.txt"))
                .await
                .signed_id,
        );
    }
    let scheduled = schedule_files(&mut author, &ids).await;
    assert_eq!(scheduled.attachments.len(), 10);
    let path = format!("/api/v1/scheduled_messages/{}", scheduled.id);
    let response = author
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"attachmentSignedIds": []}),
        ))
        .await;
    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
    a.db()
        .read(move |conn| {
            assert_eq!(
                campfire_db::ScheduledMessage::find(conn, scheduled.id)?
                    .attachments(conn)?
                    .len(),
                10
            );
            Ok(())
        })
        .await
        .unwrap();
    let response = author
        .write(json_body(
            Method::POST,
            &format!("{path}/send_now"),
            &json!({}),
        ))
        .await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    let sent: api::ScheduledMessage = parse(&response);
    a.db()
        .read(move |conn| {
            let message = campfire_db::Message::find(conn, sent.sent_message_id.unwrap())?;
            assert_eq!(message.attachments(conn)?.len(), 10);
            for (_, blob) in message.attachments(conn)? {
                assert!(!blob.metadata.unwrap().contains("scheduled_by"));
            }
            Ok(())
        })
        .await
        .unwrap();
    let mut other = a.sign_in(KEVIN).await;
    assert_eq!(
        other
            .send(get(&sent.attachments[0].attachment.url))
            .await
            .status,
        StatusCode::FOUND
    );
}
