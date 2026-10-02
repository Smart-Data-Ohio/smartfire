//! `Google::DriveLink`: metadata-free extraction for the URL shapes supported by the browser.
use regex::Regex;
use serde_json::Value;
use std::sync::LazyLock;
static PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [r"(?i)\Ahttps://docs\.google\.com/(?:u/[0-9]+/)?(?:document|spreadsheets|presentation|forms)/(?:u/[0-9]+/)?d/(?P<id>[A-Za-z0-9_-]{10,})",
    r"(?i)\Ahttps://drive\.google\.com/(?:u/[0-9]+/)?file/(?:u/[0-9]+/)?d/(?P<id>[A-Za-z0-9_-]{10,})",
    r"(?i)\Ahttps://drive\.google\.com/(?:u/[0-9]+/)?drive/(?:u/[0-9]+/)?folders/(?P<id>[A-Za-z0-9_-]{10,})",
    r"(?i)\Ahttps://drive\.google\.com/(?:u/[0-9]+/)?open\?(?:[^#]*&)?id=(?P<id>[A-Za-z0-9_-]{10,})(?:&|#|\z)"]
    .iter().map(|pattern|Regex::new(pattern).expect("Rails Drive link pattern")).collect()
});
pub fn file_id(value: &Value) -> Option<String> {
    let text = campfire_richtext::ruby::json_value_to_s(value);
    PATTERNS.iter().find_map(|pattern| {
        pattern
            .captures(&text)
            .map(|captures| captures["id"].to_string())
    })
}
pub fn valid_id(value: &Value) -> bool {
    crate::models::message::valid_drive_file_id(&campfire_richtext::ruby::json_value_to_s(value))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_pinned_rails_url_and_id_vectors() {
        let v: Value =
            serde_json::from_str(include_str!("../../../../vectors/google_drive_link.json"))
                .unwrap();
        for case in v["urls"].as_array().unwrap() {
            assert_eq!(
                file_id(&case["input"]).as_deref(),
                case["output"].as_str(),
                "{case}"
            );
        }
        for case in v["ids"].as_array().unwrap() {
            assert_eq!(
                valid_id(&case["input"]),
                case["valid"].as_bool().unwrap(),
                "{case}"
            );
        }
    }
}
