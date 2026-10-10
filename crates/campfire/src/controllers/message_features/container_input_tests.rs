//! Full actual HTTP envelopes, persisted tables and broadcasts.
//! Producer control: check_input_mutants.py changes the actual slash response
//! and saved-item status writes; neither expected state nor input is mutated.
use super::{
    comparison_support,
    quote_integration_tests::{app_rows, stream},
};
use crate::controllers::presenters::test_support::*;
use serde_json::{Value, json};
use std::collections::HashMap;
#[tokio::test]
async fn exceptional_root_and_nested_containers_match_rails_with_flat_reads() {
    let vector: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/container_inputs.json"
    ))
    .unwrap();
    assert_eq!(compare_feature_input_requests(vector).await, 84);
}
pub(super) async fn compare_feature_input_requests(vector: Value) -> usize {
    let mut counts = HashMap::new();
    let mut failures = vec![];
    let mut checked = 0;
    // The oracle runs with allow_forgery_protection off, so Rails' button_to omits
    // the per-request masked token that this app (correctly) renders.
    let authenticity_token =
        regex::Regex::new(r#"<input type="hidden" name="authenticity_token" value="[^"]*" />"#)
            .unwrap();
    let saved_item = regex::Regex::new(r#"(?s)<article id="saved_item_.*?</article>"#).unwrap();
    for group in vector["groups"].as_array().unwrap() {
        for case in group["cases"].as_array().unwrap() {
            // Independent fixed entropy on both runtimes. Never use an expected
            // message ID/UUID or frame to construct the observed result.
            let inputs = campfire_db::database::FixtureInputs {
                message_uuid: std::sync::Arc::new(|| "fixture-relative-message".into()),
                sqlite_now: campfire_db::Timestamp::parse_db(SEED_NOW).unwrap(),
            };
            let app =
                crate::test_support::with_message_inputs(inputs, app_rows(group["rows"].clone()))
                    .await
                    .without_job_runner()
                    .await;
            if let Some(zone) = case["zone"].as_str() {
                let zone = zone.to_owned();
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
            }
            let (mut socket, server) = stream(&app).await;
            let mut browser = app.david();
            browser.authenticity_token().await;
            let query_log = app.db().capture_queries();
            let response = browser
                .write(
                    Req::new(hyper::Method::POST, case["path"].as_str().unwrap())
                        .header("accept", "application/json")
                        .header("content-type", "application/json")
                        .body(serde_json::to_vec(&case["params"]).unwrap()),
                )
                .await;
            app.db().stop_capturing_queries();
            let reads = query_log.lock().unwrap().len();
            let actual = json!({"status":response.status.as_u16(),"content_type":response.header("content-type"),"body":response.text(),"location":response.header("location")});
            let expected = json!({"status":case["status"],"content_type":case["content_type"],"body":case["body"],"location":case["location"]});
            if actual != expected {
                failures.push(json!({"size":group["size"],"case":case["name"],"actual":actual,"expected":expected}));
            }
            let tables = comparison_support::expected_tables(group, case);
            app.db()
                .read(move |conn| {
                    for (table, expected) in &tables {
                        let ids = conn
                            .prepare(&format!("SELECT id FROM {table} ORDER BY id"))?
                            .query_map([], |r| r.get::<_, i64>(0))?
                            .collect::<rusqlite::Result<Vec<_>>>()?;
                        assert_eq!(
                            ids.len(),
                            expected.as_array().unwrap().len(),
                            "container persisted row count: {table}"
                        );
                        for (id, expected) in ids.into_iter().zip(expected.as_array().unwrap()) {
                            comparison_support::same_row(
                                &comparison_support::row(conn, table, id)?,
                                expected,
                                "container persisted rows",
                            );
                        }
                    }
                    Ok(())
                })
                .await
                .unwrap();
            comparison_support::published_frames(
                &app,
                &mut socket,
                &case["frames"],
                &format!(
                    "container actual publications {}",
                    case["name"].as_str().unwrap()
                ),
            )
            .await;
            socket.assert_silent().await;
            if case["name"] == "UTC/slash_0" {
                let reads = warm_message_zones(&app, group).await;
                if let Some(previous) = counts.insert("legacy warm four-zone fragments".into(), reads) {
                    assert_eq!(previous, reads, "warm zone physical read growth");
                }
                println!("WS8bm2 warm legacy message size={}: Rust {reads}; complete independently captured Rails partials and actual cache hits in four zones", group["size"]);
            }
            if !case["saved_page"].is_null() {
                // The Saved page renders each stored reminder; compare its actual items.
                let page = browser.get("/saved").await;
                let body = authenticity_token
                    .replace_all(&page.text(), "")
                    .into_owned();
                let items = saved_item
                    .find_iter(&body)
                    .map(|item| item.as_str().to_owned())
                    .collect::<Vec<_>>();
                let actual = json!({"status":page.status.as_u16(),"items":items});
                if actual != case["saved_page"] {
                    failures.push(json!({"size":group["size"],"case":case["name"],"saved_page":actual,"expected":case["saved_page"]}));
                }
            }
            let key = case["name"].as_str().unwrap().to_owned();
            println!(
                "WS8bm2 container {key} size={}: Rust {reads}; Rails {}",
                group["size"], case["reads"]
            );
            if let Some(previous) = counts.insert(key, reads) {
                assert_eq!(previous, reads, "container physical read growth");
            }
            checked += 1;
            server.abort();
        }
    }
    for failure in &failures {
        eprintln!("{failure}");
    }
    println!(
        "WS8bm2 container Rust: {checked} requests; {} envelope differences; complete persisted tables, ordered publications and flat reads",
        failures.len()
    );
    assert!(
        failures.is_empty(),
        "container actual HTTP envelope differs from Rails"
    );
    checked
}

/// Warm the real record-version API with UTC, then render the same persisted
/// message in three other zones and return to UTC. Every whole partial is from
/// an independently captured real Rails slash publication, not a rebuilt tag.
/// Producer control 12 in check_rendering_mutants.py collapses the real cache
/// key to UTC and must fail at the New York whole-fragment assertion.
async fn warm_message_zones(app: &TestApp, group: &Value) -> usize {
    use crate::controllers::presenters::{Presenter, page};
    let expected = group["cases"].as_array().unwrap().iter().filter_map(|case| {
        let name = case["name"].as_str()?;
        let zone = name.strip_suffix("/slash_0")?;
        let frame = case["frames"][0]["html"].as_str().unwrap();
        let html = frame.split_once("<template>").unwrap().1.strip_suffix("</template></turbo-stream>").unwrap();
        Some((zone.to_owned(), html.to_owned()))
    }).collect::<HashMap<_,_>>();
    assert_eq!(expected.len(), 4);
    let runtime = app.booted.app.clone();
    let queries = app.db().capture_queries();
    app.db().read(move |conn| {
        // The independently installed entropy provider identifies the observed
        // row. No expected frame or persisted-state field supplies this ID.
        let id = conn.query_row("SELECT id FROM messages WHERE client_message_id=?", ["fixture-relative-message"], |row| row.get::<_,i64>(0))?;
        let message = campfire_db::Message::find(conn,id)?;
        let account = campfire_db::Account::first(conn)?;
        for name in ["UTC", "America/New_York", "Australia/Lord_Howe", "Pacific/Apia", "UTC"] {
            let zone = campfire_views::time::Zone::lookup(name).unwrap();
            let mut presenter = Presenter::new(conn,&runtime,None);
            presenter.render_zone = zone.clone();
            let view = presenter.message(&message)?;
            page::render_detached_in_zone(&runtime,account.as_ref(),"http://campfire.test",&zone,|ctx| {
                let actual = campfire_views::messages::message(ctx,&view);
                assert_eq!(actual,expected[name],"warm zone actual legacy fragment differs from Rails: {name}");
                let stored = campfire_views::messages::cached_message_fragment_with_cards_in_zone(view.id,view.updated_at,&ctx.base_url,&view.components.github_cards_stamp,&zone).expect("warm zone actual legacy cache lookup missed its rendered fragment");
                assert_eq!(stored.as_str(),actual,"warm zone actual cached bytes differ from the real renderer");
                if name == "UTC" {
                    assert_eq!(campfire_views::messages::cached_message_fragment(view.id,view.updated_at,&ctx.base_url).unwrap().as_str(),actual,"default legacy cache reader must share the UTC renderer key");
                }
            });
        }
        Ok(())
    }).await.unwrap();
    app.db().stop_capturing_queries();
    queries.lock().unwrap().len()
}

/// Producer controls also exercise the real consumer output, not a separately
/// reconstructed parser result. See check_input_mutants.py and the report.
#[tokio::test]
async fn exceptional_relative_consumers_match_rails_complete_state_with_flat_reads() {
    let vector: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/relative_consumers.json"
    )).unwrap();
    assert_eq!(compare_feature_input_requests(vector).await, 80);
}

/// PR #223 review: `/event Review in 10^140 days` (and 10^142 hours) returned 500 in
/// test builds and a negative year in release. 10^130 days is the positive control.
#[tokio::test]
async fn relative_overflow_event_urls_match_fresh_rails() {
    let mut vector: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/relative_overflow_consumers.json"
    ))
    .unwrap();
    for group in vector["groups"].as_array_mut().unwrap() {
        group["cases"]
            .as_array_mut()
            .unwrap()
            .retain(|case| case["name"].as_str().unwrap().contains("/event_"));
    }
    assert_eq!(compare_feature_input_requests(vector).await, 6);
}

/// The same wide instants through `/remind`: notice, saved row, broadcast and Saved page.
#[tokio::test]
async fn relative_overflow_reminders_match_fresh_rails_rows_and_saved_page() {
    let mut vector: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/relative_overflow_consumers.json"
    ))
    .unwrap();
    for group in vector["groups"].as_array_mut().unwrap() {
        group["cases"]
            .as_array_mut()
            .unwrap()
            .retain(|case| case["name"].as_str().unwrap().contains("/remind_"));
    }
    assert_eq!(compare_feature_input_requests(vector).await, 4);
}

use campfire_web::controllers::presenters::Rendering;
