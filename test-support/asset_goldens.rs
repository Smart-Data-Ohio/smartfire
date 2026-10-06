//! Page parity permits stale fingerprints only in identified local pipeline URL fields.
//! All surrounding HTML/JSON/header bytes and explicitly frozen fixture fields stay exact.

use std::{ops::Range, sync::LazyLock};

pub fn compare(name: &str, actual: &str, expected: &str) -> bool {
    compare_with_frozen_fields(name, actual, expected, &[])
}

/// Field ordinals come from the expected render, not a global URL allowlist. Each selected
/// fixture field stays byte-exact even if another field resolves that same asset live.
pub fn compare_with_frozen_fields(
    name: &str,
    actual: &str,
    expected: &str,
    frozen: &[usize],
) -> bool {
    match matching_fields(actual, expected, frozen) {
        Ok(()) => true,
        Err(error) => {
            eprintln!("{name}: {error}");
            false
        }
    }
}

/// Serialized fixture URLs identify their expected src fields. Refuse ambiguous provenance
/// rather than silently treating an additional live use of the same URL as frozen.
pub fn frozen_fixture_fields(expected: &str, inputs: &[&str]) -> Vec<usize> {
    let fields = url_fields(expected);
    let mut frozen = Vec::new();
    for input in inputs {
        let matches: Vec<_> = fields
            .iter()
            .enumerate()
            .filter_map(|(index, range)| (&expected[range.clone()] == *input).then_some(index))
            .collect();
        assert_eq!(
            matches.len(),
            1,
            "fixture URL must identify one field: {input}"
        );
        frozen.push(matches[0]);
    }
    frozen
}

#[cfg(test)]
fn matching_bytes(actual: &str, expected: &str) -> Result<(), String> {
    matching_fields(actual, expected, &[])
}

fn matching_fields(actual: &str, expected: &str, frozen: &[usize]) -> Result<(), String> {
    let actual_fields = url_fields(actual);
    let expected_fields = url_fields(expected);
    if actual_fields.len() != expected_fields.len() {
        return Err("URL field count differs".into());
    }
    if frozen.iter().any(|index| *index >= expected_fields.len()) {
        return Err("frozen fixture field is absent".into());
    }
    let actual = logical_references(actual, &actual_fields, true, frozen)?;
    let expected = logical_references(expected, &expected_fields, false, frozen)?;
    if actual == expected {
        return Ok(());
    }
    let at = actual
        .bytes()
        .zip(expected.bytes())
        .position(|(a, b)| a != b)
        .unwrap_or(actual.len().min(expected.len()));
    Err(format!(
        "page differs at byte {at} after validating asset digests ({} vs {} bytes)",
        actual.len(),
        expected.len()
    ))
}

// Only quoted/unquoted href/src/content attribute values are candidates. Consume complete
// tags/attributes so matching text inside another attribute, a comment or raw-text element
// cannot become a URL field. Original offsets preserve every byte outside the URL value.
static TAGS: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(concat!(
        r"(?is)<!--.*?(?:-->|\z)|<![^>]*(?:>|\z)|<(?P<close>/)?(?P<tag>[a-z][a-z0-9:-]*)",
        r#"(?P<attrs>(?:[^"'<>]|"[^"]*"|'[^']*')*)>"#,
    ))
    .unwrap()
});
static ATTRS: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(concat!(
        r#"(?:^|\s)(?P<name>[^\s"'<>/=]+)\s*=\s*"#,
        r#"(?:"(?P<double>[^"]*)"|'(?P<single>[^']*)'|(?P<bare>[^\s"'=<>`]+))"#,
    ))
    .unwrap()
});
static STRINGS: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r#""(?:[^"\\]|\\.)*""#).unwrap());
static LINKS: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?:^\s*|,\s*)<([^<>]*)>\s*;").unwrap());

