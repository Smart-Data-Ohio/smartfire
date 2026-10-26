use super::*;

const MEMBER: i64 = 773523953;

async fn setup(a: &TestApp, room: i64) -> Value {
    let mut admin = a.sign_in(DAVID).await;
    let response = admin
        .write(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{room}/webhooks"),
            &json!({"name":"Deployments"}),
        ))
        .await;
    assert_eq!(response.status, StatusCode::CREATED, "{}", response.text());
    parse(&response)
}

fn execute(hook: &Value, wait: bool) -> String {
    format!(
        "/api/webhooks/{}/{}{}",
        hook["webhook"]["id"],
        hook["token"].as_str().unwrap(),
        if wait { "?wait=true" } else { "" }
    )
}

#[tokio::test]
async fn channel_webhook_author_cannot_be_unbanned_or_managed_after_deletion() {
    let a = app(true)
        .await
        .expect("restored seed")
        .without_job_runner()
        .await;
    let hook = setup(&a, HQ).await;
    let hook_id = hook["webhook"]["id"].as_i64().unwrap();
    let author = a
        .db()
        .read(move |conn| {
            Ok(
                campfire_db::models::channel_webhook::ChannelWebhook::find(conn, hook_id)?
                    .unwrap()
                    .author_id,
            )
        })
        .await
        .unwrap();
    let mut admin = a.sign_in(DAVID).await;
    admin.grant_sudo().await;
    for deleted in [false, true] {
        if deleted {
            let response = admin
                .write(json_body(
                    Method::DELETE,
                    &format!("/api/v1/rooms/{HQ}/webhooks/{hook_id}"),
                    &Value::Null,
                ))
                .await;
            assert_eq!(
                response.status,
                StatusCode::NO_CONTENT,
                "{}",
                response.text()
            );
        }
        for (method, path, body) in [
            (
                Method::DELETE,
                format!("/api/v1/people/{author}/ban"),
                Value::Null,
            ),
            (
                Method::PATCH,
                format!("/api/v1/admin/people/{author}"),
                json!({"role":"administrator"}),
            ),
            (
                Method::PUT,
                format!("/api/v1/admin/bots/{author}/key"),
                Value::Null,
            ),
            (
                Method::PATCH,
                format!("/api/v1/admin/bots/{author}"),
                json!({"name":"Visible"}),
            ),
            (
                Method::POST,
                format!("/api/v1/admin/people/{author}/two_factor_reset"),
                Value::Null,
            ),
        ] {
            let response = admin.write(json_body(method, &path, &body)).await;
            assert_eq!(
                response.status,
                StatusCode::NOT_FOUND,
                "{path}: {}",
                response.text()
            );
        }
        for path in ["/api/v1/people", "/api/v1/directs/candidates"] {
            let response = admin.send(get(path)).await;
            assert_eq!(response.status, StatusCode::OK, "{}", response.text());
            let page: Value = parse(&response);
            assert!(
                !page["users"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|user| user["id"] == author)
            );
        }
        let response = admin
            .write(json_body(
                Method::POST,
                "/api/v1/directs",
                &json!({"userIds":[author]}),
            ))
            .await;
        // An unavailable direct-message target gets the same refusal as a missing one.
        assert_eq!(
            response.status,
            StatusCode::NOT_FOUND,
            "{}",
            response.text()
        );
        assert!(
            a.db()
                .read(move |conn| Ok(campfire_db::Membership::for_user(conn, author)?.is_empty()))
                .await
                .unwrap()
        );
        let user = a
            .db()
            .read(move |conn| campfire_db::User::find(conn, author))
            .await
            .unwrap();
        assert!(user.is_deactivated());
        assert!(user.bot_token_digest.is_none());
        assert!(user.email_address.is_none() && user.password_digest.is_none());
    }
}

