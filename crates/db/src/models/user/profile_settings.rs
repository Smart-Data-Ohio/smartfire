//! Assignments made by Users::ProfilesController, including the model normalizers.
use crate::{Result, Tx};
use campfire_richtext::ruby::{is_blank, strip};
use rails_compat::unicode;
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
    pub appearance_preferences: Option<Value>,
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
    pub appearance_preferences: Option<Value>,
}
pub fn appearance(conn: &rusqlite::Connection, user: i64) -> Result<Appearance> {
    Ok(conn.query_row(
        "SELECT theme,text_size,time_zone,appearance_preferences FROM users WHERE id=?",
        [user],
        |r| {
            Ok(Appearance {
                theme: r.get(0)?,
                text_size: r.get(1)?,
                time_zone: r.get(2)?,
                appearance_preferences: r.get::<_, Option<String>>(3)?
                    .map(|raw| serde_json::from_str(&raw).map_err(|error| rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(error))))
                    .transpose()?,
            })
        },
    )?)
}

/// Run inside the same writer transaction as core profile, security and attachment changes.
/// Validation also examines unchanged settings, as User#save does in Rails.
pub fn update(tx: &Tx<'_>, user: i64, changes: Changes) -> Result<()> {
    let appearance = changes.appearance_preferences.map(|value| -> Result<String> {
        validate_appearance(&value)?;
        Ok(serde_json::to_string(&value).expect("JSON appearance"))
    }).transpose()?;
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
        let accounts: Vec<(String, Option<String>)> = crate::sql::query_all(
            tx.conn(),
            "SELECT github_login,disconnected_reason FROM github_connected_accounts WHERE user_id=?",
            [user],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let verified_login = accounts
            .iter()
            .find(|(_, reason)| reason.as_deref().is_none_or(is_blank))
            .map(|(login, _)| unicode::downcase(strip(login)));
        let login = unicode::downcase(strip(&login));
        let login = (!is_blank(&login)).then_some(login);
        let previous: Option<String> =
            tx.conn()
                .query_row("SELECT github_login FROM users WHERE id=?", [user], |row| {
                    row.get(0)
                })?;
        if login != previous && verified_login.is_some() && login != verified_login {
            let mut errors = crate::Errors::default();
            errors.add("github_login", "is set by your linked GitHub account");
            return Err(crate::Error::RecordInvalid(errors));
        }
        attrs.insert(
            "github_login".into(),
            login.map(Value::String).unwrap_or(Value::Null),
        );
    }

    if let Some(preferences) = changes.inbox_preferences {
        let raw: Option<String> = tx.conn().query_row(
            "SELECT inbox_preferences FROM users WHERE id=?",
            [user],
            |r| r.get(0),
        )?;
        let mut existing = raw
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default();
        let value = if let Some(preferences) = preferences.as_object() {
            for (key, value) in preferences {
                if INBOX_KEYS.contains(&key.as_str()) || crate::models::notification_policy::NOTIFICATION_PREFERENCE_KEYS.contains(&key.as_str()) {
                    existing.insert(key.clone(), value.clone());
                }
            }
            Value::Object(existing)
        } else if preferences.is_null() {
            existing.retain(|key, _| key == "settings_revision");
            Value::Object(existing)
        } else {
            preferences
        };
        attrs.insert(
            "inbox_preferences".into(),
            Value::String(serde_json::to_string(&value).expect("JSON preferences")),
        );
    }
    crate::slash_commands::user_settings::update(tx, user, Value::Object(attrs))?;
    if let Some(appearance) = appearance {
        tx.conn().execute("UPDATE users SET appearance_preferences=? WHERE id=?", rusqlite::params![appearance, user])?;
    }
    Ok(())
}

/// Settings and notification counts share the persisted activity counter. Keep ordering
/// metadata out of the legacy preferences JSON; request bodies cannot assign the counter.
pub fn bump_revision(tx: &Tx<'_>, user: i64) -> Result<()> {
    tx.conn().execute(
        "UPDATE users SET activity_revision=activity_revision+1 WHERE id=?",
        [user],
    )?;
    Ok(())
}

