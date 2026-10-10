//! `reference/app/models/account.rb` and `account/joinable.rb`.

use rusqlite::{Connection, Row, params};
use serde_json::{Map, Value};

use crate::database::Tx;
use crate::error::{Error, OptionalExt, Result};
use crate::sql::{self, CachedStatements, query_one};
use crate::time::Timestamp;

#[derive(Debug, Clone, PartialEq)]
pub struct Account {
    pub id: i64,
    pub name: String,
    pub join_code: String,
    pub custom_styles: Option<String>,
    /// The raw `settings` JSON column; read it through [`Account::settings`].
    pub settings_json: Option<String>,
    pub singleton_guard: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// `has_json :settings, restrict_room_creation_to_administrators: false`
/// (`ActiveModel::SchematizedJson`): the stored hash with the schema defaults merged in.
#[derive(Debug, Clone, PartialEq)]
pub struct AccountSettings {
    data: Map<String, Value>,
}

const RESTRICT_ROOM_CREATION: &str = "restrict_room_creation_to_administrators";
const UPLOAD_LIMIT: &str = "upload_limit_bytes";
const DESCRIPTION: &str = "description";
const VANITY_SLUG: &str = "vanity_slug";
const ANIMATED_EMOJI_LIMIT: &str = "animated_emoji_limit";
pub const DEFAULT_ANIMATED_EMOJI_LIMIT: i64 = 250;
pub const DEFAULT_UPLOAD_LIMIT_BYTES: i64 = 100 * 1024 * 1024;
/// Largest integer the SPA can represent exactly.
pub const MAX_UPLOAD_LIMIT_BYTES: i64 = 9_007_199_254_740_991;

pub const DESCRIPTION_MAX_CHARS: usize = 300;
pub const RESERVED_VANITY_SLUGS: &[&str] = &[
    "app",
    "api",
    "rooms",
    "users",
    "session",
    "sessions",
    "join",
    "assets",
    "admin",
    "account",
    "accounts",
    "settings",
    "about",
    "privacy",
    "terms",
    "up",
    "health",
    "cable",
    "huddle",
    "uploads",
    "files",
    "blobs",
    "storage",
    "messages",
    "threads",
    "events",
    "boards",
    "bots",
    "agents",
    "integrations",
    "first_run",
    "first-run",
    "sudo",
    "two_factor",
    "two-factor",
    "two_factor_setup",
    "two-factor-setup",
    "pwa",
    "manifest",
    "service-worker",
    "favicon",
    "robots",
    "rails",
];

pub fn validate_description(value: &str) -> Result<&str> {
    let value = value.trim();
    if value.chars().count() > DESCRIPTION_MAX_CHARS {
        return Err(Error::Other("must be 300 characters or fewer".into()));
    }
    if value.contains(['<', '>'])
        || value
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
    {
        return Err(Error::Other(
            "must be plain text without HTML or control characters".into(),
        ));
    }
    Ok(value)
}

pub fn validate_vanity_slug(value: &str) -> Result<&str> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(value);
    }
    if !(3..=32).contains(&value.len())
        || value.starts_with('-')
        || value.ends_with('-')
        || !value
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
    {
        return Err(Error::Other("must be 3–32 lowercase letters, digits or hyphens, without a leading or trailing hyphen".into()));
    }
    if RESERVED_VANITY_SLUGS.contains(&value) {
        return Err(Error::Other("is reserved; choose another slug".into()));
    }
    if looks_like_join_code(value) {
        return Err(Error::Other("can't look like an invite code".into()));
    }
    Ok(value)
}

/// `validate_vanity_slug`, plus a check against the workspace's current join code, so a
/// slug can never become a second way into `/join/:code` after a reset.
pub fn validate_vanity_slug_for<'a>(value: &'a str, join_code: &str) -> Result<&'a str> {
    let value = validate_vanity_slug(value)?;
    if !value.is_empty() && value == join_code {
        return Err(Error::Other("can't look like an invite code".into()));
    }
    Ok(value)
}

/// Whether a value has the shape of `generate_join_code`: 4-4-4 alphanumeric groups.
fn looks_like_join_code(value: &str) -> bool {
    value.len() == 14
        && value.bytes().enumerate().all(|(i, c)| {
            if i == 4 || i == 9 {
                c == b'-'
            } else {
                c.is_ascii_alphanumeric()
            }
        })
}

