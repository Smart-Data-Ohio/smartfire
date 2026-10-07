//! The S2 message actions on `/api/v1` (`campfire_api::message_actions`): edit, delete, reactions
//! and boosts, pins, saved items and forwards. Each answers in the contract's shape and publishes
//! its JSON twin, and the classic frames of every broadcast these touch are byte-for-byte the
//! same with the sync engine on.

use std::time::Duration;

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use serde_json::{Value, json};

use super::api_tests::{BOOSTED, Sync, app, created_in, get, json_body, parse, serve, tag};
use crate::controllers::presenters::test_support::{
    ALL_TALK, DAVID, HQ, JASON, KEVIN, Req, TestApp,
};

/// David's newest root message in All Talk.
const DAVIDS: i64 = 935962034;
/// The board where David replied in thread 4.
const BOARD: i64 = 699448332;
const BOARD_THREAD: i64 = 4;
const BOARD_REPLY: i64 = 935962049;
/// A message in All Pets, which Kevin isn't in.
const ALL_PETS_MESSAGE: i64 = 935961886;

fn removed(id: i64) -> impl Fn(&api::SyncEvent) -> bool {
    move |event| matches!(&event.payload, api::SyncPayload::MessageRemoved(gone) if gone.id == id)
}

fn updated(id: i64) -> impl Fn(&api::SyncEvent) -> bool {
    move |event| matches!(&event.payload, api::SyncPayload::MessageUpdated(message) if message.id == id)
}

