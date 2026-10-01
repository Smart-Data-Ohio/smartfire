//! PR #176 regressions use the pinned Rails callbacks and real production adapters.
use super::*;
use crate::controllers::presenters::test_support::{BENDER, DAVID, TestApp};
use crate::integrations::{
    net::Network,
    test_support::{FakeResolver, FakeServer, MappingDialer, Route},
};
use serde_json::{Value, json};
fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/agents_review_fixes_contract.json"
    ))
    .unwrap()
}
#[tokio::test]
async fn ws11_review_stream_github_failure_precedes_quote_sync() {
    use crate::controllers::presenters::test_support::ALL_TALK;
    use campfire_db::{Message, NewMessage};
    let (app, _dir) = TestApp::boot()
        .await
        .expect("default seed")
        .stop_jobs()
        .await;
    let mid = app
        .db
        .write(|tx| {
            let source = Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    body: Some("Quote source".into()),
                    ..Default::default()
                },
            )?;
            let message = Message::create_markdown(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: BENDER,
                    streaming: true,
                    client_message_id: Some("review-stream-reference-order".into()),
                    ..Default::default()
                },
                &format!(
                    "/rooms/{ALL_TALK}/@{} https://github.com/a/b/pull/0",
                    source.id
                ),
            )?;
            Ok(message.id)
        })
        .await
        .unwrap();
    let failed = app
        .db
        .write(move |tx| Message::find(tx.conn(), mid)?.finalize_stream(tx))
        .await;
    assert!(failed.is_err());
    let (streaming, quotes) = app
        .db
        .read(move |conn| {
            Ok((
                Message::find(conn, mid)?.streaming,
                conn.query_row(
                    "SELECT COUNT(*) FROM message_references WHERE message_id=?",
                    [mid],
                    |r| r.get::<_, i64>(0),
                )?,
            ))
        })
        .await
        .unwrap();
    println!("REVIEW stream reference error: streaming={streaming} quote_references={quotes}");
    assert!(!streaming);
    assert_eq!(
        json!(quotes),
        oracle()["results"]["stream_failure"]["quote_references"]
    );
}

