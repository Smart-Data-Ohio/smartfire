//! Shared MessagePin operations through both authenticated transports.
use serde_json::Value;
async fn group(mcp: bool) {
    let vectors: Value =
        serde_json::from_str(include_str!("../../../../vectors/agent_pins_http.json")).unwrap();
    for case in vectors["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| case["name"].as_str().unwrap().starts_with("mcp_") == mcp)
    {
        super::agent_reads_tests::check(case).await;
    }
}
#[tokio::test]
async fn agent_pins_rest_bytes() {
    group(false).await;
}
#[tokio::test]
async fn agent_pins_mcp_bytes() {
    group(true).await;
}
