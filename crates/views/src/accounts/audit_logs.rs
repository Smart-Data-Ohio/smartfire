//! `accounts/audit_logs/show` and `AuditLogsHelper`, with prepared read-only inputs.
use crate::{
    ViewContext,
    helpers::{self as h, filters},
    layouts::Page,
};
use askama::Template;
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
pub use campfire_presentation::accounts::audit_logs::*;
