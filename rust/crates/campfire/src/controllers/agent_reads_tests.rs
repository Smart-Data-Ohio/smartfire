//! Rails wire bytes for room/history/board/work reads through the production router.
use super::agent_http_tests::{AGENT, SECRET, setup};
use super::presenters::test_support::Req;
use campfire_db::{Message, NewMessage};
use campfire_kit::Method;
use serde_json::Value;

pub(super) async fn check(case: &Value) {
    let app = setup().await.without_job_runner().await;
    let config = case["setup"].clone();
    app.db().write(move |tx| {
        for cap in config["grant"].as_array().into_iter().flatten() {
            tx.conn().execute("INSERT INTO agent_grants(agent_id,capability,room_id,granted_by_id,revoked_at,created_at,updated_at) VALUES(?,?,?,127326141,?,?,?)",rusqlite::params![AGENT,cap.as_str(),config["grant_room"].as_i64(),if config["revoked"]==true {Some(tx.now())} else {None},tx.now(),tx.now()])?;
        }
        tx.conn().execute("UPDATE agents SET owner_id=127326141 WHERE id=?",[AGENT])?;
        tx.conn().execute("UPDATE sqlite_sequence SET seq=1900700000 WHERE name='messages'",[])?;
        for (creator,source,client) in [(127326141,"Source α & β","read-source"),(394959859,"Agent","read-agent")] {
            Message::create(tx,NewMessage{room_id:486777696,creator_id:creator,markdown_source:Some(source.into()),client_message_id:Some(client.into()),..Default::default()})?;
        }
        tx.conn().execute("INSERT INTO channel_threads(id,name,room_id,creator_id,work_owner_id,work_status,locked_at,last_activity_at,created_at,updated_at) VALUES(1900700020,'Owned α & β',486777696,394959859,?,'in_progress',?,?,?,?)",rusqlite::params![if config["other_owner"]==true {127326141} else {394959859},if config["locked"]==true {Some(tx.now())} else {None},tx.now(),tx.now(),tx.now()])?;
        campfire_db::ThreadTag::create(tx,1900700020,"api")?;
        Message::create(tx,NewMessage{room_id:486777696,creator_id:394959859,thread_id:Some(1900700020),markdown_source:Some("Thread".into()),client_message_id:Some("read-thread".into()),..Default::default()})?;
        if config["board"]==true {tx.conn().execute("UPDATE rooms SET type='Rooms::Board' WHERE id=486777696",[])?;}
        if config["remove_member"]==true {tx.conn().execute("DELETE FROM memberships WHERE room_id=486777696 AND user_id=394959859",[])?;}
        if config["pin"]==true {
            campfire_db::MessagePin::pin(tx,&Message::find(tx.conn(),1900700001)?,394959859)?.unwrap();
        }
        if config["cap"]==true {
            for i in 0..50 {
                let message=if i==0 {Message::find(tx.conn(),1900700001)?} else {Message::create(tx,NewMessage{room_id:486777696,creator_id:127326141,markdown_source:Some(format!("Cap {i}")),client_message_id:Some(format!("cap-{i}")),..Default::default()})?};
                campfire_db::MessagePin::create(tx,&message,486777696,394959859)?;
            }
        }
        Ok(())
    }).await.unwrap();
    let request = || {
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
        req
    };
    for _ in 0..case["setup"]["repeat"].as_u64().unwrap_or(0) {
        app.anonymous().send(request()).await;
    }
    let reply = app.anonymous().send(request()).await;
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
        "{name}"
    );
    for (key, expected) in case["response_headers"].as_object().unwrap() {
        assert_eq!(reply.header(key), expected.as_str(), "{name}: {key}");
    }
    eprintln!("WS11-api read wire case {name}: 1 passed; 0 failed");
}
async fn group(prefixes: &[&str]) {
    let vectors: Value =
        serde_json::from_str(include_str!("../../../../vectors/agent_reads_http.json")).unwrap();
    for case in vectors["cases"].as_array().unwrap().iter().filter(|case| {
        prefixes
            .iter()
            .any(|prefix| case["name"].as_str().unwrap().starts_with(prefix))
    }) {
        check(case).await;
    }
}
#[tokio::test]
async fn agent_reads_rooms_bytes() {
    group(&["mcp_list_rooms", "mcp_rooms_"]).await;
}
#[tokio::test]
async fn agent_reads_history_bytes() {
    group(&["mcp_history_"]).await;
}
#[tokio::test]
async fn agent_reads_work_bytes() {
    group(&["work_", "mcp_list_work", "mcp_work_"]).await;
}
#[tokio::test]
async fn agent_reads_board_bytes() {
    group(&["posts_", "mcp_posts_"]).await;
}
