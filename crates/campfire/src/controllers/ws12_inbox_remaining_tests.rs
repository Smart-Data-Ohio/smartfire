//! Full named inbox request sequences with the same persisted target across steps.
use super::presenters::test_support::{Req, TestApp, with_fixed_render_secrets};
use campfire_kit::Method;
use serde_json::{Value, json};
async fn compare(key: &str) {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../vectors/ws12_inbox_remaining.json"
    ))
    .unwrap();
    let row = oracle["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == key)
        .unwrap();
    let mut measured = Vec::new();
    for size in [10, 100] {
        let app = TestApp::boot_frozen()
            .await
            .unwrap()
            .without_job_runner()
            .await;
        let setup = row["setup"].clone();
        app.db().write(move |tx|{
        tx.conn().execute_batch("PRAGMA defer_foreign_keys=ON")?;
        for sql in setup.as_array().unwrap(){tx.conn().execute_batch(sql.as_str().unwrap())?;}
        for i in 0..size {
            let uid=901881000+i;
            tx.conn().execute("INSERT INTO users(id,name,role,status,created_at,updated_at) VALUES(?,'Quiet inbox roster member',0,0,?,?)",rusqlite::params![uid,tx.now(),tx.now()])?;
            tx.conn().execute("INSERT INTO memberships(room_id,user_id,involvement,created_at,updated_at) VALUES(486777696,?,'mentions',?,?)",rusqlite::params![uid,tx.now(),tx.now()])?;
        }
        Ok(())
    }).await.unwrap();
        let mut david = app.david();
        let mut jason = app.sign_in(super::presenters::test_support::JASON).await;
        let mut counts = Vec::new();
        for (index, step) in row["steps"].as_array().unwrap().iter().enumerate() {
            if step["method"] == "get" && !step["headers"]["content-type"].as_str().is_some_and(|value| value.starts_with("application/json")) { continue; }

            if let Some(sql) = step["sql"].as_array() {
                let sql = sql.clone();
                app.db()
                    .write(move |tx| {
                        for q in sql {
                            tx.conn().execute_batch(q.as_str().unwrap())?;
                        }
                        Ok(())
                    })
                    .await
                    .unwrap();
            }
            let request = Req::new(
                Method::from_bytes(step["method"].as_str().unwrap().to_uppercase().as_bytes())
                    .unwrap(),
                step["path"].as_str().unwrap(),
            )
            .header(
                "accept",
                step["accept"].as_str().unwrap_or("application/json"),
            )
            .header("turbo-frame", "activity_test")
            .header("content-type", "application/json")
            .body(serde_json::to_vec(&step["params"]).unwrap());
            let browser = if step["viewer"] == "jason" {
                &mut jason
            } else {
                &mut david
            };
            let queries = app.db().capture_queries();
            let response = with_fixed_render_secrets(browser.write(request)).await;
            app.db().stop_capturing_queries();
            counts.push(queries.lock().unwrap().len());
            assert_eq!(
                response.status.as_u16() as u64,
                step["status"].as_u64().unwrap(),
                "{key} step {index}: status {}",
                response.text()
            );
            if step["headers"]["content-type"].as_str().is_some_and(|value| value.starts_with("application/json")) && response.text() != step["body"].as_str().unwrap() {
                super::presenters::test_support::rails_mismatch(
                    &response.text(),
                    step["body"].as_str().unwrap(),
                    &format!("{key} step {index}: complete Rails response bytes"),
                );
            }
            for header in ["content-type", "cache-control", "pragma", "location"] {
                assert_eq!(
                    response.header(header),
                    step["headers"][header].as_str(),
                    "{key} step {index}: {header}"
                );
            }
            let facts=app.db().read(|conn|{
            let items=all(conn,"SELECT id,user_id,event_type,read_at,handled_at FROM activity_items ORDER BY id",[],|r|{
                let time=|t:campfire_db::Timestamp|t.jiff().strftime("%Y-%m-%dT%H:%M:%S.000Z").to_string();
                Ok(json!([r.get::<_,i64>(0)?,r.get::<_,i64>(1)?,r.get::<_,String>(2)?,r.get::<_,Option<campfire_db::Timestamp>>(3)?.map(time),r.get::<_,Option<campfire_db::Timestamp>>(4)?.map(time)]))
            })?;
            let approvals=all(conn,"SELECT id,status FROM agent_approvals ORDER BY id",[],|r|Ok(json!([r.get::<_,i64>(0)?,r.get::<_,String>(1)?])))?;
            Ok(json!({"items":items,"approvals":approvals}))
        }).await.unwrap();
            assert_eq!(
                facts, step["facts"],
                "{key} step {index}: original target, recipient, timestamps and expiry state"
            );
        }
        println!("WS12_INBOX_NAMED {key} roster={size} SELECTs={counts:?}");
        measured.push(counts);
    }
    assert_eq!(
        measured[0], measured[1],
        "{key}: reads stay flat across the room roster"
    );
}

#[tokio::test]
async fn ws12_inbox_c001_complete_named_rails_http_sequence() {
    compare("c001").await;
}

#[tokio::test]
async fn ws12_inbox_c005_complete_named_rails_http_sequence() {
    compare("c005").await;
}

#[tokio::test]
async fn ws12_inbox_c010_complete_named_rails_http_sequence() {
    compare("c010").await;
}

#[tokio::test]
async fn ws12_inbox_c013_complete_named_rails_http_sequence() {
    compare("c013").await;
}

#[tokio::test]
async fn ws12_inbox_c015_complete_named_rails_http_sequence() {
    compare("c015").await;
}

#[tokio::test]
async fn ws12_inbox_c016_complete_named_rails_http_sequence() {
    compare("c016").await;
}

#[tokio::test]
async fn ws12_inbox_c017_complete_named_rails_http_sequence() {
    compare("c017").await;
}

#[tokio::test]
async fn ws12_inbox_c018_complete_named_rails_http_sequence() {
    compare("c018").await;
}

#[tokio::test]
async fn ws12_inbox_c020_complete_named_rails_http_sequence() {
    compare("c020").await;
}

fn all(
    conn: &campfire_db::Connection,
    sql: &str,
    _: [(); 0],
    f: impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<Value>,
) -> campfire_db::Result<Vec<Value>> {
    Ok(conn
        .prepare(sql)?
        .query_map([], f)?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}
