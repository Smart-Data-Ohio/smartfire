use super::api_tests::{Sync, app, get, json_body, parse, serve};
use crate::controllers::presenters::test_support::{
    BENDER, DAVID, DIRECT_DAVID_JASON, JASON, KEVIN,
};
use axum::http::{Method, StatusCode};
use serde_json::{Value, json};

const DESIGNERS: i64 = 654632876;

#[tokio::test]
async fn ignore_saved_reminder_api_carries_the_marking_without_a_badge() {
    use campfire_db::{Message, NewMessage, NewSavedItem, SavedItem};
    let Some(a) = app(true).await else { return };
    let a = a.without_job_runner().await;
    let mut david = a.sign_in(DAVID).await;
    a.db()
        .write(|tx| {
            tx.conn()
                .execute("DELETE FROM activity_items WHERE user_id=?", [DAVID])?;
            let message = Message::create(
                tx,
                NewMessage {
                    room_id: DESIGNERS,
                    creator_id: JASON,
                    markdown_source: Some("Ignored reminder excerpt".into()),
                    ..Default::default()
                },
            )?;
            let saved = SavedItem::create(
                tx,
                NewSavedItem {
                    user_id: DAVID,
                    message_id: message.id,
                    remind_at: Some(tx.now().since(jiff::SignedDuration::from_mins(1))),
                    ..Default::default()
                },
            )?;
            SavedItem::dispatch_reminder(
                tx,
                saved.id,
                tx.now().since(jiff::SignedDuration::from_mins(2)),
            )?;
            campfire_db::models::user_block::UserBlock::set(tx, DAVID, JASON, true)?;
            Ok(())
        })
        .await
        .unwrap();
    let page: Value = parse(&david.send(get("/api/v1/activity?type=reminders")).await);
    assert_eq!(page["items"].as_array().unwrap().len(), 1);
    assert_eq!(page["items"][0]["source"]["ignored"], true);
    assert_eq!(page["unreadCount"], 0);
    let count: Value = parse(&david.send(get("/api/v1/activity/unread_count")).await);
    assert_eq!(count["unreadCount"], 0);
}

#[tokio::test]
async fn ignore_does_not_change_the_authors_observed_edit_sequence() {
    use campfire_api_types::SyncPayload;
    let Some(a) = app(true).await else { return };
    let a = a.without_job_runner().await;
    let david = a.sign_in(DAVID).await;
    let jason = a.sign_in(JASON).await;
    let (addr, server) = serve(&a).await;
    let topics = [format!("room:{DESIGNERS}")];
    let mut blocker = Sync::connect(addr, &david.cookie_header(), &topics).await;
    let mut author = Sync::connect(addr, &jason.cookie_header(), &topics).await;
    blocker.welcome().await;
    author.welcome().await;
    let mut sequences = Vec::new();
    for ignored in [false, false, true, true] {
        a.db()
            .write(move |tx| {
                campfire_db::models::user_block::UserBlock::set(tx, DAVID, JASON, ignored)?;
                Ok(())
            })
            .await
            .unwrap();
        let broadcasts = a.booted.app.broadcasts.clone();
        a.db().read(move |conn| {
            let id = conn.query_row("SELECT id FROM messages WHERE room_id=? AND creator_id=? AND thread_id IS NULL LIMIT 1", [DESIGNERS, JASON], |row| row.get(0))?;
            broadcasts.sync_message(conn, &campfire_db::Message::find(conn, id)?, false);
            Ok(())
        }).await.unwrap();
        let wanted = |event: &campfire_api_types::SyncEvent| {
            matches!(event.payload, SyncPayload::MessageUpdated(_))
        };
        let public = author.until(wanted, |_| false).await;
        let private = blocker.until(wanted, |_| false).await;
        assert_eq!(public.seq, private.seq);
        sequences.push(public.seq);
        let SyncPayload::MessageUpdated(message) = private.payload else {
            unreachable!()
        };
        assert_eq!(message.ignored, ignored.then_some(true));
    }
    assert_eq!(
        sequences,
        (sequences[0]..sequences[0] + 4).collect::<Vec<_>>()
    );
    server.abort();
}

