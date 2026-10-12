//! The S2 thread endpoints on `/api/v1` (`campfire_api::threads`). Each answers in the
//! contract's shape and publishes its JSON twin, and the classic frames of every thread broadcast
//! these touch are byte-for-byte the same with the sync engine on.


use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use serde_json::{Value, json};

use super::api_tests::{Sync, app, get, json_body, parse, serve, tag};
use crate::controllers::presenters::test_support::{DAVID, JASON, KEVIN, Req};

/// A closed room with threads 1, 2 (closed), 3 (locked) and 8. David created it; Kevin is a
/// plain member.
const DESIGNERS: i64 = 654632876;
/// Thread 1 hangs off this root message; Jason replied in it.
const THREAD: i64 = 1;
const THREAD_PARENT: i64 = 935962043;
const JASONS_REPLY: i64 = 935962046;
const LOCKED_THREAD: i64 = 3;
/// David's newest root message in Designers, with no thread.
const UNTHREADED: i64 = 935962057;
/// A board thread; Kevin isn't on the board.
const BOARD_THREAD: i64 = 4;
/// Its board, which David created.
const BOARD: i64 = 699448332;
/// JZ: a plain member, like Kevin.
const JZ: i64 = 773523953;
/// A root message in All Talk (another room).
const ALL_TALK_MESSAGE: i64 = 935962034;

fn reply_body(client_message_id: &str, markdown: &str) -> Value {
    json!({"clientMessageId": client_message_id, "markdownSource": markdown})
}

type Wanted<'a> = &'a dyn Fn(&api::SyncEvent) -> bool;

/// The first event each of `wanted` matches, in `wanted`'s order, whatever order they arrive in.
async fn gather(sync: &mut Sync, wanted: &[Wanted<'_>]) -> Vec<api::SyncEvent> {
    let mut found: Vec<Option<api::SyncEvent>> = wanted.iter().map(|_| None).collect();
    while found.iter().any(Option::is_none) {
        let event = sync
            .until(
                |event| {
                    wanted
                        .iter()
                        .zip(&found)
                        .any(|(wanted, found)| found.is_none() && wanted(event))
                },
                |_| false,
            )
            .await;
        let index = wanted
            .iter()
            .zip(&found)
            .position(|(wanted, found)| found.is_none() && wanted(&event))
            .unwrap();
        found[index] = Some(event);
    }
    found.into_iter().map(Option::unwrap).collect()
}

fn on(topic: String, kind: fn(&api::SyncPayload) -> bool) -> impl Fn(&api::SyncEvent) -> bool {
    move |event| event.topic == topic && kind(&event.payload)
}

fn envelope(reply: &crate::controllers::presenters::test_support::Reply) -> api::ApiError {
    parse::<api::ApiErrorResponse>(reply).error
}

#[tokio::test]
async fn threads_list_show_and_page() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;

    let ids = |list: &api::ThreadList| {
        list.threads
            .iter()
            .map(|row| row.thread.id)
            .collect::<Vec<_>>()
    };
    let all: api::ThreadList = parse(
        &david
            .send(get(&format!("/api/v1/rooms/{DESIGNERS}/threads?state=all")))
            .await,
    );
    let mut sorted = ids(&all);
    sorted.sort();
    assert_eq!(sorted, [1, 2, 3, 8]);
    // Most recently active first, as `channel_threads#index` orders them.
    for pair in all.threads.windows(2) {
        assert!(pair[0].thread.last_activity_at >= pair[1].thread.last_activity_at);
    }
    let one = all
        .threads
        .iter()
        .find(|row| row.thread.id == THREAD)
        .unwrap();
    assert_eq!(one.thread.parent_message_id, Some(THREAD_PARENT));
    assert_eq!(one.thread.name, "Launch review");
    assert_eq!(
        one.membership
            .as_ref()
            .map(|membership| membership.involvement),
        Some(api::ThreadInvolvement::Mentions)
    );
    assert!(all.users.iter().any(|user| user.id == DAVID));
    let locked: api::ThreadList = parse(
        &david
            .send(get(&format!(
                "/api/v1/rooms/{DESIGNERS}/threads?state=locked"
            )))
            .await,
    );
    assert_eq!(ids(&locked), [LOCKED_THREAD]);
    assert_eq!(locked.threads[0].thread.status, api::ThreadStatus::Locked);
    let closed: api::ThreadList = parse(
        &david
            .send(get(&format!(
                "/api/v1/rooms/{DESIGNERS}/threads?state=closed"
            )))
            .await,
    );
    assert!(
        ids(&closed).contains(&2) && ids(&closed).contains(&LOCKED_THREAD),
        "{closed:?}"
    );
    let active: api::ThreadList = parse(
        &david
            .send(get(&format!("/api/v1/rooms/{DESIGNERS}/threads")))
            .await,
    );
    assert!(
        active
            .threads
            .iter()
            .all(|row| row.thread.status == api::ThreadStatus::Active)
    );
    assert!(!ids(&active).contains(&2) && !ids(&active).contains(&LOCKED_THREAD));
    let reply = david
        .send(get(&format!(
            "/api/v1/rooms/{DESIGNERS}/threads?state=soon"
        )))
        .await;
    assert_eq!(
        (reply.status, tag(&reply)),
        (StatusCode::UNPROCESSABLE_ENTITY, "Validation".into())
    );

    // The pane's header.
    let detail: api::ThreadDetail =
        parse(&david.send(get(&format!("/api/v1/threads/{THREAD}"))).await);
    assert_eq!(detail.thread.id, THREAD);
    assert_eq!(
        detail.parent_message.as_ref().map(|message| message.id),
        Some(THREAD_PARENT)
    );
    assert!(detail.users.iter().any(|user| user.id == DAVID));
    assert_eq!(
        detail.permissions,
        api::ThreadPermissions {
            can_rename: true,
            can_close: detail.thread.status == api::ThreadStatus::Active,
            can_reopen: detail.thread.status == api::ThreadStatus::Closed,
            can_lock: true,
            can_unlock: false,
            can_delete: true,
            // Untracked: David, its creator, may start tracking it.
            can_convert_work: true,
            can_manage_work: false,
            can_update_work_status: false,
            can_assign_work: false,
            can_remove_work: false,
        }
    );
    // Kevin isn't a moderator or the creator, nor a member of the thread.
    let detail: api::ThreadDetail =
        parse(&kevin.send(get(&format!("/api/v1/threads/{THREAD}"))).await);
    assert_eq!(detail.membership, None);
    assert_eq!(
        detail.permissions,
        api::ThreadPermissions {
            can_rename: false,
            can_close: false,
            can_reopen: false,
            can_lock: false,
            can_unlock: false,
            can_delete: false,
            can_convert_work: false,
            can_manage_work: false,
            can_update_work_status: false,
            can_assign_work: false,
            can_remove_work: false,
        }
    );
    // A room Kevin isn't in, and a thread that doesn't exist.
    for path in [
        format!("/api/v1/threads/{BOARD_THREAD}"),
        format!("/api/v1/threads/{BOARD_THREAD}/messages"),
        "/api/v1/threads/999999".into(),
        "/api/v1/threads/nope".into(),
        "/api/v1/rooms/699448332/threads".into(),
    ] {
        let reply = kevin.send(get(&path)).await;
        assert_eq!(
            (reply.status, tag(&reply)),
            (StatusCode::NOT_FOUND, "NotFound".into()),
            "{path}"
        );
    }

    // The replies page.
    let page: api::MessagePage = parse(
        &david
            .send(get(&format!("/api/v1/threads/{THREAD}/messages")))
            .await,
    );
    assert!(
        page.messages
            .iter()
            .any(|message| message.id == JASONS_REPLY),
        "{page:?}"
    );
    assert!(
        page.messages
            .iter()
            .all(|message| message.thread_id == Some(THREAD))
    );
    assert!(page.users.iter().any(|user| user.id == JASON));
    // A cursor off the thread's timeline (the parent is on the room's) is a 404.
    let reply = david
        .send(get(&format!(
            "/api/v1/threads/{THREAD}/messages?around={THREAD_PARENT}"
        )))
        .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn starting_a_thread_answers_and_publishes() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let jason = a.sign_in(JASON).await;
    let mut sync =
        Sync::connect(addr, &jason.cookie_header(), &[format!("room:{DESIGNERS}")]).await;
    sync.welcome().await;

    let path = format!("/api/v1/rooms/{DESIGNERS}/threads");
    let body = json!({
        "parentMessageId": UNTHREADED,
        "name": null,
        "message": reply_body("first-reply", "Kicking this off"),
    });
    let reply = david.write(json_body(Method::POST, &path, &body)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let created: api::ThreadCreated = parse(&reply);
    let thread = created.detail.thread.clone();
    assert_eq!(thread.parent_message_id, Some(UNTHREADED));
    assert_eq!(thread.creator_id, DAVID);
    assert_eq!(thread.reply_count, 1);
    assert_eq!(thread.status, api::ThreadStatus::Active);
    assert!(!thread.name.is_empty());
    assert_eq!(
        created
            .detail
            .membership
            .as_ref()
            .map(|membership| membership.thread_id),
        Some(thread.id)
    );
    assert_eq!(created.message.thread_id, Some(thread.id));
    assert_eq!(created.message.client_message_id, "first-reply");
    assert!(created.message.body_html.contains("Kicking this off"));

    let thread_id = thread.id;
    let events = gather(
        &mut sync,
        &[
            &|event| matches!(&event.payload, api::SyncPayload::ThreadCreated(created) if created.id == thread_id),
            &|event| matches!(&event.payload, api::SyncPayload::ThreadIndicator(changed) if changed.parent_message_id == UNTHREADED),
        ],
    )
    .await;
    assert_eq!(events[0].topic, format!("room:{DESIGNERS}"));
    let api::SyncPayload::ThreadIndicator(changed) = events[1].payload.clone() else {
        unreachable!()
    };
    assert_eq!(events[1].topic, format!("room:{DESIGNERS}"));
    let indicator = changed.thread.expect("the new thread's indicator");
    assert_eq!((indicator.thread_id, indicator.reply_count), (thread.id, 1));
    assert_eq!(indicator.replier_ids, [DAVID]);

    // A retry gets the same thread; another thread on the same message is a conflict.
    let reply = david.write(json_body(Method::POST, &path, &body)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert_eq!(
        parse::<api::ThreadCreated>(&reply).detail.thread.id,
        thread.id
    );
    let again = json!({"parentMessageId": UNTHREADED, "name": "Twice", "message": reply_body("second", "Again")});
    let reply = david.write(json_body(Method::POST, &path, &again)).await;
    assert_eq!(
        (reply.status, tag(&reply)),
        (StatusCode::CONFLICT, "Conflict".into()),
        "{}",
        reply.text()
    );
    // A first reply reusing the `clientMessageId` of a message outside any thread.
    let root = json!({"clientMessageId": "root-taken", "markdownSource": "A root message"});
    let reply = david
        .write(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{DESIGNERS}/messages"),
            &root,
        ))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let root: api::MessageDTO = parse(&reply);
    let body = json!({"parentMessageId": root.id, "name": null, "message": reply_body("root-taken", "Hi")});
    let reply = david.write(json_body(Method::POST, &path, &body)).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.text()
    );
    let envelope: api::ApiErrorResponse = parse(&reply);
    let api::ApiError::Validation { fields, .. } = envelope.error else {
        panic!("{envelope:?}")
    };
    assert!(fields.contains_key("clientMessageId"), "{fields:?}");
    // A parent off this room's root timeline: another room's, or a reply.
    for parent in [ALL_TALK_MESSAGE, JASONS_REPLY] {
        let body = json!({"parentMessageId": parent, "name": "Elsewhere", "message": reply_body("elsewhere", "Hi")});
        let reply = david.write(json_body(Method::POST, &path, &body)).await;
        assert_eq!(
            reply.status,
            StatusCode::NOT_FOUND,
            "{parent}: {}",
            reply.text()
        );
    }
    // A blank first reply, and a name over 100 characters.
    let body = json!({"parentMessageId": THREAD_PARENT, "name": null, "message": reply_body("blank", "  ")});
    let reply = david.write(json_body(Method::POST, &path, &body)).await;
    assert_eq!(
        (reply.status, tag(&reply)),
        (StatusCode::UNPROCESSABLE_ENTITY, "Validation".into())
    );
    let body = json!({"parentMessageId": 935962056, "name": "n".repeat(101), "message": reply_body("long", "Hi")});
    let reply = david.write(json_body(Method::POST, &path, &body)).await;
    assert_eq!(
        (reply.status, tag(&reply)),
        (StatusCode::UNPROCESSABLE_ENTITY, "Validation".into())
    );
    server.abort();
}

