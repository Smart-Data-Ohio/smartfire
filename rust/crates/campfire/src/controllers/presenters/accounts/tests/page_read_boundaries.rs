//! Controller boundaries compared byte-for-byte with pinned Rails.
use crate::controllers::presenters::test_support::{BENDER, SEED_NOW, TestApp};
use axum::http::StatusCode;
use campfire_db::{Agent, AgentApproval, models::agent_approval::NewApproval};
use serde_json::{Value, json};
use std::sync::Arc;

fn corpus() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../../../vectors/agent-page-read-boundaries.json"
    ))
    .unwrap()
}

fn tours() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../../../vectors/agent-tour-values.json"
    ))
    .unwrap()
}
fn tour_fragment(body: &str) -> &str {
    let start = body.find("<div id=\"tour\" hidden").unwrap();
    let end = start + body[start..].find("\n</div>").unwrap() + "\n</div>".len();
    &body[start..end]
}
async fn store_tour(t: &TestApp, case: &Value) {
    let sql = case["sql"].as_str().unwrap().to_owned();
    t.db()
        .write(move |tx| {
            tx.conn().execute(&sql, [])?;
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn ws11ui_review199_valid_tour_timestamps_match_rails_bytes() {
    let t = TestApp::boot_frozen()
        .await
        .unwrap()
        .without_job_runner()
        .await;
    let mut browser = t.david();
    let data = tours();
    let mut failures = Vec::new();
    let mut checked = 0;
    for case in data["cases"].as_array().unwrap().iter().filter(|c| {
        matches!(
            c["name"].as_str(),
            Some("month_abbrev_dot" | "year_month_name")
        )
    }) {
        store_tour(&t, case).await;
        for expected in case["responses"].as_array().unwrap() {
            let path = expected["path"].as_str().unwrap();
            let response = browser.get(path).await;
            assert!(
                expected["body"]
                    .as_str()
                    .unwrap()
                    .contains("data-tour-auto-start-value=\"false\"")
            );
            if response.status.as_u16() as u64 != expected["status"].as_u64().unwrap()
                || tour_fragment(&response.text()) != expected["body"].as_str().unwrap()
            {
                failures.push(format!("{} {path}", case["name"]));
            }
            checked += 1;
        }
    }
    assert_eq!(checked, 4);
    println!(
        "Tour valid timestamp regression: 2 values; {checked} responses; {} mismatches",
        failures.len()
    );
    assert!(failures.is_empty(), "{failures:?}");
}

#[tokio::test]
async fn ws11ui_review199_tour_differential_reports_known_main_differences() {
    let t = TestApp::boot_frozen()
        .await
        .unwrap()
        .without_job_runner()
        .await;
    let mut browser = t.david();
    let data = tours();
    let cases = data["cases"].as_array().unwrap();
    // This unchanged Rails fragment is an independent control for main's SQL
    // non-null projection. Each case's own Rails fragment is still compared raw.
    let completed = cases
        .iter()
        .find(|c| c["name"] == "original_valid_datetime")
        .unwrap();
    let mut failures = Vec::new();
    let mut known = Vec::new();
    let mut checked = 0;
    for case in cases {
        store_tour(&t, case).await;
        for expected in case["responses"].as_array().unwrap() {
            let path = expected["path"].as_str().unwrap();
            let response = browser.get(path).await;
            let body = response.text();
            let actual = tour_fragment(&body);
            assert_eq!(
                response.status.as_u16() as u64,
                expected["status"].as_u64().unwrap()
            );
            let equal = actual == expected["body"].as_str().unwrap();
            if case["known_difference"].is_string() {
                let control = completed["responses"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["path"] == path)
                    .unwrap();
                assert_eq!(
                    actual,
                    control["body"].as_str().unwrap(),
                    "keep main's complete non-null tour fragment: {} {path}",
                    case["name"]
                );
                if equal {
                    failures.push(format!("known difference changed: {} {path}", case["name"]));
                }
                println!(
                    "Known main tour difference {} {path}; fix after #196 merges\nRust: {actual}\nRails: {}",
                    case["name"],
                    expected["body"].as_str().unwrap()
                );
                known.push(format!("{} {path}", case["name"]));
            } else if !equal {
                failures.push(format!("{} {path}", case["name"]));
            }
            checked += 1;
        }
    }
    assert_eq!(cases.len(), 53);
    assert_eq!(checked, 106);
    assert_eq!(known.len(), 28);
    println!(
        "Tour Rails differential: 53 values; {checked} responses; {} known main differences; {} unexpected differences; 0 skipped; raw fragments unchanged",
        known.len(),
        failures.len()
    );
    assert!(failures.is_empty(), "{failures:?}");
}

#[tokio::test]
async fn ws11ui_review199_expiry_crosses_writer_wait() {
    let clock = Arc::new(campfire_kit::FrozenClock::new(SEED_NOW.parse().unwrap()));
    let t = TestApp::boot_with_test_clock(clock.clone())
        .await
        .unwrap()
        .without_job_runner()
        .await;
    let mut browser = t.david();
    assert_eq!(browser.get("/agents").await.status, StatusCode::OK);
    let agent_id = t
        .db()
        .write(|tx| {
            let agent = Agent::for_user(tx.conn(), BENDER)?.unwrap();
            tx.conn()
                .execute("DELETE FROM agent_approvals WHERE agent_id=?", [agent.id])?;
            for (summary, delta) in [
                ("review199 overdue", -10),
                ("review199 crosses deadline", 1),
            ] {
                let approval = AgentApproval::create(
                    tx,
                    NewApproval {
                        agent_id: agent.id,
                        action: "deploy".into(),
                        summary: summary.into(),
                        ..Default::default()
                    },
                )?;
                let expires = tx.now().since(jiff::SignedDuration::from_secs(delta));
                tx.conn().execute(
                    "UPDATE agent_approvals SET expires_at=? WHERE id=?",
                    rusqlite::params![expires, approval.id],
                )?;
            }
            Ok(agent.id)
        })
        .await
        .unwrap();
    let queries = t.db().capture_queries();
    let db = t.db().clone();
    let (entered, ready) = tokio::sync::oneshot::channel();
    let (release, held) = std::sync::mpsc::channel();
    let writer = tokio::spawn(async move {
        db.write(move |_| {
            entered.send(()).unwrap();
            held.recv().unwrap();
            Ok(())
        })
        .await
    });
    ready.await.unwrap();
    let path = format!("/agents/{agent_id}/approvals");
    let (response, (queued, selected)) = tokio::join!(browser.get(&path), async {
        // Observe the real queue, not elapsed time. The deadline only prevents a hung test.
        let queued = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while t.db().queued_writes() == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await;
        let selected = queries
            .lock()
            .unwrap()
            .iter()
            .any(|sql| sql.starts_with("SELECT * FROM agent_approvals WHERE agent_id="));
        clock.advance(jiff::SignedDuration::from_secs(2));
        release.send(()).unwrap();
        (queued, selected)
    });
    writer.await.unwrap().unwrap();
    t.db().stop_capturing_queries();
    assert!(
        queued.is_ok() && selected,
        "history must select before its expiry write queues"
    );
    let states = t.db().read(move |conn| {
        let mut query = conn.prepare("SELECT summary,status,(SELECT count(*) FROM activity_items i WHERE i.source_type='AgentApproval' AND i.source_id=a.id AND i.handled_at IS NULL) FROM agent_approvals a WHERE agent_id=? ORDER BY id")?;
        Ok(query.query_map([agent_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?)))?.collect::<rusqlite::Result<Vec<_>>>()?)
    }).await.unwrap();
    let data = corpus();
    let expected = &data["expiry"];
    println!(
        "Approval writer-wait differential: Rust={}; Rails={}",
        json!(states),
        expected["rows"]
    );
    assert_eq!(json!(states), expected["rows"]);
    assert_eq!(
        response.status.as_u16() as u64,
        expected["status"].as_u64().unwrap()
    );
    let body = response.text();
    let start = body
        .find("<menu class=\"flex flex-column gap margin-none pad txt-align-start\">")
        .unwrap();
    let end = start + body[start..].find("</menu>").unwrap() + "</menu>".len();
    assert_eq!(
        &body[start..end],
        expected["body"].as_str().unwrap(),
        "complete approval menu bytes after the writer wait"
    );
}
