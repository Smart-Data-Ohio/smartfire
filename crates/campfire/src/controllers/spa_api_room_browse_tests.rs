//! The channel directory over `/api/v1/rooms/browse`: only alive open rooms, searched by name and
//! topic, paged by an opaque cursor, each with its member count and whether the viewer has joined.

use axum::http::StatusCode;
use campfire_api_types as api;
use campfire_db::{CachedStatements, Involvement, RoomType, Timestamp};
use serde_json::Value;

use super::api_tests::{app, get, parse};
use crate::controllers::presenters::test_support::{Browser, DAVID, HQ, TestApp};

async fn insert_room(
    app: &TestApp,
    kind: RoomType,
    name: Option<&str>,
    topic: Option<&str>,
    updated_at: &str,
) -> i64 {
    let (name, topic) = (name.map(str::to_owned), topic.map(str::to_owned));
    let updated_at = Timestamp::parse_db(updated_at).expect("timestamp");
    app.db()
        .write(move |tx| {
            let id = tx.conn().query_row_cached(
                "INSERT INTO rooms (created_at,creator_id,type,updated_at,name,topic) VALUES (?,?,?,?,?,?) RETURNING id",
                (tx.now(), DAVID, kind, updated_at, name, topic),
                |row| row.get(0),
            )?;
            Ok(id)
        })
        .await
        .unwrap()
}

async fn open_room(app: &TestApp, name: &str, topic: Option<&str>, updated_at: &str) -> i64 {
    insert_room(app, RoomType::Open, Some(name), topic, updated_at).await
}

async fn set_membership(app: &TestApp, room_id: i64, user_id: i64, involvement: Involvement) {
    app.db()
        .write(move |tx| {
            tx.conn().execute_cached(
                "INSERT INTO memberships (room_id,user_id,involvement,created_at,updated_at) VALUES (?1,?2,?3,?4,?4) ON CONFLICT (room_id,user_id) DO UPDATE SET involvement=excluded.involvement",
                (room_id, user_id, involvement, tx.now()),
            )?;
            Ok(())
        })
        .await
        .unwrap();
}

fn path(query: &str, cursor: Option<&str>) -> String {
    let mut path = format!(
        "/api/v1/rooms/browse?q={}",
        url::form_urlencoded::byte_serialize(query.as_bytes()).collect::<String>()
    );
    if let Some(cursor) = cursor {
        path.push_str(&format!("&cursor={cursor}"));
    }
    path
}

async fn browse(
    browser: &mut Browser<'_>,
    query: &str,
    cursor: Option<&str>,
) -> api::ChannelDirectory {
    let reply = browser.send(get(&path(query, cursor))).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    parse(&reply)
}

/// Every page for `query`, checking each but the last is full.
async fn walk(browser: &mut Browser<'_>, query: &str) -> Vec<api::DirectoryChannel> {
    let (mut channels, mut cursor) = (Vec::new(), None::<String>);
    loop {
        let page = browse(browser, query, cursor.as_deref()).await;
        let full = page.channels.len() == 50;
        channels.extend(page.channels);
        match page.next_cursor {
            Some(next) => {
                assert!(full, "only the last page may be short");
                cursor = Some(next);
            }
            None => return channels,
        }
    }
}

fn ids(channels: &[api::DirectoryChannel]) -> Vec<i64> {
    channels.iter().map(|channel| channel.room_id).collect()
}

