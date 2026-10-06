//! LinkedIn's card/chip partial, using the message's own raw reference URL.
use askama::Template;
pub struct Card {
    pub embed: crate::link_embeds::Card,
    pub player_url: Option<String>,
}
impl Card {
    pub fn render(&self) -> String {
        CardPartial { card: self }
            .render()
            .expect("LinkedIn card renders")
    }
}
#[derive(Template)]
#[template(path = "linkedin/posts/_card.html")]
struct CardPartial<'a> {
    card: &'a Card,
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    #[test]
    fn ws15e_linkedin_cards_match_pinned_rails_bytes() {
        let vectors: Value =
            serde_json::from_str(include_str!("../../../vectors/ws15e_linkedin_cards.json"))
                .unwrap();
        for case in vectors["cards"].as_array().unwrap() {
            let mut embed: crate::link_embeds::Card =
                serde_json::from_value(case["attributes"].clone()).unwrap();
            embed.url = case["url"].as_str().unwrap().into();
            let card = Card {
                embed,
                player_url: case["player_url"].as_str().map(str::to_owned),
            };
            assert_eq!(card.render(), case["html"].as_str().unwrap(), "{case}");
        }
    }
}