#[tokio::test]
async fn edits_and_deletes_answer_and_publish() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut jason = a.sign_in(JASON).await;
    let topics = [format!("room:{ALL_TALK}"), format!("thread:{BOARD_THREAD}")];
    let mut sync = Sync::connect(addr, &david.cookie_header(), &topics).await;
    sync.welcome().await;

    let path = format!("/api/v1/messages/{DAVIDS}");
    let reply = david
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"markdownSource": "Edited **here**"}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let edited: api::MessageDTO = parse(&reply);
    assert_eq!(edited.id, DAVIDS);
    assert!(
        edited.body_html.contains("<strong>here</strong>"),
        "{}",
        edited.body_html
    );
    let event = sync.until(updated(DAVIDS), |_| false).await;
    assert_eq!(event.topic, format!("room:{ALL_TALK}"));
    let source: api::MessageSource = parse(&david.send(get(&format!("{path}/source"))).await);
    assert_eq!(source.markdown_source, "Edited **here**");

    // Only the author edits; a long source is a 422.
    let reply = jason
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"markdownSource": "Mine now"}),
        ))
        .await;
    assert_eq!(
        (reply.status, tag(&reply)),
        (StatusCode::FORBIDDEN, "Forbidden".into())
    );
    let reply = jason.send(get(&format!("{path}/source"))).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    let long = "a".repeat(50_001);
    let reply = david
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"markdownSource": long}),
        ))
        .await;
    assert_eq!(
        (reply.status, tag(&reply)),
        (StatusCode::UNPROCESSABLE_ENTITY, "Validation".into())
    );

    // A reply in a thread: the thread hears it, until the thread is locked.
    let reply_path = format!("/api/v1/messages/{BOARD_REPLY}");
    let reply = david
        .write(json_body(
            Method::PATCH,
            &reply_path,
            &json!({"markdownSource": "Reply edited"}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let event = sync.until(updated(BOARD_REPLY), |_| false).await;
    assert_eq!(event.topic, format!("thread:{BOARD_THREAD}"));
    let api::SyncPayload::MessageUpdated(message) = event.payload else {
        unreachable!()
    };
    assert!(message.body_html.contains("Reply edited"));
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE channel_threads SET locked_at = '2026-03-02 15:30:00' WHERE id = ?",
                [BOARD_THREAD],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let reply = david
        .write(json_body(
            Method::PATCH,
            &reply_path,
            &json!({"markdownSource": "Again"}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.text());
    let envelope: api::ApiErrorResponse = parse(&reply);
    assert_eq!(
        envelope.error,
        api::ApiError::Forbidden {
            message: "This thread is locked".into()
        }
    );

    // Deleting: the author or an administrator (Jason is one), and the room hears it.
    let reply = jason
        .write(Req::new(Method::DELETE, &path).header("accept", "application/json"))
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.text());
    sync.until(removed(DAVIDS), |_| false).await;
    let reply = david.send(get(&format!("{path}/source"))).await;
    assert_eq!(
        (reply.status, tag(&reply)),
        (StatusCode::NOT_FOUND, "NotFound".into())
    );
    server.abort();
}

async fn reactions_event(sync: &mut Sync) -> api::MessageReactions {
    let event = sync
        .until(
            |event| matches!(event.payload, api::SyncPayload::MessageReactions(_)),
            |_| false,
        )
        .await;
    assert_eq!(event.topic, format!("room:{ALL_TALK}"));
    let api::SyncPayload::MessageReactions(reactions) = event.payload else {
        unreachable!()
    };
    reactions
}

#[tokio::test]
async fn reactions_and_boosts_answer_and_publish() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut sync = Sync::connect(addr, &david.cookie_header(), &[format!("room:{ALL_TALK}")]).await;
    sync.welcome().await;
    let path = format!("/api/v1/messages/{BOOSTED}/boosts");
    let reacted = |reply: &crate::controllers::presenters::test_support::Reply| {
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        parse::<api::MessageReactions>(reply)
    };

    // A reaction toggles: on, then off.
    let on = reacted(
        &david
            .write(json_body(Method::POST, &path, &json!({"content": "👍"})))
            .await,
    );
    assert_eq!(on.message_id, BOOSTED);
    let thumbs = on
        .reactions
        .iter()
        .find(|reaction| reaction.content == "👍")
        .expect("👍");
    assert!(thumbs.reactor_ids.contains(&DAVID), "{on:?}");
    assert_eq!(thumbs.title, "Thumbs up");
    assert_eq!(reactions_event(&mut sync).await.reactions, on.reactions);
    let off = reacted(
        &david
            .write(json_body(Method::POST, &path, &json!({"content": "👍"})))
            .await,
    );
    assert!(
        off.reactions
            .iter()
            .all(|reaction| reaction.content != "👍"),
        "{off:?}"
    );
    reactions_event(&mut sync).await;

    // Free text is a boost, removed by id.
    let boosted = reacted(
        &david
            .write(json_body(
                Method::POST,
                &path,
                &json!({"content": "Nice one"}),
            ))
            .await,
    );
    let boost = boosted
        .boosts
        .iter()
        .find(|boost| boost.content == "Nice one")
        .expect("the boost");
    assert_eq!(boost.booster_id, DAVID);
    reactions_event(&mut sync).await;
    let gone = reacted(
        &david
            .write(
                Req::new(Method::DELETE, &format!("{path}/{}", boost.id))
                    .header("accept", "application/json"),
            )
            .await,
    );
    assert!(gone.boosts.iter().all(|kept| kept.id != boost.id));
    assert!(gone.updated_at >= boosted.updated_at);
    reactions_event(&mut sync).await;
    // Someone else's boost isn't David's to remove.
    let jasons = on.boosts.first().map(|boost| boost.id);
    if let Some(id) = jasons.filter(|_| on.boosts[0].booster_id != DAVID) {
        let reply = david
            .write(
                Req::new(Method::DELETE, &format!("{path}/{id}"))
                    .header("accept", "application/json"),
            )
            .await;
        assert_eq!(
            (reply.status, tag(&reply)),
            (StatusCode::NOT_FOUND, "NotFound".into())
        );
    }

    // Blank or long free text is a 422; a long `:shortcode:` is an icon and goes through.
    for content in ["  ", "seventeen letters"] {
        let reply = david
            .write(json_body(Method::POST, &path, &json!({"content": content})))
            .await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{content}: {}",
            reply.text()
        );
        let envelope: api::ApiErrorResponse = parse(&reply);
        let api::ApiError::Validation { fields, .. } = envelope.error else {
            panic!("{}", reply.text())
        };
        assert!(fields.contains_key("content"), "{fields:?}");
    }
    let shortcode = reacted(
        &david
            .write(json_body(
                Method::POST,
                &path,
                &json!({"content": ":white_check_mark:"}),
            ))
            .await,
    );
    assert!(
        shortcode
            .reactions
            .iter()
            .any(|reaction| reaction.content == "✅"),
        "{shortcode:?}"
    );
    server.abort();
}

#[tokio::test]
async fn pins_answer_list_and_publish() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut sync = Sync::connect(addr, &david.cookie_header(), &[format!("room:{ALL_TALK}")]).await;
    sync.welcome().await;
    let path = format!("/api/v1/messages/{DAVIDS}/pin");
    let pinned_event = |pinned: bool| move |event: &api::SyncEvent| matches!(&event.payload, api::SyncPayload::MessagePinned(state) if state.message_id == DAVIDS && state.pinned == pinned);

    let reply = david
        .write(json_body(Method::POST, &path, &json!({})))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let state: api::PinState = parse(&reply);
    assert_eq!(
        (state.message_id, state.room_id, state.pinned),
        (DAVIDS, ALL_TALK, true)
    );
    // The pin note is a new message, as on the classic page. The two twins come from separate
    // broadcasts, so either may come first.
    let (mut pinned, mut note) = (None, None);
    while pinned.is_none() || note.is_none() {
        let event = sync
            .until(
                |event| pinned_event(true)(event) || created_in(ALL_TALK)(event),
                |_| false,
            )
            .await;
        match event.payload {
            api::SyncPayload::MessageCreated(created) => note = Some(created),
            other => pinned = Some(other),
        }
    }
    assert_eq!(pinned, Some(api::SyncPayload::MessagePinned(state)));
    let note = note.unwrap();
    assert!(note.system_note, "{note:?}");
    let note_id = note.id;
    // Pinning again changes nothing.
    let again: api::PinState = parse(
        &david
            .write(json_body(Method::POST, &path, &json!({})))
            .await,
    );
    assert_eq!(again, state);

    let list: api::PinList = parse(
        &david
            .send(get(&format!("/api/v1/rooms/{ALL_TALK}/pins")))
            .await,
    );
    assert_eq!(list.pins[0].message_id, DAVIDS);
    assert_eq!(list.pins.len() as i64, state.pin_count);
    assert!(
        list.pins.iter().all(|pin| list
            .messages
            .iter()
            .any(|message| message.id == pin.message_id)),
        "every pin has its message: {list:?}"
    );
    assert!(list.messages.iter().any(|message| message.id == DAVIDS));
    assert!(list.users.iter().any(|user| user.id == DAVID));

    let reply = david
        .write(Req::new(Method::DELETE, &path).header("accept", "application/json"))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let state: api::PinState = parse(&reply);
    assert!(!state.pinned);
    // The note went out once: `deliver` publishes it, and nothing else does.
    sync.until(pinned_event(false), created_again(note_id))
        .await;

    // A room with 50 pins takes no more.
    a.db()
        .write(|tx| {
            let ids: Vec<i64> = tx
                .conn()
                .prepare("SELECT id FROM messages WHERE room_id = ? AND id <> ? ORDER BY id LIMIT 50")?
                .query_map([ALL_TALK, DAVIDS], |row| row.get(0))?
                .collect::<rusqlite::Result<_>>()?;
            for id in ids {
                tx.conn().execute(
                    "INSERT INTO message_pins (message_id, pinner_id, room_id, created_at, updated_at) VALUES (?, ?, ?, '2026-03-02 15:00:00', '2026-03-02 15:00:00')",
                    [id, DAVID, ALL_TALK],
                )?;
            }
            Ok(())
        })
        .await
        .unwrap();
    let reply = david
        .write(json_body(Method::POST, &path, &json!({})))
        .await;
    assert_eq!(
        (reply.status, tag(&reply)),
        (StatusCode::UNPROCESSABLE_ENTITY, "Validation".into())
    );
    server.abort();
}

