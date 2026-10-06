//! Real writer-queue interleavings, compared to pinned Rails' independent saves.
use campfire_db::Agent;
use campfire_db::AgentChanges;
use campfire_db::User;
use serde_json::Value;
use serde_json::json;
use crate::controllers::presenters::test_support::{BENDER, Req, TestApp};
use axum::http::Method;

fn changes(value: &Value) -> AgentChanges {
    AgentChanges {
        provider: value.get("provider").map(|v| v.as_str().map(str::to_owned)),
        runtime: value.get("runtime").map(|v| v.as_str().map(str::to_owned)),
        description: value
            .get("description")
            .map(|v| v.as_str().map(str::to_owned)),
        daily_message_cap_before_type_cast: value.get("daily_message_cap").cloned(),
        daily_board_post_cap_before_type_cast: value.get("daily_board_post_cap").cloned(),
        daily_external_action_cap_before_type_cast: value.get("daily_external_action_cap").cloned(),
        ..Default::default()
    }
}
#[tokio::test]
async fn pr196_interleaved_edit_preserves_clean_agent_columns() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/bot-ui-concurrent-edits.json"
    ))
    .unwrap();
    let mut failures = Vec::new();
    let cases = oracle["cases"].as_array().unwrap();
    for case in cases {
        let t = TestApp::boot_frozen()
            .await
            .unwrap()
            .without_job_runner()
            .await;
        let initial = changes(&oracle["initial"]);
        t.db()
            .write(move |tx| {
                Agent::for_user(tx.conn(), BENDER)?
                    .unwrap()
                    .update(tx, initial)
            })
            .await
            .unwrap();
        let mut browser = t.david();
        browser.grant_sudo().await;
        browser.get(&format!("/account/bots/{BENDER}/edit")).await;
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let db = t.db().clone();
        let blocker = tokio::spawn(async move {
            db.write(move |_| {
                entered_tx.send(()).unwrap();
                release_rx
                    .recv_timeout(std::time::Duration::from_secs(10))
                    .unwrap();
                Ok(())
            })
            .await
            .unwrap()
        });
        entered_rx.await.unwrap();
        let before = t
            .db()
            .read(|conn| {
                Ok(
                    conn.query_row("SELECT COALESCE(MAX(id),0) FROM audit_logs", [], |r| {
                        r.get::<_, i64>(0)
                    })?,
                )
            })
            .await
            .unwrap();
        let req = browser.write(
            Req::new(Method::PATCH, &format!("/account/bots/{BENDER}"))
                .header("accept", "text/html")
                .header("content-type", "application/json")
                .body(
                    json!({"user":{"name":"Review interleaved"},"agent":case["submitted"]})
                        .to_string(),
                ),
        );
        tokio::pin!(req);
        tokio::select! {
            _ = &mut req => panic!("request ended while writer blocked"),
            result = tokio::time::timeout(std::time::Duration::from_secs(5), async { while t.db().queued_writes() < 1 { tokio::task::yield_now().await; } }) => result.expect("A did not enqueue"),
        }
        let db = t.db().clone();
        let competing = changes(&case["concurrent"]);
        let concurrent = tokio::spawn(async move {
            db.write(move |tx| {
                assert_eq!(
                    User::find(tx.conn(), BENDER)?.name,
                    "Review interleaved",
                    "A's bot commit must precede B"
                );
                Agent::for_user(tx.conn(), BENDER)?
                    .unwrap()
                    .update(tx, competing)
            })
            .await
            .unwrap()
        });
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while t.db().queued_writes() < 2 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("B did not enqueue");
        release_tx.send(()).unwrap();
        let response = req.await;
        blocker.await.unwrap();
        concurrent.await.unwrap();
        let actual = t.db().read(move |conn| {
            let agent = Agent::for_user(conn, BENDER)?.unwrap();
            let details = conn.prepare("SELECT details FROM audit_logs WHERE id>? AND action='agent.update' ORDER BY id")?.query_map([before], |r| r.get::<_, String>(0))?.collect::<Result<Vec<_>, _>>()?;
            let audits: Vec<Value> = details.iter().map(|s| serde_json::from_str(s).unwrap()).collect();
            Ok(json!({"stored":{"provider":agent.provider,"runtime":agent.runtime,"description":agent.description,"daily_message_cap":agent.daily_message_cap,"daily_board_post_cap":agent.daily_board_post_cap,"daily_external_action_cap":agent.daily_external_action_cap},"audits":audits}))
        }).await.unwrap();
        let expected = json!({"stored":case["stored"],"audits":case["audits"]});
        if actual != expected || response.status.as_u16() != case["status"].as_u64().unwrap() as u16
        {
            failures.push(format!(
                "{}: HTTP {}; {actual}; Rails {expected}",
                case["name"], response.status
            ));
        }
    }
    println!(
        "Concurrent edit differential: {} writer-queue interleavings; {} mismatches",
        cases.len(),
        failures.len()
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
