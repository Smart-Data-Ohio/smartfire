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

async fn image_upload(a: &TestApp) -> (campfire_storage::Blob, String) {
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
    (blob, signed)
}

#[tokio::test]
async fn scheduled_files_cannot_be_assigned_as_legacy_avatars() {
    let a = app(true)
        .await
        .expect("restored default seed")
        .without_job_runner()
        .await;
    let (_, signed) = image_upload(&a).await;
    let mut author = a.sign_in(DAVID).await;
    let mut other = a.sign_in(KEVIN).await;
    schedule_files(&mut author, std::slice::from_ref(&signed)).await;
    for browser in [&mut other, &mut author] {
        let reply = browser
            .write(json_body(
                Method::PATCH,
                "/users/me/profile",
                &json!({"user": {"avatar": signed}}),
            ))
            .await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            reply.text()
        );
    }
    a.db()
        .read(|conn| {
            for user in [DAVID, KEVIN] {
                assert!(
                    campfire_storage::Blob::attached(conn, "User", user, "avatar")
                        .unwrap()
                        .is_none()
                );
            }
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn scheduled_files_cannot_be_assigned_to_public_api_image_slots() {
    let a = app(true)
        .await
        .expect("restored default seed")
        .without_job_runner()
        .await;
    let (_, signed) = image_upload(&a).await;
    let mut author = a.sign_in(DAVID).await;
    author.grant_sudo().await;
    schedule_files(&mut author, std::slice::from_ref(&signed)).await;
    for (method, path, body) in [
        (
            Method::PUT,
            "/api/v1/settings/avatar",
            json!({"signedId": signed}),
        ),
        (
            Method::PUT,
            "/api/v1/settings/banner",
            json!({"signedId": signed}),
        ),
        (
            Method::PUT,
            "/api/v1/admin/workspace/logo",
            json!({"signedId": signed}),
        ),
        (
            Method::PUT,
            "/api/v1/admin/workspace/banner",
            json!({"signedId": signed}),
        ),
        (
            Method::POST,
            "/api/v1/admin/icons",
            json!({"name": "private_icon", "title": "Private icon", "signedId": signed}),
        ),
        (
            Method::POST,
            "/api/v1/admin/bots",
            json!({"name": "Private bot", "avatar": signed}),
        ),
    ] {
        let reply = author.write(json_body(method, path, &body)).await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{path}: {}",
            reply.text()
        );
    }
}

#[tokio::test]
async fn scheduled_files_unpublished_uploads_and_representations_never_offer_public_caching() {
    let a = app(true)
        .await
        .expect("restored default seed")
        .without_job_runner()
        .await;
    let (blob, _) = image_upload(&a).await;
    let verifier = &*a.booted.app.storage.verifier;
    let variation = campfire_storage::Variation::resize_to_limit(64, 64, Some("webp"));
    let mut author = a.sign_in(DAVID).await;
    for url in [
        campfire_storage::paths::blob_redirect_path(verifier, &blob, None),
        campfire_storage::paths::representation_redirect_path(verifier, &blob, &variation),
    ] {
        let redirect = author.send(get(&url)).await;
        assert_eq!(redirect.status, StatusCode::FOUND);
        assert_eq!(redirect.header("cache-control"), Some("private, no-store"));
        let disk = redirect.location().unwrap().to_owned();
        let proxy = url.replace("/redirect/", "/proxy/");
        for path in [disk, proxy] {
            let response = author.send(get(&path)).await;
            assert_eq!(response.status, StatusCode::OK, "{path}");
            assert_eq!(
                response.header("cache-control"),
                Some("private, no-store"),
                "{path}"
            );
        }
    }
}

#[tokio::test]
async fn scheduled_files_legacy_avatar_serving_preserves_owner_authorization() {
    let a = app(true)
        .await
        .expect("restored default seed")
        .without_job_runner()
        .await;
    let (blob, signed) = image_upload(&a).await;
    let mut author = a.sign_in(DAVID).await;
    let mut other = a.sign_in(KEVIN).await;
    let mut anonymous = a.anonymous();
    schedule_files(&mut author, std::slice::from_ref(&signed)).await;
    let secrets = a.booted.app.secrets.clone();
    let url = a
        .db()
        .write(move |tx| {
            campfire_storage::blob::insert_attachment(
                tx.conn(),
                "avatar",
                "User",
                KEVIN,
                blob.id,
                tx.now().jiff(),
            )
            .unwrap();
            Ok(crate::controllers::presenters::avatar_path(
                &secrets,
                &campfire_db::User::find(tx.conn(), KEVIN)?,
            ))
        })
        .await
        .unwrap();
    assert_eq!(other.send(get(&url)).await.status, StatusCode::NOT_FOUND);
    assert_eq!(anonymous.send(get(&url)).await.status, StatusCode::FOUND);
    let allowed = author.send(get(&url)).await;
    assert_eq!(allowed.status, StatusCode::OK, "{}", allowed.text());
    assert_eq!(allowed.header("cache-control"), Some("private, no-store"));
}

#[tokio::test]
async fn scheduled_files_alternate_branding_and_icon_urls_preserve_owner_authorization() {
    let a = app(true)
        .await
        .expect("restored default seed")
        .without_job_runner()
        .await;
    let (blob, signed) = image_upload(&a).await;
    let mut author = a.sign_in(DAVID).await;
    let mut other = a.sign_in(KEVIN).await;
    schedule_files(&mut author, &[signed]).await;
    let banner_url = campfire_routes::fresh_user_banner(
        crate::controllers::presenters::avatar_token(&a.booted.app.secrets, KEVIN),
        blob.id,
        false,
    );
    a.db().write(move |tx| {
        let account = campfire_db::Account::first(tx.conn())?.unwrap();
        tx.conn().execute("INSERT INTO workspace_icons (name,title,creator_id,created_at,updated_at) VALUES ('private_upload','Private upload',?,?,?)", rusqlite::params![DAVID, tx.now(), tx.now()])?;
        let icon_id = tx.conn().last_insert_rowid();
        for (kind, id, name) in [("Account", account.id, "logo"), ("Account", account.id, "banner"), ("User", KEVIN, "banner"), ("WorkspaceIcon", icon_id, "image")] {
            campfire_storage::blob::insert_attachment(tx.conn(), name, kind, id, blob.id, tx.now().jiff()).unwrap();
        }
        Ok(())
    }).await.unwrap();
    for url in [
        "/account/logo",
        "/account/logo?animated=1",
        "/account/banner",
        "/icons/private_upload",
    ] {
        let response = other.send(get(url)).await;
        assert_eq!(
            response.status,
            StatusCode::NOT_FOUND,
            "{url}: {}",
            response.text()
        );
        let response = author.send(get(url)).await;
        assert_eq!(
            response.status,
            StatusCode::OK,
            "{url}: {}",
            response.text()
        );
        assert_eq!(
            response.header("cache-control"),
            Some("private, no-store"),
            "{url}"
        );
    }
    assert_eq!(
        other.send(get(&banner_url)).await.status,
        StatusCode::NOT_FOUND
    );
    let response = author.send(get(&banner_url)).await;
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.header("cache-control"), Some("private, no-store"));
}