#[tokio::test]
async fn saving_answers_and_tells_the_other_tabs() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let mut sync = Sync::connect(addr, &david.cookie_header(), &[]).await;
    sync.welcome().await;
    let changed =
        |event: &api::SyncEvent| matches!(event.payload, api::SyncPayload::SavedChanged(_));

    let body = json!({"messageId": BOOSTED, "remindAt": "2026-03-03T09:00:00Z"});
    let reply = david
        .write(json_body(Method::POST, "/api/v1/saved", &body))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let item: api::SavedItem = parse(&reply);
    assert_eq!(
        (item.message_id, item.status),
        (BOOSTED, api::SavedStatus::InProgress)
    );
    assert_eq!(item.remind_at.as_deref(), Some("2026-03-03T09:00:00.000Z"));
    let event = sync.until(changed, |_| false).await;
    assert_eq!(event.topic, "user");
    assert_eq!(
        event.payload,
        api::SyncPayload::SavedChanged(api::SavedChanged {
            message_id: BOOSTED,
            item: Some(item.clone())
        })
    );
    // Saving again updates the reminder; a past one or a bad time is a 422.
    let again: api::SavedItem = parse(
        &david
            .write(json_body(
                Method::POST,
                "/api/v1/saved",
                &json!({"messageId": BOOSTED, "remindAt": null}),
            ))
            .await,
    );
    assert_eq!((again.id, again.remind_at), (item.id, None));
    sync.until(changed, |_| false).await;
    for remind_at in ["2026-03-01T09:00:00Z", "tomorrow"] {
        let body = json!({"messageId": BOOSTED, "remindAt": remind_at});
        let reply = david
            .write(json_body(Method::POST, "/api/v1/saved", &body))
            .await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{remind_at}: {}",
            reply.text()
        );
        let envelope: api::ApiErrorResponse = parse(&reply);
        let api::ApiError::Validation { fields, .. } = envelope.error else {
            panic!()
        };
        assert!(fields.contains_key("remindAt"), "{fields:?}");
    }
    // Someone else's saved item is a 404; a message outside the viewer's rooms too.
    let path = format!("/api/v1/saved/{}", item.id);
    let reply = kevin
        .write(Req::new(Method::DELETE, &path).header("accept", "application/json"))
        .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    let reply = kevin
        .write(json_body(
            Method::POST,
            "/api/v1/saved",
            &json!({"messageId": ALL_PETS_MESSAGE, "remindAt": null}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND, "{}", reply.text());

    let reply = david
        .write(Req::new(Method::DELETE, &path).header("accept", "application/json"))
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.text());
    let event = sync.until(changed, |_| false).await;
    assert_eq!(
        event.payload,
        api::SyncPayload::SavedChanged(api::SavedChanged {
            message_id: BOOSTED,
            item: None
        })
    );
    server.abort();
}

