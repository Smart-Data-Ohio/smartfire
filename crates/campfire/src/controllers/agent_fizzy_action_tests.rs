//! Production-router approval requests, exact Rails wire bodies and stored execution payloads.
use super::agent_http_tests::{AGENT, SECRET, initialize};
use super::presenters::test_support::{Req, TestApp};
use crate::integrations::fizzy::{
    State,
    accounts::{Account, Input},
};
use crate::integrations::test_support::{FakeResolver, MappingDialer, network};
use campfire_kit::Method;
use serde_json::{Value, json};
use std::sync::Arc;

async fn check(case: &Value) {
    let config = case["setup"].clone();
    let resolver = Arc::new(FakeResolver::new([]));
    let dialer = Arc::new(MappingDialer {
        public: Default::default(),
        to: "127.0.0.1:52900".parse().unwrap(),
        dialed: Default::default(),
    });
    let clock = Arc::new(campfire_kit::FrozenClock::new("2026-03-02T16:00:00Z".parse().unwrap()));
    let app = TestApp::boot_with_fizzy(
        clock,
        State {
            network: network(resolver.clone(), dialer.clone()),
            base: "https://app.fizzy.do".into(),
        },
    )
    .await
    .unwrap();
    initialize(&app).await;
    let crypto = app.booted.app.ar_encryption.clone();
    app.db().write(move |tx| {
        tx.conn().execute("UPDATE agents SET owner_id=?,daily_external_action_cap=? WHERE id=?",rusqlite::params![if config["no_owner"]==true {None} else {Some(127326141)},config["cap"].as_i64(),AGENT])?;
        if config["no_grant"]!=true {
            tx.conn().execute("INSERT INTO agent_grants(agent_id,capability,room_id,granted_by_id,created_at,updated_at) VALUES(?,'external_action',?,127326141,?,?)",rusqlite::params![AGENT,if config["room_grant"]==true {Some(486777696)} else {None},tx.now(),tx.now()])?;
        }
        tx.conn().execute("UPDATE users SET time_zone=? WHERE id=394959859",[config["zone"].as_str()])?;
        tx.conn().execute("UPDATE users SET inbox_preferences=? WHERE id=127326141",[if config.get("preference").is_some() {json!({"agent_approvals":config["preference"]}).to_string()} else {"{}".into()}])?;
        Account::disconnect(tx,127326141)?;
        if config["no_account"]!=true {
            let account=Account::create(tx,&crypto,&Input{user_id:127326141,account_id:"897362094",account_name:None,fizzy_user_id:Some("owner-id"),fizzy_user_name:Some("Fixture Owner"),token:"fixture-owner"})?;
            if config["disconnected"]==true {account.mark_disconnected(tx,"Disconnected")?;}
            if config["corrupt"]==true {tx.conn().execute("UPDATE fizzy_connected_accounts SET access_token='not encrypted' WHERE id=?",[account.id])?;}
        }
        tx.conn().execute("DELETE FROM activity_items WHERE source_type='AgentApproval' AND source_id IN (SELECT id FROM agent_approvals WHERE agent_id=?)",[AGENT])?;
        tx.conn().execute("DELETE FROM agent_approvals WHERE agent_id=?",[AGENT])?;
        tx.conn().execute("DELETE FROM agent_budget_notices WHERE agent_id=?",[AGENT])?;
        tx.conn().execute("UPDATE sqlite_sequence SET seq=1900700000 WHERE name='agent_approvals'",[])?;
        if config["replay"]==true {
            let approval=campfire_db::AgentApproval::create(tx,campfire_db::NewApproval{agent_id:AGENT,action:"fizzy.close".into(),summary:"Previous".into(),payload:Some("{\"kind\":\"close\"}".into()),external_id:if config["null_external"]==true {None} else {Some("wire-action".into())},..Default::default()})?;
            if config["expired"]==true {tx.conn().execute("UPDATE agent_approvals SET expires_at=? WHERE id=?",rusqlite::params![tx.now().since(jiff::SignedDuration::from_secs(-1)),approval.id])?;}
        }
        Ok(())
    }).await.unwrap();
    let reply = app
        .anonymous()
        .send(
            Req::new(Method::POST, case["path"].as_str().unwrap())
                .header("accept", "application/json")
                .header("content-type", "application/json")
                .header("authorization", &["Bearer", SECRET].join(" "))
                .body(case["body"].as_str().unwrap()),
        )
        .await;
    let name = case["name"].as_str().unwrap();
    assert_eq!(reply.status.as_u16(), case["status"].as_u64().unwrap() as u16, "{name}: {}", reply.text());
    assert_eq!(reply.text(), case["response_body"].as_str().unwrap(), "{name}: raw response bytes");
    for header in ["Content-Type", "Cache-Control", "Pragma", "Retry-After", "Location"] {
        assert_eq!(
            reply.header(header),
            case["response_headers"][header.to_ascii_lowercase()].as_str(),
            "{name}: {header}"
        );
    }
    assert!(resolver.lookups().is_empty(), "requesting approval must never contact Fizzy");
    assert!(dialer.dialed.lock().unwrap().is_empty());
    let (approvals,notices,reason,inbox)=app.db().read(|c| {
        let credential:i64=c.query_row("SELECT id FROM agent_credentials WHERE agent_id=?",[AGENT],|r|r.get(0))?;
        let account=Account::for_user(c,127326141)?;
        let account_id=account.as_ref().map(|a|a.id);
        let rows=c.prepare("SELECT action,summary,payload,external_id,status,expires_at,agent_credential_id,fizzy_connected_account_id,fizzy_user_id,fizzy_user_name FROM agent_approvals WHERE agent_id=? ORDER BY id")?.query_map([AGENT],|r| {
            let expires:campfire_db::Timestamp=r.get(5)?;
            Ok(json!({"action":r.get::<_,String>(0)?,"summary":r.get::<_,String>(1)?,"payload":r.get::<_,Option<String>>(2)?,"external_id":r.get::<_,Option<String>>(3)?,"status":r.get::<_,String>(4)?,"expires_at":format!("{}.{:03}Z",expires.jiff().strftime("%Y-%m-%dT%H:%M:%S"),expires.subsec_microsecond()/1000),"credential_matches":r.get::<_,Option<i64>>(6)?==Some(credential),"account_matches":r.get::<_,Option<i64>>(7)?==account_id,"fizzy_user_id":r.get::<_,Option<String>>(8)?,"fizzy_user_name":r.get::<_,Option<String>>(9)?}))
        })?.collect::<rusqlite::Result<Vec<Value>>>()?;
        let notices=c.prepare("SELECT cap,day FROM agent_budget_notices WHERE agent_id=? ORDER BY id")?.query_map([AGENT],|r|Ok(json!({"cap":r.get::<_,String>(0)?,"day":r.get::<_,String>(1)?})))?.collect::<rusqlite::Result<Vec<Value>>>()?;
        let inbox=c.prepare("SELECT user_id,event_type FROM activity_items WHERE source_type='AgentApproval' AND source_id IN (SELECT id FROM agent_approvals WHERE agent_id=?) ORDER BY user_id")?.query_map([AGENT],|r|Ok(json!({"user":r.get::<_,i64>(0)?,"event":r.get::<_,String>(1)?})))?.collect::<rusqlite::Result<Vec<Value>>>()?;
        Ok((json!(rows),json!(notices),account.and_then(|a|a.disconnected_reason),json!(inbox)))
    }).await.unwrap();
    assert_eq!(approvals, case["approvals"], "{name}: stored execution payload and identity");
    assert_eq!(inbox,case["inbox"],"{name}: committed approval inbox");
    assert_eq!(notices, case["notices"], "{name}: budget notice");
    assert_eq!(json!(reason), case["reason"], "{name}: account usability");
    println!("WS11-api Fizzy approval wire case {name}: 1 passed; 0 failed");
}
fn cases() -> Vec<Value> {
    serde_json::from_str::<Value>(include_str!("../../../../vectors/agent_fizzy_action_http.json")).unwrap()["cases"]
        .as_array()
        .unwrap()
        .clone()
}
#[tokio::test]
async fn fizzy_action_rest_wire_successes() {
    for case in cases().into_iter().take(10).filter(|c| c["name"].as_str().unwrap().starts_with("rest_")) {
        check(&case).await;
    }
}
#[tokio::test]
async fn fizzy_action_mcp_wire_successes() {
    for case in cases().into_iter().take(10).filter(|c| c["name"].as_str().unwrap().starts_with("mcp_")) {
        check(&case).await;
    }
}
#[tokio::test]
async fn fizzy_action_wire_errors_replays_and_coercions() {
    for case in cases().into_iter().skip(10) {
        check(&case).await;
    }
}

#[tokio::test]
async fn fizzy_action_replay_types() {for case in cases().into_iter().filter(|c|c["name"].as_str().unwrap().contains("replay_type_")) {check(&case).await;}}
