//! Plain view facts for our status and notification partials; no model queries or writes.
use crate::{
    ViewContext,
    helpers::{self as h, filters},
};
use askama::Template;
pub trait SettingsFormDataRendering {
    fn field(&self, attribute: &str, html: h::Html) -> h::Html;
    fn label(&self, form: &h::FormWith, attribute: &str, text: &str, class: &str) -> h::Html;
    fn text(
        &self,
        form: &h::FormWith,
        attribute: &str,
        value: Option<&str>,
        options: h::Attrs,
    ) -> h::Html;
    fn clock_field(&self, form: &h::FormWith, attribute: &str, value: Option<&str>) -> h::Html;
    fn select(
        &self,
        attribute: &str,
        current: &str,
        options: &[(&str, &str)],
        model: bool,
    ) -> h::Html;
    fn ooo_select(&self) -> h::Html;
    fn check(&self, attribute: &str, checked: &bool) -> h::Html;
    fn hidden(&self, attribute: &str) -> h::Html;
    fn tag_label(&self, attribute: &str, text: &str) -> h::Html;
    fn note_field(&self) -> h::Html;
    fn keyword_field(&self) -> h::Html;
    fn remove_button(&self, person: &SettingsPerson) -> h::Html;

    fn appearance_radio(&self, attribute: &str, value: &str, current: &str) -> h::Html;
    fn zone_select(&self) -> h::Html;
}

impl SettingsFormDataRendering for SettingsFormData {
    fn field(&self, attribute: &str, html: h::Html) -> h::Html {
        if self.has_errors(attribute) {
            h::content_tag("div", h::attrs().class("field_with_errors"), &html.0)
        } else {
            html
        }
    }
    fn label(&self, form: &h::FormWith, attribute: &str, text: &str, class: &str) -> h::Html {
        self.field(
            attribute,
            form.label(attribute, text, h::attrs().class(class)),
        )
    }
    fn text(
        &self,
        form: &h::FormWith,
        attribute: &str,
        value: Option<&str>,
        options: h::Attrs,
    ) -> h::Html {
        self.field(attribute, form.text_field(attribute, value, options))
    }
    fn clock_field(&self, form: &h::FormWith, attribute: &str, value: Option<&str>) -> h::Html {
        self.text(
            form,
            attribute,
            value,
            h::attrs()
                .class("input")
                .id(format!("user_{attribute}"))
                .attr_opt("value", value)
                .type_("time"),
        )
    }
    fn select(
        &self,
        attribute: &str,
        current: &str,
        options: &[(&str, &str)],
        model: bool,
    ) -> h::Html {
        let name = format!("user[{attribute}]");
        let id = format!("user_{attribute}");
        let attrs = if model {
            h::attrs().class("input").id(id).name(name)
        } else {
            h::attrs().name(name).id(id).class("input")
        };
        let options = options
            .iter()
            .map(|(label, value)| {
                let attrs = if *value == current {
                    h::attrs().attr("selected", "selected")
                } else {
                    h::attrs()
                };
                h::content_tag_text("option", attrs.value(*value), label).0
            })
            .collect::<Vec<_>>()
            .join("\n");
        let html = h::content_tag("select", attrs, &options);
        if model {
            self.field(attribute, html)
        } else {
            html
        }
    }
    fn ooo_select(&self) -> h::Html {
        self.select(
            "ooo_preset",
            "",
            &[
                ("Don't change", ""),
                ("Until tomorrow", "tomorrow"),
                ("Until Monday", "monday"),
                ("1 week", "week"),
                ("Custom date and time", "custom"),
            ],
            false,
        )
    }
    fn check(&self, attribute: &str, checked: &bool) -> h::Html {
        h::legacy_tag(
            "input",
            h::attrs()
                .type_("checkbox")
                .name(format!("user[{attribute}]"))
                .id(format!("user_{attribute}"))
                .value("1")
                .class("switch__input")
                .attr("checked", *checked),
        )
    }
    fn hidden(&self, attribute: &str) -> h::Html {
        h::hidden_field_tag(
            &format!("user[{attribute}]"),
            Some("0"),
            h::attrs().attr_opt("id", None::<&str>),
        )
    }
    fn tag_label(&self, attribute: &str, text: &str) -> h::Html {
        h::content_tag_text(
            "label",
            h::attrs()
                .class("txt-medium")
                .attr("for", format!("user_{attribute}")),
            text,
        )
    }
    fn note_field(&self) -> h::Html {
        h::legacy_tag(
            "input",
            h::attrs()
                .type_("text")
                .name("user[ooo_note]")
                .id("user_ooo_note")
                .attr_opt("value", self.ooo_note.as_deref())
                .class("input flex-item-grow")
                .maxlength(140)
                .placeholder("Back soon, slow to reply…")
                .autocomplete("off"),
        )
    }
    fn keyword_field(&self) -> h::Html {
        h::content_tag(
            "textarea",
            h::attrs()
                .name("user[keyword_alerts]")
                .id("user_keyword_alerts")
                .rows(4)
                .class("input full-width")
                .autocomplete("off")
                .placeholder("production\ndeploy freeze"),
            &h::escape(&self.keyword_alerts),
        )
    }
    fn remove_button(&self, person: &SettingsPerson) -> h::Html {
        h::button_to(
            &h::routes::user_dnd_allowance(person.id).to_string(),
            h::attrs()
                .method("delete")
                .class("btn btn--negative txt-small")
                .aria(
                    "label",
                    format!("Remove {} from DND exceptions", person.name),
                ),
            "Remove",
        )
    }