#[tokio::test]
async fn channel_webhook_forward_keeps_embeds_and_attribution_in_saved_items() {
    let a = app(true)
        .await
        .expect("restored seed")
        .without_job_runner()
        .await;
    let hook = setup(&a, HQ).await;
    let mut guest = a.anonymous();
    let response = guest.send(json_body(Method::POST, &execute(&hook, true), &json!({"username":"CI", "embeds":[{"title":"Passed","description":"All checks green"}]}))).await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    let original: Value = parse(&response);
    let original_id = original["id"].as_i64().unwrap();
    let mut admin = a.sign_in(DAVID).await;
    let response = admin
        .write(json_body(
            Method::POST,
            &format!("/api/v1/messages/{original_id}/forwards"),
            &json!({"destinations":[{"roomId":HQ}],"note":"FYI"}),
        ))
        .await;
    assert_eq!(response.status, StatusCode::CREATED, "{}", response.text());
    let forwarded: Value = parse(&response);
    let copy = &forwarded["forwards"][0];
    assert_eq!(
        copy["embeds"],
        json!([{"title":"Passed","description":"All checks green"}])
    );
    assert_eq!(copy["webhook"], original["webhook"]);
    assert_eq!(copy["creatorId"], DAVID);
    let copy_id = copy["id"].as_i64().unwrap();
    for message_id in [original_id, copy_id] {
        let response = admin
            .write(json_body(
                Method::POST,
                "/api/v1/saved",
                &json!({"messageId":message_id}),
            ))
            .await;
        assert_eq!(response.status, StatusCode::CREATED, "{}", response.text());
    }
    let response = admin.send(get("/api/v1/saved")).await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    let saved: Value = parse(&response);
    for message_id in [original_id, copy_id] {
        let message = saved["messages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|message| message["id"] == message_id)
            .unwrap();
        assert_eq!(
            message["embeds"],
            json!([{"title":"Passed","description":"All checks green"}])
        );
        assert_eq!(message["webhook"], original["webhook"]);
    }
}

#[tokio::test]
async fn channel_webhook_json_wait_sync_unread_and_digest() {
    let a = app(true)
        .await
        .expect("restored seed")
        .without_job_runner()
        .await;
    let hook = setup(&a, HQ).await;
    let id = hook["webhook"]["id"].as_i64().unwrap();
    let raw = hook["token"].as_str().unwrap().to_owned();
    a.db()
        .read(move |conn| {
            let (digest, author): (String, i64) = conn.query_row(
                "SELECT token_digest,author_id FROM channel_webhooks WHERE id=?",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            assert_eq!(digest.len(), 64);
            assert_ne!(digest, raw);
            assert_eq!(
                campfire_db::models::workspace_invite::WorkspaceInvite::digest(&raw),
                digest
            );
            let user = campfire_db::User::find(conn, author)?;
            assert!(user.is_bot());
            assert!(!user.is_active());
            assert!(
                user.email_address.is_none()
                    && user.password_digest.is_none()
                    && user.bot_token_digest.is_none()
            );
            assert!(
                !campfire_db::User::active_bots_ordered(conn)?
                    .iter()
                    .any(|u| u.id == author)
            );
            Ok(())
        })
        .await
        .unwrap();
    let (addr, server) = serve(&a).await;
    let kevin = a.sign_in(KEVIN).await;
    let mut sync = Sync::connect(addr, &kevin.cookie_header(), &[format!("room:{HQ}")]).await;
    sync.welcome().await;
    let mut guest = a.anonymous();
    let payload = json!({"content":"Release **ready** @[Kevin]", "username":"Build server", "embeds":[{"title":"Release", "url":"https://example.test/release", "color":16777215,"fields":[{"name":"Status","value":"Passed","inline":true}],"footer":{"text":"CI"},"timestamp":"2026-10-11T17:00:00Z"}]});
    let response = guest
        .send(json_body(Method::POST, &execute(&hook, true), &payload))
        .await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    let message: Value = parse(&response);
    assert!(
        message["bodyHtml"]
            .as_str()
            .unwrap()
            .contains("<strong>ready</strong>")
    );
    assert_eq!(message["webhook"]["name"], "Deployments");
    assert_eq!(message["webhook"]["username"], "Build server");
    assert_eq!(message["embeds"], payload["embeds"]);
    let event = sync.until(created_in(HQ), |_| false).await;
    let api::SyncPayload::MessageCreated(published) = event.payload else {
        panic!("created event")
    };
    let mut published = serde_json::to_value(published).unwrap();
    assert!(published["cardsAsOf"].as_str().unwrap() <= message["cardsAsOf"].as_str().unwrap());
    published["cardsAsOf"] = message["cardsAsOf"].clone();
    assert_eq!(published, message);
    let message_id = message["id"].as_i64().unwrap();
    a.db()
        .read(move |conn| {
            let item = campfire_db::ActivityItem::find_by_user_and_source(
                conn, KEVIN, "Message", message_id,
            )?
            .expect("mention notification");
            assert_eq!(item.event_type, "mention");
            assert!(item.unread());
            Ok(())
        })
        .await
        .unwrap();
    sync.until(
        |e| {
            matches!(
                e.payload,
                api::SyncPayload::RoomUnread(api::RoomUnread { room_id: HQ, .. })
            )
        },
        |_| false,
    )
    .await;
    let mut kevin = a.sign_in(KEVIN).await;
    let detail: api::RoomDetail = parse(&kevin.send(get(&format!("/api/v1/rooms/{HQ}"))).await);
    assert_eq!(
        detail.unread,
        Some(api::UnreadDivider {
            first_unread_message_id: message_id,
            count: 1
        })
    );
    let response = guest
        .send(json_body(
            Method::POST,
            &execute(&hook, false),
            &json!({"embeds":[{"description":"Embed only"}]}),
        ))
        .await;
    assert_eq!(
        response.status,
        StatusCode::NO_CONTENT,
        "{}",
        response.text()
    );
    assert!(response.body.is_empty());
    let mut admin = a.sign_in(DAVID).await;
    let listed: Value = parse(
        &admin
            .send(get(&format!("/api/v1/rooms/{HQ}/webhooks")))
            .await,
    );
    assert!(listed["webhooks"][0]["lastUsedAt"].is_string());
    assert!(listed["webhooks"][0].get("token").is_none());
    server.abort();
}

#[tokio::test]
async fn channel_webhook_bad_credentials_delete_and_regenerate() {
    let a = app(true)
        .await
        .expect("restored seed")
        .without_job_runner()
        .await;
    let hook = setup(&a, HQ).await;
    let id = hook["webhook"]["id"].as_i64().unwrap();
    let mut guest = a.anonymous();
    let payload = json!({"content":"Hello"});
    let wrong_token = guest
        .send(json_body(
            Method::POST,
            &format!("/api/webhooks/{id}/wrong"),
            &payload,
        ))
        .await;
    let wrong_id = guest
        .send(json_body(Method::POST, "/api/webhooks/0/wrong", &payload))
        .await;
    let invalid_id = guest
        .send(json_body(
            Method::POST,
            "/api/webhooks/nope/wrong",
            &payload,
        ))
        .await;
    assert_eq!(wrong_token.status, StatusCode::NOT_FOUND);
    assert_eq!(wrong_id.status, wrong_token.status);
    assert_eq!(wrong_id.body, wrong_token.body);
    assert_eq!(invalid_id.body, wrong_token.body);
    let mut admin = a.sign_in(DAVID).await;
    let path = format!("/api/v1/rooms/{HQ}/webhooks/{id}");
    let response = admin
        .write(json_body(
            Method::POST,
            &format!("{path}/regenerate_token"),
            &json!({}),
        ))
        .await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    let new: Value = parse(&response);
    assert_ne!(new["token"], hook["token"]);
    assert_eq!(
        guest
            .send(json_body(Method::POST, &execute(&hook, false), &payload))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        guest
            .send(json_body(Method::POST, &execute(&new, false), &payload))
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        admin
            .write(json_body(Method::DELETE, &path, &json!({})))
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        guest
            .send(json_body(Method::POST, &execute(&new, false), &payload))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    a.db().read(move |conn| {
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM memberships JOIN users ON users.id=memberships.user_id WHERE room_id=? AND users.name='Deployments'", [HQ], |r| r.get(0))?;
        assert_eq!(count, 0);
        Ok(())
    }).await.unwrap();
    let page: Value = parse(
        &admin
            .send(get(&format!("/api/v1/rooms/{HQ}/messages")))
            .await,
    );
    assert_eq!(page["messages"][0]["webhook"]["name"], "Deployments");
}

#[tokio::test]
async fn channel_webhook_embed_validation_edges() {
    let a = app(true)
        .await
        .expect("restored seed")
        .without_job_runner()
        .await;
    let hook = setup(&a, HQ).await;
    let mut guest = a.anonymous();
    let mut invalid = vec![
        json!({}),
        json!({"content":"  "}),
        json!({"username":"", "content":"x"}),
        json!({"username":"x".repeat(81), "content":"x"}),
        json!({"content":"x".repeat(50001)}),
        json!({"embeds":[{}]}),
        json!({"embeds":vec![json!({"title":"x"});11]}),
    ];
    for embed in [
        json!({"title":"x".repeat(257)}),
        json!({"description":"x".repeat(4097)}),
        json!({"color":-1}),
        json!({"color":16777216}),
        json!({"color":1.5}),
        json!({"url":"javascript:alert(1)"}),
        json!({"timestamp":"yesterday"}),
        json!({"fields":vec![json!({"name":"n","value":"v"});26]}),
        json!({"fields":[{"name":"","value":"v"}]}),
        json!({"fields":[{"name":"n".repeat(257),"value":"v"}]}),
        json!({"fields":[{"name":"n","value":"v".repeat(1025)}]}),
        json!({"footer":{"text":"x".repeat(2049)}}),
        json!({"image":{"url":"https://example.test/private"}}),
    ] {
        invalid.push(json!({"embeds":[embed]}));
    }
    invalid.push(
        json!({"embeds":[{"description":"x".repeat(3001)},{"description":"x".repeat(3000)}]}),
    );
    for payload in invalid {
        let response = guest
            .send(json_body(Method::POST, &execute(&hook, true), &payload))
            .await;
        assert_eq!(
            response.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "payload {payload}: {}",
            response.text()
        );
    }
    for payload in [
        json!({"content":"x".repeat(50000)}),
        json!({"embeds":vec![json!({"title":"x"});10]}),
        json!({"embeds":[{"description":"x".repeat(3000)},{"description":"x".repeat(3000)}]}),
        json!({"embeds":[{"title":"x".repeat(256),"fields":vec![json!({"name":"n","value":"v"});25]}]}),
    ] {
        let response = guest
            .send(json_body(Method::POST, &execute(&hook, false), &payload))
            .await;
        assert_eq!(
            response.status,
            StatusCode::NO_CONTENT,
            "{}",
            response.text()
        );
    }
}

fn multipart(path: &str, payload: &Value, count: usize, size: usize) -> Req {
    let boundary = "webhook-boundary";
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"payload_json\"\r\n\r\n{payload}\r\n"
    )
    .into_bytes();
    for i in 0..count {
        body.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"files[{i}]\"; filename=\"file{i}.txt\"\r\nContent-Type: text/plain\r\n\r\n").as_bytes());
        body.extend(vec![b'x'; size]);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    Req::new(Method::POST, path)
        .header(
            "content-type",
            &format!("multipart/form-data; boundary={boundary}"),
        )
        .body(body)
}

#[tokio::test]
async fn channel_webhook_multipart_and_workspace_file_limits() {
    let a = app(true)
        .await
        .expect("restored seed")
        .without_job_runner()
        .await;
    let hook = setup(&a, HQ).await;
    let mut guest = a.anonymous();
    for count in [1, 10] {
        let response = guest
            .send(multipart(
                &execute(&hook, true),
                &json!({"username":"Files"}),
                count,
                5,
            ))
            .await;
        assert_eq!(response.status, StatusCode::OK, "{}", response.text());
        let message: Value = parse(&response);
        assert_eq!(message["attachments"].as_array().unwrap().len(), count);
    }
    assert_eq!(
        guest
            .send(multipart(&execute(&hook, false), &json!({}), 11, 5))
            .await
            .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    a.db().write(|tx| { tx.conn().execute("UPDATE accounts SET settings=json_set(COALESCE(settings,'{}'),'$.upload_limit_bytes',1048576)", [])?; Ok(()) }).await.unwrap();
    assert_eq!(
        guest
            .send(multipart(&execute(&hook, false), &json!({}), 1, 1048577))
            .await
            .status,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert_eq!(
        guest
            .send(multipart(&execute(&hook, false), &json!({}), 1, 1048576))
            .await
            .status,
        StatusCode::NO_CONTENT
    );
}

#[tokio::test]
async fn channel_webhook_management_permissions_update_and_test() {
    let a = app(true)
        .await
        .expect("restored seed")
        .without_job_runner()
        .await;
    let room = a
        .db()
        .write(|tx| {
            Ok(campfire_db::Room::create_for(
                tx,
                campfire_db::RoomType::Closed,
                Some("Kevin's channel"),
                KEVIN,
                &[DAVID, KEVIN, MEMBER],
            )?
            .id)
        })
        .await
        .unwrap();
    let mut manager = a.sign_in(KEVIN).await;
    let base = format!("/api/v1/rooms/{room}/webhooks");
    let response = manager
        .write(json_body(Method::POST, &base, &json!({"name":"Original"})))
        .await;
    assert_eq!(response.status, StatusCode::CREATED, "{}", response.text());
    let hook: Value = parse(&response);
    let path = format!("{base}/{}", hook["webhook"]["id"]);
    let mut member = a.sign_in(MEMBER).await;
    assert_eq!(member.send(get(&base)).await.status, StatusCode::FORBIDDEN);
    for (method, url, payload) in [
        (Method::POST, base.clone(), json!({"name":"No"})),
        (Method::PATCH, path.clone(), json!({"name":"No"})),
        (Method::DELETE, path.clone(), json!({})),
        (Method::POST, format!("{path}/regenerate_token"), json!({})),
        (Method::POST, format!("{path}/test"), json!({})),
    ] {
        assert_eq!(
            member.write(json_body(method, &url, &payload)).await.status,
            StatusCode::FORBIDDEN
        );
    }
    let mut admin = a.sign_in(DAVID).await;
    assert_eq!(admin.send(get(&base)).await.status, StatusCode::OK);
    let response = manager
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"name":"Renamed","avatarSignedId":null}),
        ))
        .await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    let updated: Value = parse(&response);
    assert_eq!(updated["name"], "Renamed");
    let response = manager
        .write(json_body(Method::POST, &format!("{path}/test"), &json!({})))
        .await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    let message: Value = parse(&response);
    assert_eq!(message["webhook"]["name"], "Renamed");
    assert!(message["bodyHtml"].as_str().unwrap().contains("test"));
}

