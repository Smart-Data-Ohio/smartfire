//! Each declaration in Rooms::FavoritesControllerTest, including denied access before writes.
use super::organization_rails_support::*;
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::Membership;
fn request(method: Method, room: i64, position: Option<&str>) -> Req {
    let req = Req::new(method, &format!("/rooms/{room}/favorite.json"));
    match position { Some(position) => req.form(&[("position", position)]), None => req }
}
async fn ids(app: &TestApp) -> Vec<i64> {
    app.db().read(|conn| Ok(Membership::favorites_for_user(conn, DAVID)?.into_iter().map(|m| m.id).collect())).await.unwrap()
}
#[tokio::test]
async fn create_favourites_the_room_at_the_end() {
    let app = setup().await;
    favorite(&app, HQ).await;
    assert_eq!(app.david().write(request(Method::POST, DESIGNERS, None)).await.status, StatusCode::OK);
    let membership = membership(&app, DESIGNERS).await;
    assert!(membership.favorited());
    assert_eq!(membership.favorite_position, Some(1));
}
#[tokio::test]
async fn create_is_idempotent() {
    let app = setup().await;
    favorite(&app, DESIGNERS).await;
    let before = ids(&app).await;
    assert_eq!(app.david().write(request(Method::POST, DESIGNERS, None)).await.status, StatusCode::OK);
    assert_eq!(ids(&app).await, before);
}
#[tokio::test]
async fn destroy_unfavourites_the_room() {
    let app = setup().await;
    favorite(&app, DESIGNERS).await;
    assert_eq!(app.david().write(request(Method::DELETE, DESIGNERS, None)).await.status, StatusCode::OK);
    assert!(!membership(&app, DESIGNERS).await.favorited());
}
#[tokio::test]
async fn update_moves_the_favourite_to_an_absolute_position() {
    let app = setup().await;
    for room in [DESIGNERS, HQ, PETS] { favorite(&app, room).await; }
    assert_eq!(app.david().write(request(Method::PATCH, PETS, Some("0"))).await.status, StatusCode::OK);
    let mut expected = vec![];
    for room in [PETS, DESIGNERS, HQ] { expected.push(membership(&app, room).await.id); }
    assert_eq!(ids(&app).await, expected);
}
#[tokio::test]
async fn update_clamps_out_of_range_positions() {
    let app = setup().await;
    favorite(&app, DESIGNERS).await;
    assert_eq!(app.david().write(request(Method::PATCH, DESIGNERS, Some("99"))).await.status, StatusCode::OK);
    assert_eq!(membership(&app, DESIGNERS).await.favorite_position, Some(0));
}
#[tokio::test]
async fn favourites_in_a_room_the_user_cannot_access_are_not_found() {
    let app = setup().await;
    let secret = secret(&app).await;
    let before = ids(&app).await;
    for method in [Method::POST, Method::DELETE, Method::PATCH] {
        assert_eq!(app.david().write(request(method, secret.id, Some("0"))).await.status, StatusCode::NOT_FOUND);
    }
    assert_eq!(ids(&app).await, before);
}
