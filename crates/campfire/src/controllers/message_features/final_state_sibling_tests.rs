use super::comparison_support::{row, same_row};
use super::quote_integration_tests::{app_rows};
use crate::integrations::link_embed::{Embed, metadata_parser::Metadata};
use serde_json::{Value, json};
#[tokio::test]
async fn final_state_sibling_claims_match_rails() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/final_state_siblings.json"
    ))
    .unwrap();
    let mut differences = Vec::new();
    for case in oracle.as_array().unwrap() {
        let app = app_rows(case["rows"].clone()).await;
        app.db()
            .write(|tx| {
                tx.conn().execute("DELETE FROM background_jobs", [])?;
                Ok(())
            })
            .await
            .unwrap();

        let id = case["primary_id"].as_i64().unwrap();
        let sibling = case["sibling_id"].as_i64().unwrap();
        let message = case["message_id"].as_i64().unwrap();
        let mode = case["mode"].as_str().unwrap().to_string();
        app.db().write(move|tx|{let save=|tx:&mut campfire_db::Tx<'_>,title:&str|Embed::find(tx.conn(),id)?.save_metadata(tx,&Metadata{title:Some(title.into()),..Default::default()});
   if mode=="savepoint_then_removed"{let r:campfire_db::Result<()>=tx.savepoint(|tx|{save(tx,"Rolled-back intermediate")?;Err(campfire_db::Error::Other("review rollback".into()))});assert!(r.is_err());}
   save(tx,"After parent")?;
   if mode=="suppressed"{tx.conn().execute("UPDATE messages SET markdown_source='Newer message state',embeds_suppressed=1 WHERE id=?",[message])?;}else{tx.conn().execute("DELETE FROM link_embed_references WHERE message_id=?",[message])?;tx.conn().execute("UPDATE messages SET markdown_source='Newer message state' WHERE id=?",[message])?;}Ok(())
  }).await.unwrap();


        let data = case.clone();
        let(actual,pending)=app.db().read(move|c|{same_row(&row(c,"messages",message)?,&data["message"],"superseding message");let pending=c.prepare("SELECT arguments FROM background_jobs WHERE job_class='LinkEmbed::FetchJob' ORDER BY id")?.query_map([],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?.into_iter().map(|s|serde_json::from_str::<Value>(&s).unwrap()["embed_id"].clone()).collect::<Vec<_>>();Ok((row(c,"link_embeds",sibling)?,json!(pending)))}).await.unwrap();
        println!(
            "WS8bm2 final-state siblings {}/{}: Rust jobs={pending}, Rails jobs={}; Rust claim={}, Rails claim={}; complete frames and newer message row match",
            case["kind"],
            case["mode"],
            case["pending"],
            actual["fetch_requested_at"],
            case["sibling"]["fetch_requested_at"]
        );
        if pending != case["pending"] || actual != case["sibling"] {
            differences.push(format!(
                "{}/{}: {pending} != {}",
                case["kind"], case["mode"], case["pending"]
            ));
        }

    }
    println!(
        "WS8bm2 final-state siblings Rust: 6/6 transactions; complete persisted rows, durable sibling jobs and exact frames checked"
    );
    assert!(
        differences.is_empty(),
        "stale sibling parity differences: {differences:?}"
    );
}
#[tokio::test]
async fn final_state_sibling_queue_failure_rolls_back_metadata_claim_and_jobs() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/final_state_siblings.json"
    ))
    .unwrap();
    for case in [oracle[0].clone(), oracle[3].clone()] {
        let app = app_rows(case["rows"].clone()).await;

        let parent = case["primary_id"].as_i64().unwrap();
        let sibling = case["sibling_id"].as_i64().unwrap();
        let before = app
            .db()
            .read(move |c| {
                Ok((
                    row(c, "link_embeds", parent)?,
                    row(c, "link_embeds", sibling)?,
                ))
            })
            .await
            .unwrap();
        app.db().write(|tx| {
            tx.conn().execute_batch("DELETE FROM background_jobs; CREATE TRIGGER ws8_sibling_queue_failure BEFORE INSERT ON background_jobs WHEN NEW.job_class='LinkEmbed::FetchJob' BEGIN SELECT RAISE(ABORT,'fixture sibling queue failure'); END")?;
            Ok(())
        }).await.unwrap();
        let result = app
            .db()
            .write(move |tx| {
                Embed::find(tx.conn(), parent)?.save_metadata(
                    tx,
                    &Metadata {
                        title: Some("After failed queue".into()),
                        ..Default::default()
                    },
                )
            })
            .await;
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("fixture sibling queue failure")
        );
        app.db()
            .read(move |c| {
                same_row(
                    &row(c, "link_embeds", parent)?,
                    &before.0,
                    "parent rollback",
                );
                same_row(
                    &row(c, "link_embeds", sibling)?,
                    &before.1,
                    "claim rollback",
                );
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
    println!(
        "WS8bm2 final-state sibling queue failures: 2/2 transactions rolled back metadata, claims, durable jobs and frames"
    );
}
