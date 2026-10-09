//! GitHub repository subscriptions and inbound email on `/api/v1`, with the classic
//! authorization and validation. Missing seeds fail rather than skip.
use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use campfire_db::{CachedStatements, Room, RoomType};
use serde_json::{Value, json};

use super::admin_tests::{error, get, parse, write};
use crate::controllers::presenters::test_support::{DAVID, DIRECT_DAVID_JASON, HQ, KEVIN, TestApp};

const PLAIN_MEMBER: i64 = 773523953;

async fn app(extra: &[(&str, &str)]) -> TestApp {
    let mut values = vec![("SPA_ENABLED", "1")];
    values.extend_from_slice(extra);
    TestApp::boot_frozen_with_env(&values)
        .await
        .expect("frozen seeds required")
        .without_job_runner()
        .await
}

async fn room(app: &TestApp, kind: RoomType, name: &str, members: &[i64]) -> i64 {
    let name = name.to_string();
    let members = members.to_vec();
    app.db()
        .write(move |tx| Ok(Room::create_for(tx, kind, Some(&name), DAVID, &members)?.id))
        .await
        .unwrap()
}

async fn token(app: &TestApp, id: i64) -> Option<String> {
    app.db()
        .read(move |conn| Ok(Room::find(conn, id)?.inbound_email_token))
        .await
        .unwrap()
}

