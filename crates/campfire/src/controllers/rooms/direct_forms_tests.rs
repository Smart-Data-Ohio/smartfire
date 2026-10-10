//! Settings/picker authorization precedes parameters; invalid names preserve attempted state.
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::Room;
#[tokio::test]
async fn direct_forms_keep_human_membership_and_group_delete_gates() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let id = app
        .db()
        .write(|tx| Ok(Room::find_or_create_direct_for(tx, &[DAVID, JASON, KEVIN], DAVID)?.id))
        .await
        .unwrap();
    let path = format!("/rooms/directs/{id}/edit");
    assert_eq!(
        app.sign_in(773523953)
            .await
            .send(Req::new(Method::GET, &path).header("x-requested-with", "XMLHttpRequest"))
            .await
            .location(),
        Some("http://campfire.test/")
    );
    assert_eq!(
        app.anonymous()
            .send(Req::new(Method::GET, &path).header("x-requested-with", "XMLHttpRequest"))
            .await
            .location(),
        Some("http://campfire.test/session/new")
    );
    assert_eq!(
        app.anonymous()
            .send(Req::new(
                Method::GET,
                &format!("{path}?bot_key={BENDER_KEY}")
            ))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    let reply = app
        .sign_in(KEVIN)
        .await
        .send(Req::new(Method::GET, &path).header("x-requested-with", "XMLHttpRequest"))
        .await;
    assert_eq!(reply.status, StatusCode::FOUND);
    assert_eq!(reply.location(), Some(format!("http://campfire.test/app/r/{id}").as_str()));
}
#[tokio::test]
async fn invalid_direct_rename_leaves_the_name_unchanged() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let id = app
        .db()
        .write(|tx| Ok(Room::find_or_create_direct_for(tx, &[DAVID, JASON, KEVIN], DAVID)?.id))
        .await
        .unwrap();
    let attempted = "é".repeat(101);
    let reply = app
        .david()
        .write(
            Req::new(Method::PATCH, &format!("/rooms/directs/{id}"))
                .form(&[("room[name]", &format!("  {attempted}  "))]),
        )
        .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        app.db()
            .read(move |conn| Ok(Room::find(conn, id)?.name))
            .await
            .unwrap(),
        None
    );
}