#[tokio::test]
async fn ignore_endpoints_are_private_idempotent_and_allow_bots_but_not_self() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    for target in [JASON, BENDER] {
        for _ in 0..2 {
            let reply = david
                .write(json_body(
                    Method::PUT,
                    &format!("/api/v1/people/{target}/block"),
                    &json!({}),
                ))
                .await;
            assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
            assert_eq!(parse::<Value>(&reply)["ignoredByMe"], true);
        }
    }
    let list: Value = parse(&david.send(get("/api/v1/blocks")).await);
    assert_eq!(list["people"].as_array().unwrap().len(), 2);
    let directory: Value = parse(&david.send(get("/api/v1/people")).await);
    for target in [JASON, BENDER] {
        let person = directory["people"]
            .as_array()
            .unwrap()
            .iter()
            .find(|person| person["userId"] == target)
            .unwrap();
        assert_eq!(person["ignoredByMe"], true);
    }
    let members: Value = parse(
        &david
            .send(get(&format!("/api/v1/rooms/{DESIGNERS}/members")))
            .await,
    );
    assert!(
        members["users"]
            .as_array()
            .unwrap()
            .iter()
            .any(|user| user["id"] == JASON)
    );
    assert_eq!(
        parse::<Value>(&kevin.send(get("/api/v1/blocks")).await)["people"],
        json!([])
    );
    let profile: Value = parse(&kevin.send(get(&format!("/api/v1/people/{JASON}"))).await);
    assert_eq!(profile["ignoredByMe"], false);
    let own = david
        .write(json_body(
            Method::PUT,
            &format!("/api/v1/people/{DAVID}/block"),
            &json!({}),
        ))
        .await;
    assert_eq!(own.status, StatusCode::UNPROCESSABLE_ENTITY);
    for _ in 0..2 {
        let reply = david
            .write(json_body(
                Method::DELETE,
                &format!("/api/v1/people/{JASON}/block"),
                &json!({}),
            ))
            .await;
        assert_eq!(reply.status, StatusCode::OK);
        assert_eq!(parse::<Value>(&reply)["ignoredByMe"], false);
    }
}

