//! Real room/thread header security checks for every merged shared message form.
use std::sync::Arc;
use axum::http::{Method, StatusCode};
use campfire_db::{Boost, ChannelThread, Message, NewChannelThread, NewMessage, ThreadMembership};
use campfire_kit::clock::FrozenClock;
use serde_json::Value;
use crate::controllers::presenters::test_support::*;



fn oracle() -> Value { serde_json::from_str(include_str!("../../../../../vectors/messaging/cached-csrf.json")).unwrap() }

async fn fixture() -> TestApp {
    // Rails queues a fetch for a new reference even when its fixture card is fresh.
    // These cache/form tests supply fixed fetched cards; provider execution has
    // separate tests. Stop the consumer before mounting the shared cached bytes.
    let app = TestApp::boot_with_test_clock(Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()))).await.unwrap().without_job_runner().await;
    app.db().write(|tx| {
        let pr=crate::integrations::github::pull_requests::PullRequest::for_reference(tx,"rails","rails",3141)?;
        tx.conn().execute("UPDATE github_pull_requests SET private=0,title='Cached card',state='open',fetched_at=?,fetch_requested_at=NULL,updated_at=? WHERE id=?",(tx.now(),tx.now(),pr.id))?;
        let card=Message::create(tx,NewMessage {room_id:ALL_TALK,creator_id:DAVID,markdown_source:Some("see https://github.com/rails/rails/pull/3141".into()),client_message_id:Some("csrf-pr".into()),..Default::default()})?;
        let poll_message=Message::create(tx,NewMessage {room_id:ALL_TALK,creator_id:DAVID,markdown_source:Some("Lunch?".into()),client_message_id:Some("csrf-poll".into()),..Default::default()})?;
        tx.conn().execute("INSERT INTO polls (id,message_id,created_at,updated_at) VALUES (?,?,?,?)",(oracle()["poll_id"].as_i64().unwrap(),poll_message.id,tx.now(),tx.now()))?;
        for option in oracle()["poll_options"].as_array().unwrap() {
            tx.conn().execute("INSERT INTO poll_options (id,poll_id,label,position,created_at,updated_at) VALUES (?,?,?,?,?,?)",rusqlite::params![option[0].as_i64(),oracle()["poll_id"].as_i64(),option[1].as_str(),option[2].as_i64(),tx.now(),tx.now()])?;
        }
        assert_eq!(poll_message.id,oracle()["poll_message_id"].as_i64().unwrap());
        let boosted=Message::create(tx,NewMessage {room_id:ALL_TALK,creator_id:DAVID,markdown_source:Some("Boost me".into()),client_message_id:Some("csrf-boosts".into()),..Default::default()})?;
        Boost::create(tx,boosted.id,DAVID,"👍")?;
        let legacy=Boost::create(tx,boosted.id,JASON,"Legacy text boost")?;
        let mut thread=ChannelThread::create(tx,NewChannelThread {room_id:ALL_TALK,creator_id:DAVID,name:Some("Cached thread".into()),..Default::default()})?;
        ThreadMembership::join(tx,thread.id,DAVID)?;
        let reply=thread.post_message(tx,DAVID,NewMessage {markdown_source:Some("In the thread".into()),client_message_id:Some("csrf-thread".into()),..Default::default()})?;
        Boost::create(tx,reply.id,DAVID,"🎉")?;
        for (key,id) in [("card_id",card.id),("boosted_id",boosted.id),("legacy_boost_id",legacy.id),("thread_id",thread.id),("reply_id",reply.id)] { assert_eq!(oracle()[key].as_i64(),Some(id),"{key}"); }
        Ok(())
    }).await.unwrap();
    app
}

async fn submit_cached_forms(room_shell: bool) {
    let oracle=if room_shell {serde_json::from_str::<Value>(include_str!("../../../../../vectors/messaging/room-csrf.json")).unwrap()} else {oracle()};
    let app=fixture().await;
    let mut first=app.david();let foreign=first.authenticity_token().await;
    let mut viewer=app.sign_in(JASON).await;
    let token=viewer.authenticity_token().await;
    let rendered=oracle["forms"].as_array().unwrap().iter().map(|row| {
        let mut params=row["params"].as_object().unwrap().iter().map(|(k,v)|
            (k.clone(),v.as_str().or_else(||v.as_array().and_then(|v|v.first()).and_then(Value::as_str)).unwrap().to_owned())
        ).collect::<std::collections::BTreeMap<_,_>>();
        params.insert("_method".into(),row["method"].as_str().unwrap().into());
        (row["action"].as_str().unwrap().to_owned(),params)
    }).collect::<Vec<_>>();
    for ((action,mut params),expected) in rendered.into_iter().zip(oracle["forms"].as_array().unwrap()) {
        assert!(!params.contains_key("authenticity_token"));
        let method=params.remove("_method").unwrap_or("post".into());
        assert_eq!(action,expected["action"].as_str().unwrap());assert_eq!(method,expected["method"].as_str().unwrap());
        let actual=params.iter().map(|(key,value)| (key.clone(),if key.ends_with("[]") {serde_json::json!([value])} else {serde_json::json!(value)})).collect::<serde_json::Map<_,_>>();
        assert_eq!(Value::Object(actual),expected["params"]);
        let pairs=params.iter().map(|(k,v)|(k.as_str(),v.as_str())).collect::<Vec<_>>();
        let request=|| Req::new(Method::from_bytes(method.to_uppercase().as_bytes()).unwrap(),&action).form(&pairs).header("accept","text/vnd.turbo-stream.html, text/html, application/xhtml+xml");
        let before=app.db().read(|conn| Ok((Message::count(conn)?,conn.query_row("SELECT COUNT(*) FROM boosts",[],|r|r.get::<_,i64>(0))?,conn.query_row("SELECT COUNT(*) FROM channel_threads",[],|r|r.get::<_,i64>(0))?,conn.query_row("SELECT COUNT(*) FROM poll_votes",[],|r|r.get::<_,i64>(0))?))).await.unwrap();
        for csrf in [None,Some(foreign.as_str())] {
            let mut req=request();if let Some(csrf)=csrf {req=req.header("x-csrf-token",csrf);}
            assert_eq!(viewer.send(req).await.status,StatusCode::UNPROCESSABLE_ENTITY,"{method} {action}");
        }
        let after=app.db().read(|conn| Ok((Message::count(conn)?,conn.query_row("SELECT COUNT(*) FROM boosts",[],|r|r.get::<_,i64>(0))?,conn.query_row("SELECT COUNT(*) FROM channel_threads",[],|r|r.get::<_,i64>(0))?,conn.query_row("SELECT COUNT(*) FROM poll_votes",[],|r|r.get::<_,i64>(0))?))).await.unwrap();
        assert_eq!(before,after,"forgery refusals do not write");
        let response=viewer.send(request().header("x-csrf-token",&token)).await;
        assert_eq!(response.status.as_u16(),if action.ends_with("/vote") { 302 } else { expected["status"].as_u64().unwrap() as u16 },"{method} {action}: {}",response.text());
    }
}

#[tokio::test]
async fn message_mutations_accept_own_tokens_and_reject_foreign_or_missing_tokens() {
    submit_cached_forms(false).await;
    submit_cached_forms(true).await;
}
