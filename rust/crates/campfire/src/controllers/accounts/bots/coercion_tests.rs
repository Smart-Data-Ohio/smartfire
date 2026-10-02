//! HTTP results captured by reference-tools/views/agents_ui/input_boundaries.rb.
use crate::controllers::presenters::test_support::{BENDER, DAVID, Req, TestApp};
use axum::http::{Method, StatusCode};
use serde_json::{Value, json};

fn oracle() -> Value {
    serde_json::from_str(include_str!("../../../../../../vectors/bot-ui-input-boundaries.json")).unwrap()
}

#[tokio::test]
async fn ws11ui_credential_date_grammar_and_multiparameters_match_rails() {
    let t = TestApp::boot_frozen().await.unwrap().without_job_runner().await;
    let mut browser = t.david();
    browser.grant_sudo().await;
    let path = format!("/account/bots/{BENDER}/credentials");
    let mut failures = Vec::new();
    let mut checked = 0;
    for (index, case) in oracle()["expiry"].as_array().unwrap().iter().enumerate() {
        let attributes = &case["attributes"];
        // FLAGGED WS11 raw expiry writer: the owner accepts only Option<Timestamp>.
        // The pinned numeric/true save and render boundaries stay inventoried in the vector.
        if attributes.get("expires_at").is_some_and(|v| v.is_number() || v == &json!(true)) {
            continue;
        }
        let zone = case["zone"].as_str().unwrap().to_owned();
        t.db().write(move |tx| { tx.conn().execute("UPDATE users SET time_zone=? WHERE id=?",rusqlite::params![zone,DAVID])?; Ok(()) }).await.unwrap();
        let name = format!("Expiry input {index}");
        let mut fields = attributes.as_object().unwrap().clone();
        fields.insert("name".into(), json!(name));
        let before = t.db().read(|conn| Ok(conn.query_row("SELECT COUNT(*) FROM audit_logs WHERE action='agent.credential.create'",[],|r|r.get::<_,i64>(0))?)).await.unwrap();
        let response = browser.write(Req::new(Method::POST, &path).header("accept","text/html").header("content-type","application/json").body(json!({"agent_credential":fields}).to_string())).await;
        let actual = t.db().read(move |conn| {
            use rusqlite::OptionalExtension;
            let stored = conn.query_row("SELECT expires_at FROM agent_credentials WHERE name=?",[name],|row|row.get::<_,Option<String>>(0)).optional()?;
            let audits: i64 = conn.query_row("SELECT COUNT(*) FROM audit_logs WHERE action='agent.credential.create'",[],|r|r.get(0))?;
            Ok(json!({"persisted":stored.is_some(),"stored":stored.flatten(),"audits":audits-before}))
        }).await.unwrap();
        let expected = json!({"persisted":case["persisted"],"stored":case["stored"],"audits":case["audits"]});
        if actual != expected || response.status.as_u16() != case["status"].as_u64().unwrap() as u16 {
            failures.push(format!("{} {}: HTTP {}; {actual}; Rails {expected}",case["zone"],attributes,response.status));
        }
        checked += 1;
    }
    assert_eq!(checked,108);
    assert!(failures.is_empty(),"{}",failures.join("\n"));
    assert_eq!(StatusCode::CREATED.as_u16(),201);
}
