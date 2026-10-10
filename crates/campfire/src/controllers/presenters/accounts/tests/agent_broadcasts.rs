//! Real socket coverage for WS11 callbacks rendered by WS11-ui after commit.
use super::*;
use futures_util::{SinkExt, Stream, StreamExt};
use serde_json::{Value, json};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{Message, client::IntoClientRequest},
};
type Socket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

pub(super) async fn human_socket(
    test: &Test,
    address: std::net::SocketAddr,
    viewer: i64,
) -> Socket {
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
    socket
}

pub(super) async fn activity_socket(
    test: &Test,
    address: std::net::SocketAddr,
    viewer: i64,
) -> Socket {
    let mut socket = human_socket(test, address, viewer).await;
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
        let created = receive_expected(
            &mut owner_socket,
            "committed approval must broadcast its activity item",
        )
        .await;
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

/// Silence means no application broadcasts within the original fixed deadline.
/// Protocol heartbeats are harmless; closed/broken/malformed transports are invalid.
pub(super) async fn next_broadcast(socket: &mut Socket) -> Result<Option<Value>, String> {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(250);
    loop {
        let frame = match tokio::time::timeout_at(deadline, socket.next()).await {
            Err(_) => return Ok(None),
            Ok(Some(Ok(frame))) => frame,
            Ok(Some(Err(error))) => return Err(format!("cable silence transport failed: {error}")),
            Ok(None) => return Err("cable silence transport ended".into()),
        };
        match frame {
            Message::Text(text) => {
                let value: Value = serde_json::from_str(&text)
                    .map_err(|error| format!("cable silence invalid JSON: {error}"))?;
                if value["type"] == "ping" {
                    eprintln!("Cable silence: ignored ActionCable heartbeat");
                    continue;
                }
                if value["identifier"].is_string() && !value["message"].is_null() {
                    return Ok(Some(value));
                }
                return Err(format!("cable silence invalid control frame: {value}"));
            }
            Message::Ping(_) | Message::Pong(_) => continue,
            other => return Err(format!("cable silence invalid transport frame: {other:?}")),
        }
    }
}



#[tokio::test]
async fn working_presence_silence_transport_failure_is_invalid() {
    let test = boot_seed("default").await.expect("default seed");
    let listener = crate::channels::tests::support::bind_listener().await;
    let address = listener.local_addr().unwrap();
    let router = test.booted.router.clone();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut socket = human_socket(&test, address, test.label("users.david").parse().unwrap()).await;
    socket.close(None).await.unwrap();
    assert!(
        next_broadcast(&mut socket).await.is_err(),
        "a closed transport is invalid, not a status callback"
    );
    server.abort();
}

pub(super) async fn receive<S, E>(socket: &mut S) -> Value
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