#[tokio::test]
async fn scheduled_files_assignment_rechecks_derivative_privacy_inside_the_writer() {
    use campfire_runtime::presenters::attachments::Assignment;
    let a = app(true)
        .await
        .expect("restored default seed")
        .without_job_runner()
        .await;
    let (blob, signed) = image_upload(&a).await;
    let image = crate::active_storage::processed_representation(
        &a.booted.app,
        blob,
        campfire_storage::Variation::resize_to_limit(64, 64, Some("webp")),
    )
    .await
    .unwrap();
    let derivative =
        campfire_storage::paths::signed_blob_id(&*a.booted.app.storage.verifier, image.id, None);
    let assignment = Assignment::Signed(derivative.clone())
        .stage(&a.booted.app)
        .await
        .unwrap();
    let mut author = a.sign_in(DAVID).await;
    schedule_files(&mut author, &[signed]).await;
    let result = a
        .db()
        .write(move |tx| {
            crate::controllers::messages::attachment_blob(
                tx,
                assignment,
                crate::controllers::messages::AttachmentPolicy::SignedBlob,
                "attachment",
            )
        })
        .await;
    assert!(
        matches!(result, Err(campfire_db::Error::RecordInvalid(_))),
        "{result:?}"
    );
    assert!(matches!(
        Assignment::Signed(derivative).stage(&a.booted.app).await,
        Err(campfire_kit::Error::Status(
            StatusCode::UNPROCESSABLE_ENTITY
        ))
    ));
}