#[tokio::test]
async fn replies_unread_and_read_answer_and_publish() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut jason = a.sign_in(JASON).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let topics = [format!("room:{DESIGNERS}"), format!("thread:{THREAD}")];
    let mut sync = Sync::connect(addr, &david.cookie_header(), &topics).await;
    sync.welcome().await;

    let path = format!("/api/v1/threads/{THREAD}/messages");
    let reply = jason
        .write(json_body(
            Method::POST,
            &path,
            &reply_body("jason-1", "Replying here"),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let message: api::MessageDTO = parse(&reply);
    assert_eq!(message.thread_id, Some(THREAD));
    let reply = jason
        .write(json_body(
            Method::POST,
            &path,
            &reply_body("jason-1", "Replying here"),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(parse::<api::MessageDTO>(&reply).id, message.id);

    // David is a member of thread 1: the reply lands on its topic, makes it unread for him, and
    // moves the parent's indicator and the thread's count on both topics.
    let message_id = message.id;
    let room = format!("room:{DESIGNERS}");
    let thread = format!("thread:{THREAD}");
    let events = gather(
        &mut sync,
        &[
            &|event| matches!(&event.payload, api::SyncPayload::MessageCreated(created) if created.id == message_id),
            &|event| matches!(event.payload, api::SyncPayload::ThreadUnread(_)),
            &|event| matches!(event.payload, api::SyncPayload::ThreadIndicator(_)),
            &on(room.clone(), |payload| matches!(payload, api::SyncPayload::ThreadUpdated(_))),
            &on(thread.clone(), |payload| matches!(payload, api::SyncPayload::ThreadUpdated(_))),
        ],
    )
    .await;
    assert_eq!(events[0].topic, thread);
    assert_eq!(events[1].topic, "user");
    assert_eq!(
        events[1].payload,
        api::SyncPayload::ThreadUnread(api::ThreadUnread {
            thread_id: THREAD,
            room_id: DESIGNERS,
            refresh_only: false
        })
    );
    assert_eq!(events[2].topic, room);
    let api::SyncPayload::ThreadIndicator(changed) = events[2].payload.clone() else {
        unreachable!()
    };
    assert_eq!(
        (changed.room_id, changed.parent_message_id),
        (DESIGNERS, THREAD_PARENT)
    );
    let indicator = changed.thread.expect("thread 1's indicator");
    assert_eq!(indicator.replier_ids.first(), Some(&JASON));
    for event in &events[3..] {
        let api::SyncPayload::ThreadUpdated(updated) = &event.payload else {
            unreachable!()
        };
        assert_eq!(
            (updated.id, updated.reply_count),
            (THREAD, indicator.reply_count)
        );
    }

    // Reading it clears the membership and tells David's other tabs.
    let read = david
        .write(
            Req::new(Method::POST, &format!("/api/v1/threads/{THREAD}/read"))
                .header("accept", "application/json"),
        )
        .await;
    assert_eq!(read.status, StatusCode::OK, "{}", read.text());
    let state: api::ThreadMembershipState = parse(&read);
    assert_eq!(state.membership.unread_at, None);
    let event = sync
        .until(
            |event| matches!(event.payload, api::SyncPayload::ThreadRead(_)),
            |_| false,
        )
        .await;
    assert_eq!(
        (event.topic.as_str(), event.payload),
        (
            "user",
            api::SyncPayload::ThreadRead(api::ThreadRead {
                thread_id: THREAD,
                room_id: DESIGNERS
            })
        )
    );
    // Kevin never joined it.
    let reply = kevin
        .write(
            Req::new(Method::POST, &format!("/api/v1/threads/{THREAD}/read"))
                .header("accept", "application/json"),
        )
        .await;
    assert_eq!(
        (reply.status, tag(&reply)),
        (StatusCode::NOT_FOUND, "NotFound".into())
    );

    // Deleting a reply refreshes every room member's row.
    let reply = jason
        .write(
            Req::new(Method::DELETE, &format!("/api/v1/messages/{}", message.id))
                .header("accept", "application/json"),
        )
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    let event = sync
        .until(
            |event| matches!(&event.payload, api::SyncPayload::ThreadUnread(unread) if unread.refresh_only),
            |_| false,
        )
        .await;
    assert_eq!(event.topic, "user");

    // A locked thread refuses replies, its moderators' too; a blank reply is a 422.
    let reply = david
        .write(json_body(
            Method::POST,
            &format!("/api/v1/threads/{LOCKED_THREAD}/messages"),
            &reply_body("locked", "Hello?"),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.text());
    assert_eq!(
        envelope(&reply),
        api::ApiError::Forbidden {
            message: "This thread is locked".into()
        }
    );
    let reply = david
        .write(json_body(Method::POST, &path, &reply_body("blank", " ")))
        .await;
    assert_eq!(
        (reply.status, tag(&reply)),
        (StatusCode::UNPROCESSABLE_ENTITY, "Validation".into())
    );
    server.abort();
}

#[tokio::test]
async fn joining_and_leaving_answer() {
    let Some(a) = app(true).await else { return };
    let mut kevin = a.sign_in(KEVIN).await;
    let path = format!("/api/v1/threads/{THREAD}/join");
    let reply = kevin
        .write(Req::new(Method::POST, &path).header("accept", "application/json"))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let state: api::ThreadMembershipState = parse(&reply);
    assert_eq!(
        (state.membership.thread_id, state.membership.involvement),
        (THREAD, api::ThreadInvolvement::Mentions)
    );
    let reply = kevin
        .write(json_body(
            Method::POST,
            &path,
            &json!({"involvement": "everything"}),
        ))
        .await;
    assert_eq!(
        parse::<api::ThreadMembershipState>(&reply)
            .membership
            .involvement,
        api::ThreadInvolvement::Everything
    );
    // No involvement keeps the current one.
    let reply = kevin
        .write(json_body(
            Method::POST,
            &path,
            &json!({"involvement": null}),
        ))
        .await;
    assert_eq!(
        parse::<api::ThreadMembershipState>(&reply)
            .membership
            .involvement,
        api::ThreadInvolvement::Everything
    );
    let reply = kevin
        .write(json_body(
            Method::POST,
            &path,
            &json!({"involvement": "loud"}),
        ))
        .await;
    assert_eq!(
        (reply.status, tag(&reply)),
        (StatusCode::UNPROCESSABLE_ENTITY, "Validation".into())
    );
    let reply = kevin
        .write(Req::new(Method::DELETE, &path).header("accept", "application/json"))
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    let detail: api::ThreadDetail =
        parse(&kevin.send(get(&format!("/api/v1/threads/{THREAD}"))).await);
    assert_eq!(detail.membership, None);
}

/// The auto-archive duration is a thread setting: whoever may rename it may set it, only to one
/// of the durations the model allows, and both topics hear the change.
#[tokio::test]
async fn setting_the_auto_archive_duration_answers_and_publishes() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let topics = [format!("room:{DESIGNERS}"), "thread:8".to_string()];
    let mut sync = Sync::connect(addr, &kevin.cookie_header(), &topics).await;
    sync.welcome().await;
    let path = "/api/v1/threads/8";

    // Kevin is neither a moderator nor the thread's creator.
    let reply = kevin
        .write(json_body(
            Method::PATCH,
            path,
            &json!({"autoArchiveAfterMinutes": 1440}),
        ))
        .await;
    assert_eq!(
        (reply.status, tag(&reply)),
        (StatusCode::FORBIDDEN, "Forbidden".into()),
        "{}",
        reply.text()
    );

    // A duration the model doesn't allow.
    let reply = david
        .write(json_body(
            Method::PATCH,
            path,
            &json!({"autoArchiveAfterMinutes": 720}),
        ))
        .await;
    assert_eq!(
        (reply.status, tag(&reply)),
        (StatusCode::UNPROCESSABLE_ENTITY, "Validation".into()),
        "{}",
        reply.text()
    );
    let envelope: api::ApiErrorResponse = parse(&reply);
    let api::ApiError::Validation { fields, .. } = envelope.error else {
        panic!("{envelope:?}")
    };
    assert!(fields.contains_key("autoArchiveAfterMinutes"), "{fields:?}");

    // One week, then a day: the duration alone changes, and both topics hear it.
    let reply = david
        .write(json_body(
            Method::PATCH,
            path,
            &json!({"autoArchiveAfterMinutes": 10080}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let detail: api::ThreadDetail = parse(&reply);
    assert_eq!(
        (detail.thread.auto_archive_after_minutes, detail.thread.status),
        (10080, api::ThreadStatus::Active)
    );
    let updated = |payload: &api::SyncPayload| matches!(payload, api::SyncPayload::ThreadUpdated(_));
    let events = gather(
        &mut sync,
        &[
            &on(format!("room:{DESIGNERS}"), updated),
            &on("thread:8".into(), updated),
        ],
    )
    .await;
    for event in events {
        let api::SyncPayload::ThreadUpdated(thread) = event.payload else {
            unreachable!()
        };
        assert_eq!(thread.auto_archive_after_minutes, 10080);
    }
    let reply = david
        .write(json_body(
            Method::PATCH,
            path,
            &json!({"name": "Renamed", "autoArchiveAfterMinutes": 1440}),
        ))
        .await;
    let detail: api::ThreadDetail = parse(&reply);
    assert_eq!(
        (
            detail.thread.name.as_str(),
            detail.thread.auto_archive_after_minutes
        ),
        ("Renamed", 1440)
    );
    server.abort();
}

/// A board never auto-archives, so its post refuses a duration, even from its owner, and keeps
/// the stored one. Other fields in later updates still save.
#[tokio::test]
async fn a_board_post_refuses_an_auto_archive_duration() {
    let Some(a) = app(true).await else { return };
    let (_addr, server) = serve(&a).await;
    let mut kevin = a.sign_in(KEVIN).await;
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "INSERT INTO memberships (room_id, user_id, created_at, updated_at) VALUES (?, ?, '2026-03-02 15:00:00', '2026-03-02 15:00:00')",
                [BOARD, KEVIN],
            )?;
            tx.conn().execute(
                "UPDATE channel_threads SET work_owner_id = ? WHERE id = ?",
                [KEVIN, BOARD_THREAD],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let path = format!("/api/v1/threads/{BOARD_THREAD}");
    let before: api::ThreadDetail = parse(&kevin.send(get(&path)).await);
    assert!(before.permissions.can_rename, "{before:?}");
    let stored = before.thread.auto_archive_after_minutes;
    let target = if stored == 60 { 10080 } else { 60 };

    let reply = kevin
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"autoArchiveAfterMinutes": target}),
        ))
        .await;
    assert_eq!(
        (reply.status, tag(&reply)),
        (StatusCode::UNPROCESSABLE_ENTITY, "Validation".into()),
        "{}",
        reply.text()
    );
    let envelope: api::ApiErrorResponse = parse(&reply);
    let api::ApiError::Validation { fields, .. } = envelope.error else {
        panic!("{envelope:?}")
    };
    assert!(fields.contains_key("autoArchiveAfterMinutes"), "{fields:?}");
    let after: api::ThreadDetail = parse(&kevin.send(get(&path)).await);
    assert_eq!(after.thread.auto_archive_after_minutes, stored);

    let reply = kevin
        .write(json_body(Method::PATCH, &path, &json!({"name": "Still works"})))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let renamed: api::ThreadDetail = parse(&reply);
    assert_eq!(
        (renamed.thread.name.as_str(), renamed.thread.auto_archive_after_minutes),
        ("Still works", stored)
    );
    server.abort();
}

#[tokio::test]
async fn changing_and_deleting_threads_answer_and_publish() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let topics = [format!("room:{DESIGNERS}"), "thread:8".to_string()];
    let mut sync = Sync::connect(addr, &kevin.cookie_header(), &topics).await;
    sync.welcome().await;
    let path = "/api/v1/threads/8";

    // Kevin may do none of it.
    for body in [
        json!({"name": "Mine"}),
        json!({"status": "closed"}),
        json!({"status": "locked"}),
    ] {
        let reply = kevin.write(json_body(Method::PATCH, path, &body)).await;
        assert_eq!(
            (reply.status, tag(&reply)),
            (StatusCode::FORBIDDEN, "Forbidden".into()),
            "{body}"
        );
    }
    let reply = kevin
        .write(Req::new(Method::DELETE, path).header("accept", "application/json"))
        .await;
    assert_eq!(
        (reply.status, tag(&reply)),
        (StatusCode::FORBIDDEN, "Forbidden".into())
    );

    // David renames and closes it; both topics hear it.
    let reply = david
        .write(json_body(
            Method::PATCH,
            path,
            &json!({"name": "Renamed review", "status": "closed"}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let detail: api::ThreadDetail = parse(&reply);
    assert_eq!(
        (detail.thread.name.as_str(), detail.thread.status),
        ("Renamed review", api::ThreadStatus::Closed)
    );
    let updated =
        |payload: &api::SyncPayload| matches!(payload, api::SyncPayload::ThreadUpdated(_));
    let events = gather(
        &mut sync,
        &[
            &on(format!("room:{DESIGNERS}"), updated),
            &on("thread:8".into(), updated),
        ],
    )
    .await;
    for event in events {
        let api::SyncPayload::ThreadUpdated(thread) = event.payload else {
            unreachable!()
        };
        assert_eq!(
            (thread.id, thread.name.as_str(), thread.status),
            (8, "Renamed review", api::ThreadStatus::Closed)
        );
    }
    let reply = david
        .write(json_body(Method::PATCH, path, &json!({"name": ""})))
        .await;
    assert_eq!(
        (reply.status, tag(&reply)),
        (StatusCode::UNPROCESSABLE_ENTITY, "Validation".into())
    );

    // Locking, then unlocking.
    let reply = david
        .write(json_body(Method::PATCH, path, &json!({"status": "locked"})))
        .await;
    assert_eq!(
        parse::<api::ThreadDetail>(&reply).thread.status,
        api::ThreadStatus::Locked
    );
    let reply = david
        .write(json_body(Method::PATCH, path, &json!({"status": "active"})))
        .await;
    let detail: api::ThreadDetail = parse(&reply);
    assert_ne!(detail.thread.status, api::ThreadStatus::Locked);

    // Deleting it.
    let reply = david
        .write(Req::new(Method::DELETE, path).header("accept", "application/json"))
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.text());
    let removed = |payload: &api::SyncPayload| {
        *payload
            == api::SyncPayload::ThreadRemoved(api::ThreadRemoved {
                thread_id: 8,
                room_id: DESIGNERS,
            })
    };
    gather(
        &mut sync,
        &[
            &on(format!("room:{DESIGNERS}"), removed),
            &on("thread:8".into(), removed),
        ],
    )
    .await;
    let reply = david.send(get(path)).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    server.abort();
}

#[tokio::test]
async fn deleting_a_thread_clears_its_parents_indicator() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut sync =
        Sync::connect(addr, &david.cookie_header(), &[format!("room:{DESIGNERS}")]).await;
    sync.welcome().await;
    let reply = david
        .write(
            Req::new(Method::DELETE, &format!("/api/v1/threads/{THREAD}"))
                .header("accept", "application/json"),
        )
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.text());
    let events = gather(
        &mut sync,
        &[
            &|event| matches!(&event.payload, api::SyncPayload::ThreadIndicator(changed) if changed.parent_message_id == THREAD_PARENT),
            &|event| matches!(event.payload, api::SyncPayload::ThreadRemoved(_)),
        ],
    )
    .await;
    let api::SyncPayload::ThreadIndicator(changed) = events[0].payload.clone() else {
        unreachable!()
    };
    assert_eq!(changed.thread, None);
    server.abort();
}

/// The classic frames of the thread actions: joining, a reply (its append, the parent's
/// indicator and the member's unread ping), a reply's deletion (the refresh pings), and a
/// thread's creation, change and deletion.




#[tokio::test]
async fn a_board_posts_owner_may_rename_it_and_boards_take_no_threads() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let mut jz = a.sign_in(JZ).await;
    // Kevin owns the post's work; JZ is on the board too, with no say over it.
    a.db()
        .write(|tx| {
            for user in [KEVIN, JZ] {
                tx.conn().execute(
                    "INSERT INTO memberships (room_id, user_id, created_at, updated_at) VALUES (?, ?, '2026-03-02 15:00:00', '2026-03-02 15:00:00')",
                    [BOARD, user],
                )?;
            }
            tx.conn().execute(
                "UPDATE channel_threads SET work_owner_id = ? WHERE id = ?",
                [KEVIN, BOARD_THREAD],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let path = format!("/api/v1/threads/{BOARD_THREAD}");
    let detail: api::ThreadDetail = parse(&kevin.send(get(&path)).await);
    assert!(detail.permissions.can_rename, "{detail:?}");
    let detail: api::ThreadDetail = parse(&jz.send(get(&path)).await);
    assert!(!detail.permissions.can_rename, "{detail:?}");
    let reply = jz
        .write(json_body(Method::PATCH, &path, &json!({"name": "JZ's"})))
        .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.text());

    let mut sync = Sync::connect(addr, &kevin.cookie_header(), &[format!("room:{BOARD}")]).await;
    sync.welcome().await;
    // An empty change publishes nothing; the rename that follows is the first update.
    let reply = kevin
        .write(json_body(Method::PATCH, &path, &json!({})))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let reply = kevin
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"name": "Kevin's post"}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let event = sync
        .until(
            |event| matches!(event.payload, api::SyncPayload::ThreadUpdated(_)),
            |_| false,
        )
        .await;
    let api::SyncPayload::ThreadUpdated(updated) = event.payload else {
        unreachable!()
    };
    assert_eq!(updated.name, "Kevin's post");

    // A board takes posts, not threads.
    let body = json!({"parentMessageId": THREAD_PARENT, "name": null, "message": reply_body("on-board", "Hi")});
    let reply = david
        .write(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{BOARD}/threads"),
            &body,
        ))
        .await;
    assert_eq!(
        (reply.status, tag(&reply)),
        (StatusCode::FORBIDDEN, "Forbidden".into()),
        "{}",
        reply.text()
    );
    server.abort();
}

