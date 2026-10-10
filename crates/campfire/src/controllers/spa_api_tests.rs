//! `/api/v1` and the `/api/v1/sync` socket (`campfire_api`) over the seeded app with
//! `SPA_ENABLED`: the contract's shapes, the envelope for every failure, idempotent posting, and
//! the JSON twins arriving alongside (never instead of) the classic broadcasts.

use std::net::SocketAddr;
use std::time::Duration;

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use futures_util::{SinkExt as _, StreamExt as _};
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::tungstenite::client::IntoClientRequest as _;

use crate::controllers::presenters::test_support::{
    ALL_TALK, Browser, DAVID, HQ, JASON, KEVIN, Reply, Req, TestApp, seed_clock,
};

/// Rooms in the seed: HQ holds David and Kevin (and no messages); All Talk holds David and 131
/// messages; All Pets holds David but not Kevin.
pub(super) const ALL_PETS: i64 = 104393281;

pub(super) async fn app(enabled: bool) -> Option<TestApp> {
    let env: &[(&str, &str)] = if enabled {
        &[("SPA_ENABLED", "1")]
    } else {
        &[]
    };
    TestApp::boot_seed_with_env("default", seed_clock(), env).await
}

pub(super) fn get(path: &str) -> Req {
    Req::new(Method::GET, path).header("accept", "application/json")
}

pub(super) fn json_body(method: Method, path: &str, body: &Value) -> Req {
    Req::new(method, path)
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .body(body.to_string())
}

pub(super) fn parse<T: serde::de::DeserializeOwned>(reply: &Reply) -> T {
    serde_json::from_slice(&reply.body).unwrap_or_else(|error| panic!("{error}: {}", reply.text()))
}

pub(super) fn tag(reply: &Reply) -> String {
    let envelope: api::ApiErrorResponse = parse(reply);
    let value = serde_json::to_value(&envelope.error).unwrap();
    value["_tag"].as_str().unwrap().to_string()
}

async fn post(b: &mut Browser<'_>, room_id: i64, client_message_id: &str, source: &str) -> Reply {
    let body = json!({"clientMessageId": client_message_id, "markdownSource": source, "replyToMessageId": null, "replyNotifyAuthor": null});
    b.write(json_body(
        Method::POST,
        &format!("/api/v1/rooms/{room_id}/messages"),
        &body,
    ))
    .await
}

#[tokio::test]
async fn play_chat_sound_messages_match_the_classic_catalog() {
    let Some(a) = app(true).await else { return };
    let mut b = a.sign_in(DAVID).await;
    for (name, presentation) in [
        (
            "bell",
            api::SoundPresentation::Text {
                text: "🔔".into()
            },
        ),
        (
            "56k",
            api::SoundPresentation::Image {
                url: campfire_assets::image_path("sounds/56k.webp"),
                width: 79,
                height: 33,
            },
        ),
    ] {
        let reply = b
            .write(json_body(
                Method::POST,
                &format!("/api/v1/rooms/{HQ}/slash_commands"),
                &json!({"text": format!("/play {name}"), "threadId": null}),
            ))
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        let result: api::SlashCommandResult = parse(&reply);
        let api::SlashCommandResult::Posted { message_id: id, .. } = result else {
            panic!("/play did not post: {result:?}");
        };
        let read: api::MessageRead = parse(&b.send(get(&format!("/api/v1/messages/{id}"))).await);
        assert_eq!(read.message.markdown_source, Some(format!("/play {name}")));
        assert_eq!(
            read.message.sound,
            Some(api::MessageSound {
                name: name.into(),
                url: campfire_assets::asset_path(&format!("{name}.mp3")),
                presentation,
            })
        );
        let classic = b
            .send(Req::new(Method::GET, &format!("/rooms/{HQ}?classic=1")))
            .await;
        assert!(classic.text().contains(&format!(
            "data-sound-url-value=\"{}\"",
            read.message.sound.unwrap().url
        )));
    }
    let unknown: api::MessageDTO =
        parse(&post(&mut b, HQ, "unknown-play-sound", "/play unknown").await);
    assert_eq!(unknown.sound, None);
}

#[tokio::test]
async fn play_chat_sound_quiet_policy_is_the_classic_layout_policy() {
    let Some(a) = app(true).await else { return };
    a.db().write(|tx| {
        tx.conn().execute("UPDATE users SET presence_setting='dnd',quiet_hours_enabled=1,quiet_hours_start_minute=1320,quiet_hours_end_minute=420,time_zone='America/New_York',meeting_status_enabled=1,meeting_dnd_enabled=1,ooo_calendar_enabled=1,ooo_notify_enabled=0 WHERE id=?", [DAVID])?;
        tx.conn().execute("INSERT OR REPLACE INTO calendar_meeting_caches(user_id,busy_intervals,ooo_intervals,created_at,updated_at) VALUES(?,'[[\"2026-10-09T12:00:00Z\",\"2026-10-09T13:00:00Z\"]]','[[\"2026-10-09T14:00:00Z\",\"2026-10-09T15:00:00Z\"]]','2026-10-09 11:00:00','2026-10-09 11:00:00')", [DAVID])?;
        Ok(())
    }).await.unwrap();
    let mut b = a.sign_in(DAVID).await;
    let me: api::Me = parse(&b.send(get("/api/v1/me")).await);
    assert_eq!(
        me.chat_sounds,
        api::ChatSounds {
            muted: true,
            quiet_hours: Some(api::QuietHours {
                start_minute: 1320,
                end_minute: 420
            }),
            time_zone: "America/New_York".into(),
            quiet_windows: vec![(1791547200, 1791550800), (1791554400, 1791558000)],
        }
    );
}

#[tokio::test]
async fn the_api_exists_only_with_the_spa() {
    let Some(a) = app(false).await else { return };
    let mut b = a.sign_in(DAVID).await;
    let unknown = b.send(get("/no-such-page")).await.status;
    for path in [
        "/api/v1/me",
        "/api/v1/sidebar",
        &format!("/api/v1/rooms/{HQ}"),
        "/api/v1/sync",
    ] {
        assert_eq!(b.send(get(path)).await.status, unknown, "{path}");
    }
    assert!(!a.booted.app.cable.sync_enabled());
}