#[tokio::test]
async fn only_alive_open_rooms_are_listed_even_by_exact_name() {
    let app = app(true).await.expect("seed required");
    let open = open_room(&app, "Hidden Cove", None, "2026-03-01 00:00:00.000000").await;
    let mut refused = Vec::new();
    for kind in [
        RoomType::Closed,
        RoomType::Direct,
        RoomType::Voice,
        RoomType::Stage,
        RoomType::Board,
    ] {
        let name = format!("Hidden Cove {}", kind.class_name());
        refused.push((
            name.clone(),
            insert_room(&app, kind, Some(&name), None, "2026-03-01 00:00:00.000000").await,
        ));
    }
    let deleted = open_room(
        &app,
        "Hidden Cove deleted",
        None,
        "2026-03-01 00:00:00.000000",
    )
    .await;
    app.db()
        .write(move |tx| {
            tx.conn().execute_cached(
                "UPDATE rooms SET deleted_at=? WHERE id=?",
                (tx.now(), deleted),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    refused.push(("Hidden Cove deleted".into(), deleted));
    let mut david = app.sign_in(DAVID).await;

    for (name, _) in &refused {
        let page = browse(&mut david, name, None).await;
        assert!(page.channels.is_empty(), "{name}: {:?}", page.channels);
    }
    assert_eq!(
        ids(&browse(&mut david, "hidden cove", None).await.channels),
        [open]
    );
    let everything = ids(&walk(&mut david, "").await);
    assert!(everything.contains(&open) && everything.contains(&HQ));
    for (name, id) in &refused {
        assert!(!everything.contains(id), "{name} listed");
    }
}

#[tokio::test]
async fn joined_means_a_membership_that_is_not_hidden() {
    let app = app(true).await.expect("seed required");
    let room = open_room(
        &app,
        "Joinable Heron",
        Some("birds"),
        "2026-03-01 12:00:00.123456",
    )
    .await;
    let mut david = app.sign_in(DAVID).await;

    let reply = david.send(get(&path("joinable heron", None))).await;
    let body: Value = serde_json::from_slice(&reply.body).unwrap();
    assert_eq!(
        body.as_object().unwrap().keys().collect::<Vec<_>>(),
        ["channels", "nextCursor"]
    );
    assert_eq!(
        body["channels"][0]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        [
            "roomId",
            "name",
            "topic",
            "memberCount",
            "lastActiveAt",
            "joined"
        ]
    );
    let page: api::ChannelDirectory = parse(&reply);
    assert_eq!(
        page,
        api::ChannelDirectory {
            channels: vec![api::DirectoryChannel {
                room_id: room,
                name: "Joinable Heron".into(),
                topic: Some("birds".into()),
                member_count: 0,
                last_active_at: Timestamp::parse_db("2026-03-01 12:00:00.123456")
                    .unwrap()
                    .to_wire(),
                joined: false,
            }],
            next_cursor: None,
        }
    );

    set_membership(&app, room, DAVID, Involvement::Invisible).await;
    let hidden = &browse(&mut david, "joinable heron", None).await.channels[0];
    assert_eq!((hidden.joined, hidden.member_count), (false, 1));

    set_membership(&app, room, DAVID, Involvement::Mentions).await;
    assert!(browse(&mut david, "joinable heron", None).await.channels[0].joined);
    assert!(
        browse(&mut david, "HQ", None)
            .await
            .channels
            .iter()
            .any(|channel| channel.room_id == HQ && channel.joined)
    );
}

#[tokio::test]
async fn search_ranks_exact_then_prefix_then_anywhere_then_topic() {
    let app = app(true).await.expect("seed required");
    let topic_only = open_room(
        &app,
        "Lounge",
        Some("PLOVER sightings"),
        "2026-03-05 00:00:00.000000",
    )
    .await;
    let anywhere = open_room(&app, "The Plover Club", None, "2026-03-04 00:00:00.000000").await;
    let newer_prefix = open_room(&app, "plover-west", None, "2026-03-03 00:00:00.000000").await;
    let older_prefix = open_room(&app, "Plover East", None, "2026-03-02 00:00:00.000000").await;
    let exact = open_room(&app, "plover", None, "2026-03-01 00:00:00.000000").await;
    open_room(
        &app,
        "Unrelated",
        Some("nothing here"),
        "2026-03-06 00:00:00.000000",
    )
    .await;
    let mut david = app.sign_in(DAVID).await;

    assert_eq!(
        ids(&browse(&mut david, "  PLOVER ", None).await.channels),
        [exact, newer_prefix, older_prefix, anywhere, topic_only]
    );
    assert_eq!(
        ids(&browse(&mut david, "sightings", None).await.channels),
        [topic_only]
    );
}

/// SQLite's `LOWER` folds ASCII only, so the query does too: a non-ASCII letter matches in its
/// own case, and the ASCII letters around it in either case.
#[tokio::test]
async fn non_ascii_names_match_in_their_own_case() {
    let app = app(true).await.expect("seed required");
    let prefix = open_room(&app, "Équipe Sud", None, "2026-03-02 00:00:00.000000").await;
    let exact = open_room(&app, "Équipe", None, "2026-03-01 00:00:00.000000").await;
    let mut david = app.sign_in(DAVID).await;

    for query in ["Équipe", "ÉQUIPE"] {
        assert_eq!(
            ids(&browse(&mut david, query, None).await.channels),
            [exact, prefix],
            "{query}"
        );
    }
}

#[tokio::test]
async fn like_wildcards_in_the_query_are_literal() {
    let app = app(true).await.expect("seed required");
    let percent = open_room(&app, "zz100% done", None, "2026-03-01 00:00:00.000000").await;
    open_room(&app, "zz1000 done", None, "2026-03-01 00:00:00.000000").await;
    let underscore = open_room(&app, "zz_a room", None, "2026-03-01 00:00:00.000000").await;
    open_room(&app, "zzxa room", None, "2026-03-01 00:00:00.000000").await;
    let backslash = open_room(&app, r"zz\q room", None, "2026-03-01 00:00:00.000000").await;
    open_room(&app, "zzq room", None, "2026-03-01 00:00:00.000000").await;
    let mut david = app.sign_in(DAVID).await;

    assert_eq!(
        ids(&browse(&mut david, "zz100%", None).await.channels),
        [percent]
    );
    assert_eq!(
        ids(&browse(&mut david, "zz_a", None).await.channels),
        [underscore]
    );
    assert_eq!(
        ids(&browse(&mut david, r"zz\q", None).await.channels),
        [backslash]
    );
    assert!(browse(&mut david, "zz%", None).await.channels.is_empty());
}

#[tokio::test]
async fn pages_return_every_open_room_once_in_order() {
    let app = app(true).await.expect("seed required");
    // Pairs share a timestamp, so the id breaks the tie.
    for index in 0..60 {
        let at = format!("2026-02-{:02} 00:00:00.000000", 1 + index / 3);
        open_room(&app, &format!("Paging room {index}"), None, &at).await;
    }
    let mut expected: Vec<(String, i64)> = app
        .db()
        .read(|conn| {
            Ok(conn
                .prepare("SELECT updated_at, id FROM rooms WHERE type='Rooms::Open' AND deleted_at IS NULL")?
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()?)
        })
        .await
        .unwrap();
    expected.sort_by(|a, b| b.cmp(a));
    assert!(expected.len() > 60);
    let mut david = app.sign_in(DAVID).await;

    let first = browse(&mut david, "", None).await;
    assert_eq!(first.channels.len(), 50);
    assert!(first.next_cursor.is_some());
    assert_eq!(
        ids(&walk(&mut david, "").await),
        expected.iter().map(|(_, id)| *id).collect::<Vec<_>>()
    );

    let invalid = david.send(get(&path("", Some("not-a-cursor")))).await;
    assert_eq!(
        invalid.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        invalid.text()
    );
    let api::ApiError::Validation { fields, .. } = parse::<api::ApiErrorResponse>(&invalid).error
    else {
        panic!("{}", invalid.text())
    };
    assert_eq!(fields.keys().collect::<Vec<_>>(), ["cursor"]);
}

#[tokio::test]
async fn pages_of_a_search_cross_rank_boundaries_in_order() {
    let app = app(true).await.expect("seed required");
    let mut expected = Vec::new();
    // 40 prefix, 20 anywhere and 10 topic-only matches: the first page ends among the anywhere
    // matches, so the second continues into the topic-only ones.
    for (rank, count) in [(1, 40), (2, 20), (3, 10)] {
        let mut group = Vec::new();
        for index in 0..count {
            let at = format!("2026-02-{:02} 00:00:00.000000", 1 + index / 2);
            let id = match rank {
                1 => open_room(&app, &format!("pagebird {index}"), None, &at).await,
                2 => open_room(&app, &format!("the pagebird {index}"), None, &at).await,
                _ => open_room(&app, &format!("perch {index}"), Some("a pagebird"), &at).await,
            };
            group.push((at, id));
        }
        group.sort_by(|a, b| b.cmp(a));
        expected.extend(group.into_iter().map(|(_, id)| id));
    }
    let mut david = app.sign_in(DAVID).await;

    assert_eq!(ids(&walk(&mut david, "pagebird").await), expected);
}

#[tokio::test]
async fn member_count_is_what_the_members_pane_lists() {
    let app = app(true).await.expect("seed required");
    let others: Vec<i64> = app
        .db()
        .read(|conn| {
            Ok(conn
                .prepare("SELECT user_id FROM memberships WHERE room_id=? AND user_id!=? ORDER BY user_id")?
                .query_map((HQ, DAVID), |row| row.get(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?)
        })
        .await
        .unwrap();
    assert!(others.len() >= 2, "HQ needs two other members");
    let (hidden, deactivated) = (others[0], others[1]);
    set_membership(&app, HQ, hidden, Involvement::Invisible).await;
    app.db()
        .write(move |tx| {
            tx.conn()
                .execute_cached("UPDATE users SET status=1 WHERE id=?", [deactivated])?;
            Ok(())
        })
        .await
        .unwrap();
    let mut david = app.sign_in(DAVID).await;

    let pane = david
        .send(get(&format!("/api/v1/rooms/{HQ}/members")))
        .await;
    assert_eq!(pane.status, StatusCode::OK, "{}", pane.text());
    let pane: api::MemberList = parse(&pane);
    let listed: Vec<i64> = pane.members.iter().map(|member| member.user_id).collect();
    assert!(listed.contains(&hidden) && !listed.contains(&deactivated));

    let hq = browse(&mut david, "HQ", None)
        .await
        .channels
        .into_iter()
        .find(|channel| channel.room_id == HQ)
        .expect("HQ listed");
    assert_eq!(hq.member_count, listed.len() as i64);
}
