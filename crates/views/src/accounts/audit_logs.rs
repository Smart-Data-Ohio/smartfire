//! `accounts/audit_logs/show` and `AuditLogsHelper`, with prepared read-only inputs.
use crate::{
    ViewContext,
    helpers::{self as h, filters},
    layouts::Page,
};
use askama::Template;

#[derive(Clone, Debug, Default)]
pub struct Filters {
    pub actor: Option<String>,
    pub action: Option<String>,
    pub target_type: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}
impl Filters {
    pub fn export_path(&self) -> String {
        self.path(Some(("format", "csv")))
    }
    pub fn next_path(&self, next: &str) -> String {
        self.path(Some(("page", next)))
    }
    pub fn path(&self, extra: Option<(&str, &str)>) -> String {
        let mut query = Vec::new();
        for (key, value) in [
            ("actor", &self.actor),
            ("audit_action", &self.action),
            ("target_type", &self.target_type),
            ("from", &self.from),
            ("to", &self.to),
        ] {
            if let Some(value) = value {
                query.push((key, value.as_str()));
            }
        }
        if let Some(extra) = extra {
            query.push(extra);
        }
        let mut pairs: Vec<_> = query
            .iter()
            .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
            .collect();
        pairs.sort();
        let path = if extra.is_some_and(|(k, _)| k == "format") {
            format!("{}.csv", h::routes::account_audit_log())
        } else {
            h::routes::account_audit_log()
        };
        pairs.retain(|p| !p.starts_with("format="));
        if pairs.is_empty() {
            path
        } else {
            format!("{path}?{}", pairs.join("&"))
        }
    }
}
fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'-' | b'.' | b'*' => {
                (b as char).to_string()
            }
            b' ' => "+".into(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}
#[derive(Clone, Debug)]
pub struct Entry {
    pub created_at: jiff::Timestamp,
    pub action: String,
    pub actor: String,
    pub target: String,
    pub changes: String,
    pub ip_address: String,
}
#[derive(Template)]
#[template(path="accounts/audit_logs/show.html", blocks=["head","nav","content"])]
pub struct Show<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub filters: Filters,
    pub entries: Vec<Entry>,
    pub actions: Vec<String>,
    pub target_types: Vec<String>,
    pub export_truncated: bool,
    pub export_limit: String,
    pub first_page: bool,
    pub next_page: Option<String>,
}
impl Page for Show<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Audit log".into())
    }
}
impl Show<'_> {
    fn select(
        &self,
        name: &str,
        first: &str,
        choices: &[String],
        selected: Option<&str>,
    ) -> h::Html {
        let mut options = vec![h::content_tag("option", h::attrs().value(""), first).to_string()];
        for choice in choices {
            let mut attrs = h::attrs();
            if Some(choice.as_str()) == selected {
                attrs = attrs.attr("selected", "selected");
            }
            options.push(
                h::content_tag("option", attrs.value(choice), &h::escape(choice)).to_string(),
            );
        }
        h::content_tag(
            "select",
            h::attrs().class("input").name(name).id(name),
            &options.join("\n"),
        )
    }
    fn time(&self, entry: &Entry) -> h::Html {
        h::local_datetime_tag(
            &self.ctx.time_zone,
            entry.created_at,
            "datetime",
            h::attrs(),
            &self.ctx.time_zone.to_fs(entry.created_at, "short"),
        )
    }
}

pub fn changes_summary(details: &serde_json::Value) -> String {
    use serde_json::Value;
    fn value(v: &Value) -> String {
        match v {
            Value::Null => "∅".into(),
            Value::String(s) => truncate(s),
            Value::Array(_) | Value::Object(_) => truncate(&h::to_rails_json(v)),
            _ => v.to_string(),
        }
    }
    fn truncate(s: &str) -> String {
        if s.chars().count() > 80 {
            format!("{}...", s.chars().take(77).collect::<String>())
        } else {
            s.into()
        }
    }
    let Some(details) = details.as_object().filter(|d| !d.is_empty()) else {
        return "—".into();
    };
    details
        .iter()
        .map(|(k, v)| {
            if let Some(pair) = v
                .as_object()
                .filter(|p| p.len() == 2 && p.contains_key("before") && p.contains_key("after"))
            {
                format!(
                    "{k}: {} → {}",
                    value(&pair["before"]),
                    value(&pair["after"])
                )
            } else {
                format!("{k}: {}", value(v))
            }
        })
        .collect::<Vec<_>>()
        .join("; ")
}
