//! Open-room preview and join over `/api/v1`, beside the other spa room tests.
//! A nonmember of an alive open room may preview it (no messages) and join it; closed, direct
//! and every other state stay 404, the same refusal `GET /api/v1/rooms/:id` already gives.

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use campfire_db::{CachedStatements, Membership, RoomType};
use serde_json::Value;

use super::api_tests::{Sync, app, get, json_body, parse, serve, tag};
use crate::controllers::presenters::test_support::{ALL_TALK, DAVID, HQ, KEVIN, TestApp};

fn oracle() -> Value {
    serde_json::from_str(include_str!("../../../../vectors/rooms_join.json")).unwrap()
}

async fn drop_member(app: &TestApp, room_id: i64, user_id: i64) {
    app.db()
        .write(move |tx| {
            tx.conn().execute_cached(
                "DELETE FROM memberships WHERE room_id=? AND user_id=?",
                (room_id, user_id),
            )?;
            Ok(())
        })
        .await
        .unwrap();
}

async fn member(app: &TestApp, room_id: i64, user_id: i64) -> Option<Membership> {
    app.db()
        .read(move |conn| Membership::find_by_room_and_user(conn, room_id, user_id))
        .await
        .unwrap()
}

fn post_join(path: &str) -> crate::controllers::presenters::test_support::Req {
    json_body(Method::POST, path, &serde_json::json!({}))
}

