//! Root-controller attribution on the merged WS11 delivery path. Only external
//! DNS/HTTP is redirected to a held listener; controller/model/jobs are real.
use super::*;
use crate::controllers::presenters::test_support::*;
use crate::integrations::test_support::{FakeResolver,FakeServer,MappingDialer,Route,network};
use axum::http::{Method,StatusCode};
use campfire_db::AgentGrant;
use campfire_db::models::agent_grant::NewGrant;
use campfire_kit::clock::FrozenClock;
use serde_json::{Value,json};
use std::{collections::HashSet,sync::{Arc,Mutex}};
fn oracle()->Value {serde_json::from_str(include_str!("../../../../../vectors/messaging/bot-controller-declarations.json")).unwrap()}
async fn app()->TestApp {let mut app=TestApp::boot_with_test_clock(Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()))).await.unwrap();app.booted.jobs.stop(Duration::from_secs(1)).await;app.db().write(|tx|{tx.conn().execute("UPDATE webhooks SET url='http://bots.example:8080/hook' WHERE user_id=?",[BENDER])?;Ok(())}).await.unwrap();app}
async fn net()->(FakeServer,Network) {
    let server=FakeServer::start(vec![Route::new("POST","*","/hook",200)]).await;
    let network=network(Arc::new(FakeResolver::new([("bots.example",vec!["93.184.216.34"])])),Arc::new(MappingDialer {public:HashSet::from(["93.184.216.34".parse().unwrap()]),to:server.addr,dialed:Mutex::new(vec![])}));
    (server,network)
}
async fn post(app:&TestApp,input:&Value)->i64 {
    let response=app.david().write(Req::new(Method::POST,&format!("/rooms/{ALL_TALK}/messages.turbo_stream")).header("content-type","application/json").body(json!({"message":input}).to_string())).await;
    assert_eq!(response.status,StatusCode::OK,"{}",response.text());
    let client=input["client_message_id"].as_str().unwrap().to_owned();
    app.db().read(move|conn|Ok(Message::find_duplicate(conn,ALL_TALK,DAVID,&client)?.unwrap().id)).await.unwrap()
}
async fn event(app:&TestApp,message:i64)->i64 {app.db().read(move|conn|Ok(conn.query_row("SELECT id FROM agent_events WHERE message_id=? AND event_type='mention'",[message],|r|r.get(0))?)).await.unwrap()}

#[tokio::test]
async fn rich_text_and_markdown_root_mentions_enqueue_exactly_one_agent_job_and_no_legacy_job() {
    let app=app().await;let mut previous=0;
    for row in oracle()["rows"].as_array().unwrap() {
        let input=&row["input"];
        let response=app.david().write(Req::new(Method::POST,&format!("/rooms/{ALL_TALK}/messages.turbo_stream")).header("content-type","application/json").body(json!({"message":input}).to_string())).await;
        assert_eq!(response.status.as_u16(),row["status"].as_u64().unwrap() as u16);
        if response.text()!=row["body"].as_str().unwrap(){rails_mismatch(&response.text(),row["body"].as_str().unwrap(),"root bot mention");}
        let jobs=app.db().read(|conn|Ok((conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Agent::DeliveryJob'",[],|r|r.get::<_,i64>(0))?,conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Bot::WebhookJob'",[],|r|r.get::<_,i64>(0))?))).await.unwrap();
        assert_eq!(jobs.0-previous,row["delivery"].as_i64().unwrap());assert_eq!(jobs.1,row["legacy"].as_i64().unwrap());previous=jobs.0;
    }
}

#[tokio::test]
async fn root_agent_mention_runs_delivery_and_posts_one_real_webhook_with_agent_identity() {
    let app=app().await;let (server,net)=net().await;
    let message=post(&app,&json!({"markdown_source":"Hey @[Bender Bot]","client_message_id":"agent-once"})).await;
    let id=event(&app,message).await;
    app.db().write(move|tx|domain::perform_delivery(tx,id)).await.unwrap();
    let job=domain::EventWebhookJob{event_id:id,attempt:Some(0)};
    post_with_network(&app.booted.app,job.clone(),&net).await.unwrap();
    // Replaying the same durable attempt must not POST again.
    post_with_network(&app.booted.app,job,&net).await.unwrap();
    let received=server.received();assert_eq!(received.len(),1);
    let body:Value=serde_json::from_slice(&received[0].body).unwrap();assert_eq!(body["agent"]["id"],oracle()["agent_id"]);
    assert!(received[0].header("X-Smartfire-Signature").is_some());
    assert_eq!(app.db().read(|conn|Ok(conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Bot::WebhookJob'",[],|r|r.get::<_,i64>(0))?)).await.unwrap(),0);
}

#[tokio::test]
async fn revoked_root_mention_cannot_post_an_agent_or_legacy_webhook() {
    let app=app().await;let (server,net)=net().await;
    let agent=oracle()["agent_id"].as_i64().unwrap();
    let grant=app.db().write(move|tx|AgentGrant::create(tx,NewGrant{agent_id:agent,room_id:Some(ALL_TALK),capability:"read_messages".into(),granted_by_id:DAVID,revoked_at:None})).await.unwrap();
    let message=post(&app,&json!({"markdown_source":"Hey @[Bender Bot]","client_message_id":"agent-revoked"})).await;
    let id=event(&app,message).await;
    app.db().write(move|tx|{let mut grant=grant;grant.revoke(tx)?;domain::perform_delivery(tx,id)}).await.unwrap();
    post_with_network(&app.booted.app,domain::EventWebhookJob{event_id:id,attempt:Some(0)},&net).await.unwrap();
    assert!(server.received().is_empty());
    assert_eq!(app.db().read(|conn|Ok(conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class IN ('Agent::EventWebhookJob','Bot::WebhookJob')",[],|r|r.get::<_,i64>(0))?)).await.unwrap(),0);
}

#[tokio::test]
async fn root_legacy_bot_without_agent_enqueues_and_posts_only_the_legacy_webhook() {
    let app=app().await;let (server,net)=net().await;
    let bot=app.db().write(|tx|{let bot=User::create_bot(tx,"Legacy Bot",Some("http://bots.example:8080/hook"))?;Room::find(tx.conn(),ALL_TALK)?.grant_to(tx,&[bot.id])?;Ok(bot.id)}).await.unwrap();
    let message=post(&app,&json!({"markdown_source":"Hey @[Legacy Bot]","client_message_id":"legacy-only"})).await;
    app.db().read(move|conn| {
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Bot::WebhookJob' AND json_extract(arguments,'$.bot_id')=?",[bot],|r|r.get::<_,i64>(0))?,1);
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Agent::DeliveryJob'",[],|r|r.get::<_,i64>(0))?,0);Ok(())
    }).await.unwrap();
    crate::integrations::jobs::deliver_webhook_with_network(&app.booted.app,crate::jobs::WebhookJob{bot_id:bot,message_id:message},&net).await.unwrap();
    let received=server.received();assert_eq!(received.len(),1);let body:Value=serde_json::from_slice(&received[0].body).unwrap();assert!(body.get("agent").is_none());
}
