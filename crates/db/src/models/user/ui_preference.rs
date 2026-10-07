//! Which UI a person uses: the classic pages, or the new SPA under `/app/` (`crates/spa`).
//!
//! Stored without a migration, as a boolean `next_ui` key in the user's preferences JSON
//! (`users.inbox_preferences`, the one per-user JSON preferences column): `true` is the new UI,
//! `false` is classic, and no key is no choice yet (the deployment's `SPA_DEFAULT` decides).
//! It's a boolean because the profile validation accepts only booleans there, and every reader of
//! that column looks up its own keys by name, so a key it doesn't know changes nothing: the
//! classic profile form permits and merges only the inbox keys, so saving it keeps this one.
//!
//! Writing it touches nothing else: not `updated_at` (which would change the avatar URL and bust
//! its caches), and no broadcast, since only the person's own navigation reads it.

use rusqlite::{Connection, OptionalExtension};
use serde_json::Value;

use crate::{Result, Tx};

/// The key in `users.inbox_preferences`.
pub const KEY: &str = "next_ui";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiPreference {
    Classic,
    Next,
}

impl UiPreference {
    /// `classic` or `next`, as forms and the SPA spell it.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "classic" => Some(Self::Classic),
            "next" => Some(Self::Next),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::Next => "next",
        }
    }

    /// The UI a person with `stored` gets, `next` being the deployment's default for people who
    /// haven't chosen.
    pub fn effective(stored: Option<Self>, default_next: bool) -> Self {
        stored.unwrap_or(if default_next {
            Self::Next
        } else {
            Self::Classic
        })
    }
}

/// The person's choice, if they've made one. A missing user reads as none.
pub fn stored(conn: &Connection, user: i64) -> Result<Option<UiPreference>> {
    let raw: Option<Option<String>> = conn
        .query_row(
            "SELECT inbox_preferences FROM users WHERE id=?",
            [user],
            |row| row.get(0),
        )
        .optional()?;
    let preferences = raw
        .flatten()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok());
    Ok(
        match preferences
            .as_ref()
            .and_then(|preferences| preferences.get(KEY))
        {
            Some(Value::Bool(true)) => Some(UiPreference::Next),
            Some(Value::Bool(false)) => Some(UiPreference::Classic),
            _ => None,
        },
    )
}

/// Records the person's choice, keeping every other key. Preferences that aren't a JSON object
/// (never written by the app) start over as one. A missing user (deleted meanwhile) is a no-op.
pub fn store(tx: &Tx<'_>, user: i64, preference: UiPreference) -> Result<()> {
    let Some(raw) = tx
        .conn()
        .query_row(
            "SELECT inbox_preferences FROM users WHERE id=?",
            [user],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?
    else {
        return Ok(());
    };
    let mut preferences = raw
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    preferences.insert(KEY.into(), Value::Bool(preference == UiPreference::Next));
    let json = serde_json::to_string(&Value::Object(preferences)).expect("JSON preferences");
    tx.conn().execute(
        "UPDATE users SET inbox_preferences=? WHERE id=?",
        rusqlite::params![json, user],
    )?;
    Ok(())
}