impl AccountSettings {
    fn from_column(raw: Option<&str>) -> Self {
        let mut data = raw
            .and_then(|r| serde_json::from_str::<Map<String, Value>>(r).ok())
            .unwrap_or_default();
        data.entry(RESTRICT_ROOM_CREATION)
            .or_insert(Value::Bool(false));
        Self { data }
    }

    pub fn description(&self) -> &str {
        self.data
            .get(DESCRIPTION)
            .and_then(Value::as_str)
            .unwrap_or("")
    }

    pub fn vanity_slug(&self) -> Option<&str> {
        self.data
            .get(VANITY_SLUG)
            .and_then(Value::as_str)
            .filter(|slug| !slug.is_empty())
    }

    /// `restrict_room_creation_to_administrators?`: `present?` of the stored value.
    pub fn restrict_room_creation_to_administrators(&self) -> bool {
        present(self.data.get(RESTRICT_ROOM_CREATION))
    }

    pub fn upload_limit_bytes(&self) -> i64 {
        self.data
            .get(UPLOAD_LIMIT)
            .and_then(Value::as_i64)
            .filter(|bytes| (1..=MAX_UPLOAD_LIMIT_BYTES).contains(bytes))
            .unwrap_or(DEFAULT_UPLOAD_LIMIT_BYTES)
    }

    pub fn animated_emoji_limit(&self) -> i64 {
        self.data
            .get(ANIMATED_EMOJI_LIMIT)
            .and_then(Value::as_i64)
            .filter(|limit| (0..=MAX_UPLOAD_LIMIT_BYTES).contains(limit))
            .unwrap_or(DEFAULT_ANIMATED_EMOJI_LIMIT)
    }

    /// `restrict_room_creation_to_administrators = value`, cast as a boolean.
    pub fn set_restrict_room_creation_to_administrators(&mut self, value: &str) {
        let cast = cast_boolean(value).map(Value::Bool).unwrap_or(Value::Null);
        self.data.insert(RESTRICT_ROOM_CREATION.into(), cast);
    }

    /// `assign_data_with_type_casting`: every key must be in the schema.
    pub fn assign(&mut self, values: &[(&str, &str)]) -> Result<()> {
        for (key, value) in values {
            match *key {
                RESTRICT_ROOM_CREATION => self.set_restrict_room_creation_to_administrators(value),
                UPLOAD_LIMIT => {
                    let bytes = value
                        .parse::<i64>()
                        .ok()
                        .filter(|bytes| (1..=MAX_UPLOAD_LIMIT_BYTES).contains(bytes))
                        .ok_or_else(|| {
                            Error::Other("upload limit must be a positive safe integer".into())
                        })?;
                    self.data.insert(UPLOAD_LIMIT.into(), Value::from(bytes));
                }
                ANIMATED_EMOJI_LIMIT => {
                    let limit = value
                        .parse::<i64>()
                        .ok()
                        .filter(|limit| (0..=MAX_UPLOAD_LIMIT_BYTES).contains(limit))
                        .ok_or_else(|| {
                            Error::Other(
                                "animated emoji limit must be a non-negative safe integer".into(),
                            )
                        })?;
                    self.data
                        .insert(ANIMATED_EMOJI_LIMIT.into(), Value::from(limit));
                }
                DESCRIPTION => {
                    self.data.insert(DESCRIPTION.into(), Value::from(validate_description(value)?));
                }
                VANITY_SLUG => {
                    self.data.insert(VANITY_SLUG.into(), Value::from(validate_vanity_slug(value)?));
                }
                other => {
                    return Err(Error::Other(format!(
                        "undefined method '{other}=' for account settings"
                    )));
                }
            }
        }
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.data).expect("settings encode")
    }
}

/// `ActiveModel::Type::Boolean#cast` of a form value: blank is nil, the `FALSE_VALUES` are
/// false, anything else is true.
pub fn cast_boolean(value: &str) -> Option<bool> {
    const FALSE_VALUES: &[&str] = &["0", "f", "F", "false", "FALSE", "off", "OFF"];
    if value.is_empty() {
        None
    } else {
        Some(!FALSE_VALUES.contains(&value))
    }
}

fn present(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) | Some(Value::Bool(false)) => false,
        Some(Value::String(s)) => !s.trim().is_empty(),
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(o)) => !o.is_empty(),
        Some(_) => true,
    }
}