#[tokio::test]
async fn channel_webhook_room_delete_cascades_and_bot_key_is_unchanged() {
    let a = app(true)
        .await
        .expect("restored seed")
        .without_job_runner()
        .await;
    let room = a
        .db()
        .write(|tx| {
            Ok(campfire_db::Room::create_for(
                tx,
                campfire_db::RoomType::Closed,
                Some("Temporary"),
                DAVID,
                &[DAVID],
            )?
            .id)
        })
        .await
        .unwrap();
    let hook = setup(&a, room).await;
    let mut admin = a.sign_in(DAVID).await;
    let response = admin
        .write(json_body(
            Method::DELETE,
            &format!("/api/v1/rooms/{room}"),
            &json!({}),
        ))
        .await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    let id = hook["webhook"]["id"].as_i64().unwrap();
    a.db()
        .read(move |conn| {
            assert_eq!(
                conn.query_row(
                    "SELECT COUNT(*) FROM channel_webhooks WHERE id=?",
                    [id],
                    |r| r.get::<_, i64>(0)
                )?,
                0
            );
            Ok(())
        })
        .await
        .unwrap();
    let bot = a
        .db()
        .write(|tx| campfire_db::User::create_bot(tx, "Existing bot", None))
        .await
        .unwrap();
    let mut guest = a.anonymous();
    let response = guest
        .send(
            Req::new(
                Method::POST,
                &format!("/rooms/{HQ}/{}/messages", bot.bot_key()),
            )
            .header("content-type", "text/plain")
            .body("Old bot body"),
        )
        .await;
    assert_eq!(response.status, StatusCode::CREATED, "{}", response.text());
}

