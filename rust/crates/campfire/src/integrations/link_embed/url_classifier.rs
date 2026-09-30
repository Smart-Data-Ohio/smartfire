//! `LinkEmbed::UrlClassifier` and `LinkEmbed.normalize_url`, from our fork.
use crate::integrations::linkedin;
use campfire_richtext::uri;
use regex::Regex;
use std::sync::LazyLock;

static URL_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)https?://[^ \t\r\n\x0b\x0c<>"']+"#).unwrap());
static SUPPRESSED_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)<(https?://[^<> \t\r\n\x0b\x0c]+)>").unwrap());
static INTERNAL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)/rooms/[0-9]+").unwrap());
static GITHUB: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"https://github\.com/(?P<owner>[A-Za-z0-9_.-]+)/(?P<repo>[A-Za-z0-9_.-]+)/(?:pull|pulls)/[0-9]+\b").unwrap()
});
static TWITTER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"https?://(?:www\.|mobile\.)?(?:twitter\.com|x\.com)/(?:i/(?:web/)?status/|[A-Za-z0-9_]{1,15}/status(?:es)?/)[0-9]{1,25}\b")
        .unwrap()
});
static DRIVE: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"(?i)^https://docs\.google\.com/(?:u/[0-9]+/)?(?:document|spreadsheets|presentation|forms)/(?:u/[0-9]+/)?d/[A-Za-z0-9_-]{10,}",
        r"(?i)^https://drive\.google\.com/(?:u/[0-9]+/)?file/(?:u/[0-9]+/)?d/[A-Za-z0-9_-]{10,}",
        r"(?i)^https://drive\.google\.com/(?:u/[0-9]+/)?drive/(?:u/[0-9]+/)?folders/[A-Za-z0-9_-]{10,}",
        r"(?i)^https://drive\.google\.com/(?:u/[0-9]+/)?open\?(?:[^#]*&)?id=[A-Za-z0-9_-]{10,}(?:&|#|$)",
    ]
    .into_iter()
    .map(|s| Regex::new(s).unwrap())
    .collect()
});

pub fn normalize_url(url: &str) -> Option<String> {
    let parsed = uri::parse(url.trim_matches(|c| matches!(c, '\0' | '\t' | '\n' | '\x0b' | '\x0c' | '\r' | ' '))).ok()?;
    let host = parsed.host.as_deref().filter(|s| !s.is_empty())?;
    if !parsed.is_http() {
        return None;
    }
    let scheme = parsed.scheme.as_deref()?.to_ascii_lowercase();
    let mut key = format!("{scheme}://{}", host.to_ascii_lowercase());
    if let Some(port) = parsed.port
        && port != if scheme == "http" { 80 } else { 443 }
    {
        key.push_str(&format!(":{port}"));
    }
    let path = parsed.path.as_deref().filter(|p| !p.is_empty()).unwrap_or("/");
    key.push_str(if path == "/" { path } else { path.strip_suffix('/').unwrap_or(path) });
    if let Some(query) = parsed.query.filter(|q| !q.is_empty()) {
        key.push('?');
        key.push_str(&query);
    }
    Some(key)
}

pub fn github_pr_url(url: &str) -> bool {
    GITHUB.captures_iter(url).any(|m| !matches!(&m["owner"], "." | "..") && !matches!(&m["repo"], "." | ".."))
}

/// Configured-origin matching from `Fizzy::CardUrl.pattern`; only this predicate is needed
/// by the composer. Fizzy's persistence and client remain a separate integration.
pub fn fizzy_card_url(url: &str, base: &str) -> bool {
    let parsed = uri::parse(base).ok().filter(|u| u.host.as_deref().is_some_and(|h| !h.is_empty()));
    match parsed {
        Some(parsed) => {
            let scheme = if parsed.scheme.as_deref() == Some("http") { "http" } else { "https" };
            Regex::new(&format!(r"{scheme}://{}(?::[0-9]+)?/[A-Za-z0-9_-]+/cards/[0-9]+\b", regex::escape(parsed.host.as_deref().unwrap())))
                .unwrap()
                .is_match(url)
        }
        None => Regex::new(r"https://app\.fizzy\.do/[A-Za-z0-9_-]+/cards/[0-9]+\b").unwrap().is_match(url),
    }
}

