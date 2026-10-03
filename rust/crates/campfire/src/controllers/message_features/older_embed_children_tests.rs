//! Real bounded fetch jobs: TLS/HTTP, old root/reply frames, stale sibling claims and rollbacks.
use super::{
    comparison_support::{embed_groups, embed_streams},
    quote_integration_tests::app_rows,
};
use crate::{
    controllers::presenters::test_support::*,
    integrations::{
        link_embed::{self, Embed, FetchJob},
        test_support::*,
    },
};
use campfire_db::Event;
use campfire_jobs::{Execution, JobQueue, Outcome, QueueConfig, Registry, RunnerConfig};
use serde_json::{Value, json};
use std::{collections::HashMap, sync::Arc, time::Duration};
fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/older_embed_children.json"
    ))
    .unwrap()
}
fn reads(sql: &[String]) -> usize {
    sql.iter()
        .filter(|s| s.trim_start().starts_with("SELECT") || s.trim_start().starts_with("WITH"))
        .count()
}
#[tokio::test]
async fn queued_stale_generic_and_linkedin_children_execute_after_parent_with_flat_reads() {
    let mut counts = HashMap::new();
    for group in embed_groups(oracle()) {
        let app = app_rows(group["rows"].clone()).await;
        let (mut client, server) = embed_streams(&app, group["thread_id"].as_i64().unwrap()).await;
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
        // Window rendering may claim the unrelated provider; restore the oracle snapshot.
        let restore = group["rows"]["link_embeds"].clone();
        app.db()
            .write(move |tx| {
                for row in restore.as_array().unwrap() {
                    tx.conn().execute(
                        "UPDATE link_embeds SET fetch_requested_at=? WHERE id=?",
                        rusqlite::params![
                            row["fetch_requested_at"].as_str(),
                            row["id"].as_i64().unwrap()
                        ],
                    )?;
                }
                tx.conn().execute("DELETE FROM background_jobs", [])?;
                Ok(())
            })
            .await
            .unwrap();
        for job in group["jobs"].as_array().unwrap() {
            let sibling = group["sibling_id"].as_i64().unwrap();
            app.db().write(move |tx| {tx.conn().execute("UPDATE link_embeds SET fetch_requested_at='2026-03-02 15:49:00',expires_at='2026-03-02 15:59:00' WHERE id=?",[sibling])?;Ok(())}).await.unwrap();
            let routes = job["routes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| {
                    let mut route = Route::new(
                        r["method"].as_str().unwrap(),
                        r["host"].as_str().unwrap(),
                        r["path"].as_str().unwrap(),
                        r["status"].as_u64().unwrap() as u16,
                    )
                    .body(r["body"].as_str().unwrap());
                    for (key, value) in r["headers"].as_object().unwrap() {
                        route = route.header(key, value.as_str().unwrap());
                    }
                    route
                })
                .collect();
            let host = job["routes"][0]["host"].as_str().unwrap();
            let (http, roots) = FakeServer::start_named_tls_ws15e(routes, vec![host.into()]).await;
            let resolver = Arc::new(FakeResolver::new([(host, vec!["93.184.216.34"])]));
            let dialer = Arc::new(MappingDialer {
                public: ["93.184.216.34".parse().unwrap()].into(),
                to: http.addr,
                dialed: Default::default(),
            });
            let mut net = network(resolver, dialer);
            net.tls = crate::integrations::net::tls_config(roots);
            let (completed, mut completion) = tokio::sync::mpsc::unbounded_channel();
            let mut registry = Registry::new();
            registry.register(move |app: crate::app::App, job: FetchJob, _: Execution| {
                let net = net.clone();
                let completed = completed.clone();
                async move {
                    let queries = app.db.capture_queries();
                    let result = link_embed::fetcher::fetch(&app, &net, job.embed_id).await;
                    app.db.stop_capturing_queries();
                    completed
                        .send((
                            job.embed_id,
                            result.as_ref().map(|_| ()).map_err(ToString::to_string),
                            reads(&queries.lock().unwrap()),
                        ))
                        .unwrap();
                    result.map_err(crate::jobs::discard_missing)?;
                    Ok(Outcome::Done)
                }
            });
            let config = RunnerConfig::new(vec![QueueConfig::new("default", 1)]);
            let queue = JobQueue::new(&registry, &config).unwrap();
            let runner = campfire_jobs::start(
                app.db().clone(),
                queue,
                registry,
                app.booted.app.clone(),
                config,
            );
            let id = group["embed_id"].as_i64().unwrap();
            app.db()
                .write(move |tx| {
                    tx.emit_after_commit(Event::job(&FetchJob { embed_id: id }));
                    Ok(())
                })
                .await
                .unwrap();
            let (parent_id, result, count) = completion.recv().await.unwrap();
            assert_eq!(parent_id, id);
            result.unwrap();
            let (child_id, result, child_count) = completion.recv().await.unwrap();
            assert_eq!(child_id, sibling);
            result.unwrap();
            runner.shutdown(Duration::from_secs(1)).await;
            super::comparison_support::frames(
                &app,
                &mut client,
                &job["frames"],
                &format!("{} {}", group["kind"], job["name"]),
            )
            .await;
            client.assert_silent().await;
            let calls = http
                .received
                .lock()
                .unwrap()
                .iter()
                .map(|r| {
                    assert!(r.header("Cookie").is_none());
                    assert!(r.header("Authorization").is_none());
                    json!({"host":r.header("Host").expect("received HTTP Host header"),"method":r.method,"path":r.target})
                })
                .collect::<Vec<_>>();
            assert_eq!(json!(calls), job["calls"], "actual wire HTTP calls");
            let expected = job["state"].clone();
            let expected_child = job["child"].clone();
            let ids = [
                group["opposite_id"].as_i64().unwrap(),
                group["suppressed_id"].as_i64().unwrap(),
            ];
            app.db().read(move |c| {
                let saved=Embed::find(c,id)?;assert_eq!(json!({"title":saved.title,"description":saved.description,"site_name":saved.site_name,"image_url":saved.image_url,"fetch_error":saved.fetch_error}),json!({"title":expected["title"],"description":expected["description"],"site_name":expected["site_name"],"image_url":expected["image_url"],"fetch_error":expected["fetch_error"]}));
                for (actual,key) in [(saved.fetched_at,"fetched_at"),(saved.expires_at,"expires_at")] {assert_eq!(actual.unwrap(),campfire_db::Timestamp::parse_db(expected[key].as_str().unwrap()).unwrap());}
                let jobs=c.prepare("SELECT arguments FROM background_jobs WHERE job_class='LinkEmbed::FetchJob' ORDER BY id")?.query_map([],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?.into_iter().map(|r|serde_json::from_str::<Value>(&r).unwrap()["embed_id"].clone()).collect::<Vec<_>>();assert!(jobs.is_empty(), "both actual jobs must be acknowledged");
                let child=Embed::find(c,sibling)?;
                assert_eq!(json!({"title":child.title,"description":child.description,"site_name":child.site_name,"image_url":child.image_url,"fetch_error":child.fetch_error}),json!({"title":expected_child["title"],"description":expected_child["description"],"site_name":expected_child["site_name"],"image_url":expected_child["image_url"],"fetch_error":expected_child["fetch_error"]}));
                for (actual,key) in [(child.fetched_at,"fetched_at"),(child.expires_at,"expires_at")] { assert_eq!(actual.unwrap(),campfire_db::Timestamp::parse_db(expected_child[key].as_str().unwrap()).unwrap()); }
                for id in ids {assert_eq!(Embed::find(c,id)?.fetch_requested_at.unwrap().jiff(),"2026-03-02T15:49:00Z".parse::<jiff::Timestamp>().unwrap());}
                assert_eq!(Embed::find(c,sibling)?.fetch_requested_at.unwrap().jiff(),"2026-03-02T16:00:00Z".parse::<jiff::Timestamp>().unwrap());Ok(())
            }).await.unwrap();
            app.db()
                .write(|tx| {
                    tx.conn().execute("DELETE FROM background_jobs", [])?;
                    Ok(())
                })
                .await
                .unwrap();
            let key = format!(
                "{} {}",
                group["kind"].as_str().unwrap(),
                job["name"].as_str().unwrap()
            );
            println!(
                "WS8bm2 older-embed job Rust {key} size={}: {count} consumer reads; Rails={}; {} exact frames",
                group["size"],
                job["reads"],
                job["frames"].as_array().unwrap().len()
            );
            println!(
                "WS8bm2 older-embed child Rust {key} size={}: {child_count} consumer reads; Rails={}",
                group["size"], job["child_reads"]
            );
            if let Some(before) = counts.insert(format!("{key} child"), child_count) {
                assert_eq!(child_count, before, "child consumer query growth");
            }
            if let Some(before) = counts.insert(key, count) {
                assert_eq!(count, before, "fetch consumer query growth");
            }
        }
        let id = group["embed_id"].as_i64().unwrap();
        let sibling = group["sibling_id"].as_i64().unwrap();
        app.db()
            .write(move |tx| {
                tx.conn().execute(
                    "UPDATE link_embeds SET fetch_requested_at='2026-03-02 15:49:00' WHERE id=?",
                    [sibling],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        for nested in [false, true] {
            let before = app
                .db()
                .read(move |c| Ok((Embed::find(c, id)?, Embed::find(c, sibling)?)))
                .await
                .unwrap();
            let rolled: campfire_db::Result<()> = app
                .db()
                .write(move |tx| {
                    let result = tx.savepoint(|tx| {
                        Embed::find(tx.conn(), id)?.save_metadata(
                            tx,
                            &link_embed::metadata_parser::Metadata {
                                title: Some("Rolled back secret".into()),
                                ..Default::default()
                            },
                        )?;
                        Err(campfire_db::Error::Other("fixture rollback".into()))
                    });
                    if nested {
                        assert!(result.is_err());
                        Ok(())
                    } else {
                        result
                    }
                })
                .await;
            assert_eq!(rolled.is_ok(), nested);
            client.assert_silent().await;
            app.db()
                .read(move |c| {
                    assert_eq!(Embed::find(c, id)?, before.0);
                    assert_eq!(Embed::find(c, sibling)?, before.1);
                    assert_eq!(
                        c.query_row("SELECT COUNT(*) FROM background_jobs", [], |r| r
                            .get::<_, i64>(0))?,
                        0
                    );
                    Ok(())
                })
                .await
                .unwrap();
        }
        server.abort();
    }
    println!(
        "WS8bm2 older-embed jobs Rust: 20 real parent network jobs; 400 exact Rails frames; 20 actual queued same-provider children executed; 8 silent outer/savepoint rollbacks; flat consumer reads; no external network"
    );
}

/// An observed legal Rails wire order must not fail the Rust job comparator.
/// Replay actual Redis/Cable captures through the real socket, preserving every
/// payload and duplicate. Job execution/serialization remain tested separately.
#[tokio::test]
async fn recorded_rails_delivery_orders_preserve_all_queued_child_frames() {
    use super::quote_integration_tests::{insert_rows, stream};
    use crate::channels::tests::support::identifier;
    let groups = embed_groups(oracle());
    let app = app_rows(groups[0]["rows"].clone()).await;
    for group in &groups[1..] {
        insert_rows(&app, group["rows"].clone()).await;
    }
    let (mut client, server) = stream(&app).await;
    for group in &groups {
        let gid = campfire_views::helpers::gid_param(
            "ChannelThread",
            group["thread_id"].as_i64().unwrap(),
        );
        let signed =
            rails_compat::turbo::signed_stream_name(&app.booted.app.secrets, &[&gid, "messages"]);
        client
            .confirm(&identifier(
                json!({"channel":"RoomMessagesChannel","signed_stream_name":signed}),
            ))
            .await;
    }
    let source = oracle();
    let transcript = source["groups"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|g| g["jobs"].as_array().unwrap())
        .flat_map(|j| j["frames"].as_array().unwrap())
        .cloned()
        .collect::<Vec<_>>();
    let orders: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/older_embed_wire_orders.json"
    ))
    .unwrap();
    let case = &orders["cases"][(super::comparison_support::embed_seed() % 2) as usize];
    // Replay an actual allowed Rails arrival order, not an invented permutation.
    // This is a transport/comparator regression; the test above executes real jobs.
    // No await: the real connection reads a backlog within its unchanged capacity.
    for index in case["delivery_indices"].as_array().unwrap() {
        let frame = &transcript[index.as_u64().unwrap() as usize];
        assert_eq!(
            app.booted.app.cable.broadcast(
                frame["stream"].as_str().unwrap(),
                frame["html"].as_str().unwrap()
            ),
            1
        );
    }
    super::comparison_support::frames(
        &app,
        &mut client,
        &json!(transcript),
        "recorded Rails delivery order",
    )
    .await;
    client.assert_silent().await;
    server.abort();
    println!(
        "WS8bm2 Rails delivery replay: 400 exact envelopes; unchanged capacity/deadlines; real socket and five authorized streams"
    );
}