#[tokio::test]
async fn channel_webhook_avatar_snapshot_and_patch_presence() {
    let a = app(true)
        .await
        .expect("restored seed")
        .without_job_runner()
        .await;
    let hook = setup(&a, HQ).await;
    let staged = a
        .booted
        .app
        .storage
        .stage_bytes(
            include_bytes!("../../../static_assets/media/images/campfire-icon.png"),
            campfire_storage::Filename::new("avatar.png"),
            Some("image/png"),
        )
        .unwrap();
    let blob = a
        .db()
        .write(move |tx| campfire_runtime::messaging::save_staged(tx, staged))
        .await
        .unwrap();
    let signed =
        campfire_storage::paths::signed_blob_id(&*a.booted.app.storage.verifier, blob.id, None);
    let id = hook["webhook"]["id"].as_i64().unwrap();
    let path = format!("/api/v1/rooms/{HQ}/webhooks/{id}");
    let mut admin = a.sign_in(DAVID).await;
    let response = admin
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"avatarSignedId":signed}),
        ))
        .await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    let updated: Value = parse(&response);
    let avatar = updated["avatarUrl"].clone();
    assert!(avatar.is_string());
    let response = admin
        .write(json_body(Method::PATCH, &path, &json!({"name":"Renamed"})))
        .await;
    let updated: Value = parse(&response);
    assert_eq!(updated["avatarUrl"], avatar);
    let mut guest = a.anonymous();
    let response = guest
        .send(json_body(
            Method::POST,
            &execute(&hook, true),
            &json!({"content":"Avatar"}),
        ))
        .await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    let message: Value = parse(&response);
    assert_eq!(message["webhook"]["avatarUrl"], avatar);
    assert_eq!(message["webhook"]["name"], "Renamed");
    let response = admin
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"avatarSignedId":null}),
        ))
        .await;
    let updated: Value = parse(&response);
    assert!(updated["avatarUrl"].is_null());
    let response = admin
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"avatarSignedId":"not-a-signature"}),
        ))
        .await;
    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
    let text = super::grouped_files_tests::upload(&mut admin, "not-avatar.txt").await;
    let response = admin
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"avatarSignedId":text.signed_id}),
        ))
        .await;
    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        admin
            .write(json_body(Method::DELETE, &path, &json!({})))
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    let read: Value = parse(
        &admin
            .send(get(&format!("/api/v1/messages/{}", message["id"])))
            .await,
    );
    assert_eq!(read["message"]["webhook"], message["webhook"]);
    assert!(read["message"]["attachments"].is_null());
}