async fn subscriptions(app: &TestApp, room_id: i64) -> Vec<(String, String)> {
    app.db()
        .read(move |conn| {
            let mut statement = conn.prepare(
                "SELECT owner, repo FROM github_repository_subscriptions WHERE room_id=? ORDER BY owner, repo",
            )?;
            let rows = statement
                .query_map([room_id], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        })
        .await
        .unwrap()
}

async fn github_bot_in(app: &TestApp, room_id: i64) -> bool {
    app.db()
        .read(move |conn| {
            Ok(conn.query_row_cached(
                "SELECT EXISTS(SELECT 1 FROM memberships WHERE room_id=? AND user_id=(SELECT id FROM users WHERE role=2 AND status=0 AND name='GitHub' ORDER BY id LIMIT 1))",
                [room_id],
                |row| row.get(0),
            )?)
        })
        .await
        .unwrap()
}

fn subscribe_body(name: &str, events: &[&str], skip: bool) -> Value {
    json!({
        "fullName": name,
        "events": events,
        "skipAccessCheck": skip,
    })
}

#[tokio::test]
async fn github_subscriptions_follow_classic_authorization_and_validation() {
    let app = app(&[]).await;
    let mut david = app.sign_in(DAVID).await;
    let mut kevin = app.sign_in(KEVIN).await;
    david.authenticity_token().await;
    kevin.authenticity_token().await;

    let closed = room(&app, RoomType::Closed, "Integrations", &[DAVID, KEVIN]).await;
    let voice = room(&app, RoomType::Voice, "Integrations voice", &[DAVID]).await;
    let stage = room(&app, RoomType::Stage, "Integrations stage", &[DAVID]).await;
    let board = room(&app, RoomType::Board, "Integrations board", &[DAVID]).await;
    let kevin_room = app
        .db()
        .write(|tx| {
            Ok(Room::create_for(tx, RoomType::Closed, Some("Kevin's room"), KEVIN, &[KEVIN])?.id)
        })
        .await
        .unwrap();

    let path = format!("/api/v1/rooms/{closed}/github_subscriptions");
    let forbidden = kevin.send(get(&path)).await;
    assert_eq!(
        forbidden.status,
        StatusCode::FORBIDDEN,
        "{}",
        forbidden.text()
    );
    assert_eq!(error(&forbidden)["_tag"], "Forbidden");

    let mut outsider = app.sign_in(PLAIN_MEMBER).await;
    outsider.authenticity_token().await;
    // A member of nothing here: set_room answers 404, including for an administrator.
    let private = room(&app, RoomType::Closed, "Integrations private", &[DAVID]).await;
    let missing = outsider
        .send(get(&format!(
            "/api/v1/rooms/{private}/github_subscriptions"
        )))
        .await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND, "{}", missing.text());

    for id in [DIRECT_DAVID_JASON, closed] {
        let reply = david
            .send(get(&format!("/api/v1/rooms/{id}/github_subscriptions")))
            .await;
        if id == DIRECT_DAVID_JASON {
            assert_eq!(reply.status, StatusCode::NOT_FOUND, "{}", reply.text());
        } else {
            assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
            let list: api::GithubSubscriptionList = parse(&reply);
            assert!(list.subscriptions.is_empty());
            assert!(list.administrator);
            assert_eq!(list.events.len(), 6);
            assert!(
                list.events
                    .iter()
                    .any(|event| event.key == "opened" && event.selected_by_default)
            );
            assert!(
                list.events
                    .iter()
                    .any(|event| event.key == "closed" && !event.selected_by_default)
            );
        }
    }

    let unlinked = write(
        &mut kevin,
        Method::POST,
        &format!("/api/v1/rooms/{kevin_room}/github_subscriptions"),
        subscribe_body("rails/rails", &["opened"], true),
    )
    .await;
    assert_eq!(
        unlinked.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        unlinked.text()
    );
    let failure = error(&unlinked);
    assert_eq!(failure["_tag"], "Validation");
    let message = failure["message"].as_str().unwrap();
    assert!(message.starts_with("Could not subscribe:"), "{message}");
    assert!(message.contains("rails/rails"), "{message}");
    assert!(!message.contains("Administrators may"), "{message}");
    assert!(
        failure["fields"]["github"][0] == "not_linked"
            || failure["fields"]["github"][0] == "unreadable"
    );
    assert!(subscriptions(&app, kevin_room).await.is_empty());

    let blank = write(
        &mut david,
        Method::POST,
        &path,
        subscribe_body("  ", &[], false),
    )
    .await;
    assert_eq!(
        blank.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        blank.text()
    );
    assert!(
        error(&blank)["message"]
            .as_str()
            .unwrap()
            .starts_with("Could not subscribe:")
    );
    assert!(
        error(&blank)["message"]
            .as_str()
            .unwrap()
            .contains("can't be blank")
    );

    let unknown = write(
        &mut david,
        Method::POST,
        &path,
        subscribe_body("rails/rails", &["deployed"], true),
    )
    .await;
    assert_eq!(
        unknown.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        unknown.text()
    );
    assert!(
        error(&unknown)["message"]
            .as_str()
            .unwrap()
            .contains("must be a subset")
    );

    let created = write(
        &mut david,
        Method::POST,
        &path,
        subscribe_body("Rails/Rails", &[], true),
    )
    .await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.text());
    let created: api::GithubSubscription = parse(&created);
    assert_eq!(created.full_name, "rails/rails");
    assert_eq!(
        created.events,
        ["opened", "merged", "review_requested", "checks_failed"]
    );
    assert!(github_bot_in(&app, closed).await);
    assert_eq!(
        subscriptions(&app, closed).await,
        [("rails".into(), "rails".into())]
    );

    let duplicate = write(
        &mut david,
        Method::POST,
        &path,
        subscribe_body("rails/rails", &["opened"], true),
    )
    .await;
    assert_eq!(
        duplicate.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        duplicate.text()
    );
    assert_eq!(
        error(&duplicate)["message"],
        "Could not subscribe: Owner has already been taken."
    );

    let item = format!("{path}/{}", created.id);
    let updated = write(
        &mut david,
        Method::PATCH,
        &item,
        json!({"events":["closed"]}),
    )
    .await;
    assert_eq!(updated.status, StatusCode::OK, "{}", updated.text());
    let updated: api::GithubSubscription = parse(&updated);
    assert_eq!(updated.events, ["closed"]);

    let empty = write(&mut david, Method::PATCH, &item, json!({"events":[]})).await;
    assert_eq!(
        empty.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        empty.text()
    );
    assert!(
        error(&empty)["message"]
            .as_str()
            .unwrap()
            .starts_with("Could not update:")
    );
    assert!(
        error(&empty)["message"]
            .as_str()
            .unwrap()
            .contains("at least one event")
    );

    let other = format!(
        "/api/v1/rooms/{private}/github_subscriptions/{}",
        created.id
    );
    let cross = write(&mut david, Method::DELETE, &other, Value::Null).await;
    assert_eq!(cross.status, StatusCode::NOT_FOUND, "{}", cross.text());

    for id in [voice, stage, board] {
        let reply = write(
            &mut david,
            Method::POST,
            &format!("/api/v1/rooms/{id}/github_subscriptions"),
            subscribe_body("campfire/campfire", &["merged"], true),
        )
        .await;
        assert_eq!(reply.status, StatusCode::CREATED, "{id}: {}", reply.text());
        assert!(github_bot_in(&app, id).await);
    }

    let removed = write(&mut david, Method::DELETE, &item, Value::Null).await;
    assert_eq!(removed.status, StatusCode::OK, "{}", removed.text());
    let removed: api::GithubSubscription = parse(&removed);
    assert_eq!(removed.full_name, "rails/rails");
    assert!(subscriptions(&app, closed).await.is_empty());
    assert!(!github_bot_in(&app, closed).await);

    app.db()
        .write(move |tx| {
            tx.conn().execute_cached(
                "UPDATE rooms SET deleted_at=? WHERE id=?",
                rusqlite::params![tx.now(), closed],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let deleted = david.send(get(&path)).await;
    assert_eq!(deleted.status, StatusCode::NOT_FOUND, "{}", deleted.text());
}

#[tokio::test]
async fn inbound_email_follows_classic_authorization_and_rotation() {
    let app = app(&[("INBOUND_EMAIL_DOMAIN", "mail.campfire.test")]).await;
    let mut david = app.sign_in(DAVID).await;
    david.authenticity_token().await;
    let closed = room(&app, RoomType::Closed, "Inbound", &[DAVID, KEVIN]).await;
    let board = room(&app, RoomType::Board, "Inbound board", &[DAVID]).await;
    let path = format!("/api/v1/rooms/{closed}/inbound_email");

    let mut kevin = app.sign_in(KEVIN).await;
    kevin.authenticity_token().await;
    let forbidden = write(&mut kevin, Method::POST, &path, Value::Null).await;
    assert_eq!(
        forbidden.status,
        StatusCode::FORBIDDEN,
        "{}",
        forbidden.text()
    );
    assert_eq!(token(&app, closed).await, None);

    for id in [DIRECT_DAVID_JASON, board] {
        let reply = david
            .send(get(&format!("/api/v1/rooms/{id}/inbound_email")))
            .await;
        assert_eq!(
            reply.status,
            StatusCode::NOT_FOUND,
            "{id}: {}",
            reply.text()
        );
    }

    let before = david.send(get(&path)).await;
    assert_eq!(before.status, StatusCode::OK, "{}", before.text());
    let before: api::InboundEmail = parse(&before);
    assert!(before.enabled);
    assert_eq!(before.address, None);

    let mut previous = None;
    for _ in 0..2 {
        let reply = write(&mut david, Method::POST, &path, Value::Null).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        let body: api::InboundEmail = parse(&reply);
        let stored = token(&app, closed).await.unwrap();
        assert_eq!(stored.len(), 32);
        assert!(stored.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(
            body.address.as_deref(),
            Some(format!("room-{stored}@mail.campfire.test").as_str())
        );
        assert_ne!(previous.as_ref(), Some(&stored));
        previous = Some(stored);
    }
}

#[tokio::test]
async fn inbound_email_without_a_domain_still_rotates_and_hides_the_address() {
    let app = app(&[]).await;
    let mut david = app.sign_in(DAVID).await;
    david.authenticity_token().await;
    let path = format!("/api/v1/rooms/{HQ}/inbound_email");
    let reply = write(&mut david, Method::POST, &path, Value::Null).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let body: api::InboundEmail = parse(&reply);
    assert!(!body.enabled);
    assert_eq!(body.address, None);
    assert!(token(&app, HQ).await.is_some());
}
