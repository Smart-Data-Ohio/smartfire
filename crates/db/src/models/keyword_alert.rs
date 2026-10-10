//! `app/models/keyword_alert.rb` and `app/models/notifications/keyword_matcher.rb`.

use rusqlite::{Connection, Row, params};

use crate::error::OptionalExt;
use crate::sql::{self, CachedStatements, query_all, query_one};
use crate::{Errors, Result, Timestamp, Tx, User};

pub const MAX_PER_USER: i64 = 20;
pub const PHRASE_LIMIT: usize = 80;

#[derive(Debug, Clone, PartialEq)]
pub struct KeywordAlert {
    pub id: i64,
    pub user_id: i64,
    pub phrase: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

// Ruby String#strip and /\s/ use ASCII whitespace here, rather than Rust's Unicode trim.
fn strip(text: &str) -> &str {
    text.trim_matches(|c| matches!(c, '\0' | ' ' | '\t' | '\n' | '\r' | '\x0b' | '\x0c'))
}

pub fn normalize(phrase: &str) -> String {
    strip(phrase)
        .split([' ', '\t', '\n', '\r', '\x0b', '\x0c'])
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

impl KeywordAlert {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            user_id: row.get("user_id")?,
            phrase: row.get("phrase")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        query_one(
            conn,
            "SELECT * FROM keyword_alerts WHERE id = ?",
            [id],
            Self::from_row,
        )?
        .or_not_found("KeywordAlert")
    }

    pub fn for_user(conn: &Connection, user_id: i64) -> Result<Vec<Self>> {
        query_all(
            conn,
            "SELECT * FROM keyword_alerts WHERE user_id = ? ORDER BY id",
            [user_id],
            Self::from_row,
        )
    }

    fn validate(conn: &Connection, user_id: i64, id: Option<i64>, phrase: &str) -> Result<()> {
        let mut errors = Errors::default();
        if User::find_by_id(conn, user_id)?.is_none() {
            errors.add("user", "must exist");
        }
        if phrase.trim().is_empty() {
            errors.add("phrase", "can't be blank");
        }
        if phrase.chars().count() > PHRASE_LIMIT {
            errors.add("phrase", "is too long (maximum is 80 characters)");
        }
        // SQLite LOWER intentionally matches Rails' SQL comparison (ASCII-only case folding).
        if !phrase.is_empty()
            && sql::exists(
                conn,
                "SELECT 1 FROM keyword_alerts WHERE user_id = ? AND LOWER(phrase) = ? AND (? IS NULL OR id != ?) LIMIT 1",
                params![user_id, rails_compat::unicode::downcase(phrase), id, id],
            )?
        {
            errors.add("phrase", "is already in your list");
        }
        if id.is_none()
            && sql::count(
                conn,
                "SELECT COUNT(*) FROM keyword_alerts WHERE user_id = ?",
                [user_id],
            )? >= MAX_PER_USER
        {
            errors.add("base", "You can watch at most 20 keywords");
        }
        errors.into_result()
    }

    pub fn create(tx: &mut Tx<'_>, user_id: i64, phrase: &str) -> Result<Self> {
        let phrase = normalize(phrase);
        Self::validate(tx.conn(), user_id, None, &phrase)?;
        let id = tx.conn().query_row_cached(
            "INSERT INTO keyword_alerts (user_id, phrase, created_at, updated_at) VALUES (?, ?, ?, ?) RETURNING id",
            params![user_id, phrase, tx.now(), tx.now()], |r| r.get(0))?;
        super::user::profile_settings::bump_revision(tx, user_id)?;
        Self::find(tx.conn(), id)
    }

    pub fn update(&mut self, tx: &mut Tx<'_>, phrase: &str) -> Result<()> {
        let phrase = normalize(phrase);
        Self::validate(tx.conn(), self.user_id, Some(self.id), &phrase)?;
        if self.phrase != phrase {
            tx.conn().execute_cached(
                "UPDATE keyword_alerts SET phrase = ?, updated_at = ? WHERE id = ?",
                params![phrase, tx.now(), self.id],
            )?;
            super::user::profile_settings::bump_revision(tx, self.user_id)?;
            *self = Self::find(tx.conn(), self.id)?;
        }
        Ok(())
    }

    pub fn destroy(&self, tx: &Tx<'_>) -> Result<()> {
        let removed = tx.conn()
            .execute_cached("DELETE FROM keyword_alerts WHERE id = ?", [self.id])?;
        if removed != 0 {
            super::user::profile_settings::bump_revision(tx, self.user_id)?;
        }
        Ok(())
    }
}

/// Pure matcher, preserving Ruby hash/phrase insertion order and Set deduplication. Each
/// distinct phrase matches independently, so a longer match never consumes a shorter one.
/// Typed inputs represent the recorder's user-id => phrase-array mapping.
pub fn matching_user_ids(phrases_by_user: &[(i64, Vec<String>)], text: &str) -> Result<Vec<i64>> {
    let mut phrases: Vec<(String, Vec<i64>)> = Vec::new();
    for (user_id, user_phrases) in phrases_by_user {
        for phrase in user_phrases {
            let phrase = rails_compat::unicode::downcase(strip(phrase));
            if phrase.trim().is_empty() {
                continue;
            }
            if let Some((_, users)) = phrases.iter_mut().find(|(p, _)| *p == phrase) {
                users.push(*user_id);
            } else {
                phrases.push((phrase, vec![*user_id]));
            }
        }
    }
    let mut matched = Vec::new();
    for (phrase, users) in phrases {
        if rails_compat::keyword_regex::is_match(&phrase, text).map_err(crate::Error::Other)? {
            for user in users {
                if !matched.contains(&user) {
                    matched.push(user);
                }
            }
        }
    }
    Ok(matched)
}