#[tokio::test]
async fn forwards_list_destinations_copy_and_publish() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut sync = Sync::connect(addr, &david.cookie_header(), &[format!("room:{HQ}")]).await;
    sync.welcome().await;

    let list: api::ForwardDestinationList =
        parse(&david.send(get("/api/v1/forward_destinations")).await);
    let names: Vec<&str> = list
        .destinations
        .iter()
        .map(|room| room.name.as_str())
        .collect();
    let mut sorted = names.clone();
    sorted.sort_by_key(|name| name.to_ascii_lowercase());
    assert!(
        list.destinations.iter().any(|room| room.room_id == HQ),
        "{names:?}"
    );
    assert!(
        list.destinations.iter().all(|room| room.room_id != BOARD),
        "boards are left out"
    );
    assert!(
        list.destinations
            .iter()
            .filter(|room| room.direct)
            .all(|room| room.threads.is_empty())
    );
    let designers = list
        .destinations
        .iter()
        .find(|room| room.room_id == 654632876)
        .expect("Designers");
    assert!(
        designers
            .threads
            .iter()
            .all(|thread| thread.status != api::ThreadStatus::Locked)
    );
    assert!(designers.threads.iter().any(|thread| thread.id == 1));

    let path = format!("/api/v1/messages/{BOOSTED}/forwards");
    let body = json!({"note": "FYI", "destinations": [{"roomId": HQ, "threadId": null}]});
    let reply = david.write(json_body(Method::POST, &path, &body)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let result: api::ForwardResult = parse(&reply);
    let [forward] = result.forwards.as_slice() else {
        panic!("{result:?}")
    };
    assert_eq!((forward.room_id, forward.creator_id), (HQ, DAVID));
    assert_eq!(forward.forwarded_from_message_id, Some(BOOSTED));
    let event = sync.until(created_in(HQ), |_| false).await;
    // Each read stamps its own `cardsAsOf`.
    let api::SyncPayload::MessageCreated(published) = event.payload else {
        unreachable!()
    };
    assert_eq!(
        api::MessageDTO {
            cards_as_of: forward.cards_as_of.clone(),
            ..published
        },
        forward.clone()
    );

    for destinations in [
        json!([]),
        json!([{"roomId": BOARD, "threadId": null}]),
        json!([{"roomId": 654632876, "threadId": 3}]),
    ] {
        let body = json!({"note": null, "destinations": destinations});
        let reply = david.write(json_body(Method::POST, &path, &body)).await;
        assert_eq!(
            (reply.status, tag(&reply)),
            (StatusCode::UNPROCESSABLE_ENTITY, "Validation".into()),
            "{destinations}: {}",
            reply.text()
        );
    }
    server.abort();
}

