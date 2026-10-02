//! Message-feature consumers of the real board writer, work recorder and inbox APIs.
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{ActivityItem, Timestamp, models::saved_item::SavedItem};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};
thread_local! { static WRITER_READS: RefCell<Option<Arc<Mutex<Vec<String>>>>> = const { RefCell::new(None) }; }
fn record_writer_read(event: rusqlite::trace::TraceEvent<'_>) {
    if let rusqlite::trace::TraceEvent::Stmt(_, sql) = event
        && (sql.trim_start().starts_with("SELECT") || sql.trim_start().starts_with("WITH"))
    {
        WRITER_READS.with(|slot| {
            if let Some(log) = slot.borrow().as_ref() {
                log.lock().unwrap().push(sql.into());
            }
        });
    }
}

fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/ws12_consumers.json"
    ))
    .unwrap()
}
async fn setup(app: &TestApp, commands: &Value) {
    let sql: Vec<_> = commands
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_owned())
        .collect();
    app.db()
        .write(move |tx| {
            for statement in sql {
                tx.conn().execute(&statement, [])?;
            }
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn ws12_board_message_consumers_match_rails_through_the_real_inbox() {
    let app = TestApp::boot_frozen()
        .await
        .unwrap()
        .without_job_runner()
        .await;
    setup(&app, &oracle()["setup"]).await;
    let mut browser = app.david();
    let mut differences = Vec::new();
    for step in oracle()["steps"].as_array().unwrap() {
        if step["operation"] == "remind" {
            let at = Timestamp::from_jiff(step["at"].as_str().unwrap().parse().unwrap());
            app.db()
                .write(move |tx| SavedItem::dispatch_due(tx, at))
                .await
                .unwrap();
            continue;
        }
        let response = browser
            .write(
                Req::new(
                    Method::from_bytes(step["method"].as_str().unwrap().to_uppercase().as_bytes())
                        .unwrap(),
                    step["path"].as_str().unwrap(),
                )
                .header("accept", "application/json")
                .header("content-type", "application/json")
                .body(serde_json::to_vec(&step["params"]).unwrap()),
            )
            .await;
        let body = if response.text().is_empty() {
            Value::Null
        } else {
            response.json()
        };
        if response.status.as_u16() != step["status"].as_u64().unwrap() as u16
            || body != step["body"]
            || response.header("content-type") != step["content_type"].as_str()
        {
            differences.push(format!(
                "{}: status {}; body {body}; Rails {}",
                step["label"], response.status, step["body"]
            ));
        }
    }
    assert!(
        differences.is_empty(),
        "WS12 consumer differences: {differences:?}"
    );
    println!(
        "WS8bm2 WS12 message consumers: 15/15 Rails workflow steps; reminders, re-fire, pin, scheduled board reply, slash, wrong stream and dependent inbox deletion"
    );
}
#[tokio::test]
async fn ws12_populated_work_inbox_preloads_owner_events_in_constant_queries() {
    let app = TestApp::boot_frozen()
        .await
        .unwrap()
        .without_job_runner()
        .await;
    setup(&app, &oracle()["setup"]).await;
    let mut browser = app.david();
    let mut counts = Vec::new();
    let mut differences = Vec::new();
    for page in oracle()["pages"].as_array().unwrap() {
        setup(&app, &page["sql"]).await;
        let log = app.db().capture_read_queries();
        let writer = Arc::new(Mutex::new(Vec::new()));
        let captured = writer.clone();
        app.db()
            .write(move |tx| {
                WRITER_READS.with(|slot| slot.replace(Some(captured)));
                tx.conn().trace_v2(
                    rusqlite::trace::TraceEventCodes::SQLITE_TRACE_STMT,
                    Some(record_writer_read),
                );
                Ok(())
            })
            .await
            .unwrap();
        let response = browser
            .send(
                Req::new(Method::GET, "/activity?type=threads")
                    .header("accept", "application/json"),
            )
            .await;
        app.db().stop_capturing_read_queries();
        app.db()
            .write(|tx| {
                tx.conn()
                    .trace_v2(rusqlite::trace::TraceEventCodes::empty(), None);
                WRITER_READS.with(|slot| slot.replace(None));
                Ok(())
            })
            .await
            .unwrap();
        assert_eq!(response.status, StatusCode::OK, "{}", response.text());
        assert_eq!(response.json(), page["body"], "{} items", page["size"]);
        let mut queries = log.lock().unwrap().clone();
        queries.extend(writer.lock().unwrap().iter().cloned());
        let work = queries
            .iter()
            .filter(|sql| {
                sql.contains("FROM work_thread_events")
                    || sql.contains("FROM \"work_thread_events\"")
            })
            .count();
        println!(
            "WS8bm2 WS12 work inbox: {} results; Rust {} reads / {work} work reads; Rails {} reads / {} work reads",
            page["size"],
            queries.len(),
            page["reads"],
            page["work_reads"]
        );
        assert!(!queries.iter().any(|sql| sql.contains("FROM \"users\" WHERE \"users\".\"id\" IN") || sql.contains("FROM \"rooms\" WHERE id IN")), "JSON work sources must not preload HTML actor/room associations: {queries:?}");
        counts.push(queries.len());
        if work != page["work_reads"].as_u64().unwrap() as usize {
            differences.push(format!(
                "{}: {work} work reads, Rails {}",
                page["size"], page["work_reads"]
            ));
        }
        // The public API's access query also governs work events; never serialize raw sources.
        let viewer = app
            .db()
            .read(|conn| campfire_db::User::find(conn, DAVID))
            .await
            .unwrap();
        assert_eq!(
            app.db()
                .read(move |conn| ActivityItem::accessible_to(conn, &viewer))
                .await
                .unwrap()
                .len(),
            page["size"].as_u64().unwrap() as usize
        );
    }
    assert!(
        differences.is_empty() && counts[0] == counts[1],
        "WS12 inbox must preload once: {differences:?}; total reads {counts:?}"
    );
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "DELETE FROM memberships WHERE room_id=699448332 AND user_id=?",
                [DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let revoked = browser
        .send(Req::new(Method::GET, "/activity?type=threads").header("accept", "application/json"))
        .await;
    assert_eq!(revoked.json()["activity_items"], json!([]));
}
