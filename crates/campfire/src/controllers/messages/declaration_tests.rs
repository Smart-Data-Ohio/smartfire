//! Named remaining root/cache declarations over the actual router and stored rows.
use std::sync::Arc;
use axum::http::{Method,StatusCode};
use campfire_db::{Boost,Message,NewMessage,MessageChanges,Timestamp};
use campfire_kit::clock::FrozenClock;
use serde_json::Value;
use crate::controllers::presenters::test_support::*;

fn oracle()->Value {serde_json::from_str(include_str!("../../../../../vectors/messaging/root-declarations.json")).unwrap()}
#[tokio::test]
async fn root_edit_markers_match_rails_for_noops_attachments_formatting_reactions_fetches_tombstones_and_zones() {
    let clock=Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
    let app=TestApp::boot_with_test_clock(clock.clone()).await.unwrap().without_job_runner().await;
    let ids=app.db().write(|tx| {
        let mut ids=Vec::new();
        for (index,input) in oracle()["inputs"].as_array().unwrap().iter().enumerate() {
            let message=Message::create(tx,NewMessage {room_id:ALL_TALK,creator_id:DAVID,markdown_source:input["markdown_source"].as_str().map(str::to_string),body:input["body"].as_str().map(str::to_string),client_message_id:input["client_message_id"].as_str().map(str::to_string),attachment_blob_id:(index==2).then_some(13),..Default::default()})?;
            ids.push(message.id);
        }
        tx.conn().execute("UPDATE messages SET edited_at=? WHERE id=?",(Timestamp::from_jiff("2026-09-22T12:00:00Z".parse().unwrap()),ids[3]))?;
        Message::find(tx.conn(),ids[5])?.update(tx,MessageChanges {reply_to_message_id:Some(Some(ids[4])),..Default::default()})?;
        Ok(ids)
    }).await.unwrap();
    assert_eq!(serde_json::json!(ids),oracle()["message_ids"]);
    let mut viewer=app.david();
    for row in oracle()["rows"].as_array().unwrap() {
        clock.set(row["time"].as_str().unwrap().parse().unwrap());
        let id=ids[row["index"].as_u64().unwrap() as usize];
        let response=viewer.write(Req::new(Method::PATCH,&format!("/rooms/{ALL_TALK}/messages/{id}.json")).header("content-type","application/json").body(serde_json::json!({"message":row["input"]}).to_string())).await;
        assert_eq!(response.status.as_u16(),row["status"].as_u64().unwrap() as u16,"{}: {}",row["name"],response.text());
        if response.text()!=row["body"].as_str().unwrap(){rails_mismatch(&response.text(),row["body"].as_str().unwrap(),row["name"].as_str().unwrap());}
        let (edited,updated,body)=app.db().read(move|conn|{let m=Message::find(conn,id)?;Ok((m.edited_at.map(|t|campfire_presentation::messages::support::json_time(t.jiff())),campfire_presentation::messages::support::json_time(m.updated_at.jiff()),m.body_html(conn)?.unwrap_or_default()))}).await.unwrap();
        assert_eq!(edited,row["edited_at"].as_str().map(str::to_string),"{}",row["name"]);
        assert_eq!(updated,row["updated_at"].as_str().unwrap(),"{}",row["name"]);assert_eq!(body,row["saved_body"].as_str().unwrap());
    }
    let id=ids[0];app.db().write(move|tx|Boost::create(tx,id,JASON,"👍").map(|_|())).await.unwrap();
    let card=ids[6];app.db().write(move|tx| {
        let pr=crate::integrations::github::pull_requests::PullRequest::for_message(tx.conn(),card)?.remove(0);
        crate::integrations::github::pull_requests::update(tx,pr.id,&[("private",rusqlite::types::Value::Integer(0)),("title",rusqlite::types::Value::Text("Fetched".into())),("fetched_at",rusqlite::types::Value::Text(tx.now().to_db()))])?;
        Ok(())
    }).await.unwrap();
    let source=ids[4];assert_eq!(viewer.write(Req::new(Method::DELETE,&format!("/rooms/{ALL_TALK}/messages/{source}.turbo_stream"))).await.status,StatusCode::NO_CONTENT);
    for (key,id) in [("reaction_edited",ids[0]),("fetch_edited",ids[6]),("reply_edited",ids[5])] {
        assert_eq!(app.db().read(move|conn|Ok(Message::find(conn,id)?.edited_at.map(|t|campfire_presentation::messages::support::json_time(t.jiff())))).await.unwrap(),oracle()[key].as_str().map(str::to_owned));
    }

}
