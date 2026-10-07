//! The S2 composer endpoints on `/api/v1` (`campfire_api::composer`): uploads, autocomplete,
//! slash commands, preview and scheduled messages. Each answers in the contract's shape, and the
//! writes publish the classic broadcasts' JSON twins.

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use serde_json::{Value, json};

use super::api_tests::{ALL_PETS, Sync, app, created_in, get, json_body, parse, serve, tag};
use crate::controllers::presenters::test_support::{ALL_TALK, DAVID, JASON, KEVIN, Reply};

/// A closed room of David (admin), Jason, Kevin, JZ, Mallory (banned) and Deploy Bot, with
/// threads 1, 2, 3 and 8.
const DESIGNERS: i64 = 654632876;
const THREAD: i64 = 1;
/// A board thread, in another room.
const BOARD_THREAD: i64 = 4;
const JZ: i64 = 773523953;
const MALLORY: i64 = 773523955;
const DEPLOY_BOT: i64 = 773523956;
/// Jason's reply in thread 1, and a root message of Designers.
const JASONS_REPLY: i64 = 935962046;
const ROOT_MESSAGE: i64 = 935962057;

fn status_and_tag(reply: &Reply) -> (StatusCode, String) {
    (reply.status, tag(reply))
}

#[tokio::test]
async fn uploads_answer_a_signed_id_and_an_upload_url() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let body = json!({
        "filename": "notes.txt",
        "byteSize": 5,
        "checksum": "XUFAKrxLKna5cZ2REBfFkg==",
        "contentType": "text/plain",
    });
    let reply = david
        .write(json_body(Method::POST, "/api/v1/uploads", &body))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let upload: api::DirectUpload = parse(&reply);
    assert!(!upload.signed_id.is_empty());
    assert!(
        upload.upload_url.starts_with("/rails/active_storage/disk/"),
        "{upload:?}"
    );
    let blob = a
        .db()
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT filename, byte_size, checksum, content_type FROM active_storage_blobs ORDER BY id DESC LIMIT 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                    ))
                },
            )?)
        })
        .await
        .unwrap();
    assert_eq!(
        blob,
        (
            "notes.txt".into(),
            5,
            "XUFAKrxLKna5cZ2REBfFkg==".into(),
            Some("text/plain".into())
        )
    );

    for (body, field) in [
        (
            json!({"filename": " ", "byteSize": 5, "checksum": "x", "contentType": "text/plain"}),
            "filename",
        ),
        (
            json!({"filename": "a", "byteSize": 5, "checksum": "", "contentType": "text/plain"}),
            "checksum",
        ),
        (
            json!({"filename": "a", "byteSize": -1, "checksum": "x", "contentType": "text/plain"}),
            "byteSize",
        ),
        (
            json!({"filename": "a", "byteSize": 5, "checksum": "x", "contentType": ""}),
            "contentType",
        ),
    ] {
        let reply = david
            .write(json_body(Method::POST, "/api/v1/uploads", &body))
            .await;
        assert_eq!(
            status_and_tag(&reply),
            (StatusCode::UNPROCESSABLE_ENTITY, "Validation".into())
        );
        let api::ApiError::Validation { fields, .. } = parse::<api::ApiErrorResponse>(&reply).error
        else {
            unreachable!()
        };
        assert!(fields.contains_key(field), "{fields:?}");
    }
    let reply = david
        .write(json_body(
            Method::POST,
            "/api/v1/uploads",
            &json!({"filename": "a"}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn user_suggestions_cover_the_room_or_the_workspace() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;

    let list: api::UserSuggestionList = parse(
        &david
            .send(get(&format!(
                "/api/v1/autocomplete/users?roomId={DESIGNERS}"
            )))
            .await,
    );
    let ids = list
        .suggestions
        .iter()
        .map(|suggestion| suggestion.user.id)
        .collect::<Vec<_>>();
    // Active members only, bots included, by name.
    assert_eq!(ids, [DAVID, DEPLOY_BOT, JASON, JZ, KEVIN], "{list:?}");
    assert!(!ids.contains(&MALLORY));
    for suggestion in &list.suggestions {
        assert_eq!(
            suggestion.mention_token,
            Some(format!("@[{}]", suggestion.user.name))
        );
    }

    let list: api::UserSuggestionList = parse(
        &david
            .send(get(&format!(
                "/api/v1/autocomplete/users?roomId={DESIGNERS}&query=as"
            )))
            .await,
    );
    assert_eq!(
        list.suggestions
            .iter()
            .map(|suggestion| suggestion.user.id)
            .collect::<Vec<_>>(),
        [JASON]
    );

    // Without a room, the whole workspace.
    let list: api::UserSuggestionList = parse(&david.send(get("/api/v1/autocomplete/users")).await);
    assert!(list.suggestions.len() >= ids.len());
    assert!(list.suggestions.len() <= 20);

    // Kevin isn't in All Pets.
    for path in [
        format!("/api/v1/autocomplete/users?roomId={ALL_PETS}"),
        "/api/v1/autocomplete/users?roomId=nope".into(),
    ] {
        let reply = kevin.send(get(&path)).await;
        assert_eq!(
            status_and_tag(&reply),
            (StatusCode::NOT_FOUND, "NotFound".into()),
            "{path}"
        );
    }
}

#[tokio::test]
async fn a_duplicate_name_has_no_mention_token() {
    let Some(a) = app(true).await else { return };
    a.db()
        .write(move |tx| {
            tx.conn()
                .execute("UPDATE users SET name = 'Jason' WHERE id = ?", [KEVIN])?;
            Ok(())
        })
        .await
        .unwrap();
    let mut david = a.sign_in(DAVID).await;
    let list: api::UserSuggestionList = parse(
        &david
            .send(get(&format!(
                "/api/v1/autocomplete/users?roomId={DESIGNERS}&query=jason"
            )))
            .await,
    );
    assert_eq!(list.suggestions.len(), 2, "{list:?}");
    assert!(
        list.suggestions
            .iter()
            .all(|suggestion| suggestion.mention_token.is_none())
    );
}

#[tokio::test]
async fn icon_suggestions_and_the_icon_catalog() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;

    let list: api::IconList = parse(
        &david
            .send(get("/api/v1/autocomplete/icons?query=smile"))
            .await,
    );
    assert!(!list.icons.is_empty() && list.icons.len() <= 8, "{list:?}");
    let smile = list
        .icons
        .iter()
        .find(|icon| icon.name == "smile")
        .expect("the exact match");
    assert_eq!(smile.kind, api::IconKind::Emoji);
    assert_eq!(smile.character.as_deref(), Some("😄"));
    assert_eq!(smile.title, "Smile");
    assert_eq!(list.icons[0].name, "smile", "exact matches first");

    let list: api::IconList = parse(&david.send(get("/api/v1/autocomplete/icons?query=")).await);
    assert!(list.icons.is_empty());

    let catalog: api::IconList = parse(&david.send(get("/api/v1/icons")).await);
    assert!(!catalog.icons.is_empty());
    assert!(
        catalog
            .icons
            .iter()
            .all(|icon| icon.kind != api::IconKind::Emoji && icon.character.is_none())
    );
    // Brands, then custom icons, each by name and once.
    let kinds = catalog
        .icons
        .iter()
        .map(|icon| icon.kind == api::IconKind::Custom)
        .collect::<Vec<_>>();
    assert!(kinds.windows(2).all(|pair| pair[0] <= pair[1]));
    let mut names = catalog
        .icons
        .iter()
        .map(|icon| (icon.kind == api::IconKind::Custom, icon.name.clone()))
        .collect::<Vec<_>>();
    let listed = names.clone();
    names.sort();
    names.dedup();
    assert_eq!(listed, names);
}

#[tokio::test]
async fn slash_commands_list_the_built_ins() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let list: api::SlashCommandList = parse(
        &david
            .send(get(&format!("/api/v1/rooms/{DESIGNERS}/slash_commands")))
            .await,
    );
    let names = |list: &api::SlashCommandList| {
        list.commands
            .iter()
            .map(|command| command.name.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        names(&list)[..10],
        [
            "huddle", "event", "poll", "remind", "status", "dnd", "ooo", "shrug", "me", "play"
        ]
    );
    let remind = &list.commands[3];
    assert_eq!(remind.arg_hint, "<when> <text>");
    assert!(remind.takes_arguments);
    assert_eq!(remind.agent_name, None);

    let in_thread: api::SlashCommandList = parse(
        &david
            .send(get(&format!(
                "/api/v1/rooms/{DESIGNERS}/slash_commands?threadId={THREAD}"
            )))
            .await,
    );
    assert!(!names(&in_thread).contains(&"poll".to_string()));
    assert!(names(&in_thread).contains(&"remind".to_string()));

    for (kevins, path) in [
        (
            false,
            format!("/api/v1/rooms/{DESIGNERS}/slash_commands?threadId={BOARD_THREAD}"),
        ),
        (
            false,
            format!("/api/v1/rooms/{DESIGNERS}/slash_commands?threadId=nope"),
        ),
        (true, format!("/api/v1/rooms/{ALL_PETS}/slash_commands")),
    ] {
        let who = if kevins { &mut kevin } else { &mut david };
        let reply = who.send(get(&path)).await;
        assert_eq!(
            status_and_tag(&reply),
            (StatusCode::NOT_FOUND, "NotFound".into()),
            "{path}"
        );
    }
}

#[tokio::test]
async fn running_slash_commands_answers_each_outcome() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let jason = a.sign_in(JASON).await;
    let mut sync =
        Sync::connect(addr, &jason.cookie_header(), &[format!("room:{DESIGNERS}")]).await;
    sync.welcome().await;
    let path = format!("/api/v1/rooms/{DESIGNERS}/slash_commands");
    let mut run = async |text: &str, thread_id: Option<i64>| {
        let reply = david
            .write(json_body(
                Method::POST,
                &path,
                &json!({"text": text, "threadId": thread_id}),
            ))
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{text}: {}", reply.text());
        parse::<api::SlashCommandResult>(&reply)
    };

    let api::SlashCommandResult::Posted { message_id, notice } = run("/me waves", None).await
    else {
        panic!("/me posts");
    };
    assert_eq!(notice, None);
    let event = sync.until(created_in(DESIGNERS), |_| false).await;
    let api::SyncPayload::MessageCreated(message) = &event.payload else {
        unreachable!()
    };
    assert_eq!(message.id, message_id);
    assert_eq!(message.creator_id, DAVID);
    // It's published once: the next post's event comes with no second copy before it.
    let first = message_id;
    let api::SlashCommandResult::Posted { message_id, .. } = run("/me waves again", None).await
    else {
        panic!("/me posts");
    };
    sync.until(
        |event| matches!(&event.payload, api::SyncPayload::MessageCreated(message) if message.id == message_id),
        |event| matches!(&event.payload, api::SyncPayload::MessageCreated(message) if message.id == first),
    )
    .await;

    let api::SlashCommandResult::Posted { message_id, .. } = run("/shrug", Some(THREAD)).await
    else {
        panic!("/shrug posts");
    };
    let thread_id = a
        .db()
        .read(move |conn| Ok(campfire_db::Message::find(conn, message_id)?.thread_id))
        .await
        .unwrap();
    assert_eq!(thread_id, Some(THREAD));

    assert_eq!(run("/poll", None).await, api::SlashCommandResult::OpenPoll);
    assert!(matches!(
        run("/dnd", None).await,
        api::SlashCommandResult::Ephemeral { .. }
    ));
    let api::SlashCommandResult::Error { message } = run("/nope", None).await else {
        panic!("an unknown command fails");
    };
    assert!(message.contains("/remind"), "{message}");
    assert!(matches!(
        run("/event Launch party tomorrow at 3pm", None).await,
        api::SlashCommandResult::OpenUrl { .. }
    ));

    // A thread of another room.
    let reply = david
        .write(json_body(
            Method::POST,
            &path,
            &json!({"text": "/me waves", "threadId": BOARD_THREAD}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    server.abort();
}

#[tokio::test]
async fn previews_render_without_posting() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let path = format!("/api/v1/rooms/{DESIGNERS}/messages/preview");
    let count = async || {
        a.db()
            .read(|conn| {
                Ok(conn.query_row("SELECT COUNT(*) FROM messages", [], |row| {
                    row.get::<_, i64>(0)
                })?)
            })
            .await
            .unwrap()
    };
    let before = count().await;
    let reply = david
        .write(json_body(
            Method::POST,
            &path,
            &json!({"markdownSource": "**Bold** and @[Jason]"}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let preview: api::MessagePreview = parse(&reply);
    assert!(
        preview.body_html.contains("<strong>Bold</strong>"),
        "{preview:?}"
    );
    assert!(
        preview.body_html.contains(r#"<span class="mention"#),
        "{preview:?}"
    );
    assert!(
        !preview.body_html.contains(r#"<div class="mention"#),
        "{preview:?}"
    );
    assert_eq!(count().await, before);

    let long = "a".repeat(50_001);
    let reply = david
        .write(json_body(
            Method::POST,
            &path,
            &json!({"markdownSource": long}),
        ))
        .await;
    assert_eq!(
        status_and_tag(&reply),
        (StatusCode::UNPROCESSABLE_ENTITY, "Validation".into())
    );
    let reply = kevin
        .write(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{ALL_PETS}/messages/preview"),
            &json!({"markdownSource": "hi"}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
}

fn schedule_body(
    source: &str,
    send_at: &str,
    thread_id: Option<i64>,
    reply_to: Option<i64>,
) -> Value {
    json!({
        "markdownSource": source,
        "sendAt": send_at,
        "threadId": thread_id,
        "replyToMessageId": reply_to,
    })
}

const LATER: &str = "2030-01-01T09:00:00Z";

fn validation_fields(reply: &Reply) -> Vec<String> {
    assert_eq!(
        status_and_tag(reply),
        (StatusCode::UNPROCESSABLE_ENTITY, "Validation".into()),
        "{}",
        reply.text()
    );
    let api::ApiError::Validation { fields, .. } = parse::<api::ApiErrorResponse>(reply).error
    else {
        unreachable!()
    };
    fields.into_keys().collect()
}

#[tokio::test]
async fn scheduled_messages_are_listed_changed_and_cancelled() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let path = format!("/api/v1/rooms/{DESIGNERS}/scheduled_messages");

    let reply = david
        .write(json_body(
            Method::POST,
            &path,
            &schedule_body("Later, team", LATER, Some(THREAD), Some(JASONS_REPLY)),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let row: api::ScheduledMessage = parse(&reply);
    assert_eq!(
        (row.room_id, row.thread_id, row.reply_to_message_id),
        (DESIGNERS, Some(THREAD), Some(JASONS_REPLY))
    );
    assert_eq!(row.markdown_source, "Later, team");
    assert_eq!(row.send_at, "2030-01-01T09:00:00.000Z");
    assert_eq!(
        (row.sent_at.as_deref(), row.dropped_at.as_deref()),
        (None, None)
    );
    assert_eq!(
        (
            row.state,
            row.sendable,
            row.sent_message_id,
            row.drop_reason.as_deref()
        ),
        (api::ScheduledMessageState::Pending, true, None, None)
    );

    let list: api::ScheduledMessageList = parse(
        &david
            .send(get(&format!(
                "/api/v1/scheduled_messages?roomId={DESIGNERS}"
            )))
            .await,
    );
    // The seed's own pending one is due sooner.
    assert_eq!(list.scheduled_messages.last(), Some(&row));
    assert_eq!(list.next_cursor, None);
    let named = list
        .conversations
        .iter()
        .find(|name| name.thread_id == Some(THREAD))
        .expect("the thread's name");
    assert_eq!(
        (named.room_id, named.room_kind, named.room_name.as_str()),
        (DESIGNERS, api::RoomKind::Closed, "Designers")
    );
    assert_eq!(named.thread_name.as_deref(), Some("Launch review"));
    assert!(
        list.scheduled_messages
            .iter()
            .all(|row| list
                .conversations
                .iter()
                .any(|name| (name.room_id, name.thread_id) == (row.room_id, row.thread_id)))
    );
    assert!(
        list.scheduled_messages
            .windows(2)
            .all(|pair| pair[0].send_at <= pair[1].send_at)
    );
    assert!(
        list.scheduled_messages
            .iter()
            .all(|row| row.room_id == DESIGNERS && row.sent_at.is_none())
    );
    let list: api::ScheduledMessageList = parse(
        &david
            .send(get(&format!(
                "/api/v1/scheduled_messages?roomId={ALL_TALK}"
            )))
            .await,
    );
    assert!(list.scheduled_messages.is_empty());
    let list: api::ScheduledMessageList =
        parse(&david.send(get("/api/v1/scheduled_messages")).await);
    assert!(list.scheduled_messages.contains(&row));
    let list: api::ScheduledMessageList =
        parse(&kevin.send(get("/api/v1/scheduled_messages")).await);
    assert!(!list.scheduled_messages.contains(&row));

    // The model's rules, as 422s naming the field.
    for (body, field) in [
        (schedule_body(" ", LATER, None, None), "markdownSource"),
        (
            schedule_body("Hi", "2020-01-01T00:00:00Z", None, None),
            "sendAt",
        ),
        (schedule_body("Hi", "next tuesday", None, None), "sendAt"),
        (
            schedule_body("Hi", LATER, None, Some(JASONS_REPLY)),
            "replyToMessage",
        ),
        (
            schedule_body("Hi", LATER, None, Some(999_999)),
            "replyToMessageId",
        ),
        (
            schedule_body(&"a".repeat(50_001), LATER, None, None),
            "markdownSource",
        ),
    ] {
        let reply = david.write(json_body(Method::POST, &path, &body)).await;
        assert!(
            validation_fields(&reply).contains(&field.to_string()),
            "{body}: {}",
            reply.text()
        );
    }
    // A thread that isn't this room's, or doesn't exist, is a 404.
    for thread in [BOARD_THREAD, 999_999] {
        let reply = david
            .write(json_body(
                Method::POST,
                &path,
                &schedule_body("Hi", LATER, Some(thread), None),
            ))
            .await;
        assert_eq!(
            status_and_tag(&reply),
            (StatusCode::NOT_FOUND, "NotFound".into()),
            "{thread}: {}",
            reply.text()
        );
    }
    let reply = kevin
        .write(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{ALL_PETS}/scheduled_messages"),
            &schedule_body("Hi", LATER, None, None),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);

    let one = format!("/api/v1/scheduled_messages/{}", row.id);
    let change = json!({"markdownSource": "Later, everyone", "sendAt": "2031-02-03T04:05:06Z"});
    let reply = david.write(json_body(Method::PATCH, &one, &change)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let changed: api::ScheduledMessage = parse(&reply);
    assert_eq!(changed.markdown_source, "Later, everyone");
    assert_eq!(changed.send_at, "2031-02-03T04:05:06.000Z");
    // A field left out keeps its value.
    let reply = david
        .write(json_body(
            Method::PATCH,
            &one,
            &json!({"sendAt": "2031-03-04T05:06:07Z"}),
        ))
        .await;
    let changed: api::ScheduledMessage = parse(&reply);
    assert_eq!(
        (changed.markdown_source.as_str(), changed.send_at.as_str()),
        ("Later, everyone", "2031-03-04T05:06:07.000Z")
    );
    let reply = david
        .write(json_body(
            Method::PATCH,
            &one,
            &json!({"markdownSource": "Later, all"}),
        ))
        .await;
    let changed: api::ScheduledMessage = parse(&reply);
    assert_eq!(
        (changed.markdown_source.as_str(), changed.send_at.as_str()),
        ("Later, all", "2031-03-04T05:06:07.000Z")
    );
    let reply = david
        .write(json_body(
            Method::PATCH,
            &one,
            &json!({"markdownSource": "", "sendAt": LATER}),
        ))
        .await;
    assert_eq!(validation_fields(&reply), ["markdownSource"]);

    // Someone else's is a 404, for every action.
    for (method, path) in [
        (Method::PATCH, one.clone()),
        (Method::DELETE, one.clone()),
        (Method::POST, format!("{one}/send_now")),
    ] {
        let reply = kevin.write(json_body(method, &path, &change)).await;
        assert_eq!(
            status_and_tag(&reply),
            (StatusCode::NOT_FOUND, "NotFound".into()),
            "{path}"
        );
    }

    // One the scheduler is sending is a 409.
    let id = row.id;
    let now = a.booted.app.db.env().now();
    a.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE scheduled_messages SET claimed_at = ? WHERE id = ?",
                rusqlite::params![now, id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let list: api::ScheduledMessageList =
        parse(&david.send(get("/api/v1/scheduled_messages")).await);
    let listed = list
        .scheduled_messages
        .iter()
        .find(|row| row.id == id)
        .unwrap();
    assert_eq!(listed.state, api::ScheduledMessageState::Sending);
    for (method, path) in [(Method::PATCH, one.clone()), (Method::DELETE, one.clone())] {
        let reply = david.write(json_body(method, &path, &change)).await;
        assert_eq!(
            status_and_tag(&reply),
            (StatusCode::CONFLICT, "Conflict".into()),
            "{path}"
        );
    }
    a.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE scheduled_messages SET claimed_at = NULL WHERE id = ?",
                [id],
            )?;
            Ok(())
        })
        .await
        .unwrap();

    let reply = david
        .write(json_body(Method::DELETE, &one, &json!({})))
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.text());
    let reply = david
        .write(json_body(Method::DELETE, &one, &json!({})))
        .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    let list: api::ScheduledMessageList =
        parse(&david.send(get("/api/v1/scheduled_messages")).await);
    assert!(list.scheduled_messages.iter().all(|row| row.id != id));
}

#[tokio::test]
async fn sending_a_scheduled_message_now_posts_and_publishes_it() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let jason = a.sign_in(JASON).await;
    let mut sync =
        Sync::connect(addr, &jason.cookie_header(), &[format!("room:{DESIGNERS}")]).await;
    sync.welcome().await;
    let path = format!("/api/v1/rooms/{DESIGNERS}/scheduled_messages");
    let reply = david
        .write(json_body(
            Method::POST,
            &path,
            &schedule_body("Sent early", LATER, None, Some(ROOT_MESSAGE)),
        ))
        .await;
    let row: api::ScheduledMessage = parse(&reply);

    let send_now = format!("/api/v1/scheduled_messages/{}/send_now", row.id);
    let reply = david
        .write(json_body(Method::POST, &send_now, &json!({})))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let sent: api::ScheduledMessage = parse(&reply);
    assert!(sent.sent_at.is_some(), "{sent:?}");
    assert_eq!(
        (sent.state, sent.sendable),
        (api::ScheduledMessageState::Sent, false)
    );
    assert!(sent.sent_message_id.is_some(), "{sent:?}");
    let event = sync.until(created_in(DESIGNERS), |_| false).await;
    let api::SyncPayload::MessageCreated(message) = &event.payload else {
        unreachable!()
    };
    assert_eq!(message.creator_id, DAVID);
    assert_eq!(message.reply_to_message_id, Some(ROOT_MESSAGE));
    assert!(message.body_html.contains("Sent early"), "{message:?}");

    assert_eq!(sent.sent_message_id, Some(message.id));
    // It's in the past list now, most recent first.
    let past: api::ScheduledMessageList = parse(
        &david
            .send(get("/api/v1/scheduled_messages?status=past"))
            .await,
    );
    assert_eq!(past.scheduled_messages.first(), Some(&sent));
    assert!(
        past.scheduled_messages
            .iter()
            .all(|row| row.state == api::ScheduledMessageState::Sent
                || row.state == api::ScheduledMessageState::Dropped)
    );
    let pending: api::ScheduledMessageList = parse(
        &david
            .send(get("/api/v1/scheduled_messages?status=whatever"))
            .await,
    );
    assert!(
        pending
            .scheduled_messages
            .iter()
            .all(|row| row.state == api::ScheduledMessageState::Pending)
    );

    // It's no longer pending.
    let reply = david
        .write(json_body(Method::POST, &send_now, &json!({})))
        .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);

    // One whose room the sender has left is dropped: a 422 saying why.
    let reply = david
        .write(json_body(
            Method::POST,
            &path,
            &schedule_body("Too late", LATER, None, None),
        ))
        .await;
    let row: api::ScheduledMessage = parse(&reply);
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
        status_and_tag(&reply),
        (StatusCode::UNPROCESSABLE_ENTITY, "Validation".into()),
        "{}",
        reply.text()
    );
    // It's dropped, with no reason for lost access; and once the sent message is deleted, the
    // sent one no longer names it.
    let message_id = message.id;
    a.db()
        .write(move |tx| {
            tx.conn()
                .execute("DELETE FROM messages WHERE id = ?", [message_id])?;
            Ok(())
        })
        .await
        .unwrap();
    let past: api::ScheduledMessageList = parse(
        &david
            .send(get("/api/v1/scheduled_messages?status=past"))
            .await,
    );
    let dropped = past
        .scheduled_messages
        .iter()
        .find(|one| one.id == row.id)
        .unwrap();
    assert_eq!(
        (
            dropped.state,
            dropped.sendable,
            dropped.drop_reason.as_deref()
        ),
        (api::ScheduledMessageState::Dropped, false, None)
    );
    let sent = past
        .scheduled_messages
        .iter()
        .find(|one| one.id == sent.id)
        .unwrap();
    assert_eq!(sent.sent_message_id, None);
    server.abort();
}

#[tokio::test]
async fn scheduled_messages_page_by_cursor() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let now = a.booted.app.db.env().now();
    // 120 pending ones in Designers, some sharing a time, and 3 past ones.
    a.db()
        .write(move |tx| {
            for n in 0..123_i64 {
                let send_at = campfire_db::Timestamp::from_second(1_900_000_000 + n / 3);
                let sent_at = (n >= 120).then_some(now);
                tx.conn().execute(
                    "INSERT INTO scheduled_messages (user_id, room_id, markdown_source, send_at, sent_at, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
                    rusqlite::params![DAVID, DESIGNERS, format!("Paged {n}"), send_at, sent_at, now, now],
                )?;
            }
            Ok(())
        })
        .await
        .unwrap();
    for status in ["pending", "past"] {
        let mut seen = Vec::new();
        let mut cursor: Option<String> = None;
        let mut pages = 0;
        loop {
            let path = match &cursor {
                Some(before) => format!(
                    "/api/v1/scheduled_messages?status={status}&roomId={DESIGNERS}&before={before}"
                ),
                None => format!("/api/v1/scheduled_messages?status={status}&roomId={DESIGNERS}"),
            };
            let reply = david.send(get(&path)).await;
            assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
            let page: api::ScheduledMessageList = parse(&reply);
            assert!(page.scheduled_messages.len() <= 50);
            assert!(page.next_cursor.is_none() || page.scheduled_messages.len() == 50);
            seen.extend(page.scheduled_messages);
            pages += 1;
            match page.next_cursor {
                Some(next) => cursor = Some(next),
                None => break,
            }
        }
        let keys = seen
            .iter()
            .map(|row| (row.send_at.clone(), row.id))
            .collect::<Vec<_>>();
        let mut sorted = keys.clone();
        sorted.sort();
        if status == "past" {
            sorted.reverse();
        }
        sorted.dedup();
        assert_eq!(keys, sorted, "{status}: in order, each once");
        let full: api::ScheduledMessageList = parse(
            &david
                .send(get(&format!(
                    "/api/v1/scheduled_messages?status={status}&roomId={DESIGNERS}"
                )))
                .await,
        );
        if status == "pending" {
            // The seed's one plus the 120.
            assert_eq!((seen.len(), pages), (121, 3));
            assert_eq!(full.scheduled_messages[..], seen[..50]);
        } else {
            assert_eq!((seen.len(), pages), (4, 1));
        }
    }
    let reply = david
        .send(get("/api/v1/scheduled_messages?before=not-a-cursor"))
        .await;
    assert_eq!(validation_fields(&reply), ["before"]);
}
