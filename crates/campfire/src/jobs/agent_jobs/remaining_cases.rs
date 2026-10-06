//! Previously deferred queued bot and deleted-work cases. Only DNS/dialing uses a fixture.
use crate::integrations::test_support::{FakeResolver, FakeServer, MappingDialer, Route, network};
use super::*;
use crate::controllers::presenters::test_support::{SEED_NOW, TestApp};
use campfire_db::models::channel_thread::WorkChanges;
use campfire_db::{ChannelThread, NewChannelThread, ThreadMembership};
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};
const ROOM: i64 = 486777696;
const BOT: i64 = 394959859;
const DAVID: i64 = 127326141;
fn gold(name: &str) -> Value {
    let v: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/agents_next2_jobs.json"
    ))
    .unwrap();
    v["results"][name].clone()
}
async fn ledger(app: &App) -> Value {
    app.db.read(|conn|{
    let mut query=conn.prepare("SELECT id,event_type,outcome,actor_id,webhook_status,metadata FROM agent_events ORDER BY id")?;
    let rows=query.query_map([],|r|Ok(json!({"id":r.get::<_,i64>(0)?,"event_type":r.get::<_,String>(1)?,"outcome":r.get::<_,Option<String>>(2)?,"actor_id":r.get::<_,Option<i64>>(3)?,"webhook_status":r.get::<_,String>(4)?,"metadata":r.get::<_,Value>(5)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
    Ok(json!(rows))
}).await.unwrap()
}
async fn case(deleted: bool) {
    let (app, _dir) = TestApp::boot_with_clock(Arc::new(campfire_kit::FrozenClock::new(
        SEED_NOW.parse().unwrap(),
    )))
    .await
    .expect("default seed")
    .stop_jobs()
    .await;
    let crypto = app.ar_encryption.clone();
    app.db.write(move|tx|{
        tx.conn().execute("DELETE FROM agent_events",[])?;tx.conn().execute("DELETE FROM agent_grants",[])?;tx.conn().execute("DELETE FROM background_jobs",[])?;
        Room::find(tx.conn(),ROOM)?.grant_to(tx,&[BOT])?;
        let secret=crypto.encrypt("ws11-next-2-public-signing-material");
        tx.conn().execute("UPDATE webhooks SET url='http://93.184.216.34:8080/hook',signing_secret=? WHERE user_id=?",rusqlite::params![secret,BOT])?;
        tx.conn().execute("UPDATE agents SET webhook_signing_secret=? WHERE user_id=?",rusqlite::params![secret,BOT])?;
        Ok(())
    }).await.unwrap();
    if deleted {
        app.db
            .write(|tx| {
                tx.conn().execute(
                    "UPDATE sqlite_sequence SET seq=1901820000 WHERE name='channel_threads'",
                    [],
                )?;
                let mut thread = ChannelThread::create(
                    tx,
                    NewChannelThread {
                        room_id: ROOM,
                        creator_id: DAVID,
                        name: Some("Agent work".into()),
                        ..Default::default()
                    },
                )?;
                ThreadMembership::join(tx, thread.id, DAVID)?;
                let human = User::find(tx.conn(), DAVID)?;
                thread.update_work(
                    tx,
                    &human,
                    WorkChanges {
                        status: Some(Some("planned".into())),
                        ..Default::default()
                    },
                )?;
                thread.update_work(
                    tx,
                    &human,
                    WorkChanges {
                        owner_id: Some(json!(BOT)),
                        ..Default::default()
                    },
                )?;
                thread.destroy_by(tx, Some(DAVID))
            })
            .await
            .unwrap();
    } else {
        app.db
            .write(|tx| User::find(tx.conn(), BOT)?.deliver_webhook_later(tx, 935961918))
            .await
            .unwrap();
    }
    let before = ledger(&app).await;
    let jobs=app.db.read(move|conn|{
        let class=if deleted{"Agent::EventWebhookJob"}else{"Bot::WebhookJob"};let mut query=conn.prepare("SELECT arguments FROM background_jobs WHERE job_class=? ORDER BY id")?;
        let rows=query.query_map([class],|r|{let a:Value=r.get(0)?;Ok(json!({"class":class,"args":if deleted{json!([a["event_id"],a["attempt"]])}else{json!([format!("gid://campfire/User/{}",a["bot_id"]),format!("gid://campfire/Message/{}",a["message_id"])])}}))})?.collect::<std::result::Result<Vec<_>,_>>()?;Ok(json!(rows))
    }).await.unwrap();
    let server = FakeServer::start(vec![Route::new("POST", "*", "/hook", 200)]).await;
    let net = network(
        Arc::new(FakeResolver::new([])),
        Arc::new(MappingDialer {
            public: HashSet::from(["93.184.216.34".parse().unwrap()]),
            to: server.addr,
            dialed: Mutex::new(vec![]),
        }),
    );
    assert!(server.received().is_empty());
    // The durable runner decodes/claims the committed class and arguments. The wrappers
    // replace only Network::system(), as Rails replaces Net::HTTP.start at the dial boundary.
    let mut registry = Registry::new();
    if deleted {
        registry.register(move |app: App, job: EventWebhook, execution: Execution| {
            let net = net.clone();
            async move {
                post_deferred_with_network(&app, job, execution.id, &net).await?;
                Ok(Outcome::Done)
            }
        });
    } else {
        registry.register(
            move |app: App, job: crate::queue::WebhookJob, _: Execution| {
                let net = net.clone();
                async move { jobs::deliver_webhook_with_network(&app, job, &net).await }
            },
        );
    }
    let runner = campfire_jobs::start(
        app.db.clone(),
        app.jobs.queue.clone(),
        registry,
        app.clone(),
        crate::queue::runner_config(&app.config),
    );
    let watched = if deleted {
        "Agent::EventWebhookJob"
    } else {
        "Bot::WebhookJob"
    };
    crate::test_support::eventually("queued webhook acknowledged", || async {
        app.db
            .read(move |conn| {
                Ok(conn.query_row(
                    "SELECT count(*)=0 FROM background_jobs WHERE job_class=?",
                    [watched],
                    |r| r.get::<_, bool>(0),
                )?)
            })
            .await
            .unwrap()
    })
    .await;
    runner.shutdown(Duration::from_secs(2)).await;
    let requests=server.received().into_iter().map(|r|json!({"body":String::from_utf8(r.body.clone()).unwrap(),"timestamp":r.header("X-Smartfire-Timestamp"),"signature":r.header("X-Smartfire-Signature"),"content_type":r.header("Content-Type")})).collect::<Vec<_>>();
    let actual = if deleted {
        json!({"exists":app.db.read(|conn|Ok(ChannelThread::find_by_id(conn,1901820001)?.is_some())).await.unwrap(),"jobs":jobs,"before":before,"after":ledger(&app).await,"requests":requests})
    } else {
        json!({"jobs":jobs,"requests":requests})
    };
    assert_eq!(
        actual,
        gold(if deleted {
            "delete_owned"
        } else {
            "queued_bot"
        })
    );
    println!(
        "WS11 remaining queued {}: durable claim, actual handler, rows/body/HMAC match Rails",
        if deleted { "deletion" } else { "bot" }
    );
}
#[tokio::test]
async fn ws11_next2_bot_actual_queued_delivery() {
    case(false).await;
}
#[tokio::test]
async fn ws11_next2_assignment_deleted_owner_snapshot_delivery() {
    case(true).await;
}
