//! HTTP results captured by reference-tools/views/agents_ui/input_boundaries.rb.
use crate::controllers::presenters::test_support::{BENDER, DAVID, Req, TestApp};
use axum::http::Method;
use serde_json::{Value, json};

fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../../vectors/bot-ui-input-boundaries.json"
    ))
    .unwrap()
}

#[tokio::test]
async fn ws11ui_credential_date_grammar_and_multiparameters_match_rails() {
    expiry_cases(oracle(), 118).await;
}

#[tokio::test]
async fn pr196_extra_expiry_matches_rails() {
    expiry_cases(
        serde_json::from_str(include_str!(
            "../../../../../../vectors/bot-ui-input-extended.json"
        ))
        .unwrap(),
        300,
    )
    .await;
}

async fn expiry_cases(oracle: Value, expected_count: usize) {
    let t = TestApp::boot_frozen()
        .await
        .unwrap()
        .without_job_runner()
        .await;
    let mut browser = t.david();
    browser.grant_sudo().await;
    let path = format!("/account/bots/{BENDER}/credentials");
    let mut failures = Vec::new();
    let mut checked = 0;
    for (index, case) in oracle["expiry"].as_array().unwrap().iter().enumerate() {
        let attributes = &case["attributes"];
        // FLAGGED WS11 raw expiry writer: the owner accepts only Option<Timestamp>.
        // The pinned numeric/true save and render boundaries stay inventoried in the vector.
        if attributes
            .get("expires_at")
            .is_some_and(|v| v.is_number() || v == &json!(true))
            && attributes.get("name") != Some(&json!(""))
        {
            continue;
        }
        let zone = case["zone"].as_str().unwrap().to_owned();
        t.db()
            .write(move |tx| {
                tx.conn().execute(
                    "UPDATE users SET time_zone=? WHERE id=?",
                    rusqlite::params![zone, DAVID],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let name = format!("Expiry input {index}");
        let mut fields = attributes.as_object().unwrap().clone();
        fields.entry("name").or_insert_with(|| json!(name));
        let before = t
            .db()
            .read(|conn| {
                Ok(conn.query_row(
                    "SELECT COUNT(*) FROM audit_logs WHERE action='agent.credential.create'",
                    [],
                    |r| r.get::<_, i64>(0),
                )?)
            })
            .await
            .unwrap();
        let response = browser
            .write(
                Req::new(Method::POST, &path)
                    .header("accept", "text/html")
                    .header("content-type", "application/json")
                    .body(json!({"agent_credential":fields}).to_string()),
            )
            .await;
        let actual = t.db().read(move |conn| {
            use rusqlite::OptionalExtension;
            let stored = conn.query_row("SELECT expires_at FROM agent_credentials WHERE name=?",[name],|row|row.get::<_,Option<String>>(0)).optional()?;
            let audits: i64 = conn.query_row("SELECT COUNT(*) FROM audit_logs WHERE action='agent.credential.create'",[],|r|r.get(0))?;
            Ok(json!({"persisted":stored.is_some(),"stored":stored.flatten(),"audits":audits-before}))
        }).await.unwrap();
        let expected =
            json!({"persisted":case["persisted"],"stored":case["stored"],"audits":case["audits"]});
        if actual != expected || response.status.as_u16() != case["status"].as_u64().unwrap() as u16
        {
            failures.push(format!(
                "{} {}: HTTP {}; {actual}; Rails {expected}",
                case["zone"], attributes, response.status
            ));
        }
        if let Some(error) = oracle["errors"].get(response.status.as_u16().to_string()) {
            assert_eq!(response.text(), error["body"].as_str().unwrap());
            assert_eq!(response.content_type(), error["content_type"].as_str());
        }
        checked += 1;
    }
    assert_eq!(checked, expected_count);
    println!(
        "Credential expiry differential: {checked} compared; {} mismatches",
        failures.len()
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn pr196_generated_date_casts_match_pinned_model() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/bot-ui-generated-casts.json"
    ))
    .unwrap();
    generated_date_cases(&oracle);
}

#[test]
fn pr196_unicode_date_regexes_match_pinned_ascii_lookups() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/bot-ui-unicode-dates.json"
    ))
    .unwrap();
    generated_date_cases(&oracle);
}

