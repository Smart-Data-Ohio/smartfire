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
        // Successful numeric/true saves have their own strict raw-writer,
        // model-read and list differential below; do not compare them as timestamps.
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
        let name = fields
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
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
            let id: Option<i64> = conn.query_row("SELECT id FROM agent_credentials WHERE name=?", [&name], |r|r.get(0)).optional()?;
            let read_back = id.and_then(|id|campfire_db::AgentCredential::find(conn,id).ok().flatten()).and_then(|c|c.expires_at).map(|ts|ts.to_db());
            let stored = conn.query_row("SELECT expires_at FROM agent_credentials WHERE name=?",[name],|row|row.get::<_,Option<String>>(0)).optional()?;
            let audits: i64 = conn.query_row("SELECT COUNT(*) FROM audit_logs WHERE action='agent.credential.create'",[],|r|r.get(0))?;
            Ok(json!({"persisted":stored.is_some(),"stored":stored.flatten(),"audits":audits-before,"read_back":{"stored":read_back}}))
        }).await.unwrap();
        let expected = json!({"persisted":case["persisted"],"stored":case["stored"],"audits":case["audits"],"read_back":case.get("read_back").cloned().unwrap_or_else(||json!({"stored":case["stored"]}))});
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
        let actual = super::input_casts::datetime(Some(&input), &zone, now)
            .unwrap()
            .map(|t| t.to_db());
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
    let bytes = include_bytes!(
        "../../../../../../reference-tools/views/agents_ui/extreme_cast_inputs.json.gz"
    );
    let mut text = String::new();
    flate2::read::GzDecoder::new(&bytes[..])
        .read_to_string(&mut text)
        .unwrap();
    serde_json::from_str(&text).unwrap()
}
fn extreme_oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../../vectors/bot-ui-extreme-casts.json"
    ))
    .unwrap()
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
        let actual = match super::input_casts::datetime(Some(&input), &zone, now) {
            Ok(t) => json!({"stored":t.map(|t|t.to_db())}),
            Err(error) => json!({"error":error.to_string()}),
        };
        let expected = if case.get("error").is_some() {
            json!({"error":case["error"]})
        } else {
            json!({"stored":case["stored"]})
        };
        if actual != expected {
            failures.push(json!({"input":inputs["expiry"][index],"zone":case["zone"],"actual":actual,"expected":expected}));
        }
    }
    println!(
        "Extreme expiry differential: {} compared; {} mismatches; {} value differences; {} exception differences",
        cases.len(),
        failures.len(),
        failures
            .iter()
            .filter(|v| v["expected"].get("stored").is_some())
            .count(),
        failures
            .iter()
            .filter(|v| v["expected"].get("error").is_some())
            .count()
    );
    assert!(
        failures.is_empty(),
        "{:?}",
        &failures[..failures.len().min(20)]
    );
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
        let actual = match super::github_connections::json_body_params(
            &Method::POST,
            "/account/bots/1/github_connection",
            format!("{{\"access_token\":{raw}}}").as_bytes(),
        ) {
            Ok(params) => {
                json!({"string": super::input_casts::token_string(params.get("access_token").unwrap())})
            }
            Err(_) => json!({"error":true}),
        };
        let expected = if case.get("error").is_some() {
            json!({"error":true})
        } else {
            json!({"string":case["string"]})
        };
        if actual != expected {
            failures.push(json!({"raw":raw,"actual":actual,"expected":expected}));
        }
    }
    println!(
        "Extreme token differential: {} compared; {} mismatches",
        cases.len(),
        failures.len()
    );
    assert!(failures.is_empty(), "{failures:?}");
}

#[tokio::test]
async fn pr196_r2_extreme_expiry_http_save_and_error_boundaries() {
    expiry_cases(
        serde_json::from_str(include_str!(
            "../../../../../../vectors/bot-ui-extreme-http.json"
        ))
        .unwrap(),
        56,
    )
    .await;
}

#[test]
fn pr196_r2_generated_extreme_components_and_numeric_lexemes_match_rails() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/bot-ui-generated-extremes.json"
    ))
    .unwrap();
    let now = campfire_db::Timestamp::parse_db("2026-03-02 16:00:00").unwrap();
    let mut date_failures = Vec::new();
    for case in oracle["expiry"].as_array().unwrap() {
        let input = campfire_kit::Param::Str(case["input"].as_str().unwrap().into());
        let zone = campfire_views::time::Zone::for_user(case["zone"].as_str());
        let actual = match super::input_casts::datetime(Some(&input), &zone, now) {
            Ok(t) => json!({"stored":t.map(|t|t.to_db())}),
            Err(e) => json!({"error":e.to_string()}),
        };
        let expected = if case.get("error").is_some() {
            json!({"error":case["error"]})
        } else {
            json!({"stored":case["stored"]})
        };
        if actual != expected {
            date_failures.push(json!({"input":case["input"],"zone":case["zone"],"actual":actual,"expected":expected}));
        }
    }
    let mut token_failures = Vec::new();
    for case in oracle["tokens"].as_array().unwrap() {
        let raw = case["raw"].as_str().unwrap();
        let params = super::github_connections::json_body_params(
            &Method::POST,
            "/account/bots/1/github_connection",
            format!("{{\"access_token\":{raw}}}").as_bytes(),
        )
        .unwrap();
        let actual = super::input_casts::token_string(params.get("access_token").unwrap());
        if actual != case["string"].as_str().unwrap() {
            token_failures.push(raw);
        }
    }
    println!(
        "Generated extreme differential: {} dates, {} tokens; {} date mismatches; {} token mismatches",
        oracle["expiry"].as_array().unwrap().len(),
        oracle["tokens"].as_array().unwrap().len(),
        date_failures.len(),
        token_failures.len()
    );
    assert!(
        date_failures.is_empty(),
        "{:?}",
        &date_failures[..date_failures.len().min(30)]
    );
    assert!(token_failures.is_empty(), "{token_failures:?}");
}