#[tokio::test]
async fn reads_answer_in_the_contracts_shapes() {
    let Some(a) = app(true).await else { return };
    let mut b = a.sign_in(DAVID).await;

    let reply = b.send(get("/api/v1/me")).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert_eq!(reply.header("cache-control"), Some("no-store"));
    let me: api::Me = parse(&reply);
    assert_eq!(me.user.id, DAVID);
    assert!(me.last_room_id.is_some());

    let sidebar: api::Sidebar = parse(&b.send(get("/api/v1/sidebar")).await);
    let hq = sidebar
        .rows
        .iter()
        .find(|row| row.room.id == HQ)
        .expect("HQ in David's sidebar");
    assert_eq!(hq.display_name, "HQ");
    assert!(hq.last_message.is_none());
    assert!(sidebar.can_create_rooms);
    assert!(
        sidebar.rows.iter().any(|row| row
            .last_message
            .as_ref()
            .is_some_and(|last| !last.excerpt.is_empty())),
        "a direct row previews its newest message"
    );
    for row in sidebar
        .rows
        .iter()
        .filter(|row| row.room.kind == api::RoomKind::Direct)
    {
        assert!(row.room.name.is_none());
        assert!(!row.direct_member_ids.is_empty());
        for id in &row.direct_member_ids {
            assert!(
                sidebar.users.iter().any(|user| user.id == *id),
                "user {id} of {}",
                row.display_name
            );
        }
    }

    let detail: api::RoomDetail = parse(&b.send(get(&format!("/api/v1/rooms/{HQ}"))).await);
    assert_eq!(
        (
            detail.room.id,
            detail.membership.user_id,
            detail.display_name.as_str()
        ),
        (HQ, DAVID, "HQ")
    );
    assert!(detail.member_preview_ids.len() <= 5 && detail.member_preview_ids.contains(&DAVID));

    let newest: api::MessagePage = parse(
        &b.send(get(&format!("/api/v1/rooms/{ALL_TALK}/messages")))
            .await,
    );
    assert_eq!(newest.messages.len(), 40);
    assert!(
        newest
            .messages
            .windows(2)
            .all(|pair| pair[0].id != pair[1].id)
    );
    assert_eq!(newest.after, None, "the newest page reaches the present");
    for message in &newest.messages {
        assert_eq!((message.room_id, message.thread_id), (ALL_TALK, None));
        assert!(
            newest
                .users
                .iter()
                .any(|user| user.id == message.creator_id)
        );
    }
    let before = newest.before.expect("All Talk has more than a page");
    assert_eq!(before, newest.messages[0].id);
    let older: api::MessagePage = parse(
        &b.send(get(&format!(
            "/api/v1/rooms/{ALL_TALK}/messages?before={before}"
        )))
        .await,
    );
    assert_eq!(older.messages.len(), 40);
    assert!(older.messages.iter().all(|message| message.id != before));
    assert_eq!(
        older.after,
        older.messages.last().map(|message| message.id),
        "an older page has newer ones after it"
    );
    let newer: api::MessagePage = parse(
        &b.send(get(&format!(
            "/api/v1/rooms/{ALL_TALK}/messages?after={}",
            older.messages[39].id
        )))
        .await,
    );
    assert_eq!(
        newer.messages.first().map(|message| message.id),
        Some(before)
    );
    let anchor = older.messages[20].id;
    let around: api::MessagePage = parse(
        &b.send(get(&format!(
            "/api/v1/rooms/{ALL_TALK}/messages?around={anchor}"
        )))
        .await,
    );
    let at = around
        .messages
        .iter()
        .position(|message| message.id == anchor)
        .expect("the anchor");
    assert_eq!(
        (at, around.messages.len() - at - 1),
        (40, 40),
        "40 on each side of the anchor"
    );
    let both = b
        .send(get(&format!(
            "/api/v1/rooms/{ALL_TALK}/messages?before={before}&after={before}"
        )))
        .await;
    assert_eq!(
        (both.status, tag(&both)),
        (StatusCode::UNPROCESSABLE_ENTITY, "Validation".into())
    );
    let unknown = b
        .send(get(&format!("/api/v1/rooms/{ALL_TALK}/messages?around=1")))
        .await;
    assert_eq!(
        (unknown.status, tag(&unknown)),
        (StatusCode::NOT_FOUND, "NotFound".into())
    );

    let users: api::UserList = parse(
        &b.send(get(&format!(
            "/api/v1/users?ids={KEVIN},{DAVID},999,nope,{DAVID}"
        )))
        .await,
    );
    assert_eq!(
        users.users.iter().map(|user| user.id).collect::<Vec<_>>(),
        [DAVID, KEVIN]
    );
    let presence: api::PresenceList = parse(
        &b.send(get(&format!("/api/v1/presence?ids={KEVIN},{DAVID}")))
            .await,
    );
    assert_eq!(
        presence
            .presences
            .iter()
            .map(|row| row.user_id)
            .collect::<Vec<_>>(),
        [DAVID, KEVIN]
    );

    let mut kevin = a.sign_in(KEVIN).await;
    let outside = kevin.send(get(&format!("/api/v1/rooms/{ALL_PETS}"))).await;
    assert_eq!(
        (outside.status, tag(&outside)),
        (StatusCode::NOT_FOUND, "NotFound".into())
    );
    let outside = kevin
        .send(get(&format!("/api/v1/rooms/{ALL_PETS}/messages")))
        .await;
    assert_eq!(outside.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn signed_out_and_forged_requests_get_the_envelope() {
    let Some(a) = app(true).await else { return };
    let mut anonymous = a.anonymous();
    // No Accept header: the API is JSON whatever the client asks for.
    for path in [
        "/api/v1/me",
        "/api/v1/sidebar",
        &format!("/api/v1/rooms/{HQ}/messages"),
    ] {
        let reply = anonymous
            .send(Req::new(Method::GET, path).header("accept", "*/*"))
            .await;
        assert_eq!(
            (reply.status, tag(&reply)),
            (StatusCode::UNAUTHORIZED, "Unauthorized".into()),
            "{path}"
        );
        assert_eq!(
            reply.content_type(),
            Some("application/json; charset=utf-8")
        );
    }

    let mut b = a.sign_in(DAVID).await;
    let body = json!({"clientMessageId": "forged", "markdownSource": "hi", "replyToMessageId": null, "replyNotifyAuthor": null});
    let forged = b
        .send(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{HQ}/messages"),
            &body,
        ))
        .await;
    assert_eq!(
        (forged.status, tag(&forged)),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            "InvalidAuthenticityToken".into()
        )
    );
    let forged = b
        .send(
            Req::new(Method::POST, &format!("/api/v1/rooms/{HQ}/read"))
                .header("x-csrf-token", "nope"),
        )
        .await;
    assert_eq!(tag(&forged), "InvalidAuthenticityToken");
    let created = a
        .db()
        .read(|conn| campfire_db::Message::find_duplicate(conn, HQ, DAVID, "forged"))
        .await
        .unwrap();
    assert!(created.is_none());
}

