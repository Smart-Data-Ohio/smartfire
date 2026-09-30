//! `Fizzy::CardUrl`: configured-origin matching, ordered unique pairs and a four-card cap.
use campfire_richtext::uri;
use regex::Regex;
use serde::Serialize;
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Reference {
    pub account_id: String,
    pub number: i64,
}
pub fn extract(text: &str, base: &str) -> Result<Vec<Reference>, &'static str> {
    let parsed = uri::parse(base)
        .ok()
        .filter(|u| u.host.as_deref().is_some_and(|s| !s.is_empty()));
    let prefix = match parsed {
        Some(u) => format!(
            r"{}://{}(?::[0-9]+)?",
            if u.scheme.as_deref() == Some("http") {
                "http"
            } else {
                "https"
            },
            regex::escape(u.host.as_deref().unwrap())
        ),
        None => r"https://app\.fizzy\.do".into(),
    };
    let pattern = Regex::new(&format!(
        r"{prefix}/(?P<account>[A-Za-z0-9_-]+)/cards/(?P<number>[0-9]+)\b"
    ))
    .unwrap();
    let mut refs = Vec::new();
    for capture in pattern.captures_iter(text) {
        // SQLite's integer range is also the range accepted by Rails when the card is persisted.
        let number = capture["number"]
            .parse()
            .map_err(|_| "Fizzy card number exceeds SQLite integer range")?;
        let pair = Reference {
            account_id: capture["account"].into(),
            number,
        };
        if !refs.contains(&pair) {
            refs.push(pair);
        }
        if refs.len() == 4 {
            break;
        }
    }
    Ok(refs)
}
