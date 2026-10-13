use super::agent_http_tests::{AGENT, SECRET, setup};
use super::presenters::test_support::{
    ALL_TALK, BENDER, BENDER_KEY, DAVID, JASON, Reply, Req, TestApp,
};
use campfire_db::models::{
    agent_delivery::{self as delivery, AgentEvent, NewEvent},
    agent_event_access as events,
};
use campfire_db::{AgentGrant, ChannelThread, NewChannelThread, NewGrant, Room};
use campfire_kit::Method;
use rusqlite::params;
use serde_json::{Value, json};
const DESIGNERS: i64 = 654632876;

async fn personal() -> TestApp {
    let a = setup().await.without_job_runner().await;
    a.db()
        .write(|tx| {
            tx.conn()
                .execute("DELETE FROM agent_events WHERE agent_id=?", [AGENT])?;
            tx.conn().execute(
                "UPDATE agents SET kind='personal',owner_id=?,daily_message_cap=NULL WHERE id=?",
                params![DAVID, AGENT],
            )?;
            tx.conn().execute(
                "UPDATE users SET bot_token_digest=NULL WHERE id=?",
                [BENDER],
            )?;
            tx.conn()
                .execute("DELETE FROM webhooks WHERE user_id=?", [BENDER])?;
            for room in [ALL_TALK, DESIGNERS] {
                Room::find(tx.conn(), room)?.grant_to(tx, &[BENDER, DAVID])?;
                for cap in ["read_messages", "post_messages", "react", "manage_threads"] {
                    AgentGrant::create(
                        tx,
                        NewGrant {
                            agent_id: AGENT,
                            room_id: Some(room),
                            capability: cap.into(),
                            granted_by_id: DAVID,
                            ..Default::default()
                        },
                    )?;
                }
            }
            Ok(())
        })
        .await
        .unwrap();
    a
}
async fn request(a: &TestApp, method: Method, path: &str, body: Value) -> Reply {
    a.anonymous()
        .send(
            Req::new(method, path)
                .header("authorization", &["Bearer", SECRET].join(" "))
                .header("accept", "application/json")
                .json_body(&body),
        )
        .await
}
async fn rpc(a: &TestApp, method: &str, params: Value) -> Value {
    let r = request(
        a,
        Method::POST,
        "/agents/mcp",
        json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}),
    )
    .await;
    assert_eq!(r.status.as_u16(), 200, "{}", r.text());
    serde_json::from_slice(&r.body).unwrap()
}
async fn tool(a: &TestApp, name: &str, args: Value) -> Value {
    rpc(a, "tools/call", json!({"name":name,"arguments":args})).await
}

#[tokio::test]
async fn personal_agent_mcp_inventory_and_every_denied_tool() {
    let a = personal().await;
    let listed = rpc(&a, "tools/list", json!({})).await;
    let names = listed["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(
        names,
        campfire_controllers::controllers::agents::mcp::PERSONAL_TOOLS
            .into_iter()
            .collect()
    );
    for denied in campfire_controllers::controllers::agents::mcp::PERSONAL_DENIED_TOOLS {
        let response = tool(&a, denied, json!({})).await;
        assert_eq!(response["result"]["isError"], true, "{denied}: {response}");
        assert_eq!(
            response["result"]["structuredContent"]["status"], "forbidden",
            "{denied}: {response}"
        );
    }
}

#[tokio::test]
async fn personal_agent_room_departure_filters_rooms_and_chat_mutations() {
    let a = personal().await;
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "DELETE FROM memberships WHERE user_id=? AND room_id=?",
                params![DAVID, ALL_TALK],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let rooms = tool(&a, "list_rooms", json!({})).await;
    let rooms = rooms["result"]["structuredContent"].as_array().unwrap();
    assert!(!rooms.iter().any(|r| r["id"] == ALL_TALK));
    assert!(rooms.iter().any(|r| r["id"] == DESIGNERS));
    for (name, args) in [
        ("read_messages", json!({"room_id":ALL_TALK})),
        ("post_message", json!({"room_id":ALL_TALK,"body":"denied"})),
        ("react", json!({"message_id":136976342,"content":"👍"})),
    ] {
        let r = tool(&a, name, args).await;
        assert_eq!(r["result"]["isError"], true, "{name}: {r}");
    }
}

