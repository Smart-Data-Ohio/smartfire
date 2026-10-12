use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use campfire_db::models::sticker::{Sticker, StickerValues};
use campfire_db::{Message, Room};
use serde_json::{Value, json};

use super::agent_http_tests::{AGENT, SECRET, setup as agent_setup};
use crate::controllers::presenters::test_support::{
    ALL_TALK, BENDER, BENDER_KEY, Browser, DAVID, HQ, Reply, Req, TestApp, seed_clock,
};

const PNG: &[u8] = include_bytes!("../../../../test-support/attachments/stickers/static.png");
const GIF: &[u8] = include_bytes!("../../../../test-support/attachments/stickers/animated.gif");
const ADMIN_STICKERS: &str = "/api/v1/admin/stickers";

async fn spa_app() -> TestApp {
    TestApp::boot_seed_with_env("default", seed_clock(), &[("SPA_ENABLED", "1")])
        .await
        .expect("restored default seed")
        .without_job_runner()
        .await
}

fn json_req(method: Method, path: &str, body: &Value) -> Req {
    Req::new(method, path)
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .body(body.to_string())
}

async fn http_sticker(b: &mut Browser<'_>, name: &str, bytes: &[u8]) -> i64 {
    let reply = b
        .write(Req::new(Method::POST, ADMIN_STICKERS).multipart(
            &[
                ("name", name),
                ("description", "Greeting"),
                ("tags", "wave"),
            ],
            ("image", "sticker.bin", "application/octet-stream", bytes),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let catalog: api::StickerCatalog = serde_json::from_slice(&reply.body).unwrap();
    catalog
        .stickers
        .into_iter()
        .find(|s| s.name == name)
        .unwrap()
        .id
}

async fn db_sticker(app: &TestApp, name: &str) -> i64 {
    let staged = app
        .booted
        .app
        .storage
        .stage_bytes(
            PNG,
            campfire_storage::Filename::new("wave.png"),
            Some("image/png"),
        )
        .unwrap();
    let blob = app
        .db()
        .write(move |tx| crate::controllers::messages::save_staged(tx, staged))
        .await
        .unwrap();
    let blob_id = blob.id;
    let name = name.to_string();
    app.db()
        .write(move |tx| {
            let sticker = Sticker::create(
                tx,
                DAVID,
                StickerValues {
                    name,
                    description: "Greeting".into(),
                    tags: "wave".into(),
                },
            )?;
            let blob = campfire_storage::Blob::find(tx.conn(), blob_id)
                .map_err(|e| campfire_db::Error::Other(e.to_string()))?
                .ok_or(campfire_db::Error::RecordNotFound("Blob"))?;
            crate::controllers::presenters::attachments::attach_existing(
                tx,
                crate::controllers::presenters::attachments::Record::sticker(sticker.id),
                "image",
                blob,
            )?;
            Ok(sticker.id)
        })
        .await
        .unwrap()
}

async fn grant_agent(app: &TestApp, agent: i64, capabilities: &[&str]) {
    let capabilities: Vec<String> = capabilities.iter().map(|c| c.to_string()).collect();
    app.db()
        .write(move |tx| {
            for capability in &capabilities {
                tx.conn().execute(
                    "INSERT INTO agent_grants(agent_id,capability,granted_by_id,created_at,updated_at) VALUES(?,?,127326141,?,?)",
                    rusqlite::params![agent, capability, tx.now(), tx.now()],
                )?;
            }
            Ok(())
        })
        .await
        .unwrap();
}

async fn revoked_post_grant(app: &TestApp) {
    app.db().write(|tx| {
        tx.conn().execute(
            "INSERT INTO agent_grants(agent_id,capability,granted_by_id,revoked_at,created_at,updated_at) VALUES(?,'post_messages',127326141,?,?,?)",
            rusqlite::params![AGENT, tx.now(), tx.now(), tx.now()],
        )?;
        Ok(())
    }).await.unwrap();
}

async fn bender_agent_id(app: &TestApp) -> i64 {
    app.db()
        .read(|conn| {
            Ok(
                conn.query_row("SELECT id FROM agents WHERE user_id=?", [BENDER], |r| {
                    r.get(0)
                })?,
            )
        })
        .await
        .unwrap()
}

fn path_of(url: &str) -> String {
    match url.find("://") {
        Some(at) => match url[at + 3..].find('/') {
            Some(slash) => url[at + 3 + slash..].to_string(),
            None => "/".into(),
        },
        None => url.to_string(),
    }
}

async fn fetch_url(app: &TestApp, url: &str) -> Reply {
    let mut path = path_of(url);
    for _ in 0..3 {
        let reply = app.anonymous().send(Req::new(Method::GET, &path)).await;
        if reply.status.is_redirection() {
            path = path_of(reply.location().expect("redirect location"));
            continue;
        }
        return reply;
    }
    panic!("too many redirects fetching {url}");
}

async fn bot_page(app: &TestApp, bot_path: &str) -> Value {
    let reply = app
        .anonymous()
        .send(Req::new(Method::GET, bot_path).header("accept", "application/json"))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    reply.json()
}

async fn bot_post(app: &TestApp, path: &str, content_type: &str, body: String) -> Reply {
    app.anonymous()
        .send(
            Req::new(Method::POST, path)
                .header("accept", "application/json")
                .header("content-type", content_type)
                .body(body),
        )
        .await
}

async fn sticker_rows(app: &TestApp) -> Vec<(Option<i64>, Option<String>)> {
    app.db()
        .read(|conn| {
            let mut stmt =
                conn.prepare("SELECT id FROM messages WHERE sticker_id IS NOT NULL ORDER BY id")?;
            let ids = stmt
                .query_map([], |r| r.get::<_, i64>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            ids.into_iter()
                .map(|id| Message::find(conn, id).map(|m| (m.sticker_id, m.markdown_source)))
                .collect::<campfire_db::Result<Vec<_>>>()
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn bot_reads_include_the_sticker_object_and_deleted_placeholder() {
    let a = spa_app().await;
    let agent = bender_agent_id(&a).await;
    grant_agent(&a, agent, &["read_messages"]).await;
    a.db()
        .write(|tx| Room::find(tx.conn(), ALL_TALK)?.grant_to(tx, &[BENDER]))
        .await
        .unwrap();
    let mut david = a.sign_in(DAVID).await;
    let sticker_id = http_sticker(&mut david, "WaveBot", GIF).await;
    let messages = format!("/api/v1/rooms/{ALL_TALK}/messages");
    let mut ids = Vec::new();
    for (client, markdown) in [("bot-read-text", "Welcome!"), ("bot-read-only", "")] {
        let reply = david
            .write(json_req(
                Method::POST,
                &messages,
                &json!({"clientMessageId":client,"markdownSource":markdown,"stickerId":sticker_id}),
            ))
            .await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
        let dto: api::MessageDTO = serde_json::from_slice(&reply.body).unwrap();
        ids.push(dto.id);
    }
    let plain = david
        .write(json_req(
            Method::POST,
            &messages,
            &json!({"clientMessageId":"bot-read-plain","markdownSource":"Plain"}),
        ))
        .await;
    let plain: api::MessageDTO = serde_json::from_slice(&plain.body).unwrap();

    let bot_path = format!("/rooms/{ALL_TALK}/{BENDER_KEY}/messages");
    let page = bot_page(&a, &bot_path).await;
    let page = page.as_array().unwrap();
    let by_id = |id: i64| {
        page.iter()
            .find(|m| m["id"] == id)
            .unwrap_or_else(|| panic!("message {id} in bot page"))
            .clone()
    };
    for id in &ids {
        let sticker = &by_id(*id)["sticker"];
        assert_eq!(sticker["kind"], "available", "message {id}: {sticker}");
        assert_eq!(sticker["id"], sticker_id);
        assert_eq!(sticker["name"], "WaveBot");
        assert_eq!(sticker["animated"], true);
        let image_url = sticker["imageUrl"].as_str().expect("image_url");
        assert!(
            image_url.contains("/rails/active_storage/blobs/redirect/"),
            "{image_url}"
        );
        assert!(
            sticker["stillUrl"].as_str().is_some_and(|u| !u.is_empty()),
            "{sticker}"
        );
        let fetched = fetch_url(&a, image_url).await;
        assert_eq!(
            fetched.status,
            StatusCode::OK,
            "bot fetches the sticker image like an attachment"
        );
        assert_eq!(fetched.body, GIF);
    }
    assert_eq!(by_id(ids[0])["body"]["plain_text"], "Welcome!");
    assert!(
        by_id(plain.id).get("sticker").is_none(),
        "messages without stickers keep their shape"
    );

    let deleted = david
        .write(json_req(
            Method::DELETE,
            &format!("{ADMIN_STICKERS}/{sticker_id}"),
            &Value::Null,
        ))
        .await;
    assert!(deleted.status.is_success(), "{}", deleted.text());
    let page = bot_page(&a, &bot_path).await;
    let page = page.as_array().unwrap();
    for id in &ids {
        let entry = page.iter().find(|m| m["id"] == *id).unwrap();
        assert_eq!(
            entry["sticker"],
            json!({"kind":"deleted","name":"WaveBot"}),
            "message {id}"
        );
    }
}

async fn mcp_call(app: &TestApp, name: &str, arguments: Value) -> Reply {
    let body = json!({"jsonrpc":"2.0","id":77,"method":"tools/call","params":{"name":name,"arguments":arguments}});
    app.anonymous()
        .send(
            Req::new(Method::POST, "/agents/mcp")
                .header("authorization", &["Bearer", SECRET].join(" "))
                .header("accept", "application/json")
                .header("content-type", "application/json")
                .body(body.to_string()),
        )
        .await
}

fn tool_payload(reply: &Reply) -> Value {
    let body = reply.json();
    let result = &body["result"];
    if !result["structuredContent"].is_null() {
        return result["structuredContent"].clone();
    }
    result["content"]
        .as_array()
        .and_then(|parts| parts.first())
        .and_then(|part| part["text"].as_str())
        .and_then(|text| serde_json::from_str(text).ok())
        .unwrap_or(Value::Null)
}

#[tokio::test]
async fn mcp_post_message_advertises_sticker_input() {
    let a = agent_setup().await.without_job_runner().await;
    let list = a
        .anonymous()
        .send(
            Req::new(Method::POST, "/agents/mcp")
                .header("authorization", &["Bearer", SECRET].join(" "))
                .header("accept", "application/json")
                .header("content-type", "application/json")
                .body(json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string()),
        )
        .await;
    assert_eq!(list.status, StatusCode::OK, "{}", list.text());
    let tools = list.json();
    let post = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "post_message")
        .expect("post_message tool")
        .clone();
    assert_eq!(
        post["inputSchema"]["properties"]["sticker_id"]["type"], "integer",
        "{post}"
    );
    assert!(
        post["description"].as_str().unwrap().contains("sticker"),
        "{post}"
    );
}

#[tokio::test]
async fn mcp_post_message_accepts_a_sticker_and_reads_it_back() {
    let a = agent_setup().await.without_job_runner().await;
    let sticker_id = db_sticker(&a, "WaveMcp").await;
    revoked_post_grant(&a).await;
    let denied = mcp_call(
        &a,
        "post_message",
        json!({"room_id":ALL_TALK,"sticker_id":sticker_id}),
    )
    .await;
    assert_eq!(
        denied.json()["result"]["structuredContent"]["status"],
        "forbidden"
    );
    grant_agent(&a, AGENT, &["post_messages", "read_messages"]).await;

    let before: i64 = a
        .db()
        .read(|c| Ok(c.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0))?))
        .await
        .unwrap();
    let unknown = mcp_call(
        &a,
        "post_message",
        json!({"room_id":ALL_TALK,"sticker_id":999999999}),
    )
    .await;
    assert_eq!(unknown.status, StatusCode::OK, "{}", unknown.text());
    let body = unknown.json();
    assert_eq!(body["result"]["isError"], true, "{body}");
    assert_eq!(
        body["result"]["structuredContent"]["status"], "unprocessable_entity",
        "{body}"
    );
    assert!(
        body["result"]["structuredContent"]["error"]
            .as_str()
            .unwrap()
            .contains("Sticker"),
        "{body}"
    );
    let after: i64 = a
        .db()
        .read(|c| Ok(c.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0))?))
        .await
        .unwrap();
    assert_eq!(before, after, "no message is posted for an unknown sticker");

    let mut posted = Vec::new();
    for arguments in [
        json!({"room_id":ALL_TALK,"sticker_id":sticker_id}),
        json!({"room_id":ALL_TALK,"sticker_id":sticker_id,"markdown_source":"Ahoy"}),
    ] {
        let reply = mcp_call(&a, "post_message", arguments).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        let payload = tool_payload(&reply);
        assert_eq!(payload["sticker"]["kind"], "available", "{payload}");
        assert_eq!(payload["sticker"]["id"], sticker_id);
        assert_eq!(payload["sticker"]["name"], "WaveMcp");
        assert!(
            payload["sticker"]["imageUrl"]
                .as_str()
                .is_some_and(|u| u.contains("/rails/active_storage/blobs/redirect/")),
            "{payload}"
        );
        posted.push(payload["id"].as_i64().expect("posted message id"));
    }
    let rows = a
        .db()
        .read(move |conn| {
            posted
                .iter()
                .map(|id| Message::find(conn, *id).map(|m| (m.sticker_id, m.sticker_name)))
                .collect::<campfire_db::Result<Vec<_>>>()
        })
        .await
        .unwrap();
    assert_eq!(
        rows,
        vec![
            (Some(sticker_id), Some("WaveMcp".into())),
            (Some(sticker_id), Some("WaveMcp".into()))
        ]
    );

    let read = mcp_call(&a, "read_messages", json!({"room_id":ALL_TALK,"limit":5})).await;
    assert_eq!(read.status, StatusCode::OK, "{}", read.text());
    let payload = tool_payload(&read);
    let messages = payload["messages"]
        .as_array()
        .cloned()
        .unwrap_or_else(|| payload.as_array().cloned().unwrap_or_default());
    assert!(
        messages
            .iter()
            .any(|m| m["sticker"]["id"] == sticker_id && m["sticker"]["kind"] == "available"),
        "MCP reads carry the sticker object: {payload}"
    );
    a.db()
        .write(move |tx| Sticker::find(tx.conn(), sticker_id)?.destroy(tx))
        .await
        .unwrap();
    let deleted = mcp_call(
        &a,
        "post_message",
        json!({"room_id":ALL_TALK,"sticker_id":sticker_id,"body":"Nope"}),
    )
    .await;
    assert_eq!(
        deleted.json()["result"]["structuredContent"]["status"],
        "unprocessable_entity"
    );
    let deleted_count: i64 = a
        .db()
        .read(|c| Ok(c.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0))?))
        .await
        .unwrap();
    assert_eq!(deleted_count, before + 2);
}

