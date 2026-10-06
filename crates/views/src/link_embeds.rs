//! Plain card facts and Rails' generic embed partial. No persistence or fetching here.
use askama::Template;
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Card {
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub site_name: Option<String>,
    pub image_url: Option<String>,
}
impl Card {
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref().filter(|s| !crate::helpers::is_blank(s))
    }
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref().filter(|s| !crate::helpers::is_blank(s))
    }
    pub fn site_name(&self) -> Option<&str> {
        self.site_name.as_deref().filter(|s| !crate::helpers::is_blank(s))
    }
    pub fn image(&self) -> Option<&str> {
        self.image_url.as_deref().filter(|s| !crate::helpers::is_blank(s))
    }
    pub fn usable(&self) -> bool {
        self.title().is_some() || self.description().is_some()
    }
    pub fn render(&self) -> String {
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
