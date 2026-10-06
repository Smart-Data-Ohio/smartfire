//! Fresh pinned Rails values for all 24 PR #223 panic outcomes and the hour renderer.
//! Regenerate with reference-tools/messaging/relative_overflow_inputs.rb.
use super::time_parser;
use crate::Timestamp;
use bnum::types::I512;
use serde_json::{Value, json};

fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../vectors/messaging/relative_overflow_inputs.json"
    ))
    .unwrap()
}
fn stamp(t: Timestamp) -> String {
    let s = t.to_db();
    if s.contains('.') {
        s
    } else {
        format!("{s}.000000")
    }
}
#[test]
fn relative_overflow_boundaries_match_fresh_rails() {
    let mut failures = vec![];
    let mut count = 0;
    for case in oracle()["cases"].as_array().unwrap() {
        let text = case["input"].as_str().unwrap();
        let zone = case["zone"].as_str().unwrap();
        let now = Timestamp::parse_db(case["now"].as_str().unwrap()).unwrap();
        for operation in ["parse", "leading", "trailing"] {
            let actual = std::panic::catch_unwind(|| match operation {
                "parse" => json!({"value":time_parser::parse_checked(text,zone,now).unwrap().map(stamp)}),
                "leading" => json!({"value":time_parser::split_leading_time(text,zone,now).map(|(time,rest)|json!([stamp(time),rest]))}),
                "trailing" => {
                    let (title, time) = time_parser::split_trailing_time(text, zone, now);
                    json!({"value":[title,time.map(stamp)]})
                }
                _ => unreachable!(),
            }).unwrap_or_else(|_| json!({"panic":true}));
            count += 1;
            if actual != case[operation] {
                failures.push(json!({"case":case["id"],"operation":operation,"actual":actual,"rails":case[operation]}));
            }
        }
    }
    for failure in &failures {
        eprintln!("{failure}");
    }
    println!(
        "PR223 overflow Rust: {} matched; {} differed",
        count - failures.len(),
        failures.len()
    );
    assert!(
        failures.is_empty(),
        "relative overflow actual parser/split differs from fresh Rails"
    );
}
/// Renderer and offset arithmetic, reached from Rails' own stored values so the
/// comparison holds even where the parser fails first.
#[test]
fn relative_overflow_render_and_offsets_match_fresh_rails() {
    for case in oracle()["cases"].as_array().unwrap() {
        // Independently decode Rails' persisted UTC encoding to reach the renderer
        // even when the unfixed parser panics first on civil-day arithmetic.
        let at = Timestamp::parse_db(case["parse"]["value"].as_str().unwrap()).unwrap();
        let zone = time_parser::zone(case["zone"].as_str().unwrap());
        assert_eq!(
            rails_compat::datetime::render(at, &zone, true),
            case["render"],
            "{}",
            case["id"]
        );
        let nanos = jiff::SignedDuration::from_nanos(1001);
        assert_eq!(stamp(at.since(nanos)), case["since"], "{}", case["id"]);
        assert_eq!(stamp(at.ago(nanos)), case["ago"], "{}", case["id"]);
        assert_eq!(stamp(at.since(-nanos)), case["ago"], "{}", case["id"]);
        assert_eq!(stamp(at.ago(-nanos)), case["since"], "{}", case["id"]);
    }
    println!(
        "PR223 overflow Rust: {} render/since/ago boundaries matched fresh Rails",
        oracle()["cases"].as_array().unwrap().len()
    );
}

#[test]
fn relative_overflow_hours_render_matches_fresh_rails() {
    let vector = oracle();
    let case = vector["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == "UTC/142_hours")
        .unwrap();
    let now = Timestamp::parse_db(case["now"].as_str().unwrap()).unwrap();
    let at = time_parser::parse_checked(case["input"].as_str().unwrap(), "UTC", now)
        .unwrap()
        .unwrap();
    assert_eq!(stamp(at), case["parse"]["value"]);
    assert_eq!(
        rails_compat::datetime::render(at, &jiff::tz::TimeZone::UTC, true),
        case["render"]
    );
    println!("PR223 overflow Rust: 10^142 hours parse/render matched fresh Rails");
}

/// Amounts whose instant lands at the edge of the wide representation: the
/// largest still-representable instant renders in every offset direction, and one
/// past it is a range rejection rather than an overflow (panic in test builds,
/// a wrapped year in release).
#[test]
fn relative_amounts_at_the_wide_limit_reject_instead_of_overflowing() {
    let now = Timestamp::parse_db("2026-03-02 16:00:00").unwrap();
    let hour = I512::from(3_600_000_000_i64);
    let edge = (I512::MAX - now.as_wide_microsecond()) / hour;
    // The largest whole-microsecond SignedDuration, applied both ways.
    let far = jiff::SignedDuration::from_secs(i64::MAX);
    for zone in [
        "UTC",
        "Asia/Tokyo",
        "Pacific/Kiritimati",
        "America/New_York",
    ] {
        let tz = time_parser::zone(zone);
        let mut accepted = 0;
        for hours in [
            edge,
            edge - I512::from(1_u64 << 30),
            edge - (I512::ONE << 70u32),
        ] {
            let text = format!("in {hours} hours");
            let at = std::panic::catch_unwind(|| time_parser::parse(&text, zone, now))
                .unwrap_or_else(|_| panic!("{zone} {text} panicked"));
            let Some(at) = at else { continue };
            accepted += 1;
            let rendered = std::panic::catch_unwind(|| {
                (
                    rails_compat::datetime::render(at, &tz, true),
                    at.since(far).ago(far),
                )
            })
            .unwrap_or_else(|_| panic!("{zone} {text} render panicked"));
            assert!(!rendered.0.starts_with('-'), "{zone} {text} wrapped");
            assert_eq!(rendered.1, at);
        }
        assert!(time_parser::parse(&format!("in {} hours", edge + I512::ONE), zone, now).is_none());
        assert_eq!(
            accepted, 1,
            "{zone}: only the amount clear of the margin parses"
        );
    }
}
