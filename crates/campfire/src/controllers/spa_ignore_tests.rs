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
            assert_eq!(parse::<Value>(&reply)["viewerId"], DAVID);
        }
    }
    let list: Value = parse(&david.send(get("/api/v1/blocks")).await);
    assert_eq!(list["people"].as_array().unwrap().len(), 2);
    // The list says whose it is, so a client can tell another sign-in's answer apart.
    assert_eq!(list["viewerId"], DAVID);
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
    let kevins: Value = parse(&kevin.send(get("/api/v1/blocks")).await);
    assert_eq!(kevins["people"], json!([]));
    assert_eq!(kevins["viewerId"], KEVIN);
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
        assert_eq!(data["data"]["viewerId"], DAVID);
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
                    SyncPayload::RoomUnread(row) if row.room_id == DESIGNERS && row.unchanged != Some(true)
                ) || matches!(&e.payload, SyncPayload::ThreadUnread(row) if row.thread_id == 1 && !row.refresh_only)
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

#[tokio::test]
async fn ignore_html_direct_targets_match_missing_people_on_create_and_add() {
    use crate::controllers::presenters::test_support::Req;
    let a = app(true).await.expect("seed required").without_job_runner().await;
    let group = a.db().write(|tx| {
        let room = campfire_db::Room::find_or_create_direct_for(tx, &[JASON, DAVID, BENDER], JASON)?;
        campfire_db::models::user_block::UserBlock::set(tx, KEVIN, JASON, true)?;
        Ok(room)
    }).await.unwrap();
    let mut jason = a.sign_in(JASON).await;
    for path in ["/rooms/directs".to_owned(), format!("/rooms/directs/{}/add_members", group.id), format!("/rooms/directs/{DIRECT_DAVID_JASON}/add_members")] {
        for extra in [None, Some(DAVID)] {
            let mut replies = Vec::new();
            for target in [KEVIN, 999999999] {
                let target = target.to_string();
                let extra = extra.map(|id| id.to_string());
                let mut fields = vec![("user_ids[]", target.as_str())];
                if let Some(extra) = &extra { fields.push(("user_ids[]", extra.as_str())); }
                replies.push(jason.write(Req::new(Method::POST, &path).form(&fields)).await);
            }
            assert_eq!(replies[0].status, StatusCode::NOT_FOUND, "{path}");
            assert_eq!(replies[0].status, replies[1].status, "{path}");
            assert_eq!(replies[0].text(), replies[1].text(), "{path}");
        }
    }
}

