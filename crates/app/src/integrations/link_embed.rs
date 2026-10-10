//! LinkEmbed domain: URL policy, persisted cache/references and transactional fetch requests.
pub mod fetcher;
pub mod metadata_parser;
pub mod store;
pub mod url_classifier;

pub use fetcher::{FetchJob, perform};
pub use store::{Embed, Reference, sync_message};

use crate::integrations::linkedin;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReferenceUrl {
    pub normalized_url: String,
    pub url: String,
}

/// The pure selection half of `LinkEmbed::ReferenceSync`: LinkedIn first, then generic
/// links, carrying each author's original URL beside the shared cache key. WS8a's
/// transaction seam will persist these and atomically enqueue the fetches. Its caller
/// must gate on markdown/forwarded_markdown and clear references for legacy bodies.
pub fn reference_urls(
    html: &str,
    markdown_source: &str,
    forward_note: &str,
) -> Result<Vec<ReferenceUrl>, campfire_richtext::dom::ParseError> {
    let suppressed = url_classifier::suppressed_urls(&[markdown_source, forward_note]);
    let body = linkedin::non_code_text(html)?;
    let text = [body.as_str(), forward_note].into_iter().filter(|s| !s.trim().is_empty()).collect::<Vec<_>>().join("\n");
    let mut selected = Vec::new();
    let linkedin_urls = linkedin::extract(&text).into_iter().filter_map(|r| url_classifier::normalize_url(&r.url));
    for normalized_url in linkedin_urls.chain(url_classifier::extract(&text, &suppressed)) {
        if suppressed.contains(&normalized_url) || selected.iter().any(|r: &ReferenceUrl| r.normalized_url == normalized_url) {
            continue;
        }
        let url = url_classifier::first_seen_url(&text, &normalized_url);
        selected.push(ReferenceUrl { normalized_url, url });
    }
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ws15e_reference_selection_keeps_raw_urls_per_message() {
        let vectors: serde_json::Value = serde_json::from_str(include_str!("../../../../vectors/ws15e_urls.json")).unwrap();
        for case in vectors["references"].as_array().unwrap() {
            let selected =
                reference_urls(case["html"].as_str().unwrap(), case["source"].as_str().unwrap(), case["note"].as_str().unwrap()).unwrap();
            assert_eq!(serde_json::to_value(selected).unwrap(), case["selected"], "{case}");
        }
    }

    #[test]
    fn a_link_inside_a_spoiler_is_not_unfurled() {
        let html = r#"<p><span class="spoiler" data-spoiler=""><a href="https://example.com/hidden">secret</a></span> <a href="https://example.com/shown">open</a></p>"#;
        let selected = reference_urls(html, "", "").unwrap();
        assert_eq!(selected.len(), 1, "{selected:?}");
        assert!(selected[0].url.contains("shown"), "{selected:?}");
        assert!(!selected[0].url.contains("hidden"), "{selected:?}");
    }

    #[test]
    fn a_link_whose_label_holds_a_spoiler_is_not_unfurled() {
        let html = r#"<p><a href="https://example.com/alice-dies" title="Alice dies">see <span class="spoiler" data-spoiler="">ending</span></a> <a href="https://example.com/shown">open</a></p>"#;
        let selected = reference_urls(html, "", "").unwrap();
        assert_eq!(selected.len(), 1, "{selected:?}");
        assert!(selected[0].url.contains("shown"), "{selected:?}");
    }
}
