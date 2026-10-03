//! Pinned Rails JSON and timestamp boundaries from normalized_casts.sh.
use super::{github_connections, input_casts};
use campfire_db::Timestamp;
use campfire_kit::{Method, Param};
use campfire_views::time::Zone;
use serde_json::{Value, json};

fn inputs() -> Value {
    use std::io::Read;
    let bytes = include_bytes!(
        "../../../../../../reference-tools/views/agents_ui/normalized_cast_inputs.json.gz"
    );
    let mut text = String::new();
    flate2::read::GzDecoder::new(&bytes[..])
        .read_to_string(&mut text)
        .unwrap();
    serde_json::from_str(&text).unwrap()
}
fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../../vectors/bot-ui-normalized-casts.json"
    ))
    .unwrap()
}
fn token_cases(nested: bool) {
    let inputs = inputs();
    let oracle = oracle();
    let mut count = 0;
    let mut failures = Vec::new();
    for case in oracle["tokens"].as_array().unwrap() {
        let raw = inputs["tokens"][case["input_index"].as_u64().unwrap() as usize]
            .as_str()
            .unwrap();
        if raw.starts_with("[[[[") != nested {
            continue;
        }
        let actual = match github_connections::json_body_params(
            &Method::POST,
            "/account/bots/1/github_connection",
            format!("{{\"access_token\":{raw}}}").as_bytes(),
        ) {
            Ok(params) => {
                json!({"string": input_casts::token_string(params.get("access_token").unwrap()).trim_matches(|c:char| c == '\0' || c.is_ascii_whitespace())})
            }
            Err(_) => json!({"error":true}),
        };
        let expected = if case.get("error").is_some() {
            json!({"error":true})
        } else {
            json!({"string":case["string"]})
        };
        if actual != expected {
            failures.push(json!({"index":case["input_index"],"actual":actual,"expected":expected}));
        }
        count += 1;
    }
    println!(
        "R3 token differential nested={nested}: {count} compared; {} mismatches",
        failures.len()
    );
    assert!(failures.is_empty(), "{failures:?}");
}
#[test]
fn pr196_r3_numeric_tokens_match_rails() {
    token_cases(false);
}
#[test]
fn pr196_r3_nested_tokens_match_rails() {
    token_cases(true);
}
#[test]
fn pr196_r3_expiry_values_and_timestamp_roundtrips_match_rails() {
    let inputs = inputs();
    let oracle = oracle();
    let now = Timestamp::parse_db("2026-03-02 16:00:00").unwrap();
    let mut failures = Vec::new();
    let mut roundtrip_failures = Vec::new();
    let mut roundtrips = 0;
    let cases = oracle["expiry"].as_array().unwrap();
    for case in cases {
        let input = Param::Str(
            inputs["expiry"][case["input_index"].as_u64().unwrap() as usize]
                .as_str()
                .unwrap()
                .into(),
        );
        let actual = match input_casts::datetime(
            Some(&input),
            &Zone::for_user(case["zone"].as_str()),
            now,
        ) {
            Ok(ts) => {
                if let Some(ts) = ts {
                    roundtrips += 1;
                    if Timestamp::parse_db(&ts.to_db()) != Some(ts) {
                        roundtrip_failures.push(ts.to_db());
                    }
                }
                json!({"stored":ts.map(|ts|ts.to_db())})
            }
            Err(error) => json!({"error":error.to_string()}),
        };
        let expected = if case.get("error").is_some() {
            json!({"error":case["error"]})
        } else {
            json!({"stored":case["stored"]})
        };
        if actual != expected {
            failures.push(case.clone());
        }
    }
    println!(
        "R3 expiry differential: {} compared; {} mismatches",
        cases.len(),
        failures.len()
    );
    println!(
        "R3 timestamp roundtrips: {roundtrips} compared; {} mismatches",
        roundtrip_failures.len()
    );
    assert!(
        failures.is_empty(),
        "{:?}",
        &failures[..failures.len().min(8)]
    );
    assert!(
        roundtrip_failures.is_empty(),
        "{:?}",
        &roundtrip_failures[..roundtrip_failures.len().min(8)]
    );
}
#[test]
fn pr196_r3_non_token_routes_keep_ordinary_parser() {
    let inputs = inputs();
    let mut checked = 0;
    for (method, path) in [
        (Method::POST, "/account/bots"),
        (Method::PATCH, "/account/bots/1"),
        (Method::POST, "/account/bots/1/credentials"),
        (Method::POST, "/rooms/1/messages"),
        (Method::POST, "/rooms"),
        (Method::PATCH, "/users/1"),
        (Method::POST, "/github/connections"),
        (Method::POST, "/login"),
    ] {
        for raw in inputs["tokens"].as_array().unwrap() {
            let body = format!(
                "{{\"access_token\":{},\"value\":{}}}",
                raw.as_str().unwrap(),
                raw.as_str().unwrap()
            );
            assert!(
                github_connections::scoped_json_body_params(&method, path, body.as_bytes())
                    .is_none()
            );
            checked += 1;
        }
    }
    println!(
        "R3 non-token parser scope: {checked} route/body combinations; 0 overridden parser results"
    );
}

#[test]
fn pr196_r3_ordinary_and_scoped_parsers_share_rails_depth_limit() {
    let inputs = inputs();
    let oracle = oracle();
    let mut failures = Vec::new();
    let mut checked = 0;
    for case in oracle["tokens"].as_array().unwrap() {
        let raw = inputs["tokens"][case["input_index"].as_u64().unwrap() as usize]
            .as_str()
            .unwrap();
        if !raw.starts_with("[[[[") {
            continue;
        }
        let body = format!("{{\"access_token\":{raw}}}");
        if campfire_kit::params::from_json_body(body.as_bytes()).is_err()
            != case.get("error").is_some()
        {
            failures.push(case["input_index"].clone());
        }
        checked += 1;
    }
    println!(
        "R3 ordinary JSON nesting: {checked} compared; {} mismatches",
        failures.len()
    );
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn pr196_r3_further_exponent_boundaries_match_rails() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/bot-ui-json-exponent-boundaries.json"
    ))
    .unwrap();
    let mut failures = Vec::new();
    for case in oracle["tokens"].as_array().unwrap() {
        let raw = case["raw"].as_str().unwrap();
        let params = github_connections::json_body_params(
            &Method::POST,
            "/account/bots/1/github_connection",
            format!("{{\"access_token\":{raw}}}").as_bytes(),
        )
        .unwrap();
        let actual = input_casts::token_string(params.get("access_token").unwrap());
        if actual != case["string"].as_str().unwrap() {
            failures.push(case.clone());
        }
    }
    println!(
        "R3 further exponent differential: {} compared; {} mismatches",
        oracle["tokens"].as_array().unwrap().len(),
        failures.len()
    );
    assert!(
        failures.is_empty(),
        "{:?}",
        &failures[..failures.len().min(8)]
    );
}
