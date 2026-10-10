//! Ports of the seven cases in test/controllers/rooms/reads_controller_test.rb.
use axum::http::{Method, StatusCode};
use campfire_db::{CachedStatements, Membership, Message};
use crate::controllers::presenters::test_support::*;

fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../../../vectors/reads_refresh.json")).unwrap()
}

#[tokio::test]
async fn read_and_unread_advance_only_the_requesters_root_pointer() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let mut david = app.david();
    let path = format!("/rooms/{ALL_TALK}/read");
    let fixture = oracle();
    let jason_before=app.db().read(|conn|Membership::find_by_room_and_user(conn,ALL_TALK,JASON)).await.unwrap();
    for (key, method, index) in [("create", Method::POST, None), ("second", Method::DELETE, Some(1)), ("first", Method::DELETE, Some(0))] {
        let message_id = index.map(|i| fixture["root_ids"][i].as_i64().unwrap());
        let req = Req::new(method, &path).header("Accept", "application/json");
        let req = if let Some(id) = message_id { req.form(&[("message_id", &id.to_string())]) } else { req };
        let reply = david.write(req).await;
        assert_eq!(reply.status, StatusCode::OK, "{key}: {}", reply.text());
        assert_eq!(reply.json(), fixture["cases"][key]["json"]);
        let jason_before=jason_before.clone();
        app.db().read(move |conn| {
            let membership = Membership::find_by_room_and_user(conn, ALL_TALK, DAVID)?.unwrap();
            assert_eq!(membership.unread(), message_id.is_some());
            let expected = oracle()["cases"][key]["pointer"].as_i64();
            assert_eq!(membership.last_read_message_id, expected);
            if let Some(id) = message_id { assert_eq!(membership.unread_at, Some(Message::find(conn,id)?.created_at)); }
            assert_eq!(Membership::find_by_room_and_user(conn, ALL_TALK, JASON)?,jason_before);
            Ok(())
        }).await.unwrap();
    }
}

#[tokio::test]
async fn reads_reject_nonmembers_deleted_rooms_and_bot_credentials() {
    let app = TestApp::boot().await.expect("seed required");
    let path = format!("/rooms/{ALL_TALK}/read");
    let outsider = app.sign_in(KEVIN).await.write(Req::new(Method::POST, &path)).await;
    assert_eq!(outsider.status.as_u16() as u64, oracle()["cases"]["outsider"]["status"].as_u64().unwrap());
    let anonymous = app.anonymous().send(Req::new(Method::POST, &path)).await;
    assert_eq!(anonymous.status.as_u16() as u64, oracle()["cases"]["anonymous"]["status"].as_u64().unwrap());
    let bot = app.anonymous().send(Req::new(Method::POST, &format!("{path}?bot_key={BENDER_KEY}"))).await;
    assert_eq!(bot.status.as_u16() as u64, oracle()["cases"]["bot"]["status"].as_u64().unwrap());
    app.db().write(|tx| {tx.conn().execute_cached("UPDATE rooms SET deleted_at=? WHERE id=?",(tx.now(),ALL_TALK))?; Ok(())}).await.unwrap();
    assert_eq!(app.david().write(Req::new(Method::POST,&path)).await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn unread_rejects_foreign_missing_and_thread_messages_without_writing() {
    let app = TestApp::boot().await.expect("seed required");
    let (foreign, thread) = app.db().write(|tx| {
        let foreign = Message::for_room(tx.conn(),699448329)?.first().unwrap().id;
        let root=Message::for_room(tx.conn(),ALL_TALK)?.first().unwrap().id;
        tx.conn().execute_cached("INSERT INTO channel_threads (room_id,creator_id,name,created_at,updated_at,last_activity_at) VALUES (?,?, 'Read guard',?,?,?)",(ALL_TALK,DAVID,tx.now(),tx.now(),tx.now()))?;
        let thread_id=tx.conn().last_insert_rowid();
        tx.conn().execute_cached("UPDATE messages SET thread_id=? WHERE id=?",(thread_id,root))?;
        Ok((foreign,root))
    }).await.unwrap();
    let before=app.db().read(|conn|Membership::find_by_room_and_user(conn,ALL_TALK,DAVID)).await.unwrap();
    for id in [foreign,9999999999,thread] {
        let reply=app.david().write(Req::new(Method::DELETE,&format!("/rooms/{ALL_TALK}/read")).form(&[("message_id",&id.to_string())])).await;
        assert_eq!(reply.status,StatusCode::NOT_FOUND,"{}",reply.text());
        assert_eq!(app.db().read(|conn|Membership::find_by_room_and_user(conn,ALL_TALK,DAVID)).await.unwrap(),before);
    }
}

#[tokio::test]
async fn unread_predecessor_uses_id_for_equal_timestamps_and_write_failure_rolls_back() {
    let app=TestApp::boot_frozen().await.expect("seed required");
    let (target,previous)=app.db().write(|tx| {
        let mut ids=oracle()["root_ids"].as_array().unwrap().iter().map(|id|id.as_i64().unwrap()).collect::<Vec<_>>();
        ids.sort();
        tx.conn().execute_cached("UPDATE messages SET created_at=? WHERE room_id=?",(tx.now(),ALL_TALK))?;
        let target=ids[1];
        let previous=tx.conn().query_row_cached("SELECT id FROM messages WHERE room_id=? AND thread_id IS NULL AND id<? ORDER BY id DESC LIMIT 1",(ALL_TALK,target),|r|r.get::<_,i64>(0))?;
        Ok((target,previous))
    }).await.unwrap();
    let mut david=app.david();
    let request=||Req::new(Method::DELETE,&format!("/rooms/{ALL_TALK}/read")).form(&[("message_id",&target.to_string())]);
    assert_eq!(david.write(request()).await.status,StatusCode::OK);
    let before=app.db().read(|conn|Membership::find_by_room_and_user(conn,ALL_TALK,DAVID)).await.unwrap();
    assert_eq!(before.as_ref().unwrap().last_read_message_id,Some(previous));
    app.db().write(|tx|{tx.conn().execute_batch("CREATE TRIGGER reject_read BEFORE UPDATE ON memberships BEGIN SELECT RAISE(ABORT,'injected read failure'); END;")?;Ok(())}).await.unwrap();
    assert_eq!(david.write(Req::new(Method::POST,&format!("/rooms/{ALL_TALK}/read"))).await.status,StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(app.db().read(|conn|Membership::find_by_room_and_user(conn,ALL_TALK,DAVID)).await.unwrap(),before);
}

#[tokio::test]
async fn legacy_refresh_redirects_members_and_denies_nonmembers() {
    let app=TestApp::boot_frozen().await.expect("seed required");
    for accept in ["text/html", "text/vnd.turbo-stream.html", "application/json"] {
        let reply=app.david().send(Req::new(Method::GET,"/rooms/699448329/refresh?since=1772467200000").header("Accept",accept)).await;
        assert_eq!(reply.status,StatusCode::FOUND);
        assert_eq!(reply.location(), Some("http://campfire.test/rooms/699448329"));
        assert!(reply.text().is_empty());
    }
    let denied=app.sign_in(KEVIN).await.get(&format!("/rooms/{ALL_TALK}/refresh?since=0")).await;
    assert_eq!(denied.status,StatusCode::NOT_FOUND);
}