pub fn special_url(url: &str) -> bool {
    github_pr_url(url)
        || TWITTER.is_match(url)
        || linkedin::is_post_url(url)
        || DRIVE.iter().any(|r| r.is_match(url))
        || uri::parse(url).ok().and_then(|u| u.host).is_some_and(|h| h.to_ascii_lowercase().contains("fizzy"))
        || INTERNAL.is_match(url)
}

pub fn clean_candidate(url: &str) -> &str {
    let mut cleaned = url;
    loop {
        let before = cleaned;
        cleaned = cleaned.trim_end_matches(['.', ',', ';', ':', '!', '?', '}']);
        for (opener, closer) in [('(', ')'), ('[', ']')] {
            while cleaned.ends_with(closer)
                && cleaned.chars().filter(|c| *c == closer).count() > cleaned.chars().filter(|c| *c == opener).count()
            {
                cleaned = &cleaned[..cleaned.len() - 1];
            }
        }
        if cleaned == before {
            return cleaned;
        }
    }
}

pub fn suppressed_urls(sources: &[&str]) -> Vec<String> {
    let mut suppressed = Vec::new();
    for source in sources {
        for matched in SUPPRESSED_PATTERN.captures_iter(source) {
            if let Some(url) = normalize_url(clean_candidate(&matched[1]))
                && !suppressed.contains(&url)
            {
                suppressed.push(url);
            }
        }
    }
    suppressed
}

pub fn extract(text: &str, suppressed: &[String]) -> Vec<String> {
    let mut urls = Vec::new();
    for matched in URL_PATTERN.find_iter(text) {
        let candidate = clean_candidate(matched.as_str());
        let Some(normalized) = normalize_url(candidate) else { continue };
        if urls.contains(&normalized) || suppressed.contains(&normalized) || special_url(candidate) {
            continue;
        }
        urls.push(normalized);
        if urls.len() == 3 {
            break;
        }
    }
    urls
}

pub fn first_seen_url(text: &str, normalized: &str) -> String {
    URL_PATTERN
        .find_iter(text)
        .map(|m| clean_candidate(m.as_str()))
        .find(|url| normalize_url(url).as_deref() == Some(normalized))
        .unwrap_or(normalized)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ws15e_link_url_policy_matches_rails() {
        let vectors: serde_json::Value = serde_json::from_str(include_str!("../../../../../vectors/ws15e_urls.json")).unwrap();
        for case in vectors["normalized"].as_array().unwrap() {
            assert_eq!(normalize_url(case["url"].as_str().unwrap()).as_deref(), case["expected"].as_str(), "{case}");
        }
        for case in vectors["classifier"].as_array().unwrap() {
            let text = case["text"].as_str().unwrap();
            assert_eq!(serde_json::json!(extract(text, &[])), case["extracted"], "{case}");
            assert_eq!(special_url(text), case["special"].as_bool().unwrap(), "{case}");
            assert_eq!(github_pr_url(text), case["github"].as_bool().unwrap(), "{case}");
        }
        let sources: Vec<&str> = vectors["suppression"]["sources"].as_array().unwrap().iter().map(|s| s.as_str().unwrap()).collect();
        let suppressed = suppressed_urls(&sources);
        assert_eq!(serde_json::json!(suppressed), vectors["suppression"]["suppressed"]);
        assert_eq!(
            serde_json::json!(extract("https://example.com/kept https://example.com/hidden https://example.com/a/", &suppressed)),
            vectors["suppression"]["extracted"]
        );
        for case in vectors["fizzy"].as_array().unwrap() {
            assert_eq!(
                fizzy_card_url(case["url"].as_str().unwrap(), case["base"].as_str().unwrap()),
                case["card"].as_bool().unwrap(),
                "{case}"
            );
        }
    }
}
