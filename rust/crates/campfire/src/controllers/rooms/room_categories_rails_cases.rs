//! Every RoomCategoriesControllerTest declaration; scope and mutations use the actual domain.
use super::organization_rails_support::*;
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{Membership, RoomCategory};
async fn rows(app: &TestApp) -> Vec<RoomCategory> {
    app.db().read(|conn| RoomCategory::ordered_for_user(conn, DAVID)).await.unwrap()
}
fn redirect(reply: &Reply) {
    assert_eq!(reply.status, StatusCode::FOUND);
    assert_eq!(reply.location(), Some(format!("http://campfire.test{}", campfire_routes::user_sidebar()).as_str()));
}
#[tokio::test]
async fn index_lists_only_the_user_s_categories_in_order() {
    let app = setup().await;
    category(&app, DAVID, "Second", 2).await;
    category(&app, DAVID, "First", 1).await;
    category(&app, JASON, "Theirs", 0).await;
    let reply = app.david().get("/room_categories.json").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.json().as_array().unwrap().iter().map(|r| r["name"].as_str().unwrap()).collect::<Vec<_>>(), ["First", "Second"]);
}
#[tokio::test]
async fn create_adds_a_category_at_the_end() {
    let app = setup().await;
    category(&app, DAVID, "First", 1).await;
    let before = rows(&app).await.len();
    redirect(&app.david().write(Req::new(Method::POST, "/room_categories").form(&[("room_category[name]", "Team")])).await);
    let rows = rows(&app).await;
    assert_eq!(rows.len(), before + 1);
    let last = rows.last().unwrap();
    assert_eq!((last.name.as_str(), last.position, last.collapsed), ("Team", 2, false));
}
#[tokio::test]
async fn create_with_a_blank_name_changes_nothing_and_reloads_the_sidebar() {
    let app = setup().await;
    let before = rows(&app).await.len();
    redirect(&app.david().write(Req::new(Method::POST, "/room_categories").form(&[("room_category[name]", "")])).await);
    assert_eq!(rows(&app).await.len(), before);
}
#[tokio::test]
async fn update_renames_and_collapses() {
    let app = setup().await;
    let category = category(&app, DAVID, "Team", 1).await;
    redirect(&app.david().write(Req::new(Method::PATCH, &format!("/room_categories/{}", category.id)).form(&[("room_category[name]", "Squad"), ("room_category[collapsed]", "true")])).await);
    let category = app.db().read(move |conn| RoomCategory::find(conn, category.id)).await.unwrap();
    assert_eq!(category.name, "Squad");
    assert!(category.collapsed);
}
#[tokio::test]
async fn destroy_deletes_the_category_and_unassigns_its_rooms() {
    let app = setup().await;
    let category = category(&app, DAVID, "Team", 1).await;
    let id = category.id;
    app.db().write(move |tx| Membership::find_by_room_and_user(tx.conn(), DESIGNERS, DAVID)?.unwrap().update_category(tx, Some(id))).await.unwrap();
    redirect(&app.david().write(Req::new(Method::DELETE, &format!("/room_categories/{id}"))).await);
    assert!(app.db().read(move |conn| RoomCategory::find_by_id(conn, id)).await.unwrap().is_none());
    assert_eq!(membership(&app, DESIGNERS).await.room_category_id, None);
}
#[tokio::test]
async fn another_user_s_categories_are_not_found() {
    let app = setup().await;
    let category = category(&app, JASON, "Theirs", 1).await;
    for method in [Method::PATCH, Method::DELETE] {
        assert_eq!(app.david().write(Req::new(method, &format!("/room_categories/{}", category.id)).form(&[("room_category[name]", "Mine")])).await.status, StatusCode::NOT_FOUND);
    }
    assert_eq!(app.db().read(move |conn| RoomCategory::find(conn, category.id)).await.unwrap().name, "Theirs");
}