#[tokio::test]
async fn personal_agent_private_thread_removal_hides_mcp_context_work_events_and_media() {
    let a = personal().await;
    let mut owner = a.sign_in(DAVID).await;
    let uploaded=owner.write(Req::new(Method::POST,"/api/v1/uploads").json_body(&json!({"filename":"personal.txt","contentType":"text/plain","byteSize":4,"checksum":campfire_storage::key::checksum(b"test")}))).await;
    assert_eq!(uploaded.status.as_u16(), 201, "{}", uploaded.text());
    let upload: campfire_api_types::DirectUpload = serde_json::from_slice(&uploaded.body).unwrap();
    assert_eq!(
        owner
            .send(
                Req::new(Method::PUT, &upload.upload_url)
                    .header("content-type", "text/plain")
                    .header("content-length", "4")
                    .body(b"test".as_slice())
            )
            .await
            .status
            .as_u16(),
        204
    );
    let blob = campfire_storage::paths::verify_signed_blob_id(
        &*a.booted.app.storage.verifier,
        &upload.signed_id,
        a.booted.app.clock.now(),
    )
    .unwrap();
    let (thread, message) = a
        .db()
        .write(move |tx| {
            let thread = ChannelThread::create(
                tx,
                NewChannelThread {
                    room_id: DESIGNERS,
                    creator_id: JASON,
                    name: Some("Owner ceiling".into()),
                    visibility: campfire_db::channel_thread::ThreadVisibility::Private,
                    ..Default::default()
                },
            )?;
            thread.add_access_member(tx, JASON, BENDER)?;
            thread.add_access_member(tx, JASON, DAVID)?;
            tx.conn().execute(
                "UPDATE channel_threads SET work_status='in_progress',work_owner_id=? WHERE id=?",
                params![BENDER, thread.id],
            )?;
            let message = campfire_db::Message::create(
                tx,
                campfire_db::NewMessage {
                    room_id: DESIGNERS,
                    thread_id: Some(thread.id),
                    creator_id: JASON,
                    markdown_source: Some("Private work".into()),
                    attachment_blob_id: Some(blob),
                    ..Default::default()
                },
            )?;
            Ok((thread.id, message.id))
        })
        .await
        .unwrap();
    let record = a
        .db()
        .read(move |conn| {
            campfire_storage::Blob::find(conn, blob)
                .map_err(|e| campfire_db::Error::Other(e.to_string()))
        })
        .await
        .unwrap()
        .unwrap();
    let path =
        campfire_storage::paths::blob_proxy_path(&*a.booted.app.storage.verifier, &record, None);
    assert_eq!(
        request(&a, Method::GET, &path, json!({}))
            .await
            .status
            .as_u16(),
        200
    );
    let read = tool(&a, "read_messages", json!({"thread_id":thread})).await;
    assert_eq!(read["result"]["isError"], false, "{read}");
    let r = request(
        &a,
        Method::GET,
        &format!("/agents/work/{thread}"),
        json!({}),
    )
    .await;
    assert_eq!(r.status.as_u16(), 200, "{}", r.text());
    a.db()
        .write(move |tx| {
            tx.conn().execute(
                "DELETE FROM thread_access_memberships WHERE thread_id=? AND user_id=?",
                params![thread, DAVID],
            )?;
            tx.conn()
                .execute("UPDATE users SET role=0 WHERE id=?", [DAVID])?;
            Ok(())
        })
        .await
        .unwrap();
    let read = tool(&a, "read_messages", json!({"thread_id":thread})).await;
    assert_eq!(
        read["result"]["structuredContent"]["status"], "not_found",
        "{read}"
    );
    let context = tool(
        &a,
        "get_context",
        json!({"room_id":DESIGNERS,"thread_id":thread}),
    )
    .await;
    assert_eq!(context["result"]["isError"], true, "{context}");
    assert_eq!(
        context["result"]["structuredContent"]["status"], "forbidden",
        "{context}"
    );
    let r = request(
        &a,
        Method::GET,
        &format!("/agents/work/{thread}"),
        json!({}),
    )
    .await;
    assert_eq!(r.status.as_u16(), 404, "{}", r.text());
    assert_eq!(
        request(&a, Method::GET, &path, json!({}))
            .await
            .status
            .as_u16(),
        404
    );
    for (name, args) in [
        (
            "post_message",
            json!({"room_id":DESIGNERS,"thread_id":thread,"body":"denied"}),
        ),
        ("react", json!({"message_id":message,"content":"👍"})),
        ("update_work", json!({"work_id":thread,"status":"done"})),
    ] {
        assert_eq!(
            tool(&a, name, args).await["result"]["isError"],
            true,
            "{name}"
        );
    }
    let listed = tool(&a, "list_work", json!({})).await;
    assert!(!listed.to_string().contains("Owner ceiling"));
}

