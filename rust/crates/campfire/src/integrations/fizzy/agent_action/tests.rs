use super::*;
use crate::integrations::test_support::{FakeResolver, FakeServer, MappingDialer, Route, network};
use std::sync::Arc;
#[test]
fn ws15e_fizzy_agent_action_matches_pinned_rails() {
    // JSON.parse defaults to max_nesting=100, the same pinned boundary as Client.
    for depth in [98, 99, 100] {
        let text=format!("{{\"kind\":\"comment\",\"extra\":{}0{}}}","[".repeat(depth),"]".repeat(depth));
        assert_eq!(Action::from_stored(&text).is_some(),depth<100);
    }
    let vectors: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vectors/ws15e_fizzy_agent_action.json"
    )))
    .unwrap();
    for vector in vectors["actions"].as_array().unwrap() {
        let action = Action::from_payload(vector["input"].clone());
        let errors = action.errors();
        let mut actual = serde_json::Map::<String, Value>::new();
        for (key, message) in &errors.0 {
            actual
                .entry(*key)
                .or_insert(json!([]))
                .as_array_mut()
                .unwrap()
                .push(json!(message));
        }
        assert_eq!(errors.is_empty(), vector["valid"].as_bool().unwrap());
        assert_eq!(json!(actual), vector["errors"]);
        assert_eq!(action.action_name(), vector["action"].as_str().unwrap());
        assert_eq!(json!(action.summary()), vector["summary"]);
        assert_eq!(
            action.payload_json(),
            vector["payload_json"].as_str().unwrap()
        );
        let payload: Value = serde_json::from_str(&action.payload_json()).unwrap();
        assert_eq!(payload, vector["payload"]);
        let rebuilt = Action::from_stored(&action.payload_json()).unwrap();
        assert_eq!(rebuilt.summary(), action.summary());
    }
}
#[tokio::test]
async fn ws15e_fizzy_agent_action_dispatches_all_five_writes() {
    let server = FakeServer::start(vec![
        Route::new("POST", "app.fizzy.do", "/acc/boards/board/cards.json", 201)
            .body("{\"number\":580}"),
        Route::new("POST", "app.fizzy.do", "/acc/cards/579/comments.json", 201)
            .body("{\"url\":\"https://app.fizzy.do/comment\"}"),
        Route::new("POST", "app.fizzy.do", "/acc/cards/579/triage.json", 204),
        Route::new("POST", "app.fizzy.do", "/acc/cards/579/closure.json", 204),
        Route::new("DELETE", "app.fizzy.do", "/acc/cards/579/closure.json", 204),
    ])
    .await;
    let resolver = Arc::new(FakeResolver::new([("app.fizzy.do", vec!["93.184.216.34"])]));
    let dialer = Arc::new(MappingDialer {
        public: ["93.184.216.34".parse().unwrap()].into(),
        to: server.addr,
        dialed: Default::default(),
    });
    let client = Client::new(
        network(resolver.clone(), dialer),
        "owner-token".into(),
        "http://app.fizzy.do",
    );
    for kind in ["create", "comment", "move", "close", "reopen"] {
        let action = Action::from_payload(
            json!({"account_id":"acc","kind":kind,"board_id":"board","number":579,"column_id":"col","title":" T ","body":" Nice "}),
        );
        assert!(action.errors().is_empty());
        let value = action.perform(&client).await.unwrap();
        match kind {
            "create" => assert_eq!(value["number"], 580),
            "comment" => assert_eq!(value["url"], "https://app.fizzy.do/comment"),
            _ => assert_eq!(value, true),
        }
    }
    assert_eq!(server.received.lock().unwrap().len(), 5);
    assert_eq!(resolver.lookups().len(), 5);
}
