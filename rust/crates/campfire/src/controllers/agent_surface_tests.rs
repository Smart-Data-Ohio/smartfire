//! Service-group contracts generated through the pinned Rails HTTP stack.
use super::agent_http_tests::{AGENT, SECRET, setup};
use super::presenters::test_support::Req;
use campfire_kit::Method;
use serde_json::Value;

async fn check(case: &Value) {
    let app = setup().await;
    let config = case["setup"].clone();
    app.db().write(move |tx| {
        tx.conn().execute("DELETE FROM agent_approvals", [])?;
        tx.conn().execute("DELETE FROM sqlite_sequence WHERE name='agent_approvals'", [])?;
        tx.conn().execute("UPDATE agents SET daily_external_action_cap=?,daily_message_cap=NULL,daily_board_post_cap=NULL WHERE id=?",rusqlite::params![config["cap"].as_i64(),AGENT])?;
        if let Some(grant) = config["grant"].as_str() {
            tx.conn().execute("INSERT INTO agent_grants(agent_id,capability,granted_by_id,created_at,updated_at) VALUES(?,?,127326141,?,?)",rusqlite::params![AGENT,grant,tx.now(),tx.now()])?;
        }
        if let Some(status) = config["approval"].as_str() {
            let approval = campfire_db::AgentApproval::create(tx,campfire_db::NewApproval { agent_id:AGENT, room_id:Some(486777696),action:"deploy".into(),summary:"Existing".into(),external_id:Some("surface-replay".into()), ..Default::default() })?;
            let expires=if status=="expired" {tx.now().ago(jiff::SignedDuration::from_mins(1))} else {tx.now().since(jiff::SignedDuration::from_hours(24))};
            tx.conn().execute("UPDATE agent_approvals SET id=900100001,status=?,expires_at=? WHERE id=?",rusqlite::params![if status=="expired" {"pending"}else{status},expires,approval.id])?;
            tx.conn().execute("UPDATE sqlite_sequence SET seq=900100001 WHERE name='agent_approvals'",[])?;
        }
        Ok(())
    }).await.unwrap();
    let request = || {
        let req=Req::new(Method::from_bytes(case["method"].as_str().unwrap().to_uppercase().as_bytes()).unwrap(),case["path"].as_str().unwrap())
            .header("authorization", &["Bearer",SECRET].join(" ")).header("accept","application/json").header("content-type","application/json");
        if let Some(body)=case["body"].as_str() {req.body(body)}else{req}
    };
    for _ in 0..case["setup"]["repeat"].as_u64().unwrap_or(0) {app.anonymous().send(request()).await;}
    let reply=app.anonymous().send(request()).await;
    let name=case["name"].as_str().unwrap();
    assert_eq!(reply.status.as_u16(),case["status"].as_u64().unwrap() as u16,"{name}: {}",reply.text());
    let body=if reply.body.is_empty() {Value::Null}else{reply.json()};
    assert_eq!(body,case["response"],"{name}");
    for (key,expected) in case["response_headers"].as_object().unwrap() {assert_eq!(reply.header(key),expected.as_str(),"{name}: {key}");}
    eprintln!("WS11-api Rails case passed: {name}");
}
async fn group(prefixes: &[&str]) {
    let vectors:Value=serde_json::from_str(include_str!("../../../../vectors/agent_surface.json")).unwrap();
    let cases:Vec<_>=vectors["cases"].as_array().unwrap().iter().filter(|case| prefixes.iter().any(|p|case["name"].as_str().unwrap().starts_with(p))).collect();
    assert!(!cases.is_empty());
    for case in cases {check(case).await;}
}
#[tokio::test]
async fn agent_surface_approvals_rest() {group(&["approvals_"]).await;}
#[tokio::test]
async fn agent_surface_approvals_mcp() {group(&["mcp_request_approval","mcp_get_approval"]).await;}