#[tokio::test]
async fn personal_agent_owner_dm_and_spa_directory_rejections() {
    let a = personal().await;
    a.db()
        .write(|tx| {
            tx.conn()
                .execute("DELETE FROM agent_grants WHERE agent_id=?", [AGENT])?;
            Ok(())
        })
        .await
        .unwrap();
    for body in ["First message", "Follow-up"] {
        let r = tool(&a, "open_dm", json!({"user_id":DAVID,"body":body})).await;
        assert_eq!(r["result"]["isError"], false, "{r}");
    }
    assert_eq!(
        tool(&a, "open_dm", json!({"user_id":JASON,"body":"forbidden"})).await["result"]["structuredContent"]
            ["status"],
        "forbidden"
    );
    let mut owner = a.sign_in(DAVID).await;
    let mut other = a.sign_in(JASON).await;
    let r = owner
        .write(Req::new(Method::POST, "/api/v1/directs").json_body(&json!({"userIds":[BENDER]})))
        .await;
    assert_eq!(r.status.as_u16(), 200, "{}", r.text());
    for ids in [vec![BENDER], vec![BENDER, DAVID]] {
        let r = other
            .write(Req::new(Method::POST, "/api/v1/directs").json_body(&json!({"userIds":ids})))
            .await;
        assert_eq!(r.status.as_u16(), 422, "{}", r.text());
        assert!(r.text().contains("userIds"));
    }
    let group = a
        .db()
        .write(|tx| Room::find_or_create_direct_for(tx, &[DAVID, JASON, 712064548], DAVID))
        .await
        .unwrap();
    let r = owner
        .write(
            Req::new(
                Method::POST,
                &format!("/api/v1/directs/{}/members", group.id),
            )
            .json_body(&json!({"userIds":[BENDER]})),
        )
        .await;
    assert_eq!(r.status.as_u16(), 422, "{}", r.text());
}

#[tokio::test]
async fn personal_agent_token_preserves_existing_api_boundaries() {
    let a = personal().await;
    for path in [
        "/api/v1/search?q=test",
        "/api/v1/agents",
        "/api/v1/rooms/486777696/files",
        "/api/v1/drive/files",
        "/api/v1/scheduled_messages",
        &format!("/rooms/{ALL_TALK}/{BENDER_KEY}/messages"),
    ] {
        let r = request(&a, Method::GET, path, json!({})).await;
        assert!(!r.status.is_success(), "{path}: {}", r.text());
    }
}

