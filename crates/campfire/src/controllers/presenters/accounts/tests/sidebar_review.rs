//! Complete pinned Rails responses, with enabled fragment caching and real profile writes.
use crate::controllers::presenters::test_support::{
    Browser, Req, TestApp, with_fixed_render_secrets,
};
use axum::http::Method;
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn corpus() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../../../vectors/agent-sidebar-review.json"
    ))
    .unwrap()
}
fn counts(queries: &[String]) -> BTreeMap<String, usize> {
    let from = regex::Regex::new(r#"(?i)\bFROM\s+"?([a-z_]+)"#).unwrap();
    let mut counts = BTreeMap::new();
    for sql in queries
        .iter()
        .filter(|q| q.trim_start().to_uppercase().starts_with("SELECT"))
    {
        *counts.entry("total".into()).or_default() += 1;
        if let Some(c) = from.captures(sql) {
            *counts.entry(c[1].into()).or_default() += 1;
        }
    }
    counts
}
async fn boot(case: &Value) -> TestApp {
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
    let sql: Vec<String> = case["sql"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().into())
        .collect();
    t.db()
        .write(move |tx| {
            tx.conn().execute_batch("PRAGMA defer_foreign_keys=ON")?;
            for statement in sql {
                tx.conn().execute(&statement, [])?;
            }
            Ok(())
        })
        .await
        .unwrap();
    t
}
fn viewer<'a>(t: &'a TestApp, case: &Value) -> Browser<'a> {
    let labels: Value = serde_json::from_str(
        &std::fs::read_to_string(
            crate::controllers::presenters::test_support::seed_dir("default")
                .unwrap()
                .join("labels.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let mut b = t.anonymous();
    b.absorb_cookie_header(&format!(
        "session_token={}",
        labels[format!("session_cookies.{}", case["viewer"].as_str().unwrap())]
            .as_str()
            .unwrap()
    ));
    b
}
/// Keep the frozen Rails body exact except for S8's worker-selection metadata on the root.
fn without_worker_url(body: &str) -> String {
    let Some((before, root)) = body.split_once("<html ") else {
        return body.to_string();
    };
    let (attributes, after) = root.split_once('>').unwrap();
    let worker_url = " data-service-worker-url=\"/service-worker.js\"";
    assert!(
        attributes.contains(worker_url),
        "the classic page selects its classic worker"
    );
    format!(
        "{before}<html {}>{after}",
        attributes.replacen(worker_url, "", 1)
    )
}

async fn compare(kind: Option<&str>) {
    let corpus = corpus();
    let mut failures = Vec::new();
    let mut costs = BTreeMap::new();
    let mut captures = Vec::new();
    for case in corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["kind"].as_str().map(|s| s.split('_').next().unwrap()) == kind)
    {
        let t = boot(case).await;
        let mut browser = viewer(&t, case);
        for step in case["steps"].as_array().unwrap() {
            let method = Method::from_bytes(step["method"].as_str().unwrap().as_bytes()).unwrap();
            let mut request = Req::new(method.clone(), step["path"].as_str().unwrap())
                .header("user-agent", "Mozilla/5.0 Chrome/140.0.0.0")
                .header("accept", "text/html");
            if step["frame"] == true {
                request = request.header("turbo-frame", "ui_matrix");
            }
            if let Some(params) = step["params"].as_object() {
                let pairs: Vec<_> = params
                    .iter()
                    .map(|(k, v)| (k.as_str(), v.as_str().unwrap()))
                    .collect();
                request = request.form(&pairs);
            }
            if method != Method::GET {
                request = request.header(
                    campfire_kit::csrf::HEADER,
                    &browser.authenticity_token().await,
                );
            }
            let log = t.db().capture_queries();
            let response = with_fixed_render_secrets(browser.send(request)).await;
            t.db().stop_capturing_queries();
            let actual = counts(&log.lock().unwrap());
            let rails: BTreeMap<String, usize> =
                serde_json::from_value(step["result"]["counts"].clone()).unwrap();
            let rails_total: usize = rails.values().sum();
            let name = format!(
                "{} {}",
                case["name"].as_str().unwrap(),
                step["label"].as_str().unwrap()
            );
            println!(
                "Sidebar review SELECTs {name}: Rust {}; Rails {rails_total}",
                actual["total"]
            );
            if response.status.as_u16() as u64 != step["result"]["status"].as_u64().unwrap()
                || response.location() != step["result"]["location"].as_str()
                || !crate::app::asset_goldens::compare(
                    &name,
                    &without_worker_url(&response.text()),
                    step["result"]["body"].as_str().unwrap(),
                )
            {
                if let Ok(dir) = std::env::var("WS11UI_DIFF_DIR") {
                    std::fs::create_dir_all(&dir).unwrap();
                    let stem = name.replace('/', "_");
                    std::fs::write(format!("{dir}/{stem}.actual"), response.text()).unwrap();
                    std::fs::write(
                        format!("{dir}/{stem}.rails"),
                        step["result"]["body"].as_str().unwrap(),
                    )
                    .unwrap();
                }
                failures.push(format!("{name}: complete HTTP response differs from Rails"));
            }
            if case["kind"].is_string() {
                if actual.get("searches") != Some(&1) {
                    failures.push(format!("{name}: layout loaded more than once"));
                }
                costs.insert(
                    (
                        case["kind"].as_str().unwrap().to_owned(),
                        step["label"].as_str().unwrap().to_owned(),
                        case["size"].as_u64().unwrap(),
                    ),
                    (actual["total"], rails_total),
                );
            }
            captures.push(json!({"case":case["name"],"step":step["label"],"rust":actual,"rails":rails,"rails_total":rails_total}));
        }
        if kind.is_none() {
            assert_eq!(corpus["caching"], true);
            assert_eq!(case["membership_versions_unchanged"], true);
            assert!(
                case["warm_cache_keys"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|k| k.as_str().unwrap().contains("sidebars/rooms/_direct"))
            );
            let zone: String = t
                .db()
                .read(|conn| {
                    Ok(conn.query_row(
                        "SELECT time_zone FROM users WHERE id=127326141",
                        [],
                        |row| row.get(0),
                    )?)
                })
                .await
                .unwrap();
            assert_eq!(zone, "Asia/Tokyo", "actual profile PATCH must save");
        }
    }
    for ((name, label, size), (small, rails_small)) in &costs {
        if *size == 2 {
            let (large, rails_large) = costs[&(name.clone(), label.clone(), 12)];
            println!(
                "Sidebar review growth {name} {label}: Rust {small}->{large}; Rails {rails_small}->{rails_large}"
            );
            if large.saturating_sub(*small) > rails_large.saturating_sub(*rails_small) {
                failures.push(format!("{name} {label}: SELECT growth exceeds Rails"));
            }
        }
    }
    if let Ok(dir) = std::env::var("WS11UI_DIFF_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            format!("{dir}/{}.json", kind.unwrap_or("zone")),
            serde_json::to_vec_pretty(&captures).unwrap(),
        )
        .unwrap();
    }
    println!(
        "Sidebar review differential: {} responses; {} failures",
        captures.len(),
        failures.len()
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
#[tokio::test]
async fn ws11ui_sidebar_cold_and_warm_reads_match_rails_growth() {
    compare(Some("sidebar")).await;
}
#[tokio::test]
async fn ws11ui_self_profile_loads_layout_once_with_rails_read_growth() {
    compare(Some("profile")).await;
}
#[tokio::test]
async fn ws11ui_warm_dm_fragment_survives_real_profile_zone_patch_like_rails() {
    compare(None).await;
}
