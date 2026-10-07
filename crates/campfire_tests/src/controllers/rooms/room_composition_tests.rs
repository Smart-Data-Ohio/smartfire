use crate::controllers::presenters::{
    page,
    test_support::{DAVID, TestApp},
};
use campfire_views::helpers::request_forgery::{RequestSecrets, rendering_with};
use campfire_views::rooms::RoomView;
use serde_json::Value;
#[tokio::test]
async fn room_composition_replaces_the_upstream_composer_with_rails_markdown() {
    let Some(test) = TestApp::boot().await else {
        return;
    };
    let mut browser = test.sign_in(DAVID).await;
    let html = browser.get("/rooms/486777696").await.text();
    for marker in [
        "composer__textarea",
        "markdown-autocomplete",
        "id=\"channel-members\"",
        "id=\"thread-panel\"",
        "id=\"poll-builder\"",
        "id=\"schedule-send\"",
    ] {
        assert!(html.contains(marker), "missing {marker}");
    }
    assert!(!html.contains("<lexxy-editor"));
}

#[tokio::test]
async fn room_composition_matches_thirty_complete_rails_partials() {
    let vectors: Value =
        serde_json::from_str(include_str!("room_composition_vectors.json")).unwrap();
    assert_eq!(vectors["cases"].as_array().unwrap().len(), 30);
    let Some(test) = TestApp::boot().await else {
        return;
    };
    for case in vectors["cases"].as_array().unwrap() {
        let room: RoomView = serde_json::from_value(case["input"]["room"].clone()).unwrap();
        let drive = serde_json::from_value(case["input"]["drive"].clone()).unwrap();
        let actual =
            page::render_detached_at(&test.booted.app, None, "http://campfire.test", |ctx| {
                rendering_with(
                    RequestSecrets {
                        tokens: Box::new(super::call_page_tests::Tokens),
                        csp_nonce: Some("NONCE".into()),
                    },
                    || {
                        campfire_views::rooms::composition::render(
                            ctx,
                            &room,
                            case["input"]["neutral_name"].as_str(),
                            case["partial"].as_str().unwrap(),
                            drive,
                        )
                    },
                )
            });
        let expected = case["html"].as_str().unwrap();
        if actual != expected {
            let scratch = std::path::PathBuf::from(std::env::var_os("TMPDIR").unwrap());
            std::fs::write(scratch.join("room-composition-actual.html"), &actual).unwrap();
            std::fs::write(scratch.join("room-composition-expected.html"), expected).unwrap();
            let at = actual
                .bytes()
                .zip(expected.bytes())
                .position(|(a, b)| a != b)
                .unwrap_or(actual.len().min(expected.len()));
            panic!(
                "{} first mismatch {at}: {:?} / {:?}",
                case["name"],
                actual.get(at.saturating_sub(40)..(at + 120).min(actual.len())),
                expected.get(at.saturating_sub(40)..(at + 120).min(expected.len()))
            );
        }
    }
}
