//! Native request integration, without borrowed message/composer HTML.
use crate::controllers::presenters::test_support::*;
use axum::http::{StatusCode};


async fn zone_fixture_app() -> (TestApp, serde_json::Value) {
    let app = TestApp::boot_frozen().await.expect("default seed required");
    let oracle: serde_json::Value = serde_json::from_str(include_str!("event_zones.json")).unwrap();
    let rows = oracle["rows"].clone();
    let github = oracle["github"].clone();
    // Persist the Rails fixture records, not rendered provider HTML. Every HTTP path
    // resolves cards through the real Rust presenter and owner partials.
    app.db().write(move |tx| {
        for table in ["events", "messages", "action_text_rich_texts", "event_references"] {
            for row in rows[table].as_array().unwrap() {
                let row = row.as_object().unwrap();
                let columns = row.keys().map(|name| format!("\"{name}\"")).collect::<Vec<_>>().join(",");
                let parameters = vec!["?"; row.len()].join(",");
                let values = row.values().map(|value| match value {
                    serde_json::Value::Null => rusqlite::types::Value::Null,
                    serde_json::Value::Bool(value) => rusqlite::types::Value::Integer(i64::from(*value)),
                    serde_json::Value::Number(value) => rusqlite::types::Value::Integer(value.as_i64().unwrap()),
                    serde_json::Value::String(value) => rusqlite::types::Value::Text(value.clone()),
                    _ => panic!("unexpected SQL fixture value {value}"),
                });
                tx.conn().execute(&format!("INSERT INTO {table} ({columns}) VALUES ({parameters})"), rusqlite::params_from_iter(values))?;
            }
        }
        tx.conn().execute("UPDATE github_pull_requests SET github_updated_at=? WHERE id=?",
            (github["updated_at"].as_str().unwrap(), github["id"].as_i64().unwrap()))?;
        Ok(())
    }).await.unwrap();
    (app, oracle)
}

fn write_zone_oracle() -> serde_json::Value {
    serde_json::from_str(include_str!("card_write_zones.json")).unwrap()
}

fn zone_cases<'a>(oracle: &'a serde_json::Value, kind: &str, defaults: bool) -> Vec<&'a serde_json::Value> {
    oracle["cases"].as_array().unwrap().iter().filter(|case| {
        case["kind"] == kind && matches!(case["zone"].as_str(), None | Some("UTC" | "" | "Not a real zone")) == defaults
    }).collect()
}

async fn set_audit_zone(app: &TestApp, case: &serde_json::Value) {
    let zone = case["zone"].as_str().map(str::to_owned);
    app.db().write(move |tx| {
        tx.conn().execute("UPDATE users SET time_zone=? WHERE id=?", (zone, DAVID))?;
        Ok(())
    }).await.unwrap();
}

async fn audit_message_creation(defaults: bool) {
    let (app, _) = zone_fixture_app().await;
    let oracle = write_zone_oracle();
    let mut browser = app.david();
    for case in zone_cases(&oracle, "message_create", defaults) {
        set_audit_zone(&app, case).await;
        let sequence = case["message_id"].as_i64().unwrap() - 1;
        app.db().write(move |tx| {
            tx.conn().execute("UPDATE sqlite_sequence SET seq=? WHERE name='messages'", [sequence])?;
            Ok(())
        }).await.unwrap();
        let response = browser.write(Req::new(axum::http::Method::POST, case["path"].as_str().unwrap())
            .header("accept", "text/vnd.turbo-stream.html").form(&[
                ("message[client_message_id]", case["client_id"].as_str().unwrap()),
                ("message[markdown_source]", case["source"].as_str().unwrap()),
            ])).await;
        assert_eq!(response.status, StatusCode::CREATED);

        let id=case["message_id"].as_i64().unwrap();
        let stored=app.db().read(move |conn| campfire_db::Message::find(conn,id)).await.unwrap();
        assert_eq!(stored.creator_id,DAVID);
        assert_eq!(Some(stored.client_message_id.as_str()),case["client_id"].as_str());
        // Retries render in the current request zone without repeating the creation.
        let retry = browser.write(Req::new(axum::http::Method::POST, case["path"].as_str().unwrap())
            .header("accept", "text/vnd.turbo-stream.html").form(&[("message[client_message_id]", case["client_id"].as_str().unwrap())])).await;
        assert_eq!(retry.status, StatusCode::CREATED);
        assert_eq!(app.db().read(move |conn| Ok(campfire_db::Message::find(conn,id)?.client_message_id)).await.unwrap(),stored.client_message_id);
    }
}

#[tokio::test]
async fn zone_audit_message_creation_matches_rails_and_reload() {
    audit_message_creation(false).await;
}
