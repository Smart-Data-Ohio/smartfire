//! Complete administrator requests compared with the pinned Rails query oracle.
use crate::controllers::presenters::test_support::{Req, TestApp};
use axum::http::{Method, StatusCode};

fn counts(queries: &[String], table: &str) -> usize {
    let from = regex::Regex::new(r#"(?i)\bFROM\s+"?([a-z_]+)"#).unwrap();
    queries
        .iter()
        .filter(|sql| {
            sql.trim_start().to_uppercase().starts_with("SELECT")
                && from.captures(sql).is_some_and(|c| &c[1] == table)
        })
        .count()
}
async fn compare(kinds: &[&str], tables: &[&str]) {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../../vectors/agent-page-reads.json"
    ))
    .unwrap();
    let mut failures = Vec::new();
    for case in corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| kinds.contains(&c["kind"].as_str().unwrap()))
    {
        let t = TestApp::boot_frozen()
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
                for sql in sql {
                    tx.conn().execute(&sql, [])?;
                }
                Ok(())
            })
            .await
            .unwrap();
        let mut browser = t.david();
        let log = t.db().capture_queries();
        let response = browser
            .send(
                Req::new(Method::GET, case["path"].as_str().unwrap())
                    .header("accept", case["accept"].as_str().unwrap()),
            )
            .await;
        t.db().stop_capturing_queries();
        assert_eq!(response.status, StatusCode::OK, "{}", response.text());
        if let Some(body) = case["result"]["body"].as_str() {
            assert_eq!(response.text(), body);
        }
        for table in tables {
            let actual = counts(&log.lock().unwrap(), table);
            let rails = case["result"]["counts"][table].as_u64().unwrap() as usize;
            println!(
                "page reads {} {}: {table}={actual}; Rails={rails}",
                case["kind"], case["size"]
            );
            if actual != rails {
                failures.push(format!(
                    "{} {}: {table}={actual}, Rails={rails}",
                    case["kind"], case["size"]
                ));
            }
        }
        if case.get("loader").is_some() {
            let log = t.db().capture_queries();
            let size = t
                .db()
                .read(|conn| Ok(campfire_db::models::agent_profile::for_directory(conn)?.len()))
                .await
                .unwrap();
            t.db().stop_capturing_queries();
            assert_eq!(size as u64, case["loader"]["size"].as_u64().unwrap());
            let actual = log
                .lock()
                .unwrap()
                .iter()
                .filter(|s| s.trim_start().to_uppercase().starts_with("SELECT"))
                .count();
            let rails = case["loader"]["selects"].as_u64().unwrap() as usize;
            println!("directory loader {size}: SELECTs={actual}; Rails={rails}");
            if actual != rails {
                failures.push(format!("loader {size}: {actual}, Rails={rails}"));
            }
        }
    }
    assert!(failures.is_empty(), "Rails read counts: {failures:?}");
}
#[tokio::test]
async fn ws11ui_page_reads_history() {
    compare(
        &["approvals_denied", "approvals_pending", "events"],
        &["agent_approvals", "agent_events", "rooms"],
    )
    .await;
}
#[tokio::test]
async fn ws11ui_page_reads_grants() {
    compare(&["grants"], &["rooms"]).await;
}
#[tokio::test]
async fn ws11ui_page_reads_directory() {
    compare(&["directory"], &["users"]).await;
}
#[tokio::test]
async fn ws11ui_page_reads_bots() {
    compare(&["bots"], &["agents", "users"]).await;
}
#[tokio::test]
async fn ws11ui_page_reads_sessions() {
    compare(&["sessions"], &["sessions"]).await;
}

#[tokio::test]
async fn ws11ui_page_reads_approval_history_does_not_wait_for_the_writer() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../../vectors/agent-page-reads.json"
    ))
    .unwrap();
    for kind in ["approvals_denied", "approvals_pending"] {
        let case = corpus["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["kind"] == kind)
            .unwrap();
        let t = TestApp::boot_frozen()
            .await
            .unwrap()
            .without_job_runner()
            .await;
        let sql: Vec<String> = case["sql"]
            .as_array()
            .unwrap()
            .iter()
            .map(|sql| sql.as_str().unwrap().into())
            .collect();
        t.db()
            .write(move |tx| {
                tx.conn().execute_batch("PRAGMA defer_foreign_keys=ON")?;
                for sql in sql {
                    tx.conn().execute(&sql, [])?;
                }
                Ok(())
            })
            .await
            .unwrap();
        let mut browser = t.david();
        let path = case["path"].as_str().unwrap();
        // Restore/refresh the session before holding the queue; the page itself is a reader.
        assert_eq!(browser.get(path).await.status, StatusCode::OK);
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
        let response =
            tokio::time::timeout(std::time::Duration::from_secs(5), browser.get(path)).await;
        release.send(()).unwrap();
        writer.await.unwrap().unwrap();
        assert_eq!(
            response
                .expect("non-expiring history must use readers, even while the writer is held")
                .status,
            StatusCode::OK
        );
    }
}