#[tokio::test]
async fn ignore_mixed_workload_preserves_the_authors_sequences_and_event_types() {
    use campfire_db::{ChannelThread, Membership, NewChannelThread, Room, RoomType, ThreadMembership, ThreadInvolvement};
    use campfire_api_types::SyncPayload;
    let mut observed = Vec::new();
    for ignored in [false, true] {
        let a = app(true).await.expect("seed required").without_job_runner().await;
        let (room, board, thread) = a.db().write(move |tx| {
            let users = [JASON, DAVID];
            let room = Room::create_for(tx, RoomType::Closed, Some("Sequence room"), JASON, &users)?;
            let board = Room::create_for(tx, RoomType::Board, Some("Sequence board"), JASON, &users)?;
            let thread = ChannelThread::create(tx, NewChannelThread { room_id: room.id, creator_id: JASON, name: Some("Sequence thread".into()), ..Default::default() })?;
            ThreadMembership::join(tx, thread.id, DAVID)?.update_involvement(tx, ThreadInvolvement::Everything)?;
            Membership::find_by_room_and_user(tx.conn(), room.id, DAVID)?.unwrap().read(tx)?;
            tx.conn().execute("UPDATE memberships SET connected_at=NULL,connections=0 WHERE room_id=? AND user_id=?", [room.id, DAVID])?;
            campfire_db::models::user_block::UserBlock::set(tx, DAVID, JASON, ignored)?;
            Ok((room.id, board.id, thread.id))
        }).await.unwrap();
        let mut jason = a.sign_in(JASON).await;
        let mut david = a.sign_in(DAVID).await;
        a.booted.app.broadcasts.settle_sync().await;
        let (addr, server) = serve(&a).await;
        let topics = [format!("room:{room}"), format!("room:{board}"), format!("thread:{thread}")];
        let mut author = Sync::connect(addr, &jason.cookie_header(), &topics).await;
        let mut blocker = Sync::connect(addr, &david.cookie_header(), &topics).await;
        author.welcome().await;
        blocker.welcome().await;
        let marker = || a.booted.app.cable.sync_publish(campfire_cable::sync::SyncPublication::new(
            campfire_cable::sync::Audience::Topic(format!("room:{room}")),
            serde_json::to_string(&SyncPayload::RoomRead(campfire_api_types::RoomRead { room_id: room })).unwrap(),
        ));
        marker();
        let start = author.until(|e| matches!(e.payload, SyncPayload::RoomRead(_)), |_| false).await.seq;
        let reply = jason.write(json_body(Method::POST, &format!("/api/v1/rooms/{room}/messages"), &json!({"clientMessageId":"sequence-post", "markdownSource":"A post"}))).await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
        let message = parse::<Value>(&reply)["id"].as_i64().unwrap();
        a.booted.app.broadcasts.settle_sync().await;
        let events = std::cell::RefCell::new(Vec::new());
        let mut checkpoints = Vec::new();
        marker();
        let first = author.until(|e| {
            if matches!(e.payload, SyncPayload::RoomRead(_)) { return true; }
            events.borrow_mut().push(serde_json::to_value(&e.payload).unwrap()["type"].as_str().unwrap().to_owned());
            false
        }, |_| false).await;
        checkpoints.push(first.seq - start);
        assert_eq!(a.db().read(move |conn| {
            let membership = Membership::find_by_room_and_user(conn, room, DAVID)?.unwrap();
            Ok(Membership::find_stored(conn, membership.id)?.unread())
        }).await.unwrap(), !ignored, "chat unread state differs before muting");
        for (method, path, body) in [
            (Method::PUT, format!("/api/v1/rooms/{room}/involvement"), json!({"involvement":"muted"})),
            (Method::PUT, format!("/api/v1/rooms/{room}/involvement"), json!({"involvement":"everything"})),
            (Method::PATCH, format!("/api/v1/messages/{message}"), json!({"markdownSource":"Edited post"})),
            (Method::POST, format!("/api/v1/threads/{thread}/messages"), json!({"clientMessageId":"sequence-reply", "markdownSource":"A reply"})),
            (Method::POST, format!("/api/v1/rooms/{board}/posts"), json!({"name":"Board post", "status":"planned", "ownerId":null, "tags":[], "message":{"clientMessageId":"sequence-board", "markdownSource":"Board opener"}})),
            (Method::PUT, format!("/api/v1/rooms/{board}/involvement"), json!({"involvement":"muted"})),
            (Method::PUT, format!("/api/v1/rooms/{board}/involvement"), json!({"involvement":"everything"})),
            (Method::POST, format!("/api/v1/threads/{thread}/messages"), json!({"clientMessageId":"sequence-reply-two", "markdownSource":"Another reply"})),
            (Method::PATCH, format!("/api/v1/messages/{message}"), json!({"markdownSource":format!("After mute <@{DAVID}>")})),
            (Method::PATCH, format!("/api/v1/messages/{message}"), json!({"markdownSource":"Remove mention"})),
            (Method::POST, format!("/api/v1/messages/{message}/boosts"), json!({"content":"👍"})),
            (Method::DELETE, format!("/api/v1/messages/{message}"), json!({})),
            (Method::DELETE, format!("/api/v1/threads/{thread}"), json!({})),
        ] {
            let client = if path.ends_with("/involvement") { &mut david } else { &mut jason };
            let reply = client.write(json_body(method, &path, &body)).await;
            assert!(reply.status.is_success(), "{path}: {}", reply.text());
            a.booted.app.broadcasts.settle_sync().await;
            // Deferred recipient snapshots can interleave within an operation. The next
            // author-visible publication must have the same sequence after every operation.
            marker();
            let checkpoint = author.until(|e| {
                if matches!(e.payload, SyncPayload::RoomRead(_)) { return true; }
                events.borrow_mut().push(serde_json::to_value(&e.payload).unwrap()["type"].as_str().unwrap().to_owned());
                false
            }, |_| false).await;
            checkpoints.push(checkpoint.seq - start);
        }
        assert!(events.borrow().iter().any(|kind| kind == "message.created"));
        assert!(events.borrow().iter().any(|kind| kind == "message.updated"));
        assert!(events.borrow().iter().any(|kind| kind == "message.reactions"));
        let mut kinds = events.into_inner();
        kinds.sort();
        observed.push((checkpoints, kinds));
        server.abort();
    }
    assert_eq!(observed[0], observed[1]);
}

