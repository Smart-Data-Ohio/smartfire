//! Real mapped-thread streams and older reply windows versus unmasked Rails bytes.
use super::quote_integration_tests::{app_rows, insert_rows};
use crate::controllers::presenters::{github, page, test_support::*};
use campfire_db::{ChannelThread, Event, Message};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{Arc, Mutex},
};

const FIXTURE_TOKEN: &str = "fixture-workspace-token";
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
fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/mapped_provider_callbacks.json"
    ))
    .unwrap()
}
async fn subscriber(
    app: &TestApp,
    group: &Value,
) -> (
    crate::channels::tests::support::Client,
    tokio::task::JoinHandle<()>,
) {
    use crate::channels::tests::support::{Client, bind_listener, identifier};
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let listener = bind_listener().await;
    let address = listener.local_addr().unwrap();
    let router = app.booted.router.clone();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut request = format!("ws://{address}/cable")
        .into_client_request()
        .unwrap();
    for (key, value) in [
        ("host", "campfire.test"),
        ("origin", "http://campfire.test"),
        ("cookie", david_cookie().as_str()),
    ] {
        request.headers_mut().insert(key, value.parse().unwrap());
    }
    let (socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    let mut client = Client { socket };
    assert_eq!(client.next_text().await, r#"{"type":"welcome"}"#);
    for (table, model) in [("rooms", "Room"), ("channel_threads", "ChannelThread")] {
        for row in group["rows"][table].as_array().unwrap() {
            let id = row["id"].as_i64().unwrap();
            // Room GIDs retain the concrete Rails STI type.
            let model = if table == "rooms" {
                row["type"].as_str().unwrap()
            } else {
                model
            };
            let gid = campfire_views::helpers::gid_param(model, id);
            let signed = rails_compat::turbo::signed_stream_name(
                &app.booted.app.secrets,
                &[&gid, "messages"],
            );
            client
                .confirm(&identifier(
                    json!({"channel":"RoomMessagesChannel", "signed_stream_name":signed}),
                ))
                .await;
        }
    }
    (client, server)
}
async fn frames(
    app: &TestApp,
    client: &mut crate::channels::tests::support::Client,
    expected: &Value,
) {
    super::comparison_support::frames(app, client, expected, "mapped_provider_tests.rs").await;
    client.assert_silent().await;
}

async fn windows(app: &TestApp, group: &Value, initial: bool) {
    let ids = regex::Regex::new(r#"data-message-id="(\d+)""#).unwrap();
    for window in group["windows"].as_array().unwrap() {
        let response = app.david().get(window["path"].as_str().unwrap()).await;
        assert_eq!(
            response.status,
            axum::http::StatusCode::OK,
            "{}",
            response.text()
        );
        let actual: Vec<i64> = ids
            .captures_iter(&response.text())
            .map(|c| c[1].parse().unwrap())
            .collect();
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            window["ids"],
            "{}",
            window["kind"]
        );
        if initial && window["kind"] == "before" {
            assert!(
                response
                    .text()
                    .contains(group["reply_cards"].as_str().unwrap()),
                "older reply card bytes in the real HTTP window"
            );
        }
    }
}
#[tokio::test]
async fn mapped_provider_headers_replies_callbacks_have_rails_bytes_and_flat_reads() {
    let mut counts = HashMap::new();
    let mut differences = Vec::new();
    for group in oracle()["groups"].as_array().unwrap() {
        let app = app_rows(group["rows"].clone()).await;
        windows(&app, group, true).await;
        let header = &group["initial_headers"][0];
        let thread = group["rows"]["channel_threads"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == header["thread_id"])
            .unwrap();
        let response = app
            .david()
            .get(&format!(
                "/rooms/{}/threads/{}",
                thread["room_id"], thread["id"]
            ))
            .await;
        assert_eq!(response.status, axum::http::StatusCode::OK);
        assert!(
            response.text().contains(header["html"].as_str().unwrap()),
            "mapped header bytes in the real thread page"
        );
        let initial = group["initial_headers"].clone();
        let old_id = group["old_reply_id"].as_i64().unwrap();
        let expected_cards = group["reply_cards"].clone();
        let state = app.booted.app.clone();
        app.db()
            .read(move |conn| {
                for header in initial.as_array().unwrap() {
                    let thread = ChannelThread::find(conn, header["thread_id"].as_i64().unwrap())?;
                    let html =
                        page::render_detached_at(&state, None, "http://campfire.test", |ctx| {
                            github::thread_header(conn, ctx, &thread)
                        })?;
                    assert_eq!(html, header["html"].as_str().unwrap());
                }
                let message = Message::find(conn, old_id)?;
                let html = github::message_cards_in_zone(
                    conn,
                    &state,
                    &message,
                    &page::renderer_time_zone(),
                )?;
                assert_eq!(html, expected_cards.as_str().unwrap());
                Ok(())
            })
            .await
            .unwrap();
        let (mut client, server) = subscriber(&app, group).await;
        let id = group["callback"]["pull_request_id"].as_i64().unwrap();
        let queries = app.db().capture_read_queries();
        let writer = queries.clone();
        app.db()
            .write(move |tx| {
                WRITER.with(|slot| *slot.borrow_mut() = Some(writer));
                tx.conn().trace_v2(
                    rusqlite::trace::TraceEventCodes::SQLITE_TRACE_STMT,
                    Some(record),
                );
                let result = crate::integrations::github::pull_requests::update(
                    tx,
                    id,
                    &[
                        (
                            "title",
                            rusqlite::types::Value::Text("Updated <&> title".into()),
                        ),
                        ("state", rusqlite::types::Value::Text("merged".into())),
                        (
                            "review_decision",
                            rusqlite::types::Value::Text("approved".into()),
                        ),
                        (
                            "check_status",
                            rusqlite::types::Value::Text("passing".into()),
                        ),
                    ],
                );
                tx.conn()
                    .trace_v2(rusqlite::trace::TraceEventCodes::empty(), None);
                WRITER.with(|slot| *slot.borrow_mut() = None);
                result.map(|_| ())
            })
            .await
            .unwrap();
        app.db().stop_capturing_read_queries();
        frames(&app, &mut client, &group["callback"]["frames"]).await;
        let reads = queries.lock().unwrap().len();
        let key = group["privacy"].to_string();
        println!(
            "WS8bm2 mapped-provider {key} {} mappings: Rust {reads} reads; Rails {} reads; {} exact frames",
            group["size"],
            group["callback"]["reads"],
            group["callback"]["frames"].as_array().unwrap().len()
        );
        if let Some(previous) = counts.insert(key.clone(), reads)
            && previous != reads
        {
            differences.push(format!("{key}: {previous} -> {reads}"));
        }
        windows(&app, group, false).await;
        server.abort();
    }
    assert!(
        differences.is_empty(),
        "mapped header callback N+1: {differences:?}"
    );
    println!(
        "WS8bm2 mapped-provider callbacks: 60/60 headers; 6/6 thread-page headers; 6/6 older HTTP reply cards; 36/36 reply-window checks; 180/180 exact frames; flat 4/16 reads"
    );
}
#[tokio::test]
async fn mapped_provider_durable_jobs_replace_headers_and_older_replies() {
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
        let (mut client, server) = subscriber(&app, group).await;
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
        frames(&app, &mut client, &group["job"]["frames"]).await;
        assert_eq!(http.received().len(), 5);
        let authorization = format!("Bearer {FIXTURE_TOKEN}");
        assert!(
            http.received()
                .iter()
                .all(|r| r.header("Authorization") == Some(authorization.as_str()))
        );
        windows(&app, group, false).await;
        server.abort();
    }
    println!(
        "WS8bm2 mapped-provider jobs: 6/6 registered durable jobs; 180/180 exact frames; 18/18 reply windows; 30/30 authenticated owner API reads"
    );
}
