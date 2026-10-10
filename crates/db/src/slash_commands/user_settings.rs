//! User#update! validations for the status columns written by slash commands.
//! Other User validations conditional on changing login/icon/status do not fire here.
use super::*;
use rusqlite::types::Value as SqlValue;
use std::collections::HashMap;

pub(crate) fn update(tx: &Tx<'_>, user: i64, changes: Value) -> Result<()> {
    let now = tx.now();
    let mut stmt = tx.conn().prepare("SELECT * FROM users WHERE id=?")?;
    let columns = stmt
        .column_names()
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let original = stmt.query_row([user], |row| {
        columns
            .iter()
            .enumerate()
            .map(|(i, k)| Ok((k.clone(), row.get::<_, SqlValue>(i)?)))
            .collect::<rusqlite::Result<HashMap<_, _>>>()
    })?;
    let mut attrs = original.clone();
    let changes = changes.as_object().expect("internal status changes");
    for (key, v) in changes {
        attrs.insert(
            key.clone(),
            match v {
                Value::Null => SqlValue::Null,
                Value::Bool(b) => SqlValue::Integer(i64::from(*b)),
                Value::String(s) if key == "time_zone" && campfire_richtext::ruby::is_blank(s) => SqlValue::Null,
                Value::String(s) => SqlValue::Text(s.clone()),
                _ => unreachable!("typed internal changes"),
            },
        );
    }
    let text = |key: &str| match attrs.get(key) {
        Some(SqlValue::Text(s)) => Some(s.as_str()),
        _ => None,
    };
    let number = |key: &str| match attrs.get(key) {
        Some(SqlValue::Integer(n)) => Some(*n),
        _ => None,
    };
    let mut errors = Errors::default();
    for (key, values) in [
        ("presence_setting", &["auto", "dnd", "invisible"][..]),
        ("theme", &["light", "dark", "system"][..]),
        (
            "text_size",
            &["smaller", "small", "default", "large", "larger"][..],
        ),
    ] {
        if !text(key).is_some_and(|s| values.contains(&s)) {
            errors.add(key, "is not included in the list");
        }
    }
    for (key, limit) in [
        ("custom_status_emoji", 8),
        ("custom_status_text", 100),
        ("ooo_note", 140),
    ] {
        if text(key).is_some_and(|s| s.chars().count() > limit) {
            errors.add(key, format!("is too long (maximum is {limit} characters)"));
        }
    }
    if attrs["ooo_until"] != original["ooo_until"]
        && text("ooo_until")
            .and_then(Timestamp::parse_db)
            .is_some_and(|t| t <= now)
    {
        errors.add("ooo_until", "must be in the future");
    }
    for key in ["quiet_hours_start_minute", "quiet_hours_end_minute"] {
        if !matches!(attrs[key], SqlValue::Null) {
            if let Some(n) = number(key) {
                if n < 0 {
                    errors.add(key, "must be greater than or equal to 0");
                }
                if n >= 1440 {
                    errors.add(key, "must be less than 1440");
                }
            } else {
                errors.add(key, "must be an integer");
            }
        }
    }
    if let Some(name) = text("time_zone").filter(|s| present(s).is_some())
        && time_parser::known_zone(name).is_none()
    {
        errors.add("time_zone", "is not a valid time zone");
    }
    if number("quiet_hours_enabled") == Some(1)
        && (number("quiet_hours_start_minute").is_none()
            || number("quiet_hours_end_minute").is_none())
    {
        errors.add(
            "quiet_hours_start",
            "needs a start and an end while quiet hours are on",
        );
    }
    // Normalizations on unchanged persisted attributes are still applied on assignment by
    // Rails; slash commands only assign the typed fields above.
    let preferences: Value = text("inbox_preferences")
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(Value::Null);
    if !preferences.is_null() && !preferences.is_object() {
        errors.add("inbox_preferences", "is invalid");
    }
    if let Some(preferences) = preferences.as_object() {
        for (key, value) in preferences {
            if key == "settings_revision" {
                if value.as_i64().is_none_or(|revision| revision < 0) {
                    errors.add("inbox_preferences", "settings revision is invalid");
                }
                continue;
            }
            if crate::models::notification_policy::NOTIFICATION_PREFERENCE_KEYS.contains(&key.as_str()) {
                if !crate::models::notification_policy::valid_preference(key, value) {
                    errors.add("inbox_preferences", format!("{key} is invalid"));
                }
                continue;
            }
            if !matches!(value, Value::Bool(_))
                && ![
                    json!(0),
                    json!(1),
                    json!("0"),
                    json!("1"),
                    json!("true"),
                    json!("false"),
                ]
                .contains(value)
            {
                // ActiveModel replaces dots before humanizing nested attributes. A base
                // message preserves the arbitrary JSON key without leaking static strings.
                errors.add(
                    "base",
                    format!(
                        "Inbox preferences {} must be true or false",
                        key.replace(['_', '.'], " ")
                    ),
                );
            }
        }
    }
    if let Some(voice) = text("voice_mode").filter(|s| present(s).is_some())
        && !["voice_activity", "push_to_talk"].contains(&voice)
    {
        errors.add("voice_mode", "is invalid");
    }
    if text("push_to_talk_key").is_some_and(|s| present(s).is_some() && s.chars().count() > 20) {
        errors.add("push_to_talk_key", "is too long");
    }
    // Uniqueness runs on every save, even when the login itself was not assigned.
    if let Some(login) = text("github_login")
        && tx.conn().query_row(
            "SELECT EXISTS(SELECT 1 FROM users WHERE id!=? AND LOWER(github_login)=LOWER(?))",
            params![user, login],
            |r| r.get::<_, bool>(0),
        )?
    {
        errors.add("github_login", "is already linked to another user");
    }
    errors.into_result()?;
    let changed = changes
        .keys()
        .filter(|key| attrs[*key] != original[*key])
        .collect::<Vec<_>>();
    if changed.is_empty() {
        if changes.contains_key("inbox_preferences") {
            crate::models::user::profile_settings::bump_revision(tx, user)?;
        }
        return Ok(());
    }
    let mut values = changed
        .iter()
        .map(|key| attrs[*key].clone())
        .collect::<Vec<_>>();
    let revision = crate::User::revision_for_touch(tx, user, now)?;
    values.push(SqlValue::Text(revision.to_db()));
    values.push(SqlValue::Integer(user));
    let assignments = changed
        .iter()
        .map(|key| format!("{key}=?"))
        .collect::<Vec<_>>()
        .join(",");
    tx.conn().execute(
        &format!("UPDATE users SET {assignments},updated_at=?,activity_revision=activity_revision+1 WHERE id=?"),
        rusqlite::params_from_iter(values),
    )?;
    Ok(())
}
