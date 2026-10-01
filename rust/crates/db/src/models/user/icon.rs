//! User's icon normalization/validation from app/models/user.rb and Icons.
use crate::{Connection, Errors, Result};
use campfire_richtext::markdown::{IconCatalog, IconResolver};
use serde_json::Value;
use std::sync::LazyLock;

static BRANDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../../data/icon-names.json"))
        .expect("pinned Rails brand icon names")
});
fn strip(s: &str) -> &str {
    s.trim_matches([' ', '\t', '\n', '\r', '\u{b}', '\u{c}', '\0'])
}
/// Rails strips ASCII whitespace, then leading/trailing colons, then whitespace.
/// Ruby downcase maps each character independently (no final-sigma context).
pub fn normalize_name(name: Option<&str>) -> Option<String> {
    let name = strip(strip(name.unwrap_or_default()).trim_matches(':'));
    let name: String = name.chars().flat_map(char::to_lowercase).collect();
    (!campfire_richtext::ruby::is_blank(&name)).then_some(name)
}
/// ActiveRecord string type casting precedes the registered normalizer.
pub fn normalize_input(input: &Value) -> Option<String> {
    let text = match input {
        Value::Null => return None,
        Value::Bool(true) => "t".into(),
        Value::Bool(false) => "f".into(),
        Value::String(s) => s.clone(),
        value => value.to_string(),
    };
    normalize_name(Some(&text))
}
pub fn validate(conn: &Connection, name: Option<&str>) -> Result<Errors> {
    let mut errors = Errors::default();
    if let Some(name) = name.filter(|s| !campfire_richtext::ruby::is_blank(s)) {
        let known = BRANDS.iter().any(|s| s == name)
            || (name.is_ascii() && IconCatalog::default().find(name).is_some())
            || crate::sql::exists(conn, "SELECT 1 FROM workspace_icons WHERE name=?", [name])?;
        if !known {
            errors.add("icon_name", "is not a known icon");
        }
    }
    Ok(errors)
}
