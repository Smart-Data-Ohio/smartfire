//! The S2 directory endpoints on `/api/v1` (`campfire_api::directory`): direct messages, a
//! room's members and files, stars and the switcher. Each answers in the contract's shape; the
//! direct room changes and new messages publish `sidebar.row.upserted`, and the classic frames of
//! the broadcasts these touch are byte-for-byte the same with the sync engine on.

use std::time::Duration;

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use serde_json::{Value, json};

use super::api_tests::{ALL_PETS, Sync, app, get, json_body, parse, serve, tag};
use crate::controllers::presenters::test_support::{
    BENDER, DAVID, DIRECT_DAVID_JASON, JASON, KEVIN, Reply, Req, TestApp,
};

/// A closed room of David, Jason, Kevin, JZ, Mallory (banned) and Deploy Bot, with five files.
const DESIGNERS: i64 = 654632876;
/// David, Jason, Kevin and JZ's group direct message.
const GROUP: i64 = 699448329;
const JZ: i64 = 773523953;
const DEPLOY_BOT: i64 = 773523956;
const LONELY_LOU: i64 = 773523958;
const NEW_MEMBER: i64 = 773523959;
/// Deactivated.
const RITA: i64 = 773523954;

fn status_and_tag(reply: &Reply) -> (StatusCode, String) {
    (reply.status, tag(reply))
}

fn upserted(user_topic: bool, room_id: i64) -> impl Fn(&api::SyncEvent) -> bool {
    move |event| {
        matches!(&event.payload, api::SyncPayload::SidebarRowUpserted(row) if row.room.id == room_id)
            && (!user_topic || event.topic.starts_with("user"))
    }
}

fn row_of(event: &api::SyncEvent) -> &api::SidebarRow {
    let api::SyncPayload::SidebarRowUpserted(row) = &event.payload else {
        unreachable!()
    };
    row
}

#[tokio::test]
async fn direct_candidates_and_stars() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let list: api::DirectCandidateList =
        parse(&david.send(get("/api/v1/directs/candidates")).await);
    let ids: Vec<i64> = list.candidates.iter().map(|c| c.user_id).collect();
    assert!(!ids.contains(&DAVID) && !ids.contains(&RITA), "{ids:?}");
    for id in [JASON, KEVIN, JZ, BENDER, DEPLOY_BOT, LONELY_LOU, NEW_MEMBER] {
        assert!(ids.contains(&id), "{id}: {ids:?}");
    }
    let bender = list
        .candidates
        .iter()
        .find(|c| c.user_id == BENDER)
        .unwrap();
    assert!(bender.agent);
    assert!(list.candidates.iter().all(|c| !c.starred));
    let mut listed: Vec<i64> = list.users.iter().map(|user| user.id).collect();
    listed.sort();
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(listed, sorted);

    let star = format!("/api/v1/users/{KEVIN}/star");
    for _ in 0..2 {
        let reply = david.write(json_body(Method::PUT, &star, &json!({}))).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        assert_eq!(
            parse::<api::StarState>(&reply),
            api::StarState {
                user_id: KEVIN,
                starred: true
            }
        );
    }
    let list: api::DirectCandidateList =
        parse(&david.send(get("/api/v1/directs/candidates")).await);
    assert_eq!(list.candidates[0].user_id, KEVIN, "starred first");
    assert!(list.candidates[0].starred);
    let members: api::MemberList = parse(
        &david
            .send(get(&format!("/api/v1/rooms/{DESIGNERS}/members")))
            .await,
    );
    assert!(
        members
            .members
            .iter()
            .any(|m| m.user_id == KEVIN && m.starred)
    );

    let reply = david
        .write(json_body(Method::DELETE, &star, &json!({})))
        .await;
    assert_eq!(
        parse::<api::StarState>(&reply),
        api::StarState {
            user_id: KEVIN,
            starred: false
        }
    );
    let reply = david
        .write(json_body(
            Method::PUT,
            &format!("/api/v1/users/{DAVID}/star"),
            &json!({}),
        ))
        .await;
    assert_eq!(
        status_and_tag(&reply),
        (StatusCode::UNPROCESSABLE_ENTITY, "Validation".into())
    );
    let reply = david
        .write(json_body(
            Method::PUT,
            "/api/v1/users/999999/star",
            &json!({}),
        ))
        .await;
    assert_eq!(
        status_and_tag(&reply),
        (StatusCode::NOT_FOUND, "NotFound".into())
    );
}

