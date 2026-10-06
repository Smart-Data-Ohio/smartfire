use super::*;
use campfire_db::models::agent_delivery as domain;
use crate::controllers::presenters::test_support::TestApp;
use campfire_db::models::{
    agent_delivery::AgentEvent,
    agent_payloads::{self as payloads, Payload, RepositoryAccess},
};
use campfire_db::{ThreadTag, Timestamp};
use rusqlite::params;
use serde_json::{Value, json};

pub(crate) fn install_payload_fixture(tx: &mut campfire_db::Tx<'_>) -> campfire_db::Result<()> {
    let now = Timestamp::from_jiff("2026-03-02T16:00:00Z".parse().unwrap());
    let room = 486777696;
    let actor = 127326141;
    tx.conn().execute("INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,work_owner_id,result_markdown,result_updated_at,run_url,last_activity_at,created_at,updated_at) VALUES(900020001,?,?,'Work <>&','in_progress',394959859,'Done <>&',?,'https://example.test/run',?,?,?)",params![room,actor,now,now,now,now])?;
    ThreadTag::create(tx, 900020001, "zulu")?;
    ThreadTag::create(tx, 900020001, "alpha")?;
    tx.conn().execute("INSERT INTO events(id,room_id,organizer_id,title,starts_at,ends_at,time_zone,created_at,updated_at) VALUES(900030001,?,?,'Review <>&',?,?,'UTC',?,?)",params![room,actor,now.since(jiff::SignedDuration::from_hours(1)),now.since(jiff::SignedDuration::from_hours(2)),now,now])?;
    tx.conn().execute("INSERT INTO github_pull_requests(id,owner,repo,number,title,head_branch,base_branch,state,check_status,review_decision,private,created_at,updated_at) VALUES(900040001,'org','repo',7,'Secret title','secret','main','open','success','APPROVED',1,?,?)",params![now,now])?;
    for (kind, url, title, pr, event) in [
        (
            "drive_file",
            Some("https://drive.google.com/open?id=first"),
            Some("<>& file"),
            None,
            None,
        ),
        ("event", None, None, None, Some(900030001)),
        ("pull_request", None, None, Some(900040001), None),
    ] {
        tx.conn().execute("INSERT INTO work_thread_links(channel_thread_id,kind,url,title,github_pull_request_id,event_id,created_by_id,created_at,updated_at) VALUES(900020001,?,?,?,?,?,?,?,?)",params![kind,url,title,pr,event,actor,now,now])?;
    }
    tx.conn().execute("INSERT INTO agent_approvals(id,agent_id,action,summary,status,decided_by_id,decision_note,expires_at,created_at,updated_at) VALUES(900010001,773018776,'release','Ship it','approved',?,'<>& yes',?,?,?)",params![actor,now.since(jiff::SignedDuration::from_hours(24)),now,now])?;
    tx.conn().execute(
        "UPDATE messages SET thread_id=900020001 WHERE id=136976342",
        [],
    )?;
    tx.conn().execute("INSERT INTO github_pull_request_threads(channel_thread_id,github_pull_request_id,room_id,created_at,updated_at) VALUES(900020001,900040001,?,?,?)",params![room,now,now])?;
    tx.conn().execute("INSERT INTO drive_attachments(message_id,file_id,created_at) VALUES(136976342,'ws11_abc-123',?)",[now])?;
    Ok(())
}

