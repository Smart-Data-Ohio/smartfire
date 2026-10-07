//! S3 sidebar organisation on `/api/v1` (`campfire_api::organize`): categories, a room's
//! category, favourites and notification levels, with their `sidebar.*` twins.

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use campfire_db::{Membership, Room, RoomCategory, RoomType};
use serde_json::json;

use super::api_tests::{ALL_PETS, Sync, app, json_body, parse, serve, tag};
use crate::controllers::presenters::test_support::{
    ALL_TALK, Browser, DAVID, DIRECT_DAVID_JASON, JASON, KEVIN, QUIET_CORNER, Reply, Req, TestApp,
};

const DESIGNERS: i64 = 654632876;

/// David with no favourites and no categories.
async fn clean_slate(a: &TestApp) {
    a.db()
        .write(|tx| {
            for mut membership in Membership::for_user(tx.conn(), DAVID)? {
                membership.unfavorite(tx)?;
            }
            for category in RoomCategory::ordered_for_user(tx.conn(), DAVID)? {
                category.destroy(tx)?;
            }
            Ok(())
        })
        .await
        .unwrap();
}

async fn send(b: &mut Browser<'_>, method: Method, path: &str, body: serde_json::Value) -> Reply {
    b.write(json_body(method, path, &body)).await
}

fn ok<T: serde::de::DeserializeOwned>(reply: &Reply) -> T {
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    parse(reply)
}

