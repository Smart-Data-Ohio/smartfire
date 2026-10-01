use super::*;
fn context(zone: &str) -> ViewContext<'static> {
    ViewContext {
        current_user: None,
        account: crate::AccountSummary {
            name: String::new(),
            logo_url: String::new(),
            has_logo: false,
        },
        flash_notice: None,
        flash_alert: None,
        platform: Default::default(),
        vapid_public_key: None,
        asset_path: &|s| s.into(),
        importmap_tags: "",
        stylesheet_tags: "",
        custom_styles: None,
        cable_url: "/cable".into(),
        base_url: "http://example.org".into(),
        request_url: "http://example.org/".into(),
        referrer: None,
        last_room_visited_id: None,
        app_version: "parity".into(),
        signed_stream_name: &|_| String::new(),
        time_zone: crate::time::Zone::lookup(zone).unwrap(),
        chrome: Default::default(),
    }
}
#[test]
fn ws15e_x_cards_match_pinned_rails_bytes() {
    let vectors: Value =
        serde_json::from_str(include_str!("../../../../vectors/ws15e_twitter_cards.json")).unwrap();
    for case in vectors["cards"].as_array().unwrap() {
        let mut card: Card = serde_json::from_value(case["attributes"].clone()).unwrap();
        card.view_url = case["view_url"].as_str().unwrap().into();
        card.display_name = case["display_name"].as_str().unwrap().into();
        card.display_handle = case["display_handle"].as_str().map(str::to_owned);
        card.profile_url = case["profile_url"].as_str().map(str::to_owned);
        card.logo_url = case["logo_url"].as_str().map(str::to_owned);
        assert_eq!(
            card.render(&context(case["zone"].as_str().unwrap())),
            case["html"].as_str().unwrap(),
            "{}",
            case["name"]
        );
    }
}
#[test]
fn ws15e_x_compact_counts_match_pinned_rails_bytes() {
    let vectors: Value =
        serde_json::from_str(include_str!("../../../../vectors/ws15e_twitter_cards.json")).unwrap();
    for case in vectors["counts"].as_array().unwrap() {
        assert_eq!(
            compact_count(&case["number"].as_i64().unwrap()),
            case["html"].as_str().unwrap()
        );
    }
}
#[test]
fn ws15e_x_missing_logo_does_not_break_the_card() {
    let card = Card {
        post_id: "1".into(),
        fetched_at: Some("2026-03-02T16:00:00Z".parse().unwrap()),
        view_url: "https://x.com/i/status/1".into(),
        ..Default::default()
    };
    let html = card.render(&context("UTC"));
    assert!(!html.contains("x-post-card__logo"));
    assert!(html.contains("x-post-card__header"));
}