#[tokio::test]
async fn scheduled_files_poll_assignment_rechecks_privacy_after_schedule_and_cancel() {
    let a = app(true)
        .await
        .expect("restored default seed")
        .without_job_runner()
        .await;
    let (blob, signed) = image_upload(&a).await;
    let mut author = a.sign_in(DAVID).await;
    let mut scheduler = a.sign_in(DAVID).await;
    let hold = campfire_api::test_hooks::hold_before_poll_create_write("scheduled-poll-race");
    let body = json!({"clientMessageId": "scheduled-poll-race", "question": "Private?", "options": ["Image", "None"], "optionMedia": [{"signedId": signed}, null], "multiple": false, "anonymous": false, "closesAt": null});
    let request = json_body(
        Method::POST,
        &format!("/api/v1/rooms/{ALL_TALK}/polls"),
        &body,
    );
    let (reply, ()) = tokio::join!(author.write(request), async {
        tokio::time::timeout(Duration::from_secs(30), hold.reached.wait())
            .await
            .unwrap();
        let scheduled = schedule_files(&mut scheduler, &[signed]).await;
        let deleted = scheduler
            .write(json_body(
                Method::DELETE,
                &format!("/api/v1/scheduled_messages/{}", scheduled.id),
                &json!({}),
            ))
            .await;
        assert_eq!(deleted.status, StatusCode::NO_CONTENT);
        hold.release.wait().await;
    });
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.text()
    );
    a.db().read(move |conn| {
        let current = campfire_storage::Blob::find(conn, blob.id).unwrap().unwrap();
        assert_eq!(current.metadata.get("scheduled_by").and_then(campfire_storage::Json::as_i64), Some(DAVID));
        assert!(!conn.query_row("SELECT EXISTS(SELECT 1 FROM messages WHERE client_message_id='scheduled-poll-race')", [], |row| row.get::<_, bool>(0))?);
        Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn scheduled_files_prefetch_before_scheduling_cannot_populate_the_front_cache() {
    use axum::body::{Body, to_bytes};
    use campfire_kit::front::{ConnInfo, FrontConfig, Handler};
    let a = app(true)
        .await
        .expect("restored default seed")
        .without_job_runner()
        .await;
    let mut author = a.sign_in(DAVID).await;
    let uploaded = upload(&mut author, "prefetched.txt").await;
    let id = campfire_storage::paths::verify_signed_blob_id(
        &*a.booted.app.storage.verifier,
        &uploaded.signed_id,
        a.booted.app.clock.now(),
    )
    .unwrap();
    let blob = a
        .db()
        .read(move |conn| Ok(campfire_storage::Blob::find(conn, id).unwrap().unwrap()))
        .await
        .unwrap();
    let url =
        campfire_storage::paths::blob_proxy_path(&*a.booted.app.storage.verifier, &blob, None);
    let front = std::sync::Arc::new(Handler::new(
        &FrontConfig::from_lookup(|_| None),
        a.booted.router.clone(),
    ));
    let connection = ConnInfo {
        remote: "127.0.0.1:12345".parse().unwrap(),
        tls: false,
    };
    let request = |cookie: &str| {
        axum::http::Request::builder()
            .uri(&url)
            .header("host", "campfire.test")
            .header("cookie", cookie)
            .body(Body::empty())
            .unwrap()
    };
    let prefetch = front
        .clone()
        .call(request(&author.cookie_header()), connection)
        .await;
    assert_eq!(prefetch.status(), StatusCode::OK);
    let cache_control = prefetch.headers().get("cache-control").cloned();
    assert_eq!(
        to_bytes(prefetch.into_body(), usize::MAX).await.unwrap(),
        "hello"
    );
    schedule_files(&mut author, &[uploaded.signed_id]).await;
    let anonymous = front.clone().call(request(""), connection).await;
    assert_eq!(
        anonymous.status(),
        StatusCode::NOT_FOUND,
        "anonymous download after prefetch and scheduling"
    );
    assert_ne!(anonymous.headers().get("x-cache").unwrap(), "hit");
    assert_eq!(cache_control.unwrap(), "private, no-store");
    let owner = front
        .call(request(&author.cookie_header()), connection)
        .await;
    assert_eq!(owner.status(), StatusCode::OK);
    assert_eq!(
        owner.headers().get("cache-control").unwrap(),
        "private, no-store"
    );
    assert_eq!(
        to_bytes(owner.into_body(), usize::MAX).await.unwrap(),
        "hello"
    );
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
    for failure in ["missing", "blob", "size", "checksum", "owner", "limit"] {
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
            "checksum" => std::fs::write(a.booted.app.storage.service.path_for(&blob.key), "jello").unwrap(),
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
