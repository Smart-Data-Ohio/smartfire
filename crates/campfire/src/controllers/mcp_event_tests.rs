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
    setup_with_runner(false).await
}
async fn setup_with_runner(run_jobs: bool) -> (TestApp, FakeServer, Arc<FrozenClock>) {
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
    .unwrap();
    let app = if run_jobs { app } else { app.without_job_runner().await };
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
        if reply.body.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&reply.body).unwrap()
        },
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
    assert_eq!(modern["result"]["capabilities"]["events"], json!({}));
    for version in ["2025-03-26", "2025-11-25"] {
        let init = rpc_token(
            &app,
            "initialize",
            json!({"protocolVersion":version}),
            SECRET,
            version,
            None,
        )
        .await
        .1;
        assert_eq!(init["result"]["capabilities"]["events"], json!({}));
        let list = rpc_token(&app, "events/list", json!({}), SECRET, version, None)
            .await
            .1;
        assert_eq!(list["result"]["events"].as_array().unwrap().len(), 3);
        assert!(list["result"].get("resultType").is_none());
    }
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
    assert_eq!(unknown["result"], json!({"resultType":"complete"}));
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
    assert_eq!(first[0].header("X-MCP-Subscription-Id"), Some(id.as_str()));
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
        rpc(&app, "events/unsubscribe", params("mention")).await["result"],
        json!({"resultType":"complete"})
    );
}
#[tokio::test]
async fn mcp_events_failed_echo_persists_nothing_and_invalid_params_do_not_post() {
    let (app, server, _) = setup().await;
    server.replace_routes(vec![Route::new("POST", "*", "/hook", 200).body("{}")]);
    let v = rpc(&app, "events/subscribe", params("mention")).await;
    assert_eq!(v["error"]["code"], -32015);
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
    assert_eq!(body["eventId"], event.header("webhook-id").unwrap());
    assert!(body["eventId"].as_str().unwrap().starts_with("msg_"));
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
async fn mcp_events_reply_wins_over_mention_and_retries_the_polling_row() {
    let (app, server, clock) = setup().await;
    subscribe(&app, "mention").await;
    subscribe(&app, "reply").await;
    let root = post(&app, "Agent here", None, BENDER).await;
    let reply = post(&app, "Hey @[Bender Bot]", Some(root), DAVID).await;
    let eid = publish(&app, reply).await;
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
        .find(|row| row["id"] == eid)
        .unwrap();
    let pending = jobs(&app).await;
    assert_eq!(pending.len(), 1);
    server.replace_routes(vec![Route::new("POST", "*", "/hook", 503)]);
    deliver(&app, pending[0].clone()).await;
    clock.advance(jiff::SignedDuration::from_secs(31));
    server.replace_routes(vec![Route::new("POST", "*", "/hook", 200)]);
    deliver(&app, jobs(&app).await.remove(0)).await;
    let received = server.received();
    assert_eq!(received.len(), 3);
    assert_eq!(received[1].body, received[2].body);
    assert_ne!(
        received[1].header("webhook-timestamp"),
        received[2].header("webhook-timestamp")
    );
    for r in &received[1..] {
        signed(r, KEY);
        let body: Value = serde_json::from_slice(&r.body).unwrap();
        assert_eq!(body["name"], "reply");
        assert_eq!(body["data"], *expected);
        assert_eq!(body["eventId"], r.header("webhook-id").unwrap());
        assert!(body["eventId"].as_str().unwrap().starts_with("msg_"));
    }
    assert_eq!(status(&app).await, json!(["delivered", 2, null]));
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
    app.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE agent_events SET outcome='acknowledged' WHERE message_id=?",
                [m],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    server.replace_routes(vec![Route::new("POST", "*", "/hook", 200)]);
    deliver(&app, jobs(&app).await.remove(0)).await;
    assert_eq!(status(&app).await, json!(["delivered", 2, null]));
    let r = server.received();
    assert_eq!(r[1].header("webhook-id"), r[2].header("webhook-id"));
    assert_ne!(
        r[1].header("webhook-timestamp"),
        r[2].header("webhook-timestamp")
    );
    signed(&r[1], KEY);
    signed(&r[2], KEY);
    assert_eq!(r[1].body, r[2].body);
    let body: Value = serde_json::from_slice(&r[2].body).unwrap();
    assert_eq!(body["eventId"], r[2].header("webhook-id").unwrap());
    assert!(body["eventId"].as_str().unwrap().starts_with("msg_"));
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
async fn mcp_events_grant_revoked_suppresses_and_dead_credential_deletes_silently() {
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
        assert_eq!(r.len(), 1);
        if credential {
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
            assert!(jobs(&app).await.is_empty());
            // A revoked bearer fails authentication before the Events dispatcher.
            assert_eq!(
                rpc_token(
                    &app,
                    "events/subscribe",
                    params("mention"),
                    SECRET,
                    "2026-07-28",
                    None
                )
                .await
                .0,
                401
            );
        } else {
            assert_eq!(status(&app).await[0], "suppressed");
            assert_eq!(
                rpc(&app, "events/subscribe", params("mention")).await["error"]["code"],
                -32024
            );
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
    assert_eq!(v["result"], json!({"resultType":"complete"}));
    assert_eq!(subscribe(&app, "mention").await.len(), 32);
    for i in 1..20 {
        let mut p = params("mention");
        p["delivery"]["url"] = json!(format!("http://events.example:8080/hook?{i}"));
        app.db().write(move |tx| {
            let c:i64=tx.conn().query_row("SELECT id FROM agent_credentials WHERE name='HTTP contract'",[],|r|r.get(0))?;
            let id=domain::subscription_id(c,p["delivery"]["url"].as_str().unwrap(),"mention", &domain::Arguments::default());
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
        (e1 - 1).to_string()
    );
    deliver(&app, pending[0].clone()).await;
    assert_eq!(
        rpc(&app, "events/subscribe", params("mention")).await["result"]["cursor"],
        e2.to_string()
    );
    let r = server.received();
    assert_eq!(r.len(), 3);
    let b: Value = serde_json::from_slice(&r[2].body).unwrap();
    assert_eq!(b["data"]["id"], e1);
    assert_eq!(b["eventId"], r[2].header("webhook-id").unwrap());
    assert_eq!(b["cursor"], e2.to_string());
    assert_ne!(b["data"]["id"], old);
}

#[tokio::test]
async fn mcp_events_oauth_refresh_keeps_subscription_and_revocation_or_block_deletes_silently() {
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
        assert_eq!(r.len(), 1);
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
    assert_eq!(body["data"]["id"], e);
    assert_eq!(body["eventId"], r[1].header("webhook-id").unwrap());
    signed(&r[1], KEY);
    subscribe(&app, "mention").await;
    let m = post(&app, "Hey @[Bender Bot]", None, DAVID).await;
    publish(&app, m).await;
    clock.advance(jiff::SignedDuration::from_hours(2));
    deliver(&app, jobs(&app).await.remove(0)).await;
    assert_eq!(server.received().len(), 2);
    assert_eq!(status(&app).await[0], "expired");
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
    assert_eq!(v["error"]["code"], -32015);
    assert!(server.received().is_empty());
    server.replace_routes(vec![
        Route::new("POST", "*", "/hook", 302).header("Location", "http://events.example/redirect"),
    ]);
    let v = rpc(&app, "events/subscribe", params("mention")).await;
    assert_eq!(v["error"]["code"], -32015);
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

#[tokio::test]
async fn mcp_events_legacy_client_subscribes_receives_and_unsubscribes_without_name_header() {
    for version in ["2025-03-26", "2025-11-25", "2026-07-28"] {
        let (app, server, _) = setup().await;
        for name in [None, Some("poll_events")] {
            let result = rpc_token(
                &app,
                "tools/call",
                json!({"name":"poll_events","arguments":{}}),
                SECRET,
                version,
                name,
            )
            .await;
            assert_eq!(result.0, 200);
            assert_eq!(result.1["result"]["isError"], false);
        }
        let mismatch = rpc_token(
            &app,
            "tools/call",
            json!({"name":"poll_events","arguments":{}}),
            SECRET,
            version,
            Some("ack_events"),
        )
        .await;
        assert_eq!(mismatch.0, 400);
        assert_eq!(mismatch.1["error"]["code"], -32020);
        let mismatch = rpc_token(
            &app,
            "events/subscribe",
            params("mention"),
            SECRET,
            version,
            Some("reply"),
        )
        .await;
        assert_eq!(mismatch.0, 400);
        assert_eq!(mismatch.1["error"]["code"], -32020);
        let result = rpc_token(
            &app,
            "events/subscribe",
            params("mention"),
            SECRET,
            version,
            None,
        )
        .await
        .1;
        assert!(result["result"]["id"].is_string(), "{result}");
        assert_eq!(
            result["result"].get("resultType").is_some(),
            version == "2026-07-28"
        );
        let m = post(&app, "Hey @[Bender Bot]", None, DAVID).await;
        publish(&app, m).await;
        deliver(&app, jobs(&app).await.remove(0)).await;
        let received = server.received();
        assert_eq!(received.len(), 2);
        signed(&received[0], KEY);
        signed(&received[1], KEY);
        let body: Value = serde_json::from_slice(&received[1].body).unwrap();
        assert_eq!(body["name"], "mention");
        for name in ["mention", "mention", "unknown"] {
            let result = rpc_token(
                &app,
                "events/unsubscribe",
                params(name),
                SECRET,
                version,
                None,
            )
            .await
            .1;
            assert_eq!(
                result["result"],
                if version == "2026-07-28" {
                    json!({"resultType":"complete"})
                } else {
                    json!({})
                }
            );
        }
        assert!(jobs(&app).await.is_empty());
    }
}

#[tokio::test]
async fn mcp_events_urls_have_distinct_event_ids_for_the_same_occurrence() {
    let (app, server, _) = setup().await;
    subscribe(&app, "mention").await;
    let mut hook = Route::new("POST", "*", "/second", 200);
    hook.echo_body = true;
    server.replace_routes(vec![Route::new("POST", "*", "/hook", 200), hook]);
    let mut second = params("mention");
    second["delivery"]["url"] = json!("http://events.example:8080/second");
    assert!(rpc(&app, "events/subscribe", second).await["result"]["id"].is_string());
    let m = post(&app, "Hey @[Bender Bot]", None, DAVID).await;
    let eid = publish(&app, m).await;
    let pending = jobs(&app).await;
    assert_eq!(pending.len(), 2);
    for job in pending {
        deliver(&app, job).await;
    }
    let received = server.received();
    assert_eq!(received.len(), 4);
    let bodies: Vec<Value> = received[2..]
        .iter()
        .map(|r| {
            signed(r, KEY);
            let body: Value = serde_json::from_slice(&r.body).unwrap();
            assert_eq!(body["eventId"], r.header("webhook-id").unwrap());
            assert!(body["eventId"].as_str().unwrap().starts_with("msg_"));
            assert_eq!(body["data"]["id"], eid);
            body
        })
        .collect();
    assert_ne!(bodies[0]["eventId"], bodies[1]["eventId"]);
    assert_eq!(bodies[0]["data"], bodies[1]["data"]);
}

#[tokio::test]
async fn mcp_events_room_filters_validate_access_canonicalize_and_limit_live_and_replay() {
    let (app, server, _) = setup().await;
    let (inside, outside, denied, no_membership) = app
        .db()
        .write(|tx| {
            let inside = Room::create_for(
                tx,
                campfire_db::RoomType::Closed,
                Some("Inside"),
                DAVID,
                &[DAVID, BENDER],
            )?
            .id;
            let outside = Room::create_for(
                tx,
                campfire_db::RoomType::Closed,
                Some("Outside"),
                DAVID,
                &[DAVID, BENDER],
            )?
            .id;
            let denied = Room::create_for(
                tx,
                campfire_db::RoomType::Closed,
                Some("No grant"),
                DAVID,
                &[DAVID, BENDER],
            )?
            .id;
            let no_membership = Room::create_for(
                tx,
                campfire_db::RoomType::Closed,
                Some("No membership"),
                DAVID,
                &[DAVID],
            )?
            .id;
            for room in [inside, outside, no_membership] {
                campfire_db::AgentGrant::create(
                    tx,
                    campfire_db::NewGrant {
                        agent_id: AGENT,
                        room_id: Some(room),
                        capability: "read_messages".into(),
                        granted_by_id: DAVID,
                        ..Default::default()
                    },
                )?;
            }
            Ok((inside, outside, denied, no_membership))
        })
        .await
        .unwrap();
    for room in [denied, no_membership, i64::MAX] {
        let mut p = params("mention");
        p["arguments"] = json!({"roomIds":[room]});
        assert_eq!(
            rpc(&app, "events/subscribe", p).await["error"]["code"],
            -32024
        );
    }
    for arguments in [
        json!({"roomIds":[]}),
        json!({"roomIds":[inside,inside]}),
        json!({"roomIds":[0]}),
        json!({"roomIds":[-1]}),
        json!({"roomIds":[1.5]}),
        json!({"roomIds":["1"]}),
        json!({"roomIds":null}),
        json!({"roomIds":"1"}),
        json!({"roomIds":(1..=51).collect::<Vec<_>>()}),
        json!({"roomIds":[inside],"unknown":true}),
        json!([]),
    ] {
        let mut p = params("mention");
        p["arguments"] = arguments;
        assert_eq!(
            rpc(&app, "events/subscribe", p).await["error"]["code"],
            -32602
        );
    }
    assert!(server.received().is_empty());
    let list = rpc(&app, "events/list", json!({})).await;
    for event in list["result"]["events"].as_array().unwrap() {
        let schema = &event["inputSchema"];
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(
            schema["properties"]["roomIds"]["items"],
            json!({"type":"integer","minimum":1})
        );
        assert_eq!(schema["properties"]["roomIds"]["minItems"], 1);
        assert_eq!(schema["properties"]["roomIds"]["maxItems"], 50);
        assert_eq!(schema["properties"]["roomIds"]["uniqueItems"], true);
    }
    let mut p = params("mention");
    p["arguments"] = json!({"roomIds":[inside,ALL_TALK]});
    let first = rpc(&app, "events/subscribe", p.clone()).await;
    assert!(first["result"]["id"].is_string(), "{first}");
    p["arguments"] = json!({"roomIds":[ALL_TALK,inside]});
    assert_eq!(
        first["result"]["id"],
        rpc(&app, "events/subscribe", p.clone()).await["result"]["id"]
    );
    let stored: Value = app
        .db()
        .read(|c| {
            Ok(
                c.query_row("SELECT arguments FROM mcp_event_subscriptions", [], |r| {
                    r.get::<_, Value>(0)
                })?,
            )
        })
        .await
        .unwrap();
    let mut sorted = vec![inside, ALL_TALK];
    sorted.sort_unstable();
    assert_eq!(stored, json!({"roomIds":sorted}));
    let mut events = vec![];
    for room in [outside, inside, ALL_TALK] {
        let m = app
            .db()
            .write(move |tx| {
                Ok(Message::create(
                    tx,
                    NewMessage {
                        room_id: room,
                        creator_id: DAVID,
                        markdown_source: Some("Hey @[Bender Bot]".into()),
                        ..Default::default()
                    },
                )?
                .id)
            })
            .await
            .unwrap();
        events.push(publish(&app, m).await);
    }
    let pending = jobs(&app).await;
    assert_eq!(pending.len(), 2);
    for job in pending {
        deliver(&app, job).await;
    }
    let received = server.received();
    assert_eq!(received.len(), 3);
    for (r, eid) in received[1..].iter().zip(&events[1..]) {
        signed(r, KEY);
        assert_eq!(
            serde_json::from_slice::<Value>(&r.body).unwrap()["data"]["id"],
            *eid
        );
    }
    p["delivery"].as_object_mut().unwrap().remove("secret");
    assert_eq!(
        rpc(&app, "events/unsubscribe", p.clone()).await["result"],
        json!({"resultType":"complete"})
    );
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
    p["delivery"]["secret"] = json!(KEY);
    p["cursor"] = json!("0");
    assert!(rpc(&app, "events/subscribe", p).await["result"]["id"].is_string());
    assert_eq!(jobs(&app).await.len(), 2);
}

#[tokio::test]
async fn mcp_events_verification_expires_without_refresh_extending_it() {
    let (app, server, clock) = setup().await;
    let id = subscribe(&app, "mention").await;
    clock.advance(jiff::SignedDuration::from_hours(23));
    subscribe(&app, "mention").await;
    assert_eq!(server.received().len(), 1);
    let mut p = params("mention");
    p["ttlMs"] = Value::Null;
    let before = rpc(&app, "events/subscribe", p).await;
    clock.advance(jiff::SignedDuration::from_hours(1));
    // Delivery remains valid until refreshBefore, even after verification expires.
    let m = post(&app, "Hey @[Bender Bot]", None, DAVID).await;
    publish(&app, m).await;
    deliver(&app, jobs(&app).await.remove(0)).await;
    assert_eq!(status(&app).await[0], "delivered");
    server.replace_routes(vec![Route::new("POST", "*", "/hook", 200).body("{}")]);
    let failed = rpc(&app, "events/subscribe", params("mention")).await;
    assert_eq!(failed["error"]["code"], -32015);
    let saved = app
        .db()
        .read(move |c| domain::Subscription::find(c, &id))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        saved.refresh_before.jiff().to_string(),
        before["result"]["refreshBefore"]
    );
    let mut route = Route::new("POST", "*", "/hook", 200);
    route.echo_body = true;
    server.replace_routes(vec![route]);
    subscribe(&app, "reply").await;
    let count = server.received().len();
    subscribe(&app, "mention").await;
    assert_eq!(
        server.received().len(),
        count,
        "verification is shared by credential and URL"
    );
    clock.advance(jiff::SignedDuration::from_hours(24));
    subscribe(&app, "mention").await;
    assert_eq!(server.received().len(), count + 1);
}

#[tokio::test]
async fn mcp_events_retry_suppresses_changed_private_repository_access() {
    use crate::integrations::agent_repositories::{RepositoryReader, RepositoryRequest};
    struct AllowRepository;
    impl RepositoryReader for AllowRepository {
        fn readable(
            &self,
            _: RepositoryRequest,
        ) -> crate::net::BoxFuture<'_, campfire_db::Result<bool>> {
            Box::pin(async { Ok(true) })
        }
    }
    let (app, server, clock) = setup().await;
    app.booted
        .app
        .agent_repositories
        .install(Arc::new(AllowRepository));
    subscribe(&app, "mention").await;
    let crypto = app.booted.app.ar_encryption.clone();
    let event = app.db().write(move |tx| {
        tx.conn().execute("UPDATE agents SET owner_id=? WHERE id=?", rusqlite::params![DAVID, AGENT])?;
        tx.conn().execute("DELETE FROM github_connected_accounts WHERE user_id=?", [DAVID])?;
        tx.conn().execute("INSERT INTO github_connected_accounts(user_id,github_login,access_token,created_at,updated_at) VALUES(?,'events-owner',?,?,?)", rusqlite::params![DAVID,crypto.encrypt("fake-token"),tx.now(),tx.now()])?;
        tx.conn().execute("INSERT INTO channel_threads(id,name,room_id,creator_id,last_activity_at,created_at,updated_at) VALUES(1996101000,'Private PR',?,?,?,?,?)", rusqlite::params![ALL_TALK,DAVID,tx.now(),tx.now(),tx.now()])?;
        tx.conn().execute("INSERT INTO github_pull_requests(id,owner,repo,number,title,head_branch,private,created_at,updated_at) VALUES(1996102000,'events','private',1,'Private title','private-branch',1,?,?)", [tx.now(),tx.now()])?;
        tx.conn().execute("INSERT INTO github_pull_request_threads(channel_thread_id,github_pull_request_id,room_id,created_at,updated_at) VALUES(1996101000,1996102000,?,?,?)", rusqlite::params![ALL_TALK,tx.now(),tx.now()])?;
        let m = Message::create(tx, NewMessage {room_id:ALL_TALK,creator_id:DAVID,thread_id:Some(1996101000),markdown_source:Some("Hey @[Bender Bot]".into()),..Default::default()})?;
        Ok(m.id)
    }).await.unwrap();
    publish(&app, event).await;
    server.replace_routes(vec![Route::new("POST", "*", "/hook", 503)]);
    deliver(&app, jobs(&app).await.remove(0)).await;
    let body: Value = serde_json::from_slice(&server.received()[1].body).unwrap();
    assert!(body.to_string().contains("Private title"), "{body}");
    assert!(body.to_string().contains("private-branch"));
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "DELETE FROM github_connected_accounts WHERE user_id=?",
                [DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    clock.advance(jiff::SignedDuration::from_secs(31));
    deliver(&app, jobs(&app).await.remove(0)).await;
    assert_eq!(
        server.received().len(),
        2,
        "retry must not disclose cached private fields"
    );
    assert_eq!(
        status(&app).await,
        json!(["suppressed", 2, "access_changed"])
    );
}

#[tokio::test]
async fn mcp_events_cursor_waits_for_matching_ledger_fan_out() {
    for outcome in ["pending", "delivered", "acknowledged"] {
        let (app, server, _) = setup().await;
        let mut p = params("mention");
        p["arguments"] = json!({"roomIds":[ALL_TALK]});
        let result = rpc(&app, "events/subscribe", p.clone()).await;
        assert!(result.get("error").is_none(), "{result}");
        let first = post(&app, "Hey @[Bender Bot]", None, DAVID).await;
        let first = app
            .db()
            .write(move |tx| {
                let id: i64 = tx.conn().query_row(
                    "SELECT id FROM agent_events WHERE agent_id=? AND message_id=?",
                    rusqlite::params![AGENT, first],
                    |r| r.get(0),
                )?;
                tx.conn().execute(
                    "UPDATE agent_events SET outcome=? WHERE id=?",
                    rusqlite::params![outcome, id],
                )?;
                Ok(id)
            })
            .await
            .unwrap();
        let second = post(&app, "Hey @[Bender Bot]", None, DAVID).await;
        let second = publish(&app, second).await;
        deliver(&app, jobs(&app).await.remove(0)).await;
        let received = server.received();
        let body: Value = serde_json::from_slice(&received[1].body).unwrap();
        assert!(
            body["cursor"].as_str().unwrap().parse::<i64>().unwrap() < first,
            "{outcome}: {body}"
        );
        let result = rpc(&app, "events/subscribe", p).await;
        assert!(
            result["result"]["cursor"]
                .as_str()
                .unwrap()
                .parse::<i64>()
                .unwrap()
                < first
        );
        app.db()
            .write(move |tx| {
                if outcome == "delivered" {
                    tx.conn().execute(
                        "UPDATE agent_events SET outcome='acknowledged' WHERE id=?",
                        [first],
                    )?;
                }
                ledger::perform_delivery(tx, first)
            })
            .await
            .unwrap();
        deliver(&app, jobs(&app).await.remove(0)).await;
        let received = server.received();
        let body: Value = serde_json::from_slice(&received[2].body).unwrap();
        assert_eq!(body["cursor"], second.to_string());
    }
}

#[tokio::test]
async fn mcp_events_explicit_replay_lowers_watermark_and_cached_retry_cursor() {
    let (app, server, clock) = setup().await;
    let old = post(&app, "Hey @[Bender Bot]", None, DAVID).await;
    let old = publish(&app, old).await;
    subscribe(&app, "mention").await;
    let live = post(&app, "Hey @[Bender Bot]", None, DAVID).await;
    publish(&app, live).await;
    server.replace_routes(vec![Route::new("POST", "*", "/hook", 503)]);
    let retry = jobs(&app).await.remove(0);
    deliver(&app, retry.clone()).await;
    let first: Value = serde_json::from_slice(&server.received()[1].body).unwrap();
    let mut p = params("mention");
    p["cursor"] = json!("0");
    let replay = rpc(&app, "events/subscribe", p).await;
    assert_eq!(replay["result"]["cursor"], "0");
    clock.advance(jiff::SignedDuration::from_secs(31));
    deliver(
        &app,
        domain::DeliveryJob {
            attempt: 1,
            ..retry
        },
    )
    .await;
    let received = server.received();
    let second: Value = serde_json::from_slice(&received[2].body).unwrap();
    assert!(second["cursor"].as_str().unwrap().parse::<i64>().unwrap() < old);
    assert_eq!(first["data"], second["data"]);
    assert_eq!(first["eventId"], second["eventId"]);
    signed(&received[2], KEY);
    assert_eq!(status(&app).await[0], "pending");
}

#[tokio::test]
async fn mcp_events_expired_delivery_reactivates_on_replay_with_same_id() {
    let (app, server, clock) = setup().await;
    let subscription = subscribe(&app, "mention").await;
    let m = post(&app, "Hey @[Bender Bot]", None, DAVID).await;
    publish(&app, m).await;
    let original: String = app
        .db()
        .read(|c| {
            Ok(
                c.query_row("SELECT webhook_id FROM mcp_event_deliveries", [], |r| {
                    r.get(0)
                })?,
            )
        })
        .await
        .unwrap();
    clock.advance(jiff::SignedDuration::from_hours(2));
    deliver(&app, jobs(&app).await.remove(0)).await;
    assert_eq!(server.received().len(), 1);
    assert_eq!(status(&app).await[0], "expired");
    let mut p = params("mention");
    p["cursor"] = json!("0");
    let refreshed = rpc(&app, "events/subscribe", p).await;
    assert_eq!(refreshed["result"]["id"], subscription);
    assert_eq!(refreshed["result"]["truncated"], false);
    let queued = jobs(&app).await;
    assert_eq!(queued.len(), 1);
    deliver(&app, queued[0].clone()).await;
    assert_eq!(status(&app).await[0], "delivered");
    let received = server.received();
    assert_eq!(received.len(), 2);
    assert_eq!(received[1].header("webhook-id"), Some(original.as_str()));
}

#[tokio::test]
async fn mcp_events_mention_dispatches_through_real_job_runner() {
    let (app, server, _) = setup_with_runner(true).await;
    let subscription = subscribe(&app, "mention").await;
    let message = post(&app, "Hey @[Bender Bot]", None, DAVID).await;
    let result = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            if server.received().len() >= 2 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await;
    // Stop and join the runner even when the observable delivery times out.
    let app = app.without_job_runner().await;
    result.expect("queued mention reaches the receiver through registered job handlers");
    let received = server.received();
    assert_eq!(received.len(), 2);
    signed(&received[1], KEY);
    let body: Value = serde_json::from_slice(&received[1].body).unwrap();
    assert_eq!(body["name"], "mention");
    assert_eq!(body["data"]["message"]["id"], message);
    assert_eq!(
        received[1].header("X-MCP-Subscription-Id"),
        Some(subscription.as_str())
    );
    assert_eq!(status(&app).await[0], "delivered");
}
#[tokio::test]
async fn mcp_events_cursor_ignores_unmatched_and_suppressed_ledger_rows() {
    let (app, server, _) = setup().await;
    let mut p = params("mention");
    p["arguments"] = json!({"roomIds":[ALL_TALK]});
    let result = rpc(&app, "events/subscribe", p).await;
    assert!(result.get("error").is_none(), "{result}");
    for change in ["room", "type", "suppressed"] {
        let m = post(&app, "Hey @[Bender Bot]", None, DAVID).await;
        app.db()
            .write(move |tx| {
                tx.conn().execute(
                    match change {
                        "room" => "UPDATE agent_events SET room_id=NULL WHERE message_id=?",
                        "type" => "UPDATE agent_events SET event_type='reply' WHERE message_id=?",
                        _ => "UPDATE agent_events SET outcome='suppressed' WHERE message_id=?",
                    },
                    [m],
                )?;
                Ok(())
            })
            .await
            .unwrap();
    }
    let m = post(&app, "Hey @[Bender Bot]", None, DAVID).await;
    let event = publish(&app, m).await;
    deliver(&app, jobs(&app).await.remove(0)).await;
    let received = server.received();
    let body: Value = serde_json::from_slice(&received[1].body).unwrap();
    assert_eq!(body["cursor"], event.to_string());
}

#[tokio::test]
async fn mcp_events_reactivated_expiry_rechecks_access_and_suppression_is_permanent() {
    let (app, server, clock) = setup().await;
    subscribe(&app, "mention").await;
    let m = post(&app, "Hey @[Bender Bot]", None, DAVID).await;
    publish(&app, m).await;
    clock.advance(jiff::SignedDuration::from_hours(2));
    deliver(&app, jobs(&app).await.remove(0)).await;
    let mut p = params("mention");
    p["cursor"] = json!("0");
    let refreshed = rpc(&app, "events/subscribe", p.clone()).await;
    assert!(refreshed.get("error").is_none(), "{refreshed}");
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
    deliver(&app, jobs(&app).await.remove(0)).await;
    assert_eq!(status(&app).await[0], "suppressed");
    assert_eq!(server.received().len(), 1);
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE agent_grants SET revoked_at=NULL WHERE agent_id=?",
                [AGENT],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let replay = rpc(&app, "events/subscribe", p).await;
    assert!(replay.get("error").is_none(), "{replay}");
    assert!(jobs(&app).await.is_empty());
    assert_eq!(server.received().len(), 1);
}

#[tokio::test]
async fn mcp_events_broader_replay_can_include_previously_age_skipped_events() {
    let (app, server, clock) = setup().await;
    let m = post(&app, "Hey @[Bender Bot]", None, DAVID).await;
    let event = publish(&app, m).await;
    clock.advance(jiff::SignedDuration::from_mins(1));
    let mut p = params("mention");
    p["cursor"] = json!("0");
    p["maxAgeMs"] = json!(0);
    let first = rpc(&app, "events/subscribe", p.clone()).await;
    assert_eq!(first["result"]["truncated"], true);
    assert!(jobs(&app).await.is_empty());
    p["maxAgeMs"] = json!(86_400_000);
    let second = rpc(&app, "events/subscribe", p).await;
    assert_eq!(second["result"]["truncated"], false);
    assert_eq!(jobs(&app).await.len(), 1);
    deliver(&app, jobs(&app).await.remove(0)).await;
    let received = server.received();
    let body: Value = serde_json::from_slice(&received[1].body).unwrap();
    assert_eq!(body["data"]["id"], event);
}

#[tokio::test]
async fn assistants_owner_wake_policy_filters_mentions_replies_and_dms() {
    use super::presenters::test_support::KEVIN;
    let (app, server, _) = setup().await;
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE agents SET kind='personal',owner_id=?,wake_policy='owner' WHERE id=?",
                rusqlite::params![DAVID, AGENT],
            )?;
            Room::find(tx.conn(), ALL_TALK)?.grant_to(tx, &[KEVIN])?;
            Ok(())
        })
        .await
        .unwrap();
    for name in ["mention", "reply", "direct_message"] {
        subscribe(&app, name).await;
    }
    let bot_message = post(&app, "Ready", None, BENDER).await;
    let mention = post(&app, "@[Bender Bot] teammate", None, KEVIN).await;
    let reply = post(&app, "teammate reply", Some(bot_message), KEVIN).await;
    let dm = app
        .db()
        .write(|tx| {
            let room = Room::find_or_create_direct_for(tx, &[BENDER, DAVID], DAVID)?;
            Ok(Message::create(
                tx,
                NewMessage {
                    room_id: room.id,
                    creator_id: KEVIN,
                    body: Some("Hello".into()),
                    ..Default::default()
                },
            )?
            .id)
        })
        .await
        .unwrap();
    app.db()
        .read(move |conn| {
            for id in [mention, reply, dm] {
                assert_eq!(
                    conn.query_row(
                        "SELECT COUNT(*) FROM agent_events WHERE agent_id=? AND message_id=?",
                        rusqlite::params![AGENT, id],
                        |r| r.get::<_, i64>(0)
                    )?,
                    0
                );
            }
            assert!(
                campfire_db::models::agent_event_access::readable_page(conn, AGENT, 0, None)?
                    .is_empty()
            );
            Ok(())
        })
        .await
        .unwrap();
    assert!(jobs(&app).await.is_empty());
    let owner = post(&app, "@[Bender Bot] owner", None, DAVID).await;
    publish(&app, owner).await;
    let pending = jobs(&app).await;
    assert_eq!(pending.len(), 1);
    deliver(&app, pending[0].clone()).await;
    app.db()
        .read(|conn| {
            assert_eq!(
                campfire_db::models::agent_event_access::readable_page(conn, AGENT, 0, None)?.len(),
                1
            );
            Ok(())
        })
        .await
        .unwrap();
    app.db()
        .write(|tx| {
            tx.conn()
                .execute("UPDATE agents SET wake_policy='anyone' WHERE id=?", [AGENT])?;
            Ok(())
        })
        .await
        .unwrap();
    let teammate = post(&app, "@[Bender Bot] now allowed", None, KEVIN).await;
    publish(&app, teammate).await;
    let pending = jobs(&app).await;
    assert_eq!(pending.len(), 1);
    deliver(&app, pending[0].clone()).await;
    assert_eq!(server.received().len(), 5);
}

#[tokio::test]
async fn assistants_pause_suppresses_queued_deliveries_and_preserves_subscriptions() {
    let (app, server, _) = setup().await;
    let subscription = subscribe(&app, "mention").await;
    let first = post(&app, "@[Bender Bot] queued", None, DAVID).await;
    publish(&app, first).await;
    let queued = jobs(&app).await.pop().unwrap();
    let pending = post(&app, "@[Bender Bot] pending", None, DAVID).await;
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE agents SET paused_at=?,paused_by_id=? WHERE id=?",
                rusqlite::params![tx.now(), DAVID, AGENT],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    deliver(&app, queued).await;
    assert_eq!(status(&app).await[0], "suppressed");
    let paused = post(&app, "@[Bender Bot] paused", None, DAVID).await;
    publish(&app, pending).await;
    publish(&app, paused).await;
    app.db()
        .read(move |conn| {
            for message in [pending, paused] {
                let row = conn.query_row(
                    "SELECT outcome,detail FROM agent_events WHERE agent_id=? AND message_id=?",
                    rusqlite::params![AGENT, message],
                    |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
                )?;
                assert_eq!(row, ("suppressed".into(), "Agent paused".into()));
            }
            assert!(domain::Subscription::find(conn, &subscription)?.is_some());
            Ok(())
        })
        .await
        .unwrap();
    assert!(jobs(&app).await.is_empty());
    assert_eq!(server.received().len(), 1);
    let refusal = rpc(&app, "events/subscribe", params("mention")).await;
    assert_eq!(refusal["error"]["code"], -32024);
    assert_eq!(refusal["error"]["message"], "Agent paused");
    for method in [
        "initialize",
        "server/discover",
        "tools/list",
        "events/list",
        "ping",
    ] {
        let p = if method == "initialize" {
            json!({"protocolVersion":"2026-07-28"})
        } else {
            json!({})
        };
        let reply = rpc(&app, method, p).await;
        assert!(reply.get("error").is_none(), "{reply}");
    }
    let denied = rpc(
        &app,
        "tools/call",
        json!({"name":"post_message","arguments":{"room_id":ALL_TALK,"body":"No"}}),
    )
    .await;
    assert_eq!(denied["result"]["isError"], true);
    assert_eq!(
        denied["result"]["structuredContent"],
        json!({"error":"agent_paused","status":403})
    );
    let denied = app
        .anonymous()
        .send(
            Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/agents/messages"))
                .header("authorization", &format!("Bearer {SECRET}"))
                .json_body(&json!({"room_id":ALL_TALK,"body":"No"})),
        )
        .await;
    assert_eq!(denied.status.as_u16(), 403);
    assert_eq!(denied.json(), json!({"error":"agent_paused"}));
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE agents SET paused_at=NULL,paused_by_id=NULL WHERE id=?",
                [AGENT],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let resumed = post(&app, "@[Bender Bot] resumed", None, DAVID).await;
    publish(&app, resumed).await;
    let pending = jobs(&app).await;
    assert_eq!(pending.len(), 1);
    deliver(&app, pending[0].clone()).await;
    assert_eq!(server.received().len(), 2);
}