#[tokio::test]
async fn preview_is_allowed_for_an_open_room_and_refused_otherwise() {
    let app = app(true).await.expect("seed required");
    drop_member(&app, HQ, DAVID).await;
    let mut david = app.sign_in(DAVID).await;

    let show = david.send(get(&format!("/api/v1/rooms/{HQ}"))).await;
    assert_eq!(
        (show.status, tag(&show).as_str()),
        (StatusCode::NOT_FOUND, "NotFound"),
        "a nonmember still cannot read the room"
    );
    let messages = david
        .send(get(&format!("/api/v1/rooms/{HQ}/messages")))
        .await;
    assert_eq!(messages.status, StatusCode::NOT_FOUND);

    let reply = david
        .send(get(&format!("/api/v1/rooms/{HQ}/preview")))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let preview: api::OpenRoomPreview = parse(&reply);
    let body: Value = serde_json::from_slice(&reply.body).unwrap();
    assert_eq!(
        body.as_object().unwrap().keys().collect::<Vec<_>>(),
        ["id", "name", "memberCount"]
    );
    let count = app
        .db()
        .read(|conn| {
            let count: i64 = conn.query_row_cached(
                r#"SELECT COUNT(*) FROM "memberships" WHERE "memberships"."room_id" = ?"#,
                [HQ],
                |row| row.get(0),
            )?;
            Ok(count)
        })
        .await
        .unwrap();
    assert_eq!(
        (preview.id, preview.name.as_str(), preview.member_count),
        (HQ, "HQ", count)
    );
    assert!(count >= 1);

    for kind in [
        RoomType::Closed,
        RoomType::Direct,
        RoomType::Voice,
        RoomType::Stage,
        RoomType::Board,
    ] {
        app.db()
            .write(move |tx| {
                tx.conn()
                    .execute_cached("UPDATE rooms SET type=? WHERE id=?", (kind, HQ))?;
                Ok(())
            })
            .await
            .unwrap();
        let refused = david
            .send(get(&format!("/api/v1/rooms/{HQ}/preview")))
            .await;
        assert_eq!(
            (refused.status, tag(&refused).as_str()),
            (StatusCode::NOT_FOUND, "NotFound"),
            "{}",
            kind.class_name()
        );
        let join = david
            .write(post_join(&format!("/api/v1/rooms/{HQ}/join")))
            .await;
        assert_eq!(join.status, StatusCode::NOT_FOUND, "{}", kind.class_name());
        assert!(member(&app, HQ, DAVID).await.is_none());
    }

    app.db()
        .write(|tx| {
            tx.conn().execute_cached(
                "UPDATE rooms SET type='Rooms::Open', deleted_at=? WHERE id=?",
                (tx.now(), HQ),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    for id in [HQ, 9_999_999_999] {
        assert_eq!(
            david
                .send(get(&format!("/api/v1/rooms/{id}/preview")))
                .await
                .status,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            david
                .write(post_join(&format!("/api/v1/rooms/{id}/join")))
                .await
                .status,
            StatusCode::NOT_FOUND
        );
    }
    assert!(member(&app, HQ, DAVID).await.is_none());
}

#[tokio::test]
async fn join_creates_a_membership_is_idempotent_and_matches_classic() {
    let app = app(true).await.expect("seed required");
    drop_member(&app, HQ, DAVID).await;
    drop_member(&app, HQ, KEVIN).await;
    let audits_before = app
        .db()
        .read(|conn| {
            let count: i64 = conn.query_row_cached(
                "SELECT count(*) FROM audit_logs WHERE target_id=?",
                [HQ],
                |row| row.get(0),
            )?;
            Ok(count)
        })
        .await
        .unwrap();

    let mut david = app.sign_in(DAVID).await;
    let classic = david
        .write(crate::controllers::presenters::test_support::Req::new(
            Method::POST,
            &format!("/rooms/{HQ}/join"),
        ))
        .await;
    assert_eq!(
        classic.location(),
        oracle()["cases"]["join"]["location"].as_str()
    );
    let classic_membership = member(&app, HQ, DAVID).await.expect("classic join");
    assert_eq!(
        classic_membership.involvement.unwrap().name(),
        oracle()["cases"]["join"]["involvement"].as_str().unwrap()
    );
    assert!(!classic_membership.unread());

    let mut kevin = app.sign_in(KEVIN).await;
    let path = format!("/api/v1/rooms/{HQ}/join");
    let first = kevin.write(post_join(&path)).await;
    assert_eq!(first.status, StatusCode::OK, "{}", first.text());
    let joined: api::RoomJoin = parse(&first);
    assert_eq!(
        (
            joined.detail.room.id,
            joined.detail.membership.user_id,
            joined.detail.display_name.as_str(),
            joined.detail.membership.involvement,
            joined.row.room.id,
            joined.row.display_name.as_str(),
        ),
        (HQ, KEVIN, "HQ", api::Involvement::Mentions, HQ, "HQ",)
    );
    assert!(joined.detail.member_count >= 2);
    let created = member(&app, HQ, KEVIN).await.expect("api join");
    assert_eq!(created.id, joined.detail.membership.id);
    assert_eq!(created.involvement, classic_membership.involvement);
    assert!(!created.unread());

    let again = kevin.write(post_join(&path)).await;
    assert_eq!(again.status, StatusCode::OK, "{}", again.text());
    let repeated: api::RoomJoin = parse(&again);
    assert_eq!(repeated.detail.membership.id, created.id);
    assert_eq!(member(&app, HQ, KEVIN).await.unwrap().id, created.id);

    let audits_after = app
        .db()
        .read(|conn| {
            let count: i64 = conn.query_row_cached(
                "SELECT count(*) FROM audit_logs WHERE target_id=?",
                [HQ],
                |row| row.get(0),
            )?;
            Ok(count)
        })
        .await
        .unwrap();
    assert_eq!(audits_after, audits_before, "join writes no audit");

    let room = david.send(get(&format!("/api/v1/rooms/{HQ}"))).await;
    assert_eq!(room.status, StatusCode::OK, "{}", room.text());
}

#[tokio::test]
async fn join_publishes_the_sidebar_row_once() {
    let app = app(true).await.expect("seed required");
    drop_member(&app, HQ, DAVID).await;
    let (addr, server) = serve(&app).await;
    let mut david = app.sign_in(DAVID).await;
    let mut sync = Sync::connect(addr, &david.cookie_header(), &[]).await;
    sync.welcome().await;

    let joined = david
        .write(post_join(&format!("/api/v1/rooms/{HQ}/join")))
        .await;
    assert_eq!(joined.status, StatusCode::OK, "{}", joined.text());
    let event = sync
        .until(
            |event| {
                matches!(
                    &event.payload,
                    api::SyncPayload::SidebarRowUpserted(row) if row.room.id == HQ
                )
            },
            |_| false,
        )
        .await;
    let api::SyncPayload::SidebarRowUpserted(row) = event.payload else {
        unreachable!()
    };
    assert_eq!(row.display_name, "HQ");
    assert_eq!(row.membership.user_id, DAVID);
    assert_eq!(row.refresh_room, Some(true));

    let again = david
        .write(post_join(&format!("/api/v1/rooms/{HQ}/join")))
        .await;
    assert_eq!(again.status, StatusCode::OK, "{}", again.text());
    let read = david
        .write(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{ALL_TALK}/read"),
            &serde_json::json!({}),
        ))
        .await;
    assert_eq!(read.status, StatusCode::OK, "{}", read.text());
    sync.until(
        |event| {
            matches!(
                event.payload,
                api::SyncPayload::RoomRead(api::RoomRead { room_id: ALL_TALK })
            )
        },
        |event| {
            matches!(
                &event.payload,
                api::SyncPayload::SidebarRowUpserted(row) if row.room.id == HQ
            )
        },
    )
    .await;
    server.abort();
}
