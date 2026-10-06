//! Pinned legacy-bot HTTP bytes and actual after-commit webhook/ledger jobs.
use super::agent_http_tests::{AGENT, setup};
use super::presenters::test_support::Req;
use campfire_db::{Message, NewMessage, User};
use campfire_kit::Method;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
const LEGACY: i64 = 1901100001;
const RECEIVER: i64 = 1901100002;
const BENDER: i64 = 394959859;
const ROOM: i64 = 486777696;
const BASE: i64 = 1901100100;
async fn fixture(case: &Value) -> super::presenters::test_support::TestApp {
    let app = setup().await.without_job_runner().await;
    let config = case["setup"].clone();
    app.db().write(move |tx| {
        tx.conn().execute("UPDATE sqlite_sequence SET seq=1901100000 WHERE name='users'", [])?;
        let legacy=User::create_bot(tx,"Legacy Poster",Some("https://example.test/legacy-poster"))?;
        let receiver=User::create_bot(tx,"Legacy Receiver",if config["no_webhook"]==true {None}else{Some("https://example.test/legacy-receiver")})?;
        assert_eq!((legacy.id,receiver.id),(LEGACY,RECEIVER));
        for (id,token) in [(LEGACY,"LegacyToken1"),(RECEIVER,"ReceiverToken1")] {
            tx.conn().execute("UPDATE users SET bot_token_digest=? WHERE id=?",rusqlite::params![format!("{:x}",Sha256::digest(token)),id])?;
        }
        tx.conn().execute("UPDATE agents SET owner_id=127326141,daily_message_cap=NULL WHERE id=?",[AGENT])?;
        tx.conn().execute("UPDATE rooms SET type=? WHERE id=?",rusqlite::params![if config["direct"]==true {"Rooms::Direct"} else{"Rooms::Closed"},ROOM])?;
        tx.conn().execute("UPDATE users SET status=? WHERE id=?",rusqlite::params![if config["inactive"]==true {1}else{0},RECEIVER])?;
        campfire_db::Room::find(tx.conn(),ROOM)?.grant_to(tx,&[LEGACY,RECEIVER])?;
        if config["remove_receiver"]==true {tx.conn().execute("DELETE FROM memberships WHERE room_id=? AND user_id=?",[ROOM,RECEIVER])?;}
        if config["remove_sender"]==true {tx.conn().execute("DELETE FROM memberships WHERE room_id=? AND user_id=?",[ROOM,LEGACY])?;}
        if config["revoke_receiver"]==true {campfire_db::AgentGrant::create(tx,campfire_db::NewGrant{agent_id:AGENT,capability:"react".into(),granted_by_id:127326141,..Default::default()})?;}
        tx.conn().execute("UPDATE sqlite_sequence SET seq=? WHERE name='messages'",[BASE-1])?;
        let mut starter=Message::create(tx,NewMessage{room_id:ROOM,creator_id:if config["other_creator"]==true || config["agent_sender"]==true {BENDER}else{LEGACY},markdown_source:Some("Starter".into()),client_message_id:Some("legacy-starter".into()),..Default::default()})?;
        assert_eq!(starter.id,BASE);
        if config["board"]==true {tx.conn().execute("UPDATE rooms SET type='Rooms::Board' WHERE id=?",[ROOM])?;}
        if let Some(id)=config["attachment_old"].as_i64(){starter.replace_attachment(tx,Some(id))?;}
        if config["system"]==true {tx.conn().execute("UPDATE messages SET system_note=1 WHERE id=?",[BASE])?;}
        tx.conn().execute("DELETE FROM agent_events WHERE agent_id=?",[AGENT])?;
        if let Some(hop)=config["hop"].as_i64() {
            tx.conn().execute("INSERT INTO agent_events(id,agent_id,event_type,outcome,actor_id,room_id,message_id,chain_id,metadata,hop,created_at) VALUES(1901100200,?,'mention','delivered',127326141,?,?,'legacy-http-chain',?,?,?)",rusqlite::params![AGENT,ROOM,BASE,json!({"hop":hop}),hop,tx.now()])?;
        }
        if let Some(cap)=config["budget_cap"].as_i64(){tx.conn().execute("UPDATE agents SET daily_message_cap=? WHERE id=?",[cap,AGENT])?;}
        tx.conn().execute("UPDATE sqlite_sequence SET seq=1901100200 WHERE name='agent_events'", [])?;
        Ok(())
    }).await.unwrap();
    // Clear fixture jobs only after their real after-commit handlers have completed.
    app.db()
        .write(|tx| {
            tx.conn().execute("DELETE FROM background_jobs", [])?;
            Ok(())
        })
        .await
        .unwrap();
    app
}
async fn check(case: &Value) {
    let app = fixture(case).await;
    let sender = if case["setup"]["agent_sender"] == true {
        BENDER
    } else {
        LEGACY
    };
    let key = if case["setup"]["reply"] == true {
        rails_compat::verifiers::bot_reply::token_for(
            &app.booted.app.secrets,
            sender,
            ROOM,
            app.booted.app.clock.now(),
        )
    } else {
        format!(
            "{sender}-{}",
            if sender == BENDER {
                "BenderToken1"
            } else {
                "LegacyToken1"
            }
        )
    };
    let path = case["path"].as_str().unwrap().replace("{key}", &key);
    let request = || {
        let req = Req::new(
            Method::from_bytes(case["method"].as_str().unwrap().to_uppercase().as_bytes()).unwrap(),
            &path,
        )
        .header("accept", "application/json")
        .header(
            "content-type",
            if case["setup"]["attachment_kind"].is_string()
                && case["setup"]["attachment_kind"] != "body"
            {
                "application/json"
            } else {
                "text/plain"
            },
        );
        if let Some(body) = case["body"].as_str() {
            req.body(body)
        } else {
            req
        }
    };
    let count = |conn: &campfire_db::Connection| {
        conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get::<_, i64>(0))
            .map_err(Into::into)
    };
    let before = app.db().read(count).await.unwrap();
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
        "{name}: raw response bytes"
    );
    for (header, expected) in case["response_headers"].as_object().unwrap() {
        assert_eq!(reply.header(header), expected.as_str(), "{name}: {header}");
    }
    let rich_text = app.db().env().rich_text.clone();
    let (after,state,events,jobs)=app.db().read(move|conn| {
        let mut state=vec![];
        let ids=conn.prepare("SELECT id FROM messages WHERE id>=? ORDER BY id")?.query_map([BASE],|r|r.get::<_,i64>(0))?.collect::<Result<Vec<_>,_>>()?;
        for id in ids {let m=Message::find(conn,id)?;state.push(json!({"id":m.id,"creator_id":m.creator_id,"markdown_source":m.markdown_source,"plain_text":m.plain_text_body(conn,&*rich_text)?,"attachment_blob_id":campfire_storage::Blob::attached(conn,"Message",m.id,"attachment").map_err(|e|campfire_db::Error::Other(e.to_string()))?.map(|b|b.id)}));}
        let events=campfire_db::models::agent_delivery::AgentEvent::for_agent(conn,AGENT)?.into_iter().filter(|e|e.id>1901100200).map(|e|json!({"id":e.id,"agent_id":e.agent_id,"room_id":e.room_id,"message_id":e.message_id,"actor_id":e.actor_id,"event_type":e.event_type,"outcome":e.outcome,"hop":e.hop(),"detail":e.detail})).collect::<Vec<_>>();
        let jobs=conn.prepare("SELECT job_class,arguments FROM background_jobs WHERE job_class IN ('Bot::WebhookJob','Agent::DeliveryJob','ActiveStorage::AnalyzeJob','ActiveStorage::PurgeJob') ORDER BY id")?.query_map([],|r| {
            let class:String=r.get(0)?;let args:Value=r.get(1)?;
            Ok(if class=="Bot::WebhookJob" {json!({"class":class,"bot_id":args["bot_id"],"message_id":args["message_id"]})}else if class=="Agent::DeliveryJob" {json!({"class":class,"event_id":args["event_id"]})}else{json!({"class":class,"blob_id":args["blob_id"]})})
        })?.collect::<Result<Vec<_>,_>>()?;
        Ok((count(conn)?,json!(state),json!(events),json!(jobs)))
    }).await.unwrap();
    assert_eq!(
        after - before,
        case["delta"].as_i64().unwrap(),
        "{name}: row delta"
    );
    assert_eq!(state, case["state"], "{name}: persisted messages");
    assert_eq!(events, case["events"], "{name}: ledger");
    assert_eq!(jobs, case["jobs"], "{name}: durable jobs");
    eprintln!("WS11-api legacy bot case {name}: 1 passed; 0 failed");
}
async fn group(fanout: bool) {
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agent_legacy_bot_http.json"
    ))
    .unwrap();
    for case in vectors["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["name"].as_str().unwrap().starts_with("fanout_") == fanout)
    {
        check(case).await;
    }
}
#[tokio::test]
async fn agent_legacy_bot_http_bytes() {
    group(false).await;
}
#[tokio::test]
async fn agent_legacy_bot_fanout_http() {
    group(true).await;
}

