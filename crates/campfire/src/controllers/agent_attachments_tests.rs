//! Real multipart staging, signed-input errors and atomic callbacks.
use serde_json::Value;
async fn group(bot: bool) {
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agent_attachments_http.json"
    ))
    .unwrap();
    for case in vectors["cases"].as_array().unwrap().iter().filter(|case| {
        case["name"]
            .as_str()
            .unwrap()
            .starts_with("attachment_bot_")
            == bot
    }) {
        super::agent_reads_tests::check(case).await;
    }
}
#[tokio::test]
async fn agent_attachments_rest_bytes() {
    group(false).await;
}
#[tokio::test]
async fn agent_attachments_bot_bytes() {
    group(true).await;
}

fn stored_files(root: &std::path::Path) -> std::collections::BTreeSet<std::path::PathBuf> {
    let mut files = std::collections::BTreeSet::new();
    for entry in std::fs::read_dir(root).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            files.extend(stored_files(&entry.path()));
        } else {
            files.insert(entry.path());
        }
    }
    files
}
async fn row_counts(app: &super::presenters::test_support::TestApp) -> Vec<i64> {
    app.db()
        .read(|conn| {
            [
                "messages",
                "active_storage_blobs",
                "active_storage_attachments",
                "background_jobs",
            ]
            .iter()
            .map(|table| {
                Ok(
                    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                        row.get(0)
                    })?,
                )
            })
            .collect()
        })
        .await
        .unwrap()
}
async fn attachment_rollback(bot: bool, queue_failure: bool) {
    use super::agent_http_tests::{AGENT, SECRET, setup};
    use super::presenters::test_support::Req;
    use campfire_kit::Method;
    let app = setup().await.without_job_runner().await;
    app.db().write(move |tx| {
        tx.conn().execute("INSERT INTO agent_grants(agent_id,capability,granted_by_id,created_at,updated_at) VALUES(?,'post_messages',127326141,?,?)",rusqlite::params![AGENT,tx.now(),tx.now()])?;
        if bot {
            use sha2::{Digest, Sha256};
            tx.conn().execute("UPDATE users SET bot_token_digest=? WHERE id=394959859", [format!("{:x}", Sha256::digest("BenderToken1"))])?;
        }
        if queue_failure { tx.conn().execute_batch("CREATE TRIGGER ws11api_reject_attachment_job BEFORE INSERT ON background_jobs WHEN NEW.job_class='Room::PushMessageJob' BEGIN SELECT RAISE(ABORT,'WS11-api attachment queue unavailable'); END")?; }
        else { tx.conn().execute("UPDATE rooms SET type='Rooms::Board' WHERE id=486777696", [])?; }
        Ok(())
    }).await.unwrap();
    let before_rows = row_counts(&app).await;
    let before_files = stored_files(app.booted.app.storage.service.root());
    let (_server, mut subscriber) = super::agent_reactions_tests::subscribe(&app).await;
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agent_attachments_http.json"
    ))
    .unwrap();
    let case = vectors["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| {
            case["name"]
                == if bot {
                    "attachment_bot_0"
                } else {
                    "attachment_rest_0"
                }
        })
        .unwrap();
    let mut request = Req::new(Method::POST, case["path"].as_str().unwrap())
        .header("accept", "application/json")
        .header(
            "content-type",
            case["setup"]["content_type"].as_str().unwrap(),
        )
        .body(case["body"].as_str().unwrap());
    if !bot {
        request = request.header("authorization", &["Bearer", SECRET].join(" "));
    }
    let response = app.anonymous().send(request).await;
    assert_eq!(
        response.status.as_u16(),
        if queue_failure { 500 } else { 422 },
        "{}",
        response.text()
    );
    assert_eq!(
        row_counts(&app).await,
        before_rows,
        "attachment denial/queue failure must roll back every row"
    );
    assert_eq!(
        stored_files(app.booted.app.storage.service.root()),
        before_files,
        "attachment denial/queue failure must remove the staged file"
    );
    subscriber.assert_silent().await;
}
#[tokio::test]
async fn agent_attachment_queue_failure_rest() {
    attachment_rollback(false, true).await;
}
#[tokio::test]
async fn agent_attachment_queue_failure_bot() {
    attachment_rollback(true, true).await;
}
#[tokio::test]
async fn agent_attachment_board_denial_rest() {
    attachment_rollback(false, false).await;
}
