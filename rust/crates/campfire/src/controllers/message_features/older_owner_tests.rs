//! Fizzy and X owner callbacks/jobs on real root and thread streams beyond both windows.
use super::quote_integration_tests::{app_rows, stream};
use crate::controllers::presenters::test_support::*;
use crate::integrations::{fizzy, twitter};
use campfire_db::Event;
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{Arc, Mutex},
};

const FIXTURE_TOKEN: &str = "fixture-fizzy-token";
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
        "../../../../../vectors/messaging/older_owner_callbacks.json"
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
    let (mut client, server) = stream(app).await;
    let gid =
        campfire_views::helpers::gid_param("ChannelThread", group["thread_id"].as_i64().unwrap());
    let signed =
        rails_compat::turbo::signed_stream_name(&app.booted.app.secrets, &[&gid, "messages"]);
    client
        .confirm(&crate::channels::tests::support::identifier(
            json!({"channel":"RoomMessagesChannel","signed_stream_name":signed}),
        ))
        .await;
    (client, server)
}
async fn frames(
    app: &TestApp,
    client: &mut crate::channels::tests::support::Client,
    expected: &Value,
) {
    super::comparison_support::frames(app, client, expected, "older_owner_tests.rs").await;
    client.assert_silent().await;
}

async fn windows(app: &TestApp, group: &Value) {
    for path in [
        format!("/rooms/{QUIET_CORNER}/messages"),
        format!(
            "/rooms/{QUIET_CORNER}/threads/{}/messages",
            group["thread_id"]
        ),
    ] {
        let response = app.david().get(&path).await;
        assert_eq!(response.status, axum::http::StatusCode::OK);
        for id in group["old_ids"].as_array().unwrap() {
            assert!(
                !response
                    .text()
                    .contains(&format!("data-message-id=\"{id}\"")),
                "old reference in {path}"
            );
        }
    }
}
fn update(
    tx: &mut campfire_db::Tx<'_>,
    kind: &str,
    id: i64,
    text: &str,
) -> campfire_db::Result<()> {
    if kind == "fizzy" {
        fizzy::cards::Card::find(tx.conn(), id)?.broadcast_updates(tx);
        return Ok(());
    }
    let post = twitter::post::Post::find(tx.conn(), id)?;
    post.save_card(
        tx,
        &twitter::fetcher::Card {
            url: post.view_url(),
            author_handle: post.author_handle.clone(),
            author_name: post.author_name.clone(),
            author_avatar_url: post.author_avatar_url.clone(),
            text: Some(text.into()),
            posted_at: post.posted_at,
            replies: post.replies,
            reposts: post.reposts,
            likes: post.likes,
            media: json!(post.media),
            quote: post.quote.clone(),
        },
    )
}
#[tokio::test]
async fn older_owner_callbacks_match_rails_with_flat_reads_and_silent_rollbacks() {
    let mut counts = HashMap::new();
    let mut differences = Vec::new();
    for group in oracle()["groups"].as_array().unwrap() {
        let app = app_rows(group["rows"].clone()).await;
        windows(&app, group).await;
        let (mut client, server) = subscriber(&app, group).await;
        let kind = group["kind"].as_str().unwrap().to_owned();
        let id = group["model_id"].as_i64().unwrap();
        let rolled_kind = kind.clone();
        let rolled: campfire_db::Result<()> = app
            .db()
            .write(move |tx| {
                if rolled_kind == "fizzy" {
                    // Match the oracle's private cache write. Card.broadcast_updates
                    // is an explicit job step, not a CardCache save callback.
                    let card = fizzy::cards::Card::find(tx.conn(), id)?;
                    let cache = fizzy::cards::Cache::for_viewer(tx, &card, DAVID)?;
                    cache.save(tx, Some(&json!({"title":"Rolled back secret"})), None, None)?;
                } else {
                    update(tx, &rolled_kind, id, "Rolled back secret")?;
                }
                Err(campfire_db::Error::Other("fixture rollback".into()))
            })
            .await;
        assert!(rolled.is_err());
        client.assert_silent().await;
        if kind == "fizzy" {
            app.db()
                .read(move |conn| {
                    assert!(
                        fizzy::cards::Cache::find(conn, id, DAVID)?.is_none(),
                        "rolled-back private cache row persisted"
                    );
                    Ok(())
                })
                .await
                .unwrap();
        } else {
            app.db()
                .read(move |conn| {
                    assert_eq!(
                        twitter::post::Post::find(conn, id)?.text.as_deref(),
                        Some("Old text")
                    );
                    Ok(())
                })
                .await
                .unwrap();
        }
        let queries = app.db().capture_read_queries();
        let writer = queries.clone();
        let operation = kind.clone();
        app.db()
            .write(move |tx| {
                WRITER.with(|slot| *slot.borrow_mut() = Some(writer));
                tx.conn().trace_v2(
                    rusqlite::trace::TraceEventCodes::SQLITE_TRACE_STMT,
                    Some(record),
                );
                let result = update(tx, &operation, id, "Updated <&> text");
                tx.conn()
                    .trace_v2(rusqlite::trace::TraceEventCodes::empty(), None);
                WRITER.with(|slot| *slot.borrow_mut() = None);
                result
            })
            .await
            .unwrap();
        app.db().stop_capturing_read_queries();
        frames(&app, &mut client, &group["callback"]["frames"]).await;
        let reads = queries
            .lock()
            .unwrap()
            .iter()
            .filter(|sql| {
                sql.trim_start().starts_with("SELECT") || sql.trim_start().starts_with("WITH")
            })
            .count();
        println!(
            "WS8bm2 older-owner {kind} {} references: Rust {reads} reads; Rails {} reads; {} exact frames",
            group["size"],
            group["callback"]["reads"],
            group["callback"]["frames"].as_array().unwrap().len()
        );
        if let Some(previous) = counts.insert(kind.clone(), reads)
            && previous != reads
        {
            differences.push(format!("{kind}: {previous} -> {reads}"));
        }
        windows(&app, group).await;
        server.abort();
    }
    assert!(
        differences.is_empty(),
        "owner callback N+1: {differences:?}"
    );
    println!(
        "WS8bm2 older-owner callbacks: 40/40 exact frames; flat 4/16 reads; 4/4 silent rollbacks; roots and replies outside both windows"
    );
}
#[tokio::test]
async fn older_owner_network_jobs_match_rails_on_real_streams() {
    use crate::integrations::test_support::{
        FakeResolver, FakeServer, MappingDialer, Route, network,
    };
    use campfire_jobs::{Execution, JobQueue, Outcome, QueueConfig, Registry, RunnerConfig};
    let mut counts = HashMap::new();
    let mut differences = Vec::new();
    for group in oracle()["groups"].as_array().unwrap() {
        let app = app_rows(group["rows"].clone()).await;
        let crypto = rails_compat::ar_encryption::ArEncryption::new(&app.booted.app.secrets);
        if group["kind"] == "fizzy" {
            app.db()
                .write(move |tx| {
                    fizzy::accounts::Account::relink(
                        tx,
                        &crypto,
                        &fizzy::accounts::Input {
                            user_id: DAVID,
                            account_id: "fixture-workspace",
                            account_name: None,
                            fizzy_user_id: None,
                            fizzy_user_name: None,
                            token: FIXTURE_TOKEN,
                        },
                    )
                    .map(|_| ())
                })
                .await
                .unwrap();
        }
        let (mut client, server) = subscriber(&app, group).await;
        for job in group["jobs"].as_array().unwrap() {
            let route = &job["route"];
            let host = route["host"].as_str().unwrap();
            let path = route["path"].as_str().unwrap();
            let (http, roots) = FakeServer::start_named_tls_ws15e(
                vec![
                    Route::new("GET", host, path, route["status"].as_u64().unwrap() as u16)
                        .body(route["body"].to_string()),
                ],
                vec![host.to_owned()],
            )
            .await;
            let resolver = Arc::new(FakeResolver::new([(host, vec!["93.184.216.34"])]));
            let dialer = Arc::new(MappingDialer {
                public: ["93.184.216.34".parse().unwrap()].into(),
                to: http.addr,
                dialed: Default::default(),
            });
            let mut net = network(resolver, dialer);
            net.tls = crate::integrations::net::tls_config(roots);
            let reads = Arc::new(Mutex::new(None));
            let observed = reads.clone();
            let mut registry = Registry::new();
            if group["kind"] == "fizzy" {
                registry.register(
                    move |app: crate::app::App, job: fizzy::fetch::FetchJob, _: Execution| {
                        let net = net.clone();
                        let observed = observed.clone();
                        async move {
                            let queries = app.db.capture_read_queries();
                            let result = fizzy::fetch::fetch(
                                &app,
                                &net,
                                "https://app.fizzy.do",
                                job.card_id,
                                job.user_id,
                            )
                            .await;
                            app.db.stop_capturing_read_queries();
                            *observed.lock().unwrap() = Some(queries.lock().unwrap().len());
                            result.map_err(crate::jobs::discard_missing)?;
                            Ok(Outcome::Done)
                        }
                    },
                );
            } else {
                registry.register(
                    move |app: crate::app::App, job: twitter::fetcher::FetchJob, _: Execution| {
                        let net = net.clone();
                        let observed = observed.clone();
                        async move {
                            let queries = app.db.capture_read_queries();
                            let result = twitter::fetcher::fetch(&app, &net, job.post_id).await;
                            app.db.stop_capturing_read_queries();
                            *observed.lock().unwrap() = Some(queries.lock().unwrap().len());
                            result.map_err(crate::jobs::discard_missing)?;
                            Ok(Outcome::Done)
                        }
                    },
                );
            }
            let config = RunnerConfig::new(vec![QueueConfig::new("default", 1)]);
            let queue = JobQueue::new(&registry, &config).unwrap();
            let runner = campfire_jobs::start(
                app.db().clone(),
                queue,
                registry,
                app.booted.app.clone(),
                config,
            );
            let id = group["model_id"].as_i64().unwrap();
            let fizzy = group["kind"] == "fizzy";
            app.db()
                .write(move |tx| {
                    tx.emit_after_commit(if fizzy {
                        Event::job(&fizzy::fetch::FetchJob {
                            card_id: id,
                            user_id: DAVID,
                        })
                    } else {
                        Event::job(&twitter::fetcher::FetchJob { post_id: id })
                    });
                    Ok(())
                })
                .await
                .unwrap();
            frames(&app, &mut client, &job["frames"]).await;
            runner.shutdown(std::time::Duration::from_secs(1)).await;
            let key = format!("{} {}", group["kind"], route["status"]);
            let reads = reads.lock().unwrap().expect("owner job completed");
            println!(
                "WS8bm2 older-owner job {key} {} references: {reads} reader reads; {} exact frames",
                group["size"],
                job["frames"].as_array().unwrap().len()
            );
            if let Some(previous) = counts.insert(key.clone(), reads)
                && previous != reads
            {
                differences.push(format!("{key}: {previous} -> {reads}"));
            }
            assert_eq!(http.received.lock().unwrap().len(), 1);
            if fizzy {
                let authorization = format!("Bearer {FIXTURE_TOKEN}");
                assert_eq!(
                    http.received.lock().unwrap()[0].header("Authorization"),
                    Some(authorization.as_str())
                );
            }
            let error = app
                .db()
                .read(move |conn| {
                    if fizzy {
                        Ok(fizzy::cards::Cache::find(conn, id, DAVID)?
                            .unwrap()
                            .fetch_error)
                    } else {
                        Ok(twitter::post::Post::find(conn, id)?.fetch_error)
                    }
                })
                .await
                .unwrap();
            assert_eq!(json!(error), job["fetch_error"]);
        }
        windows(&app, group).await;
        server.abort();
    }
    assert!(
        differences.is_empty(),
        "owner job renderer N+1: {differences:?}"
    );
    println!(
        "WS8bm2 older-owner jobs: 12/12 durable jobs with injected owner transports; 120/120 exact frames; 12/12 recorded errors; 6/6 runtime-built auth headers; no external network"
    );
}
