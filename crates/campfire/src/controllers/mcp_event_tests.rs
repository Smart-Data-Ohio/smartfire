use super::agent_http_tests::{AGENT, SECRET, initialize};
use super::presenters::test_support::{ALL_TALK, BENDER, DAVID, Req, TestApp};
use crate::integrations::test_support::{
    FakeResolver, FakeServer, MappingDialer, Received, Route, network,
};
use campfire_db::{
    Message, NewMessage, Room,
    models::{agent_delivery as ledger, mcp_events as domain},
};
use campfire_kit::{FrozenClock, Method};
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

const KEY: &str = "whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw";
const KEY2: &str = "whsec_MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY=";
async fn setup() -> (TestApp, FakeServer, Arc<FrozenClock>) {
    let mut route = Route::new("POST", "*", "/hook", 200);
    route.echo_body = true;
    let server = FakeServer::start(vec![route]).await;
    let net = network(
        Arc::new(FakeResolver::new([(
            "events.example",
            vec!["93.184.216.34"],
        )])),
        Arc::new(MappingDialer {
            public: HashSet::from(["93.184.216.34".parse().unwrap()]),
            to: server.addr,
            dialed: Mutex::new(vec![]),
        }),
    );
    let clock = Arc::new(FrozenClock::new("2026-03-02T16:00:00Z".parse().unwrap()));
    let app = TestApp::boot_with_network_clock_and_env(
        net,
        clock.clone(),
        &[("RAILS_ENV", "test"), ("APP_URL", "http://campfire.test")],
    )
    .await
    .unwrap()
    .without_job_runner()
    .await;
    initialize(&app).await;
    app.db()
        .write(|tx| {
            Room::find(tx.conn(), ALL_TALK)?.grant_to(tx, &[BENDER])?;
            for capability in ["read_messages", "post_messages"] {
                campfire_db::AgentGrant::create(
                    tx,
                    campfire_db::NewGrant {
                        agent_id: AGENT,
                        room_id: Some(ALL_TALK),
                        capability: capability.into(),
                        granted_by_id: DAVID,
                        ..Default::default()
                    },
                )?;
            }
            Ok(())
        })
        .await
        .unwrap();
    (app, server, clock)
}
fn params(name: &str) -> Value {
    json!({"name":name,"arguments":{},"delivery":{"mode":"webhook","url":"http://events.example:8080/hook","secret":KEY}})
}
async fn rpc_token(
    app: &TestApp,
    method: &str,
    params: Value,
    token: &str,
    version: &str,
    name: Option<&str>,
) -> (u16, Value) {
    let mut req = Req::new(Method::POST, "/agents/mcp")
        .header("authorization", &format!("Bearer {token}"))
        .header("content-type", "application/json")
        .header("mcp-protocol-version", version)
        .header("mcp-method", method)
        .body(json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}).to_string());
    if let Some(name) = name {
        req = req.header("mcp-name", name);
    }
    let reply = app.anonymous().send(req).await;
    (
        reply.status.as_u16(),
        serde_json::from_slice(&reply.body).unwrap(),
    )
}
async fn rpc(app: &TestApp, method: &str, p: Value) -> Value {
    let name = p["name"].as_str().map(str::to_owned);
    rpc_token(app, method, p, SECRET, "2026-07-28", name.as_deref())
        .await
        .1
}
async fn subscribe(app: &TestApp, name: &str) -> String {
    let v = rpc(app, "events/subscribe", params(name)).await;
    assert!(v.get("error").is_none(), "{v}");
    v["result"]["id"].as_str().unwrap().into()
}
fn signed(r: &Received, key: &str) {
    assert!(rails_compat::webhook::verify_standard_signature(
        key,
        r.header("webhook-id").unwrap(),
        r.header("webhook-timestamp").unwrap(),
        &r.body,
        r.header("webhook-signature").unwrap()
    ));
    assert_eq!(r.header("Content-Type"), Some("application/json"));
}
async fn post(app: &TestApp, text: &str, reply: Option<i64>, creator: i64) -> i64 {
    let text = text.to_owned();
    app.db()
        .write(move |tx| {
            let message = Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: creator,
                    markdown_source: Some(text),
                    reply_to_message_id: reply,
                    ..Default::default()
                },
            )?;
            Ok(message.id)
        })
        .await
        .unwrap()
}
async fn publish(app: &TestApp, message: i64) -> i64 {
    app.db()
        .write(move |tx| {
            let id = tx.conn().query_row(
                "SELECT id FROM agent_events WHERE agent_id=? AND message_id=?",
                rusqlite::params![AGENT, message],
                |r| r.get(0),
            )?;
            ledger::perform_delivery(tx, id)?;
            Ok(id)
        })
        .await
        .unwrap()
}
async fn jobs(app: &TestApp) -> Vec<domain::DeliveryJob> {
    app.db()
        .read(|c| {
            let mut q = c.prepare(
                "SELECT id,attempts FROM mcp_event_deliveries WHERE status='pending' ORDER BY id",
            )?;
            Ok(q.query_map([], |r| {
                Ok(domain::DeliveryJob {
                    delivery_id: r.get(0)?,
                    attempt: r.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?)
        })
        .await
        .unwrap()
}
async fn deliver(app: &TestApp, job: domain::DeliveryJob) {
    campfire_channels::jobs::mcp_events::post_with_network(
        &app.booted.app,
        job,
        &app.booted.app.subscription_network,
    )
    .await
    .unwrap();
}
async fn status(app: &TestApp) -> Value {
    app.db().read(|c|Ok(c.query_row("SELECT status,attempts,last_error FROM mcp_event_deliveries ORDER BY id DESC LIMIT 1",[],|r|Ok(json!([r.get::<_,String>(0)?,r.get::<_,i64>(1)?,r.get::<_,Option<String>>(2)?])))?)).await.unwrap()
}
#[tokio::test]
async fn mcp_events_discovery_and_protocol_errors() {
    let (app, _, _) = setup().await;
    let modern = rpc(&app, "server/discover", json!({})).await;
    assert_eq!(
        modern["result"]["capabilities"]["extensions"]["io.modelcontextprotocol/events"],
        json!({})
    );
    let old = rpc_token(
        &app,
        "server/discover",
        json!({}),
        SECRET,
        "2025-03-26",
        None,
    )
    .await
    .1;
    assert!(old["result"]["capabilities"].get("extensions").is_none());
    assert_eq!(
        rpc_token(&app, "events/list", json!({}), SECRET, "2025-03-26", None)
            .await
            .1["error"]["code"],
        -32601
    );
    let list = rpc(&app, "events/list", json!({})).await;
    assert_eq!(
        list["result"]["events"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["mention", "reply", "direct_message"]
    );
    for (method, value) in [("events/poll", "poll"), ("events/stream", "push")] {
        let v = rpc(&app, method, json!({})).await;
        assert_eq!(v["error"]["code"], -32026);
        assert_eq!(v["error"]["data"]["value"], value);
    }
    let unknown = rpc(&app, "events/unsubscribe", params("unknown")).await;
    assert_eq!(unknown["error"]["code"], -32023);
    assert_eq!(unknown["error"]["data"]["kind"], "subscription");
    let mismatch = rpc_token(
        &app,
        "events/subscribe",
        params("mention"),
        SECRET,
        "2026-07-28",
        Some("reply"),
    )
    .await;
    assert_eq!(mismatch.0, 400);
    assert_eq!(mismatch.1["error"]["code"], -32020);
}
#[tokio::test]
async fn mcp_events_verification_refresh_rotation_and_unsubscribe() {
    let (app, server, clock) = setup().await;
    let id = subscribe(&app, "mention").await;
    let first = server.received();
    assert_eq!(first.len(), 1);
    signed(&first[0], KEY);
    assert_eq!(
        serde_json::from_slice::<Value>(&first[0].body).unwrap()["type"],
        "verification"
    );
    assert!(
        first[0]
            .header("webhook-id")
            .unwrap()
            .starts_with("msg_verification_")
    );
    clock.advance(jiff::SignedDuration::from_mins(10));
    let mut p = params("mention");
    p["delivery"]["secret"] = json!(KEY2);
    p["ttlMs"] = Value::Null;
    let v = rpc(&app, "events/subscribe", p).await;
    assert_eq!(v["result"]["id"], id);
    assert_eq!(v["result"]["refreshBefore"], "2026-03-03T16:10:00Z");
    assert_eq!(v["result"]["deliveryStatus"]["active"], true);
    assert_eq!(v["result"]["truncated"], false);
    assert!(v["result"]["cursor"].is_string());
    assert_eq!(server.received().len(), 1);
    let m = post(&app, "Hey @[Bender Bot]", None, DAVID).await;
    publish(&app, m).await;
    deliver(&app, jobs(&app).await.remove(0)).await;
    let received = server.received();
    let event = received.last().unwrap();
    signed(event, KEY);
    signed(event, KEY2);
    assert_eq!(
        event
            .header("webhook-signature")
            .unwrap()
            .split(' ')
            .count(),
        2
    );
    let v = rpc(&app, "events/unsubscribe", params("mention")).await;
    assert_eq!(v["result"], json!({"resultType":"complete"}));
    assert_eq!(
        rpc(&app, "events/unsubscribe", params("mention")).await["error"]["code"],
        -32023
    );
}
#[tokio::test]
async fn mcp_events_failed_echo_persists_nothing_and_invalid_params_do_not_post() {
    let (app, server, _) = setup().await;
    server.replace_routes(vec![Route::new("POST", "*", "/hook", 200).body("{}")]);
    let v = rpc(&app, "events/subscribe", params("mention")).await;
    assert_eq!(v["error"]["code"], -32027);
    assert_eq!(v["error"]["data"]["reason"], "challenge_failed");
    assert_eq!(
        app.db()
            .read(|c| Ok(
                c.query_row("SELECT COUNT(*) FROM mcp_event_subscriptions", [], |r| r
                    .get::<_, i64>(0))?
            ))
            .await
            .unwrap(),
        0
    );
    for (field, value) in [
        ("secret", json!("bad")),
        ("url", json!("ftp://events.example/a")),
        ("url", json!("/hook")),
        ("secret", json!("whsec_YQ==")),
    ] {
        let mut p = params("mention");
        p["delivery"][field] = value;
        assert_eq!(
            rpc(&app, "events/subscribe", p).await["error"]["code"],
            -32602
        );
    }
    let mut p = params("mention");
    p["arguments"] = json!({"room":1});
    assert_eq!(
        rpc(&app, "events/subscribe", p).await["error"]["code"],
        -32602
    );
    assert_eq!(
        rpc(&app, "events/subscribe", params("missing")).await["error"]["data"]["kind"],
        "event"
    );
    let mut p = params("mention");
    p["delivery"]["mode"] = json!("poll");
    assert_eq!(
        rpc(&app, "events/subscribe", p).await["error"]["code"],
        -32026
    );
    assert_eq!(server.received().len(), 1);
}
#[tokio::test]
async fn mcp_events_mention_matches_polling_and_ack_does_not_cancel_delivery() {
    let (app, server, _) = setup().await;
    let id = subscribe(&app, "mention").await;
    let m = post(&app, "Hey @[Bender Bot]", None, DAVID).await;
    let eid = publish(&app, m).await;
    app.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE agent_events SET outcome='acknowledged' WHERE id=?",
                [eid],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let poll = app
        .anonymous()
        .send(
            Req::new(Method::GET, "/agents/events?envelope=1")
                .header("authorization", &format!("Bearer {SECRET}")),
        )
        .await;
    let poll: Value = serde_json::from_slice(&poll.body).unwrap();
    let expected = poll["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"] == eid)
        .unwrap()
        .clone();
    deliver(&app, jobs(&app).await.remove(0)).await;
    let received = server.received();
    assert_eq!(received.len(), 2);
    let event = &received[1];
    signed(event, KEY);
    let body: Value = serde_json::from_slice(&event.body).unwrap();
    assert_eq!(body["data"], expected);
    assert_eq!(body["name"], "mention");
    assert_eq!(body["eventId"], eid.to_string());
    assert_eq!(event.header("X-MCP-Subscription-Id"), Some(id.as_str()));
    assert_eq!(status(&app).await[0], "delivered");
    app.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE agent_events SET outcome='acknowledged' WHERE id=?",
                [eid],
            )?;
            ledger::perform_delivery(tx, eid)
        })
        .await
        .unwrap();
    assert!(jobs(&app).await.is_empty());
    assert_eq!(
        app.db()
            .read(|c| Ok(
                c.query_row("SELECT COUNT(*) FROM mcp_event_deliveries", [], |r| r
                    .get::<_, i64>(0))?
            ))
            .await
            .unwrap(),
        1
    );
}
#[tokio::test]
async fn mcp_events_reply_wins_over_mention_and_routes_by_name() {
    let (app, server, _) = setup().await;
    subscribe(&app, "mention").await;
    subscribe(&app, "reply").await;
    let root = post(&app, "Agent here", None, BENDER).await;
    let reply = post(&app, "Hey @[Bender Bot]", Some(root), DAVID).await;
    publish(&app, reply).await;
    let jobs = jobs(&app).await;
    assert_eq!(jobs.len(), 1);
    deliver(&app, jobs[0].clone()).await;
    let r = server.received();
    assert_eq!(r.len(), 2);
    assert_eq!(
        serde_json::from_slice::<Value>(&r[1].body).unwrap()["name"],
        "reply"
    );
    signed(&r[1], KEY);
}
#[tokio::test]
async fn mcp_events_retries_keep_id_and_sweep_recovers_stranded_work() {
    let (app, server, clock) = setup().await;
    subscribe(&app, "mention").await;
    let m = post(&app, "Hey @[Bender Bot]", None, DAVID).await;
    publish(&app, m).await;
    server.replace_routes(vec![
        Route::new("POST", "*", "/hook", 503).header("Retry-After", "30"),
    ]);
    let j = jobs(&app).await.remove(0);
    deliver(&app, j.clone()).await;
    deliver(&app, j).await;
    assert_eq!(status(&app).await, json!(["pending", 1, "http_5xx"]));
    assert_eq!(server.received().len(), 2);
    clock.advance(jiff::SignedDuration::from_mins(3));
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "DELETE FROM background_jobs WHERE job_class='Agent::McpEventDeliveryJob'",
                [],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    campfire_channels::jobs::periodic::stranded_agent_webhooks(app.db())
        .await
        .unwrap();
    assert_eq!(
        app.db()
            .read(|c| Ok(c.query_row(
                "SELECT COUNT(*) FROM background_jobs WHERE job_class='Agent::McpEventDeliveryJob'",
                [],
                |r| r.get::<_, i64>(0)
            )?))
            .await
            .unwrap(),
        1
    );
    server.replace_routes(vec![Route::new("POST", "*", "/hook", 200)]);
    deliver(&app, jobs(&app).await.remove(0)).await;
    assert_eq!(status(&app).await, json!(["delivered", 2, null]));
    let r = server.received();
    assert_eq!(r[1].header("webhook-id"), r[2].header("webhook-id"));
    assert_ne!(
        r[1].header("webhook-timestamp"),
        r[2].header("webhook-timestamp")
    );
    signed(&r[2], KEY);
    let v = rpc(&app, "events/subscribe", params("mention")).await;
    assert!(v["result"]["deliveryStatus"]["lastDeliveryAt"].is_string());
    assert!(v["result"]["deliveryStatus"]["lastError"].is_null());
}
#[tokio::test]
async fn mcp_events_permanent_and_exhausted_failures() {
    for status_code in [410, 413, 400, 503] {
        let (app, server, clock) = setup().await;
        subscribe(&app, "mention").await;
        let m = post(&app, "Hey @[Bender Bot]", None, DAVID).await;
        publish(&app, m).await;
        server.replace_routes(vec![Route::new("POST", "*", "/hook", status_code)]);
        for _ in 0..if status_code == 503 { 5 } else { 1 } {
            deliver(&app, jobs(&app).await.remove(0)).await;
            clock.advance(jiff::SignedDuration::from_mins(8));
        }
        let result = status(&app).await;
        assert_eq!(result[0], "failed");
        assert_eq!(result[1], if status_code == 503 { 5 } else { 1 });
        assert!(jobs(&app).await.is_empty());
    }
}
#[tokio::test]
async fn mcp_events_grant_revoked_suppresses_and_dead_credential_terminates_once() {
    for credential in [false, true] {
        let (app, server, _) = setup().await;
        subscribe(&app, "mention").await;
        for _ in 0..2 {
            let m = post(&app, "Hey @[Bender Bot]", None, DAVID).await;
            publish(&app, m).await;
        }
        let pending = jobs(&app).await;
        app.db()
            .write(move |tx| {
                tx.conn().execute(
                    if credential {
                        "UPDATE agent_credentials SET revoked_at=? WHERE agent_id=?"
                    } else {
                        "UPDATE agent_grants SET revoked_at=? WHERE agent_id=?"
                    },
                    rusqlite::params![tx.now(), AGENT],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        for job in pending {
            deliver(&app, job).await;
        }
        let r = server.received();
        assert_eq!(r.len(), if credential { 2 } else { 1 });
        if credential {
            signed(&r[1], KEY);
            assert_eq!(
                serde_json::from_slice::<Value>(&r[1].body).unwrap(),
                json!({"type":"terminated","error":{"code":-32024,"message":"Access revoked"}})
            );
        } else {
            assert_eq!(status(&app).await[0], "suppressed");
        }
    }
}
#[tokio::test]
async fn mcp_events_limit_access_and_credential_ownership() {
    let (app, server, _) = setup().await;
    subscribe(&app, "mention").await;
    let other = app
        .db()
        .write(|tx| {
            Ok(
                campfire_db::AgentCredential::create_with_secret(tx, AGENT, "Other", DAVID, None)?
                    .1,
            )
        })
        .await
        .unwrap();
    let v = rpc_token(
        &app,
        "events/unsubscribe",
        params("mention"),
        &other,
        "2026-07-28",
        Some("mention"),
    )
    .await
    .1;
    assert_eq!(v["error"]["code"], -32023);
    for i in 1..20 {
        let mut p = params("mention");
        p["delivery"]["url"] = json!(format!("http://events.example:8080/hook?{i}"));
        app.db().write(move |tx| {
            let c:i64=tx.conn().query_row("SELECT id FROM agent_credentials WHERE name='HTTP contract'",[],|r|r.get(0))?;
            let id=domain::subscription_id(c,p["delivery"]["url"].as_str().unwrap(),"mention");
            tx.conn().execute("INSERT INTO mcp_event_subscriptions SELECT ?,agent_id,agent_credential_id,name,arguments,?,secret,previous_secret,previous_secret_expires_at,refresh_before,verified_at,watermark_event_id,last_delivery_at,last_error,failed_since,created_at,updated_at FROM mcp_event_subscriptions LIMIT 1",rusqlite::params![id,p["delivery"]["url"].as_str().unwrap()])?;Ok(())
        }).await.unwrap();
    }
    let v = rpc(&app, "events/subscribe", params("reply")).await;
    assert_eq!(v["error"]["code"], -32025);
    assert_eq!(
        v["error"]["data"],
        json!({"limit":"subscriptions","max":20})
    );
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE agent_grants SET revoked_at=? WHERE agent_id=?",
                rusqlite::params![tx.now(), AGENT],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        rpc(&app, "events/subscribe", params("mention")).await["error"]["code"],
        -32024
    );
    assert_eq!(server.received().len(), 1);
}

#[tokio::test]
async fn mcp_events_replay_is_bounded_and_watermark_waits_for_earlier_deliveries() {
    let (app, server, clock) = setup().await;
    let old = post(&app, "Hey @[Bender Bot]", None, DAVID).await;
    let old = publish(&app, old).await;
    clock.advance(jiff::SignedDuration::from_hours(25));
    let m = post(&app, "Hey @[Bender Bot]", None, DAVID).await;
    let e1 = publish(&app, m).await;
    let m = post(&app, "Hey @[Bender Bot]", None, DAVID).await;
    let e2 = publish(&app, m).await;
    let mut p = params("mention");
    p["cursor"] = json!("0");
    let result = rpc(&app, "events/subscribe", p).await;
    assert_eq!(result["result"]["truncated"], true);
    let pending = jobs(&app).await;
    assert_eq!(pending.len(), 2);
    deliver(&app, pending[1].clone()).await;
    assert_eq!(
        rpc(&app, "events/subscribe", params("mention")).await["result"]["cursor"],
        "0"
    );
    deliver(&app, pending[0].clone()).await;
    assert_eq!(
        rpc(&app, "events/subscribe", params("mention")).await["result"]["cursor"],
        e2.to_string()
    );
    let r = server.received();
    assert_eq!(r.len(), 3);
    let b: Value = serde_json::from_slice(&r[2].body).unwrap();
    assert_eq!(b["eventId"], e1.to_string());
    assert_eq!(b["cursor"], e2.to_string());
    assert_ne!(b["eventId"], old.to_string());
}

#[tokio::test]
async fn mcp_events_oauth_refresh_keeps_subscription_and_revocation_or_block_terminates() {
    use campfire_db::models::oauth::{OAuthConnection, TokenKind};
    use sha2::{Digest, Sha256};
    for reason in ["revoked", "blocked", "expired"] {
        let (app, server, clock) = setup().await;
        let (agent,bot,connection)=app.db().write(|tx| {
            let agent=campfire_db::models::personal_agents::create(tx,campfire_db::NewUser {name:"Events Personal".into(),..Default::default()},DAVID)?;
            Room::find(tx.conn(),ALL_TALK)?.grant_to(tx,&[agent.user_id])?;
            campfire_db::AgentGrant::create(tx,campfire_db::NewGrant {agent_id:agent.id,room_id:Some(ALL_TALK),capability:"read_messages".into(),granted_by_id:DAVID,..Default::default()})?;
            let client=tx.conn().query_row("INSERT INTO oauth_clients(client_id,registration,client_name,redirect_uris,created_at,updated_at) VALUES('events-client','dcr','Events','[\"https://client.example/cb\"]',?,?) RETURNING id",rusqlite::params![tx.now(),tx.now()],|r|r.get(0))?;
            let connection=OAuthConnection::create(tx,agent.id,DAVID,client)?;
            for secret in ["mcp-events-access-1","mcp-events-access-2"] {
                tx.conn().execute("INSERT INTO oauth_tokens(oauth_connection_id,kind,token_digest,expires_at,created_at) VALUES(?,'access',?,?,?)",rusqlite::params![connection.id,format!("{:x}",Sha256::digest(secret)),connection.token_expires_at(tx.now(),TokenKind::Access),tx.now()])?;
            }
            Ok((agent.id,agent.user_id,connection.id))
        }).await.unwrap();
        let first = rpc_token(
            &app,
            "events/subscribe",
            params("mention"),
            "mcp-events-access-1",
            "2026-07-28",
            Some("mention"),
        )
        .await
        .1;
        assert!(first.get("error").is_none(), "{first}");
        let second = rpc_token(
            &app,
            "events/subscribe",
            params("mention"),
            "mcp-events-access-2",
            "2026-07-28",
            Some("mention"),
        )
        .await
        .1;
        assert_eq!(first["result"]["id"], second["result"]["id"]);
        let list = rpc_token(
            &app,
            "events/list",
            json!({}),
            "mcp-events-access-2",
            "2026-07-28",
            None,
        )
        .await
        .1;
        assert_eq!(list["result"]["events"].as_array().unwrap().len(), 3);
        app.db()
            .write(move |tx| {
                let name = campfire_db::User::find(tx.conn(), bot)?.name;
                let m = Message::create(
                    tx,
                    NewMessage {
                        room_id: ALL_TALK,
                        creator_id: DAVID,
                        markdown_source: Some(format!("Hey @[{name}]")),
                        ..Default::default()
                    },
                )?;
                let event = tx.conn().query_row(
                    "SELECT id FROM agent_events WHERE agent_id=? AND message_id=?",
                    rusqlite::params![agent, m.id],
                    |r| r.get(0),
                )?;
                ledger::perform_delivery(tx, event)
            })
            .await
            .unwrap();
        let pending = jobs(&app).await;
        assert_eq!(pending.len(), 1);
        let reason = reason.to_owned();
        app.db()
            .write(move |tx| {
                let c = OAuthConnection::find(tx.conn(), connection)?.unwrap();
                match reason.as_str() {
                    "revoked" => c.revoke(tx)?,
                    "blocked" => {
                        tx.conn().execute(
                            "UPDATE oauth_clients SET blocked_at=? WHERE id=?",
                            rusqlite::params![tx.now(), c.oauth_client_id],
                        )?;
                    }
                    _ => {
                        tx.conn().execute(
                            "UPDATE oauth_connections SET expires_at=? WHERE id=?",
                            rusqlite::params![tx.now(), connection],
                        )?;
                    }
                }
                Ok(())
            })
            .await
            .unwrap();
        clock.advance(jiff::SignedDuration::from_secs(1));
        deliver(&app, pending[0].clone()).await;
        let r = server.received();
        assert_eq!(r.len(), 2);
        signed(&r[1], KEY);
        assert_eq!(
            serde_json::from_slice::<Value>(&r[1].body).unwrap()["type"],
            "terminated"
        );
        assert_eq!(
            app.db()
                .read(|c| Ok(c.query_row(
                    "SELECT COUNT(*) FROM mcp_event_subscriptions",
                    [],
                    |r| r.get::<_, i64>(0)
                )?))
                .await
                .unwrap(),
            0
        );
    }
}

#[tokio::test]
async fn mcp_events_direct_message_and_expired_subscription() {
    let (app, server, clock) = setup().await;
    subscribe(&app, "direct_message").await;
    let e = app
        .db()
        .write(|tx| {
            let room = Room::create_for(
                tx,
                campfire_db::RoomType::Direct,
                None,
                DAVID,
                &[DAVID, BENDER],
            )?;
            campfire_db::AgentGrant::create(
                tx,
                campfire_db::NewGrant {
                    agent_id: AGENT,
                    room_id: Some(room.id),
                    capability: "read_messages".into(),
                    granted_by_id: DAVID,
                    ..Default::default()
                },
            )?;
            let m = Message::create(
                tx,
                NewMessage {
                    room_id: room.id,
                    creator_id: DAVID,
                    markdown_source: Some("A direct message".into()),
                    ..Default::default()
                },
            )?;
            let e = tx.conn().query_row(
                "SELECT id FROM agent_events WHERE agent_id=? AND message_id=?",
                rusqlite::params![AGENT, m.id],
                |r| r.get(0),
            )?;
            ledger::perform_delivery(tx, e)?;
            Ok(e)
        })
        .await
        .unwrap();
    let job = jobs(&app).await.remove(0);
    deliver(&app, job).await;
    let r = server.received();
    assert_eq!(r.len(), 2);
    let body: Value = serde_json::from_slice(&r[1].body).unwrap();
    assert_eq!(body["name"], "direct_message");
    assert_eq!(body["eventId"], e.to_string());
    signed(&r[1], KEY);
    subscribe(&app, "mention").await;
    let m = post(&app, "Hey @[Bender Bot]", None, DAVID).await;
    publish(&app, m).await;
    clock.advance(jiff::SignedDuration::from_hours(2));
    deliver(&app, jobs(&app).await.remove(0)).await;
    assert_eq!(server.received().len(), 2);
    assert_eq!(status(&app).await[0], "suppressed");
    clock.advance(jiff::SignedDuration::from_hours(24));
    campfire_channels::jobs::periodic::stranded_agent_webhooks(app.db())
        .await
        .unwrap();
    assert_eq!(
        app.db()
            .read(|c| Ok(
                c.query_row("SELECT COUNT(*) FROM mcp_event_subscriptions", [], |r| r
                    .get::<_, i64>(0))?
            ))
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn mcp_events_guard_blocks_private_hosts_and_never_follows_redirects() {
    let (app, server, _) = setup().await;
    let mut p = params("mention");
    p["delivery"]["url"] = json!("http://127.0.0.1/hook");
    let v = rpc(&app, "events/subscribe", p).await;
    assert_eq!(v["error"]["code"], -32027);
    assert!(server.received().is_empty());
    server.replace_routes(vec![
        Route::new("POST", "*", "/hook", 302).header("Location", "http://events.example/redirect"),
    ]);
    let v = rpc(&app, "events/subscribe", params("mention")).await;
    assert_eq!(v["error"]["code"], -32027);
    assert_eq!(server.received().len(), 1);
}

#[tokio::test]
async fn mcp_events_deleted_message_and_hop_limit_never_post() {
    let (app, server, _) = setup().await;
    subscribe(&app, "mention").await;
    let m = post(&app, "Hey @[Bender Bot]", None, DAVID).await;
    publish(&app, m).await;
    let job = jobs(&app).await.remove(0);
    app.db()
        .write(move |tx| Message::find(tx.conn(), m)?.destroy(tx))
        .await
        .unwrap();
    deliver(&app, job).await;
    assert_eq!(status(&app).await[0], "suppressed");
    let m = post(&app, "Hey @[Bender Bot]", None, DAVID).await;
    app.db()
        .write(move |tx| {
            let id = tx.conn().query_row(
                "SELECT id FROM agent_events WHERE agent_id=? AND message_id=?",
                rusqlite::params![AGENT, m],
                |r| r.get::<_, i64>(0),
            )?;
            tx.conn().execute(
                "UPDATE agent_events SET metadata='{\"hop\":3}' WHERE id=?",
                [id],
            )?;
            ledger::perform_delivery(tx, id)
        })
        .await
        .unwrap();
    assert!(jobs(&app).await.is_empty());
    assert_eq!(server.received().len(), 1);
}

#[tokio::test]
async fn mcp_events_oversized_payload_fails_without_post_or_retry() {
    let (app, server, _) = setup().await;
    subscribe(&app, "mention").await;
    let m = post(
        &app,
        &format!("Hey @[Bender Bot] {}", "&".repeat(40_000)),
        None,
        DAVID,
    )
    .await;
    publish(&app, m).await;
    deliver(&app, jobs(&app).await.remove(0)).await;
    assert_eq!(status(&app).await[0], "failed");
    assert!(jobs(&app).await.is_empty());
    assert_eq!(server.received().len(), 1);
}

#[tokio::test]
async fn mcp_events_production_rejects_http_callbacks() {
    let app = super::agent_http_tests::setup()
        .await
        .without_job_runner()
        .await;
    let result = rpc(&app, "events/subscribe", params("mention")).await;
    assert_eq!(result["error"]["code"], -32602);
    assert_eq!(
        app.db()
            .read(|c| Ok(
                c.query_row("SELECT COUNT(*) FROM mcp_event_subscriptions", [], |r| r
                    .get::<_, i64>(0))?
            ))
            .await
            .unwrap(),
        0
    );
}
