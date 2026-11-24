//! Confirmed addresses attached to a user. The primary address lives on users.
use rusqlite::{Connection, Row, params};

use crate::sql::{CachedStatements, query_all, query_one};
use crate::{Error, Result, Timestamp, Tx, User};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmailAddress {
    pub id: i64,
    pub email_address: String,
    pub created_at: Timestamp,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Confirmation {
    Added,
    AlreadyLinked,
    Taken,
}

impl EmailAddress {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            email_address: row.get("email_address")?,
            created_at: row.get("created_at")?,
        })
    }

    pub fn for_user(conn: &Connection, user_id: i64) -> Result<Vec<Self>> {
        query_all(
            conn,
            "SELECT * FROM user_email_addresses WHERE user_id = ? ORDER BY id",
            [user_id],
            Self::from_row,
        )
    }

    pub fn by_address(conn: &Connection, address: &str) -> Result<Option<(i64, Self)>> {
        let normalized = normalize(address);
        let mut query = conn.prepare_cached("SELECT * FROM user_email_addresses ORDER BY id")?;
        let mut rows = query.query([])?;
        while let Some(row) = rows.next()? {
            let email: String = row.get("email_address")?;
            if normalize(&email) == normalized {
                return Ok(Some((row.get("user_id")?, Self::from_row(row)?)));
            }
        }
        Ok(None)
    }

    pub fn find_owned(conn: &Connection, user_id: i64, id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM user_email_addresses WHERE user_id = ? AND id = ?",
            params![user_id, id],
            Self::from_row,
        )
    }

    pub fn remove(tx: &Tx<'_>, user_id: i64, id: i64) -> Result<Option<Self>> {
        query_one(
            tx.conn(),
            "DELETE FROM user_email_addresses WHERE user_id = ? AND id = ? RETURNING *",
            params![user_id, id],
            Self::from_row,
        )
    }

    /// A primary change can request a re-link at the same instant the old address is released.
    pub fn released_after(
        conn: &Connection,
        user_id: i64,
        address: &str,
        since: Timestamp,
    ) -> Result<bool> {
        let normalized = normalize(address);
        let mut query = conn.prepare_cached(
            "SELECT CASE action
                      WHEN 'user.email_address.remove' THEN json_extract(details, '$.email_address')
                      WHEN 'user.email.change' THEN json_extract(details, '$.email_address.before')
                    END
             FROM audit_logs
             WHERE target_type = 'User' AND target_id = ?1
               AND action IN ('user.email_address.remove', 'user.email.change') AND created_at > ?2",
        )?;
        let mut rows = query.query(params![user_id, since])?;
        while let Some(row) = rows.next()? {
            let removed: Option<String> = row.get(0)?;
            if removed.is_some_and(|address| normalize(&address) == normalized) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn is_available(conn: &Connection, address: &str) -> Result<bool> {
        Ok(!primary_conflict_for_secondary(conn, address, None)?
            && Self::by_address(conn, address)?.is_none())
    }

    /// The immediate writer transaction serializes claims against primary and secondary writes.
    pub fn confirm(tx: &Tx<'_>, user: &User, address: &str) -> Result<Confirmation> {
        let address = address.trim();
        if primary_conflict_for_secondary(tx.conn(), address, Some(user.id))? {
            return Ok(Confirmation::Taken);
        }
        if let Some((owner, _)) = Self::by_address(tx.conn(), address)? {
            return Ok(if owner == user.id {
                Confirmation::AlreadyLinked
            } else {
                Confirmation::Taken
            });
        }
        if user
            .email_address
            .as_deref()
            .is_some_and(|primary| normalize(primary) == normalize(address))
        {
            return Ok(Confirmation::AlreadyLinked);
        }
        let now = tx.now();
        tx.conn().execute_cached(
            "INSERT INTO user_email_addresses (user_id, email_address, created_at, updated_at)
             VALUES (?, ?, ?, ?)",
            params![user.id, address, now, now],
        )?;
        Ok(Confirmation::Added)
    }
}

/// Primary writes retain SQLite's ASCII comparison between primary addresses.
pub(super) fn primary_conflict(
    conn: &Connection,
    address: &str,
    except: Option<i64>,
) -> Result<bool> {
    // SQLite's trim defaults to ASCII space. Use the same whitespace as str::trim.
    const WHITESPACE: &str = concat!(
        "\t\n\x0b\x0c\r \u{85}\u{a0}\u{1680}",
        "\u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\u{200a}",
        "\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}",
    );
    Ok(conn.query_row_cached(
        "SELECT EXISTS (
            SELECT 1 FROM users WHERE trim(email_address, ?3) = ?1 COLLATE NOCASE
            AND (?2 IS NULL OR id != ?2)
        )",
        params![address.trim(), except, WHITESPACE],
        |row| row.get(0),
    )?)
}

