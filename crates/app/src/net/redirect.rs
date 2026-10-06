//! `URI::Generic#merge` as used by our OpenGraph and image-proxy redirect checks.
//! The path rules come from uri 1.1.1 in the pinned reference image.
use campfire_richtext::uri::{self, Uri};

pub fn resolve(base: &Uri, location: Option<&str>) -> Option<Uri> {
    let location = location.filter(|s| !s.trim().is_empty())?;
    let relative = uri::parse(location).ok()?;
    let merged = if relative.scheme.is_some() {
        relative
    } else if location.starts_with("//") {
        uri::parse(&format!("{}:{location}", base.scheme.as_deref()?)).ok()?
    } else {
        let mut merged = base.clone();
        let path = relative.path.as_deref().unwrap_or("");
        if path.is_empty() && relative.query.is_none() {
            if relative.fragment.is_some() {
                merged.fragment = relative.fragment;
            }
            return merged.is_http().then_some(merged);
        }
        merged.path = Some(merge_path(base.path.as_deref().unwrap_or(""), path));
        merged.query = relative.query;
        merged.fragment = relative.fragment;
        merged
    };
    merged.is_http().then_some(merged)
}

fn merge_path(base: &str, relative: &str) -> String {
    let mut base: Vec<&str> = if base.is_empty() { Vec::new() } else { base.split('/').collect() };
    let mut relative: Vec<&str> = if relative.is_empty() { Vec::new() } else { relative.split('/').collect() };
    if base.last() == Some(&"..") {
        base.push("");
    }
    while let Some(i) = base.iter().position(|s| *s == "..") {
        // Ruby Array#slice!(i - 1, 2), including its negative-index behavior.
        let start = if i == 0 { base.len() - 1 } else { i - 1 };
        base.drain(start..(start + 2).min(base.len()));
    }
    if relative.first() == Some(&"") {
        base.clear();
        relative.remove(0);
    }
    if matches!(relative.last(), Some(&"." | &"..")) {
        relative.push("");
    }
    relative.retain(|s| *s != ".");
    let mut normalized = Vec::new();
    for segment in relative {
        if segment == ".." && !normalized.is_empty() && normalized.last() != Some(&"..") {
            normalized.pop();
        } else {
            normalized.push(segment);
        }
    }
    let mut trailer = !normalized.is_empty();
    if base.is_empty() {
        base.push("");
    } else if trailer {
        base.pop();
    }
    let mut rest = normalized.into_iter();
    while let Some(segment) = rest.next() {
        if segment == ".." {
            if base.len() > 1 {
                base.pop();
            }
        } else {
            base.push(segment);
            base.extend(rest);
            trailer = false;
            break;
        }
    }
    if trailer {
        base.push("");
    }
    base.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ws15e_redirects_match_rails_uri_merge() {
        let vectors: serde_json::Value = serde_json::from_str(include_str!("../../../../vectors/ws15e_embeds.json")).unwrap();
        for case in vectors["redirects"].as_array().unwrap() {
            let base = uri::parse(case["base"].as_str().unwrap()).unwrap();
            let actual = resolve(&base, case["location"].as_str()).map(|u| u.to_s());
            assert_eq!(actual.as_deref(), case["result"]["url"].as_str(), "{case}");
        }
    }
}
