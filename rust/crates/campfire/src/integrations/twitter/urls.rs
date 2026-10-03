//! Twitter::PostUrl. IDs remain strings; handles affect fetch paths, never deduplication.
use regex::Regex;
use serde::Serialize;
use std::sync::LazyLock;
static PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"https?://(?:www\.|mobile\.)?(?:twitter\.com|x\.com)/(?:i/(?:web/)?status/|(?P<handle>[A-Za-z0-9_]{1,15})/status(?:es)?/)(?P<id>[0-9]{1,25})\b").unwrap()
});
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Reference {
    pub handle: Option<String>,
    pub post_id: String,
}
pub fn is_post_url(url: &str) -> bool {
    PATTERN.is_match(url)
}
/// PostFetcher#quote_url: the whole value must start with a post URL. The
/// shared pattern is unanchored for message text, where any prefix is fine.
pub fn starts_with_post_url(url: &str) -> bool {
    PATTERN.find(url).is_some_and(|m| m.start() == 0)
}
pub fn extract(text: &str) -> Vec<Reference> {
    let mut refs = Vec::<Reference>::new();
    for matched in PATTERN.captures_iter(text) {
        let id = &matched["id"];
        if refs.iter().any(|r| r.post_id == id) {
            continue;
        }
        refs.push(Reference {
            handle: matched.name("handle").map(|m| m.as_str().to_owned()),
            post_id: id.to_owned(),
        });
        if refs.len() == 4 {
            break;
        }
    }
    refs
}
/// Twitter and LinkedIn's pinned non-code walkers have exactly the same block-tag list,
/// newline boundaries and final href scan. Keep one HTML5 implementation.
pub fn non_code_text(html: &str) -> Result<String, campfire_richtext::dom::ParseError> {
    super::super::linkedin::non_code_text(html)
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    #[test]
    fn quote_urls_must_start_with_a_post_url() {
        assert!(starts_with_post_url("https://x.com/NASA/status/123"));
        assert!(starts_with_post_url("https://x.com/NASA/status/123?s=20"));
        assert!(starts_with_post_url("https://twitter.com/i/web/status/123"));
        let prefixed = "javascript:void(0)//https://x.com/i/status/123";
        assert!(!starts_with_post_url(prefixed));
        assert!(is_post_url(prefixed), "text detection stays unanchored");
        assert!(!starts_with_post_url(" https://x.com/NASA/status/123"));
        assert!(!starts_with_post_url(""));
    }
    #[test]
    fn ws15e_x_urls_and_non_code_text_match_pinned_rails() {
        let vectors: Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../vectors/ws15e_twitter_text.json"
        )))
        .unwrap();
        for case in vectors["extract"].as_array().unwrap() {
            let text = case["text"].as_str().unwrap_or("");
            assert_eq!(json!(extract(text)), case["refs"], "{case}");
            assert_eq!(is_post_url(text), case["matches"].as_bool().unwrap());
        }
        for case in vectors["non_code"].as_array().unwrap() {
            let text = non_code_text(case["html"].as_str().unwrap_or("")).unwrap();
            assert_eq!(text, case["text"].as_str().unwrap());
            assert_eq!(json!(extract(&text)), case["refs"]);
        }
    }
}