#[tokio::test]
async fn a_message_reads_with_its_conversation_or_not_at_all() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let reply = david.send(get(&format!("/api/v1/messages/{DAVIDS}"))).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let read: api::MessageRead = parse(&reply);
    assert_eq!(read.message.id, DAVIDS);
    assert_eq!(
        (read.conversation.room_id, read.conversation.thread_id),
        (ALL_TALK, None)
    );
    assert_eq!(read.conversation.room_kind, api::RoomKind::Closed);
    assert!(read.users.iter().any(|user| user.id == DAVID));
    assert_eq!(read.saved, None);

    let reply = david
        .send(get(&format!("/api/v1/messages/{BOARD_REPLY}")))
        .await;
    let read: api::MessageRead = parse(&reply);
    assert_eq!(
        (read.conversation.room_id, read.conversation.thread_id),
        (BOARD, Some(BOARD_THREAD))
    );
    assert!(read.conversation.thread_name.is_some());

    // A message the viewer can't reach is a 404, so a forward's origin shows as "Forwarded" only.
    let mut kevin = a.sign_in(KEVIN).await;
    let reply = kevin
        .send(get(&format!("/api/v1/messages/{ALL_PETS_MESSAGE}")))
        .await;
    assert_eq!(
        (reply.status, tag(&reply)),
        (StatusCode::NOT_FOUND, "NotFound".into()),
        "{}",
        reply.text()
    );
    let reply = kevin.send(get("/api/v1/messages/999999999")).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn people_say_whether_they_uploaded_a_picture() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let has_avatar = |page: &api::MessagePage, id: i64| {
        page.users
            .iter()
            .find(|user| user.id == id)
            .map(|user| user.has_avatar)
    };
    let path = format!("/api/v1/rooms/{ALL_TALK}/messages");
    let before: api::MessagePage = parse(&david.send(get(&path)).await);
    assert_eq!(has_avatar(&before, DAVID), Some(false));
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "INSERT INTO active_storage_blobs (key, filename, content_type, metadata, service_name, byte_size, checksum, created_at) \
                 VALUES ('avatar-david', 'david.png', 'image/png', '{}', 'local', 1, 'x', '2026-03-02 15:30:00')",
                [],
            )?;
            let blob = tx.conn().last_insert_rowid();
            tx.conn().execute(
                "INSERT INTO active_storage_attachments (name, record_type, record_id, blob_id, created_at) \
                 VALUES ('avatar', 'User', ?, ?, '2026-03-02 15:30:00')",
                [DAVID, blob],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let page: api::MessagePage = parse(&david.send(get(&path)).await);
    assert_eq!(has_avatar(&page, DAVID), Some(true));
    for user in before.users.iter().filter(|user| user.id != DAVID) {
        assert_eq!(
            has_avatar(&page, user.id),
            Some(user.has_avatar),
            "{}",
            user.id
        );
    }
    let me: api::Me = parse(&david.send(get("/api/v1/me")).await);
    assert!(me.user.has_avatar);
}

