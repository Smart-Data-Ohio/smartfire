//! The schema exactly as `bin/rails db:prepare` creates it on a fresh database, and the
//! connection settings from `reference/config/database.yml` plus the sqlite3 adapter's
//! `DEFAULT_PRAGMAS`.
//!
//! Until cutover the Rails image owns the schema (decisions.md, decision 1): this crate loads it
//! into an empty database and otherwise only checks that a database was migrated to exactly the
//! schema it was built against. It never creates or alters anything in an existing database.
//!
//! `schema.sql`, `schema_migrations.txt`, `schema_sha1.txt` and `schema_sequences.txt` are
//! generated from the reference app by `reference-tools/db/regenerate-schema.sh`, never edited
//! by hand: `schema.sql` is the
//! `sqlite_master` of a database the reference app created with `db:prepare` (loading
//! `reference/db/schema.rb`), minus the objects SQLite derives on its own (FTS5 shadow tables,
//! `sqlite_sequence`, autoindexes). A fresh `db:prepare` loads `schema.rb`, so columns come out
//! in alphabetical order; databases that were migrated keep migration order. All queries in this
//! crate name their columns, so both work.

use std::collections::BTreeSet;
use std::fmt;

use rusqlite::{Connection, OptionalExtension, params};
use sha1::{Digest, Sha1};

use crate::error::{Error, Result};
use crate::time::{Clock, Timestamp};

pub const SCHEMA_SQL: &str = include_str!("schema.sql");

/// `schema_migrations` of a freshly prepared reference database, in insertion order.
const SCHEMA_MIGRATIONS: &str = include_str!("schema_migrations.txt");

/// Every migration version in `reference/db/migrate`, in the order `db:prepare` inserts them
/// (`assume_migrated_upto_version`: the current version, then the rest newest first).
pub fn migration_versions() -> impl Iterator<Item = &'static str> {
    SCHEMA_MIGRATIONS.lines().filter(|line| !line.is_empty())
}

/// Tables `db:prepare` leaves a zeroed `sqlite_sequence` row for, in insertion order: the
/// SQLite adapter rebuilds a table by copying it for each `add_foreign_key` in schema.rb.
const SCHEMA_SEQUENCES: &str = include_str!("schema_sequences.txt");

/// SHA1 of `reference/db/schema.rb`, which `db:schema:load` records in `ar_internal_metadata`.
pub const SCHEMA_SHA1: &str = include_str!("schema_sha1.txt").trim_ascii();

/// `timeout: 5000` in `config/database.yml`.
pub const BUSY_TIMEOUT_MS: u64 = 5000;

/// Applies the per-connection settings Rails applies (`SQLite3Adapter#configure_connection`).
pub fn configure_connection(conn: &Connection) -> Result<()> {
    conn.busy_timeout(std::time::Duration::from_millis(BUSY_TIMEOUT_MS))?;
    conn.pragma_update(None, "foreign_keys", true)?;
    conn.pragma_update(None, "journal_mode", "wal")?;
    conn.pragma_update(None, "synchronous", "normal")?;
    conn.pragma_update(None, "mmap_size", 134_217_728)?;
    conn.pragma_update(None, "journal_size_limit", 67_108_864)?;
    conn.pragma_update(None, "cache_size", 2000)?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Prepared {
    /// The database was empty; the schema was loaded.
    Loaded,
    /// The database was migrated to exactly this build's schema.
    UpToDate,
}

/// How an existing database's `schema_migrations` differs from this build's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaMismatch {
    /// Versions this build's schema includes that the database hasn't run: a Rails release with
    /// those migrations hasn't booted on it yet.
    pub missing: Vec<String>,
    /// Versions the database has run that this build doesn't know: a newer Rails release
    /// migrated it. Refused too (decision 1 requires the exact set): those migrations may add
    /// columns or tables whose invariants this build wouldn't maintain on write, and rolling
    /// forward to a matching build is always possible, while writes made without them may not
    /// be recoverable.
    pub unknown: Vec<String>,
}

