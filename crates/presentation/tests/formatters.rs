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

#[test]
fn custom_fragment_keys_match_pr148_rails() {
    use campfire_presentation::cache_keys::{self as keys, MessageKey};
    let vectors: Value = serde_json::from_str(&fixture("cache_keys.json")).unwrap();
    let timestamp = |value: &Value| {
        value
            .as_str()
            .map(|value| value.parse::<jiff::Timestamp>().unwrap())
    };
    let stamps = |value: &Value| {
        value
            .as_array()
            .map(|values| values.iter().filter_map(timestamp).collect::<Vec<_>>())
            .unwrap_or_default()
    };
    assert_eq!(
        keys::PRESENTATION_CACHE_VERSION,
        vectors["presentation_cache_version"].as_i64().unwrap()
    );
    for row in vectors["quote_names_digests"].as_array().unwrap() {
        let names: Vec<(String, String)> = row[0]
            .as_array()
            .unwrap()
            .iter()
            .map(|pair| {
                (
                    pair[0].as_str().unwrap().into(),
                    pair[1].as_str().unwrap().into(),
                )
            })
            .collect();
        assert_eq!(keys::quote_names_inspect(&names), row[2].as_str().unwrap());
        assert_eq!(keys::quote_names_digest(&names), row[1].as_str().unwrap());
    }
    for row in vectors["message_with_pr_cards"].as_array().unwrap() {
        let data = MessageKey {
            record: row["message"].as_str().unwrap().into(),
            cards: stamps(&row["cards"]),
            embeds: row["embeds"]
                .as_array()
                .map(|embeds| {
                    embeds
                        .iter()
                        .map(|pair| (pair[0].as_i64().unwrap(), timestamp(&pair[1])))
                        .collect()
                })
                .unwrap_or_default(),
            has_pull_requests: row["has_pull_requests"].as_bool().unwrap_or(false),
            pr_threads_stamp: timestamp(&row["pr_threads_stamp"]),
            pins: stamps(&row["pins"]),
            thread_messages_count: row["thread_messages_count"].as_i64(),
            poll: timestamp(&row["poll"]),
            system_note: row["system_note"].as_bool().unwrap_or(false),
            streaming: row["streaming"].as_bool().unwrap_or(false),
            agent_steps: stamps(&row["agent_steps"]),
            quotes: row["quoted_sources"]
                .as_array()
                .map(|sources| {
                    sources
                        .iter()
                        .map(|source| {
                            (
                                timestamp(&source["edited_at"])
                                    .max(timestamp(&source["updated_at"]))
                                    .unwrap(),
                                source["creator"].as_str().unwrap().into(),
                                source["room"].as_str().unwrap().into(),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default(),
        };
        for zone in ["UTC", "Hawaii"] {
            let zone_model = campfire_presentation::time::Zone::lookup(zone).unwrap();
            let key = keys::message_with_pr_cards(&data);
            assert_eq!(
                keys::expand(&key, &zone_model),
                row["expanded"][zone]["key"].as_str().unwrap(),
                "{}, {zone}",
                row["label"]
            );
            assert_eq!(
                keys::fragment("messages/_message", "0123abcd", &key, &zone_model),
                row["expanded"][zone]["fragment"].as_str().unwrap()
            );
        }
    }
    for row in vectors["sidebar_membership"].as_array().unwrap() {
        let participants: Option<Vec<i64>> = row["participant_ids"]
            .as_array()
            .map(|values| values.iter().map(|value| value.as_i64().unwrap()).collect());
        let key = keys::sidebar_membership(
            row["membership"].as_str().unwrap(),
            participants.as_deref(),
            row["administrator"].as_bool().unwrap(),
        );
        assert_eq!(
            keys::expand(&key, &campfire_presentation::time::Zone::utc()),
            row["key"].as_str().unwrap()
        );
    }
}