fn url_fields(text: &str) -> Vec<Range<usize>> {
    if text.trim_start().starts_with(['{', '[']) {
        return json_fields(text, 0, false);
    }
    if let Some(first) = LINKS.find(text)
        && first.start() == 0
    {
        return LINKS
            .captures_iter(text)
            .map(|cap| cap.get(1).unwrap().range())
            .collect();
    }
    let mut fields = Vec::new();
    let mut raw: Option<(String, bool, usize)> = None;
    for tag in TAGS.captures_iter(text) {
        let Some(name) = tag.name("tag") else {
            continue;
        };
        let closing = tag.name("close").is_some();
        if let Some((raw_name, importmap, start)) = &raw {
            if closing && name.as_str().eq_ignore_ascii_case(raw_name) {
                if *importmap {
                    fields.extend(json_fields(
                        &text[*start..tag.get(0).unwrap().start()],
                        *start,
                        true,
                    ));
                }
                raw = None;
            }
            continue;
        }
        if closing {
            continue;
        }
        let attrs = tag.name("attrs").unwrap();
        let mut importmap = false;
        for attr in ATTRS.captures_iter(attrs.as_str()) {
            let value = attr
                .name("double")
                .or_else(|| attr.name("single"))
                .or_else(|| attr.name("bare"))
                .unwrap();
            let key = attr.name("name").unwrap().as_str();
            if ["href", "src", "content"]
                .iter()
                .any(|name| key.eq_ignore_ascii_case(name))
            {
                fields.push(attrs.start() + value.start()..attrs.start() + value.end());
            }
            if key.eq_ignore_ascii_case("type") && value.as_str() == "importmap" {
                importmap = true
            }
        }
        if ["script", "style", "textarea", "title"]
            .iter()
            .any(|tag| name.as_str().eq_ignore_ascii_case(tag))
        {
            raw = Some((
                name.as_str().to_string(),
                importmap && name.as_str().eq_ignore_ascii_case("script"),
                tag.get(0).unwrap().end(),
            ));
        }
    }
    fields
}

fn json_fields(text: &str, offset: usize, importmap: bool) -> Vec<Range<usize>> {
    if serde_json::from_str::<serde_json::Value>(text).is_err() {
        return Vec::new();
    }
    let strings: Vec<_> = STRINGS.find_iter(text).collect();
    strings
        .iter()
        .enumerate()
        .filter_map(|(index, value)| {
            // Object keys are strict; ordinary JSON only permits the named URL fields (PWA src).
            if text[value.end()..].trim_start().starts_with(':') {
                return None;
            }
            let named_value = index.checked_sub(1).is_some_and(|previous| {
                let key = &strings[previous];
                ["href", "src", "content"].contains(&&key.as_str()[1..key.len() - 1])
                    && text[key.end()..value.start()].trim() == ":"
            });
            (importmap || named_value)
                .then_some(offset + value.start() + 1..offset + value.end() - 1)
        })
        .collect()
}

fn logical_references(
    text: &str,
    fields: &[Range<usize>],
    live: bool,
    frozen: &[usize],
) -> Result<String, String> {
    let mut output = String::with_capacity(text.len());
    let mut end = 0;
    for (index, field) in fields.iter().enumerate() {
        output.push_str(&text[end..field.start]);
        let value = &text[field.clone()];
        let replacement = if frozen.contains(&index) {
            None
        } else {
            logical_url(value, live)?
        };
        output.push_str(replacement.as_deref().unwrap_or(value));
        end = field.end;
    }
    output.push_str(&text[end..]);
    Ok(output)
}

