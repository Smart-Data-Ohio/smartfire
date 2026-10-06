//! Focused original named assertions: committed sequences, exact wire output and state.
use super::agent_http_tests::SECRET;
use super::presenters::test_support::Req;
use campfire_kit::Method;
use serde_json::{Value, json};

async fn run(key: &str) {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agent_work_named_http.json"
    ))
    .unwrap();
    let case = oracle["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["key"] == key)
        .unwrap();
    let app = super::agent_reads_tests::prepare(case).await;
    let mut actual = Vec::new();
    for step in case["observations"].as_array().unwrap() {
        let mut path = step["path"].as_str().unwrap().to_owned();
        if step["auth"] == "bot_key" {
            path.push_str("?bot_key=394959859-BenderToken1");
        }
        let mut req = Req::new(
            Method::from_bytes(step["method"].as_str().unwrap().to_uppercase().as_bytes()).unwrap(),
            &path,
        )
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .header("user-agent", "ws11api-work-contract")
        .header("x-forwarded-for", "203.0.113.31");
        match step["auth"].as_str() {
            Some("session") => {}
            Some("bot_key") => {}
            None => {
                req = req.header("authorization", &format!("Bearer {SECRET}"));
            }
            Some(other) => panic!("unknown credential {other}"),
        }
        if let Some(body) = step["request_body"].as_str() {
            req = req.body(body);
        }
        let reply = if step["auth"] == "session" {
            app.david().write(req).await
        } else {
            app.anonymous().send(req).await
        };
        let headers = step["response_headers"]
            .as_object()
            .unwrap()
            .keys()
            .map(|k| (k.clone(), json!(reply.header(k))))
            .collect::<serde_json::Map<_, _>>();
        actual.push(json!({"status":reply.status.as_u16(),"body":reply.text(),"headers":headers}));
    }
    let expected = case["observations"].as_array().unwrap().iter().map(|step|json!({"status":step["status"],"body":step["response_body"],"headers":step["response_headers"]})).collect::<Vec<_>>();
    assert_eq!(actual, expected, "{key}: named response sequence");
    super::agent_work_writes_tests::assert_state(&app, &case["state"], key).await;
    println!(
        "WS11_NAMED_API {key}: {} responses; complete persisted rows/jobs",
        actual.len()
    );
}

macro_rules! cases { ($($name:ident => $key:literal),* $(,)?) => { $(#[tokio::test] async fn $name() { run($key).await; })* }; }
cases! {
    ws11_named_mcp_not_owner => "mcp_not_owner",
    ws11_named_mcp_suspended_receiver => "mcp_suspended_receiver",
    ws11_named_posts_nonmember => "posts_nonmember",
    ws11_named_posts_credentials => "posts_credentials",
    ws11_named_show_owned => "show_owned",
    ws11_named_show_not_owner => "show_not_owner",
    ws11_named_patch_not_owner => "patch_not_owner",
    ws11_named_manage_other_room => "manage_other_room",
    ws11_named_ignore_assignment => "ignore_assignment",
    ws11_named_tags_only => "tags_only",
    ws11_named_blank_status => "blank_status",
    ws11_named_note_only => "note_only",
    ws11_named_tags_no_manage => "tags_no_manage",
    ws11_named_result_limit => "result_limit",
    ws11_named_result_clear => "result_clear",
    ws11_named_result_noop => "result_noop",
    ws11_named_writes_credentials => "writes_credentials",
    ws11_named_show_other_read_room => "show_other_read_room",
    ws11_named_rest_not_owner => "rest_not_owner",
    ws11_named_rest_human_handoff => "rest_human_handoff",
}
