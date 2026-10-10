//! Read-only presentation seams: WS11 inbox/agents, WS13 calls, WS14g Google,
//! Merged GitHub/Fizzy connection and WS17 forms render directly. No credentials here.
use super::*;
impl ProfileShow<'_> {
    pub(super) fn github_disabled(&self) -> Option<&str> {
        self.sections.github_verified.then_some("disabled")
    }
}

/// Pure form helpers shared with owner-supplied status facts.
pub trait ProfileFormFields {
    fn switch_hidden(&self, key: impl AsRef<str>) -> h::Html {
        let key = key.as_ref();
        // Rails id:nil removes the default id; omit it rather than emitting an empty id.
        h::legacy_tag(
            "input",
            h::attrs()
                .type_("hidden")
                .name(format!("user[{key}]"))
                .value("0"),
        )
    }
    fn switch_input(
        &self,
        key: impl AsRef<str>,
        enabled: impl std::borrow::Borrow<bool>,
    ) -> h::Html {
        let key = key.as_ref();
        let mut a = h::attrs()
            .type_("checkbox")
            .name(format!("user[{key}]"))
            .id(format!("user_{}", key.replace('[', "_").replace(']', "")))
            .value("1")
            .class("switch__input");
        if *enabled.borrow() {
            a = a.attr("checked", "checked");
        }
        h::legacy_tag("input", a)
    }
    fn select(
        &self,
        key: &str,
        selected: &str,
        choices: &[(&str, &str)],
        options: h::Attrs,
    ) -> h::Html {
        let choices = choices
            .iter()
            .map(|(label, value)| {
                let a = if *value == selected {
                    h::attrs().attr("selected", "selected")
                } else {
                    h::attrs()
                };
                h::content_tag_text("option", a.value(*value), label).0
            })
            .collect::<Vec<_>>()
            .join("\n");
        h::content_tag(
            "select",
            options
                .name(format!("user[{key}]"))
                .id(format!("user_{key}")),
            &choices,
        )
    }
    fn time_input(&self, key: &str, value: Option<&str>) -> h::Html {
        h::legacy_tag(
            "input",
            h::attrs()
                .class("input")
                .id(format!("user_{key}"))
                .attr_opt("value", value)
                .type_("time")
                .name(format!("user[{key}]")),
        )
    }
}

impl ProfileFormFields for ProfileShow<'_> {}

#[derive(Template)]
#[template(path = "users/profiles/_google_calendar.html")]
pub struct GoogleCalendar {
    pub sections: ProfileSections,
}
impl ProfileShow<'_> {
    pub(super) fn google_calendar(&self) -> h::Html {
        h::raw(
            GoogleCalendar {
                sections: self.sections.clone(),
            }
            .render()
            .unwrap(),
        )
    }
    pub(super) fn google_sign_in_panel(&self) -> h::Html {
        h::raw(google::SignIn { data: google::SignInData {
            configured: self.sections.google.sign_in_configured,
            email: self.sections.google.identity_email.clone(),
        }}.render().expect("Google sign-in profile"))
    }
    pub(super) fn github_connection(&self) -> h::Html {
        h::raw(crate::github::connections::profile(&self.github))
    }
}
impl GoogleCalendar {
    fn calendar_data(&self) -> google::CalendarData {
        let panel = &self.sections.google;
        google::CalendarData {
            configured: panel.calendar_configured,
            account: panel.account_exists.then(|| google::Account {
                email: panel.email.clone(), connected: panel.connected,
                calendar: panel.calendar, drive: panel.drive,
            }),
        }
    }
    fn connect(&self, label: &str, drive: bool) -> h::Html {
        google::Calendar { data: self.calendar_data() }.connect(label, drive)
    }
    fn disconnect(&self) -> h::Html {
        google::Calendar { data: self.calendar_data() }.disconnect()
    }
}

#[derive(Template)]
#[template(path = "users/profiles/_status.html")]
pub struct StatusPanel<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub sections: ProfileSections,
}
impl ProfileFormFields for StatusPanel<'_> {}
impl StatusPanel<'_> {
    fn status_fields(&self) -> h::Html {
        StatusFieldsView {
            fields: self.sections.status.clone(),
            id_prefix: "user".into(),
        }
        .html()
    }
}

/// Complete pinned Fizzy connection fragment; credentials stay in the owner model.
#[derive(Template)]
#[template(path = "users/profiles/_fizzy_connection.html")]
pub struct FizzyConnection<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub panel: &'a ConnectionPanel,
}
impl ProfileShow<'_> {
    pub(super) fn fizzy_connection(&self) -> h::Html {
        h::raw(FizzyConnection {ctx:self.ctx, panel:&self.sections.fizzy}.render().expect("Fizzy profile"))
    }
}
pub use campfire_presentation::users::profile_sections::*;
