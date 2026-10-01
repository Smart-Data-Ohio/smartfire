//! Assignments made by Users::ProfilesController, including the model normalizers.
use crate::{Result, Tx};
use campfire_richtext::ruby::{is_blank, strip};
use serde_json::{Map, Value};

pub const INBOX_KEYS: &[&str] = &[
    "github_review_requests",
    "agent_approvals",
    "agent_work",
    "event_reminders",
    "huddle_invitations",
];

#[derive(Clone, Default)]
pub struct Changes {
    pub theme: Option<String>,
    pub text_size: Option<String>,
    pub time_zone: Option<String>,
    pub voice_mode: Option<String>,
    pub push_to_talk_key: Option<String>,
    pub github_login: Option<String>,
    pub inbox_preferences: Option<Value>,
}

pub fn zone_identifier(stored: &str) -> Option<String> {
    crate::slash_commands::time_parser::known_zone(stored)?
        .iana_name()
        .map(str::to_owned)
}

#[derive(Clone)]
pub struct Appearance {
    pub theme: String,
    pub text_size: String,
    pub time_zone: Option<String>,
}
pub fn appearance(conn: &rusqlite::Connection, user: i64) -> Result<Appearance> {
    Ok(conn.query_row(
        "SELECT theme,text_size,time_zone FROM users WHERE id=?",
        [user],
        |r| {
            Ok(Appearance {
                theme: r.get(0)?,
                text_size: r.get(1)?,
                time_zone: r.get(2)?,
            })
        },
    )?)
}

/// Run inside the same writer transaction as core profile, security and attachment changes.
/// Validation also examines unchanged settings, as User#save does in Rails.
pub fn update(tx: &Tx<'_>, user: i64, changes: Changes) -> Result<()> {
    let mut attrs = Map::new();
    for (key, value) in [
        ("theme", changes.theme),
        ("text_size", changes.text_size),
        ("voice_mode", changes.voice_mode),
    ] {
        if let Some(value) = value {
            attrs.insert(key.into(), Value::String(value));
        }
    }
    // Zone writes and the explicit-choice marker use WS9 UserChanges before this writer.
    if let Some(key) = changes.push_to_talk_key {
        let key = strip(&key);
        attrs.insert(
            "push_to_talk_key".into(),
            if is_blank(key) {
                Value::Null
            } else {
                Value::String(key.into())
            },
        );
    }
    if let Some(login) = changes.github_login {
        let reasons: Vec<Option<String>> = crate::sql::query_all(
            tx.conn(),
            "SELECT disconnected_reason FROM github_connected_accounts WHERE user_id=?",
            [user],
            |r| r.get(0),
        )?;
        let verified = reasons
            .iter()
            .any(|reason| reason.as_deref().is_none_or(is_blank));
        if !verified {
            let login = strip(&login).to_lowercase();
            attrs.insert(
                "github_login".into(),
                if is_blank(&login) {
                    Value::Null
                } else {
                    Value::String(login)
                },
            );
        }
    }
    if let Some(preferences) = changes.inbox_preferences {
        let value = if let Some(preferences) = preferences.as_object() {
            let raw: Option<String> = tx.conn().query_row(
                "SELECT inbox_preferences FROM users WHERE id=?",
                [user],
                |r| r.get(0),
            )?;
            let mut existing = raw
                .and_then(|s| serde_json::from_str::<Value>(&s).ok())
                .and_then(|v| v.as_object().cloned())
                .unwrap_or_default();
            for (key, value) in preferences {
                if INBOX_KEYS.contains(&key.as_str()) {
                    existing.insert(key.clone(), value.clone());
                }
            }
            Value::Object(existing)
        } else {
            preferences
        };
        attrs.insert(
            "inbox_preferences".into(),
            Value::String(serde_json::to_string(&value).expect("JSON preferences")),
        );
    }
    crate::slash_commands::user_settings::update(tx, user, Value::Object(attrs))
}
