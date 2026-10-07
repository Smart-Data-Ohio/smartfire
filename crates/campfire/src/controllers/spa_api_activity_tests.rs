//! The S3 inbox, saved and scheduled endpoints on `/api/v1` (`campfire_api::activity`,
//! `message_actions` saved, `composer` scheduled): each answers in the contract's shape, pages
//! by cursor, and publishes its JSON twin (`activity.item`, `activity.removed`,
//! `scheduled.changed`, `scheduled.removed`, `saved.changed`) on the owner's `user` topic.

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use serde_json::json;

use super::api_tests::{BOOSTED, Sync, app, get, json_body, parse, serve, tag};
use crate::controllers::presenters::test_support::{DAVID, JASON, KEVIN, Req, TestApp};

const DESIGNERS: i64 = 654632876;
const THREAD: i64 = 1;
/// Jason's reply in Designers' thread 1.
const JASONS_REPLY: i64 = 935962046;
const ROOT_MESSAGE: i64 = 935962057;
const LATER: &str = "2030-01-01T09:00:00Z";

/// Inserts an inbox item, answering its id.
async fn item(
    a: &TestApp,
    user_id: i64,
    source: (&'static str, i64),
    event_type: &'static str,
    age_seconds: i64,
) -> i64 {
    let at =
        campfire_db::Timestamp::from_second(a.booted.app.db.env().now().as_second() - age_seconds);
    a.db()
        .write(move |tx| {
            Ok(tx.conn().query_row(
                "INSERT INTO activity_items (user_id, source_type, source_id, event_type, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?) RETURNING id",
                rusqlite::params![user_id, source.0, source.1, event_type, at, at],
                |row| row.get(0),
            )?)
        })
        .await
        .unwrap()
}

fn validation_fields(reply: &crate::controllers::presenters::test_support::Reply) -> Vec<String> {
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.text()
    );
    let api::ApiError::Validation { fields, .. } = parse::<api::ApiErrorResponse>(reply).error
    else {
        panic!("{}", reply.text())
    };
    fields.into_keys().collect()
}

fn activity_item(id: i64) -> impl Fn(&api::SyncEvent) -> bool {
    move |event| matches!(&event.payload, api::SyncPayload::ActivityItem(changed) if changed.item.id == id)
}

