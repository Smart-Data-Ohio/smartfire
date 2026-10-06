//! One Rust test per declaration in rooms/inbound_email_addresses_controller_test.rb.
//! Seeded private rooms use the same permission/membership conditions, with a non-admin
//! creator as an additional discrimination from the Rails fixture's admin creator.
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{CachedStatements, Room, RoomType};
async fn setup(domain: bool) -> (TestApp, i64) {
    let values = if domain {
        vec![("INBOUND_EMAIL_DOMAIN", "mail.test")]
    } else {
        Vec::new()
    };
    let app = TestApp::boot_frozen_with_env(&values)
        .await
        .expect("seed required");
    let id = app
        .db()
        .write(|tx| {
            Ok(Room::create_for(
                tx,
                RoomType::Closed,
                Some("Designers"),
                KEVIN,
                &[DAVID, JASON, KEVIN],
            )?
            .id)
        })
        .await
        .unwrap();
    (app, id)
}
async fn token(app: &TestApp, id: i64) -> Option<String> {
    app.db()
        .read(move |conn| Ok(Room::find(conn, id)?.inbound_email_token))
        .await
        .unwrap()
}
fn create(id: i64) -> Req {
    Req::new(Method::POST, &format!("/rooms/{id}/inbound_email_address"))
}
#[tokio::test]
async fn an_administrator_can_create_the_room_address() {
    let (app, id) = setup(false).await;
    let reply = app.sign_in(JASON).await.write(create(id)).await;
    assert_eq!(reply.status, StatusCode::FOUND);
    assert_eq!(
        reply.location(),
        Some(format!("http://campfire.test/rooms/{id}/edit").as_str())
    );
    assert!(token(&app, id).await.is_some());
}
#[tokio::test]
async fn the_creator_can_rotate_the_address() {
    let (app, id) = setup(false).await;
    let previous = app
        .db()
        .write(move |tx| Room::find(tx.conn(), id)?.regenerate_inbound_email_token(tx))
        .await
        .unwrap();
    let reply = app.sign_in(KEVIN).await.write(create(id)).await;
    assert_eq!(reply.status, StatusCode::FOUND);
    assert_ne!(token(&app, id).await.as_deref(), Some(previous.as_str()));
}
#[tokio::test]
async fn a_plain_member_is_forbidden() {
    let (app, id) = setup(false).await;
    let jz = 773523953;
    app.db()
        .write(move |tx| Room::find(tx.conn(), id)?.grant_to(tx, &[jz]))
        .await
        .unwrap();
    let reply = app.sign_in(jz).await.write(create(id)).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    assert_eq!(token(&app, id).await, None);
}
#[tokio::test]
async fn a_non_member_gets_a_404() {
    let (app, id) = setup(false).await;
    app.db()
        .write(move |tx| {
            tx.conn().execute_cached(
                "DELETE FROM memberships WHERE room_id=? AND user_id=?",
                [id, DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        app.david().write(create(id)).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(token(&app, id).await, None);
}
#[tokio::test]
async fn direct_rooms_never_have_an_address() {
    let (app, _) = setup(false).await;
    assert_eq!(
        app.david().write(create(DIRECT_DAVID_JASON)).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(token(&app, DIRECT_DAVID_JASON).await, None);
}
#[tokio::test]
async fn board_rooms_never_have_an_address() {
    let (app, _) = setup(false).await;
    let id = app
        .db()
        .write(|tx| Ok(Room::create_for(tx, RoomType::Board, Some("Launch"), DAVID, &[DAVID])?.id))
        .await
        .unwrap();
    assert_eq!(
        app.david().write(create(id)).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(token(&app, id).await, None);
}
#[tokio::test]
async fn the_edit_page_shows_the_address_once_created() {
    let (app, id) = setup(true).await;
    let token = app
        .db()
        .write(move |tx| Room::find(tx.conn(), id)?.regenerate_inbound_email_token(tx))
        .await
        .unwrap();
    let page = app.david().get(&format!("/rooms/closeds/{id}/edit")).await;
    assert_eq!(page.status, StatusCode::OK);
    assert!(page.text().contains(&format!("room-{token}@mail.test")));
}
#[tokio::test]
async fn the_edit_page_explains_the_missing_domain() {
    let (app, id) = setup(false).await;
    let page = app.david().get(&format!("/rooms/closeds/{id}/edit")).await;
    assert_eq!(page.status, StatusCode::OK);
    assert!(page.text().contains("INBOUND_EMAIL_DOMAIN"));
}
