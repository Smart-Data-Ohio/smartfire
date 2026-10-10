//! `app/views/rooms/github_subscriptions/_section.html.erb`.
use crate::helpers as h;
fn checkbox(name: &str, id: &str, value: &str, checked: bool) -> String {
    h::legacy_tag(
        "input",
        h::attrs()
            .type_("checkbox")
            .name(name)
            .id(id)
            .value(value)
            .attr_opt("checked", checked.then_some("checked")),
    )
    .0
}
fn submit(label: &str, class: &str) -> String {
    h::legacy_tag(
        "input",
        h::attrs()
            .type_("submit")
            .name("commit")
            .value(label)
            .class(class)
            .data("disable-with", label),
    )
    .0
}
fn event_fields(prefix: &str, selected: impl Fn(&str, bool) -> bool, indent: usize) -> String {
    let pad = " ".repeat(indent);
    let mut html = String::new();
    for (key, label, default) in EVENTS {
        html.push_str(&format!("{pad}<label class=\"flex align-center gap txt-small\">\n{pad}  {}\n{pad}  {label}\n{pad}</label>\n",checkbox("github_repository_subscription[events][]",&format!("github_subscription_{prefix}_{key}"),key,selected(key,default))));
    }
    html
}
pub trait SectionRendering {
    fn render(&self) -> h::Html;
}
impl SectionRendering for Section {
    fn render(&self) -> h::Html {
        if !self.can_administer {
            return h::Safe(String::new());
        }
        let mut html = String::from(
            "  <section class=\"panel txt-align-center\" id=\"github-subscriptions\">\n    <div class=\"pad-inline-double center\">\n      <h2 class=\"margin-none\">GitHub</h2>\n      <p class=\"txt-small margin-none-block-start\">\n        Subscribed repositories post pull-request events to this room as the GitHub bot.\n        Your linked GitHub account must be able to read the repository, and titles from private repositories become visible to the whole room.\n      </p>\n    </div>\n\n    <div class=\"pad-inline pad-block-start txt-align-start\">\n",
        );
        if self.subscriptions.is_empty() {
            html.push_str("        <p class=\"txt-small\">No repositories subscribed yet.</p>\n");
        } else {
            html.push_str("        <menu class=\"flex flex-column gap margin-none pad\">\n");
            for s in &self.subscriptions {
                let url = format!("/rooms/{}/github_subscriptions/{}", self.room_id, s.id);
                let remove = h::button_to_form(
                    &url,
                    h::attrs()
                        .attr("method", "delete")
                        .class("btn btn--negative txt-small"),
                    h::attrs().data("turbo-confirm", format!("Unsubscribe {}?", s.full_name)),
                    "Remove",
                );
                html.push_str(&format!("            <li class=\"flex flex-column gap\">\n              <div class=\"flex align-center gap\">\n                <strong>{}</strong>\n                {}\n              </div>\n\n              ",h::escape(&s.full_name),remove.0));
                let mut fields = format!(
                    "\n                {}\n",
                    h::hidden_field_tag(
                        "github_repository_subscription[events][]",
                        Some(""),
                        h::attrs()
                    )
                    .0
                );
                fields.push_str(&event_fields(
                    &s.id.to_string(),
                    |key, _| s.events.iter().any(|s| s == key),
                    18,
                ));
                fields.push_str(&format!(
                    "                {}\n",
                    submit("Save", "btn txt-small")
                ));
                html.push_str(
                    &h::form_with(url)
                        .method("patch")
                        .class("flex align-center gap")
                        .wrap(&fields)
                        .0,
                );
                html.push_str("            </li>\n");
            }
            html.push_str("        </menu>\n");
        }
        html.push_str("    </div>\n\n    <div class=\"pad-inline pad-block txt-align-start\">\n      <h3 class=\"txt-large margin-none\">Subscribe a repository</h3>\n\n      ");
        let input = h::legacy_tag(
            "input",
            h::attrs()
                .type_("text")
                .name("github_repository_subscription[full_name]")
                .id("github_repository_subscription_full_name")
                .attr("placeholder", "owner/repo")
                .attr("required", true)
                .class("input")
                .attr("autocomplete", "off")
                .data("1p-ignore", true),
        );
        let mut fields = format!(
            "\n        <div class=\"flex align-center gap\">\n          <label class=\"flex align-center gap flex-item-grow txt-large input input--actor\">\n            {}\n          </label>\n        </div>\n\n        <div class=\"flex align-center gap\">\n          {}\n",
            input.0,
            h::hidden_field_tag(
                "github_repository_subscription[events][]",
                Some(""),
                h::attrs()
            )
            .0
        );
        fields.push_str(&event_fields("new", |_, default| default, 12));
        fields.push_str("        </div>\n\n");
        if self.administrator {
            fields.push_str(&format!("          <label class=\"flex align-center gap txt-small\">\n            {}\n            Subscribe without verifying my GitHub access (private pull-request titles stay hidden)\n          </label>\n",checkbox("github_repository_subscription[skip_access_check]","github_subscription_new_skip_access_check","1",false)));
        }
        fields.push_str(&format!(
            "\n        <div>\n          {}\n        </div>\n",
            submit("Subscribe", "btn btn--reversed")
        ));
        html.push_str(
            &h::form_with(format!("/rooms/{}/github_subscriptions", self.room_id))
                .class("flex flex-column gap margin-block-start")
                .wrap(&fields)
                .0,
        );
        html.push_str("    </div>\n  </section>\n");
        h::Safe(html)
    }
}

pub use campfire_presentation::github::subscriptions::*;
