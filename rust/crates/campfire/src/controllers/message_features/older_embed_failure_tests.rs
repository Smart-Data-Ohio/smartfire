//! Actual job failure/deletion outcomes; response gates prove the read/write interleaving.
use super::quote_integration_tests::{app_rows, stream};
use crate::{
    controllers::presenters::test_support::*,
    integrations::{
        link_embed::{self, FetchJob},
        test_support::*,
    },
};
use campfire_db::Event;
use campfire_jobs::{Execution, JobQueue, Outcome, QueueConfig, Registry, RunnerConfig};
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
#[tokio::test]
async fn old_embed_deleted_during_fetch_and_failed_writes_match_rails_durable_job_outcomes() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/older_embed_failures.json"
    ))
    .unwrap();
    for group in oracle["groups"].as_array().unwrap() {
        for case in group["cases"].as_array().unwrap() {
            let app = app_rows(group["rows"].clone()).await;
            let (mut client, server) = stream(&app).await;
            let gid = campfire_views::helpers::gid_param(
                "ChannelThread",
                group["thread_id"].as_i64().unwrap(),
            );
            let signed = rails_compat::turbo::signed_stream_name(
                &app.booted.app.secrets,
                &[&gid, "messages"],
            );
            client
                .confirm(&crate::channels::tests::support::identifier(
                    json!({"channel":"RoomMessagesChannel","signed_stream_name":signed}),
                ))
                .await;
            let id = group["embed_id"].as_i64().unwrap();
            let sibling = group["sibling_id"].as_i64().unwrap();
            let name = case["name"].as_str().unwrap();
            let host = group["host"].as_str().unwrap();
            let path = group["path"].as_str().unwrap();
            let gate = Arc::new(ResponseGate::default());
            let mut route = Route::new("GET", host, path, 200)
                .header("Content-Type", "text/html")
                .body("<meta property=\"og:title\" content=\"After deterministic fetch\">");
            if name == "deleted_during_fetch" {
                route.response_gate = Some(gate.clone());
            }
            let (http, roots) =
                FakeServer::start_named_tls_ws15e(vec![route], vec![host.into()]).await;
            let resolver = Arc::new(FakeResolver::new([(host, vec!["93.184.216.34"])]));
            let dialer = Arc::new(MappingDialer {
                public: ["93.184.216.34".parse().unwrap()].into(),
                to: http.addr,
                dialed: Default::default(),
            });
            let mut net = network(resolver, dialer);
            net.tls = crate::net::tls_config(roots);
            if name == "write_failed" {
                app.db().write(move|tx|{tx.conn().execute_batch(&format!("CREATE TRIGGER ws8_failure BEFORE UPDATE ON link_embeds WHEN OLD.id={id} BEGIN SELECT RAISE(ABORT,'fixture embed writer failure'); END"))?;Ok(())}).await.unwrap();
            }
            let (send, recv) = tokio::sync::oneshot::channel();
            let send = Arc::new(std::sync::Mutex::new(Some(send)));
            let mut registry = Registry::new();
            registry.register(move |app: crate::app::App, job: FetchJob, _: Execution| {
                let net = net.clone();
                let send = send.clone();
                async move {
                    let result = link_embed::fetcher::fetch(&app, &net, job.embed_id).await;
                    send.lock()
                        .unwrap()
                        .take()
                        .unwrap()
                        .send(result.as_ref().map(|_| ()).map_err(ToString::to_string))
                        .unwrap();
                    result.map_err(crate::queue::discard_missing)?;
                    Ok(Outcome::Done)
                }
            });
            let config = RunnerConfig::new(vec![QueueConfig::new("default", 1)]);
            let queue = JobQueue::new(&registry, &config).unwrap();
            app.db()
                .write(move |tx| {
                    tx.conn().execute("DELETE FROM background_jobs", [])?;
                    tx.emit_after_commit(Event::job(&FetchJob { embed_id: id }));
                    Ok(())
                })
                .await
                .unwrap();
            if name == "deleted_before_job" {
                delete(&app, id).await;
            }
            let runner = campfire_jobs::start(
                app.db().clone(),
                queue,
                registry,
                app.booted.app.clone(),
                config,
            );
            if name == "deleted_during_fetch" {
                gate.entered.notified().await;
                delete(&app, id).await;
                gate.released.notify_one();
            }
            let result = recv.await.unwrap();
            assert!(
                result.is_err(),
                "{name}: deletion or writer failure must reach the consumer"
            );
            runner.shutdown(Duration::from_secs(1)).await;
            client.assert_silent().await;
            let calls = http
                .received
                .lock()
                .unwrap()
                .iter()
                .map(|r| {
                    assert!(r.header("Cookie").is_none());
                    assert!(r.header("Authorization").is_none());
                    json!({"method":r.method,"path":r.target})
                })
                .collect::<Vec<_>>();
            assert_eq!(json!(calls), case["calls"], "{name}");
            let expected = case["after"].clone();
            let outcome = case["outcome"].as_str().unwrap().to_owned();
            app.db()
                .read(move |c| {
                    let mut query =
                        c.prepare("SELECT * FROM link_embeds WHERE id IN (?,?) ORDER BY id")?;
                    let names = query
                        .column_names()
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>();
                    let rows = query
                        .query_map([id, sibling], |r| {
                            let mut row = serde_json::Map::new();
                            for (i, n) in names.iter().enumerate() {
                                let v: rusqlite::types::Value = r.get(i)?;
                                row.insert(
                                    n.clone(),
                                    match v {
                                        rusqlite::types::Value::Null => Value::Null,
                                        rusqlite::types::Value::Integer(v) => json!(v),
                                        rusqlite::types::Value::Text(v) => json!(v),
                                        _ => panic!("fixture scalar"),
                                    },
                                );
                            }
                            Ok(Value::Object(row))
                        })?
                        .collect::<rusqlite::Result<Vec<_>>>()?;
                    assert_eq!(
                        json!(rows),
                        expected,
                        "{outcome}: unchanged siblings and writer rollback"
                    );
                    let jobs = c
                        .prepare(
                            "SELECT status,attempts,last_error FROM background_jobs ORDER BY id",
                        )?
                        .query_map([], |r| {
                            Ok((
                                r.get::<_, String>(0)?,
                                r.get::<_, i64>(1)?,
                                r.get::<_, Option<String>>(2)?,
                            ))
                        })?
                        .collect::<rusqlite::Result<Vec<_>>>()?;
                    if outcome == "failed" {
                        assert_eq!(jobs.len(), 1);
                        assert_eq!(jobs[0].0, "failed");
                        assert_eq!(jobs[0].1, 1);
                        assert!(
                            jobs[0]
                                .2
                                .as_ref()
                                .unwrap()
                                .contains("fixture embed writer failure")
                        );
                    } else {
                        assert!(jobs.is_empty(), "discarded missing record acknowledged");
                    }
                    Ok(())
                })
                .await
                .unwrap();
            server.abort();
        }
    }
    println!(
        "WS8bm2 older-embed failure Rust: 12 real durable jobs; 4 deterministic deletions during GET; 4 discarded missing records; 4 failed writes; unchanged siblings; no frames or child jobs; no external network"
    );
}
async fn delete(app: &TestApp, id: i64) {
    app.db()
        .write(move |tx| {
            tx.conn().execute(
                "DELETE FROM link_embed_references WHERE link_embed_id=?",
                [id],
            )?;
            tx.conn()
                .execute("DELETE FROM link_embeds WHERE id=?", [id])?;
            Ok(())
        })
        .await
        .unwrap();
}
