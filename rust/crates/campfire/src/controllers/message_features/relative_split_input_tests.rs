//! Real TimeParser outputs. Producer control: check_input_mutants.py changes
//! the relative-duration multiplier and the trailing title slice independently.
use campfire_db::{Timestamp, slash_commands::time_parser};
use serde_json::{Value, json};
fn stamp(t: Timestamp) -> String {
    let s = t.to_db();
    if s.contains('.') {
        s
    } else {
        format!("{s}.000000")
    }
}
#[test]
fn exceptional_relative_and_split_inputs_match_rails() {
    let vector: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/relative_split_inputs.json"
    ))
    .unwrap();
    let now = Timestamp::parse_db("2026-03-02 16:00:00").unwrap();
    let mut failures = vec![];
    for case in vector["cases"].as_array().unwrap() {
        let text = campfire_richtext::ruby::json_value_to_s(&case["input"]);
        let zone = case["zone"].as_str().unwrap();
        let parsed = match time_parser::parse_checked(&text, zone, now) {
            Ok(t) => json!({"value": t.map(stamp)}),
            Err(e) => {
                let s = e.to_string();
                let (class, message) = s.split_once(": ").unwrap_or(("ArgumentError", &s));
                json!({"error":class,"message":message})
            }
        };
        let leading = json!({"value":time_parser::split_leading_time(&text,zone,now).map(|(time,rest)|json!([stamp(time),rest]))});
        let (title, time) = time_parser::split_trailing_time(&text, zone, now);
        let trailing = json!({"value":[title,time.map(stamp)]});
        for (operation, actual) in [
            ("parse", parsed),
            ("leading", leading),
            ("trailing", trailing),
        ] {
            if actual != case[operation] {
                failures.push(json!({"zone":zone,"input":case["input"],"operation":operation,"rails":case[operation],"rust":actual}));
            }
        }
    }
    for failure in &failures {
        eprintln!("{failure}");
    }
    println!(
        "WS8bm2 relative/split Rust: {} matched; {} differed",
        vector["cases"].as_array().unwrap().len() * 3 - failures.len(),
        failures.len()
    );
    assert!(
        failures.is_empty(),
        "relative/split actual output differs from Rails"
    );
}

/// PR #223 review boundaries: the JSON renderer (`as_json` in the viewer's zone)
/// for the real parser's wide output, which overflowed the ago/since intermediate.
#[test]
fn relative_overflow_json_matches_fresh_rails() {
    let vector: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/relative_overflow_inputs.json"
    ))
    .unwrap();
    let cases = vector["cases"].as_array().unwrap();
    for case in cases {
        let now = Timestamp::parse_db(case["now"].as_str().unwrap()).unwrap();
        let zone_name = case["zone"].as_str().unwrap();
        let at = time_parser::parse(case["input"].as_str().unwrap(), zone_name, now).unwrap();
        assert_eq!(stamp(at), case["parse"]["value"], "{}", case["id"]);
        assert_eq!(
            super::json_time_in_zone(at, &time_parser::zone(zone_name)),
            case["json"],
            "{}",
            case["id"]
        );
    }
    println!(
        "PR223 overflow Rust: {} parsed JSON renders matched fresh Rails",
        cases.len()
    );
}
