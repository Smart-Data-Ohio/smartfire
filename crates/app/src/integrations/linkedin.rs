//! `Linkedin::PostUrl` (`app/models/linkedin/post_url.rb`).
use regex::Regex;
use serde::Serialize;
use std::sync::LazyLock;

static PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"https?://(?:www\.)?linkedin\.com/(?:posts/(?P<slug>[^/?# \t\r\n\x0b\x0c<>"'()\]]+)|feed/update/(?P<urn>urn:li:(?:activity|share|ugcPost):[0-9]+))"#).unwrap()
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Reference {
    pub url: String,
    pub urn: Option<String>,
}

fn allowed_urn_trailer(text: &str) -> bool {
    text.chars().next().is_none_or(|c| c.is_ascii_whitespace() || "/?#<>\"'()].,;:!?}".contains(c))
}

fn matches(text: &str) -> impl Iterator<Item = regex::Captures<'_>> {
    PATTERN.captures_iter(text).filter(move |m| m.name("urn").is_none_or(|urn| allowed_urn_trailer(&text[urn.end()..])))
}

fn clean_url(url: &str) -> &str {
    url.trim_end_matches(['.', ',', ';', ':', '!', '?', '}'])
}

pub fn is_post_url(text: &str) -> bool {
    matches(text).next().is_some()
}

pub fn extract(text: &str) -> Vec<Reference> {
    let mut references = Vec::new();
    for matched in matches(text) {
        let url = clean_url(matched.get(0).unwrap().as_str()).to_string();
        if references.iter().any(|r: &Reference| r.url == url) {
            continue;
        }
        let urn = matched.name("urn").map(|m| m.as_str().to_string());
        references.push(Reference { url, urn });
        if references.len() == 3 {
            break;
        }
    }
    references
}

pub fn embed_url_for(url: &str) -> Option<String> {
    matches(clean_url(url)).next()?.name("urn").map(|urn| format!("https://www.linkedin.com/embed/feed/update/{}", urn.as_str()))
}

/// Text and hrefs outside code/pre, with block boundaries preserved (the Rails HTML5 DOM).
pub fn non_code_text(html: &str) -> Result<String, campfire_richtext::dom::ParseError> {
    use campfire_richtext::dom::{Dom, NodeData, NodeId};
    fn walk(dom: &Dom, node: NodeId, text: &mut String, hrefs: &mut Vec<String>) {
        let name = dom.name(node);
        // Code and spoilers are not part of the message a card may unfurl. A spoiler's text and
        // its links stay hidden until the reader reveals that spoiler.
        let spoiler = dom.has_attr(node, "data-spoiler")
            || dom.attr(node, "class").is_some_and(|classes| classes.split_whitespace().any(|class| class == "spoiler"));
        if matches!(name.as_ref(), "code" | "pre") || spoiler {
            return;
        }
        if let NodeData::Text(value) = &dom.node(node).data {
            text.push_str(value);
        }
        if name == "br" {
            text.push('\n');
        }
        if name == "a"
            && let Some(href) = dom.attr(node, "href")
        {
            hrefs.push(href.to_string());
        }
        for child in dom.children(node) {
            walk(dom, *child, text, hrefs);
        }
        if matches!(
            name.as_ref(),
            "address"
                | "article"
                | "aside"
                | "blockquote"
                | "dd"
                | "dialog"
                | "div"
                | "dl"
                | "dt"
                | "fieldset"
                | "figcaption"
                | "figure"
                | "footer"
                | "form"
                | "h1"
                | "h2"
                | "h3"
                | "h4"
                | "h5"
                | "h6"
                | "header"
                | "hgroup"
                | "hr"
                | "li"
                | "main"
                | "nav"
                | "ol"
                | "p"
                | "section"
                | "table"
                | "td"
                | "th"
                | "tr"
                | "ul"
        ) {
            text.push('\n');
        }
    }
    let mut dom = Dom::new();
    let root = dom.parse_fragment(html)?;
    let (mut text, mut hrefs) = (String::new(), Vec::new());
    walk(&dom, root, &mut text, &mut hrefs);
    for href in hrefs {
        text.push('\n');
        text.push_str(&href);
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ws15e_linkedin_url_and_html_corpus_matches_rails() {
        let vectors: serde_json::Value = serde_json::from_str(include_str!("../../../../vectors/ws15e_urls.json")).unwrap();
        for case in vectors["linkedin"].as_array().unwrap() {
            let text = case["text"].as_str().unwrap();
            assert_eq!(is_post_url(text), case["is_post"].as_bool().unwrap(), "{case}");
            assert_eq!(embed_url_for(text).as_deref(), case["embed"].as_str(), "{case}");
            assert_eq!(serde_json::to_value(extract(text)).unwrap(), case["extracted"], "{case}");
        }
        for case in vectors["non_code"].as_array().unwrap() {
            assert_eq!(non_code_text(case["html"].as_str().unwrap()).unwrap(), case["text"].as_str().unwrap(), "{case}");
        }
    }

    #[test]
    fn spoilers_are_skipped_like_code() {
        let html = r#"<p>see https://www.linkedin.com/feed/update/urn:li:activity:111 <a href="https://example.com/shown">shown</a> <span class="spoiler" data-spoiler="">https://www.linkedin.com/feed/update/urn:li:activity:222 <a href="https://example.com/hidden" title="secret">tweet</a></span></p>"#;
        let text = non_code_text(html).unwrap();
        assert!(text.contains("urn:li:activity:111"), "{text}");
        assert!(text.contains("https://example.com/shown"), "{text}");
        assert!(!text.contains("urn:li:activity:222"), "{text}");
        assert!(!text.contains("example.com/hidden"), "{text}");
        assert!(!text.contains("secret"), "{text}");
        assert!(!text.contains("tweet"), "{text}");
    }
}