#[tokio::test]
async fn channel_webhook_rate_limit_is_per_webhook() {
    let a = app(true)
        .await
        .expect("restored seed")
        .without_job_runner()
        .await;
    let hook = setup(&a, HQ).await;
    let mut guest = a.anonymous();
    for _ in 0..30 {
        let response = guest
            .send(json_body(
                Method::POST,
                &execute(&hook, false),
                &json!({"content":"Rate test"}),
            ))
            .await;
        assert_eq!(
            response.status,
            StatusCode::NO_CONTENT,
            "{}",
            response.text()
        );
    }
    let response = guest
        .send(json_body(
            Method::POST,
            &execute(&hook, false),
            &json!({"content":"Refused"}),
        ))
        .await;
    assert_eq!(response.status, StatusCode::TOO_MANY_REQUESTS);
    assert!(response.headers.contains_key("retry-after"));
    let other = setup(&a, HQ).await;
    assert_eq!(
        guest
            .send(json_body(
                Method::POST,
                &execute(&other, false),
                &json!({"content":"Another webhook"})
            ))
            .await
            .status,
        StatusCode::NO_CONTENT
    );
}

#[tokio::test]
async fn channel_webhook_survives_room_settings_without_joining_the_roster() {
    let a = app(true)
        .await
        .expect("restored seed")
        .without_job_runner()
        .await;
    let room = a
        .db()
        .write(|tx| {
            Ok(campfire_db::Room::create_for(
                tx,
                campfire_db::RoomType::Closed,
                Some("Channel"),
                DAVID,
                &[DAVID, KEVIN],
            )?
            .id)
        })
        .await
        .unwrap();
    let hook = setup(&a, room).await;
    let mut admin = a.sign_in(DAVID).await;
    let response = admin
        .write(json_body(
            Method::PATCH,
            &format!("/api/v1/rooms/{room}"),
            &json!({"type":"closed","name":"Renamed channel","userIds":[DAVID,KEVIN]}),
        ))
        .await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    let mut guest = a.anonymous();
    let response = guest
        .send(json_body(
            Method::POST,
            &execute(&hook, false),
            &json!({"content":"Still works"}),
        ))
        .await;
    assert_eq!(
        response.status,
        StatusCode::NO_CONTENT,
        "{}",
        response.text()
    );
    let detail: api::RoomDetail = parse(&admin.send(get(&format!("/api/v1/rooms/{room}"))).await);
    assert_eq!(detail.member_count, 2);
    assert_eq!(detail.member_preview_ids.len(), 2);
}
