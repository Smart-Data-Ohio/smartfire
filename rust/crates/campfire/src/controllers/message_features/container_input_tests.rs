//! Full actual HTTP envelopes, persisted tables and broadcasts.
//! Producer control: check_input_mutants.py changes the actual slash response
//! and saved-item status writes; neither expected state nor input is mutated.
use super::{
    comparison_support,
    quote_integration_tests::{app_rows, stream},
};
use crate::controllers::presenters::test_support::*;
use serde_json::{Value, json};
use std::collections::HashMap;
#[tokio::test]
async fn exceptional_root_and_nested_containers_match_rails_with_flat_reads() {
    let vector: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/container_inputs.json"
    ))
    .unwrap();
    assert_eq!(compare_feature_input_requests(vector).await, 84);
}
pub(super) async fn compare_feature_input_requests(vector: Value) -> usize {
    let mut counts = HashMap::new();
    let mut failures = vec![];
    let mut checked = 0;
    for group in vector["groups"].as_array().unwrap() {
        for case in group["cases"].as_array().unwrap() {
            // Independent fixed entropy on both runtimes. Never use an expected
            // message ID/UUID or frame to construct the observed result.
            let inputs = campfire_db::database::FixtureInputs {
                message_uuid: std::sync::Arc::new(|| "fixture-relative-message".into()),
                sqlite_now: campfire_db::Timestamp::parse_db(SEED_NOW).unwrap(),
            };
            let app =
                crate::test_support::with_message_inputs(inputs, app_rows(group["rows"].clone()))
                    .await
                    .without_job_runner()
                    .await;
            if let Some(zone) = case["zone"].as_str() {
                let zone = zone.to_owned();
                app.db()
                    .write(move |tx| {
                        tx.conn().execute(
                            "UPDATE users SET time_zone=? WHERE id=?",
                            rusqlite::params![zone, DAVID],
                        )?;
                        Ok(())
                    })
                    .await
                    .unwrap();
            }
            let (mut socket, server) = stream(&app).await;
            let mut browser = app.david();
            browser.authenticity_token().await;
            let query_log = app.db().capture_queries();
            let response = browser
                .write(
                    Req::new(hyper::Method::POST, case["path"].as_str().unwrap())
                        .header("accept", "application/json")
                        .header("content-type", "application/json")
                        .body(serde_json::to_vec(&case["params"]).unwrap()),
                )
                .await;
            app.db().stop_capturing_queries();
            let reads = query_log.lock().unwrap().len();
            let actual = json!({"status":response.status.as_u16(),"content_type":response.header("content-type"),"body":response.text(),"location":response.header("location")});
            let expected = json!({"status":case["status"],"content_type":case["content_type"],"body":case["body"],"location":case["location"]});
            if actual != expected {
                failures.push(json!({"size":group["size"],"case":case["name"],"actual":actual,"expected":expected}));
            }
            let tables = comparison_support::expected_tables(group, case);
            app.db()
                .read(move |conn| {
                    for (table, expected) in &tables {
                        let ids = conn
                            .prepare(&format!("SELECT id FROM {table} ORDER BY id"))?
                            .query_map([], |r| r.get::<_, i64>(0))?
                            .collect::<rusqlite::Result<Vec<_>>>()?;
                        assert_eq!(
                            ids.len(),
                            expected.as_array().unwrap().len(),
                            "container persisted row count: {table}"
                        );
                        for (id, expected) in ids.into_iter().zip(expected.as_array().unwrap()) {
                            comparison_support::same_row(
                                &comparison_support::row(conn, table, id)?,
                                expected,
                                "container persisted rows",
                            );
                        }
                    }
                    Ok(())
                })
                .await
                .unwrap();
            comparison_support::published_frames(
                &app,
                &mut socket,
                &case["frames"],
                &format!(
                    "container actual publications {}",
                    case["name"].as_str().unwrap()
                ),
            )
            .await;
            socket.assert_silent().await;
            let key = case["name"].as_str().unwrap().to_owned();
            println!(
                "WS8bm2 container {key} size={}: Rust {reads}; Rails {}",
                group["size"], case["reads"]
            );
            if let Some(previous) = counts.insert(key, reads) {
                assert_eq!(previous, reads, "container physical read growth");
            }
            checked += 1;
            server.abort();
        }
    }
    for failure in &failures {
        eprintln!("{failure}");
    }
    println!(
        "WS8bm2 container Rust: {checked} requests; {} envelope differences; complete persisted tables, ordered publications and flat reads",
        failures.len()
    );
    assert!(
        failures.is_empty(),
        "container actual HTTP envelope differs from Rails"
    );
    checked
}

/// Producer controls also exercise the real consumer output, not a separately
/// reconstructed parser result. See check_input_mutants.py and the report.
#[tokio::test]
async fn exceptional_relative_consumers_match_rails_complete_state_with_flat_reads() {
    let vector: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/relative_consumers.json"
    )).unwrap();
    assert_eq!(compare_feature_input_requests(vector).await, 80);
}
