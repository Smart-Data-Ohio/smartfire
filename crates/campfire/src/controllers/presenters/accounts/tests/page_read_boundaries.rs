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

#[tokio::test]
async fn ws11ui_review199_expiry_crosses_writer_wait() {
    let clock = Arc::new(campfire_kit::FrozenClock::new(SEED_NOW.parse().unwrap()));
    let t = TestApp::boot_with_test_clock(clock.clone())
        .await
        .unwrap()
        .without_job_runner()
        .await;
    let mut browser = t.david();
    assert_eq!(browser.get("/api/v1/agents").await.status, StatusCode::OK);
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
    let path = format!("/api/v1/agents/{agent_id}/approvals");
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
}


#[tokio::test]
async fn ws11ui_next_stored_datetime_cast_values_match_rails() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../../../../../vectors/agent-tour-casts.json"
    ))
    .unwrap();
    let t = TestApp::boot_frozen()
        .await
        .unwrap()
        .without_job_runner()
        .await;
    let mut failures = Vec::new();
    for case in data["cases"].as_array().unwrap() {
        store_tour(&t, case).await;
        let actual = t
            .db()
            .read(|conn| {
                Ok(conn.query_row(
                    "SELECT tour_completed_at FROM users WHERE id=127326141",
                    [],
                    |r| {
                        use rusqlite::types::ValueRef;
                        Ok(match r.get_ref(0)? {
                            ValueRef::Null => Value::Null,
                            ValueRef::Integer(v) => json!(v),
                            ValueRef::Real(v) => json!(v),
                            ValueRef::Text(text) => json!(std::str::from_utf8(text).ok()
                    .and_then(rails_compat::datetime::deserialize::<campfire_db::Timestamp>)
                    .map(|value| value.to_db())),
                            ValueRef::Blob(bytes) => json!(
                                rails_compat::datetime::deserialize_sqlite_blob::<
                                    campfire_db::Timestamp,
                                >(bytes)
                                .map(|value| value.to_db())
                            ),
                        })
                    },
                )?)
            })
            .await
            .unwrap();
        if actual != case["stored"] {
            failures.push(format!(
                "{}: {actual}; Rails {}",
                case["name"], case["stored"]
            ));
        }
    }
    println!(
        "Shared stored datetime cast differential: 199 values; {} mismatches",
        failures.len()
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
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