impl fmt::Display for SchemaMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the database schema doesn't match this build's")?;
        if !self.missing.is_empty() {
            write!(
                f,
                "; missing migrations {} (boot the Rails image once to migrate)",
                self.missing.join(", ")
            )?;
        }
        if !self.unknown.is_empty() {
            write!(
                f,
                "; unknown migrations {} (a newer Rails release migrated this database: run a build of the same release)",
                self.unknown.join(", ")
            )?;
        }
        Ok(())
    }
}

/// `bin/rails db:prepare`, less the migrating: loads the schema into an empty database, or
/// verifies that an existing one has run exactly this build's migrations. We don't port
/// migrations, so anything else is a [`SchemaMismatch`] and the app refuses to boot.
pub fn prepare(conn: &mut Connection, environment: &str, clock: &dyn Clock) -> Result<Prepared> {
    if is_empty(conn)? {
        load_schema(conn, environment, clock)?;
        return Ok(Prepared::Loaded);
    }
    let mismatch = schema_mismatch(conn)?;
    if mismatch.missing.is_empty() && mismatch.unknown.is_empty() {
        Ok(Prepared::UpToDate)
    } else {
        Err(Error::SchemaMismatch(mismatch))
    }
}

/// Compares the database's `schema_migrations` with [`migration_versions`]. A database without
/// the table (not a Rails database, or one never prepared) is missing every version.
pub fn schema_mismatch(conn: &Connection) -> Result<SchemaMismatch> {
    let expected: BTreeSet<&str> = migration_versions().collect();
    let present: BTreeSet<String> = if table_exists(conn, "schema_migrations")? {
        let mut stmt = conn.prepare(r#"SELECT "version" FROM "schema_migrations""#)?;
        stmt.query_map([], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<_>>()?
    } else {
        BTreeSet::new()
    };
    Ok(SchemaMismatch {
        missing: expected
            .iter()
            .filter(|v| !present.contains(**v))
            .map(|v| v.to_string())
            .collect(),
        unknown: present
            .iter()
            .filter(|v| !expected.contains(v.as_str()))
            .cloned()
            .collect(),
    })
}

fn is_empty(conn: &Connection) -> Result<bool> {
    Ok(!conn.prepare("SELECT 1 FROM sqlite_master")?.exists([])?)
}

fn load_schema(conn: &mut Connection, environment: &str, clock: &dyn Clock) -> Result<()> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    tx.execute_batch(SCHEMA_SQL)?;
    for table in SCHEMA_SEQUENCES.lines().filter(|line| !line.is_empty()) {
        tx.execute("INSERT INTO sqlite_sequence (name, seq) VALUES (?, 0)", [table])?;
    }

    for version in migration_versions() {
        tx.execute(
            r#"INSERT INTO "schema_migrations" ("version") VALUES (?)"#,
            [version],
        )?;
    }

    set_internal_metadata(&tx, "environment", environment, clock.now())?;
    set_internal_metadata(&tx, "schema_sha1", SCHEMA_SHA1, clock.now())?;
    tx.commit()?;
    Ok(())
}

fn set_internal_metadata(conn: &Connection, key: &str, value: &str, now: Timestamp) -> Result<()> {
    let existing: Option<String> = conn
        .query_row(
            r#"SELECT "value" FROM "ar_internal_metadata" WHERE "key" = ?"#,
            [key],
            |r| r.get(0),
        )
        .optional()?;
    match existing {
        None => {
            conn.execute(
                r#"INSERT INTO "ar_internal_metadata" ("key", "value", "created_at", "updated_at") VALUES (?, ?, ?, ?)"#,
                params![key, value, now, now],
            )?;
        }
        Some(current) if current != value => {
            conn.execute(
                r#"UPDATE "ar_internal_metadata" SET "value" = ?, "updated_at" = ? WHERE "key" = ?"#,
                params![value, now, key],
            )?;
        }
        Some(_) => {}
    }
    Ok(())
}

fn table_exists(conn: &Connection, name: &str) -> Result<bool> {
    Ok(conn
        .prepare("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?")?
        .exists([name])?)
}

/// SHA1 hex of a schema file, as Rails computes it for `schema_sha1`.
pub fn schema_sha1(contents: &[u8]) -> String {
    hex::encode(Sha1::digest(contents))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::SystemClock;

    /// `sqlite_master` as `reference-tools/db/regenerate-schema.sh` dumps it.
    fn dump_schema(conn: &Connection) -> String {
        let mut stmt = conn
            .prepare("SELECT sql || ';' FROM sqlite_master WHERE sql IS NOT NULL AND name NOT LIKE 'sqlite_%' AND name NOT LIKE 'message_search_index_%' ORDER BY rowid")
            .unwrap();
        let rows = stmt.query_map([], |r| r.get::<_, String>(0)).unwrap();
        rows.map(|r| r.unwrap() + "\n").collect()
    }

    fn prepared() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        assert_eq!(prepare(&mut conn, "production", &SystemClock).unwrap(), Prepared::Loaded);
        conn
    }

    fn mismatch(result: Result<Prepared>) -> SchemaMismatch {
        match result {
            Err(Error::SchemaMismatch(mismatch)) => mismatch,
            other => panic!("expected a schema mismatch, got {other:?}"),
        }
    }

    /// The schema files were generated from the Rails app, which is gone: they are the frozen
    /// record of the schema it last migrated to (129 migrations through 2026-10-03), so any
    /// change to them is a mistake.
    #[test]
    fn schema_files_are_frozen() {
        let versions: BTreeSet<&str> = migration_versions().collect();
        assert_eq!((migration_versions().count(), versions.len()), (129, 129));
        assert_eq!(versions.first(), Some(&"20231215043540"));
        assert_eq!(versions.last(), Some(&"20261003180000"));
        for (name, contents, sha256) in [
            ("schema.sql", SCHEMA_SQL, "8087be3847f61d3d390881f0d9a714da6c89bfed6a49c6d013c33c65353dabc1"),
            ("schema_migrations.txt", SCHEMA_MIGRATIONS, "eee52ef4996494591474c15f4dd5c54c7a2b0b8d40ae5cb2b79503a60714f7fa"),
            ("schema_sequences.txt", SCHEMA_SEQUENCES, "5dbef42959e66e2389c3b9a4b3a9ca74b7df176d471e84ea5c19fb79a48b25ad"),
            ("schema_sha1.txt", include_str!("schema_sha1.txt"), "4649da67165b760883605a1a3c038e6016269971e70c758d210425cc7334f499"),
        ] {
            assert_eq!(hex::encode(sha2::Sha256::digest(contents)), sha256, "{name} changed");
        }
    }

    #[test]
    fn schema_sql_is_the_whole_reference_schema() {
        // The FTS5 index and Rails' own two tables (every other table is `schema.sql`'s, which
        // `a_prepared_database_has_exactly_schema_sql` and `schema_files_are_frozen` cover).
        let conn = prepared();
        for table in ["message_search_index", "schema_migrations", "ar_internal_metadata"] {
            assert!(table_exists(&conn, table).unwrap(), "missing table {table}");
        }
        // Partial and expression indexes come through as written.
        assert!(SCHEMA_SQL.contains(r#"CREATE UNIQUE INDEX "index_rooms_on_direct_member_key" ON "rooms" ("direct_member_key") WHERE direct_member_key IS NOT NULL AND deleted_at IS NULL;"#));
        assert!(SCHEMA_SQL.contains(r#"CREATE UNIQUE INDEX "index_users_on_lower_github_login" ON "users" (LOWER(github_login)) WHERE github_login IS NOT NULL;"#));
    }

    /// `db:prepare` leaves a zeroed `sqlite_sequence` row for each table whose foreign keys
    /// schema.rb adds after `create_table` (the adapter rebuilds those tables by copying them):
    /// 59 of them in our schema.
    #[test]
    fn a_prepared_database_has_rails_sqlite_sequence_rows() {
        let conn = prepared();
        let rows: Vec<(String, i64)> = conn
            .prepare("SELECT name, seq FROM sqlite_sequence ORDER BY rowid")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(rows.len(), 59);
        assert!(rows.iter().all(|(_, seq)| *seq == 0));
        assert_eq!(rows[0].0, "active_storage_attachments");
    }

    #[test]
    fn a_prepared_database_has_exactly_schema_sql() {
        assert_eq!(dump_schema(&prepared()), SCHEMA_SQL);
    }

    #[test]
    fn prepare_loads_then_is_idempotent() {
        let mut conn = prepared();
        let before = dump_schema(&conn);
        assert_eq!(
            prepare(&mut conn, "production", &SystemClock).unwrap(),
            Prepared::UpToDate
        );
        assert_eq!(dump_schema(&conn), before, "prepare never alters an existing database");

        // `assume_migrated_upto_version`: the newest version first, then the rest newest first.
        let versions: Vec<String> = conn
            .prepare("SELECT version FROM schema_migrations ORDER BY rowid")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        let mut newest_first = versions.clone();
        newest_first.sort_by(|a, b| b.cmp(a));
        assert_eq!(versions, newest_first);
        assert_eq!(versions.len(), migration_versions().count());

        let metadata: Vec<(String, String)> = conn
            .prepare("SELECT key, value FROM ar_internal_metadata ORDER BY rowid")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(
            metadata,
            [
                ("environment".to_string(), "production".to_string()),
                ("schema_sha1".to_string(), SCHEMA_SHA1.to_string())
            ]
        );
        conn.execute(
            "INSERT INTO message_search_index(rowid, body) VALUES (1, 'running dogs')",
            [],
        )
        .unwrap();
        let hit: i64 = conn
            .query_row(
                "SELECT rowid FROM message_search_index WHERE body MATCH 'run'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(hit, 1, "porter tokenizer");
    }

    #[test]
    fn prepare_refuses_a_database_missing_migrations() {
        let mut conn = prepared();
        let newest = migration_versions().next().unwrap();
        conn.execute("DELETE FROM schema_migrations WHERE version = ?", [newest])
            .unwrap();
        let before = dump_schema(&conn);

        let error = prepare(&mut conn, "production", &SystemClock).unwrap_err();
        let message = error.to_string();
        assert!(message.contains(&format!("missing migrations {newest}")), "{message}");
        assert!(message.contains("boot the Rails image once to migrate"), "{message}");
        assert!(!message.contains("unknown"), "{message}");
        assert_eq!(dump_schema(&conn), before);
    }

    #[test]
    fn prepare_refuses_a_database_with_unknown_migrations() {
        let mut conn = prepared();
        conn.execute("INSERT INTO schema_migrations (version) VALUES ('29991231235959')", [])
            .unwrap();
        let found = mismatch(prepare(&mut conn, "production", &SystemClock));
        assert_eq!(found.missing, Vec::<String>::new());
        assert_eq!(found.unknown, ["29991231235959"]);
        let message = Error::SchemaMismatch(found).to_string();
        assert!(message.contains("unknown migrations 29991231235959"), "{message}");
    }

    #[test]
    fn prepare_refuses_a_database_that_rails_never_prepared() {
        // Upstream's schema: tables, but none of our versions (and no schema_migrations at all).
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(r#"CREATE TABLE "users" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL)"#)
            .unwrap();
        let found = mismatch(prepare(&mut conn, "production", &SystemClock));
        assert_eq!(found.missing.len(), migration_versions().count());
        assert!(found.unknown.is_empty());
        assert!(!table_exists(&conn, "schema_migrations").unwrap());
    }
}