#[tokio::test]
async fn the_inbox_lists_changes_and_publishes_items() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let mut sync = Sync::connect(addr, &david.cookie_header(), &[]).await;
    sync.welcome().await;
    let before: api::ActivityUnreadCount =
        parse(&david.send(get("/api/v1/activity/unread_count")).await);

    let mention = item(&a, DAVID, ("Message", JASONS_REPLY), "mention", 60).await;
    let reply = item(&a, DAVID, ("Message", ROOT_MESSAGE), "reply", 30).await;
    let kevins = item(&a, KEVIN, ("Message", BOOSTED), "mention", 30).await;

    let response = david.send(get("/api/v1/activity")).await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    assert_eq!(response.header("cache-control"), Some("no-store"));
    let list: api::ActivityList = parse(&response);
    assert_eq!(list.unread_count, before.unread_count + 2);
    assert_eq!(list.next_cursor, None);
    // Newest first, unread only by default.
    let ids = list.items.iter().map(|item| item.id).collect::<Vec<_>>();
    let (at_reply, at_mention) = (
        ids.iter().position(|id| *id == reply).unwrap(),
        ids.iter().position(|id| *id == mention).unwrap(),
    );
    assert!(at_reply < at_mention, "{ids:?}");
    assert!(!ids.contains(&kevins));
    let listed = &list.items[at_mention];
    assert_eq!(
        (listed.event_type, listed.state, listed.read_at.as_deref()),
        (
            api::ActivityEventType::Mention,
            api::ActivityState::Unread,
            None
        )
    );
    let source = &listed.source;
    assert_eq!(
        (
            source.source_type,
            source.source_id,
            source.room_id,
            source.thread_id,
            source.message_id,
            source.creator_id
        ),
        (
            api::ActivitySourceType::Message,
            JASONS_REPLY,
            Some(DESIGNERS),
            Some(THREAD),
            Some(JASONS_REPLY),
            Some(JASON)
        )
    );
    assert!(source.title.starts_with("Designers"), "{source:?}");
    assert!(
        !source.body.is_empty() && !source.path.is_empty(),
        "{source:?}"
    );
    assert!(list.users.iter().any(|user| user.id == JASON));

    // The tabs filter by type.
    let mentions: api::ActivityList =
        parse(&david.send(get("/api/v1/activity?type=mentions")).await);
    assert!(mentions.items.iter().any(|item| item.id == mention));
    let events: api::ActivityList = parse(&david.send(get("/api/v1/activity?type=events")).await);
    assert!(
        events
            .items
            .iter()
            .all(|item| item.id != mention && item.id != reply)
    );

    // Reading answers the item and the count, and tells the other tabs.
    let path = format!("/api/v1/activity/{mention}");
    let response = david
        .write(json_body(Method::PATCH, &path, &json!({"action": "read"})))
        .await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    let changed: api::ActivityItemChanged = parse(&response);
    assert_eq!(changed.item.state, api::ActivityState::Read);
    assert!(changed.item.read_at.is_some());
    assert_eq!(changed.unread_count, before.unread_count + 1);
    let event = sync.until(activity_item(mention), |_| false).await;
    assert_eq!(event.topic, "user");
    assert_eq!(
        event.payload,
        api::SyncPayload::ActivityItem(changed.clone())
    );
    let read: api::ActivityList = parse(&david.send(get("/api/v1/activity?status=read")).await);
    assert!(read.items.iter().any(|item| item.id == mention));

    for (action, state) in [
        ("handled", api::ActivityState::Handled),
        ("unhandled", api::ActivityState::Read),
        ("unread", api::ActivityState::Unread),
    ] {
        let changed: api::ActivityItemChanged = parse(
            &david
                .write(json_body(Method::PATCH, &path, &json!({"action": action})))
                .await,
        );
        assert_eq!(changed.item.state, state, "{action}");
        let event = sync.until(activity_item(mention), |_| false).await;
        let api::SyncPayload::ActivityItem(published) = event.payload else {
            unreachable!()
        };
        assert_eq!(published.item.state, state, "{action}");
    }

    // Opening marks it read, never handled.
    let response = david
        .write(json_body(
            Method::POST,
            &format!("/api/v1/activity/{reply}/open"),
            &json!({}),
        ))
        .await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    let opened: api::ActivityItemChanged = parse(&response);
    assert_eq!(opened.item.state, api::ActivityState::Read);
    let count: api::ActivityUnreadCount =
        parse(&david.send(get("/api/v1/activity/unread_count")).await);
    assert_eq!(count.unread_count, before.unread_count + 1);

    // Someone else's item, or none, is a 404; an unknown action or a bad cursor a 422.
    for path in [
        format!("/api/v1/activity/{kevins}"),
        "/api/v1/activity/999999999".into(),
    ] {
        let response = david
            .write(json_body(Method::PATCH, &path, &json!({"action": "read"})))
            .await;
        assert_eq!(response.status, StatusCode::NOT_FOUND, "{path}");
        assert_eq!(tag(&response), "NotFound");
    }
    let response = kevin
        .write(json_body(
            Method::POST,
            &format!("/api/v1/activity/{mention}/open"),
            &json!({}),
        ))
        .await;
    assert_eq!(response.status, StatusCode::NOT_FOUND);
    let response = david
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"action": "archive"}),
        ))
        .await;
    assert_eq!(
        response.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        response.text()
    );
    let response = david
        .send(get("/api/v1/activity?before=not-a-cursor"))
        .await;
    assert_eq!(validation_fields(&response), ["before"]);
    server.abort();
}