// The app suite calls this against the actual parity seed, whose stored Action Text matches
// the Rails oracle. The fixture-only DB seed has different rendered bodies.
pub fn compare_payload_matrix(tx: &mut campfire_db::Tx<'_>) -> campfire_db::Result<()> {
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agents_event_payload_contract.json"
    ))
    .unwrap();
    install_payload_fixture(tx)?;
    let cases = [
        (
            "slash",
            "slash_command",
            json!({"thread_id":321,"command":"inspect","arguments":"<>& é\u{2028}"}),
            Some(486777696),
            Some(127326141),
            None,
        ),
        (
            "slash_no_actor",
            "slash_command",
            json!([]),
            None,
            None,
            None,
        ),
        (
            "approval",
            "approval_decided",
            Value::Null,
            None,
            None,
            Some(900010001),
        ),
        (
            "approval_metadata",
            "approval_decided",
            json!({"approval_id":900010001}),
            None,
            None,
            None,
        ),
        (
            "github",
            "github_action_completed",
            json!({"approval_id":900010001,"action":"github.comment","status":"completed","url":"https://example.test/?a=1&b=2","message":""}),
            None,
            None,
            None,
        ),
        (
            "fizzy",
            "fizzy_action_completed",
            json!({"approval_id":900010001,"action":"fizzy.close","status":false,"url":null,"message":"é"}),
            None,
            None,
            None,
        ),
        (
            "github_empty",
            "github_action_completed",
            json!([]),
            None,
            None,
            None,
        ),
        (
            "assigned",
            "work_assigned",
            json!({"thread_id":900020001,"assigned_by":"David"}),
            Some(486777696),
            None,
            None,
        ),
        (
            "unassigned",
            "work_unassigned",
            json!({"thread_id":900020001,"assigned_by":null}),
            Some(486777696),
            None,
            None,
        ),
        (
            "handoff",
            "work_handed_off",
            json!({"thread_id":900020001,"assigned_by":"David","handoff":{"summary":"<>&","messages":[]}}),
            Some(486777696),
            None,
            None,
        ),
        (
            "snapshot",
            "work_unassigned",
            json!({"thread_id":0,"work_snapshot":{"id":7,"title":"Old","thread_id":7,"status":"planned","assigned_by":null}}),
            None,
            None,
            None,
        ),
        (
            "snapshot_handoff",
            "work_handed_off",
            json!({"thread_id":0,"work_snapshot":{"id":8},"handoff":{"summary":"<>&"}}),
            None,
            None,
            None,
        ),
    ];
    for (index, (key, kind, metadata, room, actor, approval)) in cases.into_iter().enumerate() {
        let event_id = 900000000 + index as i64;
        tx.conn().execute("INSERT INTO agent_events(id,agent_id,event_type,outcome,metadata,room_id,actor_id,agent_approval_id,created_at) VALUES(?,773018776,?,'delivered',?,?,?,?,?)",params![event_id,kind,metadata,room,actor,approval,tx.now()])?;
        let e = AgentEvent::find(tx.conn(), event_id)?.unwrap();
        let Payload::Ready { body, sync_message } =
            payloads::build(tx.conn(), tx.rich_text(), &e, &RepositoryAccess::default())?
        else {
            panic!("{key}: missing payload");
        };
        assert!(sync_message.is_none());
        assert_eq!(body, vectors["payloads"][key].as_str().unwrap(), "{key}");
    }
    tx.conn().execute("INSERT INTO agent_events(id,agent_id,event_type,message_id,created_at) VALUES(123,773018776,'mention',136976342,?)",[tx.now()])?;
    let e = AgentEvent::find(tx.conn(), 123)?.unwrap();
    let assert_message = |key: &str, access: &RepositoryAccess| -> campfire_db::Result<()> {
        let Payload::Ready { body, sync_message } =
            payloads::build(tx.conn(), tx.rich_text(), &e, access)?
        else {
            panic!("{key}: missing message");
        };
        assert!(sync_message.is_some());
        assert_eq!(body, vectors["payloads"][key].as_str().unwrap(), "{key}");
        Ok(())
    };
    assert_message("message_private", &RepositoryAccess::default())?;
    tx.conn().execute(
        "UPDATE github_pull_requests SET private=0 WHERE id=900040001",
        [],
    )?;
    assert_message("message_public", &RepositoryAccess::default())?;
    let mut work = e.clone();
    work.id = 900000014;
    work.event_type = "work_assigned".into();
    work.message_id = None;
    work.metadata = json!({"thread_id":900020001,"assigned_by":"David"});
    let Payload::Ready { body, .. } = payloads::build(
        tx.conn(),
        tx.rich_text(),
        &work,
        &RepositoryAccess::default(),
    )?
    else {
        panic!("missing public work");
    };
    assert_eq!(
        body,
        vectors["payloads"]["assigned_public"].as_str().unwrap()
    );

    tx.conn().execute(
        "UPDATE github_pull_requests SET private=1 WHERE id=900040001",
        [],
    )?;
    assert_message(
        "message_owner_readable",
        &RepositoryAccess::from([(127326141, "org".into(), "repo".into())]),
    )?;
    // A decision for a different owner must never expose private title or branches.
    assert_message(
        "message_private",
        &RepositoryAccess::from([(0, "org".into(), "repo".into())]),
    )?;
    for (kind, metadata) in [
        ("mention", Value::Null),
        ("approval_decided", Value::Null),
        ("work_assigned", json!({"thread_id":0})),
        ("posted", Value::Null),
    ] {
        let mut missing = e.clone();
        missing.event_type = kind.into();
        missing.message_id = Some(0);
        missing.agent_approval_id = Some(0);
        missing.metadata = metadata;
        let Payload::Unavailable(error) = payloads::build(
            tx.conn(),
            tx.rich_text(),
            &missing,
            &RepositoryAccess::default(),
        )?
        else {
            panic!("{kind}: expected unavailable");
        };
        assert_eq!(
            format!("Agent::Delivery::UndeliverableWebhook: {error}"),
            vectors["missing"][kind].as_str().unwrap()
        );
    }
    Ok(())
}

