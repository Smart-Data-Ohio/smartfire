//! Plain card facts and Rails' generic embed partial. No persistence or fetching here.
pub use campfire_presentation::link_embeds::*;

use askama::Template;
pub trait CardRendering {
    fn render(&self) -> String;
}
impl CardRendering for Card {
    fn render(&self) -> String {
        CardPartial { card: self }.render().expect("link card renders")
    }
}

#[derive(Template)]
#[template(path = "link_embeds/_card.html")]
struct CardPartial<'a> {
    card: &'a Card,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ws15e_generic_cards_match_pinned_rails_bytes() {
        let vectors: serde_json::Value = serde_json::from_str(include_str!("../../../vectors/ws15e_link_embed.json")).unwrap();
        for case in vectors["cards"].as_array().unwrap() {
            let mut card: Card = serde_json::from_value(case["attributes"].clone()).unwrap();
            card.url = case["url"].as_str().unwrap().into();
            assert_eq!(card.render(), case["html"].as_str().unwrap(), "{case}");
        }
    }
}
