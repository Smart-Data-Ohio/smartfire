//! Pinned Rails callback digests above SQLite's bind limit, including cross-batch fetch claims.
use super::quote_integration_tests::{app_rows, stream};
use crate::controllers::presenters::test_support::*;
use crate::integrations::link_embed::{Embed, metadata_parser::Metadata};
use serde_json::Value;
use sha2::{Digest, Sha256};

fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/bounded_provider_callbacks.json"
    ))
    .unwrap()
}
#[test]
fn fixture_credentials_are_composed_at_runtime() {
    let token = "fixture-workspace-token";
    let header = format!("Bearer {token}");
    for source in [
        include_str!("older_provider_tests.rs"),
        include_str!("../../../../../reference-tools/messaging/older_provider_callbacks.rb"),
    ] {
        assert!(
            !source.contains(&header),
            "literal fixture authorization header remains"
        );
    }
}
async fn large_callback(index: usize) {
    let case = oracle()["cases"][index].clone();
    let app = app_rows(case["rows"].clone()).await;
    let input = case.clone();
    app.db().write(move |tx| {
        let count = input["count"].as_i64().unwrap();
        let base = input["base_id"].as_i64().unwrap();
        let kind = input["kind"].as_str().unwrap();
        tx.conn().execute("WITH RECURSIVE ids(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM ids WHERE n<?1) INSERT INTO messages (id,room_id,creator_id,client_message_id,embeds_suppressed,created_at,updated_at) SELECT ?2+n,?3,?4,?5||n,?6,'2026-03-01 16:00:00','2026-03-01 16:00:00' FROM ids", rusqlite::params![count,base,QUIET_CORNER,DAVID,format!("bounded-{kind}-"),input["suppressed"].as_bool().unwrap()])?;
        let id = input["model_id"].as_i64().unwrap();
        if kind == "github" {
            tx.conn().execute("INSERT INTO github_pull_request_references (message_id,github_pull_request_id,created_at,updated_at) SELECT id,?1,'2026-03-01 16:00:00','2026-03-01 16:00:00' FROM messages WHERE id>?2", [id,base])?;
        } else {
            for (position,embed) in input["rows"]["link_embeds"].as_array().unwrap().iter().enumerate() {
                tx.conn().execute("INSERT INTO link_embed_references (message_id,link_embed_id,position,url,created_at,updated_at) SELECT id,?1,?2,?3,'2026-03-01 16:00:00','2026-03-01 16:00:00' FROM messages WHERE id>?4",rusqlite::params![embed["id"].as_i64().unwrap(),position,embed["normalized_url"].as_str().unwrap(),base])?;
            }
        }
        Ok(())
    }).await.unwrap();
    let (mut client, server) = stream(&app).await;
    let expected = case.clone();
    let receive = tokio::spawn(async move {
        let mut digest = Sha256::new();
        for n in 1..=expected["count"].as_u64().unwrap() {
            let actual: Value = serde_json::from_str(&client.next_text().await).unwrap();
            let html = actual["message"].as_str().unwrap();
            assert!(
                html.contains(&format!(
                    "bounded-{}-{n}\"",
                    expected["kind"].as_str().unwrap()
                )),
                "frame order at {n}"
            );
            if n == 1 {
                assert_eq!(html, expected["first"].as_str().unwrap());
            }
            if n == expected["count"].as_u64().unwrap() {
                assert_eq!(html, expected["last"].as_str().unwrap());
            }
            digest.update(html.as_bytes());
            digest.update(b"\n");
        }
        assert_eq!(
            format!("{:x}", digest.finalize()),
            expected["sha256"].as_str().unwrap()
        );
    });
    let id = case["model_id"].as_i64().unwrap();
    let kind = case["kind"].as_str().unwrap().to_owned();
    let operation = kind.clone();
    let result = app
        .db()
        .write(move |tx| {
            if operation == "github" {
                crate::integrations::github::pull_requests::update(
                    tx,
                    id,
                    &[("title", rusqlite::types::Value::Text("after".into()))],
                )
                .map(|_| ())
            } else {
                Embed::find(tx.conn(), id)?.save_metadata(
                    tx,
                    &Metadata {
                        title: Some("after".into()),
                        ..Default::default()
                    },
                )
            }
        })
        .await;
    if let Err(error) = result {
        receive.abort();
        server.abort();
        panic!(
            "bounded {kind} callback rejected: {}",
            error.to_string().chars().take(150).collect::<String>()
        );
    }
    receive.await.unwrap();
    app.db().read(move |conn| {
        let jobs:i64 = conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='LinkEmbed::FetchJob'",[],|r|r.get(0))?;
        assert_eq!(jobs,case["fetch_jobs"].as_i64().unwrap());
        if kind != "github" { assert_eq!(Embed::find(conn,id)?.title.as_deref(),Some("after")); }
        println!("WS8bm2 bounded Rust {kind}: {} references; {} ordered Rails frames; {jobs} fetch jobs; exact SHA256",case["count"],case["frames"]);
        Ok(())
    }).await.unwrap();
    server.abort();
}
#[tokio::test]
async fn bounded_generic_callback_above_sqlite_bind_limit() {
    large_callback(0).await;
}
#[tokio::test]
async fn bounded_linkedin_callback_deduplicates_fetches_across_batches() {
    large_callback(1).await;
}
#[tokio::test]
async fn bounded_github_callback_above_sqlite_bind_limit() {
    large_callback(2).await;
}
