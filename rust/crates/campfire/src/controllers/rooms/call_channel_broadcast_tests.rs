use super::call_channel_tests::configured;
use crate::controllers::presenters::test_support::{DAVID, JASON, KEVIN, Req, TestApp};
use axum::http::Method;
use campfire_kit::Crypto;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::time::Duration;
use tokio_tungstenite::tungstenite::{Message, client::IntoClientRequest};
pub(super) type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;
pub(super) async fn next(socket: &mut Socket) -> Value {
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
pub(super) async fn socket(test: &TestApp, addr: std::net::SocketAddr, user: i64) -> Socket {
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
    let stream = crate::channels::broadcasts::Stream::user_rooms(user);
    let identifier=json!({"channel":"Turbo::StreamsChannel","signed_stream_name":rails_compat::turbo::signed_stream_name(&test.booted.app.secrets,&stream.streamables())}).to_string();
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
#[tokio::test]
async fn call_channel_create_and_member_revision_deliver_ordered_sidebar_and_header_frames() {
    for namespace in ["voices", "stages"] {
        let Some(test) = TestApp::boot_with_huddle(configured()).await else {
            return;
        };
        let listener = crate::channels::tests::support::bind_listener().await;
        let addr = listener.local_addr().unwrap();
        let (stop, stopping) = tokio::sync::oneshot::channel();
        let router = test.booted.router.clone();
        let serving = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async {
                    let _ = stopping.await;
                })
                .await
                .unwrap()
        });
        let mut admin = socket(&test, addr, DAVID).await;
        let mut removed = socket(&test, addr, JASON).await;
        let mut remaining = socket(&test, addr, KEVIN).await;
        let mut browser = test.sign_in(DAVID).await;
        let reply = browser
            .write(
                Req::new(Method::POST, &format!("/rooms/{namespace}")).form(&[
                    ("room[name]", "Lounge"),
                    ("user_ids[]", &DAVID.to_string()),
                    ("user_ids[]", &JASON.to_string()),
                    ("user_ids[]", &KEVIN.to_string()),
                ]),
            )
            .await;
        assert_eq!(reply.status, axum::http::StatusCode::FOUND);
        let id = reply
            .location()
            .unwrap()
            .rsplit('/')
            .next()
            .unwrap()
            .parse::<i64>()
            .unwrap();
        for client in [&mut admin, &mut removed, &mut remaining] {
            let frame = next(client).await;
            let html = frame["message"].as_str().unwrap();
            assert!(
                html.starts_with(&format!(
                    "<turbo-stream action=\"prepend\" target=\"{}_rooms\">",
                    if namespace == "voices" {
                        "voice"
                    } else {
                        "stage"
                    }
                )),
                "{namespace}: {html}"
            );
            assert!(html.contains("Lounge"));
            assert!(html.contains("voice-room"));
            assert!(campfire_cable::turbo::session_bound(html).is_none());
        }
        let reply = browser
            .write(
                Req::new(Method::PUT, &format!("/rooms/{namespace}/{id}")).form(&[
                    ("room[name]", "After"),
                    ("user_ids[]", &DAVID.to_string()),
                    ("user_ids[]", &KEVIN.to_string()),
                ]),
            )
            .await;
        assert_eq!(reply.status, axum::http::StatusCode::FOUND);
        let key = if namespace == "voices" {
            "rooms_voice"
        } else {
            "rooms_stage"
        };
        for prefix in ["header_voice_participants", "list"] {
            assert_eq!(
                next(&mut removed).await["message"],
                format!(
                    "<turbo-stream action=\"remove\" target=\"{prefix}_{key}_{id}\"></turbo-stream>"
                )
            );
        }
        for client in [&mut admin, &mut remaining] {
            for prefix in ["list", "header"] {
                let frame = next(client).await;
                let html = frame["message"].as_str().unwrap();
                assert!(
                    html.starts_with(&format!(
                        "<turbo-stream action=\"replace\" target=\"{prefix}_{key}_{id}\">"
                    )),
                    "{namespace}: {html}"
                );
                assert!(html.contains("After"));
                assert!(campfire_cable::turbo::session_bound(html).is_none());
            }
        }
        for client in [&mut admin, &mut removed, &mut remaining] {
            client.close(None).await.unwrap();
        }
        stop.send(()).unwrap();
        serving.await.unwrap();
        test.booted.jobs.shutdown(Duration::from_secs(2)).await;
    }
}
