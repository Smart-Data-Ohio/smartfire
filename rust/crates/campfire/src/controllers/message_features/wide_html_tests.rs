//! Real production saved/scheduled view adapters and complete Rails partial bytes.
//! Producer controls: check_rendering_mutants.py corrupts the real saved date,
//! scheduled form date and HTML notice; each must fail its own output assertion.
use super::quote_integration_tests::{app_rows, insert_rows};
use crate::controllers::presenters::{Presenter, page, test_support::*};
use askama::Template;
use serde_json::Value;
use std::collections::HashMap;
#[tokio::test]
async fn wide_saved_and_scheduled_html_match_rails_with_flat_reads() {
    let vector: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/wide_date_html.json"
    ))
    .unwrap();
    let mut counts = HashMap::new();
    let mut cases = 0;
    for group in vector["groups"].as_array().unwrap() {
        for case in group["cases"].as_array().unwrap() {
            let app = app_rows(group["rows"].clone())
                .await
                .without_job_runner()
                .await;
            insert_rows(&app, case["rows"].clone()).await;
            let zone_name = case["zone"].as_str().unwrap().to_owned();
            let zone = campfire_views::time::Zone::lookup(&zone_name).unwrap();
            let saved_id = case["rows"]["saved_items"][0]["id"].as_i64().unwrap();
            let scheduled_id = case["rows"]["scheduled_messages"][0]["id"]
                .as_i64()
                .unwrap();
            app.db()
                .write(move |tx| {
                    tx.conn().execute(
                        "UPDATE users SET time_zone=? WHERE id=?",
                        rusqlite::params![zone_name, DAVID],
                    )?;
                    Ok(())
                })
                .await
                .unwrap();
            let runtime = app.booted.app.clone();
            let queries = app.db().capture_queries();
            let actual = app.db().read(move |conn| {
                let viewer = campfire_db::User::find(conn,DAVID)?;
                let mut presenter = Presenter::new(conn,&runtime,None);
                presenter.render_zone = zone.clone();
                let saved = crate::controllers::saved_items::view(&presenter, conn, &viewer, &campfire_db::SavedItem::find(conn,saved_id)?)?;
                let scheduled = crate::controllers::scheduled_messages::view(&presenter,conn,&viewer,&campfire_db::ScheduledMessage::find(conn,scheduled_id)?)?;
                let account = campfire_db::Account::first(conn)?;
                Ok(page::render_detached_in_zone(&runtime, account.as_ref(), "http://campfire.test", &zone, |ctx| serde_json::json!({
                    "saved":campfire_views::saved_items::ItemPartial{ctx,item:&saved,status_filter:"all"}.render().unwrap(),
                    "scheduled":campfire_views::scheduled_messages::ItemPartial{ctx,item:&scheduled,stranded:false}.render().unwrap(),
                    "past":campfire_views::scheduled_messages::PastPartial{ctx,item:&scheduled}.render().unwrap()
                })))
            }).await.unwrap();
            app.db().stop_capturing_queries();
            for part in ["saved", "scheduled", "past"] {
                assert_eq!(
                    actual[part], case["html"][part],
                    "wide HTML actual {part} partial differs from Rails: {} {}",
                    case["zone"], case["input"]
                );
            }
            let reads = queries.lock().unwrap().len();
            let key = format!("{}/{}", case["zone"], case["input"]);
            if let Some(previous) = counts.insert(key, reads) {
                assert_eq!(previous, reads, "wide HTML physical read growth");
            }
            println!(
                "WS8bm2 wide HTML size={} zone={} input={}: Rust {reads}; Rails {}",
                group["size"], case["zone"], case["input"], case["reads"]
            );
            cases += 1;
        }
    }
    assert_eq!(cases, 32);
    println!("WS8bm2 wide HTML Rust: {cases} cases; 96 byte-identical real partials; flat reads");
}

