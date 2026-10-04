//! Representable Rails extrema and an explicit, reproducible shared-owner dependency.
//! Producer control: check_list_scaling_mutants.py changes actual relative hours.
//! probe_extreme_range.py --strict exposes the unported I512 cases without masking.
use campfire_db::{Timestamp, slash_commands::time_parser};
use serde_json::{Value, json};

fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/extreme_range.json"
    ))
    .unwrap()
}
fn stamp(at: Timestamp) -> String {
    let value = at.to_db();
    if value.contains('.') {
        value
    } else {
        format!("{value}.000000")
    }
}
fn observation(case: &Value, operation: &str) -> Value {
    let input = case["input"].as_str().unwrap();
    let zone = case["zone"].as_str().unwrap();
    let now = Timestamp::parse_db(SEED_NOW).unwrap();
    match operation {
        "parse" => {
            let result = if case["kind"] == "calendar" {
                time_parser::parse_calendar(input, &time_parser::zone(zone), now)
            } else {
                time_parser::parse_checked(input, zone, now)
            };
            match result {
                Ok(value) => json!({"value":value.map(stamp)}),
                Err(error) => {
                    let message = error.to_string();
                    let (class, text) = message
                        .split_once(": ")
                        .expect("exception class from real parser");
                    json!({"error":class,"message":text})
                }
            }
        }
        "leading" => {
            json!({"value":time_parser::split_leading_time(input,zone,now).map(|(at,rest)|json!([stamp(at),rest]))})
        }
        "trailing" => {
            let (title, at) = time_parser::split_trailing_time(input, zone, now);
            json!({"value":[title,at.map(stamp)]})
        }
        _ => panic!("unknown operation"),
    }
}
use crate::controllers::presenters::test_support::SEED_NOW;

#[test]
fn represented_extreme_relative_and_calendar_values_match_fresh_rails() {
    let mut comparisons = 0;
    for case in oracle()["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| case["fits_shared_timestamp"] == true)
    {
        for operation in ["parse", "leading", "trailing"] {
            if case[operation].is_null() {
                continue;
            }
            assert_eq!(
                observation(case, operation),
                case[operation],
                "actual represented extreme differs from Rails: {} {operation}",
                case["id"]
            );
            comparisons += 1;
        }
    }
    assert_eq!(comparisons, 34);
    println!(
        "WS8bm2 represented extreme-range: 34 fresh Rails parser/split/exception outcomes matched; shared parser reused; no SQL"
    );
}

/// This is a diagnostic, not a parity pass or an approved difference. The strict
/// probe compares these real observations to Rails and fails until the owner
/// enlarges the shared Timestamp representation (db/src/time.rs).
#[test]
fn report_unrepresented_extreme_shared_timestamp_dependency() {
    let mut blocked = 0;
    for case in oracle()["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| case["fits_shared_timestamp"] == false)
    {
        assert!(
            Timestamp::parse_db(case["parse"]["value"].as_str().unwrap()).is_none(),
            "shared Timestamp now supports this value; reconcile the dependency flag"
        );
        let actual = json!({"id":case["id"],"parse":observation(case,"parse"),"leading":observation(case,"leading"),"trailing":observation(case,"trailing")});
        println!("WS8BM2_EXTREME_ACTUAL {actual}");
        blocked += 1;
    }
    assert_eq!(blocked, 12);
    println!(
        "WS8bm2 extreme-range audit: 12 real out-of-range observations; parity unported; owner WS11-UI/shared WS8 Timestamp; not approved differences"
    );
}