#[tokio::test]
async fn a_page_names_its_thread_repliers() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let page: api::MessagePage = parse(&david.send(get("/api/v1/rooms/654632876/messages")).await);
    let root = page
        .messages
        .iter()
        .find(|message| message.thread.is_some())
        .expect("the thread's root");
    for replier in &root.thread.as_ref().unwrap().replier_ids {
        assert!(
            page.users.iter().any(|user| user.id == *replier),
            "{replier}"
        );
    }
}

#[tokio::test]
async fn a_scheduled_send_reaches_the_sync_socket() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let david = a.sign_in(DAVID).await;
    let mut sync = Sync::connect(addr, &david.cookie_header(), &[format!("room:{ALL_TALK}")]).await;
    sync.welcome().await;
    a.db()
        .write(|tx| {
            use campfire_db::{NewScheduledMessage, ScheduledMessage};
            let now = campfire_db::Timestamp::from_jiff("2026-03-02T17:00:00Z".parse().unwrap());
            let row = ScheduledMessage::create(
                tx,
                NewScheduledMessage {
                    user_id: JASON,
                    room_id: ALL_TALK,
                    thread_id: None,
                    reply_to_message_id: None,
                    markdown_source: "Sent later".into(),
                    send_at: now,
                },
            )?;
            assert!(ScheduledMessage::dispatch(tx, row.id, now, true)?);
            Ok(())
        })
        .await
        .unwrap();
    let event = sync.until(created_in(ALL_TALK), |_| false).await;
    let api::SyncPayload::MessageCreated(message) = event.payload else {
        unreachable!()
    };
    assert_eq!(message.creator_id, JASON);
    assert!(
        message.body_html.contains("Sent later"),
        "{}",
        message.body_html
    );
    // It went out once: a later message arrives with no second copy before it.
    let mut david = david;
    let body = json!({"clientMessageId": "after-scheduled", "markdownSource": "Afterwards"});
    let reply = david
        .write(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{ALL_TALK}/messages"),
            &body,
        ))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let later: api::MessageDTO = parse(&reply);
    sync.until(
        move |event| matches!(&event.payload, api::SyncPayload::MessageCreated(m) if m.id == later.id),
        created_again(message.id),
    )
    .await;
    server.abort();
}

/// A second `message.created` for a message the socket already had.
fn created_again(id: i64) -> impl Fn(&api::SyncEvent) -> bool {
    move |event| matches!(&event.payload, api::SyncPayload::MessageCreated(m) if m.id == id)
}

