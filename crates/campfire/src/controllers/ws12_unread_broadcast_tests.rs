//! Real REST/MCP board creation must reach UnreadRoomsChannel's Rails stream.
use super::presenters::test_support::JASON;
use crate::channels::tests::support::identifier;
use serde_json::{Value, json};

#[tokio::test]
async fn ws12_agent_board_creation_reaches_the_rails_unread_stream_over_both_transports() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agent_work_writes_http.json"
    ))
    .unwrap();
    for name in ["rest_create_queries_5", "mcp_create_queries_5"] {
        let case = corpus["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == name)
            .unwrap();
        let app = super::agent_reads_tests::prepare(case).await;
        let (_server, mut socket) = subscribe(&app).await;
        let unread = identifier(json!({"channel":"UnreadRoomsChannel"}));
        socket.confirm(&unread).await;
        let reply = app
            .anonymous()
            .send(super::agent_reads_tests::request(case))
            .await;
        assert_eq!(
            reply.status.as_u16(),
            case["status"].as_u64().unwrap() as u16
        );
        let frame = tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                let frame: Value = serde_json::from_str(&socket.next_text().await).unwrap();
                if frame["identifier"] == unread {
                    break frame;
                }
            }
        })
        .await
        .expect("board create must publish on UnreadRoomsChannel's stream");
        assert_eq!(frame["message"], json!({"roomId":486777696}));
        assert_eq!(
            campfire_db::broadcasts::unread_rooms_stream_name(JASON),
            format!("user_{JASON}_unreads")
        );
        println!(
            "WS12_UNREAD_STREAM {name}: user_{JASON}_unreads; complete payload agrees with Rails"
        );
    }
}

struct Listener(tokio::task::JoinHandle<()>);
impl Drop for Listener {
    fn drop(&mut self) {
        self.0.abort();
    }
}
async fn subscribe(
    app: &super::presenters::test_support::TestApp,
) -> (Listener, crate::channels::tests::support::Client) {
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let listener = crate::channels::tests::support::bind_listener().await;
    let address = listener.local_addr().unwrap();
    let router = app.booted.router.clone();
    let server = Listener(tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    }));
    let mut request = format!("ws://{address}/cable")
        .into_client_request()
        .unwrap();
    request
        .headers_mut()
        .insert("origin", format!("http://{address}").parse().unwrap());
    let sessions: Value =
        serde_json::from_str(include_str!("../../../../vectors/campfire_sessions.json")).unwrap();
    let session = sessions["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["user_name"] == "Jason")
        .unwrap();
    request.headers_mut().insert(
        "cookie",
        session["cookie_header"].as_str().unwrap().parse().unwrap(),
    );
    request.headers_mut().insert(
        "sec-websocket-protocol",
        "actioncable-v1-json".parse().unwrap(),
    );
    let (socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    let mut client = crate::channels::tests::support::Client { socket };
    assert_eq!(client.next_text().await, r#"{"type":"welcome"}"#);
    (server, client)
}
