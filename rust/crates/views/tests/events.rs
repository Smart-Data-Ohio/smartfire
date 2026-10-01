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
            message_key: vector["message_key"].as_str().unwrap(),
            entries: &entries,
        }
        .render()
        .unwrap();
        assert_bytes(&actual, vector["html"].as_str().unwrap(), "collection");
        use campfire_views::messages::{MessageComponents, MessageContent, MessageView, UserView};
        let message = MessageView {
            id: 601,
            client_message_id: vector["message_key"].as_str().unwrap().into(),
            room_id: 1,
            room_name: String::new(),
            creator: UserView {
                id: 1,
                name: String::new(),
                title: String::new(),
                avatar_url: String::new(),
                icon: None,
            },
            created_at: "2026-09-22T12:00:00Z".parse().unwrap(),
            updated_at: "2026-09-22T12:00:00Z".parse().unwrap(),
            all_emoji: false,
            content: MessageContent::Text {
                html: String::new(),
            },
            boosts: Vec::new(),
            details: Default::default(),
            components: MessageComponents {
                event_views: events,
                ..Default::default()
            },
        };
        assert_bytes(
            &campfire_views::messages::event_cards(&context(), &message).0,
            vector["html"].as_str().unwrap(),
            "message event collection",
        );
    }
}

fn context() -> campfire_views::ViewContext<'static> {
    campfire_views::ViewContext {
        current_user: None,
        account: campfire_views::AccountSummary {
            name: String::new(),
            logo_url: String::new(),
            has_logo: false,
        },
        flash_notice: None,
        flash_alert: None,
        platform: Default::default(),
        vapid_public_key: None,
        asset_path: &|path| path.into(),
        importmap_tags: "",
        stylesheet_tags: "",
        custom_styles: None,
        cable_url: String::new(),
        base_url: String::new(),
        request_url: String::new(),
        referrer: None,
        last_room_visited_id: None,
        app_version: String::new(),
        signed_stream_name: &|_| String::new(),
        time_zone: Zone::utc(),
        chrome: Default::default(),
    }
}