#[tokio::test]
async fn bot_sending_accepts_a_sticker_id_as_json() {
    let a = agent_setup().await.without_job_runner().await;
    let sticker_id = db_sticker(&a, "WaveSend").await;
    revoked_post_grant(&a).await;
    let path = format!("/rooms/{ALL_TALK}/{BENDER_KEY}/messages");
    let denied = bot_post(
        &a,
        &path,
        "application/json",
        json!({"sticker_id":sticker_id}).to_string(),
    )
    .await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);
    grant_agent(&a, AGENT, &["post_messages", "read_messages"]).await;

    for body in [
        json!({"sticker_id":sticker_id}),
        json!({"sticker_id":sticker_id,"body":"Ahoy"}),
    ] {
        let reply = bot_post(&a, &path, "application/json", body.to_string()).await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    }
    let rows = sticker_rows(&a).await;
    assert_eq!(
        rows.len(),
        2,
        "both JSON posts became sticker messages: {rows:?}"
    );
    assert!(rows.iter().all(|(id, _)| *id == Some(sticker_id)));

    let unknown = bot_post(
        &a,
        &path,
        "application/json",
        json!({"sticker_id":999999999}).to_string(),
    )
    .await;
    assert_eq!(
        unknown.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        unknown.text()
    );
    assert_eq!(
        sticker_rows(&a).await.len(),
        2,
        "an unknown sticker posts nothing"
    );

    let raw = bot_post(&a, &path, "text/plain", "plain bot text".into()).await;
    assert_eq!(raw.status, StatusCode::CREATED, "{}", raw.text());
    let read = a
        .anonymous()
        .send(Req::new(Method::GET, &path).header("accept", "application/json"))
        .await;
    assert_eq!(read.status, StatusCode::OK, "{}", read.text());
    let page = read.json();
    let page = page.as_array().unwrap();
    let senders: Vec<&Value> = page
        .iter()
        .filter(|m| m["sticker"]["id"] == sticker_id)
        .collect();
    assert_eq!(senders.len(), 2, "{page:?}");
    assert!(
        page.iter()
            .any(|m| m["body"]["plain_text"] == "plain bot text" && m.get("sticker").is_none())
    );
    assert!(
        page.iter()
            .any(|m| m["body"]["plain_text"] == "Ahoy" && m["sticker"]["name"] == "WaveSend")
    );
    let legacy_json = json!({"body":"legacy JSON text"}).to_string();
    let legacy = bot_post(&a, &path, "application/json", legacy_json.clone()).await;
    assert_eq!(legacy.status, StatusCode::CREATED);
    let messages = bot_page(&a, &path).await;
    assert!(
        messages
            .as_array()
            .unwrap()
            .iter()
            .any(|message| message["body"]["plain_text"] == legacy_json)
    );
    let forbidden_room = bot_post(
        &a,
        &format!("/rooms/{HQ}/{BENDER_KEY}/messages"),
        "application/json",
        json!({"sticker_id":sticker_id}).to_string(),
    )
    .await;
    assert_eq!(forbidden_room.status, StatusCode::NOT_FOUND);
    a.db()
        .write(move |tx| Sticker::find(tx.conn(), sticker_id)?.destroy(tx))
        .await
        .unwrap();
    let deleted = bot_post(
        &a,
        &path,
        "application/json",
        json!({"sticker_id":sticker_id,"body":"Nope"}).to_string(),
    )
    .await;
    assert_eq!(deleted.status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn incoming_webhooks_accept_a_sticker_id() {
    let a = spa_app().await;
    let mut david = a.sign_in(DAVID).await;
    let sticker_id = http_sticker(&mut david, "WaveHook", PNG).await;
    let hook = david
        .write(json_req(
            Method::POST,
            &format!("/api/v1/rooms/{HQ}/webhooks"),
            &json!({"name":"Deploys"}),
        ))
        .await;
    assert_eq!(hook.status, StatusCode::CREATED, "{}", hook.text());
    let hook = hook.json();
    let execute = format!(
        "/api/webhooks/{}/{}?wait=true",
        hook["webhook"]["id"],
        hook["token"].as_str().unwrap()
    );
    let only = a
        .anonymous()
        .send(json_req(
            Method::POST,
            &execute,
            &json!({"sticker_id":sticker_id}),
        ))
        .await;
    assert_eq!(only.status, StatusCode::OK, "{}", only.text());
    let dto: api::MessageDTO = serde_json::from_slice(&only.body).unwrap();
    assert!(
        matches!(&dto.sticker, Some(api::MessageSticker::Available { id, name, .. }) if *id == sticker_id && name == "WaveHook")
    );

    let captioned = a
        .anonymous()
        .send(json_req(
            Method::POST,
            &execute,
            &json!({"sticker_id":sticker_id,"content":"Shipped"}),
        ))
        .await;
    assert_eq!(captioned.status, StatusCode::OK, "{}", captioned.text());
    let dto: api::MessageDTO = serde_json::from_slice(&captioned.body).unwrap();
    assert!(dto.sticker.is_some());
    let row = a
        .db()
        .read(move |conn| Message::find(conn, dto.id))
        .await
        .unwrap();
    assert_eq!(row.sticker_id, Some(sticker_id));

    let unknown = a
        .anonymous()
        .send(json_req(
            Method::POST,
            &execute,
            &json!({"sticker_id":999999999,"content":"Nope"}),
        ))
        .await;
    assert_eq!(
        unknown.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        unknown.text()
    );
    let count: i64 = a
        .db()
        .read(|c| {
            Ok(c.query_row(
                "SELECT COUNT(*) FROM messages WHERE room_id=? AND sticker_id=999999999",
                [HQ],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(count, 0);
    let deleted = david
        .write(json_req(
            Method::DELETE,
            &format!("{ADMIN_STICKERS}/{sticker_id}"),
            &Value::Null,
        ))
        .await;
    assert_eq!(deleted.status, StatusCode::OK);
    let deleted = a
        .anonymous()
        .send(json_req(
            Method::POST,
            &execute,
            &json!({"sticker_id":sticker_id,"content":"Nope"}),
        ))
        .await;
    assert_eq!(deleted.status, StatusCode::UNPROCESSABLE_ENTITY);
    let total: i64 = a
        .db()
        .read(|c| {
            Ok(
                c.query_row("SELECT COUNT(*) FROM channel_webhook_messages", [], |r| {
                    r.get(0)
                })?,
            )
        })
        .await
        .unwrap();
    assert_eq!(total, 2);
}

#[tokio::test]
async fn outgoing_webhooks_deliver_fetchable_stickers() {
    outgoing_stickers(false).await;
}

#[tokio::test]
async fn agent_webhooks_deliver_fetchable_stickers() {
    outgoing_stickers(true).await;
}

async fn deliver_to_receiver(
    a: &TestApp,
    bot: i64,
    message: i64,
    net: &crate::net::Network,
    agent: bool,
) {
    if agent {
        let event = a
            .db()
            .write(move |tx| {
                campfire_db::models::agent_delivery::AgentEvent::create(
                    tx,
                    campfire_db::models::agent_delivery::NewEvent {
                        agent_id: AGENT,
                        room_id: Some(ALL_TALK),
                        message_id: Some(message),
                        event_type: "mention".into(),
                        outcome: Some("delivered".into()),
                        ..Default::default()
                    },
                )
            })
            .await
            .unwrap();
        let outcome =
            campfire_channels::jobs::agent_jobs::post_event(&a.booted.app, &event, net).await;
        match outcome {
            campfire_db::models::agent_delivery::AttemptOutcome::Delivered => {}
            campfire_db::models::agent_delivery::AttemptOutcome::Permanent(error)
            | campfire_db::models::agent_delivery::AttemptOutcome::Retry(error, _) => {
                panic!("{error}")
            }
        }
    } else {
        campfire_channels::jobs::integrations::deliver_webhook_with_network(
            &a.booted.app,
            crate::queue::WebhookJob {
                bot_id: bot,
                message_id: message,
            },
            net,
        )
        .await
        .unwrap();
    }
}

async fn outgoing_stickers(agent: bool) {
    use crate::integrations::test_support::{
        FakeResolver, FakeServer, MappingDialer, Route, network,
    };
    use std::{
        collections::HashSet,
        sync::{Arc, Mutex},
    };

    let a = spa_app().await;
    let mut david = a.sign_in(DAVID).await;
    let sticker_id = http_sticker(&mut david, "WaveOutgoing", GIF).await;
    let bot = a
        .db()
        .write(move |tx| {
            let bot = if agent {
                BENDER
            } else {
                campfire_db::User::create_bot(
                    tx,
                    "Sticker Receiver",
                    Some("http://bots.example:8080/hook"),
                )?
                .id
            };
            Room::find(tx.conn(), ALL_TALK)?.grant_to(tx, &[bot])?;
            tx.conn().execute(
                "UPDATE webhooks SET url='http://bots.example:8080/hook' WHERE user_id=?",
                [bot],
            )?;
            Ok(bot)
        })
        .await
        .unwrap();
    let server = FakeServer::start(vec![Route::new("POST", "*", "/hook", 204)]).await;
    let net = network(
        Arc::new(FakeResolver::new([("bots.example", vec!["93.184.216.34"])])),
        Arc::new(MappingDialer {
            public: HashSet::from(["93.184.216.34".parse().unwrap()]),
            to: server.addr,
            dialed: Mutex::new(vec![]),
        }),
    );
    let mut posted = Vec::new();
    for (client, markdown) in [
        ("outgoing-caption", "Ahoy"),
        ("outgoing-only", ""),
        ("outgoing-plain", "Plain"),
    ] {
        let mut body = json!({"clientMessageId":client,"markdownSource":markdown});
        if client != "outgoing-plain" {
            body["stickerId"] = sticker_id.into();
        }
        let reply = david
            .write(json_req(
                Method::POST,
                &format!("/api/v1/rooms/{ALL_TALK}/messages"),
                &body,
            ))
            .await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
        let dto: api::MessageDTO = serde_json::from_slice(&reply.body).unwrap();
        posted.push(dto.id);
        deliver_to_receiver(&a, bot, dto.id, &net, agent).await;
    }
    let requests = server.received();
    assert_eq!(requests.len(), 3);
    for (index, plain) in ["Ahoy", "Sticker: WaveOutgoing"].iter().enumerate() {
        let body: Value = serde_json::from_slice(&requests[index].body).unwrap();
        let sticker: api::MessageSticker =
            serde_json::from_value(body["message"]["sticker"].clone()).unwrap();
        let api::MessageSticker::Available {
            id,
            name,
            image_url,
            still_url,
            animated,
        } = sticker
        else {
            panic!("available sticker");
        };
        assert_eq!(
            (id, name.as_str(), animated),
            (sticker_id, "WaveOutgoing", true)
        );
        assert!(image_url.contains("/rails/active_storage/blobs/redirect/"));
        assert_eq!(fetch_url(&a, &image_url).await.body, GIF);
        let still = fetch_url(&a, &still_url).await;
        assert_eq!(still.status, StatusCode::OK);
        assert!(still.body.starts_with(b"\x89PNG"));
        assert_eq!(body["message"]["body"]["plain"], *plain);
    }
    let plain: Value = serde_json::from_slice(&requests[2].body).unwrap();
    assert!(plain["message"].get("sticker").is_none());
    let deleted = david
        .write(json_req(
            Method::DELETE,
            &format!("{ADMIN_STICKERS}/{sticker_id}"),
            &Value::Null,
        ))
        .await;
    assert_eq!(deleted.status, StatusCode::OK);
    deliver_to_receiver(&a, bot, posted[1], &net, agent).await;
    let requests = server.received();
    assert_eq!(requests.len(), 4);
    let body: Value = serde_json::from_slice(&requests[3].body).unwrap();
    assert_eq!(
        body["message"]["sticker"],
        json!({"kind":"deleted","name":"WaveOutgoing"})
    );
}

#[tokio::test]
async fn mcp_reads_include_stickers_and_deleted_placeholders() {
    let a = agent_setup().await.without_job_runner().await;
    grant_agent(&a, AGENT, &["read_messages"]).await;
    let sticker_id = db_sticker(&a, "WaveReadMcp").await;
    let message = a
        .db()
        .write(move |tx| {
            Message::create(
                tx,
                campfire_db::NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    sticker_id: Some(sticker_id),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    for deleted in [false, true] {
        if deleted {
            a.db()
                .write(move |tx| Sticker::find(tx.conn(), sticker_id)?.destroy(tx))
                .await
                .unwrap();
        }
        let reply = mcp_call(&a, "read_messages", json!({"room_id":ALL_TALK,"limit":5})).await;
        assert_eq!(reply.status, StatusCode::OK);
        let payload = tool_payload(&reply);
        let entry = payload["messages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["id"] == message.id)
            .unwrap();
        if deleted {
            assert_eq!(
                entry["sticker"],
                json!({"kind":"deleted","name":"WaveReadMcp"})
            );
        } else {
            let sticker: api::MessageSticker =
                serde_json::from_value(entry["sticker"].clone()).unwrap();
            let api::MessageSticker::Available {
                id,
                name,
                image_url,
                ..
            } = sticker
            else {
                panic!("available sticker");
            };
            assert_eq!((id, name.as_str()), (sticker_id, "WaveReadMcp"));
            let fetched = fetch_url(&a, &image_url).await;
            assert_eq!(fetched.status, StatusCode::OK);
            assert_eq!(fetched.body, PNG);
        }
    }
}

#[tokio::test]
async fn bot_sending_preserves_signed_attachments_with_stickers() {
    let a = agent_setup().await.without_job_runner().await;
    grant_agent(&a, AGENT, &["post_messages", "read_messages"]).await;
    let sticker_id = db_sticker(&a, "WaveFile").await;
    let path = format!("/rooms/{ALL_TALK}/{BENDER_KEY}/messages");
    let mut blobs = Vec::new();
    for filename in ["kept.txt", "private.txt"] {
        let staged = a
            .booted
            .app
            .storage
            .stage_bytes(
                b"attachment bytes",
                campfire_storage::Filename::new(filename),
                Some("text/plain"),
            )
            .unwrap();
        let blob = a
            .db()
            .write(move |tx| crate::controllers::messages::save_staged(tx, staged))
            .await
            .unwrap();
        let signed =
            campfire_storage::paths::signed_blob_id(&*a.booted.app.storage.verifier, blob.id, None);
        blobs.push((blob.id, signed));
    }
    let (blob_id, signed_id) = &blobs[0];
    let reply = bot_post(
        &a,
        &path,
        "application/json",
        json!({
            "sticker_id":sticker_id, "attachment":signed_id, "markdown_source":"Together"
        })
        .to_string(),
    )
    .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let id: i64 = reply
        .location()
        .unwrap()
        .rsplit('/')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let saved = a
        .db()
        .read(move |conn| {
            let message = Message::find(conn, id)?;
            let files = message
                .attachments(conn)?
                .into_iter()
                .map(|(_, blob)| blob.id)
                .collect::<Vec<_>>();
            Ok((message.sticker_id, message.markdown_source, files))
        })
        .await
        .unwrap();
    assert_eq!(
        saved,
        (Some(sticker_id), Some("Together".into()), vec![*blob_id])
    );

    let private_blob = blobs[1].0;
    a.db()
        .write(move |tx| {
            Message::create(
                tx,
                campfire_db::NewMessage {
                    room_id: HQ,
                    creator_id: DAVID,
                    attachment_blob_id: Some(private_blob),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let before: i64 = a
        .db()
        .read(|c| Ok(c.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0))?))
        .await
        .unwrap();
    let denied = bot_post(
        &a,
        &path,
        "application/json",
        json!({
            "sticker_id":sticker_id, "attachment":blobs[1].1
        })
        .to_string(),
    )
    .await;
    assert_eq!(
        denied.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        denied.text()
    );
    let after: i64 = a
        .db()
        .read(|c| Ok(c.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0))?))
        .await
        .unwrap();
    assert_eq!(after, before);
}
