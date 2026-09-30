//! Real Rails collection predicates, next-request flash, direct reuse and group additions.
use axum::http::{Method, StatusCode};
use campfire_db::{CachedStatements, Message, Room, RoomType};
use campfire_kit::Crypto;
use crate::controllers::presenters::test_support::*;

fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../../../vectors/direct_selection.json")).unwrap()
}

fn next_flash(app: &TestApp, reply: &Reply, cookie: &mut Option<String>) -> serde_json::Value {
    let key = campfire_kit::session::SESSION_KEY;
    let raw = reply.headers.get_all("set-cookie").iter().find_map(|value| {
        value.to_str().ok()?.split(';').next()?.strip_prefix(&format!("{key}="))
    });
    // Kit preserves unchanged cookie-store sessions; keep the last response value,
    // just as the browser does when a repeated alert produces no new Set-Cookie.
    if let Some(raw)=raw {*cookie=Some(raw.to_owned());}
    let session = cookie.as_ref().and_then(|raw| {
        let raw = percent_encoding::percent_decode_str(raw).decode_utf8_lossy();
        campfire_kit::RailsCrypto::new(app.booted.app.secrets.clone()).decrypt_cookie(key, &raw, jiff::Timestamp::now())
    });
    session.and_then(|value| value.get("flash")?.get("flashes").cloned()).unwrap_or_else(|| serde_json::json!({}))
}

#[tokio::test]
async fn direct_selection_queries_match_rails_and_commit_notes_audits_and_flash() {
    let app = TestApp::boot().await.expect("seed required");
    let mut david = app.david();
    let mut cookie=None;
    for case in oracle()["creates"].as_array().unwrap() {
        let before:i64 = app.db().read(|conn| Ok(conn.query_row_cached("SELECT count(*) FROM rooms WHERE type='Rooms::Direct'",[],|r|r.get(0))?)).await.unwrap();
        let reply = david.write(Req::new(Method::POST,"/rooms/directs").header("content-type","application/json").header("Accept","application/json").body(serde_json::to_vec(&serde_json::json!({"user_ids":case["input"]})).unwrap())).await;
        assert_eq!(reply.status.as_u16() as u64,case["status"].as_u64().unwrap(),"{}: {}",case["input"],reply.text());
        assert_eq!(reply.location(),case["location"].as_str());
        let id:i64 = reply.location().unwrap().rsplit('/').next().unwrap().parse().unwrap();
        let (after,mut ids) = app.db().read(move|conn| Ok((conn.query_row_cached("SELECT count(*) FROM rooms WHERE type='Rooms::Direct'",[],|r|r.get::<_,i64>(0))?,Room::find(conn,id)?.user_ids(conn)?))).await.unwrap();ids.sort();
        assert_eq!(after-before,case["count_delta"].as_i64().unwrap());
        assert_eq!(serde_json::json!(ids),case["user_ids"],"{}",case["input"]);
        assert_eq!(next_flash(&app,&reply,&mut cookie),case["next_flash"]);
    }
    for case in oracle()["additions"].as_array().unwrap() {
        let id = app.db().write(|tx| {
            let room = Room::create(tx,RoomType::Direct,Some("Selection probe"),DAVID)?;
            room.grant_to(tx,&[DAVID,JASON,KEVIN])?;Ok(room.id)
        }).await.unwrap();
        let reply = david.write(Req::new(Method::POST,&format!("/rooms/directs/{id}/add_members")).header("content-type","application/json").header("Accept","application/json").body(serde_json::to_vec(&serde_json::json!({"user_ids":case["input"]})).unwrap())).await;
        assert_eq!(reply.status.as_u16() as u64,case["status"].as_u64().unwrap(),"{}: {}",case["input"],reply.text());
        assert_eq!(reply.location(),case["location"].as_str());
        assert_eq!(next_flash(&app,&reply,&mut cookie),case["next_flash"],"{}",case["input"]);
        let renderer=app.db().env().rich_text.clone();
        let (mut ids,notes,audits) = app.db().read(move|conn| {
            let notes=Message::for_room(conn,id)?.into_iter().filter(|message|message.system_note).map(|message|message.plain_text_body(conn,renderer.as_ref())).collect::<campfire_db::Result<Vec<_>>>()?;
            let mut query=conn.prepare_cached("SELECT action,details FROM audit_logs WHERE target_type='Room' AND target_id=? ORDER BY id")?;
            let audits=query.query_map([id],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?)))?.collect::<Result<Vec<_>,_>>()?.into_iter().map(|(action,details)|serde_json::json!([action,serde_json::from_str::<serde_json::Value>(&details).unwrap()])).collect::<Vec<_>>();
            Ok((Room::find(conn,id)?.user_ids(conn)?,notes,audits))
        }).await.unwrap();ids.sort();
        assert_eq!(serde_json::json!(ids),case["user_ids"],"{}",case["input"]);
        assert_eq!(serde_json::json!(notes),case["notes"]);
        assert_eq!(serde_json::json!(audits),case["audits"]);
    }
}

#[tokio::test]
async fn direct_selection_guards_precede_nested_id_parsing() {
    let app=TestApp::boot().await.expect("seed required");
    let id=app.db().write(|tx|Ok(Room::find_or_create_direct_for(tx,&[DAVID,JASON,KEVIN],DAVID)?.id)).await.unwrap();
    let mut outsider=app.sign_in(773523953).await;
    for room_id in [id,HQ,ALL_TALK,DIRECT_KEVIN_BENDER] {
        let reply=outsider.write(Req::new(Method::POST,&format!("/rooms/directs/{room_id}/add_members")).header("content-type","application/json").body(br#"{"user_ids":[[127326141]]}"#.to_vec())).await;
        assert_eq!(reply.location(),Some("http://campfire.test/"));
    }
    let path=format!("/rooms/directs/{id}/add_members");
    assert_eq!(app.anonymous().send(Req::new(Method::POST,&path)).await.location(),Some("http://campfire.test/session/new"));
    assert_eq!(app.anonymous().write(Req::new(Method::POST,&format!("{path}?bot_key={BENDER_KEY}"))).await.status,StatusCode::FORBIDDEN);
    assert_eq!(app.david().send(Req::new(Method::POST,&path)).await.status,StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(app.db().read(move|conn|Room::find(conn,id)?.user_ids(conn)).await.unwrap().len(),3);
}