#[tokio::test]
async fn personal_agent_poll_and_ack_filter_one_revoked_room_while_another_is_live() {
    let a = personal().await;
    let ids = a
        .db()
        .write(|tx| {
            let mut ids = Vec::new();
            for (room, message) in [(ALL_TALK, 136976342), (DESIGNERS, 130701110)] {
                let message = if room == DESIGNERS {
                    campfire_db::Message::create(
                        tx,
                        campfire_db::NewMessage {
                            room_id: room,
                            creator_id: DAVID,
                            markdown_source: Some("Visible elsewhere".into()),
                            ..Default::default()
                        },
                    )?
                    .id
                } else {
                    message
                };
                ids.push(
                    AgentEvent::create(
                        tx,
                        NewEvent {
                            agent_id: AGENT,
                            event_type: "mention".into(),
                            room_id: Some(room),
                            message_id: Some(message),
                            outcome: Some("pending".into()),
                            ..Default::default()
                        },
                    )?
                    .id,
                );
            }
            tx.conn().execute(
                "DELETE FROM memberships WHERE user_id=? AND room_id=?",
                params![DAVID, ALL_TALK],
            )?;
            Ok(ids)
        })
        .await
        .unwrap();
    let page = tool(&a, "poll_events", json!({})).await;
    let selected = page["result"]["structuredContent"]["events"]
        .as_array()
        .unwrap();
    assert!(!selected.iter().any(|e| e["id"] == ids[0]));
    assert!(selected.iter().any(|e| e["id"] == ids[1]));
    let denied = tool(&a, "ack_events", json!({"event_ids":[ids[0]]})).await;
    assert!(
        denied["result"]["structuredContent"]["results"][0]["error"]
            .as_str()
            .unwrap()
            .contains("Forbidden"),
        "{denied}"
    );
    let allowed = tool(&a, "ack_events", json!({"event_ids":[ids[1]]})).await;
    assert_eq!(allowed["result"]["isError"], false, "{allowed}");
}

async fn webhook_revocation(variant: &str) {
    use crate::integrations::test_support::{
        FakeResolver, FakeServer, MappingDialer, Route, network,
    };
    use std::collections::HashSet;
    use std::sync::{Arc, Mutex};
    let a = personal().await;
    let (app, _dir) = a.stop_jobs().await;
    let event=app.db.write(|tx| {
        tx.conn().execute("INSERT INTO webhooks(user_id,url,created_at,updated_at) VALUES(?,'http://bots.example:8080/hook',?,?)",params![BENDER,tx.now(),tx.now()])?;
        let event=AgentEvent::create(tx,NewEvent{agent_id:AGENT,event_type:"mention".into(),room_id:Some(ALL_TALK),message_id:Some(136976342),outcome:Some("pending".into()),..Default::default()})?;
        delivery::perform_delivery(tx,event.id)?;
        let event=AgentEvent::find(tx.conn(),event.id)?.unwrap();
        assert_eq!(event.webhook_status,"pending");
        Ok(event)
    }).await.unwrap();
    let server = FakeServer::start(vec![Route::new(
        "POST",
        "*",
        "/hook",
        if variant == "retry" { 500 } else { 200 },
    )])
    .await;
    let network = network(
        Arc::new(FakeResolver::new([("bots.example", vec!["93.184.216.34"])])),
        Arc::new(MappingDialer {
            public: HashSet::from(["93.184.216.34".parse().unwrap()]),
            to: server.addr,
            dialed: Mutex::new(vec![]),
        }),
    );
    if variant == "retry" {
        crate::jobs::agent_jobs::post_with_network(
            &app,
            delivery::EventWebhookJob {
                event_id: event.id,
                attempt: Some(0),
            },
            &network,
        )
        .await
        .unwrap();
        assert_eq!(server.received.lock().unwrap().len(), 1);
    }
    let recover = variant == "recovery";
    let variant = variant.to_owned();
    let id = event.id;
    let attempt = app
        .db
        .write(move |tx| {
            if variant == "ack" {
                assert!(matches!(
                    events::acknowledge(tx, AGENT, id)?,
                    events::Acknowledgment::Acknowledged { .. }
                ));
                tx.conn().execute(
                    "UPDATE agent_events SET webhook_status='none' WHERE id=?",
                    [id],
                )?;
                delivery::perform_delivery(tx, id)?;
            }
            tx.conn().execute(
                "DELETE FROM memberships WHERE user_id=? AND room_id=?",
                params![DAVID, ALL_TALK],
            )?;
            tx.conn().execute(
                "UPDATE agent_events SET webhook_next_attempt_at=?,created_at=? WHERE id=?",
                params![
                    tx.now().ago(jiff::SignedDuration::from_mins(10)),
                    tx.now().ago(jiff::SignedDuration::from_mins(10)),
                    id
                ],
            )?;
            Ok(AgentEvent::find(tx.conn(), id)?.unwrap().webhook_attempts)
        })
        .await
        .unwrap();
    if recover {
        crate::jobs::periodic::stranded_agent_webhooks(&app.db)
            .await
            .unwrap();
    }
    let before = server.received.lock().unwrap().len();
    crate::jobs::agent_jobs::post_with_network(
        &app,
        delivery::EventWebhookJob {
            event_id: event.id,
            attempt: Some(attempt),
        },
        &network,
    )
    .await
    .unwrap();
    assert_eq!(
        server.received.lock().unwrap().len(),
        before,
        "revoked delivery must make zero POSTs"
    );
    let event = app
        .db
        .read(move |conn| AgentEvent::find(conn, id))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(event.webhook_status, "failed");
    server.replace_routes(vec![Route::new("POST", "*", "/hook", 200)]);
    let other = app
        .db
        .write(|tx| {
            let message = campfire_db::Message::create(
                tx,
                campfire_db::NewMessage {
                    room_id: DESIGNERS,
                    creator_id: DAVID,
                    markdown_source: Some("Still authorized".into()),
                    ..Default::default()
                },
            )?;
            let event = AgentEvent::create(
                tx,
                NewEvent {
                    agent_id: AGENT,
                    event_type: "mention".into(),
                    room_id: Some(DESIGNERS),
                    message_id: Some(message.id),
                    outcome: Some("pending".into()),
                    ..Default::default()
                },
            )?;
            delivery::perform_delivery(tx, event.id)?;
            Ok(event.id)
        })
        .await
        .unwrap();
    crate::jobs::agent_jobs::post_with_network(
        &app,
        delivery::EventWebhookJob {
            event_id: other,
            attempt: Some(0),
        },
        &network,
    )
    .await
    .unwrap();
    assert_eq!(server.received.lock().unwrap().len(), before + 1);
    assert_eq!(
        app.db
            .read(move |conn| AgentEvent::find(conn, other))
            .await
            .unwrap()
            .unwrap()
            .webhook_status,
        "delivered"
    );
}
#[tokio::test]
async fn personal_agent_webhook_revocation_after_enqueue() {
    webhook_revocation("queued").await;
}
#[tokio::test]
async fn personal_agent_webhook_revocation_before_retry() {
    webhook_revocation("retry").await;
}
#[tokio::test]
async fn personal_agent_webhook_revocation_after_ack_reenqueue() {
    webhook_revocation("ack").await;
}
#[tokio::test]
async fn personal_agent_webhook_revocation_before_recovery() {
    webhook_revocation("recovery").await;
}