#[tokio::test]
async fn board_post_permissions_follow_each_classic_role_and_lifecycle() {
    use campfire_db::{NewUser, Room, ThreadMembership, User};

    let a = app(true)
        .await
        .expect("the default frozen seed is required");
    let other = a
        .db()
        .write(|tx| {
            let other = User::create(
                tx,
                NewUser {
                    name: "Board reader".into(),
                    email_address: Some("board-reader@example.test".into()),
                    ..Default::default()
                },
            )?;
            // Give each authority to a different person. Jason is a human member who created
            // the board, JZ authored its post, Kevin owns it, and David is an administrator.
            tx.conn().execute("UPDATE users SET role = 0 WHERE id = ?", [JASON])?;
            tx.conn().execute("UPDATE rooms SET creator_id = ? WHERE id = ?", [JASON, BOARD])?;
            Room::find(tx.conn(), BOARD)?.grant_to(tx, &[KEVIN, JZ, other.id])?;
            tx.conn().execute(
                "UPDATE channel_threads SET creator_id = ?, work_owner_id = ?, closed_at = NULL, locked_at = NULL WHERE id = ?",
                [JZ, KEVIN, BOARD_THREAD],
            )?;
            tx.conn().execute("DELETE FROM thread_memberships WHERE thread_id = ?", [BOARD_THREAD])?;
            Ok(other.id)
        })
        .await
        .unwrap();
    let path = format!("/api/v1/threads/{BOARD_THREAD}");
    let flags = |permissions: api::ThreadPermissions| {
        [
            permissions.can_rename,
            permissions.can_close,
            permissions.can_reopen,
            permissions.can_lock,
            permissions.can_unlock,
            permissions.can_delete,
            permissions.can_convert_work,
            permissions.can_manage_work,
            permissions.can_update_work_status,
            permissions.can_assign_work,
            permissions.can_remove_work,
        ]
    };
    // Rename, close, reopen, lock, unlock, delete, convert, manage, status, assign, remove.
    for (user, wanted) in [
        (
            JZ,
            [
                true, false, false, false, false, false, false, true, true, true, false,
            ],
        ),
        (
            KEVIN,
            [
                true, false, false, false, false, false, false, true, true, false, false,
            ],
        ),
        (
            JASON,
            [
                true, true, false, true, false, true, false, true, true, true, false,
            ],
        ),
        (
            DAVID,
            [
                true, true, false, true, false, true, false, true, true, true, false,
            ],
        ),
        (other, [false; 11]),
    ] {
        let mut browser = a.sign_in(user).await;
        let detail: api::ThreadDetail = parse(&browser.send(get(&path)).await);
        assert_eq!(flags(detail.permissions), wanted, "user {user}: {detail:?}");
    }
    for user in [JZ, KEVIN, other] {
        let mut browser = a.sign_in(user).await;
        for status in ["closed", "locked"] {
            let reply = browser
                .write(json_body(Method::PATCH, &path, &json!({"status": status})))
                .await;
            assert_eq!(
                reply.status,
                StatusCode::FORBIDDEN,
                "user {user}, {status}: {}",
                reply.text()
            );
        }
        let reply = browser
            .write(json_body(Method::DELETE, &path, &json!({})))
            .await;
        assert_eq!(
            reply.status,
            StatusCode::FORBIDDEN,
            "user {user}: {}",
            reply.text()
        );
    }

    let mut creator = a.sign_in(JASON).await;
    let reply = creator
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"status": "closed"}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    // Closing does not join the moderator. Even author/creator/admin authority cannot reopen
    // a closed post until the person joins it; a joined ordinary member can reopen it.
    for user in [JZ, JASON, DAVID, other] {
        let mut browser = a.sign_in(user).await;
        let detail: api::ThreadDetail = parse(&browser.send(get(&path)).await);
        assert!(!detail.permissions.can_reopen, "user {user}: {detail:?}");
        let reply = browser
            .write(json_body(
                Method::PATCH,
                &path,
                &json!({"status": "active"}),
            ))
            .await;
        assert_eq!(
            reply.status,
            StatusCode::FORBIDDEN,
            "user {user}: {}",
            reply.text()
        );
    }
    a.db()
        .write(move |tx| ThreadMembership::join(tx, BOARD_THREAD, other).map(|_| ()))
        .await
        .unwrap();
    let mut reader = a.sign_in(other).await;
    let detail: api::ThreadDetail = parse(&reader.send(get(&path)).await);
    assert!(detail.permissions.can_reopen, "{detail:?}");
    assert!(!detail.permissions.can_manage_work);
    let reply = reader
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"status": "active"}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());

    let reply = creator
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"status": "locked"}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let detail: api::ThreadDetail = parse(&reader.send(get(&path)).await);
    assert!(
        !detail.permissions.can_reopen && !detail.permissions.can_unlock,
        "{detail:?}"
    );
    let reply = reader
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"status": "active"}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.text());
    let detail: api::ThreadDetail = parse(&creator.send(get(&path)).await);
    assert!(
        detail.permissions.can_unlock && !detail.permissions.can_lock,
        "{detail:?}"
    );
    let reply = creator
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"status": "active"}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let reply = creator
        .write(json_body(Method::DELETE, &path, &json!({})))
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.text());
}


