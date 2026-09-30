use crate::controllers::presenters::test_support::{
    ALL_TALK, DAVID, SEED_NOW, TestApp, david_cookie,
};
use campfire_db::models::huddle_grant::HuddleGrant;
use campfire_db::models::room_delete::HuddleConfig;
use campfire_db::{Membership, Session};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tokio_tungstenite::tungstenite::{Message, client::IntoClientRequest};

type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

async fn next(socket: &mut Socket) -> Value {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let message = socket.next().await.expect("socket closed").unwrap();
            if let Message::Text(text) = message {
                let value: Value = serde_json::from_str(&text).unwrap();
                if value["type"] != "ping" {
                    return value;
                }
            }
        }
    })
    .await
    .expect("broadcast did not reach the socket")
}

async fn subscribe(socket: &mut Socket, channel: &str, signed: String) {
    let identifier = json!({"channel":channel, "signed_stream_name":signed}).to_string();
    socket
        .send(Message::Text(
            json!({"command":"subscribe", "identifier": identifier})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    assert_eq!(next(socket).await["type"], "confirm_subscription");
}

#[tokio::test]
async fn huddle_presence_reaches_real_sockets_and_rolled_back_revocation_stays_silent() {
    let config = crate::huddle::Config::from_lookup(|name| {
        Some(
            match name {
                "LIVEKIT_URL" => "wss://public.example.test",
                "LIVEKIT_INTERNAL_URL" => "http://internal.example.test:7880",
                _ => "ws13-fixture-value",
            }
            .into(),
        )
    });
    let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
        SEED_NOW.parse().unwrap(),
    ));
    let Some(test) = TestApp::boot_with_huddle_and_clock(config, clock).await else {
        return;
    };
    let app = test.booted.app.clone();
    let grant = app
        .db
        .write(|tx| {
            let session = Session::start(tx, DAVID, None, None)?;
            let membership =
                Membership::find_by_room_and_user(tx.conn(), ALL_TALK, DAVID)?.unwrap();
            HuddleGrant::issue(
                tx,
                session.id,
                membership.id,
                ALL_TALK,
                &HuddleConfig {
                    api_secret: Some("ws13-fixture-value".into()),
                    admin_configured: false,
                },
            )
        })
        .await
        .unwrap();
    let listener = super::tests::support::bind_listener().await;
    let addr = listener.local_addr().unwrap();
    let (stop, stopping) = tokio::sync::oneshot::channel::<()>();
    let router = test.booted.router.clone();
    let serving = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = stopping.await;
            })
            .await
            .unwrap()
    });
    let mut request = format!("ws://{addr}/cable").into_client_request().unwrap();
    request
        .headers_mut()
        .insert("origin", format!("http://{addr}").parse().unwrap());
    request
        .headers_mut()
        .insert("cookie", david_cookie().parse().unwrap());
    request.headers_mut().insert(
        "sec-websocket-protocol",
        "actioncable-v1-json".parse().unwrap(),
    );
    let (mut socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    assert_eq!(next(&mut socket).await["type"], "welcome");
    let room = app
        .db
        .read(|conn| campfire_db::Room::find(conn, ALL_TALK))
        .await
        .unwrap();
    for (channel, stream) in [
        (
            "Turbo::StreamsChannel",
            super::broadcasts::Stream::user_rooms(DAVID),
        ),
        (
            "RoomMessagesChannel",
            super::broadcasts::Stream::room_messages(&room),
        ),
    ] {
        subscribe(
            &mut socket,
            channel,
            rails_compat::turbo::signed_stream_name(&app.secrets, &stream.streamables()),
        )
        .await;
    }
    let id = grant.id;
    app.db
        .write(move |tx| {
            HuddleGrant::find_by_id(tx.conn(), id)?
                .unwrap()
                .record_seen(tx)
        })
        .await
        .unwrap();
    for placement in ["sidebar", "header"] {
        let frame = next(&mut socket).await;
        let html = frame["message"].as_str().unwrap();
        assert!(
            html.contains(&format!(
                "{placement}_voice_participants_rooms_closed_{ALL_TALK}"
            )),
            "{html}"
        );
        assert!(html.contains("1 in huddle: David"), "{html}");
        assert!(campfire_cable::turbo::session_bound(html).is_none());
        assert!(!campfire_views::helpers::request_forgery::has_token_slots(
            html
        ));
    }
    let rolled_back = app
        .db
        .write(move |tx| -> campfire_db::Result<()> {
            HuddleGrant::find_by_id(tx.conn(), id)?.unwrap().revoke(
                tx,
                false,
                &HuddleConfig::default(),
            )?;
            Err(campfire_db::Error::Other(
                "WS13 intentional rollback".into(),
            ))
        })
        .await;
    assert!(rolled_back.is_err());
    assert!(
        !app.db
            .read(move |conn| Ok(HuddleGrant::find_by_id(conn, id)?.unwrap().revoked()))
            .await
            .unwrap()
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(150), socket.next())
            .await
            .is_err(),
        "rolled-back presence escaped"
    );
    app.db
        .write(move |tx| {
            HuddleGrant::find_by_id(tx.conn(), id)?.unwrap().revoke(
                tx,
                false,
                &HuddleConfig::default(),
            )
        })
        .await
        .unwrap();
    for _ in 0..2 {
        let frame = next(&mut socket).await;
        let html = frame["message"].as_str().unwrap();
        assert!(html.contains("Nobody in huddle"), "{html}");
        assert!(!html.contains("voice-stack--live"), "{html}");
    }
    socket.close(None).await.unwrap();
    test.booted.jobs.shutdown(Duration::from_secs(2)).await;
    let _ = stop.send(());
    tokio::time::timeout(Duration::from_secs(3), serving)
        .await
        .unwrap()
        .unwrap();
}
