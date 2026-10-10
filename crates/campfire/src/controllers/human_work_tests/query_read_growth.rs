//! #193 final-review probes: actual HTTP/SQLite reads plus unmasked Rails facts.
use super::*;
use campfire_db::{ChannelThread, NewChannelThread};

fn oracle() -> Value {
    serde_json::from_str(include_str!("../../../../../vectors/work_read_growth.json")).unwrap()
}

async fn fixture(row: &Value) -> TestApp {
    let app = TestApp::boot_frozen()
        .await
        .expect("default seed required")
        .without_job_runner()
        .await;
    setup(&app, row).await;
    app
}

#[tokio::test]
async fn work_json_read_growth_meets_rails_and_preserves_complete_responses() {
    let mut counts = std::collections::HashMap::<String, Vec<usize>>::new();
    let mut rails = std::collections::HashMap::<String, Vec<usize>>::new();
    for row in oracle()["json"].as_array().unwrap() {
        let app = fixture(row).await;
        let mut browser = app.david();
        browser.authenticity_token().await;
        assert_eq!(browser.get("/work?state=all").await.status, 302);
        let log = app.db().capture_read_queries();
        let response = browser.get(row["path"].as_str().unwrap()).await;
        app.db().stop_capturing_read_queries();
        assert_eq!(
            response.status.as_u16(),
            row["status"].as_u64().unwrap() as u16
        );
        assert_eq!(
            response.text(),
            row["body"].as_str().unwrap(),
            "complete unmasked JSON"
        );
        let count = log.lock().unwrap().len();
        let kind = row["kind"].as_str().unwrap();
        println!(
            "WS12 work JSON {kind} rows={}: {count} reader SQL",
            row["size"]
        );
        counts.entry(kind.into()).or_default().push(count);
        rails
            .entry(kind.into())
            .or_default()
            .push(row["reads"].as_u64().unwrap() as usize);
    }
    for (kind, counts) in counts {
        let rails = &rails[&kind];
        assert!(
            counts[1].saturating_sub(counts[0]) <= rails[1] - rails[0],
            "{kind} JSON read growth must not exceed Rails' {rails:?}: {counts:?}"
        );
        assert_eq!(
            counts[0], counts[1],
            "JSON page facts must be batched for {kind}"
        );
    }
}

thread_local! {
    static WRITER_READS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}
fn record_read(event: rusqlite::trace::TraceEvent<'_>) {
    if let rusqlite::trace::TraceEvent::Stmt(_, sql) = event
        && (sql.trim_start().starts_with("SELECT") || sql.trim_start().starts_with("WITH"))
    {
        WRITER_READS.with(|log| log.borrow_mut().push(sql.to_owned()));
    }
}

#[tokio::test]
async fn opening_fanout_read_growth_meets_rails_and_preserves_recorder_facts() {
    let mut counts = Vec::new();
    let mut rails = Vec::new();
    for row in oracle()["opening"].as_array().unwrap() {
        let app = fixture(row).await;
        let (thread, reads) = app
            .db()
            .write(|tx| {
                WRITER_READS.with(|log| log.borrow_mut().clear());
                tx.conn().trace_v2(
                    rusqlite::trace::TraceEventCodes::SQLITE_TRACE_STMT,
                    Some(record_read),
                );
                let result = ChannelThread::create_board_post(
                    tx,
                    NewChannelThread {
                        room_id: ALL_TALK,
                        creator_id: DAVID,
                        name: Some("Opening query probe".into()),
                        work_status: Some("planned".into()),
                        ..Default::default()
                    },
                    Some("Opening message".into()),
                );
                tx.conn()
                    .trace_v2(rusqlite::trace::TraceEventCodes::empty(), None);
                Ok((
                    result?,
                    WRITER_READS.with(|log| std::mem::take(&mut *log.borrow_mut())),
                ))
            })
            .await
            .unwrap();
        assert_eq!(thread.id, row["thread_id"].as_i64().unwrap());
        let items = app.db().read(move |conn| {
            let mut query = conn.prepare("SELECT id,user_id,source_type,source_id,event_type,read_at,handled_at,created_at,updated_at FROM activity_items WHERE source_type='Message' AND source_id IN (SELECT id FROM messages WHERE thread_id=?) ORDER BY user_id")?;
            Ok(query.query_map([thread.id], |r| Ok(serde_json::json!({
                "id":r.get::<_,i64>(0)?, "user_id":r.get::<_,i64>(1)?, "source_type":r.get::<_,String>(2)?,
                "source_id":r.get::<_,i64>(3)?, "event_type":r.get::<_,String>(4)?,
                "read_at":r.get::<_,Option<String>>(5)?, "handled_at":r.get::<_,Option<String>>(6)?,
                "created_at":r.get::<_,String>(7)?, "updated_at":r.get::<_,String>(8)?
            })))?.collect::<rusqlite::Result<Vec<_>>>()?)
        }).await.unwrap();
        assert_eq!(
            serde_json::json!(items),
            row["items"],
            "all recorder facts, including IDs and timestamps"
        );
        let users = reads
            .iter()
            .filter(|sql| sql.contains("FROM \"users\"") || sql.contains("FROM users"))
            .count();
        println!(
            "WS12 opening recipients={}: {} SELECTs; {users} user reads",
            row["recipients"],
            reads.len()
        );
        counts.push(reads.len());
        rails.push(row["reads"].as_u64().unwrap() as usize);
    }
    assert!(
        counts[1].saturating_sub(counts[0]) <= rails[1] - rails[0],
        "opening read growth must not exceed Rails' {rails:?}: {counts:?}"
    );
}
