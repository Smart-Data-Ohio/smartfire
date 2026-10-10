//! MCP reactions: raw Rails bytes, shortcode replay, and current permissions.
use serde_json::Value;

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
