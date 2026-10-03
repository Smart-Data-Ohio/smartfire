//! Complete sidebar and human-profile responses through the real controller stack.
use crate::controllers::presenters::test_support::{Req, TestApp, with_fixed_render_secrets};
use axum::http::Method;
use serde_json::Value;

fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../../../vectors/agent-navigation-matrix.json"
    ))
    .unwrap()
}
fn reads(queries: &[String], table: &str) -> usize {
    let from = regex::Regex::new(r#"(?i)\bFROM\s+"?([a-z_]+)"#).unwrap();
    queries
        .iter()
        .filter(|sql| {
            sql.trim_start().to_uppercase().starts_with("SELECT")
                && from.captures(sql).is_some_and(|c| &c[1] == table)
        })
        .count()
}
async fn compare(paths: &[&str]) {
    let mut failures = Vec::new();
    let mut compared = 0;
    let mut costs = std::collections::BTreeMap::new();
    let corpus = oracle();
    for case in corpus["cases"].as_array().unwrap().iter().filter(|c| {
        paths
            .iter()
            .any(|p| c["path"].as_str().unwrap().starts_with(p))
    }) {
        let vars: Vec<_> = include_str!("../../../../../../../parity/.env.reference")
            .lines()
            .filter_map(|line| line.split_once('='))
            .filter(|(key, _)| key.starts_with("VAPID_"))
            .collect();
        let t = TestApp::boot_frozen_with_env(&vars)
            .await
            .unwrap()
            .without_job_runner()
            .await;
        compared += 1;
        let setup: Vec<String> = case["sql"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().into())
            .collect();
        let mut viewer = t.anonymous();
        let labels: Value = serde_json::from_str(
            &std::fs::read_to_string(
                crate::controllers::presenters::test_support::seed_dir("default")
                    .unwrap()
                    .join("labels.json"),
            )
            .unwrap(),
        )
        .unwrap();
        viewer.absorb_cookie_header(&format!(
            "session_token={}",
            labels[format!("session_cookies.{}", case["viewer"].as_str().unwrap())]
                .as_str()
                .unwrap()
        ));
        if case["name"].as_str().unwrap().starts_with("appearance ") {
            // Warm the real direct-room fragment before changing the viewer's zone.
            let prior = Req::new(Method::GET, case["path"].as_str().unwrap())
                .header("accept", "text/html")
                .header("turbo-frame", "ui_matrix");
            assert_eq!(
                with_fixed_render_secrets(viewer.send(prior))
                    .await
                    .status
                    .as_u16(),
                200
            );
        }
        t.db()
            .write(move |tx| {
                tx.conn().execute_batch("PRAGMA defer_foreign_keys=ON")?;
                for sql in setup {
                    tx.conn().execute(&sql, [])?;
                }
                Ok(())
            })
            .await
            .unwrap();
        let mut request = Req::new(Method::GET, case["path"].as_str().unwrap())
            .header("user-agent", "Mozilla/5.0 Chrome/140.0.0.0")
            .header("accept", "text/html");
        if case["frame"] == true {
            request = request.header("turbo-frame", "ui_matrix");
        }
        let captured = t.db().capture_queries();
        let response = with_fixed_render_secrets(viewer.send(request)).await;
        t.db().stop_capturing_queries();
        let expected = case["result"]["body"].as_str().unwrap();
        if response.status.as_u16() as u64 != case["result"]["status"].as_u64().unwrap()
            || response.text() != expected
        {
            let actual = response.text();
            let byte = actual
                .bytes()
                .zip(expected.bytes())
                .position(|(a, b)| a != b)
                .unwrap_or(actual.len().min(expected.len()));
            if let Ok(dir) = std::env::var("WS11UI_DIFF_DIR") {
                std::fs::create_dir_all(&dir).unwrap();
                let name = case["name"].as_str().unwrap().replace('/', "_");
                std::fs::write(format!("{dir}/{name}.actual"), &actual).unwrap();
                std::fs::write(format!("{dir}/{name}.rails"), expected).unwrap();
            }
            failures.push(format!(
                "{}: HTTP {}, byte {byte}, actual {}, Rails {}",
                case["name"],
                response.status,
                actual.len(),
                expected.len()
            ));
        }
        if let Some(kind) = case["kind"].as_str() {
            let mut tables = std::collections::BTreeMap::new();
            for table in [
                "users",
                "rooms",
                "memberships",
                "agents",
                "room_categories",
                "active_storage_attachments",
            ] {
                tables.insert(
                    table.to_owned(),
                    (
                        reads(&captured.lock().unwrap(), table),
                        case["result"]["counts"][table].as_u64().unwrap_or(0) as usize,
                    ),
                );
            }
            tables.insert(
                "total".into(),
                (
                    captured
                        .lock()
                        .unwrap()
                        .iter()
                        .filter(|sql| sql.trim_start().to_uppercase().starts_with("SELECT"))
                        .count(),
                    case["result"]["counts"]
                        .as_object()
                        .unwrap()
                        .values()
                        .map(|v| v.as_u64().unwrap() as usize)
                        .sum(),
                ),
            );
            costs.insert((kind.to_owned(), case["size"].as_u64().unwrap()), tables);
        }
    }
    for kind in ["sidebar", "profile"] {
        if let (Some(small), Some(large)) =
            (costs.get(&(kind.into(), 2)), costs.get(&(kind.into(), 12)))
        {
            for (table, (rust_small, rails_small)) in small {
                let (rust_large, rails_large) = large[table];
                println!(
                    "Navigation reads {kind} {table}: Rust {rust_small}->{rust_large}; Rails {rails_small}->{rails_large}"
                );
                if rust_large.saturating_sub(*rust_small) > rails_large.saturating_sub(*rails_small)
                {
                    failures.push(format!("{kind}: {table} grows faster than Rails"));
                }
            }
        }
    }
    println!(
        "Navigation response differential: {compared} compared; {} mismatches",
        failures.len()
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
#[tokio::test]
async fn ws11ui_sidebar_full_response_matrix_matches_rails_and_read_growth() {
    compare(&["/users/me/sidebar"]).await;
}
#[tokio::test]
async fn ws11ui_human_profile_full_response_matrix_matches_rails_and_read_growth() {
    compare(&[
        "/users/me/profile",
        "/users/127",
        "/users/712",
        "/users/394",
    ])
    .await;
}