#[tokio::test]
async fn ws11_review_repository_batch_order_disconnect_and_public_fields_match_rails() {
    use crate::controllers::presenters::test_support::ALL_TALK;
    use campfire_db::models::agent_delivery::{AgentEvent, EventWebhookJob, NewEvent};
    let mut mismatches = vec![];
    for case in oracle()["results"]["repositories"].as_array().unwrap() {
        let order: Vec<String> = serde_json::from_value(case["order"].clone()).unwrap();
        let disconnect = case["disconnect_index"].as_u64().unwrap() as usize;
        let public = case["public_index"].as_u64().map(|i| i as usize);
        let routes = order
            .iter()
            .enumerate()
            .map(|(i, owner)| {
                Route::new(
                    "GET",
                    "api.github.com",
                    &format!("/repos/{owner}/private"),
                    if i == disconnect { 401 } else { 200 },
                )
                .body("{}")
            })
            .collect();
        let (github, roots) =
            FakeServer::start_named_tls_ws15e(routes, vec!["api.github.com".into()]).await;
        let net = Network {
            resolver: Arc::new(FakeResolver::new([(
                "api.github.com",
                vec!["93.184.216.34"],
            )])),
            dialer: Arc::new(MappingDialer {
                public: ["93.184.216.34".parse().unwrap()].into(),
                to: github.addr,
                dialed: Default::default(),
            }),
            tls: crate::integrations::net::tls_config(roots),
        };
        let (app, _dir) = TestApp::boot_with_github_network(net)
            .await
            .expect("default seed")
            .stop_jobs()
            .await;
        let (agent, thread) = super::tests::setup(&app.db, app.ar_encryption.clone()).await;
        let event=app.db.write(move|tx| {
            for (i,owner) in order.iter().enumerate() {
                tx.conn().execute("UPDATE github_pull_requests SET owner=?,repo='private',private=?,title=?,head_branch='private-head',base_branch='private-base' WHERE id=?",rusqlite::params![owner,public!=Some(i),format!("Private {owner}"),900130001+i as i64])?;
            }
            tx.conn().execute("UPDATE webhooks SET url='http://bots.example:8080/hook' WHERE user_id=?",[BENDER])?;
            let event=AgentEvent::create(tx,NewEvent{agent_id:agent,room_id:Some(ALL_TALK),event_type:"work_assigned".into(),outcome:Some("delivered".into()),metadata:json!({"thread_id":thread}),..Default::default()})?;
            tx.conn().execute("UPDATE agent_events SET webhook_status='pending' WHERE id=?",[event.id])?;
            Ok(event.id)
        }).await.unwrap();
        let hook = FakeServer::start(vec![Route::new("POST", "*", "/hook", 200)]).await;
        let hook_net = crate::integrations::test_support::network(
            Arc::new(FakeResolver::new([("bots.example", vec!["93.184.216.34"])])),
            Arc::new(MappingDialer {
                public: ["93.184.216.34".parse().unwrap()].into(),
                to: hook.addr,
                dialed: Default::default(),
            }),
        );
        crate::integrations::agent_jobs::post_with_network(
            &app,
            EventWebhookJob {
                event_id: event,
                attempt: Some(0),
            },
            &hook_net,
        )
        .await
        .unwrap();
        let received = hook.received();
        assert_eq!(received.len(), 1);
        let payload: Value = serde_json::from_slice(&received[0].body).unwrap();
        let details:Vec<_>=payload["work"]["links"].as_array().unwrap().iter().map(|link|json!({"title":link["pull_request"]["title"],"head_branch":link["pull_request"]["head_branch"],"base_branch":link["pull_request"]["base_branch"]})).collect();
        let paths: Vec<_> = github.received().iter().map(|r| r.target.clone()).collect();
        let reason: Option<String> = app
            .db
            .read(|c| {
                Ok(c.query_row(
                    "SELECT disconnected_reason FROM github_connected_accounts WHERE user_id=?",
                    [DAVID],
                    |r| r.get(0),
                )?)
            })
            .await
            .unwrap();
        if json!(details) != case["details"]
            || json!(paths) != case["paths"]
            || json!(reason) != case["disconnected_reason"]
        {
            mismatches.push(
                json!({"case":case,"actual":{"details":details,"paths":paths,"reason":reason}}),
            );
        }
    }
    assert!(
        mismatches.is_empty(),
        "Rails repository batch mismatches: {}",
        json!(mismatches)
    );
}

