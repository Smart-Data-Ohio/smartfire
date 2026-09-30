//! Settings/picker authorization precedes parameters; invalid names preserve attempted state.
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::Room;
#[tokio::test]
async fn direct_forms_keep_human_membership_and_group_delete_gates() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let id = app.db().write(|tx| Ok(Room::find_or_create_direct_for(tx, &[DAVID,JASON,KEVIN], DAVID)?.id)).await.unwrap();
    let path = format!("/rooms/directs/{id}/edit");
    assert_eq!(app.sign_in(773523953).await.send(Req::new(Method::GET,&path)).await.location(),Some("http://campfire.test/"));
    assert_eq!(app.anonymous().send(Req::new(Method::GET,&path)).await.location(),Some("http://campfire.test/session/new"));
    assert_eq!(app.anonymous().send(Req::new(Method::GET,&format!("{path}?bot_key={BENDER_KEY}"))).await.status,StatusCode::FORBIDDEN);
    let reply=app.sign_in(KEVIN).await.send(Req::new(Method::GET,&path)).await;
    assert_eq!(reply.status,StatusCode::OK);
    assert!(reply.text().contains("Leave group"));
    assert!(!reply.text().contains("Delete Ping"));
    let reply=app.david().send(Req::new(Method::GET,&path)).await;
    assert!(reply.text().contains("Delete Ping"));
}
#[tokio::test]
async fn invalid_direct_rename_renders_the_attempted_name_without_writes() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let id = app.db().write(|tx| Ok(Room::find_or_create_direct_for(tx, &[DAVID,JASON,KEVIN], DAVID)?.id)).await.unwrap();
    let attempted="é".repeat(101);
    let reply=app.david().write(Req::new(Method::PATCH,&format!("/rooms/directs/{id}")).form(&[("room[name]",&format!("  {attempted}  "))])).await;
    assert_eq!(reply.status,StatusCode::UNPROCESSABLE_ENTITY);
    assert!(reply.text().contains(&format!("value=\"{attempted}\"")),"{}",reply.text());
    assert!(reply.text().contains("field_with_errors"));
    assert_eq!(app.db().read(move|conn| Ok(Room::find(conn,id)?.name)).await.unwrap(),None);
}
#[tokio::test]
async fn direct_picker_uses_live_viewer_stars_agent_facts_and_active_people() {
    use campfire_db::CachedStatements;
    let app=TestApp::boot_frozen().await.expect("seed required");
    app.db().write(|tx| {
        tx.conn().execute_cached("INSERT INTO user_stars(user_id,starred_user_id,created_at,updated_at) VALUES (?,?,?,?)",rusqlite::params![DAVID,KEVIN,tx.now(),tx.now()])?;
        tx.conn().execute_cached("UPDATE users SET status=1 WHERE id=?",[JASON])?;
        Ok(())
    }).await.unwrap();
    let mut david=app.david();
    let req=||Req::new(Method::GET,"/rooms/directs/new").header("Turbo-Frame","direct_rooms_control");
    let reply=david.send(req()).await;
    assert_eq!(reply.status,StatusCode::OK);
    let body=reply.text();
    assert!(body.contains("dm_picker_filter"));
    assert!(!body.contains("data-controller=\"autocomplete\""));
    let first=body.split("data-user-id=\"").nth(1).unwrap().split('"').next().unwrap();
    assert_eq!(first,KEVIN.to_string());
    assert!(body.contains("Starred by you"));
    assert!(body.contains("<span class=\"profile-card__badge\">Agent</span>"));
    assert!(!body.contains(&format!("pick_user_{JASON}")));
    assert!(!body.contains(&format!("pick_user_{DAVID}")));
    assert!(!app.sign_in(KEVIN).await.send(req()).await.text().contains("Starred by you"));
    app.db().write(|tx|{tx.conn().execute_cached("DELETE FROM user_stars WHERE user_id=?",[DAVID])?;Ok(())}).await.unwrap();
    assert!(!david.send(req()).await.text().contains("Starred by you"));
}
