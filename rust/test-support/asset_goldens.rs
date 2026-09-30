//! Page parity permits only stale fingerprint bytes in a Rails golden. Actual URLs must
//! match the live Rust/Propshaft manifest; every other byte (including importmap JSON,
//! tag order, duplicates, query strings and whitespace) remains part of the comparison.

pub fn compare(name: &str, actual: &str, expected: &str) -> bool {
    match matching_bytes(actual, expected) {
        Ok(()) => true,
        Err(error) => {
            eprintln!("{name}: {error}");
            false
        }
    }
}

fn matching_bytes(actual: &str, expected: &str) -> Result<(), String> {
    let actual = logical_references(actual, true)?;
    let expected = logical_references(expected, false)?;
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

fn logical_references(text: &str, live: bool) -> Result<String, String> {
    let mut output = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("/assets/") {
        let tail = &rest[start + "/assets/".len()..];
        let end = tail
            .find(|c: char| !c.is_ascii_alphanumeric() && !"/_-.@+".contains(c))
            .unwrap_or(tail.len());
        let path = &tail[..end];
        // A bare prefix in service-worker code/comments is not an asset URL.
        if path.is_empty() {
            output.push_str(&rest[..start + "/assets/".len()]);
            rest = tail;
            continue;
        }
        // Propshaft inserts eight SHA1 hex characters before the extension (also .js.map).
        // Already-digested vendor names are logical names themselves and stay byte-exact.
        let logical = if campfire_assets::manifest()
            .iter()
            .any(|(l, d)| *l == path && *d == path)
        {
            path.to_string()
        } else if let Some((stem, suffix)) = path.rsplit_once('-')
            && suffix.len() > 8
            && suffix.as_bytes()[8] == b'.'
            && suffix.as_bytes()[..8].iter().all(u8::is_ascii_hexdigit)
        {
            format!("{stem}{}", &suffix[8..])
        } else {
            path.to_string()
        };
        let current = campfire_assets::manifest()
            .iter()
            .find_map(|(l, d)| (*l == logical).then_some(*d))
            .ok_or_else(|| format!("unknown asset reference /assets/{path}"))?;
        if live && path != current {
            return Err(format!(
                "wrong asset digest: /assets/{path}; pipeline requires /assets/{current}"
            ));
        }
        output.push_str(&rest[..start]);
        output.push_str("/assets/");
        output.push_str(&logical);
        rest = &tail[end..];
    }
    output.push_str(rest);
    Ok(output)
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
            assert!(matching_bytes(&changed, &changed).is_err());
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
