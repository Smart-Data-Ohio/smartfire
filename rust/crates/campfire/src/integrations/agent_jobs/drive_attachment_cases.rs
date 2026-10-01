//! WS14g consumes WS11's real delivery/poll/presenter APIs. Google metadata never enters payloads.
use super::super::test_support::{FakeResolver, FakeServer, MappingDialer, Route, network};
use super::*;
use crate::controllers::presenters::{
    Presenter,
    test_support::{BENDER, DAVID, TestApp},
};
use campfire_db::{Agent, NewMessage};
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};
async fn exercise(name: &str) {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/google_agent_delivery.json"
    ))
    .unwrap();
    let row = oracle["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == name)
        .unwrap();
    let clock = Arc::new(campfire_kit::FrozenClock::new(
        "2026-03-02T16:00:00Z".parse().unwrap(),
    ));
    let (app, _dir) = TestApp::boot_with_clock(clock)
        .await
        .unwrap()
        .stop_jobs()
        .await;
    let ids = row["ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();
    let (aid, mid, eid) = app
        .db
        .write(move |tx| {
            tx.conn()
                .execute_batch("DELETE FROM agent_events;DELETE FROM agent_grants;")?;
            let agent = Agent::for_user(tx.conn(), BENDER)?.unwrap();
            Room::find(tx.conn(), 486777696)?.grant_to(tx, &[BENDER])?;
            tx.conn().execute(
                "UPDATE webhooks SET url='http://bots.example:8080/hook' WHERE user_id=?",
                [BENDER],
            )?;
            let message = Message::create(
                tx,
                NewMessage {
                    room_id: 486777696,
                    creator_id: DAVID,
                    markdown_source: Some("Hey @[Bender Bot], see these".into()),
                    client_message_id: Some("ws14g-drive-delivery".into()),
                    drive_file_ids: ids,
                    ..Default::default()
                },
            )?;
            let event = tx.conn().query_row(
                "SELECT id FROM agent_events WHERE message_id=? AND event_type='mention'",
                [message.id],
                |r| r.get::<_, i64>(0),
            )?;
            Ok((agent.id, message.id, event))
        })
        .await
        .unwrap();
    let result = if name.starts_with("poll") {
        let app2 = app.clone();
        let page = app
            .db
            .read(move |conn| {
                let mut p = Presenter::new(conn, &app2, None);
                p.current_user_id = Some(BENDER);
                p.cache_base_url = Some("http://campfire.test".into());
                p.request_host = Some("campfire.test".into());
                campfire_db::models::agent_event_polling::poll(
                    conn,
                    aid,
                    None,
                    None,
                    app2.db.env().now(),
                    &Default::default(),
                    |message| p.agent_message_payload(message),
                )
            })
            .await
            .unwrap();
        let event = page["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["message"]["id"] == mid)
            .expect("real poll omitted the produced message");
        json!({"attachments":event["message"]["drive_attachments"]})
    } else {
        deliver(
            app.clone(),
            Delivery(domain::DeliveryJob { event_id: eid }),
            Execution {
                id: 0,
                executions: 1,
                enqueued_at: app.db.env().now(),
                scheduled_at: app.db.env().now(),
            },
        )
        .await
        .unwrap();
        let server = FakeServer::start(vec![Route::new("POST", "*", "/hook", 200).body(b"")]).await;
        let net = network(
            Arc::new(FakeResolver::new([("bots.example", vec!["93.184.216.34"])])),
            Arc::new(MappingDialer {
                public: HashSet::from(["93.184.216.34".parse().unwrap()]),
                to: server.addr,
                dialed: Mutex::new(vec![]),
            }),
        );
        post_with_network(
            &app,
            domain::EventWebhookJob {
                event_id: eid,
                attempt: Some(0),
            },
            &net,
        )
        .await
        .unwrap();
        let posts = server.received();
        assert_eq!(posts.len(), 1);
        let body: Value = serde_json::from_slice(&posts[0].body).unwrap();
        let delivered = app
            .db
            .read(move |c| Ok(AgentEvent::find(c, eid)?.unwrap().webhook_status == "delivered"))
            .await
            .unwrap();
        json!({"attachments":body["message"]["drive_attachments"],"agent_id":body["agent"]["id"],"delivered":delivered})
    };
    assert_eq!(result, row["result"], "pinned Rails {name}");
    for attachment in result["attachments"].as_array().unwrap() {
        assert_eq!(
            attachment.as_object().unwrap().len(),
            2,
            "metadata leaked to agent"
        );
    }
}
#[tokio::test]
async fn ws14g_agent_polling_carries_drive_file_ids_and_urls_only() {
    exercise("poll_files").await;
}
#[tokio::test]
async fn ws14g_agent_polling_carries_an_empty_drive_array() {
    exercise("poll_empty").await;
}
#[tokio::test]
async fn ws14g_agent_webhook_posts_drive_files_without_names() {
    exercise("webhook_files").await;
}
