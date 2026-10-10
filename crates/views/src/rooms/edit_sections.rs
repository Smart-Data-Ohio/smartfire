//! Room settings sections. Read-only composition inputs for the owning integrations.
use crate::{ViewContext, helpers as h};
use askama::Template;
pub trait EditSectionsRendering {
    fn github(&self, _ctx: &ViewContext) -> String;
    fn inbound(&self) -> String;
    fn new_form(&self) -> h::FormWith;
    fn update_form(&self, repo: &Repository) -> h::FormWith;
    fn remove(&self, repo: &Repository) -> h::Html;
    fn event_box(&self, repo: Option<&Repository>, event: &str) -> h::Html;
    fn skip_box(&self) -> h::Html;
    fn full_name_field(&self) -> h::Html;
    fn submit(&self, label: &str, class: &str) -> h::Html;
    fn email_button(&self, rotate: bool) -> h::Html;
}
impl EditSectionsRendering for EditSections {
    fn github(&self, _ctx: &ViewContext) -> String {
        Github { settings: self }.render().unwrap()
    }
    fn inbound(&self) -> String {
        Inbound { settings: self }.render().unwrap()
    }
    fn new_form(&self) -> h::FormWith {
        h::form_with(format!("/rooms/{}/github_subscriptions", self.room_id))
            .class("flex flex-column gap margin-block-start")
    }
    fn update_form(&self, repo: &Repository) -> h::FormWith {
        h::form_with(format!(
            "/rooms/{}/github_subscriptions/{}",
            self.room_id, repo.id
        ))
        .method("patch")
        .class("flex align-center gap")
    }
    fn remove(&self, repo: &Repository) -> h::Html {
        h::button_to_form(
            &format!("/rooms/{}/github_subscriptions/{}", self.room_id, repo.id),
            h::attrs()
                .method("delete")
                .class("btn btn--negative txt-small"),
            h::attrs().data(
                "turbo_confirm",
                format!("Unsubscribe {}?", repo.full_name()),
            ),
            "Remove",
        )
    }
    fn event_box(&self, repo: Option<&Repository>, event: &str) -> h::Html {
        let checked = repo.map_or_else(
            || DEFAULT_EVENTS.contains(&event),
            |r| r.events.iter().any(|e| e == event),
        );
        h::legacy_tag(
            "input",
            h::attrs()
                .type_("checkbox")
                .name("github_repository_subscription[events][]")
                .id(format!(
                    "github_subscription_{}_{}",
                    repo.map(|r| r.id.to_string())
                        .unwrap_or_else(|| "new".into()),
                    event
                ))
                .value(event)
                .attr_opt("checked", checked.then_some(true)),
        )
    }
    fn skip_box(&self) -> h::Html {
        h::legacy_tag(
            "input",
            h::attrs()
                .type_("checkbox")
                .name("github_repository_subscription[skip_access_check]")
                .id("github_subscription_new_skip_access_check")
                .value("1"),
        )
    }
    fn full_name_field(&self) -> h::Html {
        h::legacy_tag(
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
        )
    }
    fn submit(&self, label: &str, class: &str) -> h::Html {
        h::legacy_tag(
            "input",
            h::attrs()
                .type_("submit")
                .name("commit")
                .value(label)
                .class(class)
                .data("disable_with", label),
        )
    }
    fn email_button(&self, rotate: bool) -> h::Html {
        let form = if rotate {
            h::attrs().data(
                "turbo_confirm",
                "Rotate this room's email address? The old address stops working.",
            )
        } else {
            h::attrs()
        };
        h::button_to_form(
            &format!("/rooms/{}/inbound_email_address", self.room_id),
            h::attrs().class(if rotate {
                "btn btn--negative txt-small"
            } else {
                "btn txt-small"
            }),
            form,
            if rotate {
                "Rotate address"
            } else {
                "Create email address"
            },
        )
    }
}

#[derive(Template)]
#[template(path = "rooms/calls/_github_section.html")]
struct Github<'a> {
    settings: &'a EditSections,
}
#[derive(Template)]
#[template(path = "rooms/calls/_inbound_section.html")]
struct Inbound<'a> {
    settings: &'a EditSections,
}
mod filters {
    pub use crate::helpers::filters::*;
}
pub use campfire_presentation::rooms::edit_sections::*;