#[tokio::test]
async fn ignore_unread_fanout_batches_policy_through_the_real_sync_sink() {
    use campfire_db::{Message, NewMessage, NewUser, Room, RoomType, User};
    let a = app(true).await.expect("seed required").without_job_runner().await;
    let (room, message) = a.db().write(|tx| {
        let mut users = vec![JASON, DAVID];
        for n in 0..28 {
            users.push(User::create(tx, NewUser { name: format!("Sync recipient {n}"), ..Default::default() })?.id);
        }
        let room = Room::create_for(tx, RoomType::Closed, Some("Thirty sync recipients"), JASON, &users)?;
        let message = Message::create(tx, NewMessage { room_id: room.id, creator_id: JASON, markdown_source: Some("Fanout post".into()), ..Default::default() })?;
        campfire_db::models::user_block::UserBlock::set(tx, DAVID, JASON, true)?;
        Ok((room.id, message))
    }).await.unwrap();
    let david = a.sign_in(DAVID).await;
    let (addr, server) = serve(&a).await;
    let mut sync = Sync::connect(addr, &david.cookie_header(), &[format!("room:{room}")]).await;
    sync.welcome().await;
    let queries = a.db().capture_read_queries();
    a.db().write(move |tx| campfire_db::models::agent_posting::broadcast_unread_room(tx, &message)).await.unwrap();
    // Sink delivery is synchronous; deferred sidebar readers are outside this policy budget.
    a.db().stop_capturing_read_queries();
    let count = queries.lock().unwrap().iter().filter(|sql| sql.starts_with("SELECT blocker_id FROM user_blocks") || sql.starts_with("SELECT 1 FROM user_blocks")).count();
    assert_eq!(count, 1, "one lookup for thirty recipients through the live sink");
    let event = sync.until(|e| matches!(e.payload, campfire_api_types::SyncPayload::RoomUnread(_)), |_| false).await;
    assert_eq!(serde_json::to_value(event.payload).unwrap()["data"]["unchanged"], true);
    server.abort();
}

#[tokio::test]
async fn ignore_board_unread_publication_keeps_its_slot_and_unchanged_state() {
    use campfire_db::{ChannelThread, NewChannelThread, Room, RoomType};
    let a = app(true).await.expect("seed required").without_job_runner().await;
    let room = a.db().write(|tx| {
        let room = Room::create_for(tx, RoomType::Board, Some("Board policy"), JASON, &[JASON, DAVID])?;
        campfire_db::models::user_block::UserBlock::set(tx, DAVID, JASON, true)?;
        Ok(room.id)
    }).await.unwrap();
    let mut david = a.sign_in(DAVID).await;
    let (addr, server) = serve(&a).await;
    let mut sync = Sync::connect(addr, &david.cookie_header(), &[format!("room:{room}")]).await;
    sync.welcome().await;
    for opener in [None, Some("Board opener".to_owned())] {
        a.db().write(move |tx| {
            ChannelThread::create_board_post(tx, NewChannelThread { room_id: room, creator_id: JASON, name: Some("Ignored post".into()), work_status: Some("planned".into()), ..Default::default() }, opener)?;
            Ok(())
        }).await.unwrap();
        let event = sync.until(|e| matches!(e.payload, campfire_api_types::SyncPayload::RoomUnread(_)), |_| false).await;
        assert_eq!(serde_json::to_value(event.payload).unwrap()["data"]["unchanged"], true);
        a.booted.app.broadcasts.settle_sync().await;
        let row = sync.until(|e| matches!(&e.payload, campfire_api_types::SyncPayload::SidebarRowUpserted(row) if row.room.id == room), |_| false).await;
        assert!(serde_json::to_value(row.payload).unwrap()["data"]["membership"]["unreadAt"].is_null());
        let sidebar: Value = parse(&david.send(get("/api/v1/sidebar")).await);
        let row = sidebar["rows"].as_array().unwrap().iter().find(|row| row["room"]["id"] == room).unwrap();
        assert!(row["membership"]["unreadAt"].is_null());
        let detail: Value = parse(&david.send(get(&format!("/api/v1/rooms/{room}"))).await);
        assert!(detail["membership"]["unreadAt"].is_null());
    }
    server.abort();
}