#[tokio::test]
async fn posting_is_idempotent_validated_and_broadcast_the_classic_way() {
    let Some(a) = app(true).await else { return };
    // A classic page following All Talk, so its stream's publications are recorded.
    let (_client, cable) =
        crate::controllers::messages::attachment_processing_tests::subscribe(&a).await;
    let mut b = a.sign_in(DAVID).await;
    b.authenticity_token().await;
    let capture = a.publications();
    capture.take();

    let first = post(&mut b, ALL_TALK, "0199b3c4-api-1", "Hello **there**").await;
    assert_eq!(first.status, StatusCode::CREATED, "{}", first.text());
    let created: api::MessageDTO = parse(&first);
    assert_eq!(
        (
            created.room_id,
            created.creator_id,
            created.client_message_id.as_str()
        ),
        (ALL_TALK, DAVID, "0199b3c4-api-1")
    );
    assert!(
        created.body_html.contains("<strong>there</strong>"),
        "{}",
        created.body_html
    );
    assert_eq!(created.markdown_source.as_deref(), Some("Hello **there**"));

    // The classic pages still get the Turbo append of the rendered message.
    let frames = capture.take();
    assert!(
        frames.iter().any(|(stream, frame)| {
            let html = serde_json::from_str::<String>(frame).unwrap_or_default();
            stream.ends_with(":messages")
                && html.starts_with(r#"<turbo-stream action="append""#)
                && html.contains(&format!(r#"data-message-id="{}""#, created.id))
        }),
        "{frames:?}"
    );

    let again = post(&mut b, ALL_TALK, "0199b3c4-api-1", "Hello **there**").await;
    assert_eq!(again.status, StatusCode::OK);
    // The same message; only `cardsAsOf`, the read time, moves on.
    let again = parse::<api::MessageDTO>(&again);
    assert!(again.cards_as_of >= created.cards_as_of);
    assert_eq!(
        api::MessageDTO {
            cards_as_of: created.cards_as_of.clone(),
            ..again
        },
        created
    );
    let count = a
        .db()
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM messages WHERE client_message_id = '0199b3c4-api-1'",
                [],
                |row| row.get::<_, i64>(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(count, 1);

    let blank = post(&mut b, ALL_TALK, "  ", "hi").await;
    assert_eq!(
        (blank.status, tag(&blank)),
        (StatusCode::UNPROCESSABLE_ENTITY, "Validation".into())
    );
    let envelope: api::ApiErrorResponse = parse(&blank);
    let api::ApiError::Validation { fields, .. } = envelope.error else {
        panic!()
    };
    assert!(fields.contains_key("clientMessageId"));
    let malformed = b
        .write(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{ALL_TALK}/messages"),
            &json!({"markdownSource": "hi"}),
        ))
        .await;
    assert_eq!(tag(&malformed), "Validation");
    let elsewhere = b.write(json_body(Method::POST, &format!("/api/v1/rooms/{ALL_TALK}/messages"), &json!({
        "clientMessageId": "0199b3c4-api-2", "markdownSource": "hi", "replyToMessageId": 1, "replyNotifyAuthor": null,
    }))).await;
    assert_eq!(tag(&elsewhere), "Validation");
    cable.abort();
}

/// A message in All Talk with reactions: Jason's 💯.
pub(super) const BOOSTED: i64 = 136976342;

#[tokio::test]
async fn messages_carry_reactions_boosts_pins_saves_threads_and_forwards() {
    let Some(a) = app(true).await else { return };
    let mut b = a.sign_in(DAVID).await;
    let page: api::MessagePage = parse(
        &b.send(get(&format!(
            "/api/v1/rooms/{ALL_TALK}/messages?around={BOOSTED}"
        )))
        .await,
    );
    let at = page
        .messages
        .iter()
        .position(|message| message.id == BOOSTED)
        .unwrap();
    let (next, other) = (page.messages[at - 1].id, page.messages[at - 2].id);
    a.db()
        .write(move |tx| {
            // After the seed's own boosts, so they come first.
            let now = "2026-03-03 00:00:00.000000";
            let sql = format!(
                "INSERT INTO boosts (booster_id, content, message_id, created_at, updated_at) VALUES ({KEVIN}, 'nice work', {BOOSTED}, '{now}', '{now}'), ({DAVID}, '💯', {BOOSTED}, '{now}', '{now}'), ({JASON}, '💯', {BOOSTED}, '{now}', '{now}');
                 INSERT INTO message_pins (message_id, pinner_id, room_id, created_at, updated_at) VALUES ({next}, {DAVID}, {ALL_TALK}, '{now}', '{now}');
                 INSERT INTO saved_items (message_id, user_id, created_at, updated_at) VALUES ({next}, {DAVID}, '{now}', '{now}'), ({other}, {KEVIN}, '{now}', '{now}');
                 UPDATE messages SET forwarded_at = '{now}', forward_note = 'FYI' WHERE id = {other};
                 INSERT INTO channel_threads (creator_id, last_activity_at, messages_count, name, parent_message_id, room_id, created_at, updated_at) VALUES ({JASON}, '{now}', 3, 'Side', {next}, {ALL_TALK}, '{now}', '{now}');
                 INSERT INTO messages (client_message_id, creator_id, room_id, thread_id, created_at, updated_at) SELECT 'r' || n, creator, {ALL_TALK}, (SELECT id FROM channel_threads WHERE parent_message_id = {next}), '{now}', '{now}' FROM (SELECT 1 AS n, {JASON} AS creator UNION ALL SELECT 2, {KEVIN} UNION ALL SELECT 3, {JASON});"
            );
            tx.conn().execute_batch(&sql)?;
            Ok(())
        })
        .await
        .unwrap();

    let page: api::MessagePage = parse(
        &b.send(get(&format!(
            "/api/v1/rooms/{ALL_TALK}/messages?around={BOOSTED}"
        )))
        .await,
    );
    let find = |id: i64| {
        page.messages
            .iter()
            .find(|message| message.id == id)
            .unwrap()
    };
    let boosted = find(BOOSTED);
    assert_eq!(boosted.reactions.len(), 1, "{:?}", boosted.reactions);
    assert_eq!(boosted.reactions[0].content, "💯");
    assert_eq!(
        boosted.reactions[0].reactor_ids,
        [JASON, DAVID],
        "distinct reactors in order of reaction"
    );
    assert_eq!(boosted.reactions[0].image_url, None);
    assert_eq!(
        boosted
            .boosts
            .iter()
            .map(|boost| (boost.booster_id, boost.content.as_str()))
            .collect::<Vec<_>>(),
        [(KEVIN, "nice work")]
    );
    assert!(!boosted.pinned && boosted.thread.is_none() && boosted.attachment.is_none());

    let pinned = find(next);
    assert!(pinned.pinned);
    let thread = pinned.thread.as_ref().expect("the reply indicator");
    assert_eq!(
        (thread.reply_count, thread.replier_ids.as_slice()),
        (3, &[JASON, KEVIN][..])
    );
    assert!(
        page.messages
            .iter()
            .all(|message| message.thread_id.is_none()),
        "replies stay off the room's timeline"
    );

    let forwarded = find(other);
    assert!(forwarded.forwarded_at.is_some());
    assert_eq!(forwarded.forward_note.as_deref(), Some("FYI"));

    let saved_item_id = a
        .db()
        .read(move |conn| {
            Ok(conn.query_row(
                "SELECT id FROM saved_items WHERE user_id = ? AND message_id = ?",
                [DAVID, next],
                |row| row.get::<_, i64>(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(
        page.saved,
        [api::SavedMark {
            message_id: next,
            saved_item_id
        }],
        "David's saves only"
    );
}

#[tokio::test]
async fn posting_attaches_a_direct_upload() {
    let Some(a) = app(true).await else { return };
    let mut b = a.sign_in(DAVID).await;
    let staged = a
        .booted
        .app
        .storage
        .stage_bytes(
            b"meeting notes",
            campfire_storage::Filename::new("notes.txt"),
            Some("text/plain"),
        )
        .unwrap();
    let blob = a
        .db()
        .write(move |tx| crate::controllers::messages::save_staged(tx, staged))
        .await
        .unwrap();
    let signed_id =
        campfire_storage::paths::signed_blob_id(&*a.booted.app.storage.verifier, blob.id, None);
    let body = |client_message_id: &str, signed_id: Option<&str>| json!({"clientMessageId": client_message_id, "markdownSource": "", "replyToMessageId": null, "replyNotifyAuthor": null, "attachmentSignedId": signed_id});
    let path = format!("/api/v1/rooms/{ALL_TALK}/messages");

    let bare = b
        .write(json_body(Method::POST, &path, &body("bare", None)))
        .await;
    assert_eq!(
        (bare.status, tag(&bare)),
        (StatusCode::UNPROCESSABLE_ENTITY, "Validation".into())
    );
    let forged = b
        .write(json_body(
            Method::POST,
            &path,
            &body("forged-upload", Some("not-a-signed-id")),
        ))
        .await;
    assert_eq!(
        (forged.status, tag(&forged)),
        (StatusCode::UNPROCESSABLE_ENTITY, "Validation".into())
    );

    let posted = b
        .write(json_body(
            Method::POST,
            &path,
            &body("upload-1", Some(&signed_id)),
        ))
        .await;
    assert_eq!(posted.status, StatusCode::CREATED, "{}", posted.text());
    let message: api::MessageDTO = parse(&posted);
    let attachment = message.attachment.expect("the attachment");
    assert_eq!(
        (
            attachment.filename.as_str(),
            attachment.byte_size,
            attachment.preview
        ),
        ("notes.txt", 13, api::AttachmentPreview::File)
    );
    assert_eq!(attachment.content_type, "text/plain");
    assert!(
        attachment
            .url
            .starts_with("/rails/active_storage/blobs/redirect/"),
        "{}",
        attachment.url
    );
    assert!(
        attachment.download_url.ends_with("disposition=attachment"),
        "{}",
        attachment.download_url
    );
}

#[tokio::test]
async fn reads_and_unreads_answer_the_read_state() {
    let Some(a) = app(true).await else { return };
    let mut b = a.sign_in(DAVID).await;
    let page: api::MessagePage = parse(
        &b.send(get(&format!("/api/v1/rooms/{ALL_TALK}/messages")))
            .await,
    );
    let message_id = page.messages.last().unwrap().id;

    let unread = b
        .write(json_body(
            Method::DELETE,
            &format!("/api/v1/rooms/{ALL_TALK}/read"),
            &json!({"messageId": message_id}),
        ))
        .await;
    assert_eq!(unread.status, StatusCode::OK, "{}", unread.text());
    assert_eq!(
        parse::<api::ReadState>(&unread),
        api::ReadState {
            room_id: ALL_TALK,
            unread: true,
            first_unread_message_id: Some(message_id),
            unread_count: 1,
        }
    );
    let detail: api::RoomDetail = parse(&b.send(get(&format!("/api/v1/rooms/{ALL_TALK}"))).await);
    assert_eq!(
        detail.unread,
        Some(api::UnreadDivider {
            first_unread_message_id: message_id,
            count: 1
        })
    );

    let read = b
        .write(
            Req::new(Method::POST, &format!("/api/v1/rooms/{ALL_TALK}/read"))
                .header("accept", "application/json"),
        )
        .await;
    assert_eq!(
        parse::<api::ReadState>(&read),
        api::ReadState {
            room_id: ALL_TALK,
            unread: false,
            first_unread_message_id: None,
            unread_count: 0,
        }
    );
    let detail: api::RoomDetail = parse(&b.send(get(&format!("/api/v1/rooms/{ALL_TALK}"))).await);
    assert_eq!(detail.unread, None);
}

// --- The sync socket ---------------------------------------------------------------------------

pub(super) struct Sync {
    socket: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    /// Events of a batch not yet looked at.
    pending: std::collections::VecDeque<api::SyncEvent>,
}

impl Sync {
    pub(super) async fn connect(addr: SocketAddr, cookie: &str, topics: &[String]) -> Self {
        Self::open(addr, cookie, topics, Value::Null).await
    }

    /// Connects and says hello, resuming from `resume` (`{epoch, seq}` or null).
    pub(super) async fn open(
        addr: SocketAddr,
        cookie: &str,
        topics: &[String],
        resume: Value,
    ) -> Self {
        let mut request = format!("ws://{addr}/api/v1/sync")
            .into_client_request()
            .unwrap();
        request
            .headers_mut()
            .insert("cookie", cookie.parse().unwrap());
        request
            .headers_mut()
            .insert("host", "campfire.test".parse().unwrap());
        request
            .headers_mut()
            .insert("origin", "http://campfire.test".parse().unwrap());
        let (socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
        let mut sync = Self {
            socket,
            pending: Default::default(),
        };
        sync.send(json!({"t": "hello", "v": 1, "resume": resume, "topics": topics}))
            .await;
        sync
    }

    async fn send(&mut self, frame: Value) {
        self.socket
            .send(WsMessage::Text(frame.to_string().into()))
            .await
            .unwrap();
    }

    /// The next server frame, `None` once the socket closes.
    pub(super) async fn next(&mut self) -> Option<api::ServerFrame> {
        loop {
            let message = tokio::time::timeout(Duration::from_secs(10), self.socket.next())
                .await
                .expect("a sync frame in time");
            match message {
                Some(Ok(WsMessage::Text(text))) => {
                    return Some(
                        serde_json::from_str(&text)
                            .unwrap_or_else(|error| panic!("{error}: {text}")),
                    );
                }
                Some(Ok(WsMessage::Close(_))) | Some(Err(_)) | None => return None,
                Some(Ok(_)) => continue,
            }
        }
    }

    pub(super) async fn welcome(&mut self) {
        assert!(matches!(
            self.next().await,
            Some(api::ServerFrame::Welcome { resumed: false, .. })
        ));
    }

    /// Events until one matches, failing on anything `forbidden` matches first.
    pub(super) async fn until(
        &mut self,
        wanted: impl Fn(&api::SyncEvent) -> bool,
        forbidden: impl Fn(&api::SyncEvent) -> bool,
    ) -> api::SyncEvent {
        loop {
            while let Some(event) = self.pending.pop_front() {
                assert!(!forbidden(&event), "unexpected {event:?}");
                if wanted(&event) {
                    return event;
                }
            }
            match self.next().await {
                Some(api::ServerFrame::Batch { events }) => self.pending.extend(events),
                Some(api::ServerFrame::Ping) => {}
                other => panic!("expected a batch, got {other:?}"),
            }
        }
    }
}

pub(super) async fn serve(a: &TestApp) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let router = a.booted.router.clone();
    (
        addr,
        tokio::spawn(async move { axum::serve(listener, router).await.unwrap() }),
    )
}

pub(super) fn created_in(room_id: i64) -> impl Fn(&api::SyncEvent) -> bool {
    move |event| matches!(&event.payload, api::SyncPayload::MessageCreated(message) if message.room_id == room_id)
}

#[tokio::test]
async fn a_classic_post_reaches_the_sync_socket_and_an_api_post_too() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let kevin = a.sign_in(KEVIN).await;
    let mut sync = Sync::connect(addr, &kevin.cookie_header(), &[format!("room:{HQ}")]).await;
    sync.welcome().await;

    let mut david = a.sign_in(DAVID).await;
    let classic = david
        .write(
            Req::new(Method::POST, &format!("/rooms/{HQ}/messages"))
                .header("accept", "text/vnd.turbo-stream.html")
                .header("content-type", "application/json")
                .body(json!({"message": {"markdown_source": "From the classic page", "client_message_id": "classic-1"}}).to_string()),
        )
        .await;
    assert_eq!(classic.status, StatusCode::OK, "{}", classic.text());
    let event = sync.until(created_in(HQ), |_| false).await;
    assert_eq!(event.topic, format!("room:{HQ}"));
    let api::SyncPayload::MessageCreated(message) = event.payload else {
        unreachable!()
    };
    assert_eq!(
        (message.creator_id, message.client_message_id.as_str()),
        (DAVID, "classic-1")
    );
    assert!(message.body_html.contains("From the classic page"));
    // Kevin's sidebar row went unread too.
    sync.until(
        |event| {
            matches!(
                event.payload,
                api::SyncPayload::RoomUnread(api::RoomUnread { room_id: HQ, .. })
            )
        },
        |_| false,
    )
    .await;

    let posted = post(&mut david, HQ, "api-1", "From the API").await;
    let posted: api::MessageDTO = parse(&posted);
    let event = sync.until(created_in(HQ), |_| false).await;
    let api::SyncPayload::MessageCreated(published) = event.payload else {
        unreachable!("created_in matches message.created only")
    };
    // The same message; `cardsAsOf` is each read's own time.
    assert_eq!(
        api::MessageDTO {
            cards_as_of: posted.cards_as_of.clone(),
            ..published
        },
        posted
    );
    server.abort();
}

#[tokio::test]
async fn a_non_member_cannot_follow_a_room() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let kevin = a.sign_in(KEVIN).await;
    let mut sync = Sync::connect(
        addr,
        &kevin.cookie_header(),
        &[format!("room:{ALL_PETS}"), format!("room:{HQ}")],
    )
    .await;
    sync.welcome().await;
    sync.send(json!({"t": "sub", "topics": [format!("room:{ALL_PETS}")]}))
        .await;

    let mut david = a.sign_in(DAVID).await;
    assert_eq!(
        post(&mut david, ALL_PETS, "pets-1", "Kevin can't see this")
            .await
            .status,
        StatusCode::CREATED
    );
    assert_eq!(
        post(&mut david, HQ, "hq-1", "Kevin sees this").await.status,
        StatusCode::CREATED
    );
    // HQ's message arrives, and nothing of All Pets' before it (events are in sequence order).
    let event = sync
        .until(created_in(HQ), |event| {
            event.topic == format!("room:{ALL_PETS}") || created_in(ALL_PETS)(event)
        })
        .await;
    assert_eq!(event.topic, format!("room:{HQ}"));
    server.abort();
}

#[tokio::test]
async fn revoking_the_session_closes_the_sync_socket() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut owner = a.sign_in(KEVIN).await;
    let victim = a.sign_in(KEVIN).await;
    let session_id = a
        .db()
        .read(|conn| {
            Ok(campfire_db::Session::for_user(conn, KEVIN)?
                .into_iter()
                .map(|s| s.id)
                .max()
                .unwrap())
        })
        .await
        .unwrap();
    let cookie = victim.cookie_header();
    let mut sync = Sync::connect(addr, &cookie, &[]).await;
    sync.welcome().await;

    let revoked = owner
        .write(Req::new(
            Method::DELETE,
            &format!("/users/me/sessions/{session_id}"),
        ))
        .await;
    assert_eq!(
        revoked.location(),
        Some("http://campfire.test/users/me/sessions")
    );
    loop {
        match sync.next().await {
            Some(api::ServerFrame::Bye { reason, .. }) => {
                assert_eq!(reason, "remote");
                break;
            }
            Some(api::ServerFrame::Batch { .. } | api::ServerFrame::Ping) => {}
            other => panic!("expected bye, got {other:?}"),
        }
    }
    assert!(sync.next().await.is_none(), "the socket closes after bye");

    // The revoked cookie can't come back.
    let mut again = Sync::connect(addr, &cookie, &[]).await;
    assert!(matches!(
        again.next().await,
        Some(api::ServerFrame::Bye {
            reconnect: false,
            ..
        })
    ));
    server.abort();
}

#[tokio::test]
async fn identical_posts_in_flight_create_one_message() {
    let Some(a) = app(true).await else { return };
    let mut first = a.sign_in(DAVID).await;
    let mut second = a.sign_in(DAVID).await;
    first.authenticity_token().await;
    second.authenticity_token().await;
    // Both posts pass the endpoint's early lookup before either writes, so only the check in the
    // create's write transaction can keep the second from making another message.
    campfire_api::test_hooks::hold_after_duplicate_check("race-1", 2);
    let (one, two) = tokio::join!(
        post(&mut first, ALL_TALK, "race-1", "Only once"),
        post(&mut second, ALL_TALK, "race-1", "Only once"),
    );
    let mut statuses = [one.status, two.status];
    statuses.sort();
    assert_eq!(
        statuses,
        [StatusCode::OK, StatusCode::CREATED],
        "{} / {}",
        one.text(),
        two.text()
    );
    assert_eq!(
        parse::<api::MessageDTO>(&one).id,
        parse::<api::MessageDTO>(&two).id
    );
    let count = a
        .db()
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM messages WHERE client_message_id = 'race-1'",
                [],
                |row| row.get::<_, i64>(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn a_cursor_on_a_deleted_message_still_pages() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let mut ids = Vec::new();
    for n in 1..=3 {
        let reply = post(
            &mut david,
            ALL_TALK,
            &format!("cursor-{n}"),
            &format!("Cursor {n}"),
        )
        .await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
        ids.push(parse::<api::MessageDTO>(&reply).id);
    }
    let deleted = david
        .write(Req::new(
            Method::DELETE,
            &format!("/rooms/{ALL_TALK}/messages/{}.turbo_stream", ids[1]),
        ))
        .await;
    assert_eq!(deleted.status, StatusCode::OK, "{}", deleted.text());

    let older = b_page(&mut david, &format!("before={}", ids[1])).await;
    assert_eq!(
        older.messages.last().map(|message| message.id),
        Some(ids[0])
    );
    assert_eq!(older.messages.len(), 40);
    let newer = b_page(&mut david, &format!("after={}", ids[1])).await;
    assert_eq!(
        newer
            .messages
            .iter()
            .map(|message| message.id)
            .collect::<Vec<_>>(),
        [ids[2]]
    );
    let gone = david
        .send(get(&format!(
            "/api/v1/rooms/{ALL_TALK}/messages?around={}",
            ids[1]
        )))
        .await;
    assert_eq!(
        gone.status,
        StatusCode::NOT_FOUND,
        "an around anchor that's gone"
    );

    // A cursor that was never on the room's timeline is a 404, not a page: another room's
    // message, or a reply in one of the room's threads (David is in both rooms).
    const THREADED_ROOM: i64 = 654632876;
    const THREAD_REPLY: i64 = 935962046;
    const ALL_PETS_MESSAGE: i64 = 935961886;
    let newest = david
        .send(get(&format!("/api/v1/rooms/{THREADED_ROOM}/messages")))
        .await;
    assert_eq!(newest.status, StatusCode::OK, "{}", newest.text());
    for (room, cursor) in [(THREADED_ROOM, THREAD_REPLY), (ALL_TALK, ALL_PETS_MESSAGE)] {
        for direction in ["before", "after"] {
            let reply = david
                .send(get(&format!(
                    "/api/v1/rooms/{room}/messages?{direction}={cursor}"
                )))
                .await;
            assert_eq!(
                reply.status,
                StatusCode::NOT_FOUND,
                "{direction}={cursor} in {room}: {}",
                reply.text()
            );
        }
    }
}

async fn b_page(b: &mut Browser<'_>, query: &str) -> api::MessagePage {
    let reply = b
        .send(get(&format!("/api/v1/rooms/{ALL_TALK}/messages?{query}")))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{query}: {}", reply.text());
    parse(&reply)
}

fn sidebar_row_removed(room_id: i64) -> impl Fn(&api::SyncEvent) -> bool {
    move |event| matches!(&event.payload, api::SyncPayload::SidebarRowRemoved(removed) if removed.room_id == room_id)
}

#[tokio::test]
async fn hiding_a_room_keeps_its_messages_coming() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut sync = Sync::connect(addr, &david.cookie_header(), &[format!("room:{ALL_TALK}")]).await;
    sync.welcome().await;

    let hidden = david
        .write(
            Req::new(
                Method::PATCH,
                &format!("/rooms/{ALL_TALK}/involvement.json"),
            )
            .form(&[("involvement", "invisible")]),
        )
        .await;
    assert_eq!(hidden.status, StatusCode::OK, "{}", hidden.text());
    sync.until(sidebar_row_removed(ALL_TALK), |_| false).await;

    // The classic page keeps streaming a room that's open, hidden or not; so does the socket.
    let mut jason = a.sign_in(JASON).await;
    let posted = post(&mut jason, ALL_TALK, "hidden-1", "Still here").await;
    assert_eq!(posted.status, StatusCode::CREATED, "{}", posted.text());
    let event = sync.until(created_in(ALL_TALK), |_| false).await;
    assert_eq!(event.topic, format!("room:{ALL_TALK}"));
    server.abort();
}

#[tokio::test]
async fn a_removed_member_stops_getting_the_rooms_events() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let cookie = david.cookie_header();
    let topics = [format!("room:{ALL_TALK}")];
    let mut sync = Sync::connect(addr, &cookie, &topics).await;
    let Some(api::ServerFrame::Welcome { epoch, mut seq, .. }) = sync.next().await else {
        panic!("a welcome")
    };

    a.db()
        .write(|tx| {
            campfire_db::Membership::find_by_room_and_user(tx.conn(), ALL_TALK, DAVID)?
                .expect("David is in All Talk")
                .destroy(tx)
        })
        .await
        .unwrap();
    // Losing a membership drops the person's connections, which reconnect (as the classic
    // pages do), resuming where they were.
    loop {
        match sync.next().await {
            Some(api::ServerFrame::Batch { events }) => {
                seq = events.last().map_or(seq, |event| event.seq);
            }
            Some(api::ServerFrame::Ping) => {}
            Some(api::ServerFrame::Bye {
                reconnect: true, ..
            }) => break,
            other => panic!("expected bye, got {other:?}"),
        }
    }
    assert!(sync.next().await.is_none());
    let mut sync = Sync::open(addr, &cookie, &topics, json!({"epoch": epoch, "seq": seq})).await;
    assert!(matches!(
        sync.next().await,
        Some(api::ServerFrame::Welcome { resumed: true, .. })
    ));

    let mut jason = a.sign_in(JASON).await;
    let posted = post(&mut jason, ALL_TALK, "after-removal", "David has left").await;
    assert_eq!(posted.status, StatusCode::CREATED, "{}", posted.text());
    // David's own topic as a fence: the room's events, if any, would come before it.
    let read = david
        .write(
            Req::new(Method::POST, &format!("/api/v1/rooms/{HQ}/read"))
                .header("accept", "application/json"),
        )
        .await;
    assert_eq!(read.status, StatusCode::OK, "{}", read.text());
    sync.until(
        |event| {
            matches!(
                event.payload,
                api::SyncPayload::RoomRead(api::RoomRead { room_id: HQ })
            )
        },
        |event| event.topic == format!("room:{ALL_TALK}") || created_in(ALL_TALK)(event),
    )
    .await;
    server.abort();
}

#[tokio::test]
async fn an_idle_admin_session_ends_on_the_next_heartbeat() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let david = a.sign_in(DAVID).await;
    let cookie = david.cookie_header();
    let mut sync = Sync::connect(addr, &cookie, &[]).await;
    sync.welcome().await;

    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE sessions SET last_active_at = '2000-01-01 00:00:00.000000' WHERE user_id = ?",
                [DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    sync.send(json!({"t": "hb", "active": false})).await;
    loop {
        match sync.next().await {
            Some(api::ServerFrame::Bye { reconnect, reason }) => {
                assert_eq!((reconnect, reason.as_str()), (false, "session_expired"));
                break;
            }
            Some(api::ServerFrame::Batch { .. } | api::ServerFrame::Ping) => {}
            other => panic!("expected bye, got {other:?}"),
        }
    }
    assert!(sync.next().await.is_none(), "the socket closes after bye");
    let mut again = Sync::connect(addr, &cookie, &[]).await;
    assert!(matches!(
        again.next().await,
        Some(api::ServerFrame::Bye {
            reconnect: false,
            ..
        })
    ));
    server.abort();
}

#[tokio::test]
async fn a_database_unread_carries_its_message_and_mention() {
    const ROOM: i64 = 654632876;
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    // David's classic page, listening for unreads, beside his sync socket.
    let (mut client, cable) =
        crate::controllers::messages::attachment_processing_tests::subscribe(&a).await;
    client
        .confirm(&crate::channels::tests::support::identifier(
            json!({ "channel": "UnreadRoomsChannel" }),
        ))
        .await;
    let david = a.sign_in(DAVID).await;
    let mut sync = Sync::connect(addr, &david.cookie_header(), &[]).await;
    sync.welcome().await;
    let capture = a.publications();
    capture.take();
    let sgid = rails_compat::global_id::attachable_sgid(
        &a.booted.app.secrets,
        &rails_compat::global_id::GlobalId::new("User", DAVID),
    );
    let mention = a
        .db()
        .write(move |tx| {
            campfire_db::Message::create(
                tx,
                campfire_db::NewMessage {
                    room_id: ROOM,
                    creator_id: JASON,
                    body: Some(format!(
                        "Thanks <action-text-attachment sgid=\"{sgid}\" content-type=\"application/vnd.campfire.mention\"></action-text-attachment>"
                    )),
                    client_message_id: Some("unread-mention".into()),
                    ..Default::default()
                },
            )
            .map(|message| message.id)
        })
        .await
        .unwrap();
    a.db()
        .write(move |tx| {
            use campfire_db::broadcasts::Broadcast;
            for (room_id, message_id) in
                [(ROOM, Some(mention)), (ALL_TALK, Some(BOOSTED)), (HQ, None)]
            {
                tx.emit_after_commit(campfire_db::Event::broadcast(&Broadcast::UnreadRoom {
                    user_id: DAVID,
                    room_id,
                    message_id,
                }));
            }
            Ok(())
        })
        .await
        .unwrap();
    // One of the rerouted emitters end to end: a scheduled message going out.
    let scheduled = a
        .db()
        .write(|tx| {
            use campfire_db::{NewScheduledMessage, ScheduledMessage};
            // An hour past the seed's frozen now, sent when it comes due.
            let now = campfire_db::Timestamp::from_jiff("2026-03-02T17:00:00Z".parse().unwrap());
            let row = ScheduledMessage::create(
                tx,
                NewScheduledMessage {
                    user_id: JASON,
                    room_id: ALL_TALK,
                    thread_id: None,
                    reply_to_message_id: None,
                    markdown_source: "Scheduled unread".into(),
                    send_at: now,
                },
            )?;
            assert!(ScheduledMessage::dispatch(tx, row.id, now, true)?);
            Ok(tx
                .conn()
                .query_row("SELECT MAX(id) FROM messages", [], |row| {
                    row.get::<_, i64>(0)
                })?)
        })
        .await
        .unwrap();
    let mut unreads = Vec::new();
    while unreads.len() < 4 {
        let event = sync
            .until(
                |event| matches!(event.payload, api::SyncPayload::RoomUnread(_)),
                |_| false,
            )
            .await;
        let api::SyncPayload::RoomUnread(unread) = event.payload else {
            unreachable!()
        };
        unreads.push(unread);
    }
    assert_eq!(
        unreads,
        [
            api::RoomUnread {
                room_id: ROOM,
                message_id: Some(mention),
                mentioned: true
            },
            api::RoomUnread {
                room_id: ALL_TALK,
                message_id: Some(BOOSTED),
                mentioned: false
            },
            api::RoomUnread {
                room_id: HQ,
                message_id: None,
                mentioned: false
            },
            api::RoomUnread {
                room_id: ALL_TALK,
                message_id: Some(scheduled),
                mentioned: false
            },
        ]
    );
    // The classic frames are unchanged: `{roomId}` on the person's unreads stream.
    let stream = campfire_db::broadcasts::unread_rooms_stream_name(DAVID);
    let mut classic = Vec::new();
    for _ in 0..100 {
        classic.extend(
            capture
                .take()
                .into_iter()
                .filter(|(name, _)| *name == stream)
                .map(|(_, frame)| serde_json::from_str::<Value>(&frame).unwrap()),
        );
        if classic.len() >= 4 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(
        classic,
        [ROOM, ALL_TALK, HQ, ALL_TALK].map(|room_id| json!({ "roomId": room_id }))
    );
    cable.abort();
    server.abort();
}

/// The classic frames (stream, payload) for one run of message create, edit and remove, and a
/// read and unread, with or without the SPA and a sync socket open.
async fn classic_frames(spa: bool) -> Option<Vec<(String, String)>> {
    use crate::controllers::presenters::test_support::SEED_NOW;
    use campfire_kit::clock::FrozenClock;
    let clock = std::sync::Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
    let env: &[(&str, &str)] = if spa { &[("SPA_ENABLED", "1")] } else { &[] };
    let a = TestApp::boot_seed_with_env("default", clock, env).await?;
    let (mut client, cable) =
        crate::controllers::messages::attachment_processing_tests::subscribe(&a).await;
    for channel in ["UnreadRoomsChannel", "ReadRoomsChannel"] {
        let identifier = crate::channels::tests::support::identifier(json!({ "channel": channel }));
        client.confirm(&identifier).await;
    }
    let mut david = a.sign_in(DAVID).await;
    let mut jason = a.sign_in(JASON).await;
    david.authenticity_token().await;
    jason.authenticity_token().await;
    let (_sync, server) = if spa {
        let (addr, server) = serve(&a).await;
        let mut sync =
            Sync::connect(addr, &david.cookie_header(), &[format!("room:{ALL_TALK}")]).await;
        sync.welcome().await;
        (Some(sync), Some(server))
    } else {
        (None, None)
    };
    assert_eq!(a.booted.app.cable.sync_wanted(), spa);
    let capture = a.publications();
    capture.take();

    let created = jason
        .write(
            Req::new(
                Method::POST,
                &format!("/rooms/{ALL_TALK}/messages.turbo_stream"),
            )
            .form(&[
                ("message[markdown_source]", "**Parity**"),
                ("message[client_message_id]", "parity-1"),
            ]),
        )
        .await;
    assert_eq!(created.status, StatusCode::OK, "{}", created.text());
    let id = a
        .db()
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT id FROM messages WHERE client_message_id = 'parity-1'",
                [],
                |row| row.get::<_, i64>(0),
            )?)
        })
        .await
        .unwrap();
    let edited = jason
        .write(
            Req::new(
                Method::PATCH,
                &format!("/rooms/{ALL_TALK}/messages/{id}.json"),
            )
            .header("content-type", "application/json")
            .body(json!({"message": {"markdown_source": "## Edited"}}).to_string()),
        )
        .await;
    assert_eq!(edited.status, StatusCode::OK, "{}", edited.text());
    let removed = jason
        .write(Req::new(
            Method::DELETE,
            &format!("/rooms/{ALL_TALK}/messages/{id}.turbo_stream"),
        ))
        .await;
    assert_eq!(removed.status, StatusCode::OK, "{}", removed.text());
    let read = david
        .write(
            Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/read"))
                .header("accept", "application/json"),
        )
        .await;
    assert_eq!(read.status, StatusCode::OK, "{}", read.text());
    let unread = david
        .write(
            Req::new(
                Method::DELETE,
                &format!("/rooms/{ALL_TALK}/read?message_id={BOOSTED}"),
            )
            .header("accept", "application/json"),
        )
        .await;
    assert_eq!(unread.status, StatusCode::OK, "{}", unread.text());

    // Some frames go out after the response (the after-commit sink): wait for them to settle.
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
async fn the_classic_frames_are_the_same_with_the_sync_engine_on() {
    let (Some(off), Some(on)) = (classic_frames(false).await, classic_frames(true).await) else {
        return;
    };
    let kinds = |stream: &str, needle: &str| {
        off.iter()
            .any(|(s, frame)| s.ends_with(stream) && frame.contains(needle))
    };
    assert!(
        kinds(":messages", "action=\\\"append\\\""),
        "a create: {off:?}"
    );
    assert!(
        kinds(":messages", "action=\\\"replace\\\""),
        "an edit: {off:?}"
    );
    assert!(
        kinds(":messages", "action=\\\"remove\\\""),
        "a remove: {off:?}"
    );
    assert!(kinds("_unreads", "roomId"), "an unread: {off:?}");
    assert!(kinds("_reads", "room_id"), "a read: {off:?}");
    assert_eq!(off.len(), on.len(), "off: {off:#?}\non: {on:#?}");
    for (index, (off, on)) in off.iter().zip(&on).enumerate() {
        assert_eq!(off, on, "frame {index}");
    }
}