#[tokio::test]
async fn ws11_agent_event_payloads_match_rails_bytes() {
    let app = TestApp::boot().await.expect("default seed");
    app.db().write(compare_payload_matrix).await.unwrap();
}

#[tokio::test]
async fn ws11_non_message_webhooks_ignore_sync_reply_bodies() {
    use crate::integrations::test_support::{
        FakeResolver, FakeServer, MappingDialer, Route, network,
    };
    use std::{
        collections::HashSet,
        sync::{Arc, Mutex},
    };
    let test = TestApp::boot().await.expect("default seed");
    let app = &test.booted.app;
    let event_id = app
        .db
        .write(|tx| {
            tx.conn().execute(
                "UPDATE webhooks SET url='http://bots.example:8080/hook' WHERE user_id=394959859",
                [],
            )?;
            let e = AgentEvent::create(
                tx,
                domain::NewEvent {
                    agent_id: 773018776,
                    event_type: "fizzy_action_completed".into(),
                    metadata: json!({"action":"fizzy.close","status":"completed"}),
                    ..Default::default()
                },
            )?;
            tx.conn().execute(
                "UPDATE agent_events SET webhook_status='pending' WHERE id=?",
                [e.id],
            )?;
            Ok(e.id)
        })
        .await
        .unwrap();
    let before = app
        .db
        .read(|c| Ok(c.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get::<_, i64>(0))?))
        .await
        .unwrap();
    let mut route = Route::new("POST", "*", "/hook", 200);
    route
        .headers
        .push(("Content-Type".into(), "text/plain".into()));
    route.body = b"Ignored reply".to_vec();
    let server = FakeServer::start(vec![route]).await;
    let net = network(
        Arc::new(FakeResolver::new([("bots.example", vec!["93.184.216.34"])])),
        Arc::new(MappingDialer {
            public: HashSet::from(["93.184.216.34".parse().unwrap()]),
            to: server.addr,
            dialed: Mutex::new(vec![]),
        }),
    );
    post_with_network(
        app,
        domain::EventWebhookJob {
            event_id,
            attempt: Some(0),
        },
        &net,
    )
    .await
    .unwrap();
    assert_eq!(server.received().len(), 1);
    let actual = app
        .db
        .read(move |c| {
            Ok((
                AgentEvent::find(c, event_id)?.unwrap(),
                c.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get::<_, i64>(0))?,
            ))
        })
        .await
        .unwrap();
    assert_eq!(actual.0.webhook_status, "delivered");
    assert_eq!(actual.1, before);
}
