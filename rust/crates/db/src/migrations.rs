//! Schema changes after the Rails era: `crates/db/migrations/<VERSION>_<name>.sql`, compiled into
//! the binary and applied only by the explicit `campfire db-migrate DATABASE` step. Server boot
//! never migrates (`schema::prepare` still insists on exactly this build's versions).
//!
//! Versions are Rails-compatible: 14-digit UTC timestamps recorded in `schema_migrations` as
//! canonical decimal strings, ordered as integers, and newer than every Rails migration in
//! `baseline/schema_migrations.txt`. After adding a migration, regenerate the schema files
//! (`schema::dump`); a test fails while they're stale.

use rusqlite::{
    Connection, TransactionBehavior,
    hooks::{AuthAction, AuthContext, Authorization},
};
use std::cmp::Ordering;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Migration {
    pub version: String,
    pub name: String,
    pub sql: String,
}

impl Migration {
    pub fn new(version: &str, name: &str, sql: &str) -> Self {
        Migration {
            version: version.into(),
            name: name.into(),
            sql: sql.into(),
        }
    }
}

/// This build's migrations, oldest first.
pub fn catalog() -> Vec<Migration> {
    const CATALOG: &[(&str, &str, &str)] = include!(concat!(env!("OUT_DIR"), "/migrations.rs"));
    let mut migrations: Vec<_> = CATALOG
        .iter()
        .map(|(version, name, sql)| Migration::new(version, name, sql))
        .collect();
    migrations.sort_by(|a, b| version_order(&a.version, &b.version));
    migrations
}

/// Rails compares versions as integers.
pub fn version_order(a: &str, b: &str) -> Ordering {
    a.len().cmp(&b.len()).then_with(|| a.cmp(b))
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid migration catalog: {0}")]
    Catalog(String),
    #[error("{0}")]
    Schema(crate::schema::SchemaMismatch),
    #[error("migration {version}: {source}")]
    Migration { version: String, source: rusqlite::Error },
    #[error("the migrations left foreign key violations (table {0}); nothing was applied")]
    ForeignKeys(String),
    #[error("another process changed schema_migrations while migrating; nothing was applied")]
    Concurrent,
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Database(#[from] crate::Error),
}

/// Migrations the database hasn't run, oldest first. Refuses (without writing) a database this
/// build can't bring to its schema: one that has run versions this build doesn't know (a newer
/// build migrated it, so running this one would be a downgrade), or one missing versions that
/// aren't in the catalog (it predates the baseline).
pub fn pending<'a>(
    conn: &Connection,
    manifest: &[&str],
    catalog: &'a [Migration],
) -> Result<Vec<&'a Migration>, Error> {
    validate(manifest, catalog)?;
    let mut mismatch = crate::schema::schema_mismatch_against(conn, manifest)?;
    let in_catalog = |version: &String| catalog.iter().any(|m| &m.version == version);
    let pending: BTreeSet<String> = mismatch.missing.iter().filter(|v| in_catalog(v)).cloned().collect();
    mismatch.missing.retain(|version| !in_catalog(version));
    if !mismatch.missing.is_empty() || !mismatch.unknown.is_empty() {
        return Err(Error::Schema(mismatch));
    }
    let mut pending: Vec<_> = catalog.iter().filter(|m| pending.contains(&m.version)).collect();
    pending.sort_by(|a, b| version_order(&a.version, &b.version));
    Ok(pending)
}

/// [`pending`] against this build's schema and catalog.
pub fn pending_for_build(conn: &Connection) -> Result<Vec<String>, Error> {
    let manifest: Vec<_> = crate::schema::migration_versions().collect();
    Ok(pending(conn, &manifest, &catalog())?
        .into_iter()
        .map(|m| m.version.clone())
        .collect())
}

/// Applies this build's pending migrations. See [`migrate_with`].
pub fn migrate(conn: &mut Connection) -> Result<Vec<String>, Error> {
    let manifest: Vec<_> = crate::schema::migration_versions().collect();
    migrate_with(conn, &manifest, &catalog())
}

