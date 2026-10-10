use crate::helpers as h;
// Read-only presentation seams: WS11 inbox/agents, WS13 calls, WS14g Google,
// Merged GitHub/Fizzy connection and WS17 forms render directly. No credentials here.
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
#[derive(Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct StatusFields {
    pub presence: String,
    pub emoji: Option<String>,
    pub text: Option<String>,
    pub ooo_note: Option<String>,
    pub ooo_return: Option<String>,
    pub manual_ooo: bool,
    pub meeting_enabled: bool,
    pub ooo_calendar_enabled: bool,
    pub fetch_error: Option<String>,
    pub errors: std::collections::BTreeMap<String, Vec<String>>,
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
#[derive(Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct GooglePanel {
    pub sign_in_configured: bool,
    pub identity_email: Option<String>,
    pub calendar_configured: bool,
    pub account_exists: bool,
    pub connected: bool,
    pub calendar: bool,
    pub drive: bool,
    pub email: String,
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

impl StatusFields {
    pub fn error(&self, key: &str) -> Option<String> {
        self.errors
            .get(key)
            .filter(|v| !v.is_empty())
            .map(|v| h::to_sentence(v, " and "))
    }
}
