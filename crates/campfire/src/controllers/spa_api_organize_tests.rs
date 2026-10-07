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
