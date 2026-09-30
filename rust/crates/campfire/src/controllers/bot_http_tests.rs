//! Actual pinned Rails bot API responses, persisted state and create-only reply-token scope.
use super::agent_http_tests::{AGENT, SECRET, setup};
use super::presenters::test_support::{BENDER_KEY, Req};
use campfire_db::{Message, NewMessage};
use campfire_kit::Method;
use serde_json::{Value, json};
const BASE: i64 = 1900300001;
async fn check(case: &Value) {
    let app = setup().await;
    let config = case["setup"].clone();
    app.db().write(move|tx|{
        tx.conn().execute("UPDATE agents SET daily_message_cap=? WHERE id=?",rusqlite::params![config["cap"].as_i64(),AGENT])?;
        if let Some(grant)=config["grant"].as_str(){tx.conn().execute("INSERT INTO agent_grants(agent_id,capability,granted_by_id,created_at,updated_at) VALUES(?,?,127326141,?,?)",rusqlite::params![AGENT,grant,tx.now(),tx.now()])?;}
        if config["revoked"].as_bool()==Some(true){tx.conn().execute("UPDATE agent_credentials SET revoked_at=? WHERE agent_id=?",rusqlite::params![tx.now(),AGENT])?;}
        if config["expired"].as_bool()==Some(true){tx.conn().execute("UPDATE agent_credentials SET expires_at=? WHERE agent_id=?",rusqlite::params![tx.now(),AGENT])?;}
        tx.conn().execute("UPDATE sqlite_sequence SET seq=? WHERE name='messages'",[BASE-1])?;
        let mut message=Message::create(tx,NewMessage{room_id:486777696,creator_id:394959859,client_message_id:Some("bot-contract-starter".into()),markdown_source:Some("Starter".into()),..Default::default()})?;
        assert_eq!(message.id,BASE);
        tx.conn().execute("INSERT INTO boosts(id,message_id,booster_id,content,created_at,updated_at) VALUES(1900300010,?,394959859,'👍',?,?),(1900300011,?,127326141,'👏',?,?)",rusqlite::params![BASE,tx.now(),tx.now(),BASE,tx.now(),tx.now()])?;
        tx.conn().execute("UPDATE sqlite_sequence SET seq=1900300011 WHERE name='boosts'",[])?;
        if config["drive"].as_bool()==Some(true){tx.conn().execute("INSERT INTO drive_attachments(message_id,file_id,created_at) VALUES(?,'1AbcDefGhIjKlMnOpQrSt',?)",rusqlite::params![BASE,tx.now()])?;}
        if config["other_creator"].as_bool()==Some(true){tx.conn().execute("UPDATE messages SET creator_id=127326141 WHERE id=?",[BASE])?;}
        if config["system"].as_bool()==Some(true){tx.conn().execute("UPDATE messages SET system_note=1 WHERE id=?",[BASE])?;}
        if config["index"].as_bool()==Some(true){
            message=Message::create(tx,NewMessage{room_id:486777696,creator_id:394959859,client_message_id:Some("bot-contract-next".into()),markdown_source:Some("**Next**".into()),..Default::default()})?;
            assert_eq!(message.id,BASE+1);
            tx.conn().execute("INSERT INTO channel_threads(id,name,room_id,creator_id,parent_message_id,last_activity_at,created_at,updated_at) VALUES(1900300008,'Excluded',486777696,394959859,?,?,?,?)",rusqlite::params![if config["summary"].as_bool()==Some(true){Some(BASE+1)}else{None},tx.now(),tx.now(),tx.now()])?;
            Message::create(tx,NewMessage{room_id:486777696,creator_id:394959859,thread_id:Some(1900300008),client_message_id:Some("bot-contract-thread".into()),markdown_source:Some("Thread only".into()),..Default::default()})?;
            if config["details"].as_bool()==Some(true){
                tx.conn().execute("UPDATE messages SET reply_to_message_id=?,reply_notify_author=1,forwarded_from_message_id=?,forwarded_at=?,forward_note='Forward note',streaming=1 WHERE id=?",rusqlite::params![BASE,BASE,tx.now(),BASE+1])?;
                tx.conn().execute("INSERT INTO drive_attachments(message_id,file_id,created_at) VALUES(?,'1AbcDefGhIjKlMnOpQrSt',?)",rusqlite::params![BASE+1,tx.now()])?;
            }
        }
        if config["board"].as_bool()==Some(true){tx.conn().execute("UPDATE rooms SET type='Rooms::Board' WHERE id=486777696",[])?;}
        tx.conn().execute("UPDATE users SET icon_name=? WHERE id=394959859",[config["icon"].as_str()])?;
        Ok(())
    }).await.unwrap();
    let now = app.booted.app.clock.now();
    let key = if case["setup"]["reply"].as_bool() == Some(true) {
        rails_compat::verifiers::bot_reply::token_for(
            &app.booted.app.secrets,
            394959859,
            486777696,
            now,
        )
    } else if case["setup"]["agent_token"].as_bool() == Some(true) {
        "credential-route".into()
    } else {
        BENDER_KEY.to_owned()
    };
    let path = case["path"].as_str().unwrap().replace("{key}", &key);
    let mut req = Req::new(
        Method::from_bytes(case["method"].as_str().unwrap().to_uppercase().as_bytes()).unwrap(),
        &path,
    )
    .header("accept", "application/json")
    .header("content-type", "text/plain");
    if let Some(body) = case["body"].as_str() {
        req = req.body(body);
    }
    if case["setup"]["agent_token"].as_bool() == Some(true) {
        req = req.header("authorization", &["Bearer", SECRET].join(" "));
    }
    let counts = |conn: &campfire_db::Connection| -> campfire_db::Result<(i64, i64)> {
        Ok((
            conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0))?,
            conn.query_row("SELECT COUNT(*) FROM boosts", [], |r| r.get(0))?,
        ))
    };
    let before = app.db().read(counts).await.unwrap();
    let reply = app.anonymous().send(req).await;
    let name = case["name"].as_str().unwrap();
    assert_eq!(
        reply.status.as_u16(),
        case["status"].as_u64().unwrap() as u16,
        "{name}: {}",
        reply.text()
    );
    assert_eq!(
        if reply.body.is_empty() {
            Value::Null
        } else {
            reply.json()
        },
        case["response"],
        "{name}"
    );
    for (key, value) in case["response_headers"].as_object().unwrap() {
        assert_eq!(reply.header(key), value.as_str(), "{name}: {key}");
    }
    let after = app.db().read(counts).await.unwrap();
    assert_eq!(
        json!({"messages":after.0-before.0,"boosts":after.1-before.1}),
        case["delta"],
        "{name}: row changes"
    );
    // Read the actual rich text implementation from the app; state has no request-specific fields.
    let rich_text = app.booted.app.db.env().rich_text.clone();
    let config = case["setup"].clone();
    let (state,created)=app.db().read(move|conn|{
        let snapshot=|m:&Message|->campfire_db::Result<Value>{Ok(json!({"markdown_source":m.markdown_source,"plain_text":m.plain_text_body(conn,&*rich_text)?,"drive_ids":m.drive_file_ids(conn)?}))};
        let state=Message::find_by_id(conn,BASE)?.as_ref().map(snapshot).transpose()?.unwrap_or(Value::Null);
        let created=if config["index"].as_bool()==Some(true){Value::Null}else{
            let id:Option<i64>=conn.query_row("SELECT MAX(id) FROM messages WHERE id>?",[BASE],|r|r.get(0))?;
            if let Some(m)=id.map(|id|Message::find_by_id(conn,id)).transpose()?.flatten(){json!({"plain_text":m.plain_text_body(conn,&*rich_text)?,"drive_ids":m.drive_file_ids(conn)?})}else{Value::Null}
        };
        Ok((state,created))
    }).await.unwrap();
    assert_eq!(state, case["state"], "{name}: original state");
    assert_eq!(created, case["created"], "{name}: created state");
    eprintln!("WS11-api Rails bot case passed: {name}");
}
async fn group(prefixes: &[&str]) {
    let vectors: Value =
        serde_json::from_str(include_str!("../../../../vectors/agent_bot_http.json")).unwrap();
    for case in vectors["cases"].as_array().unwrap().iter().filter(|c| {
        prefixes
            .iter()
            .any(|p| c["name"].as_str().unwrap().starts_with(p))
    }) {
        check(case).await;
    }
}
#[tokio::test]
async fn bot_http_message_contracts() {
    group(&["bot_create", "bot_update", "bot_destroy", "bot_index"]).await;
}
#[tokio::test]
async fn bot_http_boost_contracts() {
    group(&["bot_boost", "bot_agent"]).await;
}
#[tokio::test]
async fn bot_http_reply_create() {
    group(&["bot_reply_create"]).await;
}
#[tokio::test]
async fn bot_http_reply_read() {
    group(&["bot_reply_read"]).await;
}
#[tokio::test]
async fn bot_http_reply_update() {
    group(&["bot_reply_update"]).await;
}
#[tokio::test]
async fn bot_http_reply_destroy() {
    group(&["bot_reply_destroy"]).await;
}
#[tokio::test]
async fn bot_http_reply_boost_create() {
    group(&["bot_reply_boost_create"]).await;
}
#[tokio::test]
async fn bot_http_reply_boost_destroy() {
    group(&["bot_reply_boost_destroy"]).await;
}
#[tokio::test]
async fn bot_http_budget_overflow() {
    group(&["bot_create_budget"]).await;
}

#[tokio::test]
async fn bot_http_nonmember_is_hidden() {
    group(&["bot_create_nonmember"]).await;
}
