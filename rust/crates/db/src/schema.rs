//! The schema a fresh database gets, and the connection settings from
//! `reference/config/database.yml` plus the sqlite3 adapter's `DEFAULT_PRAGMAS`.
//!
//! Server boot loads the schema into an empty database and otherwise only checks that a database
//! has run exactly this build's migrations (decisions.md, decision 1): it never creates or alters
//! anything in an existing database. Schema changes go through `campfire db-migrate`
//! ([`crate::migrations`]), an explicit release step.
//!
//! Two layers, never edited by hand:
//!
//! - `baseline/` is the Rails-era schema, frozen at the last Rails migration (20261003180000):
//!   it was dumped from the reference app's `db:prepare` before Rails was removed.
//!   `schema.sql` is that database's `sqlite_master` minus the objects SQLite derives on its own
//!   (FTS5 shadow tables, `sqlite_sequence`, autoindexes). A fresh `db:prepare` loads
//!   `schema.rb`, so columns come out in alphabetical order; databases that were migrated keep
//!   migration order. All queries in this crate name their columns, so both work.
//!   `schema_sha1.txt` is the SHA1 of the final Rails `schema.rb`: Rails recorded it in
//!   `ar_internal_metadata`, and a fresh load still does, as a marker of the baseline it started
//!   from. Nothing reads it back, and it's never regenerated.
//! - `src/schema.sql`, `src/schema_migrations.txt` and `src/schema_sequences.txt` are what a
//!   fresh database gets: the baseline with every migration in `migrations/` applied, dumped by
//!   [`generate`]. A test fails when they're stale; to regenerate them, run
//!   `CAMPFIRE_SCHEMA_DUMP=write cargo test -p campfire_db --lib schema::tests::schema_files`.

use std::collections::BTreeSet;
use std::fmt;

use rusqlite::{Connection, OptionalExtension, params};
use sha1::{Digest, Sha1};

use crate::error::{Error, Result};
use crate::time::{Clock, Timestamp};

pub const SCHEMA_SQL: &str = include_str!("schema.sql");

/// `schema_migrations` of a fresh database, newest first (the order Rails'
/// `assume_migrated_upto_version` inserts them in).
const SCHEMA_MIGRATIONS: &str = include_str!("schema_migrations.txt");

/// Every migration version this build's schema includes, newest first: the Rails migrations in
/// the baseline, then [`crate::migrations::catalog`]. Boot requires exactly this set.
pub fn migration_versions() -> impl Iterator<Item = &'static str> {
    lines(SCHEMA_MIGRATIONS)
}

/// Tables a fresh database gets a zeroed `sqlite_sequence` row for, in insertion order. Rails'
/// `db:prepare` left them (the SQLite adapter rebuilds a table by copying it for each
/// `add_foreign_key` in schema.rb); nothing depends on them, but fresh databases match the ones
/// Rails created.
const SCHEMA_SEQUENCES: &str = include_str!("schema_sequences.txt");

const BASELINE: Files<'static> = Files {
    schema_sql: include_str!("../baseline/schema.sql"),
    schema_migrations: include_str!("../baseline/schema_migrations.txt"),
    schema_sequences: include_str!("../baseline/schema_sequences.txt"),
};

const CURRENT: Files<'static> = Files {
    schema_sql: SCHEMA_SQL,
    schema_migrations: SCHEMA_MIGRATIONS,
    schema_sequences: SCHEMA_SEQUENCES,
};

/// The Rails migration versions: the baseline every database started from.
pub fn baseline_versions() -> impl Iterator<Item = &'static str> {
    lines(BASELINE.schema_migrations)
}

/// SHA1 of the final Rails `reference/db/schema.rb`, which `db:schema:load` recorded in
/// `ar_internal_metadata`. Frozen with the baseline.
pub const SCHEMA_SHA1: &str = include_str!("../baseline/schema_sha1.txt").trim_ascii();

fn lines(text: &str) -> impl Iterator<Item = &str> {
    text.lines().filter(|line| !line.is_empty())
}

/// A schema as files: `sqlite_master`'s statements, `schema_migrations` and the tables with a
/// `sqlite_sequence` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Files<'a> {
    pub schema_sql: &'a str,
    pub schema_migrations: &'a str,
    pub schema_sequences: &'a str,
}

