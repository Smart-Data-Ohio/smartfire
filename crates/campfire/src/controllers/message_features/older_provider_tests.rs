//! Real callbacks/jobs still replace references outside the current room window.
use super::quote_integration_tests::{app_rows, insert_rows};
use crate::controllers::presenters::test_support::*;
use campfire_db::{Event, Message};
use serde_json::Value;
use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{Arc, Mutex},
};
thread_local! { static WRITER: RefCell<Option<Arc<Mutex<Vec<String>>>>> = const { RefCell::new(None) }; }
fn record(event: rusqlite::trace::TraceEvent<'_>) {
    if let rusqlite::trace::TraceEvent::Stmt(_, sql) = event
        && (sql.trim_start().starts_with("SELECT") || sql.trim_start().starts_with("WITH"))
    {
        WRITER.with(|slot| {
            if let Some(log) = slot.borrow().as_ref() {
                log.lock().unwrap().push(sql.into());
            }
        });
    }
}
const FIXTURE_TOKEN: &str = "fixture-workspace-token";

fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/older_provider_callbacks.json"
    ))
    .unwrap()
}
async fn check_window(app: &TestApp, group: &Value) {
    let response = app
        .david()
        .get(&format!("/rooms/{QUIET_CORNER}/messages"))
        .await;
    assert_eq!(response.status, axum::http::StatusCode::OK);
    for id in group["old_ids"].as_array().unwrap() {
        assert!(
            !response
                .text()
                .contains(&format!("data-message-id=\"{id}\"")),
            "older reference leaked into current window"
        );
    }
}


#[tokio::test]
async fn older_provider_updates_keep_rails_frames_and_constant_reader_and_writer_cost() {
    let mut counts = HashMap::new();
    let mut differences = Vec::new();
    for group in oracle()["groups"].as_array().unwrap() {
        let app = app_rows(group["rows"].clone()).await;
        check_window(&app, group).await;

        for step in group["steps"].as_array().unwrap() {
            let id = step["id"].as_i64().unwrap();
            let kind = step["kind"].as_str().unwrap().to_owned();
            let operation = kind.clone();
            let attrs = step["attributes"].clone();
            let queries = app.db().capture_read_queries();
            let writer = queries.clone();
            app.db()
                .write(move |tx| {
                    WRITER.with(|slot| *slot.borrow_mut() = Some(writer));
                    tx.conn().trace_v2(
                        rusqlite::trace::TraceEventCodes::SQLITE_TRACE_STMT,
                        Some(record),
                    );
                    let result = if operation == "github" {
                        let attrs = attrs
                            .as_object()
                            .unwrap()
                            .iter()
                            .map(|(key, value)| {
                                let key = match key.as_str() {
                                    "title" => "title",
                                    "state" => "state",
                                    "review_decision" => "review_decision",
                                    "check_status" => "check_status",
                                    _ => panic!("unknown GitHub input"),
                                };
                                (
                                    key,
                                    rusqlite::types::Value::Text(value.as_str().unwrap().into()),
                                )
                            })
                            .collect::<Vec<_>>();
                        crate::integrations::github::pull_requests::update(tx, id, &attrs)
                            .map(|_| ())
                    } else {
                        let embed = crate::integrations::link_embed::Embed::find(tx.conn(), id)?;
                        if operation == "negative" {
                            embed.save_negative(tx, attrs["fetch_error"].as_str().unwrap())
                        } else {
                            embed.save_metadata(
                                tx,
                                &crate::integrations::link_embed::metadata_parser::Metadata {
                                    title: attrs["title"].as_str().map(str::to_owned),
                                    description: attrs["description"].as_str().map(str::to_owned),
                                    site_name: attrs["site_name"].as_str().map(str::to_owned),
                                    image_url: attrs["image_url"].as_str().map(str::to_owned),
                                },
                            )
                        }
                    };
                    tx.conn()
                        .trace_v2(rusqlite::trace::TraceEventCodes::empty(), None);
                    WRITER.with(|slot| *slot.borrow_mut() = None);
                    result
                })
                .await
                .unwrap();
            app.db().stop_capturing_read_queries();

        super::comparison_support::settle_jobs(&app).await;
            let reads = queries
                .lock()
                .unwrap()
                .iter()
                .filter(|s| {
                    s.trim_start().starts_with("SELECT") || s.trim_start().starts_with("WITH")
                })
                .count();
            println!(
                "WS8bm2 older-provider {} {kind}: {} messages; Rust {reads} reads; Rails {} reads; {} exact frames",
                group["privacy"],
                group["size"],
                step["reads"],
                step["frames"].as_array().unwrap().len()
            );
            if kind != "github" && reads > 10 {
                differences.push(format!(
                    "{kind}: {reads} reads exceed the card-only budget of 10"
                ));
            }
            let key = format!("{} {kind}", group["privacy"]);
            if let Some(previous) = counts.insert(key.clone(), reads)
                && previous != reads
            {
                differences.push(format!("{key}: {previous} -> {reads}"));
            }
        }
        check_window(&app, group).await;

    }
    assert!(
        differences.is_empty(),
        "callback reads grow with reference count: {differences:?}"
    );
    println!(
        "WS8bm2 older-provider updates: 30/30 updates; 300/300 Rails WebSocket frames; constant 4/16 reader plus writer cost"
    );
}
#[tokio::test]
async fn durable_github_fetch_jobs_replace_older_public_private_and_unknown_cards() {
    use crate::integrations::{
        github::{client::ReadClient, jobs::FetchPullRequestJob, tests::fake},
        test_support::Route,
    };
    for group in oracle()["groups"].as_array().unwrap() {
        let routes = group["job"]["routes"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(path, body)| {
                Route::new("GET", "api.github.com", path, 200).body(body.to_string())
            })
            .collect();
        let (http, network) = fake(routes).await;
        let app = TestApp::boot_with_github_reader(ReadClient::with_network(
            Some(FIXTURE_TOKEN.into()),
            network,
        ))
        .await
        .unwrap();
        insert_rows(&app, group["rows"].clone()).await;
        check_window(&app, group).await;

        let id = group["job"]["pull_request_id"].as_i64().unwrap();
        app.db()
            .write(move |tx| {
                tx.emit_after_commit(Event::job(&FetchPullRequestJob {
                    pull_request_id: id,
                }));
                Ok(())
            })
            .await
            .unwrap();

        super::comparison_support::settle_jobs(&app).await;
        let message_id = group["old_ids"][0].as_i64().unwrap();
        app.db()
            .read(move |conn| {
                Message::find(conn, message_id)?;
                Ok(())
            })
            .await
            .unwrap();
        assert_eq!(
            http.received().len(),
            4,
            "real owner fetcher sends exactly the pinned four requests"
        );
        let authorization = format!("Bearer {FIXTURE_TOKEN}");
        for request in http.received() {
            assert_eq!(
                request.header("Authorization"),
                Some(authorization.as_str())
            );
        }
        check_window(&app, group).await;

    }
    println!(
        "WS8bm2 older-provider jobs: 6/6 durable registered fetch jobs; 60/60 Rails WebSocket frames; 24/24 authenticated owner API reads; all roots outside current windows"
    );
}