#[tokio::test]
async fn personal_agent_sync_reply_checks_revocation_inside_writer() {
    let a = personal().await;
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "DELETE FROM memberships WHERE user_id=? AND room_id=?",
                params![DAVID, ALL_TALK],
            )?;
            let trigger = campfire_db::Message::find(tx.conn(), 136976342)?;
            let before: i64 = tx
                .conn()
                .query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0))?;
            assert!(
                crate::jobs::integrations::create_reply(
                    tx,
                    Some(&trigger),
                    campfire_db::NewMessage {
                        room_id: ALL_TALK,
                        creator_id: BENDER,
                        markdown_source: Some("revoked reply".into()),
                        ..Default::default()
                    }
                )
                .is_err()
            );
            assert_eq!(
                tx.conn()
                    .query_row("SELECT COUNT(*) FROM messages", [], |r| r.get::<_, i64>(0))?,
                before
            );
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn personal_agent_admin_grant_api_returns_422_for_forbidden_capability() {
    let a = personal().await;
    let mut owner = a.sign_in(DAVID).await;
    owner.grant_sudo().await;
    let r = owner
        .write(
            Req::new(Method::POST, &format!("/api/v1/admin/bots/{BENDER}/grants"))
                .json_body(&json!({"capability":"external_action","roomId":ALL_TALK})),
        )
        .await;
    assert_eq!(r.status.as_u16(), 422, "{}", r.text());
}