#[tokio::test]
async fn ignore_marks_timelines_threads_and_existing_direct_history_only_for_viewer() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let parent_id = a.db().write(|tx| {
        let parent_id: i64 = tx.conn().query_row("SELECT parent_message_id FROM channel_threads WHERE id=1", [], |row| row.get(0))?;
        tx.conn().execute("UPDATE messages SET creator_id=? WHERE id=?", [JASON, parent_id])?;
        tx.conn().execute("INSERT OR IGNORE INTO message_pins(room_id,message_id,pinner_id,created_at) VALUES (?,?,?,?)", rusqlite::params![DESIGNERS, parent_id, DAVID, tx.now()])?;
        Ok(parent_id)
    }).await.unwrap();
    let reply = david
        .write(json_body(
            Method::PUT,
            &format!("/api/v1/people/{JASON}/block"),
            &json!({}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    for path in [
        format!("/api/v1/rooms/{DESIGNERS}/messages"),
        "/api/v1/threads/1/messages".into(),
        format!("/api/v1/rooms/{DIRECT_DAVID_JASON}/messages"),
    ] {
        let page: Value = parse(&david.send(get(&path)).await);
        let messages = page["messages"].as_array().unwrap();
        assert!(messages.iter().any(|m| m["creatorId"] == JASON));
        for message in messages {
            assert_eq!(
                message["ignored"].as_bool().unwrap_or(false),
                message["creatorId"] == JASON
            );
        }
    }
    let page: Value = parse(&kevin.send(get("/api/v1/threads/1/messages")).await);
    assert!(
        page["messages"]
            .as_array()
            .unwrap()
            .iter()
            .all(|m| !m["ignored"].as_bool().unwrap_or(false))
    );
    let detail: Value = parse(&david.send(get("/api/v1/threads/1")).await);
    assert_eq!(detail["parentMessage"]["id"], parent_id);
    assert_eq!(detail["parentMessage"]["ignored"], true);
    let detail: Value = parse(&kevin.send(get("/api/v1/threads/1")).await);
    assert!(
        !detail["parentMessage"]["ignored"]
            .as_bool()
            .unwrap_or(false)
    );
    let pins: Value = parse(
        &david
            .send(get(&format!("/api/v1/rooms/{DESIGNERS}/pins")))
            .await,
    );
    let parent = pins["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|message| message["id"] == parent_id)
        .unwrap();
    assert_eq!(parent["ignored"], true);
}

#[tokio::test]
async fn ignore_search_excludes_author_until_explicitly_filtered() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    assert_eq!(
        david
            .write(json_body(
                Method::PUT,
                &format!("/api/v1/people/{JASON}/block"),
                &json!({})
            ))
            .await
            .status,
        StatusCode::OK
    );
    let page: Value = parse(
        &david
            .send(get(&format!("/api/v1/search?q=in_id:{DESIGNERS}")))
            .await,
    );
    assert!(
        page["messages"]
            .as_array()
            .unwrap()
            .iter()
            .all(|m| m["creatorId"] != JASON)
    );
    for filter in [format!("from_id:{JASON}"), "from:Jason".into()] {
        let page: Value = parse(&david.send(get(&format!("/api/v1/search?q={filter}"))).await);
        assert!(!page["messages"].as_array().unwrap().is_empty());
        assert!(
            page["messages"]
                .as_array()
                .unwrap()
                .iter()
                .all(|m| m["creatorId"] == JASON && m["ignored"] == true)
        );
    }
}

#[tokio::test]
async fn ignore_refuses_new_contact_generically_but_allows_blocker_and_existing_dm() {
    let Some(a) = app(true).await else { return };
    let mut kevin = a.sign_in(KEVIN).await;
    let mut jason = a.sign_in(JASON).await;
    assert_eq!(
        kevin
            .write(json_body(
                Method::PUT,
                &format!("/api/v1/people/{JASON}/block"),
                &json!({})
            ))
            .await
            .status,
        StatusCode::OK
    );
    let refused = jason
        .write(json_body(
            Method::POST,
            "/api/v1/directs",
            &json!({"userIds": [KEVIN]}),
        ))
        .await;
    let unavailable = jason
        .write(json_body(
            Method::POST,
            "/api/v1/directs",
            &json!({"userIds": [999999999]}),
        ))
        .await;
    assert_eq!(refused.status, StatusCode::NOT_FOUND);
    assert_eq!(refused.status, unavailable.status);
    assert_eq!(parse::<Value>(&refused), parse::<Value>(&unavailable));
    let refused_group = jason
        .write(json_body(
            Method::POST,
            "/api/v1/directs",
            &json!({"userIds": [KEVIN, DAVID]}),
        ))
        .await;
    let missing_group = jason
        .write(json_body(
            Method::POST,
            "/api/v1/directs",
            &json!({"userIds": [999999999, DAVID]}),
        ))
        .await;
    assert_eq!(refused_group.status, missing_group.status);
    assert_eq!(
        parse::<Value>(&refused_group),
        parse::<Value>(&missing_group)
    );
    let group = a
        .db()
        .write(|tx| campfire_db::Room::find_or_create_direct_for(tx, &[JASON, DAVID, BENDER], JASON))
        .await
        .unwrap();
    let members_path = format!("/api/v1/directs/{}/members", group.id);
    let refused_member = jason
        .write(json_body(Method::POST, &members_path, &json!({"userIds": [KEVIN]})))
        .await;
    let missing_member = jason
        .write(json_body(Method::POST, &members_path, &json!({"userIds": [999999999]})))
        .await;
    assert_eq!(refused_member.status, StatusCode::NOT_FOUND);
    assert_eq!(refused_member.status, missing_member.status);
    assert_eq!(parse::<Value>(&refused_member), parse::<Value>(&missing_member));
    let allowed = kevin
        .write(json_body(
            Method::POST,
            "/api/v1/directs",
            &json!({"userIds": [JASON]}),
        ))
        .await;
    assert_eq!(allowed.status, StatusCode::CREATED, "{}", allowed.text());
    let existing = jason
        .write(json_body(
            Method::POST,
            "/api/v1/directs",
            &json!({"userIds": [KEVIN]}),
        ))
        .await;
    assert_eq!(existing.status, StatusCode::OK);
}

#[tokio::test]
async fn ignore_changes_sync_only_to_blockers_other_sessions() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let (addr, server) = serve(&a).await;
    let mut other_session = Sync::connect(addr, &david.cookie_header(), &[]).await;
    other_session.welcome().await;
    for ignored in [true, false] {
        let method = if ignored { Method::PUT } else { Method::DELETE };
        let reply = david
            .write(json_body(
                method,
                &format!("/api/v1/people/{JASON}/block"),
                &json!({}),
            ))
            .await;
        assert_eq!(reply.status, StatusCode::OK);
        let event = other_session
            .until(
                |event| serde_json::to_value(&event.payload).unwrap()["type"] == "blocks.updated",
                |_| false,
            )
            .await;
        assert_eq!(event.topic, "user");
        let data = serde_json::to_value(&event.payload).unwrap();
        assert_eq!(data["data"]["userId"], JASON);
        assert_eq!(data["data"]["ignoredByMe"], ignored);
    }
    server.abort();
}

#[tokio::test]
async fn ignore_live_messages_are_marked_privately_for_room_and_thread_subscribers() {
    use campfire_api_types::SyncPayload;
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let kevin = a.sign_in(KEVIN).await;
    let mut jason = a.sign_in(JASON).await;
    let (addr, server) = serve(&a).await;
    let topics = [format!("room:{DESIGNERS}"), "thread:1".into()];
    let mut blocker = Sync::connect(addr, &david.cookie_header(), &topics).await;
    let mut other = Sync::connect(addr, &kevin.cookie_header(), &topics).await;
    blocker.welcome().await;
    other.welcome().await;
    assert_eq!(
        david
            .write(json_body(
                Method::PUT,
                &format!("/api/v1/people/{JASON}/block"),
                &json!({})
            ))
            .await
            .status,
        StatusCode::OK
    );
    for (path, client) in [
        (
            format!("/api/v1/rooms/{DESIGNERS}/messages"),
            "ignore-live-room",
        ),
        ("/api/v1/threads/1/messages".into(), "ignore-live-thread"),
    ] {
        let posted = jason
            .write(json_body(
                Method::POST,
                &path,
                &json!({"clientMessageId": client, "markdownSource": "Ignored live message"}),
            ))
            .await;
        assert_eq!(posted.status, StatusCode::CREATED, "{}", posted.text());
        let id = parse::<Value>(&posted)["id"].as_i64().unwrap();
        let wanted = |e: &campfire_api_types::SyncEvent| matches!(&e.payload, SyncPayload::MessageCreated(m) if m.id == id);
        let blocked = blocker
            .until(wanted, |e| {
                matches!(&e.payload,
                    SyncPayload::RoomUnread(row) if row.room_id == DESIGNERS
                ) || matches!(&e.payload, SyncPayload::ThreadUnread(row) if row.thread_id == 1)
            })
            .await;
        let normal = other
            .until(wanted, |e| {
                matches!(e.payload, SyncPayload::BlocksUpdated(_))
            })
            .await;
        assert_eq!(blocked.seq, normal.seq, "personalized copies share one publication");
        let SyncPayload::MessageCreated(hidden) = blocked.payload else {
            unreachable!()
        };
        let SyncPayload::MessageCreated(visible) = normal.payload else {
            unreachable!()
        };
        assert_eq!(hidden.ignored, Some(true));
        assert_eq!(hidden.creator_id, JASON);
        assert_eq!(visible.ignored, None);
        assert_eq!(hidden.body_html, visible.body_html);
    }
    server.abort();
}

#[tokio::test]
async fn ignore_sidebar_badges_exclude_old_and_new_messages_and_mentions() {
    use campfire_db::{ActivityItem, Message, NewMessage};
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    a.db().write(|tx| {
        tx.conn().execute("DELETE FROM activity_items WHERE user_id=?", [DAVID])?;
        tx.conn().execute("UPDATE memberships SET unread_at=NULL,last_read_message_id=(SELECT id FROM messages WHERE room_id=memberships.room_id AND thread_id IS NULL ORDER BY created_at DESC,id DESC LIMIT 1) WHERE user_id=?", [DAVID])?;
        for author in [JASON, KEVIN] {
            let message = Message::create(tx, NewMessage { room_id: DESIGNERS, creator_id: author, markdown_source: Some(format!("<@{DAVID}>")), ..Default::default() })?;
            ActivityItem::record_message(tx, &message)?;
        }
        Ok(())
    }).await.unwrap();
    let before: Value = parse(&david.send(get("/api/v1/sidebar")).await);
    assert_eq!(
        david
            .write(json_body(
                Method::PUT,
                &format!("/api/v1/people/{JASON}/block"),
                &json!({})
            ))
            .await
            .status,
        StatusCode::OK
    );
    let after: Value = parse(&david.send(get("/api/v1/sidebar")).await);
    let row = |page: &Value| {
        page["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["room"]["id"] == DESIGNERS)
            .unwrap()
            .clone()
    };
    for key in ["unreadCount", "mentionCount", "notificationCount"] {
        assert_eq!(
            row(&before)[key].as_i64().unwrap(),
            row(&after)[key].as_i64().unwrap() + 1,
            "{key}"
        );
    }
    let count_before: Value = parse(&david.send(get("/api/v1/activity/unread_count")).await);
    a.db()
        .write(|tx| {
            let message = Message::create(
                tx,
                NewMessage {
                    room_id: DESIGNERS,
                    creator_id: JASON,
                    markdown_source: Some(format!("<@{DAVID}>")),
                    ..Default::default()
                },
            )?;
            ActivityItem::record_message(tx, &message)?;
            Ok(())
        })
        .await
        .unwrap();
    let newest: Value = parse(&david.send(get("/api/v1/sidebar")).await);
    for key in ["unreadCount", "mentionCount", "notificationCount"] {
        assert_eq!(row(&newest)[key], row(&after)[key], "{key}");
    }
    let count_after: Value = parse(&david.send(get("/api/v1/activity/unread_count")).await);
    assert_eq!(count_after["unreadCount"], count_before["unreadCount"]);
}