/// [`Files`] that [`generate`] or [`dump`] produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dump {
    pub schema_sql: String,
    pub schema_migrations: String,
    pub schema_sequences: String,
}

impl Dump {
    pub fn files(&self) -> Files<'_> {
        Files {
            schema_sql: &self.schema_sql,
            schema_migrations: &self.schema_migrations,
            schema_sequences: &self.schema_sequences,
        }
    }
}

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
    /// Versions this build's schema includes that the database hasn't run: `campfire db-migrate`
    /// with this build hasn't run on it yet.
    pub missing: Vec<String>,
    /// Versions the database has run that this build doesn't know: a newer build migrated it.
    /// Refused too (decision 1 requires the exact set): those migrations may add
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
                "; missing migrations {} (run `campfire db-migrate DATABASE` with this build)",
                self.missing.join(", ")
            )?;
        }
        if !self.unknown.is_empty() {
            write!(
                f,
                "; unknown migrations {} (a newer build migrated this database: run that build or a later one)",
                self.unknown.join(", ")
            )?;
        }
        Ok(())
    }
}

/// `bin/rails db:prepare`, less the migrating: loads the schema into an empty database, or
/// verifies that an existing one has run exactly this build's migrations. Anything else is a
/// [`SchemaMismatch`] and the app refuses to boot: migrating is `campfire db-migrate`'s job.
pub fn prepare(conn: &mut Connection, environment: &str, clock: &dyn Clock) -> Result<Prepared> {
    if is_empty(conn)? {
        load_schema(conn, &CURRENT, environment, clock)?;
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
    let expected: Vec<&str> = migration_versions().collect();
    schema_mismatch_against(conn, &expected)
}

/// [`schema_mismatch`] against another build's versions.
pub fn schema_mismatch_against(conn: &Connection, expected: &[&str]) -> Result<SchemaMismatch> {
    let expected: BTreeSet<&str> = expected.iter().copied().collect();
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

fn load_schema(conn: &mut Connection, files: &Files<'_>, environment: &str, clock: &dyn Clock) -> Result<()> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    tx.execute_batch(files.schema_sql)?;
    for table in lines(files.schema_sequences) {
        tx.execute("INSERT INTO sqlite_sequence (name, seq) VALUES (?, 0)", [table])?;
    }

    for version in lines(files.schema_migrations) {
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

/// The schema files for the baseline plus `catalog`: loads the baseline into an empty database,
/// applies the migrations with the same code `campfire db-migrate` runs, and dumps the result.
/// Migrations that leave rows behind are refused: a fresh load can't reproduce data.
pub fn generate(catalog: &[crate::migrations::Migration]) -> std::result::Result<Dump, crate::migrations::Error> {
    use crate::migrations::{Error as MigrationError, migrate_with};
    let mut conn = Connection::open_in_memory()?;
    conn.pragma_update(None, "foreign_keys", true)?;
    load_schema(&mut conn, &BASELINE, "production", &crate::time::SystemClock)?;
    let manifest: Vec<&str> = baseline_versions()
        .map(|version| -> &str { version })
        .chain(catalog.iter().map(|m| m.version.as_str()))
        .collect();
    migrate_with(&mut conn, &manifest, catalog)?;
    let tables: Vec<String> = conn
        .prepare(
            "SELECT name FROM pragma_table_list WHERE schema = 'main' AND type IN ('table', 'virtual') \
             AND name NOT LIKE 'sqlite_%' AND name NOT IN ('schema_migrations', 'ar_internal_metadata') ORDER BY name",
        )?
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    for table in tables {
        let rows: i64 = conn.query_row(&format!(r#"SELECT count(*) FROM "{table}""#), [], |r| r.get(0))?;
        if rows > 0 {
            return Err(MigrationError::Catalog(format!(
                "the migrations leave {rows} rows in {table}: a fresh database couldn't have them"
            )));
        }
    }
    Ok(dump(&conn)?)
}

/// A database's schema as [`Files`]: `sqlite_master` in creation order, less what SQLite derives
/// on its own (virtual table shadow tables, `sqlite_sequence`, autoindexes); `schema_migrations`
/// newest first; the tables with a `sqlite_sequence` row, in insertion order.
pub fn dump(conn: &Connection) -> Result<Dump> {
    let shadow: BTreeSet<String> = conn
        .prepare("SELECT name FROM pragma_table_list WHERE schema = 'main' AND type = 'shadow'")?
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    let mut schema_sql = String::new();
    let mut stmt = conn.prepare(
        "SELECT tbl_name, sql FROM sqlite_master WHERE sql IS NOT NULL AND name NOT LIKE 'sqlite_%' ORDER BY rowid",
    )?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        if !shadow.contains(&row.get::<_, String>(0)?) {
            schema_sql.push_str(&row.get::<_, String>(1)?);
            schema_sql.push_str(";\n");
        }
    }
    let mut versions: Vec<String> = conn
        .prepare(r#"SELECT "version" FROM "schema_migrations""#)?
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    versions.sort_by(|a, b| crate::migrations::version_order(b, a));
    let sequences: Vec<String> = if table_exists(conn, "sqlite_sequence")? {
        conn.prepare("SELECT name FROM sqlite_sequence ORDER BY rowid")?
            .query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?
    } else {
        Vec::new()
    };
    let joined = |items: Vec<String>| items.into_iter().map(|item| item + "\n").collect();
    Ok(Dump {
        schema_sql,
        schema_migrations: joined(versions),
        schema_sequences: joined(sequences),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::SystemClock;

    /// `sqlite_master` as the Rails-era schema dump read it.
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

    /// The baseline files were generated from the Rails app, which is gone: they are the frozen
    /// record of the schema it last migrated to (129 migrations through 2026-10-03), so any
    /// change to them is a mistake. New schema changes are migrations on top of them.
    #[test]
    fn baseline_is_frozen() {
        let versions: BTreeSet<&str> = baseline_versions().collect();
        assert_eq!((baseline_versions().count(), versions.len()), (129, 129));
        assert_eq!(versions.first(), Some(&"20231215043540"));
        assert_eq!(versions.last(), Some(&"20261003180000"));
        for (name, contents, sha256) in [
            ("schema.sql", BASELINE.schema_sql, "8087be3847f61d3d390881f0d9a714da6c89bfed6a49c6d013c33c65353dabc1"),
            ("schema_migrations.txt", BASELINE.schema_migrations, "eee52ef4996494591474c15f4dd5c54c7a2b0b8d40ae5cb2b79503a60714f7fa"),
            ("schema_sequences.txt", BASELINE.schema_sequences, "5dbef42959e66e2389c3b9a4b3a9ca74b7df176d471e84ea5c19fb79a48b25ad"),
            ("schema_sha1.txt", include_str!("../baseline/schema_sha1.txt"), "4649da67165b760883605a1a3c038e6016269971e70c758d210425cc7334f499"),
        ] {
            assert_eq!(hex::encode(sha2::Sha256::digest(contents)), sha256, "baseline/{name} changed");
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
        assert!(message.contains("run `campfire db-migrate DATABASE` with this build"), "{message}");
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

    /// The committed schema files are exactly what [`generate`] makes of the baseline and
    /// `migrations/`. `CAMPFIRE_SCHEMA_DUMP=write` rewrites them instead.
    #[test]
    fn schema_files_are_the_baseline_plus_every_migration() {
        let generated = generate(&crate::migrations::catalog()).unwrap();
        if std::env::var_os("CAMPFIRE_SCHEMA_DUMP").is_some_and(|value| value == "write") {
            let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
            std::fs::write(src.join("schema.sql"), &generated.schema_sql).unwrap();
            std::fs::write(src.join("schema_migrations.txt"), &generated.schema_migrations).unwrap();
            std::fs::write(src.join("schema_sequences.txt"), &generated.schema_sequences).unwrap();
            return;
        }
        assert!(
            generated.files() == CURRENT,
            "crates/db/src/schema.sql, schema_migrations.txt or schema_sequences.txt is stale: run \
             `CAMPFIRE_SCHEMA_DUMP=write cargo test -p campfire_db --lib schema::tests::schema_files` and commit the result"
        );
    }

    #[test]
    fn dump_matches_the_rails_era_dump_query_and_a_fresh_load_dumps_the_committed_files() {
        let conn = prepared();
        let dumped = dump(&conn).unwrap();
        assert_eq!(dumped.schema_sql, dump_schema(&conn));
        assert!(dumped.files() == CURRENT);
        let mut baseline = Connection::open_in_memory().unwrap();
        load_schema(&mut baseline, &BASELINE, "production", &SystemClock).unwrap();
        assert!(dump(&baseline).unwrap().files() == BASELINE);
    }

    /// The whole schema-change path with a migration that doesn't ship: a production-shaped
    /// database at the baseline migrates, passes the strict check of the build that carries the
    /// migration, and dumps exactly what that build's regenerated schema files say, which is
    /// also what a fresh database loaded from them dumps. The current build then refuses it.
    #[test]
    fn an_example_migration_round_trips_through_migrate_check_and_dump() {
        use crate::migrations::{Migration, migrate_with};
        let example = [Migration::new(
            "20991231235959",
            "add_rooms_topic",
            "ALTER TABLE \"rooms\" ADD COLUMN \"topic\" varchar; \
             CREATE INDEX \"index_rooms_on_topic\" ON \"rooms\" (\"topic\"); \
             CREATE TABLE \"room_topics\" (\"id\" integer PRIMARY KEY AUTOINCREMENT NOT NULL, \
               \"room_id\" integer NOT NULL REFERENCES \"rooms\" (\"id\"), \"body\" text NOT NULL); \
             UPDATE \"rooms\" SET \"topic\" = \"name\" WHERE \"name\" IS NOT NULL;",
        )];
        let next = generate(&example).unwrap();
        assert!(next.schema_migrations.starts_with("20991231235959\n20261003180000\n"));
        assert!(next.schema_sql.contains(r#"CREATE INDEX "index_rooms_on_topic""#));
        let manifest: Vec<&str> = lines(&next.schema_migrations).collect();

        let mut db = Connection::open_in_memory().unwrap();
        load_schema(&mut db, &BASELINE, "production", &SystemClock).unwrap();
        db.execute_batch(
            "INSERT INTO rooms (name, type, creator_id, created_at, updated_at) \
             VALUES ('All Talk', 'Rooms::Open', 1, '2026-01-01', '2026-01-01');",
        )
        .unwrap();
        assert_eq!(migrate_with(&mut db, &manifest, &example).unwrap(), ["20991231235959"]);
        let mismatch = schema_mismatch_against(&db, &manifest).unwrap();
        assert!(mismatch.missing.is_empty() && mismatch.unknown.is_empty(), "{mismatch}");
        let topic: String = db.query_row("SELECT topic FROM rooms", [], |r| r.get(0)).unwrap();
        assert_eq!(topic, "All Talk");
        let migrated = dump(&db).unwrap();
        assert_eq!(migrated.schema_sql, next.schema_sql);
        assert_eq!(migrated.schema_migrations, next.schema_migrations);

        let mut fresh = Connection::open_in_memory().unwrap();
        load_schema(&mut fresh, &next.files(), "production", &SystemClock).unwrap();
        assert_eq!(dump(&fresh).unwrap(), next);

        assert!(migrate_with(&mut db, &manifest, &example).unwrap().is_empty(), "idempotent");
        let found = mismatch_of(&db);
        assert_eq!(found.unknown, ["20991231235959"], "this build refuses the newer database");
        assert!(crate::migrations::migrate(&mut db).is_err());
    }

    #[test]
    fn generate_refuses_migrations_that_leave_rows() {
        use crate::migrations::Migration;
        let seeding = [Migration::new(
            "20261005120000",
            "seed",
            "CREATE TABLE seeds (id integer PRIMARY KEY); INSERT INTO seeds VALUES (1);",
        )];
        let error = generate(&seeding).unwrap_err();
        assert!(error.to_string().contains("1 rows in seeds"), "{error}");
    }

    fn mismatch_of(conn: &Connection) -> SchemaMismatch {
        schema_mismatch(conn).unwrap()
    }
}
