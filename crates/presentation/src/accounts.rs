pub mod bot_access;
pub mod icons;
pub mod audit_logs;

use crate::helpers::{self as h};
use crate::users::UserSummary;

pub use crate::HelpContact;

/// A bot, as `accounts/bots/_bot` and `_form` see it.
#[derive(Clone, Debug, Default)]
pub struct Bot {
    pub user: UserSummary,
    pub kind: Option<String>,
    pub owner_name: Option<String>,
    pub icon: Option<h::AvatarIcon>,
    /// `bot.rooms.without_directs.ordered`.
    pub rooms: Vec<BotRoom>,
}

#[derive(Clone, Debug)]
pub struct BotRoom {
    pub id: i64,
    /// `room_display_name(room)`, the room's name for shared rooms.
    pub name: String,
}

/// The fields `accounts/bots/_form` fills in.
#[derive(Clone, Debug, Default)]
pub struct BotForm {
    pub name: Option<String>,
    pub webhook_url: Option<String>,
    /// `url_for(bot.avatar)` when attached.
    pub avatar_attachment_url: Option<String>,
    pub persisted: bool,
    pub icon_name: Option<String>,
    pub icon: Option<h::AvatarIcon>,
    pub errors: Option<String>,
    pub error_fields: Vec<String>,
    pub agent: Option<BotAgentForm>,
    pub budget_usage_line: String,
    pub signing_secret: Option<String>,
    pub github: Option<BotGithubAccount>,
}

#[derive(Clone, Debug, Default)]
pub struct BotAgentForm {
    pub id: i64,
    pub owner_id: Option<i64>,
    pub provider: Option<String>,
    pub runtime: Option<String>,
    pub description: Option<String>,
    pub daily_message_cap: Option<i64>,
    pub daily_board_post_cap: Option<i64>,
    pub daily_external_action_cap: Option<i64>,
    pub raw_caps: std::collections::BTreeMap<String, Option<String>>,
    pub suspended: bool,
    pub errors: Option<String>,
    pub error_fields: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct BotGithubAccount {
    pub usable: bool,
    pub login: String,
    pub disconnected_reason: Option<String>,
}
impl BotAgentForm {
    pub fn cap_value(&self, name: &str) -> Option<String> {
        if let Some(value) = self.raw_caps.get(name) {
            return value.clone();
        }
        match name {
            "messages" => self.daily_message_cap,
            "board_posts" => self.daily_board_post_cap,
            _ => self.daily_external_action_cap,
        }
        .map(|value| value.to_string())
    }
}

impl Bot {
    /// "Workspace agent · Owned by Grace", as the list reads under the bot's name.
    pub fn ownership_line(&self) -> String {
        match &self.kind {
            Some(kind) => format!(
                "{} agent · {}",
                if kind == "workspace" {
                    "Workspace"
                } else {
                    "Personal"
                },
                self.owner_name
                    .as_ref()
                    .map(|name| format!("Owned by {name}"))
                    .unwrap_or_else(|| "no owner recorded".into())
            ),
            None => "no owner recorded".into(),
        }
    }
}
