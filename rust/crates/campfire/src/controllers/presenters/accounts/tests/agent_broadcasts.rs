//! Real socket coverage for WS11 callbacks rendered by WS11-ui after commit.
use super::*;
use futures_util::{SinkExt, StreamExt};
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
    for rollback in [false, true] {
        let (approval_id,item_id) = test.booted.app.db.write(move |tx| {
            let approval = AgentApproval::create(tx,NewApproval {
                agent_id,action:"deploy".into(),summary:"Private approval summary".into(),
                ..Default::default()
            })?;
            let item = campfire_db::ActivityItem::find_by_user_and_source(tx.conn(),owner_id,"AgentApproval",approval.id)?.unwrap();
            if rollback {
                tx.conn().execute_batch("CREATE TRIGGER reject_live_decision_audit BEFORE INSERT ON audit_logs WHEN NEW.action='agent.approval.decide' BEGIN SELECT RAISE(ABORT,'test audit rejection'); END;")?;
            }
            Ok((approval.id,item.id))
        }).await.unwrap();
        let created = receive(&mut owner_socket).await;
        assert_eq!(created["message"], json!({"activityItemId":item_id}));
        let response = owner
            .form(
                "patch",
                &format!("/agent_approvals/{approval_id}.json"),
                &[("decision", "denied"), ("note", "Private decision note")],
            )
            .await;
        if rollback {
            assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
            test.booted
                .app
                .db
                .read(move |conn| {
                    assert_eq!(
                        AgentApproval::find(conn, approval_id)?.unwrap().status,
                        "pending"
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
                        .is_none()
                    );
                    Ok(())
                })
                .await
                .unwrap();
        } else {
            assert_eq!(response.status, StatusCode::OK, "{}", response.text());
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
        }
        for socket in [&mut owner_socket, &mut other_socket] {
            assert!(
                tokio::time::timeout(std::time::Duration::from_millis(150), socket.next())
                    .await
                    .is_err(),
                "duplicate, rolled-back or another user's activity reached the socket"
            );
        }
    }
    owner_socket.close(None).await.unwrap();
    other_socket.close(None).await.unwrap();
    server.abort();
}
async fn receive(socket: &mut Socket) -> Value {
    loop {
        let frame = tokio::time::timeout(std::time::Duration::from_secs(2), socket.next())
            .await
            .expect("cable frame before timeout")
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
        let frame = receive(&mut socket).await;
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
    let frame = receive(&mut socket).await;
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