async fn concurrent(replay: bool) {
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agent_legacy_bot_http.json"
    ))
    .unwrap();
    let case = |name: &str| {
        vectors["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == name)
            .unwrap()
    };
    let first = case(if replay {
        "race_replay"
    } else {
        "race_budget_first"
    });
    let app = fixture(first).await;
    let path = first["path"]
        .as_str()
        .unwrap()
        .replace("{key}", "394959859-BenderToken1");
    let request = || {
        Req::new(Method::POST, &path)
            .header("accept", "application/json")
            .header("content-type", "text/plain")
            .body("Race")
    };
    let mut left = app.anonymous();
    let mut right = app.anonymous();
    let (left, right) = tokio::join!(left.send(request()), right.send(request()));
    let mut responses = [left, right];
    responses.sort_by_key(|reply| reply.status.as_u16());
    for (i, reply) in responses.iter().enumerate() {
        let expected = if replay || i == 0 {
            first
        } else {
            case("race_budget_overflow")
        };
        assert_eq!(
            reply.status.as_u16(),
            expected["status"].as_u64().unwrap() as u16,
            "concurrent posting: {}",
            reply.text()
        );
        assert_eq!(
            reply.text(),
            expected["response_body"].as_str().unwrap(),
            "concurrent posting: raw Rails bytes"
        );
        for (header, value) in expected["response_headers"].as_object().unwrap() {
            assert_eq!(
                reply.header(header),
                value.as_str(),
                "concurrent posting: {header}"
            );
        }
    }
    let state = app
        .db()
        .read(|conn| {
            let messages: i64 =
                conn.query_row("SELECT COUNT(*) FROM messages WHERE id>?", [BASE], |r| {
                    r.get(0)
                })?;
            let posted: i64 = conn.query_row(
                "SELECT COUNT(*) FROM agent_events WHERE agent_id=? AND event_type='posted'",
                [AGENT],
                |r| r.get(0),
            )?;
            let pushes: i64 = conn.query_row(
                "SELECT COUNT(*) FROM background_jobs WHERE job_class='Room::PushMessageJob'",
                [],
                |r| r.get(0),
            )?;
            let notices: i64 = conn.query_row(
                "SELECT COUNT(*) FROM agent_budget_notices WHERE agent_id=? AND cap='messages'",
                [AGENT],
                |r| r.get(0),
            )?;
            Ok((messages, posted, pushes, notices))
        })
        .await
        .unwrap();
    assert_eq!(
        state,
        (1, 1, 1, if replay { 0 } else { 1 }),
        "concurrent posting must save/deliver once and atomically enforce the cap"
    );
}
#[tokio::test]
async fn agent_concurrent_bot_budget() {
    concurrent(false).await;
}
#[tokio::test]
async fn agent_concurrent_bot_replay() {
    concurrent(true).await;
}
