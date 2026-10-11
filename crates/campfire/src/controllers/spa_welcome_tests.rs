//! The welcome card and server guide (U1, U4) over `/api/v1`: administrators replace the whole
//! configuration at `/api/v1/admin/welcome`, everyone else reads their own filtered copy at
//! `/api/v1/welcome`, and a room a viewer can't reach never reaches them, by name or by id.

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use campfire_db::{CachedStatements, Room};
use serde_json::{Value, json};

use super::api_tests::{Sync, app, get, json_body, parse, serve};
use crate::controllers::presenters::test_support::{
    ALL_TALK, DAVID, DIRECT_DAVID_JASON, HQ, KEVIN, TestApp,
};

/// A seeded room's id by name.
async fn room_named(app: &TestApp, name: &'static str) -> i64 {
    app.db()
        .read(move |conn| {
            Ok(Room::all(conn)?
                .into_iter()
                .find(|room| room.name.as_deref() == Some(name)))
        })
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("no room named {name}"))
        .id
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

fn put(body: &Value) -> crate::controllers::presenters::test_support::Req {
    json_body(Method::PUT, "/api/v1/admin/welcome", body)
}

fn error_fields(reply: &crate::controllers::presenters::test_support::Reply) -> Value {
    let envelope: api::ApiErrorResponse = parse(reply);
    let error = serde_json::to_value(&envelope.error).unwrap();
    assert_eq!(error["_tag"], "Validation", "{error}");
    error["fields"].clone()
}