#[tokio::test]
async fn personal_agent_rest_denials_survive_planted_forbidden_grants() {
    let a = personal().await;
    let approval=a.db().write(|tx| {
        tx.conn().execute_batch("DROP TRIGGER personal_agent_grant_insert;DROP TRIGGER personal_agent_grant_update;")?;
        for cap in ["external_action","fizzy","dm_anyone"] {
            tx.conn().execute("INSERT INTO agent_grants(agent_id,capability,granted_by_id,created_at,updated_at) VALUES(?,?,?,?,?)",params![AGENT,cap,DAVID,tx.now(),tx.now()])?;
        }
        Ok(campfire_db::AgentApproval::create(tx,campfire_db::NewApproval{agent_id:AGENT,room_id:Some(ALL_TALK),action:"deploy".into(),summary:"Blocked approval".into(),..Default::default()})?.id)
    }).await.unwrap();
    for (method, path, body) in [
        (Method::GET, "/agents/approvals".to_owned(), json!({})),
        (
            Method::POST,
            "/agents/approvals".to_owned(),
            json!({"room_id":ALL_TALK,"action":"deploy","summary":"blocked"}),
        ),
        (
            Method::GET,
            format!("/agents/approvals/{approval}"),
            json!({}),
        ),
        (
            Method::DELETE,
            format!("/agents/approvals/{approval}"),
            json!({}),
        ),
        (
            Method::POST,
            format!("/rooms/{ALL_TALK}/agents/slash_commands"),
            json!({"name":"blocked"}),
        ),
        (
            Method::DELETE,
            format!("/rooms/{ALL_TALK}/agents/slash_commands/blocked"),
            json!({}),
        ),
        (
            Method::POST,
            "/agents/work/1/handoff".to_owned(),
            json!({"receiver_agent_id":AGENT,"summary":"blocked"}),
        ),
        (Method::GET, "/agents/fizzy/boards".to_owned(), json!({})),
        (Method::GET, "/agents/fizzy/boards/1".to_owned(), json!({})),
        (
            Method::GET,
            "/agents/fizzy/cards/search".to_owned(),
            json!({}),
        ),
        (
            Method::GET,
            "/agents/fizzy/cards/account/1".to_owned(),
            json!({}),
        ),
        (
            Method::POST,
            "/agents/fizzy/card_actions".to_owned(),
            json!({"action":"fizzy.card.create","summary":"blocked"}),
        ),
    ] {
        let r = request(&a, method, &path, body).await;
        assert_eq!(r.status.as_u16(), 403, "{path}: {}", r.text());
    }
}