/// Secondary claims must also be unambiguous under password-reset matching.
fn primary_conflict_for_secondary(
    conn: &Connection,
    address: &str,
    except: Option<i64>,
) -> Result<bool> {
    let normalized = normalize(address);
    let mut query = conn.prepare_cached(
        "SELECT email_address FROM users
         WHERE email_address IS NOT NULL AND (?1 IS NULL OR id != ?1)",
    )?;
    let mut rows = query.query([except])?;
    while let Some(row) = rows.next()? {
        if normalize(&row.get::<_, String>(0)?) == normalized {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn invalid_primary(message: &str) -> Error {
    let mut errors = crate::Errors::default();
    errors.add("email_address", message);
    Error::RecordInvalid(errors)
}

/// Returns the linked row id if the submitted address is already a secondary of this user.
pub(super) fn check_primary(tx: &Tx<'_>, owner: Option<i64>, address: &str) -> Result<Option<i64>> {
    let linked = EmailAddress::by_address(tx.conn(), address)?;
    if primary_conflict(tx.conn(), address, owner)?
        || linked.as_ref().is_some_and(|(id, _)| Some(*id) != owner)
    {
        // The existing callers handle the same uniqueness error as the primary column's index.
        return Err(Error::Sqlite(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE),
            Some("UNIQUE constraint failed: users.email_address".into()),
        )));
    }
    Ok(linked.map(|(_, secondary)| secondary.id))
}

/// Address comparisons ignore Unicode whitespace and the NUL padding stripped by Rails.
pub fn trim(address: &str) -> &str {
    address.trim_matches(|c: char| c.is_whitespace() || c == '\0')
}

/// Shared by address claims, password sign-in, reset delivery, and the per-inbox request limit.
pub fn normalize(address: &str) -> String {
    let address = trim(address);
    if address.is_ascii() {
        address.to_ascii_lowercase()
    } else {
        rails_compat::unicode::fold(address)
    }
}

/// Load both address sources in one statement. JSON keeps the number of binds constant for
/// Slack workspaces of any size. The two IN clauses let SQLite use each address index where
/// possible.
pub(super) fn matches(
    conn: &Connection,
    addresses: &[&str],
) -> Result<std::collections::HashMap<String, Vec<User>>> {
    use std::collections::HashMap;

    if addresses.is_empty() {
        return Ok(HashMap::new());
    }
    // Keep SQLite's ASCII comparison for provider lookups.
    let secondary_keys: Vec<_> = addresses
        .iter()
        .map(|address| address.trim().to_ascii_lowercase())
        .collect();
    let sql = "SELECT users.*, LOWER(users.email_address) AS matched_address, 0 AS secondary
         FROM users WHERE LOWER(users.email_address) IN (SELECT value FROM json_each(?1))
         UNION ALL
         SELECT users.*, LOWER(link.email_address) AS matched_address, 1 AS secondary
         FROM user_email_addresses link JOIN users ON users.id = link.user_id
         WHERE link.email_address COLLATE NOCASE IN (SELECT value FROM json_each(?2))
         ORDER BY secondary, id";
    let rows = query_all(
        conn,
        sql,
        params![
            serde_json::json!(addresses).to_string(),
            serde_json::json!(secondary_keys).to_string()
        ],
        |row| {
            Ok((
                row.get::<_, bool>("secondary")?,
                row.get::<_, String>("matched_address")?,
                User::from_row(row)?,
            ))
        },
    )?;
    let mut primary_matches = HashMap::<_, Vec<_>>::new();
    let mut secondary_matches = HashMap::new();
    for (secondary, address, user) in rows {
        if secondary {
            secondary_matches.insert(address, user);
        } else {
            primary_matches.entry(address).or_default().push(user);
        }
    }
    for (&address, normalized) in addresses.iter().zip(secondary_keys) {
        if let Some(user) = secondary_matches.get(&normalized) {
            primary_matches
                .entry(address.to_owned())
                .or_insert_with(|| vec![user.clone()]);
        }
    }
    Ok(primary_matches)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NewUser, UserChanges, tests::TestDb};

    fn create(tx: &mut Tx<'_>, address: &str) -> Result<User> {
        User::create(
            tx,
            NewUser {
                name: "Member".into(),
                email_address: Some(address.into()),
                ..Default::default()
            },
        )
    }

    #[test]
    fn uniqueness_covers_both_sources_in_both_directions() {
        let db = TestDb::new();
        db.write(|tx| {
            let first = create(tx, "\u{a0} Primary@Example.test\t")?;
            let mut second = create(tx, "other@example.test")?;
            assert_eq!(
                EmailAddress::confirm(tx, &second, "PRIMARY@example.test")?,
                Confirmation::Taken
            );
            assert!(
                create(tx, "pRIMARY@example.test")
                    .unwrap_err()
                    .is_record_not_unique()
            );
            assert_eq!(
                EmailAddress::confirm(tx, &first, " Secondary@Example.test ")?,
                Confirmation::Added
            );
            assert_eq!(
                EmailAddress::confirm(tx, &second, "secondary@example.test")?,
                Confirmation::Taken
            );
            assert!(
                create(tx, "secondary@example.test")
                    .unwrap_err()
                    .is_record_not_unique()
            );
            let err = second
                .update(
                    tx,
                    UserChanges {
                        email_address: Some(Some("SECONDARY@EXAMPLE.TEST".into())),
                        ..Default::default()
                    },
                )
                .unwrap_err();
            assert!(matches!(err, Error::RecordInvalid(errors)
                if errors.on("email_address") == ["is already taken"]));
            let snapshot = crate::Snapshot::of_write(tx)?;
            for address in ["secondary@example.test", "primary@example.test"] {
                assert_eq!(
                    User::find_active_by_sign_in_address(&snapshot, address)?
                        .unwrap()
                        .user
                        .id,
                    first.id
                );
            }
            Ok(())
        });
    }

    #[test]
    fn update_compares_email_against_the_reloaded_user() {
        let db = TestDb::new();
        db.write(|tx| {
            let original = "original@example.test";
            let changed = "new@example.test";
            let make_change = |address: &str| UserChanges {
                email_address: Some(Some(address.into())),
                ..Default::default()
            };
            let mut stale = create(tx, original)?;
            let mut fresh = stale.clone();
            fresh.update(tx, make_change(changed))?;
            stale.update(tx, make_change(original))?;
            assert_eq!(
                User::find(tx.conn(), stale.id)?.email_address.as_deref(),
                Some(original)
            );
            Ok(())
        });
    }

    #[test]
    fn deactivation_releases_secondary_addresses() {
        let db = TestDb::new();
        db.write(|tx| {
            let mut first = create(tx, "first@example.test")?;
            let second = create(tx, "second@example.test")?;
            assert_eq!(
                EmailAddress::confirm(tx, &first, "linked@example.test")?,
                Confirmation::Added
            );
            first.deactivate(tx)?;
            assert!(EmailAddress::for_user(tx.conn(), first.id)?.is_empty());
            assert_eq!(
                EmailAddress::confirm(tx, &second, "linked@example.test")?,
                Confirmation::Added
            );
            Ok(())
        });
    }
}
