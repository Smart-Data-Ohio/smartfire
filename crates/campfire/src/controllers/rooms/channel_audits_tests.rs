//! Room creation/revision audit parity, including failures after the domain commit.
use axum::http::{Method,StatusCode};
use campfire_db::{CachedStatements,Membership,Room};
use crate::controllers::presenters::test_support::*;
fn oracle()->serde_json::Value {serde_json::from_str(include_str!("../../../../../vectors/channel_audits.json")).unwrap()}
async fn audit_rows(app:&TestApp,id:i64)->serde_json::Value {
    app.db().read(move|conn|{
        let mut stmt=conn.prepare("SELECT action,actor_id,actor_label,target_type,target_id,target_label,details,ip_address FROM audit_logs WHERE target_type='Room' AND target_id=? ORDER BY id")?;
        let rows=stmt.query_map([id],|r|Ok(serde_json::json!({"action":r.get::<_,String>(0)?,"actor_id":r.get::<_,Option<i64>>(1)?,"actor_label":r.get::<_,Option<String>>(2)?,"target_type":r.get::<_,String>(3)?,"target_id":r.get::<_,i64>(4)?,"target_label":r.get::<_,Option<String>>(5)?,"details":serde_json::from_str::<serde_json::Value>(&r.get::<_,String>(6)?).unwrap(),"ip_address":r.get::<_,Option<String>>(7)?})))?.collect::<Result<Vec<_>,_>>()?;
        Ok(serde_json::json!(rows))
    }).await.unwrap()
}
fn revision(name:&str,ids:&[i64])->Req {
    let mut values=vec![("room[name]".to_string(),name.to_string())];
    values.extend(ids.iter().map(|id|("user_ids[]".to_string(),id.to_string())));
    let pairs=values.iter().map(|(key,value)|(key.as_str(),value.as_str())).collect::<Vec<_>>();
    Req::new(Method::PATCH,&format!("/rooms/closeds/{ALL_TALK}")).header("x-forwarded-for","127.0.0.1").form(&pairs)
}
#[tokio::test]
async fn channel_creations_record_the_rails_actor_target_and_changes() {
    let app=TestApp::boot_frozen().await.expect("seed required");
    let mut david=app.david();
    for (key,namespace) in [("open","opens"),("closed","closeds")] {
        let expected=&oracle()["cases"][key];
        let name=expected["room"]["name"].as_str().unwrap().to_string();
        let reply=david.write(Req::new(Method::POST,&format!("/rooms/{namespace}")).header("x-forwarded-for","127.0.0.1").form(&[("room[name]",&name),("user_ids[]",&DAVID.to_string()),("user_ids[]",&JASON.to_string())])).await;
        assert_eq!(reply.location(),expected["location"].as_str());
        let id=expected["room"]["id"].as_i64().unwrap();
        assert_eq!(audit_rows(&app,id).await,expected["audits"]);
        let mut ids=app.db().read(move|conn|Room::find(conn,id)?.user_ids(conn)).await.unwrap();ids.sort();
        assert_eq!(serde_json::json!(ids),expected["room"]["user_ids"]);
    }
}
#[tokio::test]
async fn channel_revision_audits_only_actual_grants_and_revocations() {
    let app=TestApp::boot_frozen().await.expect("seed required");
    let mut david=app.david();
    for key in ["revise","no_change"] {
        let reply=david.write(revision("Revised room",&[DAVID,JASON,KEVIN])).await;
        assert_eq!(reply.status,StatusCode::FOUND);
        assert_eq!(audit_rows(&app,ALL_TALK).await,oracle()["cases"][key]["audits"]);
    }
    let denied=app.sign_in(KEVIN).await.write(revision("Stolen",&[KEVIN])).await;
    assert_eq!(denied.status,StatusCode::FORBIDDEN);
    assert_eq!(audit_rows(&app,ALL_TALK).await,oracle()["cases"]["no_change"]["audits"]);
    assert_eq!(app.db().read(|conn|Ok(Room::find(conn,ALL_TALK)?.name)).await.unwrap(),Some("Revised room".into()));
}
#[tokio::test]
async fn audit_outages_do_not_undo_channel_creations_or_revisions() {
    let app=TestApp::boot_frozen().await.expect("seed required");
    app.db().write(|tx| {tx.conn().execute_batch("CREATE TRIGGER reject_channel_audit BEFORE INSERT ON audit_logs WHEN NEW.action IN ('room.create','room.membership.change') BEGIN SELECT RAISE(ABORT,'injected channel audit failure'); END;")?;Ok(())}).await.unwrap();
    let mut david=app.david();
    for (key,namespace) in [("failed_open","opens"),("failed_closed","closeds")] {
        let name=oracle()["cases"][key]["room"]["name"].as_str().unwrap().to_string();
        let reply=david.write(Req::new(Method::POST,&format!("/rooms/{namespace}")).header("x-forwarded-for","127.0.0.1").form(&[("room[name]",&name),("user_ids[]",&DAVID.to_string()),("user_ids[]",&JASON.to_string())])).await;
        assert_eq!(reply.status,StatusCode::INTERNAL_SERVER_ERROR);
        let room=app.db().read(move|conn|{let id:i64=conn.query_row_cached("SELECT id FROM rooms WHERE name=?",[name],|r|r.get(0))?;Room::find(conn,id)}).await.unwrap();
        assert!(room.deleted_at.is_none());
        assert!(app.db().read(move|conn|Membership::find_by_room_and_user(conn,room.id,DAVID)).await.unwrap().is_some());
    }
    let reply=david.write(revision("Audit failed revision",&[DAVID,KEVIN])).await;
    assert_eq!(reply.status,StatusCode::INTERNAL_SERVER_ERROR);
    let mut ids=app.db().read(|conn|Room::find(conn,ALL_TALK)?.user_ids(conn)).await.unwrap();ids.sort();
    assert_eq!(ids,vec![DAVID,KEVIN]);
    assert_eq!(app.db().read(|conn|Ok(Room::find(conn,ALL_TALK)?.name)).await.unwrap(),Some("Audit failed revision".into()));
    assert_eq!(audit_rows(&app,ALL_TALK).await,serde_json::json!([]));
}
