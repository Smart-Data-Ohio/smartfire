//! Channel callbacks emit the actual Rails rows/headers, and withhold controller frames after audit failures.
use super::*;
use campfire_db::CachedStatements;
use super::directory::connect_user;
use crate::controllers::presenters::test_support::{ALL_TALK,JASON,KEVIN};
const JZ:i64=773523953;

#[tokio::test]
async fn closed_request_partial_failures_commit_but_publish_no_controller_frames() {
    let hub=boot().await.expect("seed required");
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../../../vectors/room_coercions.json")).unwrap();
    let global=hub.turbo(&["rooms"]);
    let mut clients=Vec::new();
    for id in [DAVID,JASON] {
        let mut client=connect_user(&hub,id).await;
        let stream=format!("{}:rooms",user_gid(id).to_param());
        client.confirm(&global).await;
        client.confirm(&hub.turbo(&[&stream])).await;
        clients.push(client);
    }
    let mut actor=hub.app.david();
    let mut room_id=0;
    for (index,case) in fixture["stream_cases"].as_array().unwrap().iter().enumerate() {
        assert_eq!(case["frames"],json!([]),"the real Rails failed partial lookup publishes nothing");
        let (method,path)=if index==0 {(Method::POST,"/rooms/closeds".into())}else{(Method::PATCH,format!("/rooms/closeds/{room_id}"))};
        let body=json!({"room":{"name":case["name"]},"user_ids":[DAVID,JASON]});
        let reply=actor.write(Req::new(method,&path).header("content-type","application/json").header("Accept","application/json").body(serde_json::to_vec(&body).unwrap())).await;
        assert_eq!(reply.status.as_u16() as u64,case["status"].as_u64().unwrap());
        assert_eq!(reply.json(),case["json"]);
        if index==0 {room_id=hub.app.db().read(|conn|Ok(conn.query_row_cached("SELECT MAX(id) FROM rooms",[],|row|row.get::<_,i64>(0))?)).await.unwrap();}
        let (name,mut ids)=hub.app.db().read(move|conn|{let room=Room::find(conn,room_id)?;Ok((room.name.clone(),room.user_ids(conn)?))}).await.unwrap();ids.sort();
        assert_eq!(json!(name),case["name"]);
        assert_eq!(json!(ids),case["user_ids"]);
        for client in &mut clients {client.assert_silent().await;}
    }
}

#[tokio::test]
async fn channel_http_rows_and_headers_match_rails_after_audits() {
    let hub=boot().await.expect("seed required");
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../../../vectors/channel_audits.json")).unwrap();
    let global=hub.turbo(&["rooms"]);
    let mut clients=Vec::new();
    for id in [DAVID,JASON,KEVIN,JZ] {
        let mut client=connect_user(&hub,id).await;
        let stream=format!("{}:rooms",user_gid(id).to_param());
        let own=hub.turbo(&[&stream]);
        client.confirm(&global).await;client.confirm(&own).await;
        clients.push((id,stream,own,client));
    }
    let mut actor=hub.app.david();
    for key in ["open","closed","open_update","revise","no_change","failed_open","failed_closed","failed_revision"] {
        let case=&fixture["cases"][key];
        if key=="failed_open" {
            hub.app.db().write(|tx| {tx.conn().execute_batch("CREATE TRIGGER reject_stream_audit BEFORE INSERT ON audit_logs WHEN NEW.action IN ('room.create','room.membership.change') BEGIN SELECT RAISE(ABORT,'injected channel audit failure'); END;")?;Ok(())}).await.unwrap();
        }
        let name=case["room"]["name"].as_str().unwrap();
        let mut pairs=vec![("room[name]".to_string(),name.to_string())];
        let (method,path)=match key {
            "open"|"failed_open"=>(Method::POST,"/rooms/opens".to_string()),
            "closed"|"failed_closed"=>{pairs.extend([DAVID,JASON].map(|id|("user_ids[]".into(),id.to_string())));(Method::POST,"/rooms/closeds".into())},
            "open_update"=>(Method::PATCH,format!("/rooms/opens/{}",case["room"]["id"])),
            _=>{let ids=if key=="failed_revision" {vec![DAVID,KEVIN]} else {vec![DAVID,JASON,KEVIN]};pairs.extend(ids.iter().map(|id|("user_ids[]".into(),id.to_string())));(Method::PATCH,format!("/rooms/closeds/{ALL_TALK}"))},
        };
        let params=pairs.iter().map(|(key,value)|(key.as_str(),value.as_str())).collect::<Vec<_>>();
        let reply=actor.write(Req::new(method,&path).form(&params)).await;
        assert_eq!(reply.status.as_u16() as u64,case["status"].as_u64().unwrap(),"{key}: {}",reply.text());
        for (id,stream,own,client) in &mut clients {
            if key=="failed_revision" && *id==JASON {
                let frames=client.until_closed().await;
                let expected=case["frames"].as_array().unwrap().iter().filter(|frame|frame["stream"].as_str()==Some(stream.as_str())).collect::<Vec<_>>();
                assert_eq!(frames.len(),expected.len()+1);
                for (frame,expected) in frames.iter().zip(expected) {assert_eq!(serde_json::from_str::<serde_json::Value>(frame).unwrap(),json!({"identifier":own,"message":expected["html"]}));}
                assert_eq!(frames.last().unwrap(),DISCONNECT_RECONNECT);
                continue;
            }
            for expected in case["frames"].as_array().unwrap().iter().filter(|frame|frame["stream"]=="rooms" || frame["stream"].as_str()==Some(stream.as_str())) {
                let html=expected["html"].as_str().unwrap();
                assert!(campfire_cable::turbo::session_bound(html).is_none());
                let actual:serde_json::Value=serde_json::from_str(&client.next_text().await).unwrap();
                let identifier=if expected["stream"]=="rooms" {global.as_str()} else {own.as_str()};
                assert_eq!(actual,json!({"identifier":identifier,"message":html}),"{key}, recipient {id}");
            }
            client.assert_silent().await;
        }
    }
}