#[tokio::test]
async fn private_threads_require_access_members_and_visibility_is_immutable() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let created = david.write(json_body(Method::POST,
        &format!("/api/v1/rooms/{DESIGNERS}/threads"),
        &json!({"parentMessageId": UNTHREADED, "visibility": "private",
            "message": reply_body("private-api-create", "Private first reply")}))).await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.text());
    let created: api::ThreadCreated = parse(&created);
    let id = created.detail.thread.id;
    assert_eq!(created.detail.thread.visibility, api::ThreadVisibility::Private);
    for path in [format!("/api/v1/threads/{id}"), format!("/api/v1/threads/{id}/messages"),
        format!("/api/v1/threads/{id}/members"), format!("/rooms/{DESIGNERS}/threads/{id}"),
        format!("/rooms/{DESIGNERS}/threads/{id}/messages")] {
        assert_eq!(kevin.send(get(&path)).await.status, StatusCode::NOT_FOUND, "{path}");
    }
    let list: api::ThreadList = parse(&kevin.send(get(&format!("/api/v1/rooms/{DESIGNERS}/threads?state=all"))).await);
    assert!(!list.threads.iter().any(|row| row.thread.id == id));
    let denied = kevin.write(json_body(Method::POST, &format!("/api/v1/threads/{id}/join"), &json!({}))).await;
    assert_eq!(denied.status, StatusCode::NOT_FOUND);
    let changed = david.write(json_body(Method::PATCH, &format!("/api/v1/threads/{id}"), &json!({"visibility": "public"}))).await;
    assert!(matches!(envelope(&changed), api::ApiError::ThreadAccess {reason: api::ThreadAccessRefusal::VisibilityImmutable, ..}));
    let denied = david.write(json_body(Method::POST, &format!("/api/v1/threads/{id}/members"), &json!({"userId": -1}))).await;
    assert!(matches!(envelope(&denied), api::ApiError::ThreadAccess {reason: api::ThreadAccessRefusal::ParentRoomRequired, ..}));
    let members = david.write(json_body(Method::POST, &format!("/api/v1/threads/{id}/members"), &json!({"userId": KEVIN}))).await;
    assert_eq!(members.status, StatusCode::OK, "{}", members.text());
    assert!(parse::<api::ThreadAccessMembers>(&members).members.iter().any(|member| member.user_id == KEVIN));
    assert_eq!(kevin.send(get(&format!("/api/v1/threads/{id}"))).await.status, StatusCode::OK);
    // A current access member may invite another parent room member, but cannot remove them.
    let added = kevin.write(json_body(Method::POST, &format!("/api/v1/threads/{id}/members"), &json!({"userId": JZ}))).await;
    assert_eq!(added.status, StatusCode::OK, "{}", added.text());
    let denied = kevin.write(json_body(Method::DELETE, &format!("/api/v1/threads/{id}/members/{JZ}"), &json!({}))).await;
    assert!(matches!(envelope(&denied), api::ApiError::ThreadAccess {reason: api::ThreadAccessRefusal::RemoveForbidden, ..}));
    let left = kevin.write(json_body(Method::DELETE, &format!("/api/v1/threads/{id}/members/{KEVIN}"), &json!({}))).await;
    assert_eq!(left.status, StatusCode::NO_CONTENT);
    assert_eq!(kevin.send(get(&format!("/api/v1/threads/{id}"))).await.status, StatusCode::NOT_FOUND);
    let hidden: api::MessagePage = parse(&kevin.send(get(&format!("/api/v1/rooms/{DESIGNERS}/messages"))).await);
    assert!(hidden.messages.iter().filter(|message| message.id == UNTHREADED).all(|message| message.thread.is_none()));
    let mentioned = david.write(json_body(Method::POST, &format!("/api/v1/threads/{id}/messages"), &reply_body("private-api-mention", "Welcome back @[Kevin]"))).await;
    assert_eq!(mentioned.status, StatusCode::CREATED, "{}", mentioned.text());
    assert_eq!(kevin.send(get(&format!("/api/v1/threads/{id}"))).await.status, StatusCode::OK);
    let removed = david.write(json_body(Method::DELETE, &format!("/api/v1/threads/{id}/members/{JZ}"), &json!({}))).await;
    assert_eq!(removed.status, StatusCode::NO_CONTENT);
    let message: api::MessageDTO = parse(&mentioned);
    let edited = david.write(json_body(Method::PATCH, &format!("/api/v1/messages/{}", message.id), &json!({"markdownSource": "Welcome back @[Kevin] and @[JZ]"}))).await;
    assert_eq!(edited.status, StatusCode::OK, "{}", edited.text());
    let members: api::ThreadAccessMembers = parse(&david.send(get(&format!("/api/v1/threads/{id}/members"))).await);
    assert!(members.members.iter().any(|member| member.user_id == JZ));
    let public: api::ThreadDetail = parse(&kevin.send(get(&format!("/api/v1/threads/{THREAD}"))).await);
    assert_eq!(public.thread.visibility, api::ThreadVisibility::Public);
    let denied = david.send(get(&format!("/api/v1/threads/{THREAD}/members"))).await;
    assert!(matches!(envelope(&denied), api::ApiError::ThreadAccess {reason: api::ThreadAccessRefusal::NotPrivate, ..}));
}

