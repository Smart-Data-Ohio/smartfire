use serde_json::Value;
fn fixture(path: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/golden/{path}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

#[test]
fn user_time_zone_uses_the_saved_rails_zone() {
    let zone = campfire_presentation::time::Zone::for_user(Some("Eastern Time (US & Canada)"));
    assert_eq!(zone.name(), "Eastern Time (US & Canada)");
    assert_eq!(zone.tz().iana_name(), Some("America/New_York"));
    assert_eq!(
        campfire_presentation::time::Zone::for_user(Some("Nowhere/Special")).name(),
        "UTC"
    );
}

#[test]
fn time_ago_matches_rails_at_rounding_and_leap_year_boundaries() {
    let helpers: Value = serde_json::from_str(&fixture("helpers.json")).unwrap();
    let zone = campfire_presentation::time::Zone::utc();
    for row in helpers["distance_of_time"].as_array().unwrap() {
        let from = row["from"].as_str().unwrap().parse().unwrap();
        let to = row["to"].as_str().unwrap().parse().unwrap();
        assert_eq!(
            campfire_presentation::time::distance_of_time_in_words(&zone, from, to, false),
            row["words"].as_str().unwrap()
        );
        assert_eq!(
            campfire_presentation::time::distance_of_time_in_words(&zone, from, to, true),
            row["words_with_seconds"].as_str().unwrap()
        );
    }
}
