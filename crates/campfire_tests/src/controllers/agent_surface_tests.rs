//! Service-group contracts generated through the pinned Rails HTTP stack.
use super::agent_http_tests::{AGENT, SECRET, setup};
use super::presenters::test_support::Req;
use campfire_kit::Method;
use serde_json::Value;

async fn check(case: &Value) {
    let app = setup().await;
    let config = case["setup"].clone();
    let encryption = app.booted.app.ar_encryption.clone();
    app.db().write(move |tx| {
        tx.conn().execute("DELETE FROM agent_approvals", [])?;
        tx.conn().execute("DELETE FROM sqlite_sequence WHERE name='agent_approvals'", [])?;
        tx.conn().execute("UPDATE agents SET owner_id=?,daily_external_action_cap=?,daily_message_cap=?,daily_board_post_cap=NULL WHERE id=?",rusqlite::params![if config["ownerless"].as_bool()==Some(true) {None}else{Some(127326141)},config["cap"].as_i64(),config["message_cap"].as_i64(),AGENT])?;
        let grants:Vec<_>=if let Some(array)=config["grant"].as_array(){array.iter().filter_map(Value::as_str).collect()}else{config["grant"].as_str().into_iter().collect()};
        for grant in grants {
            tx.conn().execute("INSERT INTO agent_grants(agent_id,capability,granted_by_id,created_at,updated_at) VALUES(?,?,127326141,?,?)",rusqlite::params![AGENT,grant,tx.now(),tx.now()])?;
        }
        if let Some(status) = config["approval"].as_str() {
            let approval = campfire_db::AgentApproval::create(tx,campfire_db::NewApproval { agent_id:AGENT, room_id:Some(486777696),action:"deploy".into(),summary:"Existing".into(),external_id:Some("surface-replay".into()), ..Default::default() })?;
            let expires=if status=="expired" {tx.now().ago(jiff::SignedDuration::from_mins(1))} else {tx.now().since(jiff::SignedDuration::from_hours(24))};
            tx.conn().execute("UPDATE agent_approvals SET id=900100001,status=?,expires_at=? WHERE id=?",rusqlite::params![if status=="expired" {"pending"}else{status},expires,approval.id])?;
            tx.conn().execute("UPDATE sqlite_sequence SET seq=900100001 WHERE name='agent_approvals'",[])?;
        }
        tx.conn().execute("UPDATE messages SET creator_id=?,streaming=0 WHERE id=935961918",[if config["own_message"].as_bool()==Some(true){394959859}else{127326141}])?;
        tx.conn().execute("DELETE FROM channel_threads WHERE id=900200001",[])?;
        if config["owned_work"].as_bool()==Some(true) {tx.conn().execute("INSERT INTO channel_threads(id,name,room_id,creator_id,work_owner_id,work_status,last_activity_at,created_at,updated_at) VALUES(900200001,'Owned',486777696,394959859,394959859,'in_progress',?,?,?)",rusqlite::params![tx.now(),tx.now(),tx.now()])?;}
        tx.conn().execute("DELETE FROM fizzy_connected_accounts",[])?;
        tx.conn().execute("DELETE FROM github_connected_accounts",[])?;
        if config["fizzy_account"].as_bool()==Some(true) {tx.conn().execute("INSERT INTO fizzy_connected_accounts(user_id,fizzy_account_id,access_token,created_at,updated_at) VALUES(127326141,'acct',?,?,?)",rusqlite::params![encryption.encrypt("ws11api-obviously-fake-fizzy"),tx.now(),tx.now()])?;}
        if config["bad_fizzy_token"].as_bool()==Some(true) {tx.conn().execute("UPDATE fizzy_connected_accounts SET access_token='unreadable-fixture'",[])?;}
        if let Some(account)=config["github_account"].as_str(){tx.conn().execute("INSERT INTO github_connected_accounts(user_id,github_login,access_token,token_source,token_expires_at,created_at,updated_at) VALUES(?,'fixture',?,?,?,?,?)",rusqlite::params![if account=="agent_pat"{394959859}else{127326141},encryption.encrypt("ws11api-obviously-fake-github"),if account=="owner_app"{"app"}else{"pat"},tx.now().ago(jiff::SignedDuration::from_hours(1)),tx.now(),tx.now()])?;}
        if config["github_pr"].as_bool()==Some(true) {
            tx.conn().execute("INSERT INTO github_pull_requests(id,owner,repo,number,created_at,updated_at) VALUES(900400001,'fixture','fixture',1,?,?)",rusqlite::params![tx.now(),tx.now()])?;
            tx.conn().execute("INSERT INTO channel_threads(id,name,room_id,creator_id,last_activity_at,created_at,updated_at) VALUES(900400002,'PR',486777696,394959859,?,?,?)",rusqlite::params![tx.now(),tx.now(),tx.now()])?;
            tx.conn().execute("INSERT INTO github_pull_request_threads(github_pull_request_id,room_id,channel_thread_id,created_at,updated_at) VALUES(900400001,486777696,900400002,?,?)",rusqlite::params![tx.now(),tx.now()])?;
        }
        Ok(())
    }).await.unwrap();
    let request = || {
        let req = Req::new(
            Method::from_bytes(case["method"].as_str().unwrap().to_uppercase().as_bytes()).unwrap(),
            case["path"].as_str().unwrap(),
        )
        .header("accept", "application/json")
        .header("content-type", "application/json");
        let req = if case["setup"]["human_session"].as_bool() == Some(true) {
            req
        } else {
            req.header(
                "authorization",
                &[
                    "Bearer",
                    if case["setup"]["invalid_token"].as_bool() == Some(true) {
                        "unrecognized-fixture"
                    } else {
                        SECRET
                    },
                ]
                .join(" "),
            )
        };
        if let Some(body) = case["body"].as_str() {
            req.body(body)
        } else {
            req
        }
    };
    for _ in 0..case["setup"]["repeat"].as_u64().unwrap_or(0) {
        app.anonymous().send(request()).await;
    }
    let reply = if case["setup"]["human_session"].as_bool() == Some(true) {
        app.david().send(request()).await
    } else {
        app.anonymous().send(request()).await
    };
    let name = case["name"].as_str().unwrap();
    assert_eq!(
        reply.status.as_u16(),
        case["status"].as_u64().unwrap() as u16,
        "{name}: {}",
        reply.text()
    );
    let body = if reply.body.is_empty() {
        Value::Null
    } else {
        reply.json()
    };
    assert_eq!(body, case["response"], "{name}");
    assert_eq!(
        reply.text(),
        case["response_body"].as_str().unwrap(),
        "{name}: raw body"
    );
    for (key, expected) in case["response_headers"].as_object().unwrap() {
        assert_eq!(reply.header(key), expected.as_str(), "{name}: {key}");
    }
    eprintln!("WS11-api Rails case passed: {name}");
}
async fn group(prefixes: &[&str]) {
    let vectors: Value =
        serde_json::from_str(include_str!("../../../../vectors/agent_surface.json")).unwrap();
    let cases: Vec<_> = vectors["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| {
            prefixes
                .iter()
                .any(|p| case["name"].as_str().unwrap().starts_with(p))
        })
        .collect();
    assert!(!cases.is_empty());
    for case in cases {
        check(case).await;
    }
}
#[tokio::test]
async fn agent_surface_approvals_rest() {
    group(&["approvals_"]).await;
}
#[tokio::test]
async fn agent_surface_approvals_mcp() {
    group(&["mcp_request_approval", "mcp_get_approval"]).await;
}

#[tokio::test]
async fn agent_surface_conversation_rest() {
    group(&["context_", "dm_", "message_", "stream_", "pin_", "poll_"]).await;
}
#[tokio::test]
async fn agent_surface_conversation_mcp() {
    group(&[
        "mcp_context_",
        "mcp_reading_",
        "mcp_message_",
        "mcp_dm_",
        "mcp_reaction_",
        "mcp_stream_",
        "mcp_pin_",
        "mcp_poll_",
    ])
    .await;
}
#[tokio::test]
async fn agent_surface_work_rest() {
    group(&["posts_", "work_"]).await;
}
#[tokio::test]
async fn agent_surface_work_mcp() {
    group(&["mcp_posts_", "mcp_work_"]).await;
}

#[tokio::test]
async fn agent_surface_integrations_rest() {
    group(&["fizzy_", "github_"]).await;
}
#[tokio::test]
async fn agent_surface_integrations_mcp() {
    group(&["mcp_fizzy_"]).await;
}

#[tokio::test]
async fn agent_surface_matrix_rest() {
    group(&["matrix_rest_"]).await;
}
#[tokio::test]
async fn agent_surface_matrix_mcp() {
    group(&["mcp_matrix_"]).await;
}
