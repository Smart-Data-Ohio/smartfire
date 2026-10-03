//! WS12 service/model costs measured through the unchanged WS11 REST and MCP adapters.
use serde_json::Value;

#[tokio::test]
async fn ws12_agent_work_service_reads_are_reduced_and_flat_at_two_sizes() {
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agent_work_writes_http.json"
    ))
    .unwrap();
    let surfaces = [
        ("rest_create", 180),
        ("mcp_create", 168),
        ("rest_update", 65),
        ("mcp_update", 65),
        ("mcp_board_update", 65),
        ("rest_result", 27),
        ("mcp_result", 27),
        ("rest_handoff", 86),
        ("mcp_handoff", 86),
    ];
    let mut failures = Vec::new();
    for (surface, ceiling) in surfaces {
        let mut counts = Vec::new();
        let mut rails = Vec::new();
        for size in [5, 50] {
            let case = vectors["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["name"] == format!("{surface}_queries_{size}"))
                .unwrap();
            let app = super::agent_reads_tests::prepare(case).await;
            let log = app.db().capture_queries();
            let reply = app
                .anonymous()
                .send(super::agent_reads_tests::request(case))
                .await;
            app.db().stop_capturing_queries();
            assert_eq!(
                reply.status.as_u16(),
                case["status"].as_u64().unwrap() as u16
            );
            assert_eq!(
                reply.text(),
                case["response_body"].as_str().unwrap(),
                "{surface}/{size}"
            );
            for (key, value) in case["response_headers"].as_object().unwrap() {
                assert_eq!(
                    reply.headers.get(key).and_then(|h| h.to_str().ok()),
                    value.as_str(),
                    "{surface}/{size}: {key}"
                );
            }
            counts.push(
                log.lock()
                    .unwrap()
                    .iter()
                    .filter(|sql| sql.trim_start().to_ascii_uppercase().starts_with("SELECT"))
                    .count(),
            );
            rails.push(case["selects"].as_u64().unwrap());
            super::agent_work_writes_tests::assert_state(&app, &case["state"], surface).await;
        }
        println!(
            "WS12_AGENT_WORK_READS {surface} size=5/50 Rust={}/{} Rails={}/{}",
            counts[0], counts[1], rails[0], rails[1]
        );
        assert_eq!(
            counts[0], counts[1],
            "{surface}: request reads must stay flat"
        );
        if counts.iter().any(|&count| count > ceiling) {
            failures.push(format!("{surface}: {counts:?} exceed {ceiling}"));
        }
    }
    assert!(
        failures.is_empty(),
        "avoidable service reads remain: {}",
        failures.join("; ")
    );
}
