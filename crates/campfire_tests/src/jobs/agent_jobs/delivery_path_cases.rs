//! Pinned DeliveryJobTest and DeliveryConcurrencyTest through real app writes/HTTP.
use crate::integrations::test_support::{FakeResolver, FakeServer, MappingDialer, Route, network};
use super::*;
use campfire_channels::jobs::integrations as jobs;
use campfire_db::Message;
use campfire_db::Room;
use campfire_db::User;
use campfire_db::models::agent_delivery as domain;
use campfire_db::models::agent_delivery::AgentEvent;
use campfire_jobs::Execution;
use crate::app::App;
use crate::net::Network;
use crate::controllers::presenters::test_support::{ALL_TALK, BENDER, DAVID, SEED_NOW, TestApp};
use campfire_db::{Agent, NewMessage};
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

async fn setup() -> (App, tempfile::TempDir) {
    let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
        SEED_NOW.parse().unwrap(),
    ));
    let (app, dir) = TestApp::boot_with_clock(clock)
        .await
        .expect("default seed")
        .stop_jobs()
        .await;
    app.db
        .write(|tx| {
            tx.conn().execute("DELETE FROM agent_events", [])?;
            tx.conn().execute("DELETE FROM agent_grants", [])?;
            Room::find(tx.conn(), ALL_TALK)?.grant_to(tx, &[BENDER])?;
            tx.conn().execute(
                "UPDATE webhooks SET url='http://bots.example:8080/hook' WHERE user_id=?",
                [BENDER],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    (app, dir)
}
async fn mention(app: &App) -> AgentEvent {
    app.db
        .write(|tx| {
            let m = Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    markdown_source: Some("Hey @[Bender Bot]".into()),
                    ..Default::default()
                },
            )?;
            let id = tx.conn().query_row(
                "SELECT id FROM agent_events WHERE message_id=? AND event_type='mention'",
                [m.id],
                |r| r.get(0),
            )?;
            Ok(AgentEvent::find(tx.conn(), id)?.unwrap())
        })
        .await
        .unwrap()
}
async fn endpoint(body: &str) -> (FakeServer, Network) {
    let s = FakeServer::start(vec![
        Route::new("POST", "*", "/hook", 200)
            .header("Content-Type", "text/plain")
            .body(body.as_bytes()),
    ])
    .await;
    let n = network(
        Arc::new(FakeResolver::new([("bots.example", vec!["93.184.216.34"])])),
        Arc::new(MappingDialer {
            public: HashSet::from(["93.184.216.34".parse().unwrap()]),
            to: s.addr,
            dialed: Mutex::new(vec![]),
        }),
    );
    (s, n)
}
async fn run(app: &App, event: i64, net: &Network) {
    deliver(
        app.clone(),
        Delivery(domain::DeliveryJob { event_id: event }),
        Execution {
            id: 0,
            executions: 1,
            enqueued_at: app.db.env().now(),
            scheduled_at: app.db.env().now(),
        },
    )
    .await
    .unwrap();
    post_with_network(
        app,
        domain::EventWebhookJob {
            event_id: event,
            attempt: Some(0),
        },
        net,
    )
    .await
    .unwrap();
}
#[tokio::test]
async fn ws11_delivery_path_additive_agent_key() {
    let (app, _dir) = setup().await;
    let e = mention(&app).await;
    let (s, n) = endpoint("").await;
    run(&app, e.id, &n).await;
    let requests = s.received();
    assert_eq!(requests.len(), 1);
    let p: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(
        p["agent"],
        json!({"id":e.agent_id,"name":"Bender Bot","owner":"David","delivery_id":e.id})
    );
    assert_eq!(p["message"]["id"], e.message_id.unwrap());
    assert!(!p.as_object().unwrap().contains_key("reply_url"));
    app.db
        .read(move |c| {
            assert_eq!(
                AgentEvent::find(c, e.id)?.unwrap().outcome.as_deref(),
                Some("delivered")
            );
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn ws11_delivery_path_same_event_posts_once() {
    let (app, _dir) = setup().await;
    let e = mention(&app).await;
    let (s, n) = endpoint("").await;
    run(&app, e.id, &n).await;
    run(&app, e.id, &n).await;
    assert_eq!(s.received().len(), 1);
    app.db
        .read(move |c| {
            let e = AgentEvent::find(c, e.id)?.unwrap();
            assert_eq!(e.outcome.as_deref(), Some("delivered"));
            assert_eq!(e.webhook_status, "delivered");
            assert_eq!(e.webhook_attempts, 1);
            Ok(())
        })
        .await
        .unwrap();
}
async fn loses_claim(suppression: bool) {
    let (app, _dir) = setup().await;
    let e = mention(&app).await;
    let id = e.id;
    app.db.write(move|tx| {
        if suppression {
            // Match Rails' intervening winner while the rate decision is in flight.
            for _ in 0..20 {AgentEvent::create(tx,domain::NewEvent {agent_id:e.agent_id,room_id:e.room_id,event_type:"mention".into(),outcome:Some("delivered".into()),..Default::default()})?;}
        }
        let winner=if suppression {"suppressed"} else {"delivered"};
        tx.conn().execute_batch(&format!("CREATE TEMP TRIGGER ws11_claim_winner BEFORE UPDATE OF outcome ON agent_events WHEN OLD.id={id} AND OLD.outcome='pending' BEGIN UPDATE agent_events SET outcome='{winner}' WHERE id=OLD.id; SELECT RAISE(IGNORE); END"))?;
        Ok(())
    }).await.unwrap();
    let (server, net) = endpoint("").await;
    run(&app, id, &net).await;
    assert!(server.received().is_empty());
    app.db.read(move|c| {
        assert_eq!(c.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Agent::EventWebhookJob' AND json_extract(arguments,'$.event_id')=?",[id],|r|r.get::<_,i64>(0))?,0);
        assert_eq!(c.query_row("SELECT COUNT(*) FROM agent_events WHERE event_type='delivery_suppressed_rate_limit'",[],|r|r.get::<_,i64>(0))?,0);Ok(())
    }).await.unwrap();
}
#[tokio::test]
async fn ws11_delivery_path_lost_delivery_claim_posts_nothing() {
    loses_claim(false).await;
}
#[tokio::test]
async fn ws11_delivery_path_lost_suppression_claim_inserts_nothing() {
    loses_claim(true).await;
}
#[tokio::test]
async fn ws11_delivery_path_agent_legacy_ping_pong_stops_at_three() {
    let (app, _dir) = setup().await;
    let legacy = app
        .db
        .write(|tx| {
            let bot = User::create_bot(tx, "Legacy Loop", Some("http://bots.example:8080/hook"))?;
            Room::find(tx.conn(), ALL_TALK)?.grant_to(tx, &[bot.id])?;
            Ok(bot.id)
        })
        .await
        .unwrap();
    let (server, net) = endpoint("Hey @[Bender Bot]").await;
    for text in ["Hey @[Legacy Loop]", "Hey @[Legacy Loop] again"] {
        let text = text.to_owned();
        let mid = app
            .db
            .write(move |tx| {
                let m = Message::create(
                    tx,
                    NewMessage {
                        room_id: ALL_TALK,
                        creator_id: BENDER,
                        markdown_source: Some(text),
                        ..Default::default()
                    },
                )?;
                Ok(m.id)
            })
            .await
            .unwrap();
        jobs::deliver_webhook_with_network(
            &app,
            crate::queue::WebhookJob {
                bot_id: legacy,
                message_id: mid,
            },
            &net,
        )
        .await
        .unwrap();
    }
    assert_eq!(server.received().len(), 2);
    app.db
        .read(|c| {
            let aid = Agent::for_user(c, BENDER)?.unwrap().id;
            let rows = AgentEvent::for_agent(c, aid)?;
            let delivered: Vec<_> = rows
                .iter()
                .filter(|e| domain::MESSAGE_TYPES.contains(&e.event_type.as_str()))
                .collect();
            assert_eq!(delivered.len(), 1);
            assert_eq!(delivered[0].hop(), 1);
            let suppressed = rows
                .iter()
                .find(|e| e.event_type == "delivery_suppressed_hop_limit")
                .unwrap();
            assert_eq!(suppressed.hop(), 3);
            assert_eq!(suppressed.outcome.as_deref(), Some("suppressed"));
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn ws11_delivery_path_concurrent_mentions_obey_rate_limit() {
    let (app, _dir) = setup().await;
    let barrier = Arc::new(tokio::sync::Barrier::new(4));
    let mut workers = Vec::new();
    for worker in 0..4 {
        let db = app.db.clone();
        let barrier = barrier.clone();
        workers.push(tokio::spawn(async move {
            barrier.wait().await;
            for i in 0..7 {
                db.write(move |tx| {
                    Message::create(
                        tx,
                        NewMessage {
                            room_id: ALL_TALK,
                            creator_id: DAVID,
                            markdown_source: Some(format!("Race {worker}-{i} @[Bender Bot]")),
                            client_message_id: Some(format!("ws11-rate-race-{worker}-{i}")),
                            ..Default::default()
                        },
                    )?;
                    Ok(())
                })
                .await
                .unwrap();
            }
        }));
    }
    for worker in workers {
        worker.await.unwrap();
    }
    app.db
        .read(|c| {
            let aid = Agent::for_user(c, BENDER)?.unwrap().id;
            let rows = AgentEvent::for_agent(c, aid)?;
            assert_eq!(rows.len(), 28);
            let deliverable = rows
                .iter()
                .filter(|e| domain::MESSAGE_TYPES.contains(&e.event_type.as_str()))
                .count();
            assert!(deliverable <= 20);
            assert_eq!(
                rows.iter()
                    .filter(|e| e.event_type == "delivery_suppressed_rate_limit")
                    .count(),
                28 - deliverable
            );
            assert_eq!(
                c.query_row(
                    "SELECT COUNT(*) FROM background_jobs WHERE job_class='Agent::DeliveryJob'",
                    [],
                    |r| r.get::<_, usize>(0)
                )?,
                deliverable
            );
            Ok(())
        })
        .await
        .unwrap();
}