#[tokio::test]
async fn only_administrators_read_or_write_the_configuration() {
    let a = app(true).await.expect("seed required");
    let mut kevin = a.sign_in(KEVIN).await;
    let body = json!({"message": "Mine now", "rooms": [], "guide": []});
    assert_eq!(
        kevin.send(get("/api/v1/admin/welcome")).await.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(kevin.write(put(&body)).await.status, StatusCode::FORBIDDEN);
    let mine: api::WorkspaceWelcome = parse(&kevin.send(get("/api/v1/welcome")).await);
    assert_eq!(mine.message_html, None);
    assert!(mine.rooms.is_empty() && mine.guide.is_empty());
    assert!(!mine.can_administer);

    let mut david = a.sign_in(DAVID).await;
    let empty: api::WelcomeAdmin = parse(&david.send(get("/api/v1/admin/welcome")).await);
    assert_eq!(empty.settings, api::WelcomeSettings::default());
    assert!(empty.choices.iter().any(|choice| choice.id == HQ));
    assert!(
        !empty
            .choices
            .iter()
            .any(|choice| choice.id == DIRECT_DAVID_JASON)
    );

    let reply = david
        .write(put(&json!({
            "message": "  Say **hi**  ",
            "rooms": [{"roomId": HQ, "emoji": "👋", "description": " Everyone's here "}],
            "guide": [{"title": "Hours", "emoji": null, "roomId": null, "body": "9 to *5*"}],
        })))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let saved: api::WelcomeAdmin = parse(&reply);
    assert_eq!(saved.settings.message, "Say **hi**");
    assert_eq!(saved.settings.rooms[0].description, "Everyone's here");
    let stored: api::WelcomeAdmin = parse(&david.send(get("/api/v1/admin/welcome")).await);
    assert_eq!(stored, saved);

    let seen: api::WorkspaceWelcome = parse(&kevin.send(get("/api/v1/welcome")).await);
    let message = seen.message_html.expect("a welcome message");
    assert!(message.contains("<strong>hi</strong>"), "{message}");
    assert_eq!(seen.rooms.len(), 1);
    assert!(seen.rooms[0].joined);
    assert!(
        seen.guide[0]
            .body_html
            .as_deref()
            .unwrap()
            .contains("<em>5</em>")
    );
}

#[tokio::test]
async fn invalid_configurations_are_refused_without_partial_writes() {
    let a = app(true).await.expect("seed required");
    // David leaves All Talk, a closed room: it is now one he can't see.
    drop_member(&a, ALL_TALK, DAVID).await;
    let mut david = a.sign_in(DAVID).await;
    let room = |id: i64| json!({"roomId": id, "emoji": null, "description": ""});
    let note = |title: &str, body: &str| json!({"title": title, "emoji": null, "roomId": null, "body": body});
    let cases = [
        (
            "message",
            json!({"message": "x".repeat(1001), "rooms": [], "guide": []}),
        ),
        (
            "rooms",
            json!({"message": "", "rooms": (1..=6).map(|_| room(HQ)).collect::<Vec<_>>(), "guide": []}),
        ),
        (
            "rooms",
            json!({"message": "", "rooms": [{"roomId": HQ, "emoji": null, "description": "x".repeat(101)}], "guide": []}),
        ),
        (
            "rooms",
            json!({"message": "", "rooms": [{"roomId": HQ, "emoji": "wave", "description": ""}], "guide": []}),
        ),
        (
            "rooms",
            json!({"message": "", "rooms": [room(DIRECT_DAVID_JASON)], "guide": []}),
        ),
        (
            "rooms",
            json!({"message": "", "rooms": [room(999_999_999)], "guide": []}),
        ),
        (
            "rooms",
            json!({"message": "", "rooms": [room(ALL_TALK)], "guide": []}),
        ),
        (
            "guide",
            json!({"message": "", "rooms": [], "guide": (0..=10).map(|_| note("Tip", "Body")).collect::<Vec<_>>()}),
        ),
        (
            "guide",
            json!({"message": "", "rooms": [], "guide": [note("", "Body")]}),
        ),
        (
            "guide",
            json!({"message": "", "rooms": [], "guide": [note("Tip", &"x".repeat(1001))]}),
        ),
        (
            "guide",
            json!({"message": "", "rooms": [], "guide": [{"title": "Both", "emoji": null, "roomId": HQ, "body": "Body"}]}),
        ),
        (
            "guide",
            json!({"message": "", "rooms": [], "guide": [{"title": "DM", "emoji": null, "roomId": DIRECT_DAVID_JASON, "body": null}]}),
        ),
    ];
    let before = a
        .db()
        .read(campfire_db::Account::first)
        .await
        .unwrap()
        .unwrap()
        .settings_json;
    for (field, body) in cases {
        let reply = david.write(put(&body)).await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{field}: {}",
            reply.text()
        );
        let fields = error_fields(&reply);
        assert!(!fields[field][0].as_str().unwrap().is_empty(), "{fields}");
        let after = a
            .db()
            .read(campfire_db::Account::first)
            .await
            .unwrap()
            .unwrap()
            .settings_json;
        assert_eq!(after, before);
    }
    // A room that doesn't exist and one David can't see are refused in the same words.
    let missing = error_fields(
        &david
            .write(put(
                &json!({"message": "", "rooms": [room(999_999_999)], "guide": []}),
            ))
            .await,
    );
    let hidden = error_fields(
        &david
            .write(put(
                &json!({"message": "", "rooms": [room(ALL_TALK)], "guide": []}),
            ))
            .await,
    );
    assert_eq!(missing, hidden);
}