    fn appearance_radio(&self, attribute: &str, value: &str, current: &str) -> h::Html {
        self.field(
            attribute,
            h::legacy_tag(
                "input",
                h::attrs()
                    .id(format!("user_{attribute}_{value}"))
                    .type_("radio")
                    .value(value)
                    .attr("checked", value == current)
                    .name(format!("user[{attribute}]")),
            ),
        )
    }
    fn zone_select(&self) -> h::Html {
        let table = profile_zone_table();
        let current = self
            .time_zone
            .as_deref()
            .and_then(|zone| table["mapping"][zone].as_str());
        let mut options =
            h::content_tag_text("option", h::attrs().value(""), "Not set (use system)").0;
        for (label, value) in &self.time_zone_choices {
            options.push('\n');
            options.push_str(
                &h::content_tag_text(
                    "option",
                    h::attrs()
                        .attr("selected", current == Some(value.as_str()))
                        .value(value),
                    label,
                )
                .0,
            );
        }
        self.field(
            "time_zone",
            h::content_tag(
                "select",
                h::attrs()
                    .class("input flex-item-grow")
                    .id("user_time_zone")
                    .name("user[time_zone]"),
                &options,
            ),
        )
    }

}

#[derive(Template)]
#[template(path = "users/settings/status.html")]
pub struct StatusForm<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub data: &'a SettingsFormData,
}
impl StatusForm<'_> {
    fn status_fields(&self) -> h::Html {
        let mut fields = super::StatusFields {
            presence: self.data.presence_setting.clone(),
            emoji: self.data.custom_status_emoji.clone(),
            text: self.data.custom_status_text.clone(),
            ..Default::default()
        };
        for (key, message) in &self.data.errors {
            fields.errors.entry(key.clone()).or_default().push(message.clone());
        }
        super::StatusFieldsView { fields, id_prefix: "user".into() }.html()
    }
}
#[derive(Template)]
#[template(path = "users/settings/notifications.html")]
pub struct NotificationForm<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub data: &'a SettingsFormData,
}

#[derive(Template)]
#[template(path = "users/settings/appearance.html")]
pub struct AppearanceForm<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub data: &'a SettingsFormData,
}

pub use campfire_presentation::users::settings::*;
