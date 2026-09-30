//! Twitter::PostFormatter. Only locally escaped strings become HTML-safe.
use crate::helpers::html::{Html, escape, raw};
// WS8bm2 adapter: use the existing view helper until WS15e is merged.
fn url_encode(text: &str) -> String {
    crate::helpers::url::cgi_escape(text).replace('+', "%20")
}
use regex::Regex;
use std::sync::LazyLock;
static URL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"https?://[^ \t\r\n\x0b\x0c<>"'`\])}]+"#).unwrap());
static TOKEN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"@[A-Za-z0-9_]{1,15}\b|#[\p{Alphabetic}\p{Nd}_]+").unwrap());
fn newline(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\n', "<br>")
}
pub fn plain(text: &str) -> Html {
    raw(newline(&escape(text)))
}
pub fn clamp(text: &str) -> bool {
    text.chars().count() > 480 || text.bytes().filter(|b| *b == b'\n').count() >= 12
}
fn chunk(text: &str) -> String {
    let mut output = String::new();
    let mut start = 0;
    for token in TOKEN.find_iter(text) {
        // Ruby's negative lookbehind \w is ASCII, while \b above is Unicode-aware.
        if text[..token.start()]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            continue;
        }
        output.push_str(&escape(&text[start..token.start()]));
        let end = token.end();
        let token = token.as_str();
        let label = escape(token);
        let href = if let Some(handle) = token.strip_prefix('@') {
            format!("https://x.com/{handle}")
        } else {
            format!("https://x.com/hashtag/{}", url_encode(&token[1..]))
        };
        output.push_str(&format!(
            "<a href=\"{href}\" target=\"_blank\" rel=\"noopener noreferrer\">{label}</a>"
        ));
        start = end;
    }
    output.push_str(&escape(&text[start..]));
    output
}
fn word(text: &str) -> String {
    let mut out = String::new();
    let mut start = 0;
    for matched in URL.find_iter(text) {
        out.push_str(&chunk(&text[start..matched.start()]));
        let original = matched.as_str();
        let url = original.trim_end_matches(['.', ',', ';', ':', '!', '?']);
        let escaped = escape(url);
        out.push_str(&format!(
            "<a href=\"{escaped}\" target=\"_blank\" rel=\"noopener noreferrer\">{escaped}</a>"
        ));
        out.push_str(&chunk(&original[url.len()..]));
        start = matched.end();
    }
    out.push_str(&chunk(&text[start..]));
    out
}
pub fn format(text: &str) -> Html {
    let mut out = String::new();
    let mut start = 0;
    for (index, ch) in text.char_indices() {
        if matches!(ch, ' ' | '\t' | '\r' | '\n' | '\u{b}' | '\u{c}') {
            out.push_str(&word(&text[start..index]));
            out.push(ch);
            start = index + ch.len_utf8();
        }
    }
    out.push_str(&word(&text[start..]));
    raw(newline(&out))
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    #[test]
    fn ws15e_x_formatter_matches_pinned_rails_bytes() {
        let vectors: Value =
            serde_json::from_str(include_str!("../../../../vectors/ws15e_twitter_text.json"))
                .unwrap();
        for case in vectors["format"].as_array().unwrap() {
            let text = case["text"].as_str().unwrap_or("");
            assert_eq!(format(text).0, case["format"].as_str().unwrap(), "{case}");
            assert_eq!(plain(text).0, case["plain"].as_str().unwrap());
            assert_eq!(clamp(text), case["clamp"].as_bool().unwrap());
        }
    }
}
