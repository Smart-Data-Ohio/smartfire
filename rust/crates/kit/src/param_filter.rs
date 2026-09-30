//! `ActiveSupport::ParameterFilter` over `config.filter_parameters`, and the request's
//! `filtered_path` (`ActionDispatch::Http::FilterParameters`): what keeps passwords, tokens and
//! codes out of the logs.
//!
//! Each filter is a string matched case-insensitively anywhere in a key (`:passw` catches
//! `password_confirmation`). One with a dot (`"message.body"`) is matched against the key's full
//! path through nested hashes (`message.body_html` too). A matching key's value becomes
//! `[FILTERED]`, whatever it holds.
//!
//! The app installs its filter once at boot ([`install`]), as Rails keeps it in the app config;
//! [`global`] is what the logging code reads.

use std::sync::OnceLock;

use regex::Regex;
use serde_json::{Map, Value};

pub const MASK: &str = "[FILTERED]";

#[derive(Debug, Clone)]
pub struct ParameterFilter {
    /// Filters without a dot, as one alternation (`(?i:passw)|(?i:email)|...`).
    keys: Option<Regex>,
    /// Filters with a dot, against the full key path.
    deep: Option<Regex>,
}

impl ParameterFilter {
    /// `ParameterFilter.new(filters)`: `precompile_filters` turns each string into
    /// `/#{Regexp.escape(filter)}/i` and joins them, keeping those with a dot apart.
    pub fn new<S: AsRef<str>>(filters: &[S]) -> Self {
        let (deep, keys): (Vec<&str>, Vec<&str>) = filters.iter().map(AsRef::as_ref).partition(|filter| filter.contains('.'));
        Self { keys: alternation(&keys), deep: alternation(&deep) }
    }

    /// The regexps' sources, as Rails reports them (`Regexp#source`).
    pub fn sources(&self) -> Vec<String> {
        [&self.keys, &self.deep].into_iter().flatten().map(|regex| regex.as_str().to_string()).collect()
    }

    /// `filter(params)` for params as JSON (hashes, arrays and strings).
    pub fn filter(&self, params: &Value) -> Value {
        match params {
            Value::Object(map) => Value::Object(self.filter_hash(map, None)),
            other => other.clone(),
        }
    }

    fn filter_hash(&self, params: &Map<String, Value>, parent: Option<&str>) -> Map<String, Value> {
        params.iter().map(|(key, value)| (key.clone(), self.value_for_key(key, value, parent))).collect()
    }

    /// `value_for_key`: a matching key masks the value; hashes recurse under the full key, and an
    /// array's elements are each treated as that key's value.
    fn value_for_key(&self, key: &str, value: &Value, parent: Option<&str>) -> Value {
        let full_key = match parent {
            Some(parent) => format!("{parent}.{key}"),
            None => key.to_string(),
        };
        if self.keys.as_ref().is_some_and(|regex| regex.is_match(key)) || self.deep.as_ref().is_some_and(|regex| regex.is_match(&full_key)) {
            return Value::String(MASK.into());
        }
        match value {
            Value::Object(map) => Value::Object(self.filter_hash(map, Some(&full_key))),
            Value::Array(values) => Value::Array(values.iter().map(|value| self.value_for_key(key, value, parent)).collect()),
            other => other.clone(),
        }
    }

    /// `filtered_query_string`: each `key=value` part (split on `&` and `;`, keeping them) with
    /// its raw, still-escaped key filtered.
    pub fn filtered_query(&self, query: &str) -> String {
        let mut out = String::with_capacity(query.len());
        let mut part = String::new();
        let flush = |part: &mut String, out: &mut String| {
            match part.split_once('=') {
                Some((key, value)) => {
                    let mut single = Map::new();
                    single.insert(key.to_string(), Value::String(value.to_string()));
                    let filtered = self.filter_hash(&single, None);
                    let value = filtered.get(key).and_then(Value::as_str).unwrap_or(value);
                    out.push_str(key);
                    out.push('=');
                    out.push_str(value);
                }
                None => out.push_str(part),
            }
            part.clear();
        };
        for c in query.chars() {
            if c == '&' || c == ';' {
                flush(&mut part, &mut out);
                out.push(c);
            } else {
                part.push(c);
            }
        }
        flush(&mut part, &mut out);
        out
    }

    /// `request.filtered_path`: the path, and the filtered query when there is one.
    pub fn filtered_path(&self, path: &str, query: Option<&str>) -> String {
        match query {
            Some(query) if !query.is_empty() => format!("{path}?{}", self.filtered_query(query)),
            _ => path.to_string(),
        }
    }
}

fn alternation(filters: &[&str]) -> Option<Regex> {
    if filters.is_empty() {
        return None;
    }
    let source = filters.iter().map(|filter| format!("(?i:{})", regex::escape(filter))).collect::<Vec<_>>().join("|");
    Some(Regex::new(&source).expect("escaped filters make a valid regex"))
}

static GLOBAL: OnceLock<ParameterFilter> = OnceLock::new();

/// Sets the app's filter (`config.filter_parameters`); the first call wins.
pub fn install(filter: ParameterFilter) {
    let _ = GLOBAL.set(filter);
}

/// The app's filter; without one installed, nothing is filtered.
pub fn global() -> &'static ParameterFilter {
    GLOBAL.get_or_init(|| ParameterFilter::new::<&str>(&[]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn filter() -> ParameterFilter {
        ParameterFilter::new(&["passw", "token", "message.body", "code"])
    }

    #[test]
    fn masks_matching_keys_anywhere_in_the_params() {
        let params = json!({
            "Password_confirmation": "x", "name": "n",
            "user": { "auth_token": "t", "profile": { "code": "c", "bio": "b" } },
            "items": [{ "token": "a" }, "plain", ["x"]],
            "message": { "body": "hi", "body_html": "<b>", "id": "1" }, "body": "top"
        });
        assert_eq!(
            filter().filter(&params),
            json!({
                "Password_confirmation": MASK, "name": "n",
                "user": { "auth_token": MASK, "profile": { "code": MASK, "bio": "b" } },
                "items": [{ "token": MASK }, "plain", ["x"]],
                "message": { "body": MASK, "body_html": MASK, "id": "1" }, "body": "top"
            })
        );
    }

    #[test]
    fn filters_query_strings_by_raw_key() {
        let filter = filter();
        assert_eq!(filter.filtered_path("/x", None), "/x");
        assert_eq!(filter.filtered_path("/x", Some("")), "/x");
        assert_eq!(filter.filtered_path("/x", Some("token=a&b=1;code=2&flag&=v")), "/x?token=[FILTERED]&b=1;code=[FILTERED]&flag&=v");
        assert_eq!(filter.filtered_path("/x", Some("%70assword=raw&message[body]=m")), "/x?%70assword=raw&message[body]=m");
    }

    #[test]
    fn keeps_rails_regexp_sources() {
        assert_eq!(filter().sources(), ["(?i:passw)|(?i:token)|(?i:code)", "(?i:message\\.body)"]);
    }
}
