//! Production after-commit sink frames observed on an authenticated WebSocket.
use crate::channels::tests::support::{Client, bind_listener};
use crate::controllers::presenters::test_support::{
    ALL_TALK, BENDER, DAVID, SEED_NOW, TestApp, david_cookie,
};
use campfire_db::{Agent, Message, MessageChanges, NewMessage, Room};
use serde_json::{Value, json};
use std::sync::Arc;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
struct Listener(tokio::task::JoinHandle<()>);
impl Drop for Listener {
    fn drop(&mut self) {
        self.0.abort();
    }
}
async fn frame(client: &mut Client, room_identifier: &str, stream: &str) -> Value {
    let envelope: Value = serde_json::from_str(&client.next_text().await).unwrap();
    let name = if envelope["identifier"] == room_identifier {
        stream.to_owned()
    } else {
        format!("user_{DAVID}_unreads")
    };
    json!({"stream":name,"body":envelope["message"]})
}
fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/agents_stream_frames_contract.json"
    ))
    .unwrap()
}
#[tokio::test]
async fn ws11_stream_peer_append_and_rendered_replacements_match_rails() {
    check_frames(&[],oracle()).await;
}

#[tokio::test]
async fn ws11_stream_peer_configured_url_defaults_match_rails() {
    let configured:Value=serde_json::from_str(include_str!("../../../../../vectors/agents_stream_configured_frames_contract.json")).unwrap();
    check_frames(&[("APP_URL","https://campfire.example.test:8443")],configured).await;
}

async fn check_frames(extra: &[(&str,&str)], gold: Value) {
    let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
        SEED_NOW.parse().unwrap(),
    ));
    let (app, _dir) = TestApp::boot_with_clock_and_env(clock,extra)
        .await
        .expect("default seed")
        .stop_jobs()
        .await;
    let listener = bind_listener().await;
    let addr = listener.local_addr().unwrap();
    let router = app.cable.router::<()>("/cable");
    let _listener = Listener(tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    }));
    let mut request = format!("ws://{addr}/cable").into_client_request().unwrap();
    request
        .headers_mut()
        .insert("origin", format!("{}://{addr}",if app.cable.config().assume_ssl {"https"} else {"http"}).parse().unwrap());
    request
        .headers_mut()
        .insert("cookie", david_cookie().parse().unwrap());
    request.headers_mut().insert(
        "sec-websocket-protocol",
        "actioncable-v1-json".parse().unwrap(),
    );
    let (socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    let mut client = Client { socket };
    assert_eq!(client.next_text().await, r#"{"type":"welcome"}"#);
    let room = app.db.read(|c| Room::find(c, ALL_TALK)).await.unwrap();
    let gid = crate::channels::room_gid(&room).to_param();
    let stream = format!("{gid}:messages");
    let signed = rails_compat::turbo::signed_stream_name(&app.secrets, &[&gid, "messages"]);
    let identifier =
        json!({"channel":"RoomMessagesChannel","signed_stream_name":signed}).to_string();
    client.confirm(&identifier).await;
    client
        .confirm(&json!({"channel":"UnreadRoomsChannel"}).to_string())
        .await;
    let mid = app
        .db
        .write(|tx| {
            let agent = Agent::for_user(tx.conn(), BENDER)?.unwrap();
            match campfire_db::models::agent_streaming::start(
                tx,
                agent.id,
                NewMessage {
                    room_id: ALL_TALK,
                    markdown_source: Some("Starting".into()),
                    client_message_id: Some("ws11-stream-frame".into()),
                    ..Default::default()
                },
            )? {
                campfire_db::models::agent_posting::PostResult::Posted(m) => Ok(m.id),
                _ => panic!("allowed stream"),
            }
        })
        .await
        .unwrap();
    assert_eq!(mid, gold["message_id"].as_i64().unwrap());
    assert_eq!(
        vec![frame(&mut client, &identifier, &stream).await],
        gold["start"].as_array().unwrap().clone()
    );
    client.assert_silent().await;
    app.db
        .write(move |tx| {
            let mut m = Message::find(tx.conn(), mid)?;
            m.update(
                tx,
                MessageChanges {
                    markdown_source: Some("Latest draft".into()),
                    ..Default::default()
                },
            )?;
            assert!(campfire_db::models::agent_streaming::broadcast_update(
                tx, &mut m
            )?);
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        vec![frame(&mut client, &identifier, &stream).await],
        gold["update"].as_array().unwrap().clone()
    );
    app.db
        .write(move |tx| {
            assert!(Message::find(tx.conn(), mid)?.finalize_stream(tx)?);
            Ok(())
        })
        .await
        .unwrap();
    let actual = vec![
        frame(&mut client, &identifier, &stream).await,
        frame(&mut client, &identifier, &stream).await,
    ];
    let expected = gold["final"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["stream"] == stream || f["stream"] == format!("user_{DAVID}_unreads"))
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
    app.db
        .write(move |tx| {
            assert!(!Message::find(tx.conn(), mid)?.finalize_stream(tx)?);
            Ok(())
        })
        .await
        .unwrap();
    client.assert_silent().await;
    client.socket.close(None).await.unwrap();
}
