//! Real socket coverage for WS11 callbacks rendered by WS11-ui after commit.
use super::*;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{Message, client::IntoClientRequest},
};
type Socket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;
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