#[tokio::test]
async fn each_viewer_sees_only_the_rooms_they_can_reach() {
    let a = app(true).await.expect("seed required");
    let pets = room_named(&a, "All Pets").await;
    let designers = room_named(&a, "Designers").await;
    let mut david = a.sign_in(DAVID).await;
    let reply = david
        .write(put(&json!({
            "message": "",
            "rooms": [
                {"roomId": HQ, "emoji": null, "description": "Company news"},
                {"roomId": ALL_TALK, "emoji": "🔒", "description": "Private chatter"},
                {"roomId": pets, "emoji": null, "description": "Pictures of pets"},
                {"roomId": designers, "emoji": null, "description": ""},
            ],
            "guide": [
                {"title": "Secret plans", "emoji": null, "roomId": ALL_TALK, "body": null},
                {"title": "Pets", "emoji": null, "roomId": pets, "body": null},
                {"title": "Hours", "emoji": null, "roomId": null, "body": "Nine to five"},
            ],
        })))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());

    let mut kevin = a.sign_in(KEVIN).await;
    let reply = kevin.send(get("/api/v1/welcome")).await;
    assert_eq!(reply.status, StatusCode::OK);
    // Kevin isn't in All Talk: nothing about it, not even its id, comes back.
    let text = reply.text();
    for leak in [
        "All Talk",
        "Private chatter",
        "Secret plans",
        "🔒",
        &ALL_TALK.to_string(),
    ] {
        assert!(!text.contains(leak), "{leak} leaked: {text}");
    }
    let seen: api::WorkspaceWelcome = parse(&reply);
    let rooms: Vec<(i64, bool)> = seen
        .rooms
        .iter()
        .map(|room| (room.room_id, room.joined))
        .collect();
    assert_eq!(rooms, vec![(HQ, true), (pets, false), (designers, true)]);
    assert_eq!(seen.message_html, None);
    let guide: Vec<(&str, Option<(i64, bool)>)> = seen
        .guide
        .iter()
        .map(|entry| {
            (
                entry.title.as_str(),
                entry.room.as_ref().map(|room| (room.id, room.joined)),
            )
        })
        .collect();
    assert_eq!(guide, vec![("Pets", Some((pets, false))), ("Hours", None)]);

    // David, a member of everything, sees it all.
    let all: api::WorkspaceWelcome = parse(&david.send(get("/api/v1/welcome")).await);
    assert_eq!(all.rooms.len(), 4);
    assert_eq!(all.guide.len(), 3);
    assert!(all.can_administer);

    // Once All Talk is deleted it goes for David too, and the configuration still saves.
    a.db()
        .write(|tx| {
            tx.conn().execute_cached(
                "UPDATE rooms SET deleted_at = CURRENT_TIMESTAMP WHERE id = ?",
                [ALL_TALK],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let all: api::WorkspaceWelcome = parse(&david.send(get("/api/v1/welcome")).await);
    assert_eq!(all.rooms.len(), 3);
}

#[tokio::test]
async fn a_saved_room_another_administrator_cant_see_stays_saveable() {
    let a = app(true).await.expect("seed required");
    let mut david = a.sign_in(DAVID).await;
    let body = json!({
        "message": "Hello",
        "rooms": [{"roomId": ALL_TALK, "emoji": null, "description": ""}],
        "guide": [],
    });
    assert_eq!(david.write(put(&body)).await.status, StatusCode::OK);
    drop_member(&a, ALL_TALK, DAVID).await;
    let reply = david.send(get("/api/v1/admin/welcome")).await;
    let admin: api::WelcomeAdmin = parse(&reply);
    assert_eq!(admin.settings.rooms[0].room_id, ALL_TALK);
    assert!(!admin.choices.iter().any(|choice| choice.id == ALL_TALK));
    let resaved = json!({
        "message": "Hello again",
        "rooms": [{"roomId": ALL_TALK, "emoji": null, "description": ""}],
        "guide": [],
    });
    let reply = david.write(put(&resaved)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
}

#[tokio::test]
async fn saving_tells_every_client_through_workspace_updated() {
    let a = app(true).await.expect("seed required");
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let kevin = a.sign_in(KEVIN).await;
    let mut sync = Sync::connect(addr, &kevin.cookie_header(), &[]).await;
    sync.welcome().await;
    let reply = david
        .write(put(&json!({"message": "Hello", "rooms": [], "guide": []})))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    sync.until(
        |event| matches!(&event.payload, api::SyncPayload::WorkspaceUpdated(_)),
        |_| false,
    )
    .await;
    server.abort();
}
