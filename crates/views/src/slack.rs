//! Slack pages: plain presentation data, independent of the importer and database.
use crate::{
    ViewContext,
    helpers::{self as h, filters},
    layouts::Page,
};
use askama::Template;
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
pub trait RunDataRendering {
    fn sample_html(&self, index: &usize) -> h::Html;
}
impl RunDataRendering for RunData {
    fn sample_html(&self, index: &usize) -> h::Html {
        h::raw(self.sample_htmls.get(*index).cloned().unwrap_or_default())
    }
}

fn checkbox(c: &serde_json::Value) -> h::Html {
    let id = text(&c["id"]);
    let name = text(&c["name"]);
    h::legacy_tag(
        "input",
        h::attrs()
            .type_("checkbox")
            .name("conversation_ids[]")
            .id(format!("conversation_{id}"))
            .value(id)
            .data("check-all-target", "checkbox")
            .attr("aria-label", format!("Import {name}"))
            .attr("checked", true),
    )
}
fn target_select(c: &serde_json::Value, rooms: &[RoomTarget]) -> h::Html {
    let id = text(&c["id"]);
    let selected = target_value(c, rooms);
    let opts = std::iter::once(("New room".to_owned(), "new".to_owned()))
        .chain(rooms.iter().map(|r| (r.name.clone(), r.id.to_string())))
        .chain(std::iter::once(("Skip".into(), "skip".into())))
        .map(|(label, value)| {
            h::content_tag(
                "option",
                h::attrs()
                    .attr_opt("selected", (selected == value).then_some("selected"))
                    .value(value),
                &h::escape(&label),
            )
            .0
        })
        .collect::<Vec<_>>()
        .join("\n");
    h::content_tag(
        "select",
        h::attrs()
            .name(format!("room_targets[{id}]"))
            .id(format!("room_targets_{id}"))
            .attr("aria-label", format!("Target for {}", text(&c["name"])))
            .attr("aria-describedby", "plan-target-note"),
        &opts,
    )
}
#[derive(Template)]
#[template(path="accounts/slack_import_runs/index.html",blocks=["head","content","nav"])]
pub struct RunList<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub runs: &'a [RunData],
}
impl Page for RunList<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Import runs".into())
    }
}
#[derive(Template)]
#[template(path="slack/import_runs/show.html",blocks=["head","content","nav"])]
pub struct RunPage<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub data: &'a RunData,
    pub admin: bool,
}
impl Page for RunPage<'_> {
    fn page_title(&self) -> Option<String> {
        Some(self.data.title())
    }
}
#[derive(Template)]
#[template(path="accounts/slack_import_runs/plan.html",blocks=["head","content","nav"])]
pub struct Plan<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub data: &'a RunData,
    pub rooms: &'a [RoomTarget],
    pub oldest: &'a str,
}
impl Page for Plan<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Import plan".into())
    }
}
#[derive(Template)]
#[template(path="slack/imports/index.html",blocks=["head","content","nav"])]
pub struct PersonalIndex<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub data: &'a SetupData,
    pub runs: &'a [RunData],
}
// The personal controller supplies the scoped current user; detached Rails renderers
// have no scoped route defaults and retain the unscoped profile path.
pub fn profile_path(ctx: &ViewContext) -> &'static str {
    if ctx.current_user.is_some() {
        "/users/me/profile"
    } else {
        "/users/profile"
    }
}
impl Page for PersonalIndex<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Import from Slack".into())
    }
}
#[derive(Template)]
#[template(path = "slack/import_runs/status.html")]
struct Status<'a> {
    data: &'a RunData,
}
pub fn status(data: &RunData) -> String {
    Status { data }.render().expect("Slack status template")
}
fn submit_confirm(label: &str, class: &str, confirm: &str) -> h::Html {
    h::legacy_tag(
        "input",
        h::attrs()
            .type_("submit")
            .name("commit")
            .value(label)
            .class(class)
            .data("turbo-confirm", confirm)
            .data("disable-with", label),
    )
}
fn preview_button() -> h::Html {
    h::button_to_form_params(
        "/slack/imports",
        h::attrs().class("btn btn--reversed"),
        h::attrs(),
        "Start preview",
        &[("mode", "dry_run")],
    )
}
pub use campfire_presentation::slack::*;
