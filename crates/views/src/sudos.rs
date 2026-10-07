//! `app/views/sudos` and `SudosHelper`.
use crate::{
    ViewContext,
    helpers::{self as h, filters},
    layouts::Page,
};
use askama::Template;
use serde_json::Value;

#[derive(Template)]
#[template(path = "sudos/new.html", blocks = ["head", "content"])]
pub struct New<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub password: bool,
    pub totp: bool,
    pub google: bool,
}
impl Page for New<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Confirm it's you".into())
    }
}
impl New<'_> {
    fn totp_options(&self) -> h::Attrs {
        h::attrs()
            .required(true)
            .class("input auth-code")
            .attr("autofocus", !self.password)
            .autocomplete("one-time-code")
            .attr("inputmode", "numeric")
            .maxlength(10)
            .placeholder("Enter your authenticator code")
            .attr("aria-label", "Authenticator code")
    }
}

#[derive(Template)]
#[template(path = "sudos/continue.html", blocks = ["head", "content"])]
pub struct Continue<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub method: String,
    pub path: String,
    pub params: Value,
}
impl Page for Continue<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Continuing".into())
    }
}

/// `sudo_replay_fields`: reject malformed names and non-scalars again at render time.
pub fn replay_fields(params: &Value, prefix: Option<&str>) -> h::Html {
    let Some(params) = params.as_object() else {
        return h::empty();
    };
    let mut output = String::new();
    for (key, value) in params {
        let name = prefix.map_or_else(|| key.clone(), |prefix| format!("{prefix}[{key}]"));
        if name.is_empty()
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-' | '[' | ']'))
        {
            continue;
        }
        match value {
            Value::Object(_) => output.push_str(&replay_fields(value, Some(&name)).0),
            Value::Array(values) => {
                for value in values {
                    if let Some(field) = scalar_field(&format!("{name}[]"), value) {
                        output.push_str(&field.0);
                    }
                }
            }
            _ => {
                if let Some(field) = scalar_field(&name, value) {
                    output.push_str(&field.0);
                }
            }
        }
    }
    h::raw(output)
}

fn scalar_field(name: &str, value: &Value) -> Option<h::Html> {
    let value = match value {
        Value::Object(_) | Value::Array(_) => return None,
        Value::Null => None,
        Value::String(value) => Some(value.clone()),
        Value::Number(number) => Some(rails_compat::numbers::number_to_s(number)),
        other => Some(other.to_string()),
    };
    Some(h::hidden_field_tag(name, value.as_deref(), h::attrs()))
}
