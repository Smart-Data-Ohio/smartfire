//! Each expected byte comes from the pinned Rails renderer, including AR string coercions.
use serde_json::Value;
fn vectors() -> Value {
    serde_json::from_str(include_str!("../../../vectors/ws15e_card_html_audit.json")).unwrap()
}
#[test]
fn ws15e_generic_card_html_edge_audit() {
    for case in vectors()["generic"].as_array().unwrap() {
        let mut card: crate::link_embeds::Card =
            serde_json::from_value(case["attributes"].clone()).unwrap();
        card.url = case["url"].as_str().unwrap().into();
        assert_eq!(
            card.render(),
            case["html"].as_str().unwrap(),
            "{}",
            case["name"]
        );
    }
}
#[test]
fn ws15e_linkedin_card_html_edge_audit() {
    for case in vectors()["linkedin"].as_array().unwrap() {
        let mut embed: crate::link_embeds::Card =
            serde_json::from_value(case["attributes"].clone()).unwrap();
        embed.url = case["url"].as_str().unwrap().into();
        let card = crate::linkedin_cards::Card {
            embed,
            player_url: case["player_url"].as_str().map(str::to_owned),
        };
        assert_eq!(
            card.render(),
            case["html"].as_str().unwrap(),
            "{}",
            case["name"]
        );
    }
}
#[test]
fn ws15e_fizzy_card_html_edge_audit() {
    for case in vectors()["fizzy"].as_array().unwrap() {
        let zone = crate::time::Zone::lookup(case["zone"].as_str().unwrap()).unwrap();
        let frame = crate::fizzy_cards::Frame {
            account: "897362094",
            number: 579,
            web_url: "https://app.fizzy.do/897362094/cards/579",
            id: "card\"<&'frame",
            connect: false,
            payload: Some(&case["payload"]),
            error: case["error"].as_str(),
            zone: &zone,
        };
        assert_eq!(
            frame.render(),
            case["html"].as_str().unwrap(),
            "{}",
            case["name"]
        );
    }
}