#[tokio::test]
async fn personal_agent_integration_execution_and_requests_deny_planted_grants_without_network() {
    use crate::integrations::{
        fizzy, github,
        test_support::{FakeResolver, MappingDialer, network},
    };
    use std::sync::Arc;
    let a = personal().await;
    let crypto = Arc::new(rails_compat::ar_encryption::ArEncryption::new(
        &a.booted.app.secrets,
    ));
    let saved_crypto = crypto.clone();
    let (github_id,fizzy_id)=a.db().write(move|tx| {
        tx.conn().execute_batch("DROP TRIGGER personal_agent_grant_insert;DROP TRIGGER personal_agent_grant_update;")?;
        for (cap,room) in [("external_action",Some(ALL_TALK)),("external_action",None),("fizzy",None)] {
            tx.conn().execute("INSERT INTO agent_grants(agent_id,capability,room_id,granted_by_id,created_at,updated_at) VALUES(?,?,?,?,?,?)",params![AGENT,cap,room,DAVID,tx.now(),tx.now()])?;
        }
        let thread=ChannelThread::create(tx,NewChannelThread{room_id:ALL_TALK,creator_id:DAVID,name:Some("Blocked PR".into()),..Default::default()})?;
        tx.conn().execute("INSERT INTO github_pull_requests(owner,repo,number,created_at,updated_at) VALUES('s1','policy',1,?,?)",[tx.now(),tx.now()])?;
        let pr=tx.conn().last_insert_rowid();
        tx.conn().execute("INSERT INTO github_pull_request_threads(github_pull_request_id,room_id,channel_thread_id,created_at,updated_at) VALUES(?,?,?,?,?)",params![pr,ALL_TALK,thread.id,tx.now(),tx.now()])?;
        match github::approval_requests::scope(tx.conn(),BENDER,ALL_TALK,Some(pr))? {
            Err(reply)=>assert_eq!(reply.status,403),Ok(_)=>panic!("personal GitHub scope was authorized"),
        }
        let account=github::accounts::Account::relink(tx,&saved_crypto,&github::accounts::AccountInput{user_id:DAVID,github_login:"policy",access_token:"fixture-policy-token",refresh_token:None,token_expires_at:None,token_source:"pat"})?;
        let reply=github::approval_requests::create(tx,github::approval_requests::Request{user_id:BENDER,room_id:ALL_TALK,pr_id:Some(pr),secret:SECRET.into(),account,submitted:json!({})})?;
        assert_eq!(reply.status,403);
        let reply=fizzy::agent_requests::create(tx,&saved_crypto,AGENT,json!({}),None,&jiff::tz::TimeZone::UTC)?;
        assert_eq!(reply.status,403);
        let mut ids=Vec::new();
        for (action,room) in [("github.comment",Some(ALL_TALK)),("fizzy.comment",None)] {
            ids.push(campfire_db::AgentApproval::create(tx,campfire_db::NewApproval{agent_id:AGENT,action:action.into(),summary:"Policy witness".into(),status:"approved".into(),room_id:room,..Default::default()})?.id);
        }
        Ok((ids[0],ids[1]))
    }).await.unwrap();
    let resolver = Arc::new(FakeResolver::default());
    let dialer = Arc::new(MappingDialer {
        public: Default::default(),
        to: "127.0.0.1:9".parse().unwrap(),
        dialed: Default::default(),
    });
    let net = network(resolver.clone(), dialer.clone());
    let result = fizzy::agent_reads::read(
        &a.booted.app,
        &net,
        "http://app.fizzy.do",
        AGENT,
        fizzy::agent_reads::Read::Boards {
            account: Value::Null,
        },
    )
    .await
    .unwrap();
    assert_eq!(result.status, 403);
    let accounts = github::accounts::Accounts::with_network(
        a.db().clone(),
        crypto,
        github::client::AppClient::new(None, None),
        net.clone(),
    );
    github::agent_actions::perform(a.db(), &accounts, github_id)
        .await
        .unwrap();
    fizzy::agent_job::execute(&a.booted.app, &net, "http://app.fizzy.do", fizzy_id)
        .await
        .unwrap();
    a.db()
        .read(move |conn| {
            for (approval, kind) in [
                (github_id, "github_action_completed"),
                (fizzy_id, "fizzy_action_completed"),
            ] {
                let metadata: Value = conn.query_row(
                    "SELECT metadata FROM agent_events WHERE agent_approval_id=? AND event_type=?",
                    params![approval, kind],
                    |r| r.get(0),
                )?;
                assert_eq!(metadata["status"], "failed");
                assert_eq!(
                    metadata["message"],
                    "Agent no longer has the external_action capability"
                );
            }
            Ok(())
        })
        .await
        .unwrap();
    assert!(resolver.lookups().is_empty());
    assert!(dialer.dialed.lock().unwrap().is_empty());
}

#[tokio::test]
async fn personal_agent_rejects_legacy_keys_and_reply_tokens_even_with_planted_digest() {
    use sha2::{Digest, Sha256};
    let a = personal().await;
    let secrets = a.booted.app.secrets.clone();
    let now = a.booted.app.clock.now();
    let token = rails_compat::verifiers::bot_reply::token_for(&secrets, BENDER, ALL_TALK, now);
    a.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE users SET bot_token_digest=? WHERE id=?",
                params![format!("{:x}", Sha256::digest("BenderToken1")), BENDER],
            )?;
            assert!(campfire_db::User::authenticate_bot(tx.conn(), BENDER_KEY)?.is_none());
            assert!(
                campfire_runtime::concerns::authenticate_bot_reply_token(
                    tx.conn(),
                    &secrets,
                    &token,
                    &ALL_TALK.to_string(),
                    now
                )?
                .is_none()
            );
            tx.conn()
                .execute("UPDATE agents SET kind='workspace' WHERE id=?", [AGENT])?;
            assert!(campfire_db::User::authenticate_bot(tx.conn(), BENDER_KEY)?.is_some());
            assert!(
                campfire_runtime::concerns::authenticate_bot_reply_token(
                    tx.conn(),
                    &secrets,
                    &token,
                    &ALL_TALK.to_string(),
                    now
                )?
                .is_some()
            );
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn personal_agent_post_only_reply_and_replay_do_not_disclose_source_body() {
    let a = personal().await;
    let source=a.db().write(|tx| {
        let source=campfire_db::Message::create(tx,campfire_db::NewMessage{room_id:ALL_TALK,creator_id:DAVID,markdown_source:Some("Confidential source body".into()),..Default::default()})?;
        tx.conn().execute("UPDATE agent_grants SET revoked_at=? WHERE agent_id=? AND capability='read_messages' AND room_id=?",params![tx.now(),AGENT,ALL_TALK])?;
        Ok(source.id)
    }).await.unwrap();
    for _ in 0..2 {
        let reply=tool(&a,"post_message",json!({"room_id":ALL_TALK,"body":"My reply","reply_to_message_id":source,"client_message_id":"s1-post-only-reply"})).await;
        assert_eq!(reply["result"]["isError"], false, "{reply}");
        assert!(
            !reply.to_string().contains("Confidential source body"),
            "{reply}"
        );
    }
}