/// The classic frames for the actions the S2 endpoints share broadcasts with: reactions and
/// boosts, pins and their notes, an edit in a thread, and a scheduled message going out.
async fn classic_action_frames(spa: bool) -> Option<Vec<(String, String)>> {
    use crate::controllers::presenters::test_support::SEED_NOW;
    use campfire_kit::clock::FrozenClock;
    let clock = std::sync::Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
    let env: &[(&str, &str)] = if spa { &[("SPA_ENABLED", "1")] } else { &[] };
    let a = TestApp::boot_seed_with_env("default", clock, env).await?;
    // A classic page open, as in `classic_frames`.
    let (mut client, cable) =
        crate::controllers::messages::attachment_processing_tests::subscribe(&a).await;
    let identifier =
        crate::channels::tests::support::identifier(json!({ "channel": "UnreadRoomsChannel" }));
    client.confirm(&identifier).await;
    // And the board thread's page, for the edit there.
    let thread_gid = campfire_app::cable::thread_gid(BOARD_THREAD).to_param();
    let signed =
        rails_compat::turbo::signed_stream_name(&a.booted.app.secrets, &[&thread_gid, "messages"]);
    let identifier = crate::channels::tests::support::identifier(
        json!({ "channel": "RoomMessagesChannel", "signed_stream_name": signed }),
    );
    client.confirm(&identifier).await;
    let mut david = a.sign_in(DAVID).await;
    david.authenticity_token().await;
    let (_sync, server) = if spa {
        let (addr, server) = serve(&a).await;
        let topics = [format!("room:{ALL_TALK}"), format!("thread:{BOARD_THREAD}")];
        let mut sync = Sync::connect(addr, &david.cookie_header(), &topics).await;
        sync.welcome().await;
        (Some(sync), Some(server))
    } else {
        (None, None)
    };
    assert_eq!(a.booted.app.cable.sync_wanted(), spa);
    let capture = a.publications();
    capture.take();
    let classic = |method: Method, path: String, body: Value| {
        Req::new(method, &path)
            .header("accept", "application/json")
            .header("content-type", "application/json")
            .body(body.to_string())
    };

    let boosts = format!("/messages/{BOOSTED}/boosts");
    for content in ["👍", "Nice one"] {
        let reply = david
            .write(classic(
                Method::POST,
                boosts.clone(),
                json!({"boost": {"content": content}}),
            ))
            .await;
        assert_eq!(reply.status, StatusCode::FOUND, "{}", reply.text());
    }
    let boost = a
        .db()
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT id FROM boosts WHERE message_id = ? AND content = 'Nice one'",
                [BOOSTED],
                |row| row.get::<_, i64>(0),
            )?)
        })
        .await
        .unwrap();
    let reply = david
        .write(classic(
            Method::DELETE,
            format!("{boosts}/{boost}"),
            json!({}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.text());
    for method in [Method::POST, Method::DELETE] {
        let reply = david
            .write(classic(
                method,
                format!("/messages/{DAVIDS}/pin"),
                json!({}),
            ))
            .await;
        assert!(reply.status.is_success(), "{}", reply.text());
    }
    let reply = david
        .write(classic(
            Method::PATCH,
            format!("/rooms/{BOARD}/threads/{BOARD_THREAD}/messages/{BOARD_REPLY}.json"),
            json!({"message": {"markdown_source": "Thread parity"}}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    a.db()
        .write(|tx| {
            use campfire_db::{NewScheduledMessage, ScheduledMessage};
            let now = campfire_db::Timestamp::from_jiff("2026-03-02T17:00:00Z".parse().unwrap());
            let row = ScheduledMessage::create(
                tx,
                NewScheduledMessage {
                    user_id: DAVID,
                    room_id: ALL_TALK,
                    thread_id: None,
                    reply_to_message_id: None,
                    markdown_source: "Scheduled parity".into(),
                    send_at: now,
                },
            )?;
            assert!(ScheduledMessage::dispatch(tx, row.id, now, true)?);
            Ok(())
        })
        .await
        .unwrap();

    let mut frames = Vec::new();
    let mut quiet = 0;
    while quiet < 10 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let more = capture.take();
        quiet = if more.is_empty() { quiet + 1 } else { 0 };
        frames.extend(more);
    }
    cable.abort();
    if let Some(server) = server {
        server.abort();
    }
    Some(frames)
}

#[tokio::test]
async fn the_action_frames_are_the_same_with_the_sync_engine_on() {
    let (Some(off), Some(on)) = (
        classic_action_frames(false).await,
        classic_action_frames(true).await,
    ) else {
        return;
    };
    let has = |needle: &str| off.iter().any(|(_, frame)| frame.contains(needle));
    assert!(has("target=\\\"boosts_message_"), "reactions: {off:#?}");
    assert!(has("Thread parity"), "a thread edit: {off:#?}");
    assert!(has("Scheduled parity"), "a scheduled send: {off:#?}");
    assert!(has("pinned a message"), "a pin note: {off:#?}");
    // The pin note and the scheduled message get fresh client ids each run.
    let uuid =
        regex::Regex::new(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}").unwrap();
    let same = |(stream, frame): &(String, String)| {
        (stream.clone(), uuid.replace_all(frame, "UUID").into_owned())
    };
    assert_eq!(off.len(), on.len(), "off: {off:#?}\non: {on:#?}");
    for (index, (off, on)) in off.iter().zip(&on).enumerate() {
        assert_eq!(same(off), same(on), "frame {index}");
    }
}
