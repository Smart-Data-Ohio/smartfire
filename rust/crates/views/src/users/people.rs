//! Plain display models for the people directory and profile-card popover.
use super::UserSummary;
use crate::{
    ViewContext,
    helpers::{self as h, filters},
    layouts::Page,
};
use askama::Template;

#[derive(Clone, Debug)]
pub struct Person {
    pub user: UserSummary,
    pub online: bool,
    pub starred: bool,
    pub agent: bool,
    pub agent_owner: Option<String>,
    pub presence: String,
    pub custom_status: Option<String>,
    pub mention: Option<String>,
}

impl Person {
    pub fn trigger(&self) -> h::Attrs {
        h::profile_card_trigger(self.user.id, false)
    }
    pub fn label(&self) -> &str {
        match self.presence.as_str() {
            "online" => "Online",
            "idle" => "Idle",
            "dnd" => "Do not disturb",
            "agent" => "Agent",
            _ => "Offline",
        }
    }
    pub fn status_subscription(&self, ctx: &ViewContext) -> h::Html {
        h::turbo_stream_from_streamables(ctx, &[&h::gid_param("User", self.user.id), "status"])
    }
    pub fn status_badge(&self) -> askama::Result<h::Html> {
        Ok(h::raw(
            super::statuses::StatusBadge {
                presence: &self.presence,
                status_text: self.custom_status.as_deref(),
            }
            .render()?,
        ))
    }
}

#[derive(Template)]
#[template(path = "users/cards/show.html")]
pub struct Card<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub person: Person,
}

impl Card<'_> {
    fn action_button(&self, call: bool) -> h::Html {
        let label = if call { "Start call" } else { "Message" };
        let button = h::button_to_form(
            &h::routes::rooms_directs(),
            h::attrs().class(if call {
                "btn full-width"
            } else {
                "btn btn--reversed full-width"
            }),
            h::attrs().data("turbo_frame", "_top"),
            label,
        );
        let mut fields = String::new();
        if call {
            fields.push_str(
                &h::legacy_tag(
                    "input",
                    h::attrs().type_("hidden").name("start_huddle").value("1"),
                )
                .0,
            );
        }
        fields.push_str(
            &h::legacy_tag(
                "input",
                h::attrs()
                    .type_("hidden")
                    .name("user_ids[]")
                    .value(self.person.user.id.to_string()),
            )
            .0,
        );
        h::raw(button.0.strip_suffix("</form>").unwrap().to_owned() + &fields + "</form>")
    }
    fn star_button(&self) -> h::Html {
        star_button(self.person.user.id, self.person.starred)
    }
}

fn star_button(user_id: i64, starred: bool) -> h::Html {
    h::button_to_form(
        &h::routes::user_star(user_id),
        h::attrs()
            .attr("method", if starred { "delete" } else { "post" })
            .class("btn full-width"),
        h::attrs().data("action", "turbo:submit-end->profile-card#starToggled"),
        if starred { "★ Unstar" } else { "☆ Star" },
    )
}

#[derive(Template)]
#[template(path = "users/stars/_toggle.html")]
pub struct StarToggle {
    pub user_id: i64,
    pub starred: bool,
}

impl StarToggle {
    fn button(&self) -> h::Html {
        star_button(self.user_id, self.starred)
    }
}

/// The controller response is session-bound and is never a cable broadcast.
pub fn star_stream(user_id: i64, starred: bool) -> askama::Result<String> {
    let html = StarToggle { user_id, starred }.render()?;
    Ok(h::content_tag(
        "turbo-stream",
        &h::attrs()
            .attr("action", "replace")
            .attr("target", format!("star_user_{user_id}")),
        &h::content_tag("template", &h::attrs(), &html).0,
    )
    .0)
}

#[derive(Template)]
#[template(path="users/index.html",blocks=["head","content","nav","sidebar"])]
pub struct Directory<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub people: Vec<Person>,
}

impl Directory<'_> {
    fn multi_select(&self) -> askama::Result<h::Html> {
        Ok(h::raw(
            crate::shared::MultiSelectBar { exit_button: false }.render()?,
        ))
    }
}
impl Page for Directory<'_> {
    fn page_title(&self) -> Option<String> {
        Some("People".into())
    }
    fn body_class(&self) -> Option<&str> {
        Some("sidebar workspace-page")
    }
    fn has_sidebar(&self) -> bool {
        true
    }
}
