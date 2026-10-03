//! Complete pinned Rails wire responses for retained snapshots of deleted work.
use super::agent_http_tests::{SECRET, setup};
use super::presenters::test_support::Req;
use campfire_kit::Method;
use serde_json::Value;

async fn deleted_work(surface: &str) {
    let vector: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agent_pr227_deleted_work.json"
    ))
    .unwrap();
    let mut small: Option<Vec<(usize, usize)>> = None;
    for case in vector["cases"].as_array().unwrap() {
        let app = setup().await.without_job_runner().await;
        let fixture = case["initial_sql"].as_array().unwrap().clone();
        app.db()
            .write(move |tx| {
                for sql in fixture {
                    tx.conn().execute_batch(sql.as_str().unwrap())?;
                }
                Ok(())
            })
            .await
            .unwrap();
        let before = app
            .db()
            .read(|conn| {
                let mut q = conn.prepare(
                    "SELECT id,metadata FROM agent_events WHERE agent_id=773018776 ORDER BY id",
                )?;
                Ok(
                    q.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
                        .collect::<std::result::Result<Vec<_>, _>>()?,
                )
            })
            .await
            .unwrap();
        let mut counts = Vec::new();
        for gold in case["observations"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|o| o["surface"] == surface)
        {
            let mut req = Req::new(
                Method::from_bytes(gold["method"].as_str().unwrap().as_bytes()).unwrap(),
                gold["path"].as_str().unwrap(),
            )
            .header("authorization", &format!("Bearer {SECRET}"))
            .header("accept", "application/json")
            .header("content-type", "application/json");
            if let Some(body) = gold["request_body"].as_str() {
                req = req.body(body);
            }
            let log = app.db().capture_queries();
            let reply = app.anonymous().send(req).await;
            app.db().stop_capturing_queries();
            let selects = log
                .lock()
                .unwrap()
                .iter()
                .filter(|q| q.trim_start().to_ascii_uppercase().starts_with("SELECT"))
                .count();
            counts.push((selects, gold["selects"].as_u64().unwrap() as usize));
            println!(
                "PR227_DELETED_WORK {surface} size={} Rust={selects} Rails={}",
                case["size"], gold["selects"]
            );
            assert_eq!(
                reply.status.as_u16(),
                gold["status"].as_u64().unwrap() as u16
            );
            assert_eq!(
                reply.text(),
                gold["response_body"].as_str().unwrap(),
                "deleted work {surface}: complete fresh Rails response bytes"
            );
            for (name, value) in gold["response_headers"].as_object().unwrap() {
                assert_eq!(
                    reply.header(name),
                    value.as_str(),
                    "deleted work {surface}: header {name}"
                );
            }
            assert_eq!(gold["cache_hits"], 0);
            let body: Value = serde_json::from_str(&reply.text()).unwrap();
            let payload = if surface == "mcp" {
                serde_json::from_str::<Value>(
                    body["result"]["content"][0]["text"].as_str().unwrap(),
                )
                .unwrap()
            } else {
                body
            };
            assert_eq!(
                payload["events"].as_array().unwrap().len() as u64,
                case["size"].as_u64().unwrap()
            );
            assert!(
                payload["events"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|e| e["thread_deleted"] == true)
            );
        }
        let after = app
            .db()
            .read(|conn| {
                let mut q = conn.prepare(
                    "SELECT id,metadata FROM agent_events WHERE agent_id=773018776 ORDER BY id",
                )?;
                Ok(
                    q.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
                        .collect::<std::result::Result<Vec<_>, _>>()?,
                )
            })
            .await
            .unwrap();
        assert_eq!(before, after, "polling retains the exact ledger snapshots");
        if let Some(ref first) = small {
            assert_eq!(
                first[0].0, counts[0].0,
                "deleted-work REST/MCP reads stay flat at 5/50 rows"
            );
            assert!(
                counts[0].0 as isize - first[0].0 as isize
                    <= counts[0].1 as isize - first[0].1 as isize
            );
        } else {
            small = Some(counts);
        }
    }
}

#[tokio::test]
async fn pr227_deleted_work_rest_bytes_and_flat_reads() {
    deleted_work("rest").await;
}

#[tokio::test]
async fn pr227_deleted_work_mcp_bytes_and_flat_reads() {
    deleted_work("mcp").await;
}
