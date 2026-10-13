//! WS14g's Drive event projection through the real REST and MCP transports.
use super::agent_http_tests::{AGENT, SECRET, setup};
use super::presenters::test_support::{Reply, Req, TestApp};
use campfire_db::{AgentGrant, ChannelThread, Message, NewChannelThread, NewGrant, NewMessage, Room, RoomType};
use campfire_db::models::agent_delivery::{AgentEvent, NewEvent};
use campfire_kit::Method;
use serde_json::{Value, json};

const DAVID: i64 = 127326141;
const BENDER: i64 = 394959859;

async fn page(app: &TestApp, since: i64) -> Reply {
    app.anonymous().send(
        Req::new(Method::GET, &format!("/agents/events?since={since}&limit=1&envelope=1"))
            .header("authorization", &format!("Bearer {SECRET}"))
            .header("accept", "application/json"),
    ).await
}

#[tokio::test]
async fn agent_polling_public_thread_in_deleted_room_serializes_and_revoked_private_rows_do_not_consume_limit() {
    let app = setup().await.without_job_runner().await;
    let (room, private, private_event, public_event, public_message) = app.db().write(|tx| {
        let room = Room::create_for(tx, RoomType::Closed, Some("Polling lifecycle"), DAVID, &[DAVID, BENDER])?;
        AgentGrant::create(tx, NewGrant {
            agent_id: AGENT, granted_by_id: DAVID, capability: "read_messages".into(),
            room_id: None, revoked_at: None,
        })?;
        let private = ChannelThread::create(tx, NewChannelThread {
            room_id: room.id, creator_id: DAVID, name: Some("Private polling context".into()),
            visibility: campfire_db::channel_thread::ThreadVisibility::Private, ..Default::default()
        })?;
        private.add_access_member(tx, DAVID, BENDER)?;
        let public = ChannelThread::create(tx, NewChannelThread {
            room_id: room.id, creator_id: DAVID, name: Some("Public polling context".into()),
            ..Default::default()
        })?;
        let mut events = Vec::new();
        let mut messages = Vec::new();
        for (thread, body) in [(&private, "Secret polling body"), (&public, "Public polling body")] {
            let message = Message::create(tx, NewMessage {
                room_id: room.id, thread_id: Some(thread.id), creator_id: DAVID,
                markdown_source: Some(body.into()), ..Default::default()
            })?;
            events.push(AgentEvent::create(tx, NewEvent {
                agent_id: AGENT, room_id: Some(room.id), actor_id: Some(DAVID),
                message_id: Some(message.id), event_type: "mention".into(), outcome: Some("delivered".into()),
                metadata: json!({"thread_id": thread.id, "thread_visibility": thread.visibility}),
                ..Default::default()
            })?.id);
            messages.push(message.id);
        }
        Ok((room.id, private.id, events[0], events[1], messages[1]))
    }).await.unwrap();
    let private_page = page(&app, private_event - 1).await;
    assert_eq!(private_page.status.as_u16(), 200, "{}", private_page.text());
    assert_eq!(private_page.json()["events"][0]["id"], private_event);
    let public_page = page(&app, public_event - 1).await;
    assert_eq!(public_page.status.as_u16(), 200, "{}", public_page.text());
    let expected = public_page.json();
    assert_eq!(expected["events"][0]["message"]["id"], public_message);
    assert_eq!(expected["events"][0]["message"]["body"]["markdown_source"], "Public polling body");
    app.db().write(move |tx| ChannelThread::find(tx.conn(), private)?.remove_access_member(tx, DAVID, BENDER)).await.unwrap();
    let revoked = page(&app, private_event - 1).await;
    assert_eq!(revoked.status.as_u16(), 200, "{}", revoked.text());
    assert_eq!(revoked.json(), expected, "private access is filtered before the single-row limit");
    app.db().write(move |tx| {
        tx.conn().execute("UPDATE rooms SET deleted_at=? WHERE id=?", rusqlite::params![tx.now(), room])?;
        Ok(())
    }).await.unwrap();
    let deleted = page(&app, private_event - 1).await;
    assert_eq!(deleted.status.as_u16(), 200, "{}", deleted.text());
    assert_eq!(deleted.json(), expected, "public workspace history survives room archival");
    assert_eq!(deleted.json()["next_since"], public_event);
    assert_eq!(deleted.header("x-smartfire-next-since"), Some(public_event.to_string().as_str()));
    let next = page(&app, public_event).await;
    assert_eq!(next.status.as_u16(), 200, "{}", next.text());
    assert_eq!(next.json(), json!({"events": [], "next_since": public_event}));
    let following = app.db().write(move |tx| {
        Message::find(tx.conn(), public_message)?.destroy(tx)?;
        assert!(AgentEvent::find(tx.conn(), public_event)?.is_some());
        let approval=campfire_db::AgentApproval::create(tx,campfire_db::NewApproval{agent_id:AGENT,action:"github.comment".into(),summary:"Live completion".into(),..Default::default()})?;
        AgentGrant::create(tx,NewGrant{agent_id:AGENT,granted_by_id:DAVID,capability:"external_action".into(),..Default::default()})?;
        Ok(AgentEvent::create(tx, NewEvent {
            agent_approval_id:Some(approval.id),
            agent_id: AGENT, event_type: "github_action_completed".into(), outcome: Some("delivered".into()),
            metadata: json!({"action": "github.comment", "status": "done"}), ..Default::default()
        })?.id)
    }).await.unwrap();
    let missing = page(&app, public_event - 1).await;
    assert_eq!(missing.status.as_u16(), 200, "{}", missing.text());
    assert_eq!(missing.json()["events"].as_array().unwrap().iter().map(|event| event["id"].as_i64().unwrap()).collect::<Vec<_>>(), [following]);
    assert_eq!(missing.json()["next_since"], following, "a missing message cannot consume the limit or stall the cursor");
}

async fn group(mcp: bool) {
    let vectors: Value =
        serde_json::from_str(include_str!("../../../../vectors/agent_polling_http.json")).unwrap();
    for case in vectors["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| case["name"].as_str().unwrap().starts_with("mcp_") == mcp)
    {
        super::agent_reads_tests::check(case).await;
    }
}
#[tokio::test]
async fn agent_polling_rest_drive_bytes() {
    group(false).await;
}
#[tokio::test]
async fn agent_polling_mcp_drive_bytes() {
    group(true).await;
}
