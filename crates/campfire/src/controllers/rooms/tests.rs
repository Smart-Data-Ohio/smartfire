//! Request-level tests for the room controllers, against the `default` parity seed.

use axum::http::{Method, StatusCode};
use campfire_db::{Membership, Room, RoomType};

use crate::controllers::presenters::test_support::*;

#[tokio::test]
async fn inaccessible_rooms_redirect_home_with_an_alert() {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    let mut david = app.david();
    let reply = david.get(&format!("/rooms/{DIRECT_KEVIN_BENDER}")).await;
    assert_eq!(reply.status, StatusCode::FOUND);
    assert_eq!(reply.location(), Some(format!("http://campfire.test/app/r/{DIRECT_KEVIN_BENDER}").as_str()));
    assert_eq!(david.get(&format!("/api/v1/rooms/{DIRECT_KEVIN_BENDER}/messages")).await.status, StatusCode::NOT_FOUND);
    let reply = david.get("/rooms/nonsense").await;
    assert_eq!(reply.status, StatusCode::FOUND);
}

#[tokio::test]
async fn index_redirects_to_the_last_room() {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    let reply = app.david().get("/rooms").await;
    assert_eq!(reply.status, StatusCode::FOUND);
    assert!(
        reply
            .location()
            .unwrap()
            .starts_with("http://campfire.test/app/")
    );
    // Anonymous: off to sign in.
    let reply = app.anonymous().get("/rooms").await;
    assert_eq!(reply.location(), Some("http://campfire.test/session/new"));
}

