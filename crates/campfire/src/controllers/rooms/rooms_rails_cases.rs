//! Individually executed RoomController cases. Owner-rendered thread/message cases and
//! the deliberately different atomic queue-failure contract remain explicitly deferred.
use super::directs_rails_cases::{group, ids, note, pending_destroy};
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{CachedStatements, Membership, Room, RoomType};
const JZ: i64 = 773523953;

async fn setup() -> TestApp {
    TestApp::boot_frozen().await.expect("seed required")
}
async fn closed(app: &TestApp) -> i64 {
    app.db()
        .write(|tx| {
            Ok(Room::create_for(
                tx,
                RoomType::Closed,
                Some("Designers"),
                DAVID,
                &[DAVID, KEVIN, JZ],
            )?
            .id)
        })
        .await
        .unwrap()
}
fn destroy(id: i64) -> Req {
    Req::new(Method::DELETE, &format!("/rooms/{id}"))
}
fn leave(id: i64) -> Req {
    Req::new(Method::DELETE, &format!("/rooms/{id}/leave"))
}
fn root(reply: &Reply) {
    assert_eq!(reply.status, StatusCode::FOUND);
    assert_eq!(reply.location(), Some("http://campfire.test/"));
}
#[tokio::test]
async fn destroy_removes_the_room_from_everyone_and_enqueues_its_deletion() {


    let app = setup().await.without_job_runner().await;
    let id = closed(&app).await;
    let mut david = app.david();
    root(&david.write(destroy(id)).await);
    pending_destroy(&app, id).await;

}
#[tokio::test]
async fn destroy_stamps_the_sweep_claim() {
    let app = setup().await.without_job_runner().await;
    let id = closed(&app).await;
    root(&app.david().write(destroy(id)).await);
    assert!(
        app.db()
            .read(move |conn| Ok(Room::find(conn, id)?.destroy_enqueued_at.is_some()))
            .await
            .unwrap()
    );
}
#[tokio::test]
async fn destroyed_room_is_inaccessible_while_deletion_is_pending() {
    let app = setup().await;
    let id = closed(&app).await;
    let mut david = app.david();
    root(&david.write(destroy(id)).await);
    root(&david.classic_page(&format!("/rooms/{id}")).await);
}
#[tokio::test]
async fn destroy_finishes_through_the_enqueued_job() {
    let app = setup().await.without_job_runner().await;
    let id = closed(&app).await;
    root(&app.david().write(destroy(id)).await);
    pending_destroy(&app, id).await;
    campfire_db::models::room_delete::perform_with_config(app.db(), id, Default::default())
        .await
        .unwrap();
    assert!(
        app.db()
            .read(move |conn| Room::find_by_id(conn, id))
            .await
            .unwrap()
            .is_none()
    );
}
#[tokio::test]
async fn destroy_only_allowed_for_creators_or_those_who_can_administer() {
    let app = setup().await.without_job_runner().await;
    let id = closed(&app).await;
    let mut jz = app.sign_in(JZ).await;
    assert_eq!(jz.write(destroy(id)).await.status, StatusCode::FORBIDDEN);
    assert!(
        app.db()
            .read(move |conn| Ok(Room::find(conn, id)?.deleted_at.is_none()))
            .await
            .unwrap()
    );
    app.db()
        .write(move |tx| {
            tx.conn()
                .execute_cached("UPDATE rooms SET creator_id=? WHERE id=?", (JZ, id))?;
            Ok(())
        })
        .await
        .unwrap();
    root(&jz.write(destroy(id)).await);
    pending_destroy(&app, id).await;
}
#[tokio::test]
async fn destroy_answers_the_sidebar_menu_with_json_and_no_redirect() {
    let app = setup().await.without_job_runner().await;
    let id = closed(&app).await;
    let reply = app
        .david()
        .write(Req::new(Method::DELETE, &format!("/rooms/{id}.json")))
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.location(), None);
    assert_eq!(
        reply.json(),
        serde_json::json!({"deleted":true,"room_id":id})
    );
    pending_destroy(&app, id).await;
}
#[tokio::test]
async fn destroy_announces_the_deleted_room() {
    let app = setup().await;
    let id = closed(&app).await;
    let reply = app.david().write(destroy(id)).await;
    root(&reply);
    assert_eq!(
        super::direct_selection_tests::next_flash(&app, &reply, &mut None)["notice"],
        "Deleted #Designers"
    );
}
#[tokio::test]
async fn destroy_of_a_group_dm_is_refused_for_non_administrators() {
    let app = setup().await;
    let id = group(&app, &[DAVID, KEVIN, JZ], JZ).await;
    assert_eq!(
        app.sign_in(JZ).await.write(destroy(id)).await.status,
        StatusCode::FORBIDDEN
    );
    assert!(
        app.db()
            .read(move |conn| Ok(Room::find(conn, id)?.deleted_at.is_none()))
            .await
            .unwrap()
    );
    assert_eq!(ids(&app, id).await.len(), 3);
}
#[tokio::test]
async fn leave_removes_only_your_membership_and_the_room_keeps_working() {
    let app = setup().await;
    let id = closed(&app).await;
    let mut jz = app.sign_in(JZ).await;
    root(&jz.write(leave(id)).await);
    assert!(!ids(&app, id).await.contains(&JZ));
    assert!(
        app.db()
            .read(move |conn| Ok(Room::find(conn, id)?.deleted_at.is_none()))
            .await
            .unwrap()
    );
    root(&jz.classic_page(&format!("/rooms/{id}")).await);
    assert_eq!(
        app.david().classic_page(&format!("/rooms/{id}")).await.status,
        StatusCode::FOUND
    );
}
#[tokio::test]
async fn leave_answers_the_sidebar_menu_with_json_and_no_redirect() {
    let app = setup().await;
    let id = closed(&app).await;
    let reply = app
        .sign_in(JZ)
        .await
        .write(Req::new(Method::DELETE, &format!("/rooms/{id}/leave.json")))
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.location(), None);
    assert_eq!(reply.json(), serde_json::json!({"left":true,"room_id":id}));
    assert!(!ids(&app, id).await.contains(&JZ));
}
#[tokio::test]
async fn the_last_member_out_does_not_delete_the_room() {
    let app = setup().await;
    let id = app
        .db()
        .write(|tx| Ok(Room::create_for(tx, RoomType::Closed, Some("Solo"), DAVID, &[DAVID])?.id))
        .await
        .unwrap();
    root(&app.david().write(leave(id)).await);
    assert!(ids(&app, id).await.is_empty());
    assert!(
        app.db()
            .read(move |conn| Ok(Room::find(conn, id)?.deleted_at.is_none()))
            .await
            .unwrap()
    );
}
#[tokio::test]
async fn leave_is_rejected_for_non_members() {
    let app = setup().await;
    root(&app.sign_in(JZ).await.write(leave(ALL_TALK)).await);
    assert_eq!(ids(&app, ALL_TALK).await.len(), 3);
}
#[tokio::test]
async fn leave_of_a_group_dm_through_the_room_route_keeps_direct_semantics() {
    let app = setup().await;
    let id = group(&app, &[DAVID, KEVIN, JZ], JZ).await;
    root(&app.sign_in(JZ).await.write(leave(id)).await);
    assert!(!ids(&app, id).await.contains(&JZ));
    assert!(
        app.db()
            .read(move |conn| Ok(Room::find(conn, id)?.deleted_at.is_none()))
            .await
            .unwrap()
    );
    assert_eq!(note(&app, id).await, "left the group");
}
#[tokio::test]
async fn show_still_redirects_non_members_of_private_rooms() {
    let app = setup().await;
    root(
        &app.sign_in(JZ)
            .await
            .classic_page(&format!("/rooms/{ALL_TALK}"))
            .await,
    );
}
#[tokio::test]
async fn show_still_redirects_non_members_of_deleted_open_rooms() {
    let app = setup().await;
    app.db()
        .write(|tx| {
            tx.conn().execute_cached(
                "UPDATE rooms SET deleted_at=? WHERE id=104393281",
                [tx.now()],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    root(&app.sign_in(JZ).await.classic_page("/rooms/104393281").await);
}
#[tokio::test]
async fn join_recreates_the_membership_and_returns_to_the_room() {
    let app = setup().await;
    let mut jz = app.sign_in(JZ).await;
    assert!(
        app.db()
            .read(|conn| Membership::find_by_room_and_user(conn, 104393281, JZ))
            .await
            .unwrap()
            .is_none()
    );
    let reply = jz
        .write(Req::new(Method::POST, "/rooms/104393281/join"))
        .await;
    assert_eq!(
        reply.location(),
        Some("http://campfire.test/rooms/104393281")
    );
    let member = app
        .db()
        .read(|conn| Membership::find_by_room_and_user(conn, 104393281, JZ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(member.involvement.unwrap().name(), "mentions");
}
#[tokio::test]
async fn join_is_refused_for_private_rooms() {
    let app = setup().await;
    let before = ids(&app, ALL_TALK).await;
    root(
        &app.sign_in(JZ)
            .await
            .write(Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/join")))
            .await,
    );
    assert_eq!(ids(&app, ALL_TALK).await, before);
}
#[tokio::test]
async fn join_of_a_room_you_already_belong_to_returns_to_it() {
    let app = setup().await;
    let before = ids(&app, HQ).await;
    let reply = app
        .sign_in(JZ)
        .await
        .write(Req::new(Method::POST, &format!("/rooms/{HQ}/join")))
        .await;
    assert_eq!(
        reply.location(),
        Some(format!("http://campfire.test/rooms/{HQ}").as_str())
    );
    assert_eq!(ids(&app, HQ).await, before);
}
