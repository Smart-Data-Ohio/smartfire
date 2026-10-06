//! Complete unmasked production HTTP output at 4/16 actual visible rows.
//! Producer controls: check_list_scaling_mutants.py changes real filenames,
//! mention JSON and blob-read batching; each must fail its intended assertion.
use super::quote_integration_tests::app_rows;
use crate::controllers::presenters::test_support::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[tokio::test]
async fn files_and_mentions_match_fresh_rails_with_flat_visible_row_reads() {
    let vector: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/list_scaling.json"
    ))
    .unwrap();
    let section = regex::Regex::new(r#"(?s)<section class="room-files".*?</section>"#).unwrap();
    let mut counts = BTreeMap::new();
    let mut captures = 0;
    for group in vector["groups"].as_array().unwrap() {
        let app = app_rows(group["rows"].clone()).await;
        let rows = group["rows"].clone();
        let zone = group["zone"].as_str().unwrap().to_owned();
        app.db()
            .write(move |tx| {
                for table in [
                    "active_storage_blobs",
                    "active_storage_attachments",
                    "drive_attachments",
                ] {
                    for row in rows[table].as_array().unwrap() {
                        let row = row.as_object().unwrap();
                        let columns = row
                            .keys()
                            .map(|key| format!("\"{key}\""))
                            .collect::<Vec<_>>()
                            .join(",");
                        let values = row.values().map(|value| match value {
                            Value::Null => rusqlite::types::Value::Null,
                            Value::Number(n) if n.is_i64() => {
                                rusqlite::types::Value::Integer(n.as_i64().unwrap())
                            }
                            Value::Number(n) => rusqlite::types::Value::Real(n.as_f64().unwrap()),
                            Value::String(s) => rusqlite::types::Value::Text(s.clone()),
                            _ => panic!("non-scalar SQL input"),
                        });
                        tx.conn().execute(
                            &format!(
                                "INSERT INTO {table} ({columns}) VALUES ({})",
                                vec!["?"; row.len()].join(",")
                            ),
                            rusqlite::params_from_iter(values),
                        )?;
                    }
                }
                tx.conn().execute(
                    "UPDATE users SET time_zone=? WHERE id=?",
                    rusqlite::params![zone, DAVID],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let mut browser = app.david();
        let size = group["size"].as_u64().unwrap() as usize;
        for case in group["cases"].as_array().unwrap() {
            let path = case["path"].as_str().unwrap();
            let mentions = path.starts_with("/autocompletable");
            for (pass, expected) in case["observations"].as_array().unwrap().iter().enumerate() {
                let queries = app.db().capture_queries();
                let response = browser
                    .send(Req::new(hyper::Method::GET, path).header(
                        "accept",
                        if mentions {
                            "application/json"
                        } else {
                            "text/html"
                        },
                    ))
                    .await;
                app.db().stop_capturing_queries();
                let reads = queries.lock().unwrap().len();
                let text = response.text();
                let body = if mentions {
                    text.as_str()
                } else {
                    section
                        .find(&text)
                        .expect("actual full Files section")
                        .as_str()
                };
                if mentions {
                    assert_eq!(
                        serde_json::from_str::<Value>(body)
                            .unwrap()
                            .as_array()
                            .unwrap()
                            .len(),
                        size,
                        "actual visible mention premise"
                    );
                } else {
                    assert_eq!(
                        body.matches("class=\"drive-attachment room-files__drive-link\"")
                            .count(),
                        size,
                        "actual visible Drive premise"
                    );
                    if !path.contains('?') {
                        assert_eq!(
                            body.matches("class=\"room-files__name\"").count(),
                            size,
                            "actual visible upload premise"
                        );
                    }
                }
                assert_eq!(
                    json!({"status":response.status.as_u16(),"content_type":response.header("content-type"),"body":body,
                    "total_count":response.header("x-total-count"),"link":response.header("link")}),
                    json!({"status":expected["status"],"content_type":expected["content_type"],"body":expected["body"],
                    "total_count":expected["total_count"],"link":expected["link"]}),
                    "actual visible-list envelope differs from Rails: {path}"
                );
                let key = format!("{}/{path}/{pass}", group["zone"].as_str().unwrap());
                if let Some((small, rails_small)) =
                    counts.insert(key, (reads, expected["reads"].as_i64().unwrap()))
                {
                    assert_eq!(
                        small, reads,
                        "visible-list physical reads grow per row: {path}"
                    );
                    assert!(
                        reads as i64 - small as i64
                            <= expected["reads"].as_i64().unwrap() - rails_small,
                        "visible-list read growth exceeds Rails"
                    );
                }
                println!(
                    "WS8bm2 visible-list Rust size={size} zone={} path={path} pass={pass}: {reads} reads; Rails {}",
                    group["zone"].as_str().unwrap(),
                    expected["reads"]
                );
                captures += 1;
            }
        }
    }
    assert_eq!(captures, 24);
    println!(
        "WS8bm2 visible-list Rust: 24 complete HTTP captures byte-identical without masks; actual visible uploads, Drive and mentions at 4/16; flat physical reads"
    );
}