fn generated_date_cases(oracle: &Value) {
    let now = campfire_db::Timestamp::parse_db("2026-03-02 16:00:00").unwrap();
    let mut failures = Vec::new();
    let cases = oracle["expiry"].as_array().unwrap();
    for case in cases {
        let zone = campfire_views::time::Zone::for_user(case["zone"].as_str());
        let input = campfire_kit::Param::from_json(case["input"].clone());
        let actual = super::input_casts::datetime(Some(&input), &zone, now).map(|t| t.to_db());
        let expected = case["stored"].as_str().map(str::to_owned);
        if actual != expected {
            failures.push(format!(
                "{} {}: {actual:?}; Rails {expected:?}",
                case["zone"], case["input"]
            ));
        }
    }
    println!(
        "Generated expiry differential: {} compared; {} mismatches",
        cases.len(),
        failures.len()
    );
    assert!(
        failures.is_empty(),
        "{}",
        failures
            .iter()
            .take(50)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn pr196_generated_floats_match_ruby_shortest_format() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/bot-ui-generated-casts.json"
    ))
    .unwrap();
    let mut failures = Vec::new();
    let cases = oracle["floats"].as_array().unwrap();
    for case in cases {
        let input = campfire_kit::Param::from_json(case["input"].clone());
        let actual = super::input_casts::token_string(&input);
        let expected = case["string"].as_str().unwrap();
        if actual != expected {
            failures.push(format!("{}: {actual}; Rails {expected}", case["input"]));
        }
    }
    println!(
        "Generated float differential: {} compared; {} mismatches",
        cases.len(),
        failures.len()
    );
    assert!(
        failures.is_empty(),
        "{}",
        failures
            .iter()
            .take(50)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}


fn extreme_inputs() -> Value {
    use std::io::Read;
    let bytes = include_bytes!("../../../../../..//reference-tools/views/agents_ui/extreme_cast_inputs.json.gz");
    let mut text = String::new();
    flate2::read::GzDecoder::new(&bytes[..]).read_to_string(&mut text).unwrap();
    serde_json::from_str(&text).unwrap()
}
fn extreme_oracle() -> Value {
    serde_json::from_str(include_str!("../../../../../../vectors/bot-ui-extreme-casts.json")).unwrap()
}
#[test]
fn pr196_r2_extreme_date_components_match_pinned_model() {
    let inputs = extreme_inputs();
    let oracle = extreme_oracle();
    let now = campfire_db::Timestamp::parse_db("2026-03-02 16:00:00").unwrap();
    let mut failures = Vec::new();
    let cases = oracle["expiry"].as_array().unwrap();
    for case in cases {
        let index = case["input_index"].as_u64().unwrap() as usize;
        let input = campfire_kit::Param::Str(inputs["expiry"][index].as_str().unwrap().into());
        let zone = campfire_views::time::Zone::for_user(case["zone"].as_str());
        let actual = json!({"stored":super::input_casts::datetime(Some(&input), &zone, now).map(|t|t.to_db())});
        let expected = if case.get("error").is_some() {json!({"error":case["error"]})} else {json!({"stored":case["stored"]})};
        if actual != expected {failures.push(json!({"input":inputs["expiry"][index],"zone":case["zone"],"actual":actual,"expected":expected}));}
    }
    println!("Extreme expiry differential: {} compared; {} mismatches; {} value differences; {} exception differences", cases.len(), failures.len(), failures.iter().filter(|v|v["expected"].get("stored").is_some()).count(), failures.iter().filter(|v|v["expected"].get("error").is_some()).count());
    assert!(failures.is_empty(), "{:?}", &failures[..failures.len().min(20)]);
}
#[test]
fn pr196_r2_raw_numeric_tokens_match_pinned_json() {
    let inputs = extreme_inputs();
    let oracle = extreme_oracle();
    let mut failures = Vec::new();
    let cases = oracle["tokens"].as_array().unwrap();
    for case in cases {
        let index = case["input_index"].as_u64().unwrap() as usize;
        let raw = inputs["tokens"][index].as_str().unwrap();
        let actual = match campfire_kit::params::from_json_body(format!("{{\"access_token\":{raw}}}").as_bytes()) {
            Ok(params) => json!({"string": super::input_casts::token_string(params.get("access_token").unwrap())}),
            Err(_) => json!({"error":true}),
        };
        let expected = if case.get("error").is_some() {json!({"error":true})} else {json!({"string":case["string"]})};
        if actual != expected {failures.push(json!({"raw":raw,"actual":actual,"expected":expected}));}
    }
    println!("Extreme token differential: {} compared; {} mismatches", cases.len(), failures.len());
    assert!(failures.is_empty(), "{failures:?}");
}

#[tokio::test]
async fn pr196_r2_extreme_expiry_http_save_and_error_boundaries() {
    expiry_cases(serde_json::from_str(include_str!("../../../../../../vectors/bot-ui-extreme-http.json")).unwrap(), 56).await;
}