#[tokio::test]
async fn the_inbox_pages_by_cursor() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let now = a.booted.app.db.env().now();
    // 130 dropped scheduled messages, each with its inbox item, some sharing a time.
    a.db()
        .write(move |tx| {
            for n in 0..130_i64 {
                let at = campfire_db::Timestamp::from_second(now.as_second() - 1_000 + n / 3);
                let id: i64 = tx.conn().query_row(
                    "INSERT INTO scheduled_messages (user_id, room_id, markdown_source, send_at, dropped_at, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING id",
                    rusqlite::params![DAVID, DESIGNERS, format!("Dropped {n}"), at, at, at, at],
                    |row| row.get(0),
                )?;
                tx.conn().execute(
                    "INSERT INTO activity_items (user_id, source_type, source_id, event_type, created_at, updated_at) VALUES (?, 'ScheduledMessage', ?, 'scheduled_message_dropped', ?, ?)",
                    rusqlite::params![DAVID, id, at, at],
                )?;
            }
            Ok(())
        })
        .await
        .unwrap();
    let mut seen = Vec::new();
    let mut cursor: Option<String> = None;
    let mut pages = 0;
    loop {
        let path = match &cursor {
            Some(before) => format!("/api/v1/activity?before={before}"),
            None => "/api/v1/activity".into(),
        };
        let reply = david.send(get(&path)).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        let page: api::ActivityList = parse(&reply);
        assert!(page.items.len() <= 100);
        assert!(page.next_cursor.is_none() || page.items.len() == 100);
        seen.extend(page.items);
        pages += 1;
        match page.next_cursor {
            Some(next) => cursor = Some(next),
            None => break,
        }
    }
    let keys = seen
        .iter()
        .map(|item| (item.updated_at.clone(), item.id))
        .collect::<Vec<_>>();
    let mut sorted = keys.clone();
    sorted.sort();
    sorted.reverse();
    sorted.dedup();
    assert_eq!(keys, sorted, "in order, each once");
    assert!(seen.len() >= 130 && pages == 2, "{} in {pages}", seen.len());
    let dropped = seen
        .iter()
        .find(|item| item.event_type == api::ActivityEventType::ScheduledMessageDropped)
        .unwrap();
    assert_eq!(
        (
            dropped.source.source_type,
            dropped.source.room_id,
            dropped.source.creator_id
        ),
        (
            api::ActivitySourceType::ScheduledMessage,
            Some(DESIGNERS),
            Some(DAVID)
        )
    );
}

