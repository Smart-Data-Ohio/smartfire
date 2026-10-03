//! Real socket coverage for WS11 callbacks rendered by WS11-ui after commit.
use super::*;
use futures_util::{SinkExt, Stream, StreamExt};
use serde_json::{Value, json};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{Message, client::IntoClientRequest},
};
type Socket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

async fn activity_socket(test: &Test, address: std::net::SocketAddr, viewer: i64) -> Socket {
    use campfire_kit::Crypto;
    let session = test
        .booted
        .app
        .db
        .write(move |tx| {
            campfire_db::Session::start_with(
                tx,
                viewer,
                campfire_db::NewSession {
                    two_factor_verified: true,
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let signed = campfire_kit::RailsCrypto::new(test.booted.app.secrets.clone()).sign_cookie(
        "session_token",
        &session.token,
        None,
    );
    let mut request = format!("ws://{address}/cable")
        .into_client_request()
        .unwrap();
    request.headers_mut().insert("host", HOST.parse().unwrap());
    request
        .headers_mut()
        .insert("origin", format!("http://{HOST}").parse().unwrap());
    request.headers_mut().insert(
        "cookie",
        format!("session_token={}", campfire_kit::cookies::escape(&signed))
            .parse()
            .unwrap(),
    );
    let (mut socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    assert_eq!(receive(&mut socket).await["type"], "welcome");
    let identifier = json!({"channel":"ActivityChannel"}).to_string();
    socket
        .send(Message::Text(
            json!({"command":"subscribe","identifier":identifier})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    assert_eq!(receive(&mut socket).await["type"], "confirm_subscription");
    socket
}

/// Rails broadcasts a private ActivityChannel ID when an approval is created or
/// settled. Cards with the viewer's decision-form tokens remain request renders.
#[tokio::test]
async fn approval_activity_ids_follow_committed_http_decisions_without_cross_user_leaks() {
    use campfire_db::{Agent, AgentApproval, NewApproval};
    let test = boot_seed("default").await.expect("default seed");
    let bot: i64 = test.label("users.bender").parse().unwrap();
    let owner_id: i64 = test.label("users.kevin").parse().unwrap();
    let other_id: i64 = test.label("users.jz").parse().unwrap();
    let agent_id = test
        .booted
        .app
        .db
        .write(move |tx| {
            let mut agent = Agent::for_user(tx.conn(), bot)?.unwrap();
            agent.update(
                tx,
                campfire_db::AgentChanges {
                    owner_id: Some(Some(owner_id)),
                    ..Default::default()
                },
            )?;
            Ok(agent.id)
        })
        .await
        .unwrap();
    let listener = crate::channels::tests::support::bind_listener().await;
    let address = listener.local_addr().unwrap();
    let router = test.booted.router.clone();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut owner_socket = activity_socket(&test, address, owner_id).await;
    let mut other_socket = activity_socket(&test, address, other_id).await;
    let mut owner = test.browser("198.51.100.157");
    owner.sign_in(&test.label("emails.kevin")).await;
    for reject_audit in [false, true] {
        let approval_id = test.booted.app.db.write(move |tx| {
            let approval = AgentApproval::create(tx,NewApproval {
                agent_id,action:"deploy".into(),summary:"Private approval summary".into(),
                ..Default::default()
            })?;
            if reject_audit {
                tx.conn().execute_batch("CREATE TRIGGER reject_live_decision_audit BEFORE INSERT ON audit_logs WHEN NEW.action='agent.approval.decide' BEGIN SELECT RAISE(ABORT,'test audit rejection'); END;")?;
            }
            Ok(approval.id)
        }).await.unwrap();
        // WS11 now fans out in a separate after-commit writer, as Rails does.
        let item_id = test
            .booted
            .app
            .db
            .read(move |conn| {
                Ok(campfire_db::ActivityItem::find_by_user_and_source(
                    conn,
                    owner_id,
                    "AgentApproval",
                    approval_id,
                )?
                .expect("committed inbox fanout")
                .id)
            })
            .await
            .unwrap();
        let created = receive_expected(&mut owner_socket, "committed approval must broadcast its activity item").await;
        assert_eq!(created["message"], json!({"activityItemId":item_id}));
        let response = owner
            .form(
                "patch",
                &format!("/agent_approvals/{approval_id}.json"),
                &[("decision", "denied"), ("note", "Private decision note")],
            )
            .await;
        if reject_audit {
            assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
            test.booted
                .app
                .db
                .read(move |conn| {
                    assert_eq!(
                        AgentApproval::find(conn, approval_id)?.unwrap().status,
                        "denied"
                    );
                    assert!(
                        campfire_db::ActivityItem::find_by_user_and_source(
                            conn,
                            owner_id,
                            "AgentApproval",
                            approval_id
                        )?
                        .unwrap()
                        .handled_at
                        .is_some()
                    );
                    Ok(())
                })
                .await
                .unwrap();
        } else {
            assert_eq!(response.status, StatusCode::OK, "{}", response.text());
        }
        // decide! committed before AuditLog.record!: even an HTTP 500 sends
        // the handled item's private frame exactly once, as the pinned oracle.
        let decided = receive(&mut owner_socket).await;
        assert_eq!(decided["message"], json!({"activityItemId":item_id}));
        assert_eq!(
            owner
                .form(
                    "patch",
                    &format!("/agent_approvals/{approval_id}.json"),
                    &[("decision", "approved")]
                )
                .await
                .status,
            StatusCode::UNPROCESSABLE_ENTITY
        );
        for socket in [&mut owner_socket, &mut other_socket] {
            assert_no_activity(socket).await;
        }
    }
    owner_socket.close(None).await.unwrap();
    other_socket.close(None).await.unwrap();
    server.abort();
}
async fn assert_no_activity<S, E>(socket: &mut S)
where
    S: Stream<Item = Result<Message, E>> + Unpin,
    E: std::fmt::Debug,
{
    // ActionCable heartbeats are connection traffic, not an ActivityChannel
    // broadcast. Bound the entire ping-filtering read by the original deadline.
    let received =
        tokio::time::timeout(std::time::Duration::from_millis(150), receive(socket)).await;
    assert!(
        received.is_err(),
        "duplicate or another user's activity reached the socket: {received:?}"
    );
}

#[tokio::test]
async fn approval_silence_filters_heartbeats_without_hiding_private_activity() {
    let ping = Message::Text(json!({"type":"ping","message":1}).to_string().into());
    let mut heartbeats =
        futures_util::stream::iter([Ok::<_, tokio_tungstenite::tungstenite::Error>(ping.clone())])
            .chain(futures_util::stream::pending());
    assert_no_activity(&mut heartbeats).await;

    let private = json!({"identifier":"ActivityChannel","message":{"activityItemId":1}});
    let mut activity = futures_util::stream::iter([
        Ok::<_, tokio_tungstenite::tungstenite::Error>(ping),
        Ok(Message::Text(private.to_string().into())),
    ]);
    assert_eq!(receive(&mut activity).await, private);
}

async fn receive<S, E>(socket: &mut S) -> Value
where
    S: Stream<Item = Result<Message, E>> + Unpin,
    E: std::fmt::Debug,
{
    receive_expected(socket, "cable frame before timeout").await
}

async fn receive_expected<S, E>(socket: &mut S, expectation: &str) -> Value
where
    S: Stream<Item = Result<Message, E>> + Unpin,
    E: std::fmt::Debug,
{
    loop {
        let frame = tokio::time::timeout(std::time::Duration::from_secs(2), socket.next())
            .await
            .expect(expectation)
            .unwrap()
            .unwrap();
        if let Message::Text(text) = frame {
            let value: Value = serde_json::from_str(&text).unwrap();
            if value["type"] != "ping" {
                return value;
            }
        }
    }
}

/// Rails Agents::Steps.broadcast_parent replaces the complete message, in its
/// conversation, even when message.updated_at and a warmed fragment do not change.
#[tokio::test]
async fn message_step_callbacks_replace_current_message_in_room_and_thread_without_cached_tokens() {
    use crate::channels::broadcasts::Stream;
    use campfire_db::models::agent_step::{self, AgentStepChanges, NewAgentStep};
    let test = boot_seed("default").await.expect("default seed");
    let bot: i64 = test.label("users.bender").parse().unwrap();
    let viewer: i64 = test.label("users.david").parse().unwrap();
    let room_id: i64 = test.label("rooms.watercooler").parse().unwrap();
    let listener = crate::channels::tests::support::bind_listener().await;
    let address = listener.local_addr().unwrap();
    let router = test.booted.router.clone();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    for in_thread in [false, true] {
        let (agent_id, message) = test
            .booted
            .app
            .db
            .write(move |tx| {
                let agent = campfire_db::Agent::for_user(tx.conn(), bot)?.unwrap();
                let thread_id = if in_thread {
                    Some(
                        campfire_db::ChannelThread::create(
                            tx,
                            campfire_db::NewChannelThread {
                                room_id,
                                creator_id: viewer,
                                name: Some("Message step conversation".into()),
                                ..Default::default()
                            },
                        )?
                        .id,
                    )
                } else {
                    None
                };
                let message = campfire_db::Message::create_markdown(
                    tx,
                    campfire_db::NewMessage {
                        room_id,
                        creator_id: bot,
                        thread_id,
                        ..Default::default()
                    },
                    "Message carrying structured progress",
                )?;
                Ok((agent.id, message))
            })
            .await
            .unwrap();
        let message_id = message.id;
        let target = format!("message_{}", message.client_message_id);
        let warm = test
            .booted
            .app
            .db
            .read({
                let app = test.booted.app.clone();
                let message = message.clone();
                move |conn| {
                    let view = crate::controllers::presenters::Presenter::new(conn, &app, None)
                        .message(&message)?;
                    let account = campfire_db::Account::first(conn)?;
                    Ok(crate::controllers::presenters::page::render_detached(
                        &app,
                        account.as_ref(),
                        |ctx| campfire_views::messages::message(ctx, &view),
                    ))
                }
            })
            .await
            .unwrap();
        assert!(!warm.contains("Inspect &lt;message&gt;"));
        let room = test
            .booted
            .app
            .db
            .read(move |conn| campfire_db::Room::find(conn, room_id))
            .await
            .unwrap();
        let stream = Stream::conversation(&room, &message);
        let signed = rails_compat::turbo::signed_stream_name(
            &test.booted.app.secrets,
            &stream.streamables(),
        );
        let identifier =
            json!({"channel":"RoomMessagesChannel","signed_stream_name":signed}).to_string();
        let mut socket = activity_socket(&test, address, viewer).await;
        socket
            .send(Message::Text(
                json!({"command":"subscribe","identifier":identifier})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
        assert_eq!(receive(&mut socket).await["type"], "confirm_subscription");
        let id = test
            .booted
            .app
            .db
            .write(move |tx| {
                let result = agent_step::create(
                    tx,
                    agent_id,
                    NewAgentStep {
                        message_id: Some(message_id),
                        name: "Inspect <message>".into(),
                        input_summary: Some("Safe & escaped".into()),
                        ..Default::default()
                    },
                )?;
                assert!(result.is_ok(), "{:?}", result.error);
                Ok(result.payload.unwrap()["id"].as_i64().unwrap())
            })
            .await
            .unwrap();
        let frame = receive_expected(&mut socket, "committed message step must broadcast the current message").await;
        let html = frame["message"].as_str().unwrap();
        assert!(
            html.starts_with(&format!(
                "<turbo-stream action=\"replace\" target=\"{target}\">"
            )),
            "{html}"
        );
        assert!(html.contains("Message carrying structured progress"));
        assert!(html.contains("Inspect &lt;message&gt;"), "message step callback must render current uncached steps");
        assert!(html.contains("Safe &amp; escaped"));
        for private in [
            "authenticity_token",
            "csrf-token",
            "__CAMPFIRE_CSRF",
            "<script",
        ] {
            assert!(!html.contains(private), "message callback leaked {private}");
        }
        test.booted
            .app
            .db
            .write(move |tx| {
                let result = agent_step::update(
                    tx,
                    agent_id,
                    id,
                    AgentStepChanges {
                        status: Some("done".into()),
                        duration_ms: Some(Some(1050)),
                        output_summary: Some(Some("Current <result>".into())),
                        ..Default::default()
                    },
                )?;
                assert!(result.is_ok());
                Ok(())
            })
            .await
            .unwrap();
        let updated = receive(&mut socket).await;
        let html = updated["message"].as_str().unwrap();
        assert!(html.contains("agent-steps__step--done"));
        assert!(html.contains("1.0s"));
        assert!(html.contains("Current &lt;result&gt;"));
        assert!(
            test.booted
                .app
                .db
                .write(move |tx| -> campfire_db::Result<()> {
                    agent_step::update(
                        tx,
                        agent_id,
                        id,
                        AgentStepChanges {
                            status: Some("failed".into()),
                            ..Default::default()
                        },
                    )?;
                    Err(campfire_db::Error::Other(
                        "rollback message step test".into(),
                    ))
                })
                .await
                .is_err()
        );
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(250), socket.next())
                .await
                .is_err(),
            "rolled-back message callback reached the socket"
        );
        socket.close(None).await.unwrap();
    }
    server.abort();
}

#[tokio::test]
async fn working_presence_is_polled_and_does_not_emit_status_callbacks() {
    // app/models/agent.rb only broadcasts status/status_note changes. The
    // member panel polls working_presence_text through Rooms::MembersController.
    let test = boot_seed("default").await.expect("default seed");
    let bot: i64 = test.label("users.bender").parse().unwrap();
    let viewer: i64 = test.label("users.david").parse().unwrap();
    let listener = crate::channels::tests::support::bind_listener().await;
    let address = listener.local_addr().unwrap();
    let router = test.booted.router.clone();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut socket = activity_socket(&test, address, viewer).await;
    let identifier = json!({"channel":"AgentsChannel"}).to_string();
    socket
        .send(Message::Text(
            json!({"command":"subscribe","identifier":identifier})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    assert_eq!(receive(&mut socket).await["type"], "confirm_subscription");
    let id = test
        .booted
        .app
        .db
        .write(move |tx| {
            let agent = campfire_db::Agent::for_user(tx.conn(), bot)?.unwrap();
            let result = campfire_db::models::agent_working_presence::set(
                tx,
                agent.id,
                Some("  Reading <queue>  "),
            )?;
            assert!(result.is_ok());
            assert_eq!(
                result.payload.unwrap()["working_presence"],
                "Reading <queue>"
            );
            let reloaded = campfire_db::Agent::find(tx.conn(), agent.id)?.unwrap();
            assert_eq!(
                reloaded.working_presence_text(tx.now()),
                Some("Reading <queue>")
            );
            assert_eq!(
                reloaded.working_presence_text(tx.now().since(jiff::SignedDuration::from_mins(5))),
                None
            );
            Ok(agent.id)
        })
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(250), socket.next())
            .await
            .is_err(),
        "working presence incorrectly broadcast a status callback"
    );
    test.booted
        .app
        .db
        .write(move |tx| {
            let result =
                campfire_db::models::agent_working_presence::set(tx, id, Some(&"x".repeat(141)))?;
            assert_eq!(result.status, 422);
            assert_eq!(
                campfire_db::Agent::find(tx.conn(), id)?
                    .unwrap()
                    .working_presence_text(tx.now()),
                Some("Reading <queue>")
            );
            let result = campfire_db::models::agent_working_presence::set(tx, id, None)?;
            assert!(result.is_ok());
            assert!(result.payload.unwrap()["working_presence"].is_null());
            Ok(())
        })
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(250), socket.next())
            .await
            .is_err(),
        "working presence clear incorrectly broadcast a status callback"
    );
    // A status-note-only write does broadcast both fragments, with current data.
    test.booted
        .app
        .db
        .write(move |tx| {
            let mut agent = campfire_db::Agent::find(tx.conn(), id)?.unwrap();
            agent.update(
                tx,
                campfire_db::AgentChanges {
                    status_note: Some(Some("Ready <again>".into())),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    for target in [
        format!("status_badge_agent_{id}"),
        format!("directory_row_agent_{id}"),
    ] {
        let frame = receive_expected(&mut socket, "committed status note must broadcast both status fragments").await;
        assert_eq!(frame["identifier"], identifier);
        let html = frame["message"].as_str().unwrap();
        assert!(html.starts_with(&format!(
            "<turbo-stream action=\"replace\" target=\"{target}\">"
        )));
        assert!(html.contains("Ready &lt;again&gt;"));
        assert!(!html.contains("Reading &lt;queue&gt;"));
    }
    socket.close(None).await.unwrap();
    server.abort();
}

#[tokio::test]
async fn status_callback_replaces_badge_then_directory_over_live_socket_after_commit() {
    let test = boot_seed("default").await.expect("default seed");
    let mut viewer = test.browser("198.51.100.171");
    viewer.sign_in(&test.label("emails.kevin")).await;
    assert_eq!(viewer.get("/agents").await.status, StatusCode::OK);
    let listener = crate::channels::tests::support::bind_listener().await;
    let address = listener.local_addr().unwrap();
    let router = test.booted.router.clone();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    // WS9's second-factor flow is not on this base. As Rails' cable tests do,
    // create a verified human session through the existing Session domain seam.
    let viewer_id: i64 = test.label("users.kevin").parse().unwrap();
    let session = test
        .booted
        .app
        .db
        .write(move |tx| {
            campfire_db::Session::start_with(
                tx,
                viewer_id,
                campfire_db::NewSession {
                    two_factor_verified: true,
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    use campfire_kit::Crypto;
    let signed = campfire_kit::RailsCrypto::new(test.booted.app.secrets.clone()).sign_cookie(
        "session_token",
        &session.token,
        None,
    );
    let cookie = format!("session_token={}", campfire_kit::cookies::escape(&signed));
    let mut request = format!("ws://{address}/cable")
        .into_client_request()
        .unwrap();
    request.headers_mut().insert("host", HOST.parse().unwrap());
    request
        .headers_mut()
        .insert("origin", format!("http://{HOST}").parse().unwrap());
    request
        .headers_mut()
        .insert("cookie", cookie.parse().unwrap());
    let (mut socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    assert_eq!(receive(&mut socket).await["type"], "welcome");
    let identifier = json!({"channel":"AgentsChannel"}).to_string();
    socket
        .send(Message::Text(
            json!({"command":"subscribe","identifier":identifier})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    assert_eq!(receive(&mut socket).await["type"], "confirm_subscription");
    let bot: i64 = test.label("users.bender").parse().unwrap();
    let id = test
        .booted
        .app
        .db
        .write(move |tx| {
            let mut agent = campfire_db::Agent::for_user(tx.conn(), bot)?.unwrap();
            agent.update(
                tx,
                campfire_db::AgentChanges {
                    status: Some("working".into()),
                    status_note: Some(Some("Review <this> & continue".into())),
                    ..Default::default()
                },
            )?;
            Ok(agent.id)
        })
        .await
        .unwrap();
    for target in [
        format!("status_badge_agent_{id}"),
        format!("directory_row_agent_{id}"),
    ] {
        let frame = receive_expected(&mut socket, "committed agent status must broadcast both status fragments").await;
        assert_eq!(frame["identifier"], identifier);
        let html = frame["message"].as_str().unwrap();
        assert!(
            html.starts_with(&format!(
                "<turbo-stream action=\"replace\" target=\"{target}\""
            )),
            "{html}"
        );
        assert!(html.contains("Working"));
        assert!(html.contains("Review &lt;this&gt; &amp; continue"));
        for private in [
            "authenticity_token",
            "csrf-token",
            "csp-nonce",
            "bender-test-secret-1234",
            "token_digest",
            "webhook_signing_secret",
        ] {
            assert!(!html.contains(private), "broadcast leaked {private}");
        }
    }
    let error = test
        .booted
        .app
        .db
        .write(move |tx| -> campfire_db::Result<()> {
            let mut agent = campfire_db::Agent::find(tx.conn(), id)?.unwrap();
            agent.update(
                tx,
                campfire_db::AgentChanges {
                    status: Some("failed".into()),
                    ..Default::default()
                },
            )?;
            Err(campfire_db::Error::Other("rollback test".into()))
        })
        .await;
    assert!(error.is_err());
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(250), socket.next())
            .await
            .is_err(),
        "rolled-back callback reached the socket"
    );
    socket.close(None).await.unwrap();
    server.abort();
}

#[tokio::test]
async fn thread_step_callback_renders_ordered_steps_and_updates_over_live_socket() {
    use crate::channels::broadcasts::Stream;
    use campfire_db::models::agent_step::{self, AgentStepChanges, NewAgentStep};
    let test = boot_seed("default").await.expect("default seed");
    let listener = crate::channels::tests::support::bind_listener().await;
    let address = listener.local_addr().unwrap();
    let router = test.booted.router.clone();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let viewer: i64 = test.label("users.david").parse().unwrap();
    let session = test
        .booted
        .app
        .db
        .write(move |tx| {
            campfire_db::Session::start_with(
                tx,
                viewer,
                campfire_db::NewSession {
                    two_factor_verified: true,
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    use campfire_kit::Crypto;
    let signed = campfire_kit::RailsCrypto::new(test.booted.app.secrets.clone()).sign_cookie(
        "session_token",
        &session.token,
        None,
    );
    let mut request = format!("ws://{address}/cable")
        .into_client_request()
        .unwrap();
    request.headers_mut().insert("host", HOST.parse().unwrap());
    request
        .headers_mut()
        .insert("origin", format!("http://{HOST}").parse().unwrap());
    request.headers_mut().insert(
        "cookie",
        format!("session_token={}", campfire_kit::cookies::escape(&signed))
            .parse()
            .unwrap(),
    );
    let (mut socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    assert_eq!(receive(&mut socket).await["type"], "welcome");
    let bot: i64 = test.label("users.bender").parse().unwrap();
    let room: i64 = test.label("rooms.board").parse().unwrap();
    let (agent_id,thread_id)=test.booted.app.db.write(move |tx| {
        let agent=campfire_db::Agent::for_user(tx.conn(),bot)?.unwrap();
        campfire_db::AgentGrant::create(tx,campfire_db::NewGrant{agent_id:agent.id,capability:"manage_threads".into(),room_id:Some(room),granted_by_id:viewer,..Default::default()})?;
        let thread=tx.conn().query_row("SELECT id FROM channel_threads WHERE work_owner_id=? AND room_id=? ORDER BY id LIMIT 1",rusqlite::params![bot,room],|r|r.get::<_,i64>(0))?;
        Ok((agent.id,thread))
    }).await.unwrap();
    let parts = Stream::thread_messages(thread_id);
    let signed =
        rails_compat::turbo::signed_stream_name(&test.booted.app.secrets, &parts.streamables());
    let identifier =
        json!({"channel":"RoomMessagesChannel","signed_stream_name":signed}).to_string();
    socket
        .send(Message::Text(
            json!({"command":"subscribe","identifier":identifier})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    assert_eq!(receive(&mut socket).await["type"], "confirm_subscription");
    let id = test
        .booted
        .app
        .db
        .write(move |tx| {
            let result = agent_step::create(
                tx,
                agent_id,
                NewAgentStep {
                    channel_thread_id: Some(thread_id),
                    name: "Inspect <work>".into(),
                    input_summary: Some("Safe & escaped".into()),
                    ..Default::default()
                },
            )?;
            assert!(result.is_ok(), "{:?}", result.error);
            let next = agent_step::create(
                tx,
                agent_id,
                NewAgentStep {
                    channel_thread_id: Some(thread_id),
                    name: "Finish work".into(),
                    ..Default::default()
                },
            )?;
            assert!(next.is_ok());
            Ok(result.payload.unwrap()["id"].as_i64().unwrap())
        })
        .await
        .unwrap();
    let frame = receive_expected(&mut socket, "committed thread step must broadcast ordered steps").await;
    let html = frame["message"].as_str().unwrap();
    assert!(html.starts_with(&format!(
        "<turbo-stream action=\"replace\" target=\"agent_steps_channel_thread_{thread_id}\""
    )));
    assert!(html.contains("Inspect &lt;work&gt;"));
    assert!(html.contains("Safe &amp; escaped"));
    assert!(!html.contains("authenticity_token"));
    assert!(html.find("Inspect &lt;work&gt;").unwrap() < html.find("Finish work").unwrap());
    assert_eq!(receive(&mut socket).await["message"], frame["message"]);
    test.booted
        .app
        .db
        .write(move |tx| {
            let result = agent_step::update(
                tx,
                agent_id,
                id,
                AgentStepChanges {
                    status: Some("done".into()),
                    duration_ms: Some(Some(1050)),
                    output_summary: Some(Some("Finished <safely>".into())),
                    ..Default::default()
                },
            )?;
            assert!(result.is_ok());
            Ok(())
        })
        .await
        .unwrap();
    let frame = receive(&mut socket).await;
    let html = frame["message"].as_str().unwrap();
    assert!(html.contains("agent-steps__step--done"));
    assert!(html.contains("1.0s"));
    assert!(html.contains("Finished &lt;safely&gt;"));
    let result = test
        .booted
        .app
        .db
        .write(move |tx| -> campfire_db::Result<()> {
            agent_step::update(
                tx,
                agent_id,
                id,
                AgentStepChanges {
                    status: Some("failed".into()),
                    ..Default::default()
                },
            )?;
            Err(campfire_db::Error::Other("rollback step test".into()))
        })
        .await;
    assert!(result.is_err());
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(250), socket.next())
            .await
            .is_err()
    );
    socket.close(None).await.unwrap();
    server.abort();
}
