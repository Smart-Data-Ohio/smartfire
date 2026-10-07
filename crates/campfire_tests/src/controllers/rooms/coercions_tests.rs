//! Active Record String casts and the pinned inherited direct-show callback failure.
use axum::http::{Method,StatusCode};
use campfire_db::{CachedStatements,Room};
use crate::controllers::presenters::test_support::*;
fn oracle()->serde_json::Value {serde_json::from_str(include_str!("../../../../../vectors/room_coercions.json")).unwrap()}
#[tokio::test]
async fn channel_creation_casts_permitted_scalar_names_like_active_record() {
    let app=TestApp::boot().await.expect("seed required");
    let mut david=app.david();
    for case in oracle()["names"].as_array().unwrap() {
        let namespace=case["namespace"].as_str().unwrap();
        let body=serde_json::json!({"room":{"name":case["input"]},"user_ids":[DAVID]});
        let reply=david.write(Req::new(Method::POST,&format!("/rooms/{namespace}")).header("content-type","application/json").header("Accept","application/json").body(serde_json::to_vec(&body).unwrap())).await;
        assert_eq!(reply.status.as_u16() as u64,case["status"].as_u64().unwrap(),"{namespace}, {}: {}",case["input"],reply.text());
        let id=app.db().read(|conn|Ok(conn.query_row_cached("SELECT MAX(id) FROM rooms",[],|r|r.get::<_,i64>(0))?)).await.unwrap();
        let name=app.db().read(move|conn|Ok(Room::find(conn,id)?.name)).await.unwrap();
        assert_eq!(serde_json::json!(name),case["name"],"{namespace}, {}",case["input"]);
        let mut ids=app.db().read(move|conn|Room::find(conn,id)?.user_ids(conn)).await.unwrap(); ids.sort();
        assert_eq!(serde_json::json!(ids),case["user_ids"]);
    }
}
#[tokio::test]
async fn channel_updates_distinguish_null_names_from_unpermitted_collections() {
    let app=TestApp::boot().await.expect("seed required");
    let mut david=app.david();
    for case in oracle()["updates"].as_array().unwrap() {
        app.db().write(|tx|{tx.conn().execute_cached("UPDATE rooms SET name='Cast baseline',type='Rooms::Open' WHERE id=?",[HQ])?;Ok(())}).await.unwrap();
        let namespace=case["namespace"].as_str().unwrap();
        let body=serde_json::json!({"room":{"name":case["input"]},"user_ids":[DAVID]});
        let reply=david.write(Req::new(Method::PATCH,&format!("/rooms/{namespace}/{HQ}")).header("content-type","application/json").header("Accept","application/json").body(serde_json::to_vec(&body).unwrap())).await;
        assert_eq!(reply.status.as_u16() as u64,case["status"].as_u64().unwrap(),"{namespace}, {}",case["input"]);
        let name=app.db().read(|conn|Ok(Room::find(conn,HQ)?.name)).await.unwrap();
        assert_eq!(serde_json::json!(name),case["name"],"{namespace}, {}",case["input"]);
    }
}
#[tokio::test]
async fn direct_namespace_show_keeps_auth_gates_and_rails_missing_room_failure() {
    let app=TestApp::boot().await.expect("seed required");
    let mut david=app.david();
    for (id,expected) in oracle()["shows"].as_object().unwrap() {
        let path=format!("/rooms/directs/{id}");
        assert_eq!(app.anonymous().get(&path).await.location(),Some("http://campfire.test/session/new"));
        assert_eq!(app.anonymous().get(&format!("{path}?bot_key={BENDER_KEY}")).await.status,StatusCode::FORBIDDEN);
        let reply=david.send(Req::new(Method::GET,&path).header("Accept","application/json")).await;
        assert_eq!(reply.status.as_u16() as u64,expected["status"].as_u64().unwrap(),"{id}");
        assert_eq!(reply.json(),expected["json"]);
    }
    // The actual page route remains scoped and working.
    assert_eq!(david.get(&format!("/rooms/{DIRECT_DAVID_JASON}")).await.status,StatusCode::OK);
    assert_eq!(david.get(&format!("/rooms/{DIRECT_KEVIN_BENDER}")).await.location(),Some("http://campfire.test/"));
}

#[tokio::test]
async fn closed_broadcasts_follow_the_request_partial_format_after_commit() {
    let app=TestApp::boot().await.expect("seed required");
    let mut david=app.david();
    for (accept,expected) in oracle()["formats"].as_object().unwrap() {
        let reply=david.write(Req::new(Method::POST,"/rooms/closeds").header("Accept",accept).form(&[("room[name]","Format probe"),("user_ids[]",&DAVID.to_string())])).await;
        assert_eq!(reply.status.as_u16() as u64,expected.as_u64().unwrap(),"{accept}: {}",reply.text());
        let id=app.db().read(|conn|Ok(conn.query_row_cached("SELECT MAX(id) FROM rooms",[],|r|r.get::<_,i64>(0))?)).await.unwrap();
        app.db().read(move|conn| {
            assert_eq!(Room::find(conn,id)?.name,Some("Format probe".into()));
            assert!(campfire_db::Membership::find_by_room_and_user(conn,id,DAVID)?.is_some());
            let count:i64=conn.query_row_cached("SELECT count(*) FROM audit_logs WHERE action='room.create' AND target_id=?",[id],|r|r.get(0))?;
            assert_eq!(count,1);
            Ok(())
        }).await.unwrap();
    }
}

#[tokio::test]
async fn closed_grantees_cast_numbers_and_nested_arrays_without_flattening_hashes() {
    let app=TestApp::boot().await.expect("seed required");
    let mut david=app.david();
    for case in oracle()["ids"].as_array().unwrap() {
        let body=serde_json::json!({"room":{"name":"ID cast probe"},"user_ids":case["input"]});
        let reply=david.write(Req::new(Method::POST,"/rooms/closeds").header("content-type","application/json").header("Accept","application/json").body(serde_json::to_vec(&body).unwrap())).await;
        assert_eq!(reply.status.as_u16() as u64,case["status"].as_u64().unwrap());
        let mut ids=app.db().read(|conn| {let id=conn.query_row_cached("SELECT MAX(id) FROM rooms",[],|r|r.get::<_,i64>(0))?;Room::find(conn,id)?.user_ids(conn)}).await.unwrap();ids.sort();
        assert_eq!(serde_json::json!(ids),case["user_ids"],"{}",case["input"]);
    }
}
