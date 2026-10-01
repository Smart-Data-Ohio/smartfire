//! All five declarations for assigning a viewer's channels to their categories.
use super::organization_rails_support::*;
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::Membership;
fn request(room: i64, category: &str) -> Req {
    Req::new(Method::PATCH, &format!("/rooms/{room}/category_assignment.json")).form(&[("room_category_id", category)])
}
#[tokio::test]
async fn update_assigns_the_channel_to_the_category() {
    let app = setup().await;
    let category = category(&app, DAVID, "Team", 1).await;
    assert_eq!(app.david().write(request(DESIGNERS, &category.id.to_string())).await.status, StatusCode::OK);
    assert_eq!(membership(&app, DESIGNERS).await.room_category_id, Some(category.id));
}
#[tokio::test]
async fn update_with_a_blank_category_unassigns_the_channel() {
    let app = setup().await;
    let category = category(&app, DAVID, "Team", 1).await;
    app.db().write(move |tx| Membership::find_by_room_and_user(tx.conn(), DESIGNERS, DAVID)?.unwrap().update_category(tx, Some(category.id))).await.unwrap();
    assert_eq!(app.david().write(request(DESIGNERS, "")).await.status, StatusCode::OK);
    assert_eq!(membership(&app, DESIGNERS).await.room_category_id, None);
}
#[tokio::test]
async fn update_rejects_another_user_s_category() {
    let app = setup().await;
    let other = category(&app, JASON, "Theirs", 1).await;
    assert_eq!(app.david().write(request(DESIGNERS, &other.id.to_string())).await.status, StatusCode::NOT_FOUND);
    assert_eq!(membership(&app, DESIGNERS).await.room_category_id, None);
}
#[tokio::test]
async fn update_rejects_rooms_that_are_not_channels() {
    let app = setup().await;
    let category = category(&app, DAVID, "Team", 1).await;
    assert_eq!(app.david().write(request(DIRECT_DAVID_JASON, &category.id.to_string())).await.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(membership(&app, DIRECT_DAVID_JASON).await.room_category_id, None);
}
#[tokio::test]
async fn categories_in_a_room_the_user_cannot_access_are_not_found() {
    let app = setup().await;
    let category = category(&app, DAVID, "Team", 1).await;
    let secret = secret(&app).await;
    assert_eq!(app.david().write(request(secret.id, &category.id.to_string())).await.status, StatusCode::NOT_FOUND);
}
