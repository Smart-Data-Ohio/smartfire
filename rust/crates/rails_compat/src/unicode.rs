//! Ruby 3.4's Unicode 15.0 casing, captured from the pinned Rails runtime.
//! Rust's lowercase applies contextual final sigma; Ruby's does not. Full folding is
//! a separate operation and must not replace `downcase` when normalizing saved strings.
use serde::Deserialize;
use std::sync::LazyLock;

#[derive(Deserialize)]
struct Maps {
    downcase: Vec<(u32, String)>,
    fold: Vec<(u32, String)>,
    upcase: Vec<(u32, String)>,
}
#[derive(Deserialize)]
struct Tables {
    maps: Maps,
    alpha_ranges: Vec<(u32, u32)>,
}
static TABLES: LazyLock<Tables> = LazyLock::new(|| {
    serde_json::from_str(include_str!("unicode_tables.json"))
        .expect("pinned Ruby Unicode tables")
});

fn map(value: &str, rows: &[(u32, String)]) -> String {
    let mut result = String::with_capacity(value.len());
    for c in value.chars() {
        if let Ok(index) = rows.binary_search_by_key(&(c as u32), |row| row.0) {
            result.push_str(&rows[index].1);
        } else {
            result.push(c);
        }
    }
    result
}
/// `String#downcase` on UTF-8, without locale options.
pub fn downcase(value: &str) -> String {
    map(value, &TABLES.maps.downcase)
}
/// `String#downcase(:fold)` / UTF-8 `String#casecmp?`, without normalization.
pub fn fold(value: &str) -> String {
    map(value, &TABLES.maps.fold)
}
/// `String#upcase` on UTF-8, without locale options.
pub fn upcase(value: &str) -> String {
    map(value, &TABLES.maps.upcase)
}

/// Ruby `[[:alpha:]]` for Active Support's humanize casing and acronym runs.
pub fn alphabetic(c: char) -> bool {
    let point = c as u32;
    let ranges = &TABLES.alpha_ranges;
    let index = ranges.partition_point(|row| row.0 <= point);
    index > 0 && point <= ranges[index - 1].1
}