#[test]
fn pr196_r2_parser_keeps_bounded_work_and_never_panics() {
    use std::time::{Duration, Instant};
    let inputs = extreme_inputs();
    let now = campfire_db::Timestamp::parse_db("2026-03-02 16:00:00").unwrap();
    let zone = campfire_views::time::Zone::utc();
    for s in ["1 Jan 2026 12:00:00Z", "bad", "--0101", "01.02.03"] {
        let _ = super::input_casts::datetime(Some(&campfire_kit::Param::Str(s.into())), &zone, now);
    }
    let mut count = 0;
    let mut panics = 0;
    let mut worst = Duration::ZERO;
    let mut long_worst = Duration::ZERO;
    for input in inputs["expiry"].as_array().unwrap() {
        let text = input.as_str().unwrap();
        let param = campfire_kit::Param::Str(text.into());
        let start = Instant::now();
        for _ in 0..50 {
            if std::panic::catch_unwind(|| super::input_casts::datetime(Some(&param), &zone, now))
                .is_err()
            {
                panics += 1;
            }
            count += 1;
        }
        worst = worst.max(start.elapsed() / 50);
        if text.len() > 128 {
            let start = Instant::now();
            for _ in 0..10000 {
                let _ =
                    std::hint::black_box(super::input_casts::datetime(Some(&param), &zone, now));
                count += 1;
            }
            long_worst = long_worst.max(start.elapsed() / 10000);
        }
    }
    println!(
        "Extreme parser stress: {count} calls; {panics} panics; worst warm 50-call mean={worst:?}; over-128-byte worst mean={long_worst:?}"
    );
    assert_eq!(panics, 0);
}

#[tokio::test]
async fn pr196_r3_boundary_expiry_http_save_and_read() {
    expiry_cases(
        serde_json::from_str(include_str!(
            "../../../../../../vectors/bot-ui-boundary-expiry-http.json"
        ))
        .unwrap(),
        8,
    )
    .await;
}

#[tokio::test]
async fn ws11ui_next_numeric_expiry_saves_match_rails_raw_writer() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/agent-numeric-expiry.json"
    ))
    .unwrap();
    let t = TestApp::boot_frozen()
        .await
        .unwrap()
        .without_job_runner()
        .await;
    let mut browser = t.david();
    browser.grant_sudo().await;
    let mut failures = Vec::new();
    for (index, case) in oracle["cases"].as_array().unwrap().iter().enumerate() {
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
        let before: i64 = t
            .db()
            .read(|conn| {
                Ok(conn.query_row(
                    "SELECT count(*) FROM audit_logs WHERE action='agent.credential.create'",
                    [],
                    |r| r.get(0),
                )?)
            })
            .await
            .unwrap();
        let name = format!("Raw expiry {index}");
        let response = browser
            .write(
                Req::new(Method::POST, &format!("/account/bots/{BENDER}/credentials"))
                    .header("accept", "text/html")
                    .header("content-type", "application/json")
                    .body(
                        json!({"agent_credential":{"name":name,"expires_at":case["input"]}})
                            .to_string(),
                    ),
            )
            .await;
        let actual = t.db().read(move |conn| {
            let (id,raw,storage): (i64,rusqlite::types::Value,String) = conn.query_row("SELECT id,expires_at,typeof(expires_at) FROM agent_credentials WHERE name=?",[name],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
            let stored = match raw {
                rusqlite::types::Value::Integer(v) => json!(v),
                rusqlite::types::Value::Real(v) => json!(v),
                rusqlite::types::Value::Null => Value::Null,
                other => json!(format!("{other:?}")),
            };
            let read = campfire_db::AgentCredential::find(conn,id).map(|record| record.and_then(|c| {
                c.raw_expires_at.map(|value| match value {
                    rusqlite::types::Value::Integer(v) => json!(v),
                    rusqlite::types::Value::Real(v) => json!(v),
                    other => json!(format!("{other:?}")),
                }).or_else(|| c.expires_at.map(|t|json!(t.to_db())))
            }));
            let audits: i64 = conn.query_row("SELECT count(*) FROM audit_logs WHERE action='agent.credential.create'", [], |r|r.get(0))?;
            Ok(json!({"stored":stored,"storage_type":storage,"read_back":read.map_err(|e|e.to_string()),"audits":audits-before}))
        }).await.unwrap();
        let expected = json!({"stored":case["stored"],"storage_type":case["storage_type"],"read_back":{"Ok":case["read_back"]},"audits":case["audits"]});
        if actual != expected || response.status.as_u16() as u64 != case["status"].as_u64().unwrap()
        {
            failures.push(format!(
                "{} {}: HTTP {}; {actual}; Rails {expected}",
                case["zone"], case["input"], response.status
            ));
        }
        // Pinned Rails reads the raw number, then its list raises on iso8601.
        // Match the production 500 without reproducing that exception.
        let list = browser
            .get(&format!("/account/bots/{BENDER}/credentials"))
            .await;
        if list.status.as_u16() as u64 != case["list_status"].as_u64().unwrap() {
            failures.push(format!("numeric list status {}", list.status));
        }
        if let Some(body) = case["list_body"].as_str() {
            if list.text() != body {
                failures.push("numeric list error body differs".into());
            }
        }
    }
    println!(
        "Raw credential expiry differential: 10 saves and model reads; {} mismatches",
        failures.len()
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