#[tokio::test]
async fn reply_summary_visibility_is_sent_only_to_current_thread_readers() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let created = david.write(json_body(Method::POST,
        &format!("/api/v1/rooms/{DESIGNERS}/threads"),
        &json!({"parentMessageId": UNTHREADED, "visibility": "private",
            "message": reply_body("summary-visibility", "A private reply")}))).await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.text());
    let created: Value = parse(&created);
    let id = created["detail"]["thread"]["id"].as_i64().unwrap();
    assert_eq!(created["detail"]["parentMessage"]["thread"]["visibility"], "private");

    let page_path = format!("/api/v1/rooms/{DESIGNERS}/messages");
    let root = |page: Value| page["messages"].as_array().unwrap().iter()
        .find(|message| message["id"] == UNTHREADED).unwrap().clone();
    let parent = root(parse(&david.send(get(&page_path)).await));
    assert_eq!(parent["thread"]["visibility"], "private");
    assert_eq!(parent["thread"]["replyCount"], 1);
    assert_eq!(parent["thread"]["replierIds"], json!([DAVID]));
    let hidden = root(parse(&kevin.send(get(&page_path)).await));
    assert_eq!(hidden["thread"], Value::Null);

    let added = david.write(json_body(Method::POST, &format!("/api/v1/threads/{id}/members"), &json!({"userId": KEVIN}))).await;
    assert_eq!(added.status, StatusCode::OK, "{}", added.text());
    let shared = root(parse(&kevin.send(get(&page_path)).await));
    assert_eq!(shared["thread"], parent["thread"]);
    let public_page: Value = parse(&kevin.send(get(&page_path)).await);
    let public = public_page["messages"].as_array().unwrap().iter()
        .find(|message| message["id"] == THREAD_PARENT).unwrap();
    assert_eq!(public["thread"]["visibility"], "public");

    let removed = david.write(json_body(Method::DELETE, &format!("/api/v1/threads/{id}/members/{KEVIN}"), &json!({}))).await;
    assert_eq!(removed.status, StatusCode::NO_CONTENT);
    let hidden = root(parse(&kevin.send(get(&page_path)).await));
    assert_eq!(hidden["thread"], Value::Null);
}

