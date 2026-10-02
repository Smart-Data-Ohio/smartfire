//! Real saved-item/reminder/message producers replayed through inbox controllers.
use super::*;
use crate::controllers::presenters::test_support::{DAVID, SEED_NOW};
use std::sync::Arc;

#[tokio::test]
async fn ws11ui_next_inbox_lifecycle_matches_rails_producers_and_response_bytes() {
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/agent-inbox-lifecycle.json"
    ))
    .unwrap();
    let clock = Arc::new(campfire_kit::FrozenClock::new(SEED_NOW.parse().unwrap()));
    let t = TestApp::boot_with_test_clock(clock.clone())
        .await
        .unwrap()
        .without_job_runner()
        .await;
    let defect = std::env::var("WS11UI_INBOX_LIFECYCLE_DEFECT").unwrap_or_default();
    t.db().write(move |tx| {
        tx.conn().execute("DELETE FROM activity_items",[])?;
        tx.conn().execute("DELETE FROM saved_items",[])?;
        let broken=match defect.as_str() {
            "reminder" => "CREATE TRIGGER ws11ui_broken_reminder BEFORE INSERT ON activity_items WHEN NEW.event_type='message_reminder' BEGIN SELECT RAISE(IGNORE); END",
            "recurrence" => "CREATE TRIGGER ws11ui_broken_recurrence AFTER UPDATE ON activity_items WHEN OLD.handled_at IS NOT NULL AND NEW.handled_at IS NULL BEGIN UPDATE activity_items SET read_at=OLD.read_at,handled_at=OLD.handled_at WHERE id=NEW.id; END",
            "deleted-source" => "CREATE TRIGGER ws11ui_broken_source_delete BEFORE DELETE ON messages BEGIN SELECT RAISE(IGNORE); END",
            "" => "",
            _ => panic!("unknown lifecycle discrimination defect"),
        };
        if !broken.is_empty() {tx.conn().execute_batch(broken)?;}
        Ok(())
    }).await.unwrap();
    let mut browser = t.david();
    let mut differences = Vec::new();
    let mut checked = 0;
    for step in data["steps"].as_array().unwrap() {
        if step["operation"] == "dispatch" {
            clock.advance(jiff::SignedDuration::from_secs(
                step["advance"].as_i64().unwrap(),
            ));
            let saved_id = step["saved_id"].as_i64().unwrap();
            let expected_id = step["item_id"].as_i64().unwrap();
            let outcome = t
                .db()
                .write(move |tx| {
                    let now = tx.now();
                    let ids = campfire_db::SavedItem::dispatch_due(tx, now)?;
                    let item = campfire_db::ActivityItem::find_by_user_and_source(
                        tx.conn(),
                        DAVID,
                        "SavedItem",
                        saved_id,
                    )?
                    .unwrap();
                    Ok((ids, item.id))
                })
                .await
                .expect("real reminder dispatch must persist an inbox item");
            assert_eq!(
                outcome,
                (vec![saved_id], expected_id),
                "same-row reminder recurrence"
            );
            continue;
        }
        let method =
            Method::from_bytes(step["method"].as_str().unwrap().to_uppercase().as_bytes()).unwrap();
        let req = Req::new(method, step["path"].as_str().unwrap())
            .header("accept", step["accept"].as_str().unwrap())
            .header("content-type", "application/json")
            .body(serde_json::to_vec(&step["params"]).unwrap());
        let response = browser.write(req).await;
        assert_eq!(
            response.status.as_u16() as u64,
            step["status"].as_u64().unwrap(),
            "{}: {}",
            step["name"],
            response.text()
        );
        // Only the global WS9 404 template is outside this controller's parity scope.
        if let Some(body) = step["body"].as_str() {
            for key in ["content-type", "cache-control", "pragma", "location"] {
                assert_eq!(
                    response.header(key),
                    step["headers"][key].as_str(),
                    "{} {key}",
                    step["name"]
                );
            }
            if response.text() != body {
                differences.push(format!(
                    "{}\nRust: {}\nRails: {body}",
                    step["name"],
                    response.text()
                ));
            }
        }
        checked += 1;
    }
    let source = data["message_id"].as_i64().unwrap();
    let exists = t
        .db()
        .read(move |conn| {
            Ok(conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM messages WHERE id=?)",
                [source],
                |r| r.get::<_, bool>(0),
            )?)
        })
        .await
        .unwrap();
    assert!(
        !exists,
        "deleted source must actually be removed by the real writer"
    );
    println!(
        "Inbox lifecycle Rails differential: {checked} HTTP responses; 2 real reminder dispatches; {} byte mismatches",
        differences.len()
    );
    assert!(differences.is_empty(), "{}", differences.join("\n"));
}
