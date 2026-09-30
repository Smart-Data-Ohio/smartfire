//! Plain view facts for our status and notification partials; no model queries or writes.
use crate::{
    ViewContext,
    helpers::{self as h, filters},
};
use askama::Template;
use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct SettingsPerson {
    pub id: i64,
    pub name: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct SettingsFormData {
    #[serde(default)]
    pub theme: String,
    #[serde(default)]
    pub text_size: String,
    #[serde(default)]
    pub time_zone: Option<String>,
    #[serde(default)]
    pub time_zone_choices: Vec<(String, String)>,
    pub presence_setting: String,
    pub custom_status_emoji: Option<String>,
    pub custom_status_text: Option<String>,
    pub ooo_note: Option<String>,
    pub dnd_active: bool,
    pub quiet_hours_enabled: bool,
    pub quiet_hours_start: Option<String>,
    pub quiet_hours_end: Option<String>,
    pub meeting_status_enabled: bool,
    pub meeting_dnd_enabled: bool,
    pub ooo_calendar_enabled: bool,
    pub ooo_notify_enabled: bool,
    pub out_of_office: bool,
    pub manual_ooo_active: bool,
    pub ooo_until_date: Option<String>,
    pub google_configured: bool,
    pub calendar_connected: bool,
    pub google_email: Option<String>,
    pub fetch_error: Option<String>,
    pub allowed_people: Vec<SettingsPerson>,
    pub keyword_alerts: String,
    pub errors: Vec<(String, String)>,
}

impl SettingsFormData {
    pub fn has_errors(&self, attribute: &str) -> bool {
        self.errors.iter().any(|(a, _)| a == attribute)
    }
    pub fn error_sentence(&self, attribute: &str) -> String {
        let errors = self
            .errors
            .iter()
            .filter(|(a, _)| a == attribute)
            .map(|(_, message)| message.clone())
            .collect::<Vec<_>>();
        h::to_sentence(&errors, " and ")
    }
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
    fn presence_select(&self) -> h::Html {
        self.select(
            "presence_setting",
            &self.presence_setting,
            &[
                ("Automatic", "auto"),
                ("Do not disturb", "dnd"),
                ("Invisible (appear offline)", "invisible"),
            ],
            true,
        )
    }
    fn expiry_select(&self) -> h::Html {
        self.select(
            "custom_status_expires_in",
            "never",
            &[
                ("30 minutes", "minutes_30"),
                ("1 hour", "hour_1"),
                ("4 hours", "hours_4"),
                ("Today", "today"),
                ("This week", "week"),
                ("Never", "never"),
            ],
            true,
        )
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
    fn fetch_error_present(&self) -> bool {
        self.fetch_error
            .as_deref()
            .is_some_and(|s| !s.chars().all(char::is_whitespace))
    }
}

#[derive(Template)]
#[template(path = "users/profiles/_status.html")]
pub struct StatusForm<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub data: &'a SettingsFormData,
}
#[derive(Template)]
#[template(path = "users/profiles/_notifications.html")]
pub struct NotificationForm<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub data: &'a SettingsFormData,
}

#[derive(Template)]
#[template(path = "users/profiles/_appearance.html")]
pub struct AppearanceForm<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub data: &'a SettingsFormData,
}

impl SettingsFormData {
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
fn profile_zone_table() -> &'static serde_json::Value {
    static TABLE: std::sync::LazyLock<serde_json::Value> = std::sync::LazyLock::new(|| {
        serde_json::from_str(include_str!("profile_zones.json")).expect("pinned Rails zone choices")
    });
    &TABLE
}

/// Rails TimeZone#to_s uses TZInfo's current *base* UTC offset (including negative DST),
/// rather than the total wall-clock offset. Vendored transitions come from the pinned image.
pub fn profile_time_zone_choices(now: jiff::Timestamp) -> Vec<(String, String)> {
    profile_zone_table()["choice_zones"]
        .as_array()
        .unwrap()
        .iter()
        .map(|zone| {
            let mut offset = zone["initial"].as_i64().unwrap();
            for change in zone["changes"].as_array().unwrap() {
                if change[0].as_i64().unwrap() > now.as_second() {
                    break;
                }
                offset = change[1].as_i64().unwrap();
            }
            let label = format!(
                "(GMT{}{:02}:{:02}) {}",
                if offset < 0 { "-" } else { "+" },
                offset.abs() / 3600,
                offset.abs() % 3600 / 60,
                zone["name"].as_str().unwrap()
            );
            (label, zone["id"].as_str().unwrap().to_owned())
        })
        .collect()
}
