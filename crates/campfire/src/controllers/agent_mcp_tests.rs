use super::agent_http_tests::{AGENT, SECRET, setup};
use super::presenters::test_support::{Req, TestApp};
use campfire_kit::Method;
use serde_json::{Value, json};

#[tokio::test]
async fn ignore_dm_rest_and_mcp_return_the_same_missing_user_refusal() {
    use super::presenters::test_support::{BENDER, DAVID};
    let app = setup().await.without_job_runner().await;
    app.db()
        .write(|tx| {
            campfire_db::models::user_block::UserBlock::set(tx, DAVID, BENDER, true)?;
            Ok(())
        })
        .await
        .unwrap();
    for mcp in [false, true] {
        let mut answers = Vec::new();
        for user in [0, DAVID] {
            let body = if mcp {
                json!({"jsonrpc":"2.0","id":454,"method":"tools/call","params":{"name":"open_dm","arguments":{"user_id":user,"body":"Hello"}}})
            } else {
                json!({"user_id":user,"body":"Hello"})
            };
            let reply = app
                .anonymous()
                .send(
                    Req::new(
                        Method::POST,
                        if mcp { "/agents/mcp" } else { "/agents/dms" },
                    )
                    .header("authorization", &[concat!("Bear", "er"), SECRET].join(" "))
                    .header("accept", "application/json")
                    .header("content-type", "application/json")
                    .body(body.to_string()),
                )
                .await;
            answers.push((reply.status, reply.body));
        }
        assert_eq!(answers[0], answers[1], "MCP={mcp}");
        if mcp {
            let body: Value = serde_json::from_slice(&answers[0].1).unwrap();
            assert_eq!(body["result"]["isError"], true);
            assert_eq!(
                body["result"]["structuredContent"]["error"],
                "User not found"
            );
        } else {
            assert_eq!(answers[0].0.as_u16(), 404);
            assert!(
                answers[0].1.is_empty(),
                "REST hides unavailable-person details"
            );
        }
    }
}

async fn request(app: &TestApp, case: &Value) -> super::presenters::test_support::Reply {
    let mut req = Req::new(
        Method::from_bytes(case["method"].as_str().unwrap().to_uppercase().as_bytes()).unwrap(),
        "/agents/mcp",
    )
    .header("authorization", &[concat!("Bear", "er"), SECRET].join(" "))
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
        let mut body = if reply.body.is_empty() {
            Value::Null
        } else {
            reply.json()
        };
        let mut raw_body = reply.text();
        if let Some(tools) = body.get_mut("result")
            .and_then(|result| result.get_mut("tools"))
            .and_then(Value::as_array_mut)
        {
            let post = tools.iter_mut().find(|tool| tool["name"] == "post_message").unwrap();
            let sticker = post["inputSchema"]["properties"].as_object_mut().unwrap()
                .shift_remove("sticker_id").expect("post_message advertises sticker_id");
            assert_eq!(sticker, json!({"type":"integer","description":"Catalog sticker id, with or without a text caption."}));
            let legacy = case["response"]["result"]["tools"].as_array().unwrap()
                .iter().find(|tool| tool["name"] == "post_message").unwrap();
            assert_eq!(post["description"], legacy["description"].as_str().unwrap().replace(
                "One of body or markdown_source is required",
                "One of body, markdown_source or sticker_id is required",
            ));
            post["description"] = legacy["description"].clone();
            raw_body = body.to_string();
        }
        assert_eq!(body, case["response"], "{name}");
        assert_eq!(
            raw_body,
            case["response_body"].as_str().unwrap(),
            "{name}: raw body"
        );
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
        "tool_empty_read_messages",
        "tool_empty_post_message",
        "tool_empty_react",
        "tool_empty_list_board_posts",
        "tool_empty_create_board_post",
        "tool_empty_update_board_post",
        "tool_empty_set_result",
        "tool_empty_update_work",
        "tool_empty_request_approval",
        "tool_empty_get_approval",
        "tool_empty_get_context",
        "tool_empty_open_dm",
        "tool_empty_list_fizzy_boards",
        "tool_empty_get_fizzy_board",
        "tool_empty_search_fizzy_cards",
        "tool_empty_get_fizzy_card",
        "tool_empty_create_fizzy_card",
        "tool_empty_comment_on_fizzy_card",
        "tool_empty_move_fizzy_card",
        "tool_empty_close_fizzy_card",
        "tool_empty_reopen_fizzy_card",
        "tool_empty_pin_message",
        "tool_empty_unpin_message",
        "tool_empty_create_poll",
        "tool_empty_get_poll",
        "tool_empty_handoff_work",
        "tool_empty_start_stream",
        "tool_empty_append_stream",
        "tool_empty_finalize_stream",
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
                    .header("authorization", &[concat!("Bear", "er"), SECRET].join(" ")),
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

#[tokio::test]
async fn mcp_lists_rooms_and_owned_work() {
    check(&["tool_empty_list_rooms", "tool_empty_list_work"]).await;
}

#[tokio::test]
async fn slash_schema_mcp_management_uses_the_shared_registration_service() {
    use super::presenters::test_support::ALL_TALK;
    let app = setup().await;
    let schema = json!({"options":[{"name":"enabled","description":"Enable it","type":"boolean","required":true}]});
    for value in [schema.clone(), Value::Null] {
        let body = json!({"jsonrpc":"2.0","id":650,"method":"tools/call","params":{"name":"register_slash_command","arguments":{"room_id":ALL_TALK,"name":"typed","option_schema":value}}});
        let reply = app
            .anonymous()
            .send(
                Req::new(Method::POST, "/agents/mcp")
                    .header("authorization", &format!("Bearer {SECRET}"))
                    .header("accept", "application/json")
                    .header("content-type", "application/json")
                    .body(body.to_string()),
            )
            .await;
        assert_eq!(reply.status.as_u16(), 200, "{}", reply.text());
        assert_ne!(reply.json()["result"]["isError"], true, "{}", reply.text());
        let expected = value.clone();
        app.db()
            .read(move |conn| {
                let command =
                    campfire_db::AgentSlashCommand::find_by_name(conn, ALL_TALK, "typed")?.unwrap();
                assert_eq!(
                    serde_json::to_value(command.option_schema()?).unwrap(),
                    expected
                );
                Ok(())
            })
            .await
            .unwrap();
    }
}