#[tokio::test]
async fn ws11_review_real_reference_adapters_run_in_rails_order() {
    use crate::controllers::presenters::test_support::ALL_TALK;
    use campfire_db::callbacks::Phase;
    use campfire_db::{Message, NewMessage};
    let (app, _dir) = TestApp::boot()
        .await
        .expect("default seed")
        .stop_jobs()
        .await;
    let stream=app.db.write(|tx| {
        let source=Message::create(tx,NewMessage{room_id:ALL_TALK,creator_id:DAVID,body:Some("Quote source".into()),..Default::default()})?;
        let stream=Message::create_markdown(tx,NewMessage{room_id:ALL_TALK,creator_id:BENDER,streaming:true,..Default::default()},&format!("/rooms/{ALL_TALK}/@{} https://github.com/ws11/order/pull/42 https://app.fizzy.do/897362094/cards/579 https://x.com/ws11/status/12345 https://example.test/reference-order",source.id))?;
        tx.conn().execute_batch("CREATE TEMP TABLE ws11_reference_order(id INTEGER PRIMARY KEY,phase TEXT NOT NULL)")?;
        for (table,phase) in [("github_pull_request_references","sync_github_pull_request_references"),("fizzy_card_references","sync_fizzy_card_references"),("twitter_post_references","sync_twitter_post_references"),("message_references","sync_message_references"),("link_embed_references","sync_link_embed_references")] {
            tx.conn().execute_batch(&format!("CREATE TEMP TRIGGER ws11_order_{table} AFTER INSERT ON {table} BEGIN INSERT INTO ws11_reference_order(phase) VALUES ('{phase}'); END"))?;
        }
        Ok(stream.id)
    }).await.unwrap();
    app.jobs
        .model_callbacks
        .install(Phase::MessageEventReferences, |tx, _| {
            tx.conn().execute(
                "INSERT INTO ws11_reference_order(phase) VALUES ('sync_event_references')",
                [],
            )?;
            Ok(())
        });
    app.db
        .write(move |tx| Message::find(tx.conn(), stream)?.finalize_stream(tx))
        .await
        .unwrap();
    let order = app
        .db
        .write(|tx| {
            Ok(tx
                .conn()
                .prepare("SELECT phase FROM ws11_reference_order ORDER BY id")?
                .query_map([], |r| r.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?)
        })
        .await
        .unwrap();
    assert_eq!(json!(order), oracle()["results"]["stream_success"]["order"]);
}

#[tokio::test]
async fn ws11_review_stale_batch_is_redacted_by_work_message_and_poll_readers() {
    use crate::controllers::presenters::test_support::ALL_TALK;
    use campfire_db::models::{
        agent_delivery::{AgentEvent, NewEvent},
        agent_event_polling, agent_payloads,
    };
    use campfire_db::{Message, NewMessage};
    let (github, roots) = FakeServer::start_named_tls_ws15e(
        vec![Route::new("GET", "api.github.com", "/repos/mixed/repo", 200).body("{}")],
        vec!["api.github.com".into()],
    )
    .await;
    let net = Network {
        resolver: Arc::new(FakeResolver::new([(
            "api.github.com",
            vec!["93.184.216.34"],
        )])),
        dialer: Arc::new(MappingDialer {
            public: ["93.184.216.34".parse().unwrap()].into(),
            to: github.addr,
            dialed: Default::default(),
        }),
        tls: crate::integrations::net::tls_config(roots),
    };
    let (app, _dir) = TestApp::boot_with_github_network(net)
        .await
        .expect("default seed")
        .stop_jobs()
        .await;
    let (agent, thread) = super::tests::setup(&app.db, app.ar_encryption.clone()).await;
    let (message,events)=app.db.write(move|tx| {
        tx.conn().execute("DELETE FROM agent_grants",[])?;
        campfire_db::Room::find(tx.conn(),ALL_TALK)?.grant_to(tx,&[BENDER])?;
        tx.conn().execute("INSERT INTO github_pull_request_threads(github_pull_request_id,channel_thread_id,room_id,created_at,updated_at) VALUES (900130001,?,?,?,?)",rusqlite::params![thread,ALL_TALK,tx.now(),tx.now()])?;
        let message=Message::create(tx,NewMessage{room_id:ALL_TALK,creator_id:DAVID,thread_id:Some(thread),body:Some("Repository context".into()),..Default::default()})?;
        let mut events=vec![];
        for (kind,mid) in [("work_assigned",None),("mention",Some(message.id))] {
            events.push(AgentEvent::create(tx,NewEvent{agent_id:agent,room_id:Some(ALL_TALK),message_id:mid,event_type:kind.into(),outcome:Some("delivered".into()),metadata:json!({"thread_id":thread}),..Default::default()})?.id);
        }
        Ok((message.id,events))
    }).await.unwrap();
    let work = app
        .agent_repositories
        .resolve_work(&app.db, agent, vec![thread])
        .await
        .unwrap();
    let messages = app
        .agent_repositories
        .resolve_messages(&app.db, agent, vec![thread])
        .await
        .unwrap();
    let polling = app
        .agent_repositories
        .resolve_events(&app.db, agent, events.clone())
        .await
        .unwrap();
    assert!(!work.is_empty() && !messages.is_empty() && !polling.is_empty());
    app.db.write(|tx|{tx.conn().execute("UPDATE github_connected_accounts SET disconnected_reason='GitHub rejected the linked token (401)' WHERE user_id=?",[DAVID])?;Ok(())}).await.unwrap();
    app.db
        .read(move |c| {
            let thread = campfire_db::ChannelThread::find(c, thread)?;
            let links =
                agent_payloads::work_payload(c, &thread, Some(DAVID), &work)?["links"].clone();
            for i in 0..2 {
                assert!(links[i]["title"].is_null());
            }
            assert_eq!(links[2]["title"], "Private title");
            let pr = agent_payloads::pull_request_for_message(
                c,
                &Message::find(c, message)?,
                Some(DAVID),
                &messages,
            )?;
            assert!(pr["title"].is_null());
            let polled = agent_event_polling::poll(
                c,
                agent,
                Some(&json!(events[0] - 1)),
                None,
                campfire_db::Timestamp::parse_db("2026-03-02 16:00:00").unwrap(),
                &polling,
                |m| Ok(json!({"id":m.id})),
            )?;
            assert_eq!(polled["events"].as_array().unwrap().len(), 2);
            assert!(polled["events"][0]["work"]["links"][0]["title"].is_null());
            assert!(polled["events"][1]["pull_request"]["title"].is_null());
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn ws11_review_deliverable_deletion_keeps_commit_on_ledger_failure() {
    use crate::controllers::presenters::test_support::ALL_TALK;
    use campfire_db::models::agent_work_events::{DeletedWorkWebhookJob, publish_deleted_webhook};
    use campfire_db::{ChannelThread, NewChannelThread};
    let (app, _dir) = TestApp::boot()
        .await
        .expect("default seed")
        .stop_jobs()
        .await;
    let thread=app.db.write(|tx| {
        tx.conn().execute("DELETE FROM agent_grants",[])?;
        campfire_db::Room::find(tx.conn(),ALL_TALK)?.grant_to(tx,&[BENDER])?;
        let thread=ChannelThread::create(tx,NewChannelThread{room_id:ALL_TALK,creator_id:DAVID,name:Some("Deliverable deletion".into()),..Default::default()})?;
        tx.conn().execute("UPDATE channel_threads SET work_owner_id=? WHERE id=?",rusqlite::params![BENDER,thread.id])?;
        tx.conn().execute_batch("CREATE TRIGGER ws11_reject_deletion_ledger BEFORE INSERT ON agent_events WHEN NEW.event_type='work_unassigned' BEGIN SELECT RAISE(ABORT,'rejected deletion ledger'); END")?;
        Ok(thread.id)
    }).await.unwrap();
    assert!(
        app.db
            .write(move |tx| ChannelThread::find(tx.conn(), thread)?.destroy(tx))
            .await
            .is_err()
    );
    let job=app.db.read(move|c| {
        let actual=json!({"thread_exists":ChannelThread::find_by_id(c,thread)?.is_some(),"events":c.query_row("SELECT COUNT(*) FROM agent_events WHERE event_type='work_unassigned' AND json_extract(metadata,'$.thread_id')=?",[thread],|r|r.get::<_,i64>(0))?});
        let expected=&oracle()["results"]["deletion_with_webhook"];
        assert_eq!(actual["thread_exists"],expected["thread_exists"]);
        assert_eq!(actual["events"],expected["events"]);
        let mut q=c.prepare("SELECT arguments FROM background_jobs WHERE job_class='Agent::EventWebhookJob'")?;
        let jobs=q.query_map([],|r|r.get::<_,Value>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        assert_eq!(jobs.len(),1,"job is persisted with deletion, never in its failed ledger callback");
        assert!(jobs[0].get("event_id").is_none(),"no identity is allocated before publication");
        assert_eq!(jobs[0]["deleted_work"]["metadata"]["thread_id"],thread);
        Ok(serde_json::from_value::<DeletedWorkWebhookJob>(jobs[0].clone()).unwrap())
    }).await.unwrap();
    assert!(
        app.db
            .write(move |tx| publish_deleted_webhook(tx,job))
            .await
            .is_err(),
        "publication cannot bypass the failing ledger insert"
    );
}

#[tokio::test]
async fn ws11_review_poll_batch_does_not_reuse_access_after_disconnect() {
    use crate::controllers::presenters::test_support::ALL_TALK;
    use campfire_db::models::{
        agent_delivery::{AgentEvent, NewEvent},
        agent_event_polling,
    };
    use campfire_db::{ChannelThread, NewChannelThread};
    let (github, roots) = FakeServer::start_named_tls_ws15e(
        vec![
            Route::new("GET", "api.github.com", "/repos/a/private", 200).body("{}"),
            Route::new("GET", "api.github.com", "/repos/z/private", 401).body("{}"),
        ],
        vec!["api.github.com".into()],
    )
    .await;
    let net = Network {
        resolver: Arc::new(FakeResolver::new([(
            "api.github.com",
            vec!["93.184.216.34"],
        )])),
        dialer: Arc::new(MappingDialer {
            public: ["93.184.216.34".parse().unwrap()].into(),
            to: github.addr,
            dialed: Default::default(),
        }),
        tls: crate::integrations::net::tls_config(roots),
    };
    let (app, _dir) = TestApp::boot_with_github_network(net)
        .await
        .expect("default seed")
        .stop_jobs()
        .await;
    let (agent, a) = super::tests::setup(&app.db, app.ar_encryption.clone()).await;
    let events=app.db.write(move|tx| {
        tx.conn().execute("DELETE FROM agent_grants",[])?;
        tx.conn().execute("DELETE FROM agent_events",[])?;
        campfire_db::Room::find(tx.conn(),ALL_TALK)?.grant_to(tx,&[BENDER])?;
        tx.conn().execute("UPDATE github_pull_requests SET owner='a',repo='private',title='Private a',head_branch='private-head',base_branch='private-base' WHERE id BETWEEN 900130001 AND 900130003",[])?;
        let z=ChannelThread::create(tx,NewChannelThread{room_id:ALL_TALK,creator_id:DAVID,name:Some("Polling z".into()),..Default::default()})?;
        tx.conn().execute("INSERT INTO github_pull_requests(id,owner,repo,number,title,head_branch,base_branch,private,created_at,updated_at) VALUES (900130004,'z','private',1,'Private z','private-head','private-base',1,?,?)",rusqlite::params![tx.now(),tx.now()])?;
        tx.conn().execute("INSERT INTO work_thread_links(channel_thread_id,kind,github_pull_request_id,created_by_id,created_at,updated_at) VALUES (?,'pull_request',900130004,?,?,?)",rusqlite::params![z.id,DAVID,tx.now(),tx.now()])?;
        let mut events=vec![];
        for thread in [a,z.id,a] {events.push(AgentEvent::create(tx,NewEvent{agent_id:agent,room_id:Some(ALL_TALK),event_type:"work_assigned".into(),outcome:Some("delivered".into()),metadata:json!({"thread_id":thread}),..Default::default()})?.id);}
        Ok(events)
    }).await.unwrap();
    let access = app
        .agent_repositories
        .resolve_events(&app.db, agent, events.clone())
        .await
        .unwrap();
    let actual=app.db.read(move|c| {
        let payload=agent_event_polling::poll(c,agent,Some(&json!(events[0]-1)),None,campfire_db::Timestamp::parse_db("2026-03-02 16:00:00").unwrap(),&access,|m|Ok(json!({"id":m.id})))?;
        let details:Vec<_>=payload["events"].as_array().unwrap().iter().map(|event| {let pr=&event["work"]["links"][0]["pull_request"];json!({"title":pr["title"],"head_branch":pr["head_branch"],"base_branch":pr["base_branch"]})}).collect();
        Ok(json!({"details":details,"cursor_advanced":payload["next_since"]==json!(events[2])}))
    }).await.unwrap();
    let mut expected = oracle()["results"]["poll_batch"].clone();
    let expected_paths = expected.as_object_mut().unwrap().remove("paths").unwrap();
    assert_eq!(actual, expected);
    assert_eq!(
        json!(
            github
                .received()
                .iter()
                .map(|r| r.target.clone())
                .collect::<Vec<_>>()
        ),
        expected_paths
    );
}
