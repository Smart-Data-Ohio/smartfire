//! Read-only presentation seams: WS11 inbox/agents, WS13 calls, WS14g Google,
//! WS15g GitHub, WS15e Fizzy/Slack, WS17 status/notifications. No clients or mutations here.
use super::*;
#[derive(Clone, Default)]
pub struct ProfileSections {
    pub github_login: Option<String>,
    pub github_verified: bool,
    pub github_errors: Vec<String>,
    pub inbox: Vec<InboxSwitch>,
    pub inbox_errors: Vec<String>,
    pub voice_mode: String,
    pub push_to_talk_key: Option<String>,
    pub call_errors: Vec<String>,
    pub status: StatusFields,
    pub notifications: NotificationFields,
    /// WS14g replaces the configuration/connection projection at integration.
    pub google: GooglePanel,
    /// WS15g/WS15e must supply token usability after decryption, including its side effect.
    pub github: ConnectionPanel,
    pub fizzy: ConnectionPanel,
    pub github_app_configured: bool,
}
#[derive(Clone)]
pub struct InboxSwitch {
    pub key: String,
    pub label: &'static str,
    pub description: &'static str,
    pub enabled: bool,
}
#[derive(Clone, Default)]
pub struct StatusFields {
    pub presence: String,
    pub emoji: Option<String>,
    pub text: Option<String>,
    pub ooo_note: Option<String>,
    pub ooo_return: Option<String>,
    pub manual_ooo: bool,
}
#[derive(Clone, Default)]
pub struct NotificationFields {
    pub manual_dnd: bool,
    pub quiet_hours: bool,
    pub quiet_start: Option<String>,
    pub quiet_end: Option<String>,
    pub meeting_dnd: bool,
    pub ooo_notify: bool,
    pub keywords: String,
    pub allowed_people: Vec<(i64, String)>,
}
#[derive(Clone, Default)]
pub struct GooglePanel {
    pub sign_in_configured: bool,
    pub identity_email: Option<String>,
    pub calendar_configured: bool,
}
#[derive(Clone, Default)]
pub enum ConnectionPanel {
    #[default]
    Missing,
    Rejected {
        reason: Option<String>,
    },
    Connected {
        name: String,
        workspace: Option<String>,
        app_token: bool,
    },
}
impl ConnectionPanel {
    pub fn connect_label(&self, provider: &str) -> String {
        format!(
            "{} {provider}",
            if self.exists() {
                "Reconnect"
            } else {
                "Connect"
            }
        )
    }
    pub fn oauth_label(&self) -> &str {
        if self.exists() {
            "Reconnect with GitHub"
        } else {
            "Connect with GitHub"
        }
    }
    pub fn exists(&self) -> bool {
        !matches!(self, Self::Missing)
    }
    pub fn connected(&self) -> bool {
        matches!(self, Self::Connected { .. })
    }
    pub fn name(&self) -> &str {
        if let Self::Connected { name, .. } = self {
            name
        } else {
            ""
        }
    }
    pub fn workspace(&self) -> &str {
        if let Self::Connected { workspace, .. } = self {
            workspace.as_deref().unwrap_or("")
        } else {
            ""
        }
    }
    pub fn app_token(&self) -> bool {
        matches!(
            self,
            Self::Connected {
                app_token: true,
                ..
            }
        )
    }
    pub fn reason(&self) -> Option<&str> {
        if let Self::Rejected { reason } = self {
            reason.as_deref()
        } else {
            None
        }
    }
}
impl ProfileShow<'_> {
    pub(super) fn github_disabled(&self) -> Option<&str> {
        self.sections.github_verified.then_some("disabled")
    }
    pub(super) fn switch_hidden(&self, key: impl AsRef<str>) -> h::Html {
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
    pub(super) fn switch_input(
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
    pub(super) fn select(
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
    pub(super) fn time_input(&self, key: &str, value: Option<&str>) -> h::Html {
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