#[tokio::test]
async fn saved_items_list_page_change_and_drop_their_reminders() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let mut sync = Sync::connect(addr, &david.cookie_header(), &[]).await;
    sync.welcome().await;

    let mut saved = Vec::new();
    for message_id in [JASONS_REPLY, ROOT_MESSAGE, BOOSTED] {
        let reply = david
            .write(json_body(
                Method::POST,
                "/api/v1/saved",
                &json!({"messageId": message_id, "remindAt": null}),
            ))
            .await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
        saved.push(parse::<api::SavedItem>(&reply));
    }
    let response = david.send(get("/api/v1/saved")).await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    let list: api::SavedItemList = parse(&response);
    assert_eq!(list.next_cursor, None);
    // Newest saved first, with each message, its creator and its conversation.
    let ids = list.items.iter().map(|item| item.id).collect::<Vec<_>>();
    assert_eq!(ids[..3], [saved[2].id, saved[1].id, saved[0].id]);
    assert_eq!(list.messages.len(), list.items.len());
    let reply_message = list.messages.iter().find(|m| m.id == JASONS_REPLY).unwrap();
    assert!(
        list.users
            .iter()
            .any(|user| user.id == reply_message.creator_id)
    );
    assert!(list.conversations.iter().any(|name| {
        (name.room_id, name.thread_id) == (DESIGNERS, Some(THREAD))
            && name.thread_name.as_deref() == Some("Launch review")
    }));
    let kevins: api::SavedItemList = parse(&kevin.send(get("/api/v1/saved")).await);
    assert!(kevins.items.iter().all(|item| !ids.contains(&item.id)));

    // Marking one done answers it, moves it between the filters and tells the other tabs.
    let one = format!("/api/v1/saved/{}", saved[1].id);
    let response = david
        .write(json_body(Method::PATCH, &one, &json!({"status": "done"})))
        .await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    let done: api::SavedItem = parse(&response);
    assert_eq!(
        (done.id, done.status),
        (saved[1].id, api::SavedStatus::Done)
    );
    let event = sync
        .until(
            |event| matches!(&event.payload, api::SyncPayload::SavedChanged(changed) if changed.item.as_ref().is_some_and(|item| item.status == api::SavedStatus::Done)),
            |_| false,
        )
        .await;
    assert_eq!(event.topic, "user");
    let done_list: api::SavedItemList = parse(&david.send(get("/api/v1/saved?status=done")).await);
    // The seed's own done item is listed too.
    assert!(done_list.items.iter().any(|item| item.id == saved[1].id));
    assert!(
        done_list
            .items
            .iter()
            .all(|item| item.status == api::SavedStatus::Done)
    );
    let open: api::SavedItemList =
        parse(&david.send(get("/api/v1/saved?status=in_progress")).await);
    assert!(open.items.iter().all(|item| item.id != saved[1].id));
    let all: api::SavedItemList = parse(&david.send(get("/api/v1/saved?status=whatever")).await);
    assert_eq!(all.items.len(), list.items.len());
    let response = kevin
        .write(json_body(Method::PATCH, &one, &json!({"status": "done"})))
        .await;
    assert_eq!(response.status, StatusCode::NOT_FOUND);
    let response = david
        .write(json_body(
            Method::PATCH,
            &one,
            &json!({"status": "archived"}),
        ))
        .await;
    assert_eq!(
        response.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        response.text()
    );
    let response = david.send(get("/api/v1/saved?before=garbage")).await;
    assert_eq!(validation_fields(&response), ["before"]);

    // Unsaving one whose reminder fired removes its inbox item: `activity.removed`.
    let reminder = item(&a, DAVID, ("SavedItem", saved[0].id), "message_reminder", 5).await;
    let reply = david
        .write(
            Req::new(Method::DELETE, &format!("/api/v1/saved/{}", saved[0].id))
                .header("accept", "application/json"),
        )
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.text());
    let event = sync
        .until(
            |event| matches!(event.payload, api::SyncPayload::ActivityRemoved(_)),
            |_| false,
        )
        .await;
    let api::SyncPayload::ActivityRemoved(removed) = event.payload else {
        unreachable!()
    };
    let count: api::ActivityUnreadCount =
        parse(&david.send(get("/api/v1/activity/unread_count")).await);
    assert_eq!(
        removed,
        api::ActivityItemRemoved {
            id: reminder,
            unread_count: count.unread_count
        }
    );
    server.abort();
}

#[tokio::test]
async fn saved_items_page_by_cursor() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let now = a.booted.app.db.env().now();
    // 120 saves, some sharing a time, of new messages in Designers.
    a.db()
        .write(move |tx| {
            for n in 0..120_i64 {
                let at = campfire_db::Timestamp::from_second(now.as_second() - 1_000 + n / 3);
                let message_id: i64 = tx.conn().query_row(
                    "INSERT INTO messages (room_id, creator_id, client_message_id, markdown_source, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?) RETURNING id",
                    rusqlite::params![DESIGNERS, JASON, format!("paged-{n}"), format!("Paged {n}"), at, at],
                    |row| row.get(0),
                )?;
                tx.conn().execute(
                    "INSERT INTO saved_items (user_id, message_id, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
                    rusqlite::params![DAVID, message_id, if n % 2 == 0 { "in_progress" } else { "done" }, at, at],
                )?;
            }
            Ok(())
        })
        .await
        .unwrap();
    for status in ["all", "done"] {
        let mut seen = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let path = match &cursor {
                Some(before) => format!("/api/v1/saved?status={status}&before={before}"),
                None => format!("/api/v1/saved?status={status}"),
            };
            let reply = david.send(get(&path)).await;
            assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
            let page: api::SavedItemList = parse(&reply);
            assert!(page.items.len() <= 50);
            assert!(page.next_cursor.is_none() || page.items.len() == 50);
            assert_eq!(page.messages.len(), page.items.len());
            seen.extend(page.items);
            match page.next_cursor {
                Some(next) => cursor = Some(next),
                None => break,
            }
        }
        let ids = seen.iter().map(|item| item.id).collect::<Vec<_>>();
        let mut unique = ids.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), ids.len(), "{status}: each once");
        if status == "done" {
            // The 60 and the seed's own done item.
            assert_eq!(ids.len(), 61);
            assert!(
                seen.iter()
                    .all(|item| item.status == api::SavedStatus::Done)
            );
        } else {
            assert!(ids.len() >= 120);
        }
    }
}

