//! The S2 thread endpoints on `/api/v1` (`campfire_api::threads`). Each answers in the
//! contract's shape and publishes its JSON twin, and the classic frames of every thread broadcast
//! these touch are byte-for-byte the same with the sync engine on.

use std::time::Duration;

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use serde_json::{Value, json};

use super::api_tests::{Sync, app, get, json_body, parse, serve, tag};
use crate::controllers::presenters::test_support::{DAVID, JASON, KEVIN, Req, TestApp};

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
async fn classic_thread_frames(spa: bool) -> Option<Vec<(String, String)>> {
    use crate::controllers::presenters::test_support::SEED_NOW;
    use campfire_kit::clock::FrozenClock;
    let clock = std::sync::Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
    let env: &[(&str, &str)] = if spa { &[("SPA_ENABLED", "1")] } else { &[] };
    let a = TestApp::boot_seed_with_env("default", clock, env).await?;
    // David's classic pages: All Talk (the helper's), Designers, thread 1 and his unread threads.
    let (mut client, cable) =
        crate::controllers::messages::attachment_processing_tests::subscribe(&a).await;
    let identifier =
        crate::channels::tests::support::identifier(json!({ "channel": "UnreadThreadsChannel" }));
    client.confirm(&identifier).await;
    let designers = a
        .db()
        .read(|conn| campfire_db::Room::find(conn, DESIGNERS))
        .await
        .unwrap();
    for gid in [
        campfire_app::cable::room_gid(&designers).to_param(),
        campfire_app::cable::thread_gid(THREAD).to_param(),
    ] {
        let signed =
            rails_compat::turbo::signed_stream_name(&a.booted.app.secrets, &[&gid, "messages"]);
        let identifier = crate::channels::tests::support::identifier(
            json!({ "channel": "RoomMessagesChannel", "signed_stream_name": signed }),
        );
        client.confirm(&identifier).await;
    }
    let mut david = a.sign_in(DAVID).await;
    let mut jason = a.sign_in(JASON).await;
    david.authenticity_token().await;
    jason.authenticity_token().await;
    let (_sync, server) = if spa {
        let (addr, server) = serve(&a).await;
        let topics = [format!("room:{DESIGNERS}"), format!("thread:{THREAD}")];
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
    let threads = format!("/rooms/{DESIGNERS}/threads");

    let reply = david
        .write(classic(
            Method::POST,
            format!("{threads}/{THREAD}/join.json"),
            json!({"involvement": "everything"}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let reply = jason
        .write(classic(
            Method::POST,
            format!("{threads}/{THREAD}/messages.json"),
            json!({"message": {"body": "Reply parity", "client_message_id": "reply-parity"}}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let reply = jason
        .write(classic(
            Method::DELETE,
            format!("{threads}/{THREAD}/messages/{JASONS_REPLY}.json"),
            json!({}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.text());
    let reply = david
        .write(classic(
            Method::POST,
            format!("{threads}/{THREAD}/read.json"),
            json!({}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let reply = david
        .write(classic(
            Method::POST,
            format!("{threads}.json"),
            json!({"parent_message_id": UNTHREADED, "name": "Parity thread",
                "message": {"body": "First parity reply", "client_message_id": "first-parity"}}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let created: Value = serde_json::from_str(&reply.text()).unwrap();
    let id = created["thread"]["id"].as_i64().expect("the thread's id");
    let reply = david
        .write(classic(
            Method::PATCH,
            format!("{threads}/{id}.json"),
            json!({"thread": {"name": "Parity renamed", "status": "closed"}}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let reply = david
        .write(classic(
            Method::DELETE,
            format!("{threads}/{id}.json"),
            json!({}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.text());

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
async fn the_thread_frames_are_the_same_with_the_sync_engine_on() {
    let (Some(off), Some(on)) = (
        classic_thread_frames(false).await,
        classic_thread_frames(true).await,
    ) else {
        return;
    };
    let has = |needle: &str| off.iter().any(|(_, frame)| frame.contains(needle));
    assert!(has("Reply parity"), "the reply: {off:#?}");
    assert!(has("thread_indicator"), "the indicator: {off:#?}");
    assert!(has("refreshOnly"), "the refresh: {off:#?}");
    assert!(
        off.iter()
            .any(|(stream, _)| stream.ends_with("_unread_threads")),
        "the unread ping: {off:#?}"
    );
    // Thread 1's reply and its deletion, then the new thread's creation and deletion.
    let indicators = off
        .iter()
        .filter(|(_, frame)| frame.contains("thread_indicator"))
        .count();
    assert!(indicators >= 4, "the indicators: {off:#?}");
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