/// Brings the database to `manifest` by applying the pending part of `catalog`, oldest first, in
/// one transaction with their `schema_migrations` rows: either all of them commit or none do.
/// Foreign key enforcement is off while they run (so a migration can rebuild a table the way
/// SQLite documents), then `PRAGMA foreign_key_check` must come back empty before the commit.
/// Migration SQL can't end the transaction, attach databases, set pragmas or touch
/// `schema_migrations`. Writes nothing when nothing is pending.
pub fn migrate_with(
    conn: &mut Connection,
    manifest: &[&str],
    catalog: &[Migration],
) -> Result<Vec<String>, Error> {
    let planned: Vec<String> = pending(conn, manifest, catalog)?
        .into_iter()
        .map(|m| m.version.clone())
        .collect();
    if planned.is_empty() {
        return Ok(planned);
    }
    let foreign_keys: bool = conn.query_row("PRAGMA foreign_keys", [], |r| r.get(0))?;
    conn.pragma_update(None, "foreign_keys", false)?;
    let result = apply(conn, manifest, catalog, &planned);
    conn.pragma_update(None, "foreign_keys", foreign_keys)?;
    result.map(|()| planned)
}

fn apply(conn: &mut Connection, manifest: &[&str], catalog: &[Migration], planned: &[String]) -> Result<(), Error> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    // Plan again while holding the write lock: another runner may have got there first.
    let migrations = pending(&tx, manifest, catalog)?;
    if migrations.iter().map(|m| &m.version).ne(planned.iter()) {
        return Err(Error::Concurrent);
    }
    for migration in migrations {
        let failed = |source| Error::Migration {
            version: migration.version.clone(),
            source,
        };
        tx.authorizer(Some(migration_authorizer));
        let result = tx.execute_batch(&migration.sql);
        tx.authorizer(None::<fn(AuthContext<'_>) -> Authorization>);
        result.map_err(failed)?;
        tx.execute(r#"INSERT INTO "schema_migrations" ("version") VALUES (?)"#, [&migration.version])
            .map_err(failed)?;
    }
    let violation: Option<String> = {
        let mut stmt = tx.prepare("PRAGMA foreign_key_check")?;
        let mut rows = stmt.query([])?;
        rows.next()?.map(|row| row.get(0)).transpose()?
    };
    if let Some(table) = violation {
        return Err(Error::ForeignKeys(table));
    }
    tx.commit()?;
    Ok(())
}

fn validate(manifest: &[&str], catalog: &[Migration]) -> Result<(), Error> {
    let mut seen = BTreeSet::new();
    for migration in catalog {
        let version = &migration.version;
        if version.is_empty() || version.starts_with('0') || !version.bytes().all(|b| b.is_ascii_digit()) {
            return Err(Error::Catalog(format!("noncanonical version {version:?}")));
        }
        if !seen.insert(version.as_str()) {
            return Err(Error::Catalog(format!("duplicate version {version}")));
        }
        if !manifest.contains(&version.as_str()) {
            return Err(Error::Catalog(format!(
                "version {version} isn't in the schema manifest (regenerate the schema files)"
            )));
        }
    }
    Ok(())
}

fn migration_authorizer(ctx: AuthContext<'_>) -> Authorization {
    match ctx.action {
        // SQL must not commit behind the runner's back, reach other databases, change connection
        // settings (foreign keys, writable_schema) or tamper with its version ledger.
        AuthAction::Transaction { .. }
        | AuthAction::Savepoint { .. }
        | AuthAction::Attach { .. }
        | AuthAction::Detach { .. }
        | AuthAction::Pragma { .. } => Authorization::Deny,
        AuthAction::Insert { table_name }
        | AuthAction::Delete { table_name }
        | AuthAction::Update { table_name, .. }
        | AuthAction::AlterTable { table_name, .. }
        | AuthAction::DropTable { table_name }
            if table_name.eq_ignore_ascii_case("schema_migrations") =>
        {
            Authorization::Deny
        }
        _ => Authorization::Allow,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SystemClock, schema};

    /// The compiled-in catalog is exactly `crates/db/migrations/` as it is on disk now: the same
    /// files, names and SQL. Read at run time, so a build that missed an added, removed, renamed
    /// or edited migration fails here instead of testing (or shipping) another catalog.
    #[test]
    fn catalog_matches_the_migrations_directory() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
        let mut on_disk: Vec<Migration> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap())
            .filter(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
            .map(|entry| {
                let file = entry.file_name().into_string().unwrap();
                let (version, name) = file.strip_suffix(".sql").unwrap().split_once('_').unwrap();
                Migration::new(version, name, &std::fs::read_to_string(entry.path()).unwrap())
            })
            .collect();
        on_disk.sort_by(|a, b| version_order(&a.version, &b.version));
        assert_eq!(
            catalog(),
            on_disk,
            "the compiled-in migration catalog is stale: run `cargo clean -p campfire_db` and rebuild"
        );
    }

    /// The image build passes the directory's digest to `build.rs`, which reruns when it changes.
    #[test]
    fn the_image_build_tracks_the_migrations_by_content() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let build = std::fs::read_to_string(root.join("build.rs")).unwrap();
        assert!(build.contains("cargo::rerun-if-env-changed=CAMPFIRE_MIGRATIONS_DIGEST"));
        assert!(build.contains("cargo::rerun-if-changed={}\", dir.display()"));
        let dockerfile = std::fs::read_to_string(root.join("../../Dockerfile")).unwrap();
        let build_step = dockerfile.split("RUN --mount=type=cache").nth(1).unwrap();
        assert!(
            build_step.contains("export CAMPFIRE_MIGRATIONS_DIGEST=") && build_step.contains("crates/db/migrations"),
            "rust/Dockerfile's cargo build must export CAMPFIRE_MIGRATIONS_DIGEST from crates/db/migrations"
        );
        assert!(build_step.find("CAMPFIRE_MIGRATIONS_DIGEST") < build_step.find("cargo build"));
    }

    fn prepared() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        schema::prepare(&mut conn, "production", &SystemClock).unwrap();
        conn
    }

    /// This build's manifest plus `extra`, as a build that ships those migrations would have it.
    fn manifest(extra: &[&'static str]) -> Vec<&'static str> {
        schema::migration_versions().chain(extra.iter().copied()).collect()
    }

    fn migration(version: &str, sql: &str) -> Migration {
        Migration::new(version, "ws18", sql)
    }

    fn exists(conn: &Connection, sql: &str) -> bool {
        conn.prepare(sql).unwrap().exists([]).unwrap()
    }

    fn versions_like(conn: &Connection, pattern: &str) -> Vec<String> {
        conn.prepare("SELECT version FROM schema_migrations WHERE version LIKE ? ORDER BY rowid")
            .unwrap()
            .query_map([pattern], |r| r.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    }

    #[test]
    fn the_shipped_catalog_is_valid_and_newer_than_every_rails_migration() {
        let catalog = catalog();
        let manifest: Vec<_> = schema::migration_versions().collect();
        validate(&manifest, &catalog).unwrap();
        let newest_rails = schema::baseline_versions()
            .max_by(|a, b| version_order(a, b))
            .unwrap();
        for migration in &catalog {
            assert_eq!(migration.version.len(), 14, "{}", migration.version);
            assert_eq!(
                version_order(&migration.version, newest_rails),
                Ordering::Greater,
                "{} must sort after the last Rails migration {newest_rails}",
                migration.version
            );
        }
        // Every manifest version is either a Rails-era one or one this build can apply.
        let baseline: BTreeSet<_> = schema::baseline_versions().collect();
        for version in manifest {
            assert!(
                baseline.contains(version) || catalog.iter().any(|m| m.version == version),
                "{version} is in schema_migrations.txt but neither in the baseline nor the catalog"
            );
        }
    }

    #[test]
    fn an_up_to_date_database_has_nothing_pending() {
        let mut conn = prepared();
        assert!(pending_for_build(&conn).unwrap().is_empty());
        assert!(migrate(&mut conn).unwrap().is_empty());
    }

    #[test]
    fn applies_in_integer_order_once_and_records_rails_strings() {
        let mut conn = prepared();
        let catalog = [
            migration("29991231235959", "INSERT INTO ws18_order VALUES ('second');"),
            migration(
                "29990101000000",
                "CREATE TABLE ws18_order(value TEXT); INSERT INTO ws18_order VALUES ('first');",
            ),
        ];
        let manifest = manifest(&["29990101000000", "29991231235959"]);
        assert_eq!(
            migrate_with(&mut conn, &manifest, &catalog).unwrap(),
            ["29990101000000", "29991231235959"]
        );
        let values: Vec<String> = conn
            .prepare("SELECT value FROM ws18_order ORDER BY rowid")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(values, ["first", "second"]);
        assert_eq!(versions_like(&conn, "2999%"), ["29990101000000", "29991231235959"]);
        assert!(migrate_with(&mut conn, &manifest, &catalog).unwrap().is_empty());
        let mismatch = schema::schema_mismatch_against(&conn, &manifest).unwrap();
        assert!(mismatch.missing.is_empty() && mismatch.unknown.is_empty());
    }

    #[test]
    fn orders_different_length_decimal_versions_as_rails_integers() {
        let mut conn = prepared();
        let catalog = [
            migration("10", "INSERT INTO ws18_order VALUES ('second');"),
            migration("9", "CREATE TABLE ws18_order(value TEXT); INSERT INTO ws18_order VALUES ('first');"),
        ];
        assert_eq!(
            migrate_with(&mut conn, &manifest(&["9", "10"]), &catalog).unwrap(),
            ["9", "10"]
        );
    }

    #[test]
    fn a_failure_anywhere_applies_nothing() {
        let versions = ["29990101000000", "29991231235959"];
        for (second, block_version_insert) in [
            ("CREATE TABLE ws18_failure(id INTEGER); SELECT missing FROM ws18_failure;", false),
            ("CREATE TABLE ws18_failure(id INTEGER);", true),
        ] {
            let mut conn = prepared();
            if block_version_insert {
                conn.execute_batch("CREATE TRIGGER ws18_no_version BEFORE INSERT ON schema_migrations WHEN NEW.version='29991231235959' BEGIN SELECT RAISE(ABORT, 'injected'); END;").unwrap();
            }
            let catalog = [
                migration(versions[0], "CREATE TABLE ws18_success(id INTEGER);"),
                migration(versions[1], second),
            ];
            assert!(migrate_with(&mut conn, &manifest(&versions), &catalog).is_err());
            assert!(!exists(&conn, "SELECT 1 FROM sqlite_schema WHERE name IN ('ws18_success', 'ws18_failure')"));
            assert!(versions_like(&conn, "2999%").is_empty());
            let foreign_keys: bool = conn.query_row("PRAGMA foreign_keys", [], |r| r.get(0)).unwrap();
            assert!(foreign_keys, "enforcement is restored after a failure");
        }
    }

    #[test]
    fn foreign_key_violations_roll_everything_back() {
        let mut conn = prepared();
        let catalog = [migration(
            "29990101000000",
            "CREATE TABLE ws18_parent(id INTEGER PRIMARY KEY); \
             CREATE TABLE ws18_child(parent_id INTEGER REFERENCES ws18_parent(id)); \
             INSERT INTO ws18_child VALUES (42);",
        )];
        let error = migrate_with(&mut conn, &manifest(&["29990101000000"]), &catalog).unwrap_err();
        assert!(matches!(error, Error::ForeignKeys(ref table) if table == "ws18_child"), "{error}");
        assert!(!exists(&conn, "SELECT 1 FROM sqlite_schema WHERE name='ws18_child'"));
    }

    #[test]
    fn a_table_rebuild_runs_with_enforcement_off_and_is_checked_after() {
        let mut conn = prepared();
        conn.execute_batch(
            "CREATE TABLE ws18_parent(id INTEGER PRIMARY KEY, name TEXT); \
             CREATE TABLE ws18_child(parent_id INTEGER REFERENCES ws18_parent(id)); \
             INSERT INTO ws18_parent VALUES (1, 'a'); INSERT INTO ws18_child VALUES (1);",
        )
        .unwrap();
        // SQLite's documented twelve-step rebuild: dropping the parent would fail (or cascade)
        // with enforcement on.
        let catalog = [migration(
            "29990101000000",
            "CREATE TABLE ws18_parent_new(id INTEGER PRIMARY KEY, name TEXT NOT NULL); \
             INSERT INTO ws18_parent_new SELECT id, name FROM ws18_parent; \
             DROP TABLE ws18_parent; \
             ALTER TABLE ws18_parent_new RENAME TO ws18_parent;",
        )];
        migrate_with(&mut conn, &manifest(&["29990101000000"]), &catalog).unwrap();
        let children: i64 = conn.query_row("SELECT count(*) FROM ws18_child", [], |r| r.get(0)).unwrap();
        assert_eq!(children, 1);
    }

    #[test]
    fn refuses_unknown_versions_as_a_downgrade_before_writing() {
        let mut conn = prepared();
        conn.execute("INSERT INTO schema_migrations(version) VALUES ('29991231235959')", [])
            .unwrap();
        let catalog = [migration("29990101000000", "CREATE TABLE ws18_failure(id INTEGER);")];
        let error = migrate_with(&mut conn, &manifest(&["29990101000000"]), &catalog).unwrap_err();
        assert!(error.to_string().contains("unknown migrations 29991231235959"), "{error}");
        assert!(!exists(&conn, "SELECT 1 FROM sqlite_schema WHERE name='ws18_failure'"));
        // The shipped build refuses it too.
        assert!(migrate(&mut conn).is_err());
    }

    #[test]
    fn refuses_a_database_missing_versions_it_cannot_apply() {
        let mut conn = prepared();
        let newest = schema::baseline_versions().next().unwrap();
        conn.execute("DELETE FROM schema_migrations WHERE version=?", [newest])
            .unwrap();
        let error = migrate(&mut conn).unwrap_err();
        assert!(error.to_string().contains(&format!("missing migrations {newest}")), "{error}");
    }

    #[test]
    fn rejects_duplicate_noncanonical_and_unlisted_versions_before_writes() {
        for (versions, listed) in [
            (vec!["29990101000000", "29990101000000"], vec!["29990101000000"]),
            (vec!["029990101000000"], vec!["029990101000000"]),
            (vec!["version"], vec!["version"]),
            (vec!["0"], vec!["0"]),
            (vec!["29990101000000"], vec![]),
        ] {
            let mut conn = prepared();
            let catalog: Vec<_> = versions
                .iter()
                .map(|v| migration(v, "CREATE TABLE ws18_failure(id INTEGER);"))
                .collect();
            let manifest: Vec<&str> = schema::migration_versions().chain(listed).collect();
            assert!(
                matches!(migrate_with(&mut conn, &manifest, &catalog), Err(Error::Catalog(_))),
                "{versions:?}"
            );
            assert!(!exists(&conn, "SELECT 1 FROM sqlite_schema WHERE name='ws18_failure'"));
        }
    }

    #[test]
    fn migration_sql_cannot_escape_its_transaction_change_settings_or_forge_versions() {
        for sql in [
            "CREATE TABLE ws18_failure(id INTEGER); COMMIT;",
            "CREATE TABLE ws18_failure(id INTEGER); SAVEPOINT ws18; RELEASE ws18;",
            "CREATE TABLE ws18_failure(id INTEGER); INSERT INTO schema_migrations(version) VALUES ('29991231235959');",
            "CREATE TABLE ws18_failure(id INTEGER); PRAGMA writable_schema = ON;",
            "CREATE TABLE ws18_failure(id INTEGER); ATTACH ':memory:' AS other;",
        ] {
            let mut conn = prepared();
            let catalog = [migration("29990101000000", sql)];
            assert!(migrate_with(&mut conn, &manifest(&["29990101000000"]), &catalog).is_err(), "{sql}");
            assert!(!exists(&conn, "SELECT 1 FROM sqlite_schema WHERE name='ws18_failure'"), "{sql}");
            assert!(versions_like(&conn, "2999%").is_empty(), "{sql}");
        }
    }
}
