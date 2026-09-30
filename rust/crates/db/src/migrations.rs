//! Explicit, post-cutover migrations. Server boot continues to use `schema::prepare` only.

use rusqlite::{
    Connection, TransactionBehavior,
    hooks::{AuthAction, AuthContext, Authorization},
};
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct Migration {
    pub version: String,
    pub sql: String,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid migration catalog: {0}")]
    Catalog(String),
    #[error("{0}")]
    Schema(crate::schema::SchemaMismatch),
    #[error("migration {version}: {source}")]
    Migration { version: String, source: rusqlite::Error },
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Database(#[from] crate::Error),
}

/// Rails stores `Migration#version.to_s`: canonical decimal strings, without leading zeroes.
/// The caller supplies a complete catalog of post-cutover migrations, including already run ones.
/// Each migration and its version insert commit together. Prior successful migrations remain
/// committed if a later migration fails, matching Rails' per-migration transactions.
pub fn apply_pending(conn: &mut Connection, migrations: &[Migration]) -> Result<Vec<String>, Error> {
    let mut catalog = BTreeSet::new();
    for migration in migrations {
        let version = &migration.version;
        if version.is_empty() || version.starts_with('0') || !version.bytes().all(|b| b.is_ascii_digit()) {
            return Err(Error::Catalog(format!("noncanonical version {version:?}")));
        }
        if !catalog.insert(version.as_str()) {
            return Err(Error::Catalog(format!("duplicate version {version}")));
        }
    }
    let mut ordered: Vec<_> = migrations.iter().collect();
    ordered.sort_by(|a, b| a.version.len().cmp(&b.version.len()).then(a.version.cmp(&b.version)));
    check_schema(conn, &catalog)?;
    let mut applied = Vec::new();
    for migration in ordered {
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        // Check again while holding the write lock: concurrent runners may have advanced it.
        check_schema(&tx, &catalog)?;
        if tx
            .prepare("SELECT 1 FROM schema_migrations WHERE version = ?")?
            .exists([&migration.version])?
        {
            continue;
        }
        tx.authorizer(Some(migration_authorizer));
        let result = tx.execute_batch(&migration.sql);
        tx.authorizer(None::<fn(AuthContext<'_>) -> Authorization>);
        result.map_err(|source| Error::Migration {
            version: migration.version.clone(),
            source,
        })?;
        tx.execute(
            "INSERT INTO schema_migrations(version) VALUES (?)",
            [&migration.version],
        )
        .map_err(|source| Error::Migration {
            version: migration.version.clone(),
            source,
        })?;
        tx.commit().map_err(|source| Error::Migration {
            version: migration.version.clone(),
            source,
        })?;
        applied.push(migration.version.clone());
    }
    Ok(applied)
}

fn check_schema(conn: &Connection, catalog: &BTreeSet<&str>) -> Result<(), Error> {
    let mut mismatch = crate::schema::schema_mismatch(conn)?;
    mismatch.unknown.retain(|version| !catalog.contains(version.as_str()));
    if mismatch.missing.is_empty() && mismatch.unknown.is_empty() {
        Ok(())
    } else {
        Err(Error::Schema(mismatch))
    }
}

fn migration_authorizer(ctx: AuthContext<'_>) -> Authorization {
    match ctx.action {
        // SQL must not commit behind the runner's back or tamper with its version ledger.
        AuthAction::Transaction { .. }
        | AuthAction::Savepoint { .. }
        | AuthAction::Attach { .. }
        | AuthAction::Detach { .. } => Authorization::Deny,
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

    fn prepared() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        schema::prepare(&mut conn, "production", &SystemClock).unwrap();
        conn
    }

    fn migration(version: &str, sql: &str) -> Migration {
        Migration {
            version: version.into(),
            sql: sql.into(),
        }
    }

    #[test]
    fn orders_versions_numerically_and_records_rails_strings_once() {
        let mut conn = prepared();
        let migrations = [
            migration("29991231235959", "INSERT INTO ws18_order VALUES ('second');"),
            migration(
                "29990101000000",
                "CREATE TABLE ws18_order(value TEXT); INSERT INTO ws18_order VALUES ('first');",
            ),
        ];
        assert_eq!(
            apply_pending(&mut conn, &migrations).unwrap(),
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
        let versions: Vec<String> = conn
            .prepare("SELECT version FROM schema_migrations WHERE version LIKE '2999%' ORDER BY rowid")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(versions, ["29990101000000", "29991231235959"]);
        assert!(apply_pending(&mut conn, &migrations).unwrap().is_empty());
    }

    #[test]
    fn failed_sql_rolls_back_schema_rows_and_version() {
        let mut conn = prepared();
        let migrations = [migration(
            "29990101000000",
            "CREATE TABLE ws18_failure(id INTEGER); INSERT INTO ws18_failure VALUES (1); SELECT missing FROM ws18_failure;",
        )];
        assert!(apply_pending(&mut conn, &migrations).is_err());
        assert!(
            !conn
                .prepare("SELECT 1 FROM sqlite_schema WHERE name='ws18_failure'")
                .unwrap()
                .exists([])
                .unwrap()
        );
        assert!(
            !conn
                .prepare("SELECT 1 FROM schema_migrations WHERE version='29990101000000'")
                .unwrap()
                .exists([])
                .unwrap()
        );
    }

    #[test]
    fn orders_different_length_decimal_versions_as_rails_integers() {
        let mut conn = prepared();
        assert_eq!(
            apply_pending(
                &mut conn,
                &[
                    migration("10", "INSERT INTO ws18_order VALUES ('second');"),
                    migration(
                        "9",
                        "CREATE TABLE ws18_order(value TEXT); INSERT INTO ws18_order VALUES ('first');"
                    ),
                ]
            )
            .unwrap(),
            ["9", "10"]
        );
    }

    #[test]
    fn failed_version_insert_rolls_back_migration_and_retains_prior_success() {
        let mut conn = prepared();
        conn.execute_batch("CREATE TRIGGER ws18_no_version BEFORE INSERT ON schema_migrations WHEN NEW.version='29991231235959' BEGIN SELECT RAISE(ABORT, 'injected'); END;").unwrap();
        let migrations = [
            migration("29990101000000", "CREATE TABLE ws18_success(id INTEGER);"),
            migration("29991231235959", "CREATE TABLE ws18_failure(id INTEGER);"),
        ];
        assert!(apply_pending(&mut conn, &migrations).is_err());
        assert!(
            conn.prepare("SELECT 1 FROM sqlite_schema WHERE name='ws18_success'")
                .unwrap()
                .exists([])
                .unwrap()
        );
        assert!(
            !conn
                .prepare("SELECT 1 FROM sqlite_schema WHERE name='ws18_failure'")
                .unwrap()
                .exists([])
                .unwrap()
        );
        assert!(
            conn.prepare("SELECT 1 FROM schema_migrations WHERE version='29990101000000'")
                .unwrap()
                .exists([])
                .unwrap()
        );
    }

    #[test]
    fn rejects_duplicate_noncanonical_and_unknown_versions_before_writes() {
        for versions in [
            vec!["29990101000000", "29990101000000"],
            vec!["029990101000000"],
            vec!["version"],
            vec!["0"],
        ] {
            let mut conn = prepared();
            let migrations: Vec<_> = versions
                .iter()
                .map(|v| migration(v, "CREATE TABLE ws18_failure(id INTEGER);"))
                .collect();
            assert!(apply_pending(&mut conn, &migrations).is_err(), "{versions:?}");
            assert!(
                !conn
                    .prepare("SELECT 1 FROM sqlite_schema WHERE name='ws18_failure'")
                    .unwrap()
                    .exists([])
                    .unwrap()
            );
        }
        let mut conn = prepared();
        conn.execute("INSERT INTO schema_migrations(version) VALUES ('29991231235959')", [])
            .unwrap();
        assert!(
            apply_pending(
                &mut conn,
                &[migration("29990101000000", "CREATE TABLE ws18_failure(id INTEGER);")]
            )
            .is_err()
        );
    }

    #[test]
    fn migration_cannot_escape_its_transaction_or_forge_versions() {
        for sql in [
            "CREATE TABLE ws18_failure(id INTEGER); COMMIT;",
            "CREATE TABLE ws18_failure(id INTEGER); INSERT INTO schema_migrations(version) VALUES ('29991231235959');",
        ] {
            let mut conn = prepared();
            assert!(apply_pending(&mut conn, &[migration("29990101000000", sql)]).is_err());
            assert!(
                !conn
                    .prepare("SELECT 1 FROM sqlite_schema WHERE name='ws18_failure'")
                    .unwrap()
                    .exists([])
                    .unwrap()
            );
        }
    }
}