#[tokio::test]
async fn scheduled_messages_tell_the_other_tabs() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut sync = Sync::connect(addr, &david.cookie_header(), &[]).await;
    sync.welcome().await;
    let scheduled_changed = |id: i64, state: api::ScheduledMessageState| {
        move |event: &api::SyncEvent| matches!(&event.payload, api::SyncPayload::ScheduledChanged(row) if row.id == id && row.state == state)
    };
    let path = format!("/api/v1/rooms/{DESIGNERS}/scheduled_messages");
    let body = json!({"markdownSource": "Later", "sendAt": LATER});

    let row: api::ScheduledMessage =
        parse(&david.write(json_body(Method::POST, &path, &body)).await);
    let event = sync
        .until(
            scheduled_changed(row.id, api::ScheduledMessageState::Pending),
            |_| false,
        )
        .await;
    assert_eq!(event.topic, "user");
    assert_eq!(
        event.payload,
        api::SyncPayload::ScheduledChanged(row.clone())
    );

    let one = format!("/api/v1/scheduled_messages/{}", row.id);
    let changed: api::ScheduledMessage = parse(
        &david
            .write(json_body(
                Method::PATCH,
                &one,
                &json!({"markdownSource": "Later, all"}),
            ))
            .await,
    );
    let event = sync
        .until(
            |event| matches!(&event.payload, api::SyncPayload::ScheduledChanged(row) if row.markdown_source == "Later, all"),
            |_| false,
        )
        .await;
    assert_eq!(event.payload, api::SyncPayload::ScheduledChanged(changed));

    let reply = david
        .write(json_body(Method::DELETE, &one, &json!({})))
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    let event = sync
        .until(
            |event| matches!(event.payload, api::SyncPayload::ScheduledRemoved(_)),
            |_| false,
        )
        .await;
    assert_eq!(
        event.payload,
        api::SyncPayload::ScheduledRemoved(api::ScheduledMessageRemoved {
            id: row.id,
            room_id: DESIGNERS
        })
    );

    // Sending now publishes the sent row.
    let row: api::ScheduledMessage =
        parse(&david.write(json_body(Method::POST, &path, &body)).await);
    let reply = david
        .write(json_body(
            Method::POST,
            &format!("/api/v1/scheduled_messages/{}/send_now", row.id),
            &json!({}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let sent: api::ScheduledMessage = parse(&reply);
    let event = sync
        .until(
            scheduled_changed(row.id, api::ScheduledMessageState::Sent),
            |_| false,
        )
        .await;
    assert_eq!(event.payload, api::SyncPayload::ScheduledChanged(sent));

    // One dropped for lost access: the bare reason as the 422's message, the dropped row and
    // its inbox item.
    let row: api::ScheduledMessage =
        parse(&david.write(json_body(Method::POST, &path, &body)).await);
    a.db()
        .write(move |tx| {
            tx.conn().execute(
                "DELETE FROM memberships WHERE room_id = ? AND user_id = ?",
                [DESIGNERS, DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let reply = david
        .write(json_body(
            Method::POST,
            &format!("/api/v1/scheduled_messages/{}/send_now", row.id),
            &json!({}),
        ))
        .await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.text()
    );
    let api::ApiError::Validation { message, fields } =
        parse::<api::ApiErrorResponse>(&reply).error
    else {
        panic!("{}", reply.text())
    };
    assert_eq!((message.as_str(), fields.len()), ("channel access lost", 0));
    sync.until(
        scheduled_changed(row.id, api::ScheduledMessageState::Dropped),
        |_| false,
    )
    .await;
    let event = sync
        .until(
            |event| matches!(&event.payload, api::SyncPayload::ActivityItem(changed) if changed.item.event_type == api::ActivityEventType::ScheduledMessageDropped),
            |_| false,
        )
        .await;
    let api::SyncPayload::ActivityItem(dropped) = event.payload else {
        unreachable!()
    };
    assert_eq!(dropped.item.source.source_id, row.id);
    server.abort();
}
