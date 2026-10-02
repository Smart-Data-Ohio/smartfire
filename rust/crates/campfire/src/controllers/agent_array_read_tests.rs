//! Whole-array Active Record lookups: missing candidates cannot add reader queries.
use super::agent_http_tests::SECRET;
use super::agent_review_tests::readers;
use super::presenters::test_support::{Req, TestApp};
use campfire_kit::Method;
use serde_json::{Value, json};

fn request(path: &str, body: Option<Value>) -> Req {
    let mut req = Req::new(if body.is_some() { Method::POST } else { Method::GET }, path)
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .header("authorization", &format!("Bearer {SECRET}"));
    if let Some(body) = body { req = req.body(body.to_string()); }
    req
}
fn rpc(tool: &str, arguments: Value) -> Value {
    json!({"jsonrpc":"2.0","id":196,"method":"tools/call","params":{"name":tool,"arguments":arguments}})
}
async fn measure(app: &TestApp, label: &str, size: usize, path: &str, body: Option<Value>) -> usize {
    app.anonymous().send(request(path, body.clone())).await;
    let warm = app.anonymous().send(request(path, body.clone())).await;
    assert_eq!(warm.status, 200, "{label}: {}", warm.text());
    let queries = if label=="react" {app.db().capture_queries()} else {app.db().capture_read_queries()};
    let reply = app.anonymous().send(request(path, body)).await;
    app.db().stop_capturing_queries();
    assert_eq!(reply.status, 200);
    assert_eq!(reply.body, warm.body, "{label}: identical requests");
    assert!(reply.json().get("error").is_none());
    assert_ne!(reply.json()["result"]["isError"], json!(true));
    let count = queries.lock().unwrap().iter().filter(|sql| sql.trim_start().to_ascii_uppercase().starts_with("SELECT")).count();
    println!("WS11-api array lookup: {label}; missing={size}; SELECTs={count}");
    count
}
#[tokio::test]
async fn ws11_array_candidate_select_counts_stay_flat() {
    let app = readers().await;
    let mut failures = vec![];
    for label in ["thread", "before", "after", "react", "context_message", "context_thread", "rest_context_message", "rest_context_thread"] {
        let mut counts = vec![];
        for size in [5, 50] {
            let mut ids = (1..=size as i64).map(|n| json!(-n)).collect::<Vec<_>>();
            ids.push(json!(if label.contains("thread") || label=="get_work" {1996000000} else if label=="before" {1996002049} else {1996002000}));
            let (path, body) = if label.starts_with("rest_") {
                let key = if label.ends_with("thread") {"thread_id"} else {"message_id"};
                (format!("/agents/context?{}&limit=5", ids.iter().map(|id|format!("{key}%5B%5D={id}")).collect::<Vec<_>>().join("&")), None)
            } else {
                let (name,args) = match label {
                    "thread" => ("read_messages", json!({"thread_id":ids,"limit":5})),
                    "before" | "after" => ("read_messages", json!({"thread_id":1996000000,label:ids,"limit":5})),
                    "react" => ("react", json!({"message_id":ids,"content":"👍"})),
                    "context_message" => ("get_context", json!({"message_id":ids,"limit":5})),
                    "context_thread" => ("get_context", json!({"thread_id":ids,"limit":5})),
                    _ => unreachable!(),
                };
                ("/agents/mcp".to_owned(), Some(rpc(name,args)))
            };
            counts.push(measure(&app,label,size,&path,body).await);
        }
        if counts[0]!=counts[1] { failures.push(format!("{label}: {counts:?}")); }
    }
    assert!(failures.is_empty(), "whole-array queries must stay flat: {}", failures.join(", "));
}

#[tokio::test]
async fn ws11_array_reads_match_pinned_rails_bytes_and_selection() {
    let vector:Value=serde_json::from_str(include_str!("../../../../vectors/agent_array_reads_http.json")).unwrap();
    for case in vector["cases"].as_array().unwrap() {
        let app=readers().await;
        let revoked=case["setup"]["revoked"]==true;let inactive=case["setup"]["inactive"]==true;
        app.db().write(move|tx|{
            tx.conn().execute("INSERT INTO channel_threads(id,name,room_id,creator_id,last_activity_at,created_at,updated_at) VALUES(1986000000,'Hidden',201306877,127326141,?,?,?)",rusqlite::params![tx.now(),tx.now(),tx.now()])?;
            if revoked {tx.conn().execute("INSERT INTO agent_grants(agent_id,capability,granted_by_id,revoked_at,created_at,updated_at) VALUES(773018776,'read_messages',127326141,?,?,?)",rusqlite::params![tx.now(),tx.now(),tx.now()])?;}
            if inactive {tx.conn().execute("UPDATE agents SET status='suspended',suspended_at=? WHERE id=773018776",[tx.now()])?;}
            Ok(())
        }).await.unwrap();
        let body=case["body"].as_str().map(|raw|serde_json::from_str::<Value>(raw).unwrap());
        // The Rails receipt is measured on the second request, including reaction replay.
        app.anonymous().send(request(case["path"].as_str().unwrap(),body.clone())).await;
        let reply=app.anonymous().send(request(case["path"].as_str().unwrap(),body)).await;
        assert_eq!(reply.status.as_u16(),case["status"].as_u64().unwrap() as u16,"{}",case["name"]);
        assert_eq!(reply.text(),case["response"].as_str().unwrap(),"{}",case["name"]);
        for (name,value) in case["headers"].as_object().unwrap(){assert_eq!(reply.header(name),value.as_str(),"{}: {name}",case["name"]);}
    }
}
#[tokio::test]
async fn ws11_array_reads_use_fewer_binds_than_sqlite_limit() {
    let app=readers().await;
    let mut ids=(-40000..0).collect::<Vec<i64>>();ids.push(1996000000);
    let reply=app.anonymous().send(request("/agents/mcp",Some(rpc("read_messages",json!({"thread_id":ids,"limit":5}))))).await;
    assert_eq!(reply.status,200);assert_eq!(reply.json()["result"]["structuredContent"]["messages"].as_array().unwrap().len(),5);
    let mut ids=(-40000..0).collect::<Vec<i64>>();ids.push(1996002049);
    let reply=app.anonymous().send(request("/agents/mcp",Some(rpc("get_context",json!({"message_id":ids,"limit":5}))))).await;
    assert_eq!(reply.status,200);assert_eq!(reply.json()["result"]["isError"],false);
}
