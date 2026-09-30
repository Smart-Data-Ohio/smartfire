use askama::Template;
use campfire_views::{
    events::{Attendance, AttendanceView, Card, CardView},
    helpers::request_forgery::{self, AuthenticityTokens, RequestSecrets},
    time::Zone,
};
use serde_json::Value;
struct Tokens;
impl AuthenticityTokens for Tokens {
    fn global(&self) -> String {
        "GLOBAL".into()
    }
    fn for_form(&self, action: &str, method: &str) -> String {
        format!("{method}:{action}")
    }
}
fn vectors() -> Value {
    serde_json::from_str(include_str!("golden/event-fragments.json")).unwrap()
}
fn assert_bytes(actual: &str, expected: &str, name: &str) {
    if actual != expected {
        let index = actual
            .bytes()
            .zip(expected.bytes())
            .position(|(a, b)| a != b)
            .unwrap_or(actual.len().min(expected.len()));
        panic!("{name}: first difference byte {index}\nactual: {actual:?}\nexpected: {expected:?}");
    }
}
#[test]
fn event_cards_are_byte_identical_to_rails_fragments() {
    for vector in vectors()["cards"].as_array().unwrap() {
        let event: CardView = serde_json::from_value(vector["event"].clone()).unwrap();
        let zone = Zone::lookup(vector["viewer_zone"].as_str().unwrap()).unwrap();
        let actual = Card {
            event: &event,
            message_id: "601",
            viewer_zone: &zone,
        }
        .render()
        .unwrap();
        assert_bytes(
            &actual,
            vector["html"].as_str().unwrap(),
            vector["name"].as_str().unwrap(),
        );
        assert!(!actual.contains("authenticity_token"));
        assert!(!actual.contains("NONCE"));
    }
}
#[test]
fn attendance_frames_are_byte_identical_to_rails_fragments() {
    for vector in vectors()["attendances"].as_array().unwrap() {
        let view: AttendanceView = serde_json::from_value(vector.clone()).unwrap();
        let actual = request_forgery::rendering_with(
            RequestSecrets {
                tokens: Box::new(Tokens),
                csp_nonce: Some("NONCE".into()),
            },
            || Attendance { view: &view }.render().unwrap(),
        );
        assert_bytes(&actual, vector["html"].as_str().unwrap(), "attendance");
    }
}

#[test]
fn meet_links_match_rails_uri_filtering() {
    for vector in vectors()["meet_links"].as_array().unwrap() {
        assert_eq!(
            rails_compat::safe_https(vector[0].as_str().unwrap()).as_deref(),
            vector[1].as_str(),
            "{}",
            vector[0]
        );
    }
}

#[test]
fn event_card_collections_match_rails_empty_and_populated() {
    for vector in vectors()["collections"].as_array().unwrap() {
        let events: Vec<CardView> = serde_json::from_value(vector["events"].clone()).unwrap();
        let entries = campfire_views::events::card_entries(&events, "601", &Zone::utc());
        let actual = campfire_views::events::Cards {
            message_key: "ws14e-golden-message",
            entries: &entries,
        }
        .render()
        .unwrap();
        assert_bytes(&actual, vector["html"].as_str().unwrap(), "collection");
    }
}