/// Semantic colour names from frontend/src/styles/tokens.css.
pub const APPEARANCE_TOKENS: &[&str] = &[
    "--bg-app",
    "--bg-sidebar",
    "--bg-pane",
    "--bg-raised",
    "--bg-hover",
    "--bg-active",
    "--bg-selected",
    "--bg-sunken",
    "--bg-skeleton",
    "--image-backdrop-a",
    "--image-backdrop-b",
    "--text",
    "--text-strong",
    "--text-muted",
    "--text-faint",
    "--border",
    "--border-strong",
    "--accent",
    "--accent-solid",
    "--accent-soft",
    "--on-accent",
    "--focus-ring",
    "--mention",
    "--mention-bg",
    "--mention-chip-bg",
    "--mention-text",
    "--success",
    "--success-text",
    "--warning",
    "--warning-text",
    "--danger",
    "--danger-solid",
    "--danger-soft",
    "--danger-text",
    "--on-danger",
    "--presence-online",
    "--presence-away",
    "--presence-dnd",
    "--presence-offline",
    "--agent",
    "--agent-text",
    "--agent-soft",
    "--scrim",
    "--tooltip-bg",
    "--tooltip-text",
    "--code-plain",
    "--code-comment",
    "--code-punctuation",
    "--code-keyword",
    "--code-control",
    "--code-string",
    "--code-regexp",
    "--code-special",
    "--code-number",
    "--code-variable",
    "--code-constant",
    "--code-function",
    "--code-type",
    "--code-invalid",
    "--code-inserted",
    "--code-deleted",
];

/// Opaque future versions round-trip; this build validates every v1 key before writing.
pub fn validate_appearance(value: &Value) -> Result<()> {
    let reject = |message: String| {
        let mut errors = crate::Errors::default();
        errors.add("appearance_preferences", message);
        crate::Error::RecordInvalid(errors)
    };
    if value.to_string().len() > 8192 {
        return Err(reject("must be at most 8192 bytes".into()));
    }
    let object = value.as_object().ok_or_else(|| reject("must be a versioned JSON object".into()))?;
    let version = object.get("version").and_then(Value::as_u64)
        .filter(|version| *version > 0)
        .ok_or_else(|| reject("version must be a positive integer".into()))?;
    if version != 1 { return Ok(()); }
    for (key, value) in object {
        let options: &[&str] = match key.as_str() {
            "version" => continue,
            "palette" => &["smartfire", "graphite", "ocean", "forest", "ember", "rose"],
            "font" => &["inter", "system", "atkinson", "serif", "mono"],
            "density" => &["comfortable", "compact"],
            "motion" => &["system", "reduce", "full"],
            "tokens" => {
                let tokens = value.as_object().ok_or_else(|| reject("tokens must be a colour map".into()))?;
                for (name, colour) in tokens {
                    if !APPEARANCE_TOKENS.contains(&name.as_str()) {
                        return Err(reject(format!("token {name} is not allowed")));
                    }
                    if !colour.as_str().is_some_and(valid_colour) {
                        return Err(reject(format!("token {name} must be a hex or rgb colour")));
                    }
                }
                continue;
            }
            _ => return Err(reject(format!("key {key} is not allowed in version 1"))),
        };
        if !value.as_str().is_some_and(|value| options.contains(&value)) {
            return Err(reject(format!("{key} must be one of {}", options.join(", "))));
        }
    }
    Ok(())
}