fn logical_url(value: &str, live: bool) -> Result<Option<String>, String> {
    // Origin, nested paths and query values never qualify. Only the entire local path does.
    let path_end = value.find(['?', '#']).unwrap_or(value.len());
    let Some(path) = value[..path_end].strip_prefix("/assets/") else {
        return Ok(None);
    };
    let Some((stem, suffix)) = path.rsplit_once('-').filter(|(_, suffix)| {
        suffix.len() > 8
            && suffix.as_bytes()[8] == b'.'
            && suffix.as_bytes()[..8].iter().all(u8::is_ascii_hexdigit)
    }) else {
        // A known logical asset emitted without its required fingerprint is also invalid.
        if live
            && let Some((_, current)) = campfire_assets::manifest()
                .iter()
                .find(|(logical, _)| *logical == path)
            && *current != path
        {
            return Err(format!(
                "wrong asset digest: /assets/{path}; pipeline requires /assets/{current}"
            ));
        }
        return Ok(None);
    };
    let logical = format!("{stem}{}", &suffix[8..]);
    let Some((_, current)) = campfire_assets::manifest()
        .iter()
        .find(|(name, _)| *name == logical)
    else {
        return Ok(None);
    };
    if live && path != *current {
        return Err(format!(
            "wrong asset digest: /assets/{path}; pipeline requires /assets/{current}"
        ));
    }
    Ok(Some(format!("/assets/{logical}{}", &value[path_end..])))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pages() -> (String, String) {
        let first = campfire_assets::stylesheet_path("people");
        let second = campfire_assets::stylesheet_path("base");
        let actual = format!(
            "<link rel=\"stylesheet\" href=\"{first}\"><link rel=\"stylesheet\" href=\"{second}\">{}",
            campfire_assets::javascript_importmap_tags()
        );
        let expected = actual.replace(&first, "/assets/people-00000000.css");
        (actual, expected)
    }

    #[test]
    fn accepts_only_current_pipeline_digests_with_stale_golden_fingerprints() {
        let (actual, expected) = pages();
        assert_ne!(actual, expected);
        assert!(matching_bytes(&actual, &expected).is_ok());
    }

    #[test]
    fn rejects_dropped_reordered_and_duplicated_stylesheets() {
        let (actual, expected) = pages();
        let tags: Vec<_> = actual.split_inclusive('>').take(2).collect();
        for changed in [
            actual.replacen(tags[0], "", 1),
            actual.replacen(
                &format!("{}{}", tags[0], tags[1]),
                &format!("{}{}", tags[1], tags[0]),
                1,
            ),
            format!("{}{actual}", tags[0]),
        ] {
            assert!(matching_bytes(&changed, &expected).is_err());
        }
    }

    #[test]
    fn rejects_wrong_and_missing_digests_even_when_golden_agrees() {
        let (actual, expected) = pages();
        let path = campfire_assets::stylesheet_path("people");
        for wrong in [
            "/assets/people-00000000.css",
            "/assets/people.css",
            "/assets/people-not-a-digest.css",
        ] {
            let changed = actual.replace(&path, wrong);
            assert!(
                matching_bytes(&changed, &expected)
                    .unwrap_err()
                    .contains("asset")
            );
            if wrong != "/assets/people-not-a-digest.css" {
                assert!(matching_bytes(&changed, &changed).is_err());
            }
        }
    }

    #[test]
    fn rejects_importmap_and_non_asset_byte_changes() {
        let (actual, expected) = pages();
        for changed in [
            format!("{actual} "),
            actual.replacen("stylesheet", "alternate", 1),
            actual.replacen("\"imports\"", "\"wrong_imports\"", 1),
            actual.replacen("type=\"importmap\"", "type=\"wrong\"", 1),
            actual.replacen("/assets/", "/wrong-assets/", 1),
        ] {
            assert_ne!(changed, actual);
            assert!(matching_bytes(&changed, &expected).is_err());
        }
    }

    #[test]
    fn checks_js_importmap_digests_without_changing_json_bytes() {
        let actual = campfire_assets::javascript_importmap_tags();
        let (logical, current) = campfire_assets::manifest()
            .iter()
            .find(|(l, _)| *l == "application.js")
            .unwrap();
        let stale = current.replace(
            current
                .rsplit_once('-')
                .unwrap()
                .1
                .split('.')
                .next()
                .unwrap(),
            "00000000",
        );
        let expected = actual.replace(current, &stale);
        assert_ne!(actual, expected);
        assert!(matching_bytes(actual, &expected).is_ok());
        assert!(matching_bytes(&expected, actual).is_err());
        assert!(matching_bytes(&actual.replace(current, logical), actual).is_err());
    }
}

#[cfg(test)]
mod reviewed_mutations {
    use super::*;

    fn rejected(wrapper: &str) {
        let current = campfire_assets::stylesheet_path("people");
        let actual = wrapper.replace("URL", &current);
        let expected = wrapper.replace("URL", "/assets/people-00000000.css");
        assert_ne!(actual, expected);
        assert!(!compare(wrapper, &actual, &expected));
    }

