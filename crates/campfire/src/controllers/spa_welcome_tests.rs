//! The welcome card and server guide (U1, U4) over `/api/v1`: administrators patch the welcome or
//! the guide at `/api/v1/admin/welcome`, everyone else reads their own filtered copy at
//! `/api/v1/welcome`, and a room someone can't reach never reaches them, by name or by id, not
//! even an administrator.

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

fn patch(body: &Value) -> crate::controllers::presenters::test_support::Req {
    json_body(Method::PATCH, "/api/v1/admin/welcome", body)
}

async fn make_admin(app: &TestApp, user_id: i64) {
    app.db()
        .write(move |tx| {
            tx.conn()
                .execute_cached("UPDATE users SET role=1 WHERE id=?", [user_id])?;
            Ok(())
        })
        .await
        .unwrap();
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
    assert_eq!(
        kevin.write(patch(&body)).await.status,
        StatusCode::FORBIDDEN
    );
    let mine: api::WorkspaceWelcome = parse(&kevin.send(get("/api/v1/welcome")).await);
    assert_eq!(mine.message_html, None);
    assert!(mine.rooms.is_empty() && mine.guide.is_empty());
    assert!(!mine.can_administer);

    let mut david = a.sign_in(DAVID).await;
    let empty: api::WelcomeAdmin = parse(&david.send(get("/api/v1/admin/welcome")).await);
    assert_eq!(empty.message, "");
    assert!(empty.rooms.is_empty() && empty.guide.is_empty());
    assert!(empty.choices.iter().any(|choice| choice.id == HQ));
    assert!(
        !empty
            .choices
            .iter()
            .any(|choice| choice.id == DIRECT_DAVID_JASON)
    );

    let reply = david
        .write(patch(&json!({
            "message": "  Say **hi**  ",
            "rooms": [{"roomId": HQ, "emoji": "👋", "description": " Everyone's here "}],
            "guide": [{"title": "Hours", "emoji": null, "roomId": null, "body": "9 to *5*"}],
        })))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let saved: api::WelcomeAdmin = parse(&reply);
    assert_eq!(saved.message, "Say **hi**");
    assert_eq!(saved.rooms[0].room_id, Some(HQ));
    assert_eq!(saved.rooms[0].keep, None);
    assert_eq!(saved.rooms[0].description, "Everyone's here");
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
        let reply = david.write(patch(&body)).await;
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
            .write(patch(
                &json!({"message": "", "rooms": [room(999_999_999)], "guide": []}),
            ))
            .await,
    );
    let hidden = error_fields(
        &david
            .write(patch(
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
        .write(patch(&json!({
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
async fn another_administrator_keeps_a_room_they_cant_see_without_learning_it() {
    let a = app(true).await.expect("seed required");
    // Kevin, now an administrator too, isn't in All Talk (closed).
    make_admin(&a, KEVIN).await;
    let mut david = a.sign_in(DAVID).await;
    let reply = david
        .write(patch(&json!({
            "message": "Hello",
            "rooms": [
                {"roomId": HQ, "emoji": null, "description": "Company news"},
                {"roomId": ALL_TALK, "emoji": "🔒", "description": "Private chatter"},
            ],
            "guide": [
                {"title": "Secret plans", "emoji": null, "roomId": ALL_TALK, "body": null},
                {"title": "Hours", "emoji": null, "roomId": null, "body": "Nine to five"},
            ],
        })))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    // David's own copy names All Talk.
    let mine: api::WelcomeAdmin = parse(&reply);
    assert_eq!(mine.rooms[1].room_id, Some(ALL_TALK));
    assert_eq!(mine.rooms[1].keep, None);

    let mut kevin = a.sign_in(KEVIN).await;
    let reply = kevin.send(get("/api/v1/admin/welcome")).await;
    assert_eq!(reply.status, StatusCode::OK);
    let text = reply.text();
    for leak in ["All Talk", &ALL_TALK.to_string()] {
        assert!(!text.contains(leak), "{leak} leaked: {text}");
    }
    let seen: api::WelcomeAdmin = parse(&reply);
    assert_eq!(seen.rooms[0].room_id, Some(HQ));
    assert_eq!(seen.rooms[1].room_id, None);
    assert_eq!(seen.rooms[1].description, "Private chatter");
    let room_keep = seen.rooms[1].keep.clone().expect("a keep token");
    assert_eq!(seen.guide[0].room_id, None);
    let guide_keep = seen.guide[0].keep.clone().expect("a keep token");
    assert!(!seen.choices.iter().any(|choice| choice.id == ALL_TALK));

    // Naming the room outright is refused, in the same words as a room that doesn't exist: no
    // way to test whether some id is the hidden one.
    let named = |id: i64| json!({"rooms": [{"roomId": id, "emoji": null, "description": ""}]});
    let hidden = error_fields(&kevin.write(patch(&named(ALL_TALK))).await);
    let missing = error_fields(&kevin.write(patch(&named(999_999_999))).await);
    assert_eq!(hidden, missing);
    // A made-up token, or one row both naming and keeping a room, is refused too.
    for row in [
        json!({"roomId": null, "keep": "forged", "emoji": null, "description": ""}),
        json!({"roomId": HQ, "keep": room_keep, "emoji": null, "description": ""}),
    ] {
        let reply = kevin.write(patch(&json!({"rooms": [row]}))).await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            reply.text()
        );
    }

    // Kevin moves the hidden row to the top and rewords it; the reply hides it again.
    let reply = kevin
        .write(patch(&json!({
            "rooms": [
                {"roomId": null, "keep": room_keep, "emoji": "🤫", "description": "Ask to join"},
                {"roomId": HQ, "keep": null, "emoji": null, "description": "Company news"},
            ],
        })))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert!(
        !reply.text().contains(&ALL_TALK.to_string()),
        "{}",
        reply.text()
    );
    let after: api::WelcomeAdmin = parse(&david.send(get("/api/v1/admin/welcome")).await);
    let rooms: Vec<(Option<i64>, &str)> = after
        .rooms
        .iter()
        .map(|room| (room.room_id, room.description.as_str()))
        .collect();
    assert_eq!(
        rooms,
        vec![(Some(ALL_TALK), "Ask to join"), (Some(HQ), "Company news")]
    );
    assert_eq!(after.rooms[0].emoji.as_deref(), Some("🤫"));
    // The guide, not part of that save, is as David left it.
    assert_eq!(after.guide.len(), 2);
    assert_eq!(after.guide[0].room_id, Some(ALL_TALK));

    // Kevin keeps the guide's hidden link while retitling it, then removes it.
    let reply = kevin
        .write(patch(&json!({
            "guide": [{"title": "Plans", "emoji": null, "roomId": null, "keep": guide_keep, "body": null}],
        })))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let after: api::WelcomeAdmin = parse(&david.send(get("/api/v1/admin/welcome")).await);
    assert_eq!(after.guide.len(), 1);
    assert_eq!(
        (after.guide[0].title.as_str(), after.guide[0].room_id),
        ("Plans", Some(ALL_TALK))
    );
    let reply = kevin.write(patch(&json!({"guide": []}))).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let after: api::WelcomeAdmin = parse(&david.send(get("/api/v1/admin/welcome")).await);
    assert!(after.guide.is_empty());
    assert_eq!(after.rooms.len(), 2);
    assert_eq!(after.message, "Hello");

    // A token for a room no longer saved keeps nothing.
    let reply = kevin
        .write(patch(&json!({
            "guide": [{"title": "Plans", "emoji": null, "roomId": null, "keep": guide_keep, "body": null}],
        })))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    david
        .write(patch(
            &json!({"rooms": [{"roomId": HQ, "emoji": null, "description": ""}], "guide": []}),
        ))
        .await;
    let fields = error_fields(
        &kevin
            .write(patch(&json!({
                "rooms": [{"roomId": null, "keep": room_keep, "emoji": null, "description": ""}],
            })))
            .await,
    );
    assert!(
        fields["rooms"][0]
            .as_str()
            .unwrap()
            .contains("no longer saved"),
        "{fields}"
    );
}

#[tokio::test]
async fn each_section_saves_on_its_own() {
    let a = app(true).await.expect("seed required");
    let mut david = a.sign_in(DAVID).await;
    let welcome = json!({
        "message": "Hello",
        "rooms": [{"roomId": HQ, "emoji": null, "description": "News"}],
    });
    let guide = json!({"guide": [{"title": "Hours", "emoji": null, "roomId": null, "body": "Nine to five"}]});
    // Two saves sent from forms loaded at the same time: neither undoes the other.
    assert_eq!(david.write(patch(&welcome)).await.status, StatusCode::OK);
    assert_eq!(david.write(patch(&guide)).await.status, StatusCode::OK);
    let both: api::WelcomeAdmin = parse(&david.send(get("/api/v1/admin/welcome")).await);
    assert_eq!(both.message, "Hello");
    assert_eq!(both.rooms.len(), 1);
    assert_eq!(both.guide.len(), 1);
    assert_eq!(
        david
            .write(patch(&json!({"message": "Hi again"})))
            .await
            .status,
        StatusCode::OK
    );
    let after: api::WelcomeAdmin = parse(&david.send(get("/api/v1/admin/welcome")).await);
    assert_eq!(after.message, "Hi again");
    assert_eq!((after.rooms.len(), after.guide.len()), (1, 1));
    // A refused section leaves everything as it was.
    let reply = david
        .write(patch(&json!({"message": "Changed", "guide": [{"title": "", "emoji": null, "roomId": null, "body": "x"}]})))
        .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    let unchanged: api::WelcomeAdmin = parse(&david.send(get("/api/v1/admin/welcome")).await);
    assert_eq!(unchanged, after);
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
        .write(patch(
            &json!({"message": "Hello", "rooms": [], "guide": []}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    sync.until(
        |event| matches!(&event.payload, api::SyncPayload::WorkspaceUpdated(_)),
        |_| false,
    )
    .await;
    server.abort();
}