#[tokio::test]
async fn private_thread_removal_cancels_an_open_sync_subscription() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let kevin = a.sign_in(KEVIN).await;
    let created: api::ThreadCreated = parse(&david.write(json_body(Method::POST,
        &format!("/api/v1/rooms/{DESIGNERS}/threads"),
        &json!({"parentMessageId": UNTHREADED, "visibility": "private",
            "message": reply_body("private-sync-create", "Private sync first reply")}))).await);
    let id = created.detail.thread.id;
    let added = david.write(json_body(Method::POST, &format!("/api/v1/threads/{id}/members"), &json!({"userId": KEVIN}))).await;
    assert_eq!(added.status, StatusCode::OK, "{}", added.text());
    a.booted.app.broadcasts.settle_sync().await;
    let (addr, server) = serve(&a).await;
    let mut sync = Sync::connect(addr, &kevin.cookie_header(), &[format!("room:{DESIGNERS}"), format!("thread:{id}")]).await;
    sync.welcome().await;
    let outsider = a.sign_in(JZ).await;
    let mut uninvited = Sync::connect(addr, &outsider.cookie_header(), &[format!("room:{DESIGNERS}"), format!("thread:{id}")]).await;
    uninvited.welcome().await;
    let posted = david.write(json_body(Method::POST, &format!("/api/v1/threads/{id}/messages"), &reply_body("private-sync-before", "Visible before removal"))).await;
    assert_eq!(posted.status, StatusCode::CREATED);
    sync.until(|event| matches!(&event.payload, api::SyncPayload::MessageCreated(message) if message.client_message_id == "private-sync-before"), |_| false).await;
    a.booted.app.broadcasts.settle_sync().await;
    let marker = david.write(json_body(Method::POST, &format!("/api/v1/rooms/{DESIGNERS}/messages"), &reply_body("private-sync-outsider-marker", "Public marker for uninvited reader"))).await;
    assert_eq!(marker.status, StatusCode::CREATED);
    uninvited.until(|event| matches!(&event.payload, api::SyncPayload::MessageCreated(message) if message.client_message_id == "private-sync-outsider-marker"),
        |event| event.topic == format!("thread:{id}") || matches!(&event.payload,
            api::SyncPayload::ThreadUpdated(thread) | api::SyncPayload::ThreadCreated(thread) if thread.id == id)
            || matches!(&event.payload, api::SyncPayload::ThreadIndicator(change) if change.thread.as_ref().is_some_and(|thread| thread.thread_id == id))).await;
    let removed = david.write(json_body(Method::DELETE, &format!("/api/v1/threads/{id}/members/{KEVIN}"), &json!({}))).await;
    assert_eq!(removed.status, StatusCode::NO_CONTENT);
    sync.until(|event| matches!(&event.payload, api::SyncPayload::ThreadRemoved(removed) if removed.thread_id == id && event.topic == "user"), |_| false).await;
    loop {
        match sync.next().await {
            Some(api::ServerFrame::Batch { .. }) => {},
            Some(api::ServerFrame::Bye { reconnect: true, .. }) => break,
            other => panic!("revocation should disconnect existing subscriptions, got {other:?}"),
        }
    }
    let mut sync = Sync::connect(addr, &kevin.cookie_header(), &[format!("room:{DESIGNERS}"), format!("thread:{id}")]).await;
    sync.welcome().await;
    let posted = david.write(json_body(Method::POST, &format!("/api/v1/threads/{id}/messages"), &reply_body("private-sync-after", "Hidden after removal"))).await;
    assert_eq!(posted.status, StatusCode::CREATED);
    a.booted.app.broadcasts.settle_sync().await;
    let marker = david.write(json_body(Method::POST, &format!("/api/v1/rooms/{DESIGNERS}/messages"), &reply_body("private-sync-marker", "Room still visible"))).await;
    assert_eq!(marker.status, StatusCode::CREATED);
    sync.until(|event| matches!(&event.payload, api::SyncPayload::MessageCreated(message) if message.client_message_id == "private-sync-marker"),
        |event| event.topic == format!("thread:{id}") || matches!(&event.payload,
            api::SyncPayload::ThreadUpdated(thread) | api::SyncPayload::ThreadCreated(thread) if thread.id == id)
            || matches!(&event.payload, api::SyncPayload::ThreadIndicator(change) if change.thread.as_ref().is_some_and(|thread| thread.thread_id == id))).await;
    // Granting access again does not silently restore the old thread subscription.
    let added = david.write(json_body(Method::POST, &format!("/api/v1/threads/{id}/members"), &json!({"userId": KEVIN}))).await;
    assert_eq!(added.status, StatusCode::OK);
    a.booted.app.broadcasts.settle_sync().await;
    let posted = david.write(json_body(Method::POST, &format!("/api/v1/threads/{id}/messages"), &reply_body("private-sync-readded", "Requires a fresh subscription"))).await;
    assert_eq!(posted.status, StatusCode::CREATED);
    a.booted.app.broadcasts.settle_sync().await;
    let marker = david.write(json_body(Method::POST, &format!("/api/v1/rooms/{DESIGNERS}/messages"), &reply_body("private-sync-marker-readded", "Second room marker"))).await;
    assert_eq!(marker.status, StatusCode::CREATED);
    sync.until(|event| matches!(&event.payload, api::SyncPayload::MessageCreated(message) if message.client_message_id == "private-sync-marker-readded"), |event| event.topic == format!("thread:{id}")).await;
    server.abort();
}

#[tokio::test]
async fn private_thread_readded_member_gets_live_replies_after_a_fresh_subscription() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let kevin = a.sign_in(KEVIN).await;
    let created: api::ThreadCreated = parse(&david.write(json_body(Method::POST,
        &format!("/api/v1/rooms/{DESIGNERS}/threads"),
        &json!({"parentMessageId": UNTHREADED, "visibility": "private",
            "message": reply_body("private-readd-create", "Private first reply")}))).await);
    let id = created.detail.thread.id;
    let added = david.write(json_body(Method::POST, &format!("/api/v1/threads/{id}/members"), &json!({"userId": KEVIN}))).await;
    assert_eq!(added.status, StatusCode::OK, "{}", added.text());
    a.booted.app.broadcasts.settle_sync().await;
    let (addr, server) = serve(&a).await;
    let mut sync = Sync::connect(addr, &kevin.cookie_header(), &[format!("room:{DESIGNERS}"), format!("thread:{id}")]).await;
    sync.welcome().await;
    let removed = david.write(json_body(Method::DELETE, &format!("/api/v1/threads/{id}/members/{KEVIN}"), &json!({}))).await;
    assert_eq!(removed.status, StatusCode::NO_CONTENT);
    sync.until(|event| matches!(&event.payload, api::SyncPayload::ThreadRemoved(removed) if removed.thread_id == id && event.topic == "user"), |_| false).await;
    loop {
        match sync.next().await {
            Some(api::ServerFrame::Batch { .. }) => {},
            Some(api::ServerFrame::Bye { reconnect: true, .. }) => break,
            other => panic!("revocation should disconnect existing subscriptions, got {other:?}"),
        }
    }
    let mut sync = Sync::connect(addr, &kevin.cookie_header(), &[format!("room:{DESIGNERS}"), format!("thread:{id}")]).await;
    sync.welcome().await;
    let added = david.write(json_body(Method::POST, &format!("/api/v1/threads/{id}/members"), &json!({"userId": KEVIN}))).await;
    assert_eq!(added.status, StatusCode::OK);
    a.booted.app.broadcasts.settle_sync().await;
    sync.until(|event| matches!(&event.payload, api::SyncPayload::ThreadUpdated(thread) if thread.id == id && event.topic == "user"), |_| false).await;
    sync.send(json!({"t": "sub", "topics": [format!("thread:{id}")]} ) ).await;
    let posted = david.write(json_body(Method::POST, &format!("/api/v1/threads/{id}/messages"), &reply_body("private-readd-live", "Live after re-add"))).await;
    assert_eq!(posted.status, StatusCode::CREATED);
    sync.until(|event| matches!(&event.payload, api::SyncPayload::MessageCreated(message) if message.client_message_id == "private-readd-live"), |_| false).await;
    server.abort();
}

