//! MCP reactions: raw Rails bytes, shortcode replay, and current permissions.
use super::presenters::test_support::TestApp;
use crate::channels::tests::support::{Client, bind_listener, identifier};
use serde_json::Value;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

pub(super) struct Server(tokio::task::JoinHandle<()>);
impl Drop for Server {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub(super) async fn subscribe(app: &TestApp) -> (Server, Client) {
    let listener = bind_listener().await;
    let address = listener.local_addr().unwrap();
    let router = app.booted.router.clone();
    let server = Server(tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    }));
    let mut request = format!("ws://{address}/cable")
        .into_client_request()
        .unwrap();
    request
        .headers_mut()
        .insert("origin", format!("http://{address}").parse().unwrap());
    request.headers_mut().insert("cookie", {
        let sessions: Value =
            serde_json::from_str(include_str!("../../../../vectors/campfire_sessions.json"))
                .unwrap();
        sessions["sessions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|session| session["user_name"] == "Jason")
            .unwrap()["cookie_header"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap()
    });
    request.headers_mut().insert(
        "sec-websocket-protocol",
        "actioncable-v1-json".parse().unwrap(),
    );
    let (socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    let mut client = Client { socket };
    assert_eq!(client.next_text().await, r#"{"type":"welcome"}"#);
    let gid = crate::channels::room_gid(
        &app.db()
            .read(|conn| campfire_db::Room::find(conn, 486777696))
            .await
            .unwrap(),
    )
    .to_param();
    let signed =
        rails_compat::turbo::signed_stream_name(&app.booted.app.secrets, &[&gid, "messages"]);
    client
        .confirm(&identifier(
            serde_json::json!({"channel":"RoomMessagesChannel","signed_stream_name":signed}),
        ))
        .await;
    (server, client)
}

#[tokio::test]
async fn agent_reactions_wire_bytes() {
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agent_reactions_http.json"
    ))
    .unwrap();
    for case in vectors["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| !case["setup"]["transition"].is_string())
    {
        super::agent_reads_tests::check(case).await;
    }
}

#[tokio::test]
async fn agent_bot_reactions_wire_bytes() {
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agent_bot_reactions_http.json"
    ))
    .unwrap();
    for case in vectors["cases"].as_array().unwrap() {
        super::agent_reads_tests::check(case).await;
    }
}

#[tokio::test]
async fn agent_reaction_permission_transitions() {
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agent_reactions_http.json"
    ))
    .unwrap();
    let cases: Vec<_> = vectors["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| case["setup"]["transition"].is_string())
        .collect();
    assert_eq!(
        cases.len(),
        7,
        "live permission transitions must be captured and asserted"
    );
    for case in cases {
        super::agent_reads_tests::check(case).await;
    }
}