#[tokio::test]
async fn personal_agent_member_owner_thread_removal_filters_board_work_and_events() {
    const KEVIN: i64 = 712064548;
    let a = personal().await;
    let (board, thread, event) = a
        .db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE agents SET owner_id=? WHERE id=?",
                params![KEVIN, AGENT],
            )?;
            let board = Room::create_for(
                tx,
                campfire_db::RoomType::Board,
                Some("Personal board"),
                DAVID,
                &[DAVID, KEVIN, BENDER],
            )?;
            for cap in ["read_messages", "post_messages", "react", "manage_threads"] {
                AgentGrant::create(
                    tx,
                    NewGrant {
                        agent_id: AGENT,
                        room_id: Some(board.id),
                        capability: cap.into(),
                        granted_by_id: DAVID,
                        ..Default::default()
                    },
                )?;
            }
            let thread = ChannelThread::create(
                tx,
                NewChannelThread {
                    room_id: board.id,
                    creator_id: DAVID,
                    name: Some("Private board work".into()),
                    work_status: Some("in_progress".into()),
                    visibility: campfire_db::channel_thread::ThreadVisibility::Private,
                    ..Default::default()
                },
            )?;
            thread.add_access_member(tx, DAVID, KEVIN)?;
            thread.add_access_member(tx, DAVID, BENDER)?;

            tx.conn().execute(
                "UPDATE channel_threads SET work_owner_id=? WHERE id=?",
                params![BENDER, thread.id],
            )?;
            let event = AgentEvent::create(
                tx,
                NewEvent {
                    agent_id: AGENT,
                    room_id: Some(board.id),
                    event_type: "work_assigned".into(),
                    outcome: Some("delivered".into()),
                    metadata: json!({"thread_id":thread.id,"thread_visibility":"private"}),
                    ..Default::default()
                },
            )?;
            Ok((board.id, thread.id, event.id))
        })
        .await
        .unwrap();
    let before = tool(&a, "list_board_posts", json!({"room_id":board})).await;
    assert!(
        before.to_string().contains("Private board work"),
        "{before}"
    );
    a.db()
        .write(move |tx| {
            ChannelThread::find(tx.conn(), thread)?.remove_access_member(tx, DAVID, KEVIN)
        })
        .await
        .unwrap();
    for (name, args) in [
        ("list_board_posts", json!({"room_id":board})),
        ("list_work", json!({})),
        ("poll_events", json!({})),
    ] {
        let after = tool(&a, name, args).await;
        assert_eq!(after["result"]["isError"], false, "{name}: {after}");
        assert!(
            !after.to_string().contains("Private board work"),
            "{name}: {after}"
        );
    }
    assert_eq!(
        request(
            &a,
            Method::GET,
            &format!("/agents/work/{thread}"),
            json!({})
        )
        .await
        .status
        .as_u16(),
        404
    );
    a.db()
        .write(move |tx| {
            assert!(!events::delivery_authorized(tx.conn(), event)?);
            assert!(!matches!(
                events::acknowledge(tx, AGENT, event)?,
                events::Acknowledgment::Acknowledged { .. }
            ));
            Ok(())
        })
        .await
        .unwrap();
}
