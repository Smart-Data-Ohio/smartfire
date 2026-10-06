//! Users::PresencesController: a read-only, batched workspace presence/status poll.
use crate::app::AppCtx;
use crate::concerns::{self, Before};
use campfire_db::models::workspace_presence_lease::Presence;
use campfire_db::{UserStatusSettings, WorkspacePresenceLease};
use campfire_kit::{Ctx, Error, Result, StatusCode};
use serde_json::{Value, json};

pub async fn show(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let value = c.param("ids").map(|p| p.to_json()).unwrap_or(Value::Null);
    let values = match &value {
        Value::Array(items) => items.as_slice(),
        Value::Null => &[],
        _ => std::slice::from_ref(&value),
    };
    // Ruby applies first(100) before SQL coercion/deduplication. Huge valid integers still
    // consume a slot even though SQLite cannot have a matching primary key.
    let ids: Vec<i64> = values
        .iter()
        .filter_map(integer_id)
        .take(100)
        .flatten()
        .collect();
    let now = c.app().db.env().now();
    let presences = c.app().db.read(move |conn| {
        let mut users = UserStatusSettings::for_ids(conn, &ids)?.into_values().filter(UserStatusSettings::active_human).collect::<Vec<_>>();
        // Rails' primary-key IN query returns users in id order, independent of requested order.
        users.sort_by_key(|user| user.user.id);
        let ids: Vec<i64> = users.iter().map(|u| u.user.id).collect();
        let states = WorkspacePresenceLease::presence_by_user_id(conn, &ids, now)?;
        Ok(users.into_iter().map(|user| json!({"id":user.user.id,
            "presence":user.effective_presence(states.get(&user.user.id).copied().unwrap_or(Presence::Offline)),
            "status":user.status_text_display(now)})).collect::<Vec<_>>())
    }).await.map_err(Error::internal)?;
    c.json(StatusCode::OK, &json!({"presences":presences}))
}

/// Integer(value, exception: false), including Ruby's base autodetection. Some(None) is a
/// valid integer outside SQLite's signed-64-bit key range; None is invalid input.
pub fn integer_id(value: &Value) -> Option<Option<i64>> {
    if let Value::Number(n) = value {
        if let Some(n) = n.as_i64() {
            return Some(Some(n));
        }
        if n.as_u64().is_some() {
            return Some(None);
        }
        let n = n.as_f64()?.trunc();
        return Some((n >= i64::MIN as f64 && n < -(i64::MIN as f64)).then_some(n as i64));
    }
    let text = value
        .as_str()?
        .trim_matches([' ', '\t', '\n', '\r', '\x0b', '\x0c']);
    let (negative, digits) = match text.as_bytes().first()? {
        b'-' => (true, &text[1..]),
        b'+' => (false, &text[1..]),
        _ => (false, text),
    };
    let (radix, digits) = if digits.len() > 1 && digits.starts_with('0') {
        match digits.as_bytes()[1] {
            b'x' | b'X' => (16, &digits[2..]),
            b'b' | b'B' => (2, &digits[2..]),
            b'o' | b'O' => (8, &digits[2..]),
            b'd' | b'D' => (10, &digits[2..]),
            _ => (8, digits),
        }
    } else {
        (10, digits)
    };
    if digits.is_empty()
        || digits.starts_with('_')
        || digits.ends_with('_')
        || digits.contains("__")
        || !digits
            .chars()
            .all(|c| c == '_' || c.is_ascii() && c.is_digit(radix))
    {
        return None;
    }
    let digits = digits.replace('_', "");
    Some(
        i128::from_str_radix(&digits, radix)
            .ok()
            .and_then(|n| i64::try_from(if negative { -n } else { n }).ok()),
    )
}