    #[test]
    fn rejects_digest_changes_in_external_urls() {
        rejected(r#"<link href="https://cdn.example.testURL">"#);
    }
    #[test]
    fn rejects_digest_changes_in_visible_text() {
        rejected("<p>URL</p>");
    }
    #[test]
    fn rejects_digest_changes_in_query_values() {
        rejected(r#"<a href="/download?next=URL">download</a>"#);
    }
    #[test]
    fn rejects_digest_changes_in_unrelated_attributes() {
        rejected(r#"<p data-note="URL">text</p>"#);
    }
    #[test]
    fn rejects_digest_changes_in_nested_non_asset_paths() {
        rejected(r#"<a href="/downloadURL">download</a>"#);
    }
    #[test]
    fn leaves_unchanged_external_urls_byte_exact() {
        let page = r#"<link href="https://cdn.example.test/assets/people-00000000.css">"#;
        assert!(compare("external URL", page, page));
    }
}

#[cfg(test)]
mod field_boundaries {
    use super::*;

    #[test]
    fn frozen_fixture_field_is_strict_while_another_use_of_the_asset_is_live() {
        let current = campfire_assets::asset_path("icons/brands/github.svg");
        let frozen = "/assets/icons/brands/github-00000000.svg";
        let actual =
            format!(r#"<img class="fixture" src="{frozen}"><img class="live" src="{current}">"#);
        let expected = format!(
            r#"<img class="fixture" src="{frozen}"><img class="live" src="/assets/icons/brands/github-11111111.svg">"#
        );
        assert!(compare_with_frozen_fields(
            "mixed",
            &actual,
            &expected,
            &[0]
        ));
        assert!(!compare("all live", &actual, &expected));
        let changed_frozen = actual.replacen(frozen, &current, 1);
        assert!(!compare_with_frozen_fields(
            "changed frozen field",
            &changed_frozen,
            &expected,
            &[0]
        ));
        let changed_live = actual.replace(&current, "/assets/icons/brands/github-22222222.svg");
        assert!(!compare_with_frozen_fields(
            "wrong live digest",
            &changed_live,
            &expected,
            &[0]
        ));
        let fields = frozen_fixture_fields(&actual, &[frozen]);
        assert_eq!(fields, [0]);
        assert!(compare_with_frozen_fields(
            "frozen provenance",
            &actual,
            &expected,
            &fields
        ));
    }

    #[test]
    fn accepts_only_local_attribute_importmap_pwa_and_link_header_values() {
        let current = campfire_assets::stylesheet_path("people");
        for actual in [
            format!(r#"<meta content='{current}'>"#),
            format!(r#"<img src={current}>"#),
            format!(r#"{{"icons":[{{"src":"{current}"}}]}}"#),
            format!("<{current}>; rel=preload; as=style"),
            format!(r#"<script type="importmap">{{"imports":{{"people":"{current}"}}}}</script>"#),
        ] {
            let expected = actual.replace(&current, "/assets/people-00000000.css");
            assert!(compare("local field", &actual, &expected));
            assert!(!compare("wrong actual", &expected, &actual));
        }
    }

    #[test]
    fn comments_raw_text_json_keys_and_unknown_paths_remain_byte_exact() {
        let current = campfire_assets::stylesheet_path("people");
        for wrapper in [
            r#"<!-- <img src="URL"> -->"#,
            r#"<!-- <img src="URL">"#,
            r#"<script>const html = '<img src="URL">'</script>"#,
            r#"<textarea><img src="URL"></textarea>"#,
            r#"<p title='src="URL"'>text</p>"#,
            r#"<script type="importmap">{"imports":{"URL":"other"}}</script>"#,
            r#"{"caption":"URL"}"#,
        ] {
            let actual = wrapper.replace("URL", &current);
            let expected = wrapper.replace("URL", "/assets/people-00000000.css");
            assert!(!compare("ordinary bytes", &actual, &expected));
        }
        let unknown = r#"<img src="/assets/not-in-pipeline-00000000.css">"#;
        assert!(compare("unchanged non-pipeline URL", unknown, unknown));
        assert!(!compare(
            "changed non-pipeline URL",
            &unknown.replace("00000000", "11111111"),
            unknown
        ));
    }
}
