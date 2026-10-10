//! Event form facts and fields, matching rooms/events/_form.html.erb.
use crate::{
    ViewContext,
    helpers::{self as h, filters},
    layouts::Page,
};
use askama::Template;
pub trait FormViewRendering {
    fn form(&self) -> h::FormWith;
    fn error_wrap(&self, field: &str, value: h::Html) -> h::Html;
    fn label(&self, field: &str, text: &str) -> h::Html;
    fn title_field(&self) -> h::Html;
    fn description_field(&self) -> h::Html;
    fn time_field(&self, field: &str) -> h::Html;
    fn until_field(&self) -> h::Html;
    fn meet_checkbox(&self) -> h::Html;
    fn zone_field(&self) -> h::Html;
    fn submit(&self) -> h::Html;
    fn option(label: &str, value: &str, selected: bool) -> String;
    fn repeat_select(&self) -> h::Html;
    fn venue_select(&self) -> h::Html;

    fn render_form(&self) -> h::Html;
}

impl FormViewRendering for FormView {
    fn form(&self) -> h::FormWith {
        h::form_with(self.path())
            .model("event")
            .method(if self.id.is_some() { "patch" } else { "post" })
            .class("flex flex-column gap")
            .data("controller", "event-time-zone")
    }
    fn error_wrap(&self, field: &str, value: h::Html) -> h::Html {
        if self.error_fields.iter().any(|f| f == field) {
            h::raw(format!(
                "<div class=\"field_with_errors\">{}</div>",
                value.0
            ))
        } else {
            value
        }
    }
    fn label(&self, field: &str, text: &str) -> h::Html {
        self.error_wrap(field, self.form().label(field, text, h::attrs()))
    }
    fn title_field(&self) -> h::Html {
        self.error_wrap(
            "title",
            self.form().text_field(
                "title",
                self.title_value.as_deref(),
                h::attrs()
                    .class("input")
                    .attr("required", true)
                    .attr("autofocus", true)
                    .attr("autocomplete", "off")
                    .attr("maxlength", 255),
            ),
        )
    }
    fn description_field(&self) -> h::Html {
        self.error_wrap(
            "description",
            self.form().text_area(
                "description",
                self.description.as_deref(),
                h::attrs().class("input").attr("rows", 4),
            ),
        )
    }
    fn time_field(&self, field: &str) -> h::Html {
        let value = if field == "starts_at" {
            self.starts_at.as_deref()
        } else {
            self.ends_at.as_deref()
        };
        let mut options = h::attrs().class("input");
        if field == "starts_at" {
            options = options.attr("required", true);
        }
        options = options.attr_opt("value", value).type_("datetime-local");
        self.error_wrap(field, self.form().text_field(field, None, options))
    }
    fn until_field(&self) -> h::Html {
        self.error_wrap(
            "recurrence_until",
            self.form().text_field(
                "recurrence_until",
                self.recurrence_until.as_deref(),
                h::attrs()
                    .class("input")
                    .attr_opt("value", self.recurrence_until.as_deref())
                    .type_("date"),
            ),
        )
    }
    fn meet_checkbox(&self) -> h::Html {
        self.form().check_box(
            "meet_link_requested",
            h::attrs(),
            "1",
            "0",
            if self.meet_link_requested { "1" } else { "0" },
        )
    }
    fn zone_field(&self) -> h::Html {
        self.form().hidden_field(
            "time_zone",
            Some(&self.time_zone),
            h::attrs()
                .value(&self.time_zone)
                .data("event_time_zone_target", "field"),
        )
    }
    fn submit(&self) -> h::Html {
        let label = if self.id.is_some() {
            "Save changes"
        } else {
            "Schedule event"
        };
        h::legacy_tag(
            "input",
            h::attrs()
                .type_("submit")
                .name("commit")
                .value(label)
                .class("btn btn--primary")
                .data("disable_with", label),
        )
    }
    fn option(label: &str, value: &str, selected: bool) -> String {
        let mut a = h::attrs();
        if selected {
            a = a.attr("selected", "selected");
        }
        a = a.value(value);
        h::content_tag("option", a, &h::escape(label)).0
    }
    fn repeat_select(&self) -> h::Html {
        let mut options = Vec::new();
        if self.id.is_none() {
            options.push(Self::option(
                "Does not repeat",
                "",
                self.recurrence_rule.as_deref().unwrap_or("").is_empty(),
            ));
        }
        for (label, value) in [
            ("Daily", "daily"),
            ("Weekly", "weekly"),
            ("Every two weeks", "biweekly"),
            ("Monthly", "monthly"),
        ] {
            options.push(Self::option(
                label,
                value,
                self.recurrence_rule.as_deref() == Some(value),
            ));
        }
        self.error_wrap(
            "recurrence_rule",
            h::content_tag(
                "select",
                h::attrs()
                    .class("input")
                    .name("event[recurrence_rule]")
                    .id("event_recurrence_rule"),
                &options.join("\n"),
            ),
        )
    }
    fn venue_select(&self) -> h::Html {
        let mut groups = Vec::new();
        for (label, stage) in [("Voice", false), ("Stage", true)] {
            let options = self
                .venues
                .iter()
                .filter(|v| v.stage == stage)
                .map(|v| Self::option(&v.name, &v.id.to_string(), self.venue_room_id == Some(v.id)))
                .collect::<Vec<_>>();
            if !options.is_empty() {
                groups.push(
                    h::content_tag(
                        "optgroup",
                        h::attrs().attr("label", label),
                        &options.join("\n"),
                    )
                    .0,
                );
            }
        }
        let mut choices = Self::option("No channel", "", false);
        choices.push('\n');
        choices.push_str(&groups.join(""));
        self.error_wrap(
            "venue_room_id",
            h::content_tag(
                "select",
                h::attrs()
                    .class("input")
                    .name("event[venue_room_id]")
                    .id("event_venue_room_id"),
                &choices,
            ),
        )
    }

    fn render_form(&self) -> h::Html {
        h::raw(format!(
            "\n{}",
            Form { view: self }.render().expect("event form")
        ))
    }

}

#[derive(Template)]
#[template(path = "rooms/events/_form.html")]
pub struct Form<'a> {
    pub view: &'a FormView,
}
#[derive(Template)]
#[template(path="rooms/events/new.html",blocks=["head","content"])]
pub struct New<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub view: &'a FormView,
}
impl Page for New<'_> {
    fn page_title(&self) -> Option<String> {
        Some("New event".into())
    }
}
#[derive(Template)]
#[template(path="rooms/events/edit.html",blocks=["head","content"])]
pub struct Edit<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub view: &'a FormView,
}
impl Page for Edit<'_> {
    fn page_title(&self) -> Option<String> {
        Some(format!("Edit {}", self.view.title))
    }
}

pub use campfire_presentation::events::forms::*;
