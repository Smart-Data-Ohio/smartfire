use crate::controllers::presenters::test_support::{TestApp};
use campfire_kit::Crypto;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::time::Duration;
use tokio_tungstenite::tungstenite::{Message, client::IntoClientRequest};
pub(in crate::controllers) type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;
pub(in crate::controllers) async fn next(socket: &mut Socket) -> Value {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let Message::Text(text) = socket.next().await.expect("closed socket").unwrap() {
                let value: Value = serde_json::from_str(&text).unwrap();
                if value["type"] != "ping" {
                    return value;
                }
            }
        }
    })
    .await
    .expect("missing controller broadcast")
}
pub(in crate::controllers) async fn socket(test: &TestApp, addr: std::net::SocketAddr, user: i64) -> Socket {
    let session = test
        .db()
        .write(move |tx| {
            campfire_db::Session::start_with(
                tx,
                user,
                campfire_db::NewSession {
                    user_agent: None,
                    ip_address: None,
                    device_id: None,
                    two_factor_verified: true,
                },
            )
        })
        .await
        .unwrap();
    let cookie = campfire_kit::RailsCrypto::new(test.booted.app.secrets.clone()).sign_cookie(
        "session_token",
        &session.token,
        None,
    );
    let mut request = format!("ws://{addr}/cable").into_client_request().unwrap();
    request
        .headers_mut()
        .insert("origin", format!("http://{addr}").parse().unwrap());
    request.headers_mut().insert(
        "cookie",
        format!("session_token={}", campfire_kit::cookies::escape(&cookie))
            .parse()
            .unwrap(),
    );
    request.headers_mut().insert(
        "sec-websocket-protocol",
        "actioncable-v1-json".parse().unwrap(),
    );
    let (mut socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    assert_eq!(next(&mut socket).await["type"], "welcome");
    let identifier=json!({"channel":"ReadRoomsChannel"}).to_string();
    socket
        .send(Message::Text(
            json!({"command":"subscribe","identifier":identifier})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    assert_eq!(next(&mut socket).await["type"], "confirm_subscription");
    socket
}
