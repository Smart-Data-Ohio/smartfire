//! Slack pages: plain presentation data, independent of the importer and database.
use crate::{
    ViewContext,
    helpers::{self as h, filters},
    layouts::Page,
};
use askama::Template;
#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct RunSummary {
    pub id: i64,
    pub kind: String,
    pub mode: String,
    pub status: String,
}
impl RunSummary {
    fn mode_label(&self) -> String {
        self.mode.replace('_', " ")
    }
}
#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct SetupData {
    pub client_id: Option<String>,
    pub configured: bool,
    pub configured_by: Option<String>,
    pub team_name: Option<String>,
    pub team_known: bool,
    pub connection_exists: bool,
    pub connected: bool,
    pub disconnected_reason: Option<String>,
    pub active_run: Option<RunSummary>,
    pub errors: Vec<String>,
    pub manifest: String,
}
impl SetupData {
    fn secret_placeholder(&self) -> &str {
        if self.configured {
            "Saved — paste a new secret to replace it"
        } else {
            "xoxp-…"
        }
    }
    fn submit_label(&self) -> &str {
        if self.configured {
            "Save new credentials"
        } else {
            "Save credentials"
        }
    }
}
#[derive(Template)]
#[template(path="accounts/slack_imports/show.html",blocks=["head","content","nav"])]
pub struct Setup<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub data: &'a SetupData,
}
impl Page for Setup<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Slack import".into())
    }
}
fn submit(label: &str, class: &str) -> h::Html {
    h::legacy_tag(
        "input",
        h::attrs()
            .type_("submit")
            .name("commit")
            .value(label)
            .class(class)
            .data("disable-with", label),
    )
}