#[tokio::test]
async fn undeclared_actions() {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    let mut david = app.david();
    assert_eq!(david.get("/rooms/new").await.status, StatusCode::NOT_FOUND);
    assert_eq!(
        david.get(&format!("/rooms/{ALL_TALK}/edit")).await.status,
        StatusCode::NOT_FOUND
    );
    let direct = david
        .get(&format!("/rooms/directs/{DIRECT_DAVID_JASON}"))
        .await;
    assert_eq!(direct.status, StatusCode::FOUND);
    let location = format!("http://campfire.test/rooms/{DIRECT_DAVID_JASON}");
    assert_eq!(direct.location(), Some(location.as_str()));
    let reply = david
        .write(Req::new(Method::DELETE, &format!("/rooms/opens/{HQ}")))
        .await;
    assert_eq!(reply.status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn open_rooms_are_created_edited_and_updated() {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    let mut david = app.david();
    let new = david.get("/rooms/opens/new").await;
    assert_eq!(new.status, StatusCode::FOUND);


    let created = david
        .write(Req::new(Method::POST, "/rooms/opens").form(&[("room[name]", "Watercooler")]))
        .await;
    assert_eq!(created.status, StatusCode::FOUND, "{}", created.text());
    let room_id: i64 = created
        .location()
        .unwrap()
        .rsplit('/')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let room = app
        .db()
        .read(move |conn| Room::find(conn, room_id))
        .await
        .unwrap();
    assert_eq!(
        (room.name.as_deref(), room.room_type),
        (Some("Watercooler"), RoomType::Open)
    );
    // Rooms::Open grants every active user after commit.
    let members = app
        .db()
        .read(move |conn| Membership::for_room(conn, room_id))
        .await
        .unwrap();
    assert!(members.len() > 3);

    assert_eq!(
        david
            .get(&format!("/rooms/opens/{room_id}/edit"))
            .await
            .status,
        StatusCode::FOUND
    );
    let shown = david.get(&format!("/rooms/opens/{room_id}")).await;
    assert_eq!(
        shown.location(),
        Some(format!("http://campfire.test/rooms/{room_id}").as_str())
    );

    let updated = david
        .write(
            Req::new(Method::PATCH, &format!("/rooms/closeds/{room_id}")).form(&[
                ("room[name]", "Private"),
                ("user_ids[]", &DAVID.to_string()),
            ]),
        )
        .await;
    assert_eq!(updated.status, StatusCode::FOUND, "{}", updated.text());
    let room = app
        .db()
        .read(move |conn| Room::find(conn, room_id))
        .await
        .unwrap();
    assert_eq!(
        (room.name.as_deref(), room.room_type),
        (Some("Private"), RoomType::Closed)
    );
    let members = app
        .db()
        .read(move |conn| Membership::for_room(conn, room_id))
        .await
        .unwrap();
    assert_eq!(
        members.iter().map(|m| m.user_id).collect::<Vec<_>>(),
        vec![DAVID]
    );

    let missing_param = david
        .write(Req::new(Method::POST, "/rooms/opens").form(&[("name", "x")]))
        .await;
    assert_eq!(missing_param.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn closed_rooms_are_created_with_the_selected_users() {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    let mut david = app.david();
    assert_eq!(david.get("/rooms/closeds/new").await.status, StatusCode::FOUND);
    let created = david
        .write(Req::new(Method::POST, "/rooms/closeds").form(&[
            ("room[name]", "Secret"),
            ("user_ids[]", &DAVID.to_string()),
            ("user_ids[]", &JASON.to_string()),
            ("user_ids[]", "999"),
        ]))
        .await;
    assert_eq!(created.status, StatusCode::FOUND, "{}", created.text());
    let room_id: i64 = created
        .location()
        .unwrap()
        .rsplit('/')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let mut members: Vec<i64> = app
        .db()
        .read(move |conn| Membership::for_room(conn, room_id))
        .await
        .unwrap()
        .iter()
        .map(|m| m.user_id)
        .collect();
    members.sort();
    assert_eq!(members, vec![DAVID, JASON]);
    assert_eq!(
        david
            .get(&format!("/rooms/closeds/{room_id}/edit"))
            .await
            .status,
        StatusCode::FOUND
    );
}

#[tokio::test]
async fn direct_rooms_are_found_or_created() {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    let mut david = app.david();
    assert_eq!(david.get("/rooms/directs/new").await.status, StatusCode::FOUND);
    let existing = david
        .write(Req::new(Method::POST, "/rooms/directs").form(&[("user_ids[]", &JASON.to_string())]))
        .await;
    assert_eq!(
        existing.location(),
        Some(format!("http://campfire.test/rooms/{DIRECT_DAVID_JASON}").as_str())
    );
    let created = david
        .write(Req::new(Method::POST, "/rooms/directs").form(&[("user_ids[]", &KEVIN.to_string())]))
        .await;
    let room_id: i64 = created
        .location()
        .unwrap()
        .rsplit('/')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let room = app
        .db()
        .read(move |conn| Room::find(conn, room_id))
        .await
        .unwrap();
    assert_eq!(room.room_type, RoomType::Direct);

    assert_eq!(
        david
            .get(&format!("/rooms/directs/{room_id}/edit"))
            .await
            .status,
        StatusCode::FOUND
    );
    let destroyed = david
        .write(Req::new(
            Method::DELETE,
            &format!("/rooms/directs/{room_id}"),
        ))
        .await;
    assert_eq!(destroyed.location(), Some("http://campfire.test/"));
    // The soft delete commits before the response; the DestroyJob it enqueues after commit may
    // already have removed the row by the time this reads it.
    let room = app
        .db()
        .read(move |conn| Room::find_by_id(conn, room_id))
        .await
        .unwrap();
    assert!(room.is_none_or(|room| room.deleted_at.is_some()));
}



#[tokio::test]
async fn involvement_is_shown_and_changed() {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    let mut david = app.david();
    let shown = david.get(&format!("/rooms/{ALL_TALK}/involvement")).await;
    assert_eq!(shown.status, StatusCode::FOUND);

    let updated = david
        .write(
            Req::new(Method::PATCH, &format!("/rooms/{ALL_TALK}/involvement"))
                .form(&[("involvement", "invisible")]),
        )
        .await;
    assert_eq!(
        updated.location(),
        Some(format!("http://campfire.test/rooms/{ALL_TALK}/involvement").as_str())
    );
    let membership = app
        .db()
        .read(|conn| Membership::find_by_room_and_user(conn, ALL_TALK, DAVID))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        membership.involvement,
        Some(campfire_db::Involvement::Invisible)
    );
    let invalid = david
        .write(
            Req::new(Method::PATCH, &format!("/rooms/{ALL_TALK}/involvement"))
                .form(&[("involvement", "loud")]),
        )
        .await;
    assert_eq!(invalid.status, StatusCode::INTERNAL_SERVER_ERROR);

    // Our fork requires a nonblank involvement.
    let missing = david
        .write(Req::new(
            Method::PATCH,
            &format!("/rooms/{ALL_TALK}/involvement"),
        ))
        .await;
    assert_eq!(missing.status, StatusCode::BAD_REQUEST);
    let membership = app
        .db()
        .read(|conn| Membership::find_by_room_and_user(conn, ALL_TALK, DAVID))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        membership.involvement,
        Some(campfire_db::Involvement::Invisible)
    );
}

#[tokio::test]
async fn rooms_are_destroyed_by_administrators() {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    let mut david = app.david();
    let reply = david
        .write(Req::new(Method::DELETE, &format!("/rooms/{QUIET_CORNER}")))
        .await;
    assert_eq!(reply.location(), Some("http://campfire.test/"));
    // The soft delete commits before the response; the DestroyJob it enqueues after commit may
    // already have removed the row by the time this reads it.
    let room = app
        .db()
        .read(|conn| Room::find_by_id(conn, QUIET_CORNER))
        .await
        .unwrap();
    assert!(room.is_none_or(|room| room.deleted_at.is_some()));
}

#[tokio::test]
async fn cross_site_writes_are_refused() {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    let mut david = app.david();
    // Another site's page can post with David's cookies, but can't read his authenticity token.
    let request = Req::new(Method::POST, "/rooms/opens").form(&[("room[name]", "x")]);
    assert_eq!(
        david.send(request).await.status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let forged = Req::new(Method::POST, "/rooms/opens")
        .form(&[("room[name]", "x"), ("authenticity_token", "forged")]);
    assert_eq!(
        david.send(forged).await.status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn the_last_room_cookie_is_set_only_when_it_changes() {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    let mut david = app.david();
    let last_room = |reply: &Reply| {
        reply
            .headers
            .get_all(axum::http::header::SET_COOKIE)
            .iter()
            .any(|c| c.to_str().unwrap().starts_with("last_room="))
    };
    assert!(last_room(&david.get(&format!("/rooms/opens/{HQ}")).await));
    assert!(
        !last_room(&david.get(&format!("/rooms/opens/{HQ}")).await),
        "the same room again"
    );
    assert!(last_room(&david.get(&format!("/rooms/opens/{ALL_TALK}")).await));
}