impl Account {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            join_code: row.get("join_code")?,
            custom_styles: row.get("custom_styles")?,
            settings_json: row.get("settings")?,
            singleton_guard: row.get("singleton_guard")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    /// `Account.first` (`Current.account`).
    pub fn first(conn: &Connection) -> Result<Option<Self>> {
        query_one(
            conn,
            r#"SELECT * FROM "accounts" ORDER BY "accounts"."id" ASC LIMIT 1"#,
            [],
            Self::from_row,
        )
    }

    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        query_one(
            conn,
            r#"SELECT * FROM "accounts" WHERE "accounts"."id" = ? LIMIT 1"#,
            [id],
            Self::from_row,
        )?
        .or_not_found("Account")
    }

    pub fn count(conn: &Connection) -> Result<i64> {
        sql::count(conn, r#"SELECT COUNT(*) FROM "accounts""#, [])
    }

    pub fn settings(&self) -> AccountSettings {
        AccountSettings::from_column(self.settings_json.as_deref())
    }

    /// `Account.create!(name:)`: a fresh join code, and the settings defaults written out.
    pub fn create(tx: &mut Tx<'_>, name: &str) -> Result<Self> {
        let now = tx.now();
        let join_code = generate_join_code();
        let settings = AccountSettings::from_column(None).to_json();
        let id: i64 = tx.conn().query_row_cached(
            r#"INSERT INTO "accounts" ("created_at", "custom_styles", "join_code", "name", "settings", "singleton_guard", "updated_at") VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING "id""#,
            params![now, None::<String>, join_code, name, settings, 0, now],
            |r| r.get(0),
        )?;
        Self::find(tx.conn(), id)
    }

    /// `reset_join_code`
    pub fn reset_join_code(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        let join_code = generate_join_code();
        let now = tx.now();
        tx.conn().execute_cached(
            r#"UPDATE "accounts" SET "join_code" = ?, "updated_at" = ? WHERE "accounts"."id" = ?"#,
            params![join_code, now, self.id],
        )?;
        self.join_code = join_code;
        self.updated_at = now;
        Ok(())
    }

    /// `update!(name:, custom_styles:, settings:)`. Only changed attributes are written; the
    /// settings column is written when its (defaulted) hash changes.
    pub fn update(
        &mut self,
        tx: &mut Tx<'_>,
        name: Option<&str>,
        custom_styles: Option<Option<&str>>,
        settings: Option<&[(&str, &str)]>,
    ) -> Result<()> {
        let mut sets: Vec<(&str, Box<dyn rusqlite::ToSql>)> = Vec::new();
        if let Some(name) = name.filter(|n| *n != self.name) {
            self.name = name.into();
            sets.push(("name", Box::new(name.to_string())));
        }
        if let Some(styles) = custom_styles
            .map(|s| s.map(str::to_string))
            .filter(|s| *s != self.custom_styles)
        {
            self.custom_styles = styles.clone();
            sets.push(("custom_styles", Box::new(styles)));
        }
        if let Some(values) = settings {
            let original = self.settings();
            let mut updated = original.clone();
            updated.assign(values)?;
            if self.settings_json.is_none() || updated != original {
                let json = updated.to_json();
                self.settings_json = Some(json.clone());
                sets.push(("settings", Box::new(json)));
            }
        }
        if sets.is_empty() {
            return Ok(());
        }
        let now = tx.now();
        self.updated_at = now;
        sets.push(("updated_at", Box::new(now)));
        let assignments: Vec<String> = sets.iter().map(|(c, _)| format!(r#""{c}" = ?"#)).collect();
        let sql = format!(
            r#"UPDATE "accounts" SET {} WHERE "accounts"."id" = ?"#,
            assignments.join(", ")
        );
        let mut values: Vec<&dyn rusqlite::ToSql> = sets.iter().map(|(_, v)| v.as_ref()).collect();
        values.push(&self.id);
        tx.conn().execute_cached(&sql, values.as_slice())?;
        Ok(())
    }

    pub fn reload(&mut self, conn: &Connection) -> Result<()> {
        *self = Self::find(conn, self.id)?;
        Ok(())
    }
}

/// `SecureRandom.alphanumeric(12).scan(/.{4}/).join("-")`
pub fn generate_join_code() -> String {
    let code = sql::alphanumeric(12);
    format!("{}-{}-{}", &code[0..4], &code[4..8], &code[8..12])
}
