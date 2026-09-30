use super::agent_http_tests::{AGENT, SECRET, setup};
use super::presenters::test_support::{Req, TestApp};
use campfire_kit::Method;
use serde_json::{Value, json};

async fn request(app: &TestApp, case: &Value) -> super::presenters::test_support::Reply {
    let mut req = Req::new(
        Method::from_bytes(case["method"].as_str().unwrap().to_uppercase().as_bytes()).unwrap(),
        "/agents/mcp",
    )
    .header("authorization", &["Bearer", SECRET].join(" "))
    .header("accept", "application/json")
    .header(
        "content-type",
        case["headers"]["Content-Type"]
            .as_str()
            .unwrap_or("application/json"),
    )
    .body(case["body"].as_str().unwrap());
    for (name, value) in case["headers"].as_object().unwrap() {
        if name.eq_ignore_ascii_case("content-type") {
            continue;
        }
        req = req.header(name, value.as_str().unwrap());
    }
    app.anonymous().send(req).await
}
async fn check(names: &[&str]) {
    let app = setup().await;
    app.db()
        .write(|tx| {
            tx.conn()
                .execute("DELETE FROM agent_events WHERE agent_id=?", [AGENT])
                .map(|_| ())
                .map_err(Into::into)
        })
        .await
        .unwrap();
    let vectors: Value =
        serde_json::from_str(include_str!("../../../../vectors/agent_mcp.json")).unwrap();
    for case in vectors["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| names.contains(&c["name"].as_str().unwrap()))
    {
        let name = case["name"].as_str().unwrap();
        let reply = request(&app, case).await;
        assert_eq!(
            reply.status.as_u16(),
            case["status"].as_u64().unwrap() as u16,
            "{name}: {}",
            reply.text()
        );
        let body = if reply.body.is_empty() {
            Value::Null
        } else {
            reply.json()
        };
        assert_eq!(body, case["response"], "{name}");
        for (header, expected) in case["response_headers"].as_object().unwrap() {
            assert_eq!(reply.header(header), expected.as_str(), "{name}: {header}");
        }
    }
}
#[tokio::test]
async fn mcp_version_and_header_security() {
    check(&[
        "unsupported",
        "unsupported_meta_type",
        "version_mismatch",
        "missing_method_header",
        "method_mismatch",
        "missing_name",
        "name_mismatch",
        "bad_base64",
        "bad_utf8_base64",
    ])
    .await;
}
#[tokio::test]
async fn mcp_origin_security() {
    check(&[
        "origin_denied",
        "origin_allowed",
        "origin_malformed",
        "origin_encoded_host",
        "origin_scheme_relative",
    ])
    .await;
}

#[tokio::test]
async fn mcp_presence_coercions() {
    check(&["presence_object", "presence_clear"]).await;
}

#[tokio::test]
async fn mcp_retains_the_kit_body_bound() {
    let app = setup().await;
    let body = " ".repeat(campfire_kit::body::MAX_BUFFERED_BODY + 1);
    let case = json!({"method":"post","body":body,"headers":{}});
    assert_eq!(request(&app, &case).await.status.as_u16(), 413);
}
#[tokio::test]
async fn mcp_protocol_vectors() {
    check(&[
        "initialize_2026-07-28",
        "initialize_2025-11-25",
        "initialize_2025-06-18",
        "initialize_2025-03-26",
        "ping_2026-07-28",
        "ping_2025-11-25",
        "ping_2025-06-18",
        "ping_2025-03-26",
        "discover_modern",
        "discover_legacy",
        "tools_list",
        "get_405",
        "delete_405",
        "parse_error",
        "text_parse_error",
        "batch",
        "null",
        "missing_jsonrpc",
        "wrong_method_type",
        "notification",
        "identified_notification",
        "unknown_method",
        "unknown_tool",
        "bad_arguments",
        "base64_name",
        "poll_empty",
        "ack_empty",
        "ack_too_many",
        "ack_missing",
        "internal_error",
        "tool_empty_poll_events",
        "tool_empty_ack_events",
        "tool_empty_set_presence",
        "tool_empty_add_step",
        "tool_empty_update_step",
        "tool_empty_register_slash_command",
        "tool_empty_unregister_slash_command",
    ])
    .await;
}
#[tokio::test]
async fn mcp_shares_rest_buckets_and_counts_endpoint_before_origin() {
    let app = setup().await;
    let mut case = json!({"method":"post","body":json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"poll_events","arguments":{}}}).to_string(),"headers":{}});
    for _ in 0..120 {
        let reply = app
            .anonymous()
            .send(
                Req::new(Method::GET, "/agents/events")
                    .header("authorization", &["Bearer", SECRET].join(" ")),
            )
            .await;
        assert_eq!(reply.status.as_u16(), 200);
    }
    let reply = request(&app, &case).await;
    assert_eq!(reply.status.as_u16(), 200);
    assert_eq!(
        reply.json()["result"]["structuredContent"]["error"],
        "rate_limited"
    );
    assert_eq!(reply.json()["result"]["isError"], true);
    assert!(reply.header("retry-after").is_some());
    case["headers"] = json!({"Origin":"https://attacker.invalid"});
    for _ in 0..599 {
        assert_eq!(request(&app, &case).await.status.as_u16(), 403);
    }
    let overflow = request(&app, &case).await;
    assert_eq!(overflow.status.as_u16(), 429);
    assert_eq!(overflow.json(), json!({"error":"rate_limited"}));
}