#[tokio::test]
async fn private_thread_review_scheduled_replay_renders_current_access_and_keeps_draft() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let created: api::ThreadCreated = parse(
        &david
            .write(json_body(
                Method::POST,
                &format!("/api/v1/rooms/{DESIGNERS}/threads"),
                &json!({"parentMessageId": UNTHREADED, "visibility": "private",
            "message": reply_body("private-replay-create", "Confidential reply target")}),
            ))
            .await,
    );
    let id = created.detail.thread.id;
    assert_eq!(
        david
            .write(json_body(
                Method::POST,
                &format!("/api/v1/threads/{id}/members"),
                &json!({"userId": KEVIN})
            ))
            .await
            .status,
        StatusCode::OK
    );
    a.booted.app.broadcasts.settle_sync().await;
    let (addr, server) = serve(&a).await;
    let mut sync = Sync::connect(addr, &kevin.cookie_header(), &[]).await;
    let Some(api::ServerFrame::Welcome { epoch, seq, .. }) = sync.next().await else {
        panic!("welcome");
    };
    let draft: api::ScheduledMessage = parse(&kevin.write(json_body(Method::POST,
        &format!("/api/v1/rooms/{DESIGNERS}/scheduled_messages"),
        &json!({"markdownSource": "My retained draft", "sendAt": "2099-01-01T12:00:00Z", "threadId": id, "replyToMessageId": created.message.id}))).await);
    assert_eq!(
        draft.reply_target.as_ref().unwrap().excerpt,
        "Confidential reply target"
    );
    sync.until(|event| matches!(&event.payload, api::SyncPayload::ScheduledChanged(message) if message.id == draft.id), |_| false).await;
    assert_eq!(
        david
            .write(json_body(
                Method::DELETE,
                &format!("/api/v1/threads/{id}/members/{KEVIN}"),
                &json!({})
            ))
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    a.booted.app.broadcasts.settle_sync().await;
    let mut replay = Sync::open(
        addr,
        &kevin.cookie_header(),
        &[],
        json!({"epoch": epoch, "seq": seq}),
    )
    .await;
    assert!(matches!(
        replay.next().await,
        Some(api::ServerFrame::Welcome { resumed: true, .. })
    ));
    let event = replay.until(|event| matches!(&event.payload, api::SyncPayload::ScheduledChanged(message) if message.id == draft.id), |_| false).await;
    let api::SyncPayload::ScheduledChanged(shown) = event.payload else {
        unreachable!()
    };
    assert!(
        shown.reply_target.is_none(),
        "revoked reply content: {:?}",
        shown.reply_target
    );
    assert!(!shown.sendable);
    assert_eq!(shown.markdown_source, "My retained draft");
    let list: api::ScheduledMessageList =
        parse(&kevin.send(get("/api/v1/scheduled_messages")).await);
    assert_eq!(
        list.scheduled_messages
            .iter()
            .find(|message| message.id == draft.id),
        Some(&shown)
    );
    server.abort();
}

#[tokio::test]
async fn private_thread_review_classic_links_treat_hidden_ids_as_missing() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let created: api::ThreadCreated = parse(
        &david
            .write(json_body(
                Method::POST,
                &format!("/api/v1/rooms/{DESIGNERS}/threads"),
                &json!({"parentMessageId": UNTHREADED, "visibility": "private",
            "message": reply_body("private-classic-create", "Hidden reply")}),
            ))
            .await,
    );
    let id = created.detail.thread.id;
    let message = created.message.id;
    let visible = david
        .get(&format!(
            "/rooms/{DESIGNERS}?thread={id}&message_id={message}"
        ))
        .await;
    assert_eq!(
        visible.location(),
        Some(format!("http://campfire.test/app/r/{DESIGNERS}/t/{id}?m={message}").as_str())
    );
    for (hidden, missing) in [
        (
            format!("/rooms/{DESIGNERS}?thread={id}"),
            format!("/rooms/{DESIGNERS}?thread=9999999999"),
        ),
        (
            format!("/rooms/{DESIGNERS}?thread={id}&message_id={message}"),
            format!("/rooms/{DESIGNERS}?thread=9999999999&message_id=9999999998"),
        ),
        (
            format!("/rooms/{DESIGNERS}?message_id={message}"),
            format!("/rooms/{DESIGNERS}?message_id=9999999998"),
        ),
        (
            format!("/rooms/{DESIGNERS}/threads/{id}/messages?message_id={message}"),
            format!("/rooms/{DESIGNERS}/threads/9999999999/messages?message_id=9999999998"),
        ),
        (
            format!("/rooms/{DESIGNERS}/threads/{id}/messages/{message}"),
            format!("/rooms/{DESIGNERS}/threads/9999999999/messages/9999999998"),
        ),
        (
            format!("/rooms/{DESIGNERS}/@{message}"),
            format!("/rooms/{DESIGNERS}/@9999999998"),
        ),
        (
            format!("/rooms/{DESIGNERS}/threads/{id}"),
            format!("/rooms/{DESIGNERS}/threads/9999999999"),
        ),
        (
            format!("/rooms/{DESIGNERS}/messages/{message}/fizzy_cards/new"),
            format!("/rooms/{DESIGNERS}/messages/9999999998/fizzy_cards/new"),
        ),
        (
            format!("/rooms/{DESIGNERS}/threads/{id}/messages/{message}/fizzy_cards/new"),
            format!("/rooms/{DESIGNERS}/threads/9999999999/messages/9999999998/fizzy_cards/new"),
        ),
        (
            format!("/rooms/{DESIGNERS}/messages?before={message}"),
            format!("/rooms/{DESIGNERS}/messages?before=9999999998"),
        ),
        (
            format!("/rooms/{DESIGNERS}/messages?after={message}"),
            format!("/rooms/{DESIGNERS}/messages?after=9999999998"),
        ),
        (
            format!("/messages/{message}/boosts/new"),
            "/messages/9999999998/boosts/new".into(),
        ),
        (
            format!("/threads/{id}/work/handoff/new"),
            "/threads/9999999999/work/handoff/new".into(),
        ),
        (
            format!("/threads/{id}/work/links"),
            "/threads/9999999999/work/links".into(),
        ),
    ] {
        for method in [Method::GET, Method::HEAD] {
            for accept in ["text/html", "application/json"] {
                let hidden_reply = kevin
                    .send(Req::new(method.clone(), &hidden).header("accept", accept))
                    .await;
                let missing_reply = kevin
                    .send(Req::new(method.clone(), &missing).header("accept", accept))
                    .await;
                assert_eq!(hidden_reply.status, missing_reply.status, "{hidden}");
                let normalize = |location: Option<&str>| {
                    location.map(|location| {
                        location
                            .replace("9999999998", "MESSAGE")
                            .replace(&message.to_string(), "MESSAGE")
                            .replace("9999999999", "THREAD")
                            .replace(&id.to_string(), "THREAD")
                    })
                };
                assert_eq!(
                    normalize(hidden_reply.location()),
                    normalize(missing_reply.location()),
                    "{hidden}"
                );
                assert_eq!(
                    normalize(Some(&hidden_reply.text())),
                    normalize(Some(&missing_reply.text())),
                    "{hidden} response body"
                );
            }
        }
    }
}