fn valid_colour(colour: &str) -> bool {
    if let Some(hex) = colour.strip_prefix('#') {
        return matches!(hex.len(), 3 | 4 | 6 | 8) && hex.bytes().all(|byte| byte.is_ascii_hexdigit());
    }
    let colour = colour.to_ascii_lowercase();
    let Some(channels) = colour.strip_prefix("rgb(").or_else(|| colour.strip_prefix("rgba("))
        .and_then(|value| value.strip_suffix(')')) else { return false };
    let comma = channels.contains(',');
    let parts: Vec<_> = if comma {
        if channels.contains('/') { return false; }
        channels.split(',').map(str::trim).collect()
    } else {
        let (rgb, alpha) = channels.split_once('/').map_or((channels, None), |(rgb, alpha)| (rgb, Some(alpha.trim())));
        let mut parts: Vec<_> = rgb.split_ascii_whitespace().collect();
        if parts.len() != 3 { return false; }
        if let Some(alpha) = alpha { parts.push(alpha); }
        parts
    };
    if !(parts.len() == 3 || parts.len() == 4)
        || (colour.starts_with("rgba(") && comma && parts.len() != 4)
        || (!comma && parts.len() == 4 && channels.matches('/').count() != 1) {
        return false;
    }
    parts.iter().enumerate().all(|(index, value)| {
        let (number, max) = if let Some(number) = value.strip_suffix('%') { (number, 100.0) }
            else { (*value, if index < 3 { 255.0 } else { 1.0 }) };
        number.parse::<f64>().is_ok_and(|number| number.is_finite() && (0.0..=max).contains(&number))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{TestDb, id};
    use serde_json::json;

    #[test]
    fn appearance_round_trips_v1_and_opaque_versions_without_changing_legacy_columns() {
        let t = TestDb::new();
        let user = id("david");
        assert!(t.read(move |conn| appearance(conn, user)).appearance_preferences.is_none());
        let before = t.read(move |conn| appearance(conn, user));
        for preferences in [
            json!({"version":1,"palette":"rose","font":"mono","density":"compact","motion":"reduce","tokens":{"--accent":"#abcd","--text":"rgba(10, 20, 30, 0.5)"}}),
            json!({"version":42,"future":{"nested":[true,null,"keep"]}}),
        ] {
            let expected = preferences.clone();
            t.write(move |tx| update(tx, user, Changes { appearance_preferences: Some(preferences), ..Default::default() }));
            let after = t.read(move |conn| appearance(conn, user));
            assert_eq!(after.appearance_preferences, Some(expected));
            assert_eq!((after.theme, after.text_size), (before.theme.clone(), before.text_size.clone()));
            t.write(move |tx| update(tx, user, Changes::default()));
            assert_eq!(t.read(move |conn| appearance(conn, user)).appearance_preferences, after.appearance_preferences);
        }
    }

    #[test]
    fn appearance_validation_rejects_bad_keys_presets_colours_versions_and_size() {
        for value in [json!([]), json!({}), json!({"version":-1}), json!({"version":1.5}),
            json!({"version":1,"palette":"neon"}), json!({"version":1,"font":"comic"}),
            json!({"version":1,"density":"small"}), json!({"version":1,"motion":"off"}),
            json!({"version":1,"tokens":{"--space-1":"#123456"}}), json!({"version":1,"tokens":{"--accent":"red"}}),
            json!({"version":1,"tokens":{"--accent":"rgb(1 / 2 3)"}}),
            json!({"version":1,"tokens":{"--accent":"rgb(1 2 3 /)"}}),
            json!({"version":1,"tokens":{"--accent":"rgb(256, 0, 0)"}}), json!({"version":1,"tokens":{"--accent":"rgba(0, 0, 0, 2)"}}),
            json!({"version":2,"future":"x".repeat(8193)}),
        ] {
            assert!(matches!(validate_appearance(&value), Err(crate::Error::RecordInvalid(_))), "{value}");
        }
        for colour in ["#abc", "#abcd", "#123ABC", "#123ABCff", "rgb(0, 255, 10)", "rgb(10% 20% 30% / 50%)", "rgba(10, 20, 30, .5)"] {
            validate_appearance(&json!({"version":1,"tokens":{"--accent":colour}})).unwrap();
        }
        let css = include_str!("../../../../../frontend/src/styles/tokens.css");
        let colours = css.split("Semantic colour").nth(1).unwrap().split("/* Shadows:").next().unwrap();
        let declarations: Vec<_> = colours.lines().filter_map(|line| line.trim().split_once(':').map(|(name, _)| name)).filter(|name| name.starts_with("--")).collect();
        assert_eq!(APPEARANCE_TOKENS, declarations);
    }

    #[test]
    fn appearance_migration_adds_only_a_nullable_column_on_the_previous_schema() {
        let catalog = crate::migrations::catalog();
        let previous = crate::schema::generate(&catalog[..catalog.len()-1]).unwrap();
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(&previous.schema_sql).unwrap();
        for version in previous.schema_migrations.lines() {
            conn.execute("INSERT INTO schema_migrations VALUES (?)", [version]).unwrap();
        }
        conn.execute("INSERT INTO users(id,name,theme,text_size,created_at,updated_at) VALUES(1,'Ada','dark','large','2026-10-10','2026-10-10')", []).unwrap();
        assert_eq!(crate::migrations::migrate(&mut conn).unwrap(), ["20261010150000"]);
        let row: (String, String, Option<String>) = conn.query_row("SELECT theme,text_size,appearance_preferences FROM users WHERE id=1", [], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap();
        assert_eq!(row, ("dark".into(), "large".into(), None));
        assert!(crate::migrations::migrate(&mut conn).unwrap().is_empty());
    }
}
