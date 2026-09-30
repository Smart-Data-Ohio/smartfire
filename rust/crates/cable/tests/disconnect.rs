//! The review's concurrent-publisher probe: the socket task runs on a separate runtime, so
//! disconnect can arrive while it is collecting an ordinary delivery batch.
mod support;

use campfire_cable::Config;
use serde_json::json;
use support::{Frame, identifier, start, test_config};

#[tokio::test]
async fn concurrent_cutoff_drops_later_publications() {
    let app = start(Config {
        stream_capacity: 512,
        max_write_batch: 64,
        ..test_config()
    })
    .await;
    let room = identifier(json!({ "channel": "RoomChannel", "room_id": 1 }));
    // Repeated real sockets exercise selection, batching and writing concurrently with the
    // publisher. No callback holds the connection until all publications are ready.
    for iteration in 0..1000 {
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            let mut client = app.connect(1).await;
            client.next_text().await;
            client.subscribe(&room).await;
            client.next_text().await;
            for i in 0..120 {
                app.server.broadcast("room:room-1", &json!({ "before": i }));
            }
            let reconnect = iteration % 2 == 0;
            assert_eq!(app.server.disconnect("user-1", reconnect), 1);
            for i in 0..64 {
                app.server.broadcast("room:room-1", &json!({ "after": i }));
            }
            let mut before = 0;
            loop {
                match client.next().await {
                    Frame::Text(text) => {
                        assert!(!text.contains("after"), "iteration={iteration}, after-cutoff delivery={text}, before_seen={before}");
                        if text.contains("disconnect") {
                            assert_eq!(text, format!(r#"{{"type":"disconnect","reason":"remote","reconnect":{reconnect}}}"#));
                            break;
                        }
                        let frame: serde_json::Value = serde_json::from_str(&text).unwrap();
                        assert_eq!(frame["identifier"], room);
                        assert_eq!(frame["message"]["before"], before, "iteration={iteration}");
                        before += 1;
                    }
                    other => panic!("unexpected {other:?}"),
                }
            }
            assert_eq!(before, 120, "iteration={iteration}");
            assert_eq!(client.next().await, Frame::Close(Some((1000, String::new()))));
            assert!(matches!(client.next().await, Frame::End | Frame::Error(_)));
        })
        .await
        .unwrap_or_else(|_| panic!("connection {iteration} did not close within 10s"));
    }
}
