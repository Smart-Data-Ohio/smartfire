//! The Rails board automation settings form; database facts are supplied by the controller.
use crate::helpers::filters;
use crate::{ViewContext, helpers as h, layouts::Page};
use askama::Template;
#[derive(Template)]
#[template(path="rooms/boards/automations.html",blocks=["head","content"])]
pub struct Show<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub settings: &'a Settings,
}
impl Page for Show<'_> {
    fn page_title(&self) -> Option<String> {
        Some(format!("Automations for {}", self.settings.room_name))
    }
}
impl Show<'_> {
    fn base(&self) -> String {
        format!("/rooms/boards/{}/automations", self.settings.room_id)
    }
    fn tag_form(&self) -> h::FormWith {
        h::form_with(format!("{}/tag_assignments", self.base())).class("board-post__form")
    }
    fn sla_form(&self) -> h::FormWith {
        h::form_with(format!("{}/sla_rules", self.base())).method("patch")
    }
    fn remove(&self, tag: &TagRule) -> h::Html {
        h::button_to(
            &format!("{}/tag_assignments/{}", self.base(), tag.id),
            h::attrs()
                .method("delete")
                .class("btn")
                .aria("label", format!("Remove auto-assign rule for {}", tag.tag)),
            "Remove",
        )
    }
    fn statuses(&self) -> [(&str, &str); 3] {
        [
            ("planned", "Planned"),
            ("in_progress", "In progress"),
            ("blocked", "Blocked"),
        ]
    }
    fn minutes(&self, form: &h::FormWith, status: &str, escalation: bool) -> h::Html {
        let field = if escalation {
            "escalate_after_minutes"
        } else {
            "nudge_after_minutes"
        };
        let value = self.settings.rules.get(status).and_then(|v| {
            if escalation {
                v.1.as_deref()
            } else {
                v.0.as_deref()
            }
        });
        form.number_field(
            &format!("sla_rules[{status}][{field}]"),
            None,
            h::attrs()
                .attr_opt("value", value)
                .attr("min", "1")
                .attr("max", "43200")
                .aria(
                    "label",
                    format!(
                        "{status} {} minutes",
                        if escalation { "escalation" } else { "nudge" }
                    ),
                ),
        )
    }
}
pub use campfire_presentation::rooms::board_automations::*;
