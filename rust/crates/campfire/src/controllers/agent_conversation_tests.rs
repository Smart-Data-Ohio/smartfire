//! Full-router Rails status, headers, raw JSON bytes and persisted-row deltas.
use super::agent_http_tests::{AGENT, SECRET, setup};
use super::presenters::test_support::Req;
use campfire_db::{Message, NewMessage};
use campfire_kit::Method;
use serde_json::{Value, json};

async fn check(case: &Value) {
    let app = setup().await;
    let config = case["setup"].clone();
    app.db().write(move |tx| {
        tx.conn().execute("UPDATE agents SET daily_message_cap=?,owner_id=127326141 WHERE id=?",rusqlite::params![config["cap"].as_i64(),AGENT])?;
        tx.conn().execute("UPDATE sqlite_sequence SET seq=1900600000 WHERE name='messages'",[])?;
        for (creator_id, client, markdown, thread_id) in [(127326141,"wire-source","Source α & β",None),(394959859,"wire-agent","Agent",None),(394959859,"wire-thread","Thread source",Some(1900600008))] {
            if thread_id.is_some() {
                tx.conn().execute("INSERT INTO channel_threads(id,name,room_id,creator_id,parent_message_id,last_activity_at,created_at,updated_at) VALUES(1900600008,'Wire thread',486777696,394959859,1900600001,?,?,?)",rusqlite::params![tx.now(),tx.now(),tx.now()])?;
            }
            Message::create(tx,NewMessage{room_id:486777696,creator_id,client_message_id:Some(client.into()),markdown_source:Some(markdown.into()),thread_id,streaming:if client=="wire-agent" {config["stream"].as_bool()==Some(true)} else if client=="wire-thread" {config["thread_stream"].as_bool()==Some(true)} else {false},..Default::default()})?;
        }
        if config["stream"].as_bool()==Some(true) {tx.conn().execute("UPDATE messages SET streaming=1,streaming_updated_at=? WHERE id=1900600002",[tx.now()])?;}
        if config["thread_stream"].as_bool()==Some(true) {tx.conn().execute("UPDATE messages SET streaming=1,streaming_updated_at=? WHERE id=1900600003",[tx.now()])?;}
        if config["locked"].as_bool()==Some(true) {tx.conn().execute("UPDATE channel_threads SET locked_at=? WHERE id=1900600008",[tx.now()])?;}
        tx.conn().execute("UPDATE sqlite_sequence SET seq=1900600010 WHERE name='messages'",[])?;
        tx.conn().execute("UPDATE sqlite_sequence SET seq=1900600020 WHERE name='rooms'",[])?;
        Ok(())
    }).await.unwrap();
    let counts = |conn: &campfire_db::Connection| -> campfire_db::Result<Value> {
        let mut counts = serde_json::Map::new();
        for table in ["messages", "rooms", "drive_attachments"] {
            let count: i64 =
                conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))?;
            counts.insert(table.into(), json!(count));
        }
        Ok(Value::Object(counts))
    };
    let before = app.db().read(counts).await.unwrap();
    let mut req = Req::new(
        Method::from_bytes(case["method"].as_str().unwrap().to_uppercase().as_bytes()).unwrap(),
        case["path"].as_str().unwrap(),
    )
    .header("accept", "application/json")
    .header("content-type", "application/json")
    .header("authorization", &["Bearer", SECRET].join(" "));
    if let Some(body) = case["body"].as_str() {
        req = req.body(body);
    }
    let reply = app.anonymous().send(req).await;
    let name = case["name"].as_str().unwrap();
    assert_eq!(
        reply.status.as_u16(),
        case["status"].as_u64().unwrap() as u16,
        "{name}: {}",
        reply.text()
    );
    assert_eq!(
        reply.text(),
        case["response_body"].as_str().unwrap(),
        "{name}: raw JSON bytes"
    );
    for header in [
        "Content-Type",
        "Cache-Control",
        "Pragma",
        "Retry-After",
        "Location",
    ] {
        assert_eq!(
            reply.header(header),
            case["response_headers"][header.to_ascii_lowercase()].as_str(),
            "{name}: {header}"
        );
    }
    let after = app.db().read(counts).await.unwrap();
    for table in ["messages", "rooms", "drive_attachments"] {
        assert_eq!(
            after[table].as_i64().unwrap() - before[table].as_i64().unwrap(),
            case["delta"][table].as_i64().unwrap(),
            "{name}: {table} delta"
        );
    }
}

async fn group(prefix: &str) {
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agent_conversation_http.json"
    ))
    .unwrap();
    for case in vectors["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["name"].as_str().unwrap().starts_with(prefix))
    {
        check(case).await;
    }
}
#[tokio::test]
async fn agent_conversation_context_bytes() {
    group("context_").await;
}
#[tokio::test]
async fn agent_conversation_post_bytes() {
    group("post_").await;
}
#[tokio::test]
async fn agent_conversation_dm_bytes() {
    group("dm_").await;
}
#[tokio::test]
async fn agent_conversation_mcp_bytes() {
    group("mcp_").await;
}

#[tokio::test]
async fn agent_conversation_stream_bytes() {
    group("stream_").await;
}

async fn stream_http_enqueue_failure(mcp: bool) {
        let app=setup().await;
        let id=app.db().write(|tx| {
            let campfire_db::models::agent_posting::PostResult::Posted(message)=campfire_db::models::agent_streaming::start(tx,AGENT,NewMessage{room_id:486777696,markdown_source:Some("Before".into()),client_message_id:Some("atomic-stream".into()),..Default::default()})? else {panic!("stream start");};
            tx.conn().execute("UPDATE messages SET stream_broadcast_at=? WHERE id=?",rusqlite::params![tx.now(),message.id])?;
            tx.conn().execute_batch("CREATE TRIGGER ws11api_reject_trailing BEFORE INSERT ON background_jobs WHEN NEW.job_class='Message::StreamTrailingBroadcastJob' BEGIN SELECT RAISE(ABORT,'WS11-api rejected trailing job'); END;")?;
            Ok(message.id)
        }).await.unwrap();
        let (method,path,body)=if mcp {(Method::POST,"/agents/mcp".to_owned(),json!({"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"append_stream","arguments":{"message_id":id,"append":"After"}}}))} else {(Method::PATCH,format!("/agents/streaming_messages/{id}"),json!({"append":"After"}))};
        let reply=app.anonymous().send(Req::new(method,&path).header("accept","application/json").header("content-type","application/json").header("authorization",&["Bearer",SECRET].join(" ")).body(body.to_string())).await;
        if mcp {assert_eq!(reply.status.as_u16(),200);assert_eq!(reply.json()["error"]["code"],-32603);} else {assert_eq!(reply.status.as_u16(),500);}
        app.db().read(move|conn|{
            let message=Message::find(conn,id)?;
            assert_eq!(message.markdown_source.as_deref(),Some("Before"));
            assert!(message.streaming);
            let count:i64=conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Message::StreamTrailingBroadcastJob'",[],|r|r.get(0))?;
            assert_eq!(count,0);
            Ok(())
        }).await.unwrap();
}

#[tokio::test] async fn agent_stream_http_enqueue_failure_rest() {stream_http_enqueue_failure(false).await;}
#[tokio::test] async fn agent_stream_http_enqueue_failure_mcp() {stream_http_enqueue_failure(true).await;}