#[tokio::test]
async fn ignore_activity_keeps_the_global_slot_with_both_sync_sockets_open() {
    use campfire_api_types::SyncPayload;
    let mut increments = Vec::new();
    for ignored in [false, true] {
        let a = app(true).await.expect("seed required").without_job_runner().await;
        let room = a.db().write(move |tx| {
            let room = campfire_db::Room::create_for(tx, campfire_db::RoomType::Closed, Some("Activity sequence"), JASON, &[JASON, DAVID])?;
            campfire_db::models::user_block::UserBlock::set(tx, DAVID, JASON, ignored)?;
            Ok(room.id)
        }).await.unwrap();
        let mut jason = a.sign_in(JASON).await;
        let mut david = a.sign_in(DAVID).await;
        let before: Value = parse(&david.send(get("/api/v1/activity/unread_count")).await);
        a.booted.app.broadcasts.settle_sync().await;
        let (addr, server) = serve(&a).await;
        let topics = [format!("room:{room}")];
        let mut author = Sync::connect(addr, &jason.cookie_header(), &topics).await;
        let mut blocker = Sync::connect(addr, &david.cookie_header(), &topics).await;
        author.welcome().await;
        blocker.welcome().await;
        let marker = || a.booted.app.cable.sync_publish(campfire_cable::sync::SyncPublication::new(
            campfire_cable::sync::Audience::Topic(format!("room:{room}")),
            serde_json::to_string(&SyncPayload::RoomRead(campfire_api_types::RoomRead { room_id: room })).unwrap(),
        ));
        marker();
        let start = author.until(|e| matches!(e.payload, SyncPayload::RoomRead(_)), |_| false).await.seq;
        let reply = jason.write(json_body(Method::POST, &format!("/api/v1/rooms/{room}/messages"), &json!({"clientMessageId":"activity-sequence", "markdownSource":format!("Hello <@{DAVID}>")}))).await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
        a.booted.app.broadcasts.settle_sync().await;
        marker();
        let end = author.until(|e| matches!(e.payload, SyncPayload::RoomRead(_)), |_| false).await.seq;
        increments.push(end - start);
        let event = blocker.until(|e| matches!(e.payload, SyncPayload::ActivityItem(_)), |_| false).await;
        let SyncPayload::ActivityItem(changed) = event.payload else { unreachable!() };
        assert_eq!(changed.item.is_none(), ignored);
        if ignored { assert_eq!(changed.unread_count, before["unreadCount"].as_i64().unwrap()); }
        server.abort();
    }
    assert_eq!(increments[0], increments[1]);
}


#[tokio::test]
async fn ignore_approval_updates_preserve_the_authors_sequence_and_hide_the_blockers_card() {
    use campfire_api_types::SyncPayload;
    use campfire_db::{AgentApproval, NewApproval};
    let mut increments = Vec::new();
    for ignored in [false, true] {
        let a = app(true).await.expect("seed required").without_job_runner().await;
        let approval = a.db().write(move |tx| {
            campfire_db::models::user_block::UserBlock::set(tx, DAVID, BENDER, ignored)?;
            AgentApproval::create(tx, NewApproval { agent_id: 773018776, action: "ignore.audit".into(), summary: "Approval slot".into(), ..Default::default() })
        }).await.unwrap();
        let jason = a.sign_in(JASON).await;
        let david = a.sign_in(DAVID).await;
        a.booted.app.broadcasts.settle_sync().await;
        let (addr, server) = serve(&a).await;
        let topic = format!("room:{DESIGNERS}");
        let mut author = Sync::connect(addr, &jason.cookie_header(), std::slice::from_ref(&topic)).await;
        let mut blocker = Sync::connect(addr, &david.cookie_header(), &[]).await;
        author.welcome().await;
        blocker.welcome().await;
        let marker = || a.booted.app.cable.sync_publish(campfire_cable::sync::SyncPublication::new(
            campfire_cable::sync::Audience::Topic(topic.clone()),
            serde_json::to_string(&SyncPayload::RoomRead(campfire_api_types::RoomRead { room_id: DESIGNERS })).unwrap(),
        ));
        marker();
        let start = author.until(|e| matches!(e.payload, SyncPayload::RoomRead(_)), |_| false).await.seq;
        a.booted.app.broadcasts.sync_approval(approval.id);
        a.booted.app.broadcasts.settle_sync().await;
        marker();
        let end = author.until(|e| matches!(e.payload, SyncPayload::RoomRead(_)), |_| false).await.seq;
        increments.push(end - start);
        let event = blocker.until(|e| matches!(e.payload, SyncPayload::ApprovalUpdated(_) | SyncPayload::ActivityItem(_)), |_| false).await;
        if ignored { assert!(matches!(event.payload, SyncPayload::ActivityItem(changed) if changed.item.is_none())); }
        else { assert!(matches!(event.payload, SyncPayload::ApprovalUpdated(_))); }
        server.abort();
    }
    assert_eq!(increments[0], increments[1]);
}
