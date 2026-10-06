//! Users::NotificationSettingsController: atomic settings and keyword-list replacement.
use crate::app::AppCtx;
use crate::concerns::{self, Before};
use campfire_db::{UserStatusSettings, models::user_status_settings::clock_time_to_minutes};
use campfire_kit::{Ctx, Error, Param, Redirect, Result, StatusCode, permit_keys};

pub async fn update(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let id = concerns::require_current_user(c)?.id;
    let raw = c.params.require("user")?.as_hash().ok_or_else(|| {
        Error::internal(anyhow::anyhow!(
            "undefined method permit for user parameter"
        ))
    })?;
    let params = raw.permit(&permit_keys(&[
        "dnd_enabled",
        "quiet_hours_enabled",
        "quiet_hours_start",
        "quiet_hours_end",
        "meeting_dnd_enabled",
        "ooo_notify_enabled",
    ]));
    let mut user = c
        .app()
        .db
        .read(move |conn| UserStatusSettings::find(conn, id))
        .await
        .map_err(Error::internal)?;
    let mut nulls = Vec::new();
    for (key, field) in [
        ("dnd_enabled", &mut user.dnd_enabled),
        ("quiet_hours_enabled", &mut user.quiet_hours_enabled),
        ("meeting_dnd_enabled", &mut user.meeting_dnd_enabled),
        ("ooo_notify_enabled", &mut user.ooo_notify_enabled),
    ] {
        if let Some(value) = params.get(key) {
            let value = value
                .to_s()
                .map(|s| campfire_db::models::account::cast_boolean(&s))
                .unwrap_or(Some(true));
            *field = value.unwrap_or(false);
            if value.is_none() {
                nulls.push(key);
            }
        }
    }
    if let Some(value) = params.get("quiet_hours_start") {
        user.quiet_hours_start_minute = value.to_s().and_then(|s| clock_time_to_minutes(&s));
    }
    if let Some(value) = params.get("quiet_hours_end") {
        user.quiet_hours_end_minute = value.to_s().and_then(|s| clock_time_to_minutes(&s));
    }
    if raw.get("dnd_enabled").is_some() {
        user.reconcile_dnd_timer(c.app().db.env().now());
    }
    let keywords = raw.get("keyword_alerts").map(keyword_lines).transpose()?;
    let submitted = user.clone();
    let result = c
        .app()
        .db
        .write(move |tx| {
            user.save_with_keywords(tx, keywords.as_deref())?;
            // Boolean casts of nil/empty remain nil in Rails and hit the NOT NULL column after
            // validation; preserve that failure and roll back the replacement, rather than turn off.
            for key in nulls {
                tx.conn()
                    .execute(&format!("UPDATE users SET {key}=NULL WHERE id=?"), [id])?;
            }
            Ok(())
        })
        .await;
    match result {
        Ok(()) => {
            let location = c.url_for(&campfire_routes::user_profile());
            c.redirect_to_with(
                &location,
                Redirect {
                    notice: Some("✓".into()),
                    ..Redirect::default()
                },
            )
        }
        Err(campfire_db::Error::RecordInvalid(errors)) => {
            super::profiles::render_settings(c, StatusCode::UNPROCESSABLE_ENTITY, submitted, errors)
                .await
        }
        Err(error) => Err(Error::internal(error)),
    }
}

pub fn keyword_lines(value: &Param) -> Result<Vec<String>> {
    match value {
        Param::Null => Ok(Vec::new()),
        Param::Array(values) => Ok(values.iter().map(ruby_to_s).collect()),
        value => Ok(vec![ruby_to_s(value)]),
    }
}

pub(super) fn ruby_to_s(value: &Param) -> String {
    value.to_s().unwrap_or_else(|| ruby_inspect(value))
}
fn ruby_inspect(value: &Param) -> String {
    match value {
        Param::Null => "nil".into(),
        Param::Str(s) => {
            let mut result = String::from("\"");
            let mut chars = s.chars().peekable();
            while let Some(c) = chars.next() {
                match c {
                    '\\' => result.push_str("\\\\"),
                    '"' => result.push_str("\\\""),
                    '\u{7}' => result.push_str("\\a"),
                    '\u{8}' => result.push_str("\\b"),
                    '\t' => result.push_str("\\t"),
                    '\n' => result.push_str("\\n"),
                    '\u{b}' => result.push_str("\\v"),
                    '\u{c}' => result.push_str("\\f"),
                    '\r' => result.push_str("\\r"),
                    '\u{1b}' => result.push_str("\\e"),
                    '#' if chars.peek().is_some_and(|c| matches!(c, '{' | '@' | '$')) => {
                        result.push_str("\\#")
                    }
                    c if c.is_control() || matches!(c, '\u{2028}' | '\u{2029}') => {
                        result.push_str(&format!("\\u{:04X}", c as u32))
                    }
                    c => result.push(c),
                }
            }
            result.push('"');
            result
        }
        Param::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(ruby_inspect)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Param::Hash(values) => format!(
            "{{{}}}",
            values
                .iter()
                .map(|(key, value)| format!(
                    "{} => {}",
                    ruby_inspect(&Param::Str(key.clone())),
                    ruby_inspect(value)
                ))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        value => value.to_s().unwrap_or_default(),
    }
}
