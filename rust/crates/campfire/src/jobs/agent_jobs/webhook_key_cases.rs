//! Actual HTTP bodies/headers compared byte-for-byte with pinned WebhookAgentKeyTest probes.
use crate::integrations::test_support::{FakeResolver, FakeServer, MappingDialer, Route, network};
use super::*;
use crate::controllers::presenters::test_support::{
    ALL_TALK, BENDER, BENDER_KEY, SEED_NOW, TestApp,
};
use campfire_db::Agent;
use rusqlite::params;
use serde_json::Value;
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};
fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/agents_webhook_paths_contract.json"
    ))
    .unwrap()
}
async fn capture(
    case: &str,
) -> (
    App,
    tempfile::TempDir,
    Vec<crate::integrations::test_support::Received>,
) {
    let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
        SEED_NOW.parse().unwrap(),
    ));
    let (app, dir) = TestApp::boot_with_clock(clock)
        .await
        .expect("default seed")
        .stop_jobs()
        .await;
    let name = case.to_owned();
    let secret = oracle()["secret"].as_str().unwrap().to_owned();
    let crypto = app.ar_encryption.clone();
    let (e,legacy)=app.db.write(move|tx| {
        let aid=Agent::for_user(tx.conn(),BENDER)?.unwrap().id;
        tx.conn().execute("UPDATE agents SET webhook_signing_secret=?,owner_id=? WHERE id=?",params![crypto.encrypt(&secret),if name=="ownerless" {None} else {Some(127326141)},aid])?;
        tx.conn().execute("UPDATE webhooks SET url='http://bots.example:8080/hook',signing_secret=NULL WHERE user_id=?",[BENDER])?;
        let legacy=name=="legacy"||name=="without_context";
        if name=="legacy" {Agent::find(tx.conn(),aid)?.unwrap().destroy(tx)?;return Ok((None,true))}
        if name=="public_pr"||name=="ordinary_thread" {
            let thread=if name=="ordinary_thread" {900191002} else {900191001};
            tx.conn().execute("INSERT INTO channel_threads(id,room_id,creator_id,name,parent_message_id,last_activity_at,created_at,updated_at) VALUES (?,?,127326141,?,?,?,?,?)",params![thread,ALL_TALK,if name=="ordinary_thread" {"Ordinary chat"} else {"PR chat"},if name=="ordinary_thread" {None} else {Some(136976342)},tx.now(),tx.now(),tx.now()])?;
            tx.conn().execute("UPDATE messages SET thread_id=? WHERE id=136976342",[thread])?;
            if name=="public_pr" {
                tx.conn().execute("INSERT INTO github_pull_requests(id,owner,repo,number,title,state,head_branch,base_branch,review_decision,check_status,html_url,private,created_at,updated_at) VALUES (900191011,'rails','rails',12,'Fix login','open','shiny','main','approved','passing','https://github.com/rails/rails/pull/12',0,?,?)",params![tx.now(),tx.now()])?;
                tx.conn().execute("INSERT INTO github_pull_request_threads(github_pull_request_id,room_id,channel_thread_id,created_at,updated_at) VALUES (900191011,?,900191001,?,?)",params![ALL_TALK,tx.now(),tx.now()])?;
            }
        }
        if name=="approval"||name=="completion" {
            tx.conn().execute("INSERT INTO agent_approvals(id,agent_id,action,summary,status,expires_at,created_at,updated_at) VALUES (900191101,?,'deploy','Ship it','pending',?,?,?)",params![aid,tx.now().since(jiff::SignedDuration::from_hours(24)),tx.now(),tx.now()])?;
        }
        let (delivery,kind,metadata,approval,message)=match name.as_str() {
            "ownerless" => (7,"mention",serde_json::json!({}),None,Some(136976342)),
            "public_pr" => (9,"mention",serde_json::json!({}),None,Some(136976342)),
            "ordinary_thread" => (10,"mention",serde_json::json!({}),None,Some(136976342)),
            "ordinary_room" => (11,"mention",serde_json::json!({}),None,Some(136976342)),
            "approval" => (7,"approval_decided",serde_json::json!({}),Some(900191101),None),
            "work" => (900191201,"work_assigned",serde_json::json!({"work_snapshot":{"title":"Signed work"}}),None,None),
            "completion" => (900191202,"github_action_completed",serde_json::json!({"approval_id":900191101,"action":"github.comment","status":"completed"}),None,None),
            _ => (123,"mention",serde_json::json!({}),None,Some(136976342)),
        };
        let mut e=AgentEvent::create(tx,domain::NewEvent {agent_id:aid,event_type:kind.into(),outcome:Some("delivered".into()),room_id:Some(ALL_TALK),message_id:message,agent_approval_id:approval,metadata,..Default::default()})?;
        // Webhook#deliver accepts a supplied delivery id independently of the ledger.
        e.id=delivery;
        Ok((Some(e),legacy))
    }).await.unwrap();
    let server = FakeServer::start(vec![Route::new("POST", "*", "/hook", 204)]).await;
    let net = network(
        Arc::new(FakeResolver::new([("bots.example", vec!["93.184.216.34"])])),
        Arc::new(MappingDialer {
            public: HashSet::from(["93.184.216.34".parse().unwrap()]),
            to: server.addr,
            dialed: Mutex::new(vec![]),
        }),
    );
    if legacy {
        jobs::deliver_webhook_with_network(
            &app,
            crate::queue::WebhookJob {
                bot_id: BENDER,
                message_id: 136976342,
            },
            &net,
        )
        .await
        .unwrap();
    } else {
        assert!(matches!(
            post_event(&app, &e.unwrap(), &net).await,
            AttemptOutcome::Delivered
        ));
    }
    let requests = server.received();
    assert_eq!(requests.len(), 1);
    let expected = &oracle()["cases"][case];
    assert_eq!(
        requests[0].body,
        expected["body"].as_str().unwrap().as_bytes(),
        "Rails raw payload {case}"
    );
    assert_eq!(
        requests[0].header("X-Smartfire-Timestamp"),
        expected["timestamp"].as_str()
    );
    assert_eq!(
        requests[0].header("X-Smartfire-Signature"),
        expected["signature"].as_str()
    );
    assert!(!String::from_utf8_lossy(&requests[0].body).contains(BENDER_KEY));
    (app, dir, requests)
}
#[tokio::test]
async fn ws11_webhook_key_without_agent_context() {
    capture("without_context").await;
}
#[tokio::test]
async fn ws11_webhook_key_legacy_signed_reply_path() {
    let (app, _dir, requests) = capture("legacy").await;
    let p: Value = serde_json::from_slice(&requests[0].body).unwrap();
    let reply = p["reply_url"].as_str().unwrap();
    assert_eq!(p["room"]["path"], reply);
    let token = reply.split('/').nth(3).unwrap();
    let token = percent_encoding::percent_decode_str(token)
        .decode_utf8()
        .unwrap();
    let claims = rails_compat::verifiers::bot_reply::verify(
        &app.secrets,
        &token,
        &ALL_TALK.to_string(),
        app.clock.now(),
    )
    .unwrap();
    assert_eq!(claims, serde_json::json!(BENDER));
}
#[tokio::test]
async fn ws11_webhook_key_agent_has_no_reply_url() {
    let (_app, _dir, r) = capture("with_context").await;
    let p: Value = serde_json::from_slice(&r[0].body).unwrap();
    assert!(!p.as_object().unwrap().contains_key("reply_url"));
}
#[tokio::test]
async fn ws11_webhook_key_additive_context() {
    capture("with_context").await;
}
#[tokio::test]
async fn ws11_webhook_key_ownerless() {
    capture("ownerless").await;
}
#[tokio::test]
async fn ws11_webhook_key_public_pr_thread() {
    capture("public_pr").await;
}
#[tokio::test]
async fn ws11_webhook_key_raw_body_signature() {
    capture("with_context").await;
}
#[tokio::test]
async fn ws11_webhook_key_nonmessage_signatures() {
    for c in ["approval", "work", "completion"] {
        capture(c).await;
    }
}
#[tokio::test]
async fn ws11_webhook_key_nonpr_thread_and_room() {
    capture("ordinary_thread").await;
    capture("ordinary_room").await;
}