/// The layout's variable tokens/assets are outside WS8 ownership. Compare the
/// complete raw time tags, datetime-local fields and actual flash span.
/// Session-bound form tokens are outside this date projection; the detached
/// partial comparison above retains every whole-partial byte.
#[tokio::test]
async fn wide_saved_and_scheduled_http_dates_and_notices_match_rails() {
    let vector: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/wide_http_html.json"
    ))
    .unwrap();
    let mut counts = HashMap::new();
    let mut pairs = 0;
    for group in vector["groups"].as_array().unwrap() {
        for case in group["cases"].as_array().unwrap() {
            let app = app_rows(group["rows"].clone())
                .await
                .without_job_runner()
                .await;
            let zone = case["zone"].as_str().unwrap().to_owned();
            app.db()
                .write(move |tx| {
                    tx.conn().execute(
                        "UPDATE users SET time_zone=? WHERE id=?",
                        rusqlite::params![zone, DAVID],
                    )?;
                    Ok(())
                })
                .await
                .unwrap();
            let mut browser = app.david();
            browser.authenticity_token().await;
            let queries = app.db().capture_queries();
            let post = browser
                .write(
                    Req::new(hyper::Method::POST, case["path"].as_str().unwrap())
                        .header("accept", "text/html")
                        .header("content-type", "application/json")
                        .body(serde_json::to_vec(&case["params"]).unwrap()),
                )
                .await;
            assert_eq!(
                serde_json::json!({"status":post.status.as_u16(),"location":post.header("location")}),
                case["post"],
                "wide HTML actual redirect differs from Rails"
            );
            let get = browser.get(case["page"].as_str().unwrap()).await;
            app.db().stop_capturing_queries();
            assert_eq!(
                get.status.as_u16(),
                case["get"]["status"].as_u64().unwrap() as u16,
                "wide HTML actual GET status differs from Rails"
            );
            assert_eq!(
                get.header("content-type"),
                case["get"]["content_type"].as_str()
            );
            let prefix = if case["kind"] == "saved" {
                "<section class=\"saved-items__page\""
            } else {
                "<section class=\"scheduled-messages__page\""
            };
            let html = get.text();
            let section = raw_element(&html, prefix, "</section>");
            let notice = raw_element(
                &html,
                "<span class=\"for-screen-reader\" role=\"alert\" aria-atomic=\"true\">",
                "</span>",
            );
            for (field, pattern) in [
                ("date_tags", r"<time\b[^>]*>.*?</time>"),
                ("date_fields", r#"<input\b[^>]*type="datetime-local"[^>]*>"#),
            ] {
                let actual = regex::Regex::new(pattern)
                    .unwrap()
                    .find_iter(section)
                    .map(|m| m.as_str())
                    .collect::<Vec<_>>();
                assert_eq!(
                    serde_json::json!(actual),
                    case["get"][field],
                    "wide HTML actual HTTP {field} differ from Rails: {} {}",
                    case["zone"],
                    case["at"]
                );
            }
            assert_eq!(
                notice,
                case["get"]["notice"].as_str().unwrap(),
                "wide HTML actual HTTP notice differs from Rails"
            );
            let state = case["state"].clone();
            app.db()
                .read(move |conn| {
                    for (table, expected) in state.as_object().unwrap() {
                        let ids = conn
                            .prepare(&format!("SELECT id FROM {table} ORDER BY id"))?
                            .query_map([], |r| r.get::<_, i64>(0))?
                            .collect::<rusqlite::Result<Vec<_>>>()?;
                        assert_eq!(
                            ids.len(),
                            expected.as_array().unwrap().len(),
                            "wide HTML actual persisted count: {table}"
                        );
                        for (id, row) in ids.into_iter().zip(expected.as_array().unwrap()) {
                            super::comparison_support::same_row(
                                &super::comparison_support::row(conn, table, id)?,
                                row,
                                "wide HTML actual persisted rows",
                            );
                        }
                    }
                    Ok(())
                })
                .await
                .unwrap();
            let reads = queries.lock().unwrap().len();
            let key = format!("{}/{}/{}", case["kind"], case["zone"], case["at"]);
            if let Some(previous) = counts.insert(key, reads) {
                assert_eq!(previous, reads, "wide HTTP physical read growth");
            }
            println!(
                "WS8bm2 wide HTTP size={} kind={} zone={} at={}: Rust {reads}; Rails {}",
                group["size"], case["kind"], case["zone"], case["at"], case["reads"]
            );
            pairs += 1;
        }
    }
    assert_eq!(pairs, 48);
    println!(
        "WS8bm2 wide HTTP Rust: {pairs} actual POST/GET pairs; byte-identical date tags/fields and notices; full persisted rows; flat reads"
    );
}
fn raw_element<'a>(html: &'a str, prefix: &str, end: &str) -> &'a str {
    let from = html.find(prefix).expect("actual owned element present");
    let to = html[from..].find(end).expect("actual owned element closes") + from + end.len();
    &html[from..to]
}