#[tokio::test]
async fn opening_a_direct_room_answers_its_sidebar_row() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let kevin = a.sign_in(KEVIN).await;
    let mut sync = Sync::connect(addr, &kevin.cookie_header(), &[]).await;
    sync.welcome().await;

    // The one with Jason exists.
    let reply = david
        .write(json_body(
            Method::POST,
            "/api/v1/directs",
            &json!({"userIds": [JASON]}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let row: api::SidebarRow = parse(&reply);
    assert_eq!(row.room.id, DIRECT_DAVID_JASON);
    assert_eq!(row.direct_member_ids, [JASON]);

    // A new group; inactive people are left out.
    let reply = david
        .write(json_body(
            Method::POST,
            "/api/v1/directs",
            &json!({"userIds": [KEVIN, LONELY_LOU, RITA]}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let row: api::SidebarRow = parse(&reply);
    let mut members = row.direct_member_ids.clone();
    members.sort();
    assert_eq!(members, [KEVIN, LONELY_LOU]);
    let event = sync.until(upserted(true, row.room.id), |_| false).await;
    assert_eq!(event.topic, "user");
    let mut members = row_of(&event).direct_member_ids.clone();
    members.sort();
    assert_eq!(members, [DAVID, LONELY_LOU]);
    // The same people get the same room.
    let reply = david
        .write(json_body(
            Method::POST,
            "/api/v1/directs",
            &json!({"userIds": [LONELY_LOU, KEVIN]}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(parse::<api::SidebarRow>(&reply).room.id, row.room.id);

    // Only the first ten ids count, and only active people among them.
    let reply = david
        .write(json_body(
            Method::POST,
            "/api/v1/directs",
            &json!({"userIds": [JASON, KEVIN, JZ, BENDER, DEPLOY_BOT, LONELY_LOU, NEW_MEMBER, 1, 2, 3]}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    assert_eq!(parse::<api::SidebarRow>(&reply).direct_member_ids.len(), 7);
    server.abort();
}

#[tokio::test]
async fn growing_and_renaming_a_group_publishes_each_members_row() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let jason = a.sign_in(JASON).await;
    let mut sync = Sync::connect(addr, &jason.cookie_header(), &[]).await;
    sync.welcome().await;

    let members = format!("/api/v1/directs/{GROUP}/members");
    let reply = david
        .write(json_body(
            Method::POST,
            &members,
            &json!({"userIds": [LONELY_LOU, JZ]}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let detail: api::RoomDetail = parse(&reply);
    assert!(detail.direct_member_ids.contains(&LONELY_LOU), "{detail:?}");
    assert_eq!(detail.member_count, 5);
    let event = sync.until(upserted(true, GROUP), |_| false).await;
    assert_eq!(event.topic, "user");
    assert!(row_of(&event).direct_member_ids.contains(&LONELY_LOU));

    // Nobody new is a no-op: the room as it is, and no event.
    let reply = david
        .write(json_body(Method::POST, &members, &json!({"userIds": [JZ]})))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert_eq!(parse::<api::RoomDetail>(&reply).member_count, 5);
    // A one-to-one room, and a room Kevin isn't in.
    let reply = david
        .write(json_body(
            Method::POST,
            &format!("/api/v1/directs/{DIRECT_DAVID_JASON}/members"),
            &json!({"userIds": [KEVIN]}),
        ))
        .await;
    assert_eq!(
        status_and_tag(&reply),
        (StatusCode::UNPROCESSABLE_ENTITY, "Validation".into())
    );
    for path in [
        format!("/api/v1/directs/{DIRECT_DAVID_JASON}/members"),
        format!("/api/v1/directs/{DESIGNERS}/members"),
    ] {
        let reply = kevin
            .write(json_body(Method::POST, &path, &json!({"userIds": [JZ]})))
            .await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{path}");
    }

    let rename = format!("/api/v1/directs/{GROUP}");
    let reply = david
        .write(json_body(
            Method::PATCH,
            &rename,
            &json!({"name": "  Launch crew "}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let detail: api::RoomDetail = parse(&reply);
    assert_eq!(detail.display_name, "Launch crew");
    sync.until(
        |event| upserted(true, GROUP)(event) && row_of(event).display_name == "Launch crew",
        |_| false,
    )
    .await;
    let reply = david
        .write(json_body(
            Method::PATCH,
            &rename,
            &json!({"name": "x".repeat(101)}),
        ))
        .await;
    assert_eq!(
        status_and_tag(&reply),
        (StatusCode::UNPROCESSABLE_ENTITY, "Validation".into())
    );
    let reply = david
        .write(json_body(Method::PATCH, &rename, &json!({"name": null})))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert_ne!(parse::<api::RoomDetail>(&reply).display_name, "Launch crew");
    let reply = david
        .write(json_body(
            Method::PATCH,
            &format!("/api/v1/directs/{DIRECT_DAVID_JASON}"),
            &json!({"name": "Us"}),
        ))
        .await;
    assert_eq!(
        status_and_tag(&reply),
        (StatusCode::UNPROCESSABLE_ENTITY, "Validation".into())
    );
    server.abort();
}

#[tokio::test]
async fn members_files_and_the_switcher() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;

    let list: api::MemberList = parse(
        &david
            .send(get(&format!("/api/v1/rooms/{DESIGNERS}/members")))
            .await,
    );
    let ids: Vec<i64> = list.members.iter().map(|m| m.user_id).collect();
    assert_eq!(
        ids,
        [DAVID, DEPLOY_BOT, JASON, JZ, KEVIN],
        "active, by name"
    );
    assert_eq!(list.users.iter().map(|u| u.id).collect::<Vec<_>>(), ids);
    let bot = list
        .members
        .iter()
        .find(|m| m.user_id == DEPLOY_BOT)
        .unwrap();
    assert_eq!(bot.presence, api::Presence::Offline);
    let reply = kevin
        .send(get(&format!("/api/v1/rooms/{ALL_PETS}/members")))
        .await;
    assert_eq!(
        status_and_tag(&reply),
        (StatusCode::NOT_FOUND, "NotFound".into())
    );

    let files: api::FileList = parse(
        &david
            .send(get(&format!("/api/v1/rooms/{DESIGNERS}/files")))
            .await,
    );
    assert_eq!(files.files.len(), 5, "{files:?}");
    assert_eq!(files.next_page, None);
    for pair in files.files.windows(2) {
        assert!(pair[0].created_at >= pair[1].created_at);
    }
    let names = |list: &api::FileList| {
        let mut names: Vec<String> = list
            .files
            .iter()
            .map(|file| file.attachment.filename.clone())
            .collect();
        names.sort();
        names
    };
    let images: api::FileList = parse(
        &david
            .send(get(&format!("/api/v1/rooms/{DESIGNERS}/files?type=images")))
            .await,
    );
    assert_eq!(names(&images), ["black_hole.jpg", "moon.jpg", "pixel.bmp"]);
    let moon: api::FileList = parse(
        &david
            .send(get(&format!(
                "/api/v1/rooms/{DESIGNERS}/files?filename=MOON&page=1"
            )))
            .await,
    );
    assert_eq!(names(&moon), ["moon.jpg"]);
    let creator = moon.files[0].creator_id;
    assert!(moon.users.iter().any(|user| user.id == creator));
    let beyond: api::FileList = parse(
        &david
            .send(get(&format!("/api/v1/rooms/{DESIGNERS}/files?page=2")))
            .await,
    );
    assert!(beyond.files.is_empty() && beyond.next_page.is_none());
    let reply = kevin
        .send(get(&format!("/api/v1/rooms/{ALL_PETS}/files")))
        .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);

    let switcher: api::Switcher = parse(&david.send(get("/api/v1/switcher")).await);
    let room = |id: i64| switcher.rooms.iter().find(|room| room.room_id == id);
    assert_eq!(
        room(DESIGNERS).map(|r| r.kind),
        Some(api::SwitcherRoomKind::Channel)
    );
    assert_eq!(
        room(DIRECT_DAVID_JASON).map(|r| r.kind),
        Some(api::SwitcherRoomKind::Dm)
    );
    assert_eq!(
        room(DIRECT_DAVID_JASON).map(|r| r.name.as_str()),
        Some("Jason")
    );
    assert_eq!(
        room(GROUP).map(|r| r.kind),
        Some(api::SwitcherRoomKind::Group)
    );
    let jason = switcher
        .people
        .iter()
        .find(|person| person.user_id == JASON)
        .unwrap();
    assert_eq!(jason.direct_room_id, Some(DIRECT_DAVID_JASON));
    assert!(
        switcher
            .people
            .iter()
            .all(|person| ![DAVID, BENDER, DEPLOY_BOT, RITA].contains(&person.user_id))
    );
    let lou = switcher
        .people
        .iter()
        .find(|person| person.user_id == LONELY_LOU)
        .unwrap();
    assert_eq!(lou.direct_room_id, None);
    let mut people: Vec<i64> = switcher.people.iter().map(|p| p.user_id).collect();
    people.sort();
    assert_eq!(
        switcher.users.iter().map(|u| u.id).collect::<Vec<_>>(),
        people
    );
    assert!(!switcher.threads.is_empty() && switcher.threads.len() <= 15);
}

#[tokio::test]
async fn a_new_message_upserts_each_unread_members_row() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let jason = a.sign_in(JASON).await;
    let mut sync = Sync::connect(addr, &jason.cookie_header(), &[]).await;
    sync.welcome().await;
    let body = json!({"clientMessageId": "row-ping", "markdownSource": "Counting rows"});
    let reply = david
        .write(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{DESIGNERS}/messages"),
            &body,
        ))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let unread = sync
        .until(
            |event| matches!(&event.payload, api::SyncPayload::RoomUnread(unread) if unread.room_id == DESIGNERS),
            |_| false,
        )
        .await;
    assert_eq!(unread.topic, "user");
    let event = sync.until(upserted(true, DESIGNERS), |_| false).await;
    let row = row_of(&event);
    assert!(row.unread_count >= 1, "{event:?}");
    assert!(row.membership.unread_at.is_some(), "{event:?}");
    // Only once: `message_create` publishes it, and the sink's unread ping doesn't again. Give a
    // second copy time to come, then mark the end with an event published at once.
    tokio::time::sleep(Duration::from_millis(300)).await;
    campfire_app::cable::sync::sidebar_row_removed(&a.booted.app.cable, JASON, 1);
    sync.until(
        |event| matches!(&event.payload, api::SyncPayload::SidebarRowRemoved(removed) if removed.room_id == 1),
        upserted(false, DESIGNERS),
    )
    .await;
    server.abort();
}

/// The classic frames of a message post and of growing, renaming and opening direct rooms.
async fn classic_directory_frames(spa: bool) -> Option<Vec<(String, String)>> {
    use crate::controllers::presenters::test_support::SEED_NOW;
    use campfire_kit::clock::FrozenClock;
    let clock = std::sync::Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
    let env: &[(&str, &str)] = if spa { &[("SPA_ENABLED", "1")] } else { &[] };
    let a = TestApp::boot_seed_with_env("default", clock, env).await?;
    // David's classic pages: All Talk (the helper's), Designers, the group, his rooms list and
    // his unreads.
    let (mut client, cable) =
        crate::controllers::messages::attachment_processing_tests::subscribe(&a).await;
    let designers = a
        .db()
        .read(|conn| campfire_db::Room::find(conn, DESIGNERS))
        .await
        .unwrap();
    let secrets = &a.booted.app.secrets;
    let group = a
        .db()
        .read(|conn| campfire_db::Room::find(conn, GROUP))
        .await
        .unwrap();
    let messages = |room: &campfire_db::Room| {
        rails_compat::turbo::signed_stream_name(
            secrets,
            &[&campfire_app::cable::room_gid(room).to_param(), "messages"],
        )
    };
    let rooms = rails_compat::turbo::signed_stream_name(
        secrets,
        &[&crate::channels::user_gid(DAVID).to_param(), "rooms"],
    );
    for identifier in [
        json!({ "channel": "RoomMessagesChannel", "signed_stream_name": messages(&designers) }),
        json!({ "channel": "RoomMessagesChannel", "signed_stream_name": messages(&group) }),
        json!({ "channel": "Turbo::StreamsChannel", "signed_stream_name": rooms }),
        json!({ "channel": "UnreadRoomsChannel" }),
    ] {
        client
            .confirm(&crate::channels::tests::support::identifier(identifier))
            .await;
    }
    let mut david = a.sign_in(DAVID).await;
    let mut jason = a.sign_in(JASON).await;
    david.authenticity_token().await;
    jason.authenticity_token().await;
    let (_sync, server) = if spa {
        let (addr, server) = serve(&a).await;
        let mut sync = Sync::connect(addr, &david.cookie_header(), &[]).await;
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

    // Jason posts, so David's unreads get the ping.
    let reply = jason
        .write(
            Req::new(
                Method::POST,
                &format!("/rooms/{DESIGNERS}/messages.turbo_stream"),
            )
            .form(&[
                ("message[markdown_source]", "Row parity"),
                ("message[client_message_id]", "row-parity"),
            ]),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let reply = david
        .write(classic(
            Method::POST,
            format!("/rooms/directs/{GROUP}/add_members"),
            json!({"user_ids": [LONELY_LOU]}),
        ))
        .await;
    assert!(reply.status.is_redirection(), "{}", reply.text());
    let reply = david
        .write(classic(
            Method::PATCH,
            format!("/rooms/directs/{GROUP}"),
            json!({"room": {"name": "Parity crew"}}),
        ))
        .await;
    assert!(reply.status.is_redirection(), "{}", reply.text());
    let reply = david
        .write(classic(
            Method::POST,
            "/rooms/directs".into(),
            json!({"user_ids": [NEW_MEMBER]}),
        ))
        .await;
    assert!(reply.status.is_redirection(), "{}", reply.text());

    let mut frames = Vec::new();
    let mut quiet = 0;
    while quiet < 10 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let more = capture.take();
        quiet = if more.is_empty() { quiet + 1 } else { 0 };
        frames.extend(more);
    }
    if let Some(server) = server {
        server.abort();
    }
    cable.abort();
    drop(client);
    Some(frames)
}

#[tokio::test]
async fn the_directory_frames_are_the_same_with_the_sync_engine_on() {
    let (Some(off), Some(on)) = (
        classic_directory_frames(false).await,
        classic_directory_frames(true).await,
    ) else {
        return;
    };
    let has = |needle: &str| off.iter().any(|(_, frame)| frame.contains(needle));
    assert!(has("Row parity"), "the message: {off:#?}");
    assert!(has("Parity crew"), "the rename: {off:#?}");
    assert!(
        has("added Lonely Lou to the group"),
        "the added note: {off:#?}"
    );
    assert!(
        off.iter().any(|(stream, _)| stream.ends_with("_unreads")),
        "the unread ping: {off:#?}"
    );
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
