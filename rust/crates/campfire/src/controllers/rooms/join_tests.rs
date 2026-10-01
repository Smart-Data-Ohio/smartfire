//! Ports of the six Rails nonmember preview/rejoin cases, with request guards and atomicity.
use axum::http::{Method,StatusCode};
use campfire_db::{CachedStatements,Membership,Room,RoomType};
use crate::controllers::presenters::test_support::*;
fn oracle()->serde_json::Value {serde_json::from_str(include_str!("../../../../../vectors/rooms_join.json")).unwrap()}
async fn nonmember(app:&TestApp) {
    app.db().write(|tx|{tx.conn().execute_cached("DELETE FROM memberships WHERE room_id=? AND user_id=?",(HQ,DAVID))?;Ok(())}).await.unwrap();
}
#[tokio::test]
async fn open_nonmembers_see_the_join_page_and_remember_the_room() {
    let app=TestApp::boot().await.expect("seed required");
    nonmember(&app).await;
    let mut david=app.david();
    let reply=david.get(&format!("/rooms/{HQ}")).await;
    assert_eq!(reply.status,StatusCode::OK,"{}",reply.text());
    assert!(reply.text().contains("<h2>#HQ</h2>"));
    assert!(reply.text().contains(&format!("action=\"/rooms/{HQ}/join\"")));
    assert!(reply.text().contains("Join channel"));
    assert!(!reply.text().contains("id=\"message-area\""));
    assert!(reply.headers.get_all("set-cookie").iter().any(|c|c.to_str().unwrap().starts_with(&format!("last_room={HQ}"))));
    let alias=david.get(&format!("/rooms/opens/{HQ}")).await;
    assert_eq!(alias.location(),Some(format!("http://campfire.test/rooms/{HQ}").as_str()));
}
#[tokio::test]
async fn join_is_idempotent_and_uses_the_default_involvement_without_an_audit() {
    let app=TestApp::boot_frozen().await.expect("seed required");
    nonmember(&app).await;
    let mut david=app.david();
    let mut first=None;
    for key in ["join","repeat"] {
        let reply=david.write(Req::new(Method::POST,&format!("/rooms/{HQ}/join"))).await;
        assert_eq!(reply.status.as_u16() as u64,oracle()["cases"][key]["status"].as_u64().unwrap());
        assert_eq!(reply.location(),oracle()["cases"][key]["location"].as_str());
        let m=app.db().read(|conn|Membership::find_by_room_and_user(conn,HQ,DAVID)).await.unwrap().unwrap();
        assert_eq!(m.involvement.unwrap().name(),oracle()["cases"][key]["involvement"].as_str().unwrap());
        assert!(!m.unread());
        if let Some(first)=&first {assert_eq!(&m,first)}else {first=Some(m)};
    }
    app.db().read(|conn|{let count:i64=conn.query_row_cached("SELECT count(*) FROM audit_logs WHERE target_id=?",[HQ],|r|r.get(0))?;assert_eq!(count,0);Ok(())}).await.unwrap();
}
#[tokio::test]
async fn join_and_preview_refuse_private_direct_venue_deleted_and_missing_rooms() {
    let app=TestApp::boot().await.expect("seed required");
    nonmember(&app).await;
    let mut david=app.david();
    for kind in [RoomType::Closed,RoomType::Direct,RoomType::Voice,RoomType::Stage,RoomType::Board] {
        app.db().write(move|tx|{tx.conn().execute_cached("UPDATE rooms SET type=? WHERE id=?",(kind,HQ))?;Ok(())}).await.unwrap();
        let reply=david.write(Req::new(Method::POST,&format!("/rooms/{HQ}/join"))).await;
        assert_eq!(reply.location(),oracle()["cases"][kind.class_name()]["location"].as_str());
        assert_eq!(david.get(&format!("/rooms/{HQ}")).await.location(),Some("http://campfire.test/"));
        assert!(app.db().read(|conn|Membership::find_by_room_and_user(conn,HQ,DAVID)).await.unwrap().is_none());
    }
    app.db().write(|tx|{tx.conn().execute_cached("UPDATE rooms SET type='Rooms::Open',deleted_at=? WHERE id=?",(tx.now(),HQ))?;Ok(())}).await.unwrap();
    for id in [HQ,9999999999] {
        assert_eq!(david.write(Req::new(Method::POST,&format!("/rooms/{id}/join"))).await.location(),Some("http://campfire.test/"));
        assert_eq!(david.get(&format!("/rooms/{id}")).await.location(),Some("http://campfire.test/"));
    }
    assert!(app.db().read(|conn|Ok(Room::find(conn,HQ)?.deleted_at.is_some())).await.unwrap());
}
#[tokio::test]
async fn join_requires_human_authentication_and_csrf_and_failed_insert_leaves_no_membership() {
    let app=TestApp::boot().await.expect("seed required");
    nonmember(&app).await;
    let path=format!("/rooms/{HQ}/join");
    assert_eq!(app.anonymous().send(Req::new(Method::POST,&path)).await.location(),Some("http://campfire.test/session/new"));
    assert_eq!(app.anonymous().send(Req::new(Method::POST,&format!("{path}?bot_key={BENDER_KEY}"))).await.status,StatusCode::FORBIDDEN);
    assert_eq!(app.david().send(Req::new(Method::POST,&path)).await.status,StatusCode::UNPROCESSABLE_ENTITY);
    app.db().write(|tx|{tx.conn().execute_batch("CREATE TRIGGER reject_join BEFORE INSERT ON memberships BEGIN SELECT RAISE(ABORT,'injected join failure'); END;")?;Ok(())}).await.unwrap();
    assert_eq!(app.david().write(Req::new(Method::POST,&path)).await.status,StatusCode::INTERNAL_SERVER_ERROR);
    assert!(app.db().read(|conn|Membership::find_by_room_and_user(conn,HQ,DAVID)).await.unwrap().is_none());
}