#[tokio::test]
async fn private_thread_unfollow_keeps_creator_access_until_members_self_delete() {
    let Some(a) = app(true).await else { return };
    let mut creator = a.sign_in(KEVIN).await;
    let created = creator.write(json_body(Method::POST,
        &format!("/api/v1/rooms/{DESIGNERS}/threads"),
        &json!({"parentMessageId": UNTHREADED, "visibility": "private",
            "message": reply_body("private-unfollow-create", "Private reply")}))).await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.text());
    let created: api::ThreadCreated = parse(&created);
    let id = created.detail.thread.id;
    assert!(created.detail.membership.is_some());
    for _ in 0..2 {
        let unfollowed = creator.write(json_body(Method::DELETE, &format!("/api/v1/threads/{id}/join"), &json!({}))).await;
        assert_eq!(unfollowed.status, StatusCode::NO_CONTENT, "{}", unfollowed.text());
        let response = creator.send(get(&format!("/api/v1/threads/{id}"))).await;
        assert_eq!(response.status, StatusCode::OK, "{}", response.text());
        let detail: api::ThreadDetail = parse(&response);
        assert!(detail.membership.is_none());
        let members: api::ThreadAccessMembers = parse(&creator.send(get(&format!("/api/v1/threads/{id}/members"))).await);
        assert!(members.members.iter().any(|member| member.user_id == KEVIN));
    }
    let followed = creator.write(json_body(Method::POST, &format!("/api/v1/threads/{id}/join"), &json!({}))).await;
    assert_eq!(followed.status, StatusCode::OK, "{}", followed.text());
    let left = creator.write(json_body(Method::DELETE, &format!("/api/v1/threads/{id}/members/{KEVIN}"), &json!({}))).await;
    assert_eq!(left.status, StatusCode::NO_CONTENT, "{}", left.text());
    assert_eq!(creator.send(get(&format!("/api/v1/threads/{id}"))).await.status, StatusCode::NOT_FOUND);
    a.db().read(move |conn| {
        assert!(campfire_db::Membership::find_by_room_and_user(conn, DESIGNERS, KEVIN)?.is_some());
        assert!(!campfire_db::ChannelThread::find(conn, id)?.access_members(conn)?.iter().any(|member| member.user_id == KEVIN));
        Ok(())
    }).await.unwrap();
    let added = creator.write(json_body(Method::POST, &format!("/api/v1/threads/{id}/members"), &json!({"userId":JZ}))).await;
    assert_eq!(added.status, StatusCode::OK, "{}", added.text());
    let roster: api::ThreadAccessMembers = parse(&added);
    assert_eq!(roster.members.iter().map(|member| member.user_id).collect::<Vec<_>>(), [JZ]);
    assert_eq!(creator.send(get(&format!("/api/v1/threads/{id}/members"))).await.status, StatusCode::NOT_FOUND);
    assert_eq!(creator.send(get(&format!("/api/v1/threads/{id}"))).await.status, StatusCode::NOT_FOUND);
    let removed = creator.write(json_body(Method::DELETE, &format!("/api/v1/threads/{id}/members/{JZ}"), &json!({}))).await;
    assert_eq!(removed.status, StatusCode::NO_CONTENT, "{}", removed.text());
    let restored = creator.write(json_body(Method::POST, &format!("/api/v1/threads/{id}/members"), &json!({"userId":KEVIN}))).await;
    assert_eq!(restored.status, StatusCode::OK, "{}", restored.text());
    assert_eq!(creator.send(get(&format!("/api/v1/threads/{id}"))).await.status, StatusCode::OK);
}

#[tokio::test]
async fn thread_timelines_include_system_notes_only_for_current_readers() {
    let Some(a) = app(true).await else { return };
    let (public_id, public_note, private_id, private_note) = a.db().write(|tx| {
        let public = campfire_db::ChannelThread::create(tx, campfire_db::NewChannelThread {
            room_id: DESIGNERS, creator_id: DAVID, name: Some("Public notes".into()),
            ..Default::default()
        })?;
        let private = campfire_db::ChannelThread::create(tx, campfire_db::NewChannelThread {
            room_id: DESIGNERS, creator_id: DAVID, name: Some("Private notes".into()),
            visibility: campfire_db::channel_thread::ThreadVisibility::Private, ..Default::default()
        })?;
        let mut notes = Vec::new();
        for (thread, text) in [(&public, "Public system note"), (&private, "Private system note")] {
            notes.push(campfire_db::Message::create(tx, campfire_db::NewMessage {
                room_id: DESIGNERS, thread_id: Some(thread.id), creator_id: DAVID,
                markdown_source: Some(text.into()), system_note: true, ..Default::default()
            })?.id);
        }
        Ok((public.id, notes[0], private.id, notes[1]))
    }).await.unwrap();
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    for (id, note) in [(public_id, public_note), (private_id, private_note)] {
        let response = david.send(get(&format!("/api/v1/threads/{id}/messages"))).await;
        assert_eq!(response.status, StatusCode::OK, "{}", response.text());
        let page: api::MessagePage = parse(&response);
        assert!(page.messages.iter().any(|message| message.id == note && message.system_note));
        let classic = david.get(&format!("/rooms/{DESIGNERS}/threads/{id}/messages.json")).await;
        assert_eq!(classic.status, StatusCode::OK, "{}", classic.text());
        assert!(classic.text().contains(&note.to_string()));
    }
    let public: api::MessagePage = parse(&kevin.send(get(&format!("/api/v1/threads/{public_id}/messages"))).await);
    assert!(public.messages.iter().any(|message| message.id == public_note && message.system_note));
    let private_path = format!("/api/v1/threads/{private_id}/messages");
    for visible in [false, true, false] {
        if visible {
            a.db().write(move |tx| {
                campfire_db::ChannelThread::find(tx.conn(), private_id)?.add_access_member(tx, DAVID, KEVIN)?;
                Ok(())
            }).await.unwrap();
        }
        let response = kevin.send(get(&private_path)).await;
        assert_eq!(response.status, if visible { StatusCode::OK } else { StatusCode::NOT_FOUND }, "{}", response.text());
        let classic = kevin.get(&format!("/rooms/{DESIGNERS}/threads/{private_id}/messages.json")).await;
        assert_eq!(classic.status, if visible { StatusCode::OK } else { StatusCode::NOT_FOUND }, "{}", classic.text());
        if visible {
            let page: api::MessagePage = parse(&response);
            assert!(page.messages.iter().any(|message| message.id == private_note && message.system_note));
            a.db().write(move |tx| campfire_db::ChannelThread::find(tx.conn(), private_id)?
                .remove_access_member(tx, DAVID, KEVIN)).await.unwrap();
        } else {
            assert!(!response.text().contains("Private system note"));
        }
    }
}

#[tokio::test]
async fn private_thread_creation_conflicts_do_not_reveal_hidden_children() {
    let a = app(true).await.expect("the frozen default seed");
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let path = format!("/api/v1/rooms/{DESIGNERS}/threads");
    let created: api::ThreadCreated = parse(&david.write(json_body(Method::POST, &path,
        &json!({"parentMessageId":UNTHREADED,"name":"Hidden child","visibility":"private",
            "message":reply_body("private-conflict-owner", "Private reply")}))).await);
    let id = created.detail.thread.id;
    for (n, visible) in [false, true, false].into_iter().enumerate() {
        if visible {
            a.db().write(move |tx| { campfire_db::ChannelThread::find(tx.conn(), id)?.add_access_member(tx, DAVID, KEVIN)?; Ok(()) }).await.unwrap();
        }
        let response = kevin.write(json_body(Method::POST, &path,
            &json!({"parentMessageId":UNTHREADED,"name":"Fresh child",
                "message":reply_body(&format!("private-conflict-{n}"), "Fresh reply")}))).await;
        assert_eq!(response.status, if visible { StatusCode::CONFLICT } else { StatusCode::NOT_FOUND }, "{}", response.text());
        let classic = kevin.write(json_body(Method::POST, &format!("/rooms/{DESIGNERS}/threads.json"),
            &json!({"thread":{"parent_message_id":UNTHREADED,"name":"Fresh child"},
                "message":{"client_message_id":format!("private-classic-conflict-{n}"),"markdown_source":"Fresh reply"}}))).await;
        assert_eq!(classic.status, if visible { StatusCode::CONFLICT } else { StatusCode::NOT_FOUND }, "{}", classic.text());
        if visible {
            a.db().write(move |tx| campfire_db::ChannelThread::find(tx.conn(), id)?.remove_access_member(tx, DAVID, KEVIN)).await.unwrap();
        } else {
            assert!(!response.text().contains("already exists"));
        }
    }
}