fn fields(reply: &Reply) -> Vec<String> {
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

fn row_of(room_id: i64) -> impl Fn(&api::SyncEvent) -> bool {
    move |event| matches!(&event.payload, api::SyncPayload::SidebarRowUpserted(row) if row.room.id == room_id)
}

fn category_upserted(id: i64) -> impl Fn(&api::SyncEvent) -> bool {
    move |event| matches!(&event.payload, api::SyncPayload::SidebarCategoryUpserted(category) if category.id == id)
}

async fn favorite_positions(a: &TestApp) -> Vec<(i64, Option<i64>)> {
    a.db()
        .read(|conn| Membership::favorites_for_user(conn, DAVID))
        .await
        .unwrap()
        .into_iter()
        .map(|m| (m.room_id, m.favorite_position))
        .collect()
}

#[tokio::test]
async fn categories_answer_and_tell_the_other_tabs() {
    let Some(a) = app(true).await else { return };
    clean_slate(&a).await;
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let mut sync = Sync::connect(addr, &david.cookie_header(), &[]).await;
    sync.welcome().await;

    let reply = send(
        &mut david,
        Method::POST,
        "/api/v1/room_categories",
        json!({"name": "Work"}),
    )
    .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let work: api::RoomCategory = parse(&reply);
    assert_eq!((work.name.as_str(), work.collapsed), ("Work", false));
    sync.until(category_upserted(work.id), |_| false).await;

    for name in ["  ", &"x".repeat(51)] {
        let reply = send(
            &mut david,
            Method::POST,
            "/api/v1/room_categories",
            json!({"name": name}),
        )
        .await;
        assert_eq!(fields(&reply), ["name"]);
    }
    let reply = send(
        &mut david,
        Method::POST,
        "/api/v1/room_categories",
        json!({"name": "Play", "collapsed": true}),
    )
    .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let play: api::RoomCategory = parse(&reply);
    assert!(play.collapsed && play.position > work.position);

    // Reordering: the whole list, each once, or a 409 that changes nothing.
    let reply = send(
        &mut david,
        Method::PUT,
        "/api/v1/room_categories/order",
        json!({"categoryIds": [play.id, work.id]}),
    )
    .await;
    let list: api::RoomCategoryList = ok(&reply);
    assert_eq!(
        list.categories
            .iter()
            .map(|c| (c.id, c.position))
            .collect::<Vec<_>>(),
        [(play.id, 1), (work.id, 2)]
    );
    sync.until(category_upserted(play.id), |_| false).await;
    for ids in [
        json!([work.id]),
        json!([work.id, work.id, play.id]),
        json!([play.id, work.id, 999]),
    ] {
        let reply = send(
            &mut david,
            Method::PUT,
            "/api/v1/room_categories/order",
            json!({"categoryIds": ids}),
        )
        .await;
        assert_eq!(
            reply.status,
            StatusCode::CONFLICT,
            "{ids}: {}",
            reply.text()
        );
        assert_eq!(tag(&reply), "Conflict");
    }
    let order = a
        .db()
        .read(|conn| RoomCategory::ordered_for_user(conn, DAVID))
        .await
        .unwrap();
    assert_eq!(
        order.iter().map(|c| c.id).collect::<Vec<_>>(),
        [play.id, work.id]
    );

    // Folding keeps the name; renaming keeps the fold.
    let path = format!("/api/v1/room_categories/{}", work.id);
    let folded: api::RoomCategory =
        ok(&send(&mut david, Method::PATCH, &path, json!({"collapsed": true})).await);
    assert_eq!((folded.name.as_str(), folded.collapsed), ("Work", true));
    sync.until(|e| matches!(&e.payload, api::SyncPayload::SidebarCategoryUpserted(c) if c.id == work.id && c.collapsed), |_| false).await;
    let renamed: api::RoomCategory =
        ok(&send(&mut david, Method::PATCH, &path, json!({"name": "Jobs"})).await);
    assert_eq!((renamed.name.as_str(), renamed.collapsed), ("Jobs", true));
    assert_eq!(
        fields(&send(&mut david, Method::PATCH, &path, json!({"name": ""})).await),
        ["name"]
    );

    // Another person's category is a 404, to change, delete or use.
    assert_eq!(
        send(&mut kevin, Method::PATCH, &path, json!({"name": "Mine"}))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        kevin.write(Req::new(Method::DELETE, &path)).await.status,
        StatusCode::NOT_FOUND
    );
    let kevins = a
        .db()
        .write(|tx| RoomCategory::create(tx, KEVIN, "Kevin's", 1, false))
        .await
        .unwrap();
    let assign = format!("/api/v1/rooms/{ALL_TALK}/category");
    assert_eq!(
        send(
            &mut david,
            Method::PUT,
            &assign,
            json!({"roomCategoryId": kevins.id})
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );

    // Into a category, then out of a deleted one.
    let row: api::SidebarRow = ok(&send(
        &mut david,
        Method::PUT,
        &assign,
        json!({"roomCategoryId": work.id}),
    )
    .await);
    assert_eq!(row.membership.room_category_id, Some(work.id));
    sync.until(|e| matches!(&e.payload, api::SyncPayload::SidebarRowUpserted(r) if r.room.id == ALL_TALK && r.membership.room_category_id == Some(work.id)), |_| false).await;
    // Only channels have a category.
    let direct = format!("/api/v1/rooms/{DIRECT_DAVID_JASON}/category");
    assert_eq!(
        fields(
            &send(
                &mut david,
                Method::PUT,
                &direct,
                json!({"roomCategoryId": work.id})
            )
            .await
        ),
        ["roomCategoryId"]
    );
    let reply = david.write(Req::new(Method::DELETE, &path)).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.text());
    // The room goes back to Channels before the category goes.
    sync.until(
        |e| matches!(&e.payload, api::SyncPayload::SidebarRowUpserted(r) if r.room.id == ALL_TALK && r.membership.room_category_id.is_none()),
        |e| matches!(&e.payload, api::SyncPayload::SidebarCategoryRemoved(_)),
    )
    .await;
    sync.until(
        |e| matches!(&e.payload, api::SyncPayload::SidebarCategoryRemoved(r) if r.id == work.id),
        |_| false,
    )
    .await;
    assert_eq!(
        david.write(Req::new(Method::DELETE, &path)).await.status,
        StatusCode::NOT_FOUND
    );
    server.abort();
}

#[tokio::test]
async fn favourites_count_only_shown_ones_and_tell_the_other_tabs() {
    let Some(a) = app(true).await else { return };
    clean_slate(&a).await;
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut sync = Sync::connect(addr, &david.cookie_header(), &[]).await;
    sync.welcome().await;

    for room in [ALL_TALK, QUIET_CORNER, DESIGNERS, ALL_PETS] {
        let path = format!("/api/v1/rooms/{room}/favorite");
        let row: api::SidebarRow = ok(&david.write(Req::new(Method::POST, &path)).await);
        assert!(row.membership.favorite_position.is_some());
        sync.until(row_of(room), |_| false).await;
    }
    // Again: unchanged.
    let again: api::SidebarRow = ok(&david
        .write(Req::new(
            Method::POST,
            &format!("/api/v1/rooms/{ALL_TALK}/favorite"),
        ))
        .await);
    assert_eq!(again.membership.favorite_position, Some(0));

    // Hide Quiet Corner, the second favourite.
    let hide = format!("/api/v1/rooms/{QUIET_CORNER}/involvement");
    let hidden: api::Membership = ok(&send(
        &mut david,
        Method::PUT,
        &hide,
        json!({"involvement": "invisible"}),
    )
    .await);
    assert_eq!(hidden.involvement, api::Involvement::Invisible);
    sync.until(
        |e| matches!(&e.payload, api::SyncPayload::SidebarRowRemoved(r) if r.room_id == QUIET_CORNER),
        |_| false,
    )
    .await;

    // The sidebar shows All Talk, Designers, Pets. Dropping All Talk at 1 puts it after
    // Designers; counting the hidden one too would leave it where it was.
    let reply = send(
        &mut david,
        Method::PATCH,
        &format!("/api/v1/rooms/{ALL_TALK}/favorite"),
        json!({"position": 1}),
    )
    .await;
    let list: api::FavoriteList = ok(&reply);
    assert_eq!(
        list.rows.iter().map(|row| row.room.id).collect::<Vec<_>>(),
        [DESIGNERS, ALL_TALK, ALL_PETS]
    );
    // Hidden ones keep their places, and positions stay dense.
    assert_eq!(
        favorite_positions(&a).await,
        [
            (QUIET_CORNER, Some(0)),
            (DESIGNERS, Some(1)),
            (ALL_TALK, Some(2)),
            (ALL_PETS, Some(3))
        ]
    );
    // The moved rows are published; the hidden one isn't, though its number changed.
    let not_hidden = |e: &api::SyncEvent| row_of(QUIET_CORNER)(e);
    sync.until(row_of(DESIGNERS), not_hidden).await;
    sync.until(row_of(ALL_TALK), not_hidden).await;

    // Clamped both ways, never rejected.
    let list: api::FavoriteList = ok(&send(
        &mut david,
        Method::PATCH,
        &format!("/api/v1/rooms/{ALL_PETS}/favorite"),
        json!({"position": -5}),
    )
    .await);
    assert_eq!(
        list.rows.iter().map(|row| row.room.id).collect::<Vec<_>>(),
        [ALL_PETS, DESIGNERS, ALL_TALK]
    );
    let list: api::FavoriteList = ok(&send(
        &mut david,
        Method::PATCH,
        &format!("/api/v1/rooms/{ALL_PETS}/favorite"),
        json!({"position": 99}),
    )
    .await);
    assert_eq!(
        list.rows.iter().map(|row| row.room.id).collect::<Vec<_>>(),
        [DESIGNERS, ALL_TALK, ALL_PETS]
    );
    assert_eq!(
        favorite_positions(&a)
            .await
            .iter()
            .map(|(_, p)| *p)
            .collect::<Vec<_>>(),
        [Some(0), Some(1), Some(2), Some(3)]
    );

    // A hidden room's row still answers, hidden; a non-favourite's move changes nothing.
    let row: api::SidebarRow = ok(&david
        .write(Req::new(
            Method::DELETE,
            &format!("/api/v1/rooms/{QUIET_CORNER}/favorite"),
        ))
        .await);
    assert_eq!(row.membership.involvement, api::Involvement::Invisible);
    assert_eq!(row.membership.favorite_position, None);
    let before = favorite_positions(&a).await;
    let list: api::FavoriteList = ok(&send(
        &mut david,
        Method::PATCH,
        &format!("/api/v1/rooms/{QUIET_CORNER}/favorite"),
        json!({"position": 0}),
    )
    .await);
    assert_eq!(list.rows.len(), 3);
    assert_eq!(favorite_positions(&a).await, before);

    // Not a member: 404.
    let secret = a
        .db()
        .write(|tx| Room::create_for(tx, RoomType::Closed, Some("Secret"), JASON, &[JASON]))
        .await
        .unwrap();
    for method in [Method::POST, Method::DELETE] {
        let reply = david
            .write(Req::new(
                method,
                &format!("/api/v1/rooms/{}/favorite", secret.id),
            ))
            .await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND);
    }
    assert_eq!(
        send(
            &mut david,
            Method::PATCH,
            &format!("/api/v1/rooms/{}/favorite", secret.id),
            json!({"position": 0})
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );
    server.abort();
}

#[tokio::test]
async fn the_classic_drag_counts_only_shown_favourites_too() {
    let Some(a) = app(true).await else { return };
    clean_slate(&a).await;
    let mut david = a.sign_in(DAVID).await;
    for room in [ALL_TALK, QUIET_CORNER, DESIGNERS, ALL_PETS] {
        let reply = david
            .write(Req::new(
                Method::POST,
                &format!("/rooms/{room}/favorite.json"),
            ))
            .await;
        assert_eq!(reply.status, StatusCode::OK);
    }
    let hidden = david
        .write(
            Req::new(
                Method::PATCH,
                &format!("/rooms/{QUIET_CORNER}/involvement.json"),
            )
            .form(&[("involvement", "invisible")]),
        )
        .await;
    assert_eq!(hidden.status, StatusCode::OK);
    // `sidebar_organize_controller.js` sends the index among the rows it shows.
    let reply = david
        .write(
            Req::new(Method::PATCH, &format!("/rooms/{ALL_TALK}/favorite.json"))
                .form(&[("position", "1")]),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(
        favorite_positions(&a).await,
        [
            (QUIET_CORNER, Some(0)),
            (DESIGNERS, Some(1)),
            (ALL_TALK, Some(2)),
            (ALL_PETS, Some(3))
        ]
    );
}

#[tokio::test]
async fn involvement_answers_the_membership_and_mutes_read() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE memberships SET unread_at = ?, involvement = 'everything' WHERE user_id = ? AND room_id = ?",
                rusqlite::params![tx.now(), DAVID, ALL_TALK],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut david = a.sign_in(DAVID).await;
    let mut sync = Sync::connect(addr, &david.cookie_header(), &[]).await;
    sync.welcome().await;
    let path = format!("/api/v1/rooms/{ALL_TALK}/involvement");

    let muted: api::Membership = ok(&send(
        &mut david,
        Method::PUT,
        &path,
        json!({"involvement": "muted"}),
    )
    .await);
    assert_eq!(muted.involvement, api::Involvement::Muted);
    assert_eq!(muted.unread_at, None);
    sync.until(
        |e| matches!(&e.payload, api::SyncPayload::RoomRead(r) if r.room_id == ALL_TALK),
        |_| false,
    )
    .await;
    sync.until(
        |e| matches!(&e.payload, api::SyncPayload::SidebarRowUpserted(r) if r.room.id == ALL_TALK && r.membership.involvement == api::Involvement::Muted),
        |_| false,
    )
    .await;

    // Hidden, then back.
    ok::<api::Membership>(
        &send(
            &mut david,
            Method::PUT,
            &path,
            json!({"involvement": "invisible"}),
        )
        .await,
    );
    sync.until(
        |e| matches!(&e.payload, api::SyncPayload::SidebarRowRemoved(r) if r.room_id == ALL_TALK),
        |_| false,
    )
    .await;
    let back: api::Membership = ok(&send(
        &mut david,
        Method::PUT,
        &path,
        json!({"involvement": "everything"}),
    )
    .await);
    assert_eq!(back.involvement, api::Involvement::Everything);
    sync.until(row_of(ALL_TALK), |_| false).await;

    // Muting a room that's already read changes no unread state: no `room.read`, just the row.
    ok::<api::Membership>(
        &send(
            &mut david,
            Method::PUT,
            &path,
            json!({"involvement": "muted"}),
        )
        .await,
    );
    sync.until(
        |e| matches!(&e.payload, api::SyncPayload::SidebarRowUpserted(r) if r.room.id == ALL_TALK && r.membership.involvement == api::Involvement::Muted),
        |e| matches!(&e.payload, api::SyncPayload::RoomRead(_)),
    )
    .await;

    // Any level for any kind of room; an unknown one is a 422; not a member, a 404.
    let direct = format!("/api/v1/rooms/{DIRECT_DAVID_JASON}/involvement");
    let hidden: api::Membership = ok(&send(
        &mut david,
        Method::PUT,
        &direct,
        json!({"involvement": "invisible"}),
    )
    .await);
    assert_eq!(hidden.involvement, api::Involvement::Invisible);
    let loud = send(
        &mut david,
        Method::PUT,
        &path,
        json!({"involvement": "loud"}),
    )
    .await;
    assert_eq!(
        loud.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        loud.text()
    );
    let mut kevin = a.sign_in(KEVIN).await;
    assert_eq!(
        send(
            &mut kevin,
            Method::PUT,
            &path,
            json!({"involvement": "muted"})
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );
    server.abort();
}

fn any_sidebar(event: &api::SyncEvent) -> bool {
    matches!(
        &event.payload,
        api::SyncPayload::SidebarRowUpserted(_)
            | api::SyncPayload::SidebarRowRemoved(_)
            | api::SyncPayload::SidebarCategoryUpserted(_)
            | api::SyncPayload::SidebarCategoryRemoved(_)
    )
}

#[tokio::test]
async fn another_persons_tab_hears_none_of_davids_organising() {
    let Some(a) = app(true).await else { return };
    clean_slate(&a).await;
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let mut davids = Sync::connect(addr, &david.cookie_header(), &[]).await;
    davids.welcome().await;
    let mut kevins = Sync::connect(addr, &kevin.cookie_header(), &[]).await;
    kevins.welcome().await;

    // Quiet Corner has both of them in it.
    let reply = send(
        &mut david,
        Method::POST,
        "/api/v1/room_categories",
        json!({"name": "Mine"}),
    )
    .await;
    let mine: api::RoomCategory = parse(&reply);
    davids.until(category_upserted(mine.id), |_| false).await;
    let path = format!("/api/v1/room_categories/{}", mine.id);
    ok::<api::RoomCategory>(
        &send(&mut david, Method::PATCH, &path, json!({"collapsed": true})).await,
    );
    davids
        .until(
            |e| matches!(&e.payload, api::SyncPayload::SidebarCategoryUpserted(c) if c.collapsed),
            |_| false,
        )
        .await;
    let favorite = format!("/api/v1/rooms/{QUIET_CORNER}/favorite");
    ok::<api::SidebarRow>(&david.write(Req::new(Method::POST, &favorite)).await);
    davids.until(row_of(QUIET_CORNER), |_| false).await;
    let involvement = format!("/api/v1/rooms/{QUIET_CORNER}/involvement");
    ok::<api::Membership>(
        &send(
            &mut david,
            Method::PUT,
            &involvement,
            json!({"involvement": "muted"}),
        )
        .await,
    );
    davids.until(|e| matches!(&e.payload, api::SyncPayload::SidebarRowUpserted(r) if r.room.id == QUIET_CORNER && r.membership.involvement == api::Involvement::Muted), |_| false).await;
    ok::<api::SidebarRow>(&david.write(Req::new(Method::DELETE, &favorite)).await);
    davids.until(|e| matches!(&e.payload, api::SyncPayload::SidebarRowUpserted(r) if r.room.id == QUIET_CORNER && r.membership.favorite_position.is_none()), |_| false).await;
    assert_eq!(
        david.write(Req::new(Method::DELETE, &path)).await.status,
        StatusCode::NO_CONTENT
    );
    davids
        .until(
            |e| matches!(&e.payload, api::SyncPayload::SidebarCategoryRemoved(_)),
            |_| false,
        )
        .await;

    // David's tabs have every event, so anything sent to Kevin went before his own next one.
    let reply = send(
        &mut kevin,
        Method::POST,
        "/api/v1/room_categories",
        json!({"name": "Kevin's"}),
    )
    .await;
    let kevins_own: api::RoomCategory = parse(&reply);
    kevins
        .until(category_upserted(kevins_own.id), |event| {
            any_sidebar(event) && !category_upserted(kevins_own.id)(event)
        })
        .await;
    server.abort();
}

#[tokio::test]
async fn classic_organising_tells_the_spa_tabs() {
    let Some(a) = app(true).await else { return };
    clean_slate(&a).await;
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut sync = Sync::connect(addr, &david.cookie_header(), &[]).await;
    sync.welcome().await;

    let created = david
        .write(
            Req::new(Method::POST, "/room_categories").form(&[("room_category[name]", "Classic")]),
        )
        .await;
    assert!(created.status.is_redirection(), "{}", created.status);
    let category = a
        .db()
        .read(|conn| RoomCategory::ordered_for_user(conn, DAVID))
        .await
        .unwrap()
        .pop()
        .expect("the category");
    sync.until(category_upserted(category.id), |_| false).await;

    let favorited = david
        .write(Req::new(
            Method::POST,
            &format!("/rooms/{ALL_TALK}/favorite.json"),
        ))
        .await;
    assert_eq!(favorited.status, StatusCode::OK);
    sync.until(
        |e| matches!(&e.payload, api::SyncPayload::SidebarRowUpserted(r) if r.room.id == ALL_TALK && r.membership.favorite_position.is_some()),
        |_| false,
    )
    .await;
    server.abort();
}

/// The classic frames for a run of classic organising, with or without the SPA and a sync
/// socket open.
async fn organizing_frames(spa: bool) -> Option<Vec<(String, String)>> {
    use crate::controllers::presenters::test_support::SEED_NOW;
    use campfire_kit::clock::FrozenClock;
    let clock = std::sync::Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
    let env: &[(&str, &str)] = if spa { &[("SPA_ENABLED", "1")] } else { &[] };
    let a = TestApp::boot_seed_with_env("default", clock, env).await?;
    clean_slate(&a).await;
    // A classic tab, listening on David's rooms stream.
    let (mut client, cable) =
        crate::controllers::messages::attachment_processing_tests::subscribe(&a).await;
    let rooms = campfire_app::cable::user_gid(DAVID).to_param();
    let signed = rails_compat::turbo::signed_stream_name(&a.booted.app.secrets, &[&rooms, "rooms"]);
    client
        .confirm(&crate::channels::tests::support::identifier(
            json!({"channel": "Turbo::StreamsChannel", "signed_stream_name": signed}),
        ))
        .await;
    let mut david = a.sign_in(DAVID).await;
    david.authenticity_token().await;
    let (_sync, server) = if spa {
        let (addr, server) = serve(&a).await;
        let mut sync = Sync::connect(addr, &david.cookie_header(), &[]).await;
        sync.welcome().await;
        (Some(sync), Some(server))
    } else {
        (None, None)
    };
    let capture = a.publications();
    capture.take();

    let created = david
        .write(Req::new(Method::POST, "/room_categories").form(&[("room_category[name]", "Work")]))
        .await;
    assert!(created.status.is_redirection(), "{}", created.status);
    let category = a
        .db()
        .read(|conn| RoomCategory::ordered_for_user(conn, DAVID))
        .await
        .unwrap()
        .pop()
        .expect("the category");
    let steps = [
        Req::new(Method::PATCH, &format!("/room_categories/{}", category.id))
            .form(&[("room_category[collapsed]", "true")]),
        Req::new(
            Method::PATCH,
            &format!("/rooms/{ALL_TALK}/category_assignment.json"),
        )
        .form(&[("room_category_id", &category.id.to_string())]),
        Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/favorite.json")),
        Req::new(
            Method::POST,
            &format!("/rooms/{QUIET_CORNER}/favorite.json"),
        ),
        Req::new(
            Method::PATCH,
            &format!("/rooms/{QUIET_CORNER}/favorite.json"),
        )
        .form(&[("position", "0")]),
        Req::new(Method::DELETE, &format!("/rooms/{ALL_TALK}/favorite.json")),
        Req::new(
            Method::PUT,
            &format!("/rooms/{QUIET_CORNER}/involvement.json"),
        )
        .form(&[("involvement", "invisible")]),
        Req::new(
            Method::PUT,
            &format!("/rooms/{QUIET_CORNER}/involvement.json"),
        )
        .form(&[("involvement", "everything")]),
        Req::new(Method::DELETE, &format!("/room_categories/{}", category.id)),
    ];
    for step in steps {
        let reply = david.write(step).await;
        assert!(
            reply.status.is_success() || reply.status.is_redirection(),
            "{}",
            reply.text()
        );
    }

    // Some frames go out after the response (the after-commit sink): wait for them to settle.
    let mut frames = Vec::new();
    let mut quiet = 0;
    while quiet < 10 {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
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
async fn classic_organising_frames_are_the_same_with_the_sync_engine_on() {
    let (Some(off), Some(on)) = (
        organizing_frames(false).await,
        organizing_frames(true).await,
    ) else {
        return;
    };
    // Favourites and categories send no classic frames; hiding and showing a room do, on the
    // person's rooms stream.
    let sent = |action: &str| {
        off.iter()
            .any(|(stream, frame)| stream.ends_with(":rooms") && frame.contains(action))
    };
    assert!(
        sent(r#"action=\"remove\""#) && sent(r#"action=\"prepend\""#),
        "{off:?}"
    );
    assert_eq!(off, on);
}

#[tokio::test]
async fn a_favourite_in_a_deleted_room_isnt_counted() {
    let Some(a) = app(true).await else { return };
    clean_slate(&a).await;
    let mut david = a.sign_in(DAVID).await;
    for room in [ALL_TALK, DESIGNERS, ALL_PETS] {
        let reply = david
            .write(Req::new(
                Method::POST,
                &format!("/api/v1/rooms/{room}/favorite"),
            ))
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    }
    // Designers goes; its membership and favourite stay, as a soft delete leaves them.
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE rooms SET deleted_at = ? WHERE id = ?",
                rusqlite::params![tx.now(), DESIGNERS],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    // The sidebar shows All Talk, Pets: All Talk at 1 goes after Pets.
    let list: api::FavoriteList = ok(&send(
        &mut david,
        Method::PATCH,
        &format!("/api/v1/rooms/{ALL_TALK}/favorite"),
        json!({"position": 1}),
    )
    .await);
    assert_eq!(
        list.rows.iter().map(|row| row.room.id).collect::<Vec<_>>(),
        [ALL_PETS, ALL_TALK]
    );
    assert_eq!(
        favorite_positions(&a).await,
        [
            (DESIGNERS, Some(0)),
            (ALL_PETS, Some(1)),
            (ALL_TALK, Some(2))
        ]
    );
}

/// A PATCH whose request reads nothing stale: held after it's parsed, while another tab
/// reorders and renames, then let go.
async fn race_a_patch(
    a: &TestApp,
    patch: serde_json::Value,
) -> (
    api::RoomCategory,
    Vec<RoomCategory>,
    api::RoomCategory,
    api::RoomCategory,
) {
    clean_slate(a).await;
    let mut tab_a = a.sign_in(DAVID).await;
    let mut tab_b = a.sign_in(DAVID).await;
    tab_a.authenticity_token().await;
    tab_b.authenticity_token().await;
    let work: api::RoomCategory = parse(
        &send(
            &mut tab_a,
            Method::POST,
            "/api/v1/room_categories",
            json!({"name": "Work"}),
        )
        .await,
    );
    let play: api::RoomCategory = parse(
        &send(
            &mut tab_a,
            Method::POST,
            "/api/v1/room_categories",
            json!({"name": "Play"}),
        )
        .await,
    );
    let hold = campfire_api::test_hooks::hold_before_category_write(work.id);
    let path = format!("/api/v1/room_categories/{}", work.id);
    let (reply, ()) = tokio::join!(send(&mut tab_b, Method::PATCH, &path, patch), async {
        hold.reached.wait().await;
        let reordered = send(
            &mut tab_a,
            Method::PUT,
            "/api/v1/room_categories/order",
            json!({"categoryIds": [play.id, work.id]}),
        )
        .await;
        assert_eq!(reordered.status, StatusCode::OK, "{}", reordered.text());
        let renamed = send(&mut tab_a, Method::PATCH, &path, json!({"name": "Jobs"})).await;
        assert_eq!(renamed.status, StatusCode::OK, "{}", renamed.text());
        hold.release.wait().await;
    });
    let answered: api::RoomCategory = ok(&reply);
    let stored = a
        .db()
        .read(|conn| RoomCategory::ordered_for_user(conn, DAVID))
        .await
        .unwrap();
    (answered, stored, work, play)
}

#[tokio::test]
async fn a_category_patch_racing_a_reorder_and_a_rename_keeps_both() {
    let Some(a) = app(true).await else { return };
    let (answered, stored, work, play) = race_a_patch(&a, json!({"collapsed": true})).await;
    // The fold lands; the reorder and the rename made meanwhile stay.
    assert_eq!(
        (
            answered.name.as_str(),
            answered.position,
            answered.collapsed
        ),
        ("Jobs", 2, true)
    );
    assert_eq!(
        stored
            .iter()
            .map(|c| (c.id, c.position, c.name.as_str(), c.collapsed))
            .collect::<Vec<_>>(),
        [(play.id, 1, "Play", false), (work.id, 2, "Jobs", true)]
    );
}

/// Holds David's mute of All Talk after its lookup, sets All Talk's unread state to `unread`
/// meanwhile (as a new message or another tab's read would), then lets the mute go on.
async fn mute_while_unread_changes(a: &TestApp, david: &mut Browser<'_>, unread: bool) -> Reply {
    let membership_id = a
        .db()
        .read(|conn| {
            Ok(Membership::find_by_room_and_user(conn, ALL_TALK, DAVID)?
                .unwrap()
                .id)
        })
        .await
        .unwrap();
    let hold = campfire_api::test_hooks::hold_before_involvement_write(membership_id);
    let path = format!("/api/v1/rooms/{ALL_TALK}/involvement");
    let (reply, ()) = tokio::join!(
        send(david, Method::PUT, &path, json!({"involvement": "muted"})),
        async {
            hold.reached.wait().await;
            a.db()
                .write(move |tx| {
                    let unread_at = unread.then(|| tx.now());
                    tx.conn().execute(
                        "UPDATE memberships SET unread_at = ? WHERE id = ?",
                        rusqlite::params![unread_at, membership_id],
                    )?;
                    Ok(())
                })
                .await
                .unwrap();
            hold.release.wait().await;
        }
    );
    reply
}

#[tokio::test]
async fn a_mute_sends_room_read_only_when_its_write_cleared_unread() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let set_unread = |unread: bool| {
        a.db().write(move |tx| {
            let unread_at = unread.then(|| tx.now());
            tx.conn().execute(
                "UPDATE memberships SET unread_at = ?, involvement = 'everything' WHERE user_id = ? AND room_id = ?",
                rusqlite::params![unread_at, DAVID, ALL_TALK],
            )?;
            Ok(())
        })
    };
    let muted_row = |e: &api::SyncEvent| matches!(&e.payload, api::SyncPayload::SidebarRowUpserted(r) if r.room.id == ALL_TALK && r.membership.involvement == api::Involvement::Muted);
    let room_read = |e: &api::SyncEvent| matches!(&e.payload, api::SyncPayload::RoomRead(r) if r.room_id == ALL_TALK);
    let mut david = a.sign_in(DAVID).await;
    let mut sync = Sync::connect(addr, &david.cookie_header(), &[]).await;
    sync.welcome().await;

    // Unread when looked up, read by another tab before the write: nothing to clear.
    set_unread(true).await.unwrap();
    let muted: api::Membership = ok(&mute_while_unread_changes(&a, &mut david, false).await);
    assert_eq!(muted.unread_at, None);
    sync.until(muted_row, room_read).await;
    // Unmuting publishes the row again; no `room.read` arrives before it either.
    let path = format!("/api/v1/rooms/{ALL_TALK}/involvement");
    ok::<api::Membership>(
        &send(
            &mut david,
            Method::PUT,
            &path,
            json!({"involvement": "everything"}),
        )
        .await,
    );
    sync.until(
        |e| matches!(&e.payload, api::SyncPayload::SidebarRowUpserted(r) if r.room.id == ALL_TALK && r.membership.involvement == api::Involvement::Everything),
        room_read,
    )
    .await;

    // Read when looked up, unread again before the write: the mute clears it, and says so.
    set_unread(false).await.unwrap();
    let muted: api::Membership = ok(&mute_while_unread_changes(&a, &mut david, true).await);
    assert_eq!(muted.unread_at, None);
    sync.until(room_read, |_| false).await;
    sync.until(muted_row, |_| false).await;
    server.abort();
}
