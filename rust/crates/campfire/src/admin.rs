//! Offline ops commands. No HTTP boot, job runner, or migration on server startup.
use campfire_db::{Connection, additive, migrations, schema};
use std::path::Path;

const USAGE: &str = "usage: campfire db-check [--immutable] DATABASE | db-migrate DATABASE MIGRATIONS_DIR | verify-additive-sqlite-migration BEFORE AFTER | twitter-backfill-references DATABASE";

pub fn run(args: &[String]) -> Option<i32> {
    if !matches!(
        args.first().map(String::as_str),
        Some("db-check" | "db-migrate" | "verify-additive-sqlite-migration" | "twitter-backfill-references")
    ) {
        return None;
    }
    Some(match execute(args) {
        Ok(output) => {
            print!("{output}");
            0
        }
        Err((1, message)) => {
            print!("{message}");
            1
        }
        Err((status, message)) => {
            eprintln!("{message}");
            status
        }
    })
}

fn execute(args: &[String]) -> Result<String, (i32, String)> {
    let fail = |error: anyhow::Error| (2, format!("ERROR: {error}"));
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["twitter-backfill-references", database] => twitter_backfill(Path::new(database)).map_err(fail),
        ["db-check", database] => check(Path::new(database), false).map_err(fail),
        ["db-check", "--immutable", database] => check(Path::new(database), true).map_err(fail),
        ["db-migrate", database, directory] => migrate(Path::new(database), Path::new(directory)).map_err(fail),
        ["verify-additive-sqlite-migration", before, after] => additive::verify(Path::new(before), Path::new(after))
            .map_err(|error| {
                let status = if matches!(error, additive::Error::Mismatch(_)) {
                    1
                } else {
                    2
                };
                (status, error.to_string())
            }),
        _ => Err((2, USAGE.into())),
    }
}

fn twitter_backfill(database: &Path) -> anyhow::Result<String> {
    // Refuse missing paths and incompatible schemas, without creating or migrating anything.
    check(database, false)?;
    let conn = Connection::open_with_flags(database, rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    conn.busy_timeout(std::time::Duration::from_millis(schema::BUSY_TIMEOUT_MS))?;
    conn.pragma_update(None, "foreign_keys", true)?;
    let registry = crate::jobs::registry();
    let config = campfire_jobs::RunnerConfig::new(
        [
            crate::jobs::DEFAULT_QUEUE,
            crate::jobs::PUSH_QUEUE,
            crate::jobs::WEBHOOKS_QUEUE,
            crate::jobs::SLACK_IMPORT_QUEUE,
        ]
        .map(|queue| campfire_jobs::QueueConfig::new(queue, 1))
        .to_vec(),
    );
    let (jobs, _ad_hoc) = crate::jobs::Jobs::new(&registry, &config)?;
    // Selection needs the real Action Text renderer (including attachables). Its
    // temporary signed markup is neither returned nor stored. Reuse the configured
    // signing base when present; an offline invocation without one uses a private,
    // ephemeral base. Signatures/local paths contain no plaintext post URLs.
    let signing_base = std::env::var("SECRET_KEY_BASE").ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let rich_text = crate::rich_text::AppRichText::new(
        std::sync::Arc::new(rails_compat::Secrets::new(&signing_base)),
        std::sync::Arc::new(campfire_kit::clock::SystemClock),
    );
    let env = campfire_db::Env {
        sink: std::sync::Arc::new(jobs),
        rich_text: std::sync::Arc::new(rich_text),
        ..Default::default()
    };
    twitter_backfill_with_env(&conn, &env)
}

fn twitter_backfill_with_env(conn: &Connection, env: &campfire_db::Env) -> anyhow::Result<String> {
    let count = crate::integrations::twitter::references::backfill_database(conn, env)?;
    Ok(format!(
        "Backfilled {count} {}\n",
        if count == 1 { "message" } else { "messages" }
    ))
}

fn check(database: &Path, immutable: bool) -> anyhow::Result<String> {
    let conn = if immutable {
        // This mode is only for a quiescent, standalone backup, never a live WAL database.
        let database = std::fs::canonicalize(database)?;
        let mut wal = database.as_os_str().to_os_string();
        wal.push("-wal");
        anyhow::ensure!(
            !std::fs::metadata(Path::new(&wal)).is_ok_and(|metadata| metadata.len() > 0),
            "immutable snapshot must have no nonempty WAL sidecar"
        );
        let encoded = percent_encoding::percent_encode(
            database.as_os_str().as_encoded_bytes(),
            percent_encoding::NON_ALPHANUMERIC,
        );
        Connection::open_with_flags(
            format!("file:{encoded}?immutable=1"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
        )?
    } else {
        additive::readonly(database)?
    };
    let integrity: String = conn.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
    anyhow::ensure!(integrity == "ok", "database integrity check failed");
    let mismatch = schema::schema_mismatch(&conn)?;
    anyhow::ensure!(mismatch.missing.is_empty() && mismatch.unknown.is_empty(), "{mismatch}");
    let counts: Vec<_> = ["users", "rooms", "messages"]
        .into_iter()
        .map(|table| conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get::<_, i64>(0)))
        .collect::<rusqlite::Result<_>>()?;
    Ok(format!(
        "SCHEMA: {} migration versions accepted (read-only)\nCOUNTS: users={} rooms={} messages={}\n",
        schema::migration_versions().count(),
        counts[0],
        counts[1],
        counts[2]
    ))
}

fn migrate(database: &Path, directory: &Path) -> anyhow::Result<String> {
    let mut catalog = Vec::new();
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();
        if path.extension().is_none_or(|extension| extension != "sql") {
            continue;
        }
        anyhow::ensure!(path.is_file(), "migration must be a regular file: {}", path.display());
        let name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow::anyhow!("invalid migration filename"))?;
        let version = name.split('_').next().unwrap_or_default();
        catalog.push(migrations::Migration {
            version: version.into(),
            sql: std::fs::read_to_string(&path)?,
        });
    }
    // No CREATE: a mistyped path must never initialize a second production database.
    let mut conn = Connection::open_with_flags(database, rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    conn.busy_timeout(std::time::Duration::from_millis(schema::BUSY_TIMEOUT_MS))?;
    conn.pragma_update(None, "foreign_keys", true)?;
    let applied = migrations::apply_pending(&mut conn, &catalog)?;
    let mut output = applied
        .iter()
        .map(|version| format!("MIGRATED: {version}\n"))
        .collect::<String>();
    output.push_str(&format!("MIGRATIONS: {} applied\n", applied.len()));
    Ok(output)
}

#[cfg(test)]
mod twitter_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twitter_operator_accepts_an_existing_database_and_prints_the_rake_summary() {
        let dir = tempfile::tempdir().unwrap();
        let database = dir.path().join("production.sqlite3");
        let mut conn = Connection::open(&database).unwrap();
        schema::prepare(&mut conn, "production", &campfire_db::SystemClock).unwrap();
        drop(conn);
        let args = [
            "twitter-backfill-references".into(),
            database.to_str().unwrap().into(),
        ];
        assert_eq!(execute(&args).unwrap(), "Backfilled 0 messages\n");
    }

    #[test]
    fn checks_existing_schema_readonly_and_rejects_unknown_versions() {
        let dir = tempfile::tempdir().unwrap();
        let database = dir.path().join("production.sqlite3");
        let mut conn = Connection::open(&database).unwrap();
        schema::prepare(&mut conn, "production", &campfire_db::SystemClock).unwrap();
        drop(conn);
        let before = std::fs::read(&database).unwrap();
        assert!(check(&database, false).unwrap().contains("read-only"));
        assert_eq!(std::fs::read(&database).unwrap(), before);
        Connection::open(&database)
            .unwrap()
            .execute("INSERT INTO schema_migrations VALUES ('29990101000000')", [])
            .unwrap();
        assert!(
            check(&database, false)
                .unwrap_err()
                .to_string()
                .contains("unknown migrations")
        );
        let missing = dir.path().join("missing.sqlite3");
        assert!(check(&missing, false).is_err());
        assert!(!missing.exists());
    }

    #[test]
    fn immutable_check_reads_standalone_snapshots_and_refuses_a_wal() {
        let dir = tempfile::tempdir().unwrap();
        let database = dir.path().join("snapshot #?.sqlite3");
        let mut conn = Connection::open(&database).unwrap();
        schema::prepare(&mut conn, "production", &campfire_db::SystemClock).unwrap();
        drop(conn);
        assert!(check(&database, true).unwrap().contains("read-only"));
        let mut wal = database.as_os_str().to_os_string();
        wal.push("-wal");
        std::fs::write(Path::new(&wal), "must not be ignored").unwrap();
        assert!(check(&database, true).unwrap_err().to_string().contains("WAL sidecar"));
    }

    #[test]
    fn explicit_migrate_reads_a_directory_and_reports_pending_versions() {
        let dir = tempfile::tempdir().unwrap();
        let database = dir.path().join("production.sqlite3");
        let mut conn = Connection::open(&database).unwrap();
        schema::prepare(&mut conn, "production", &campfire_db::SystemClock).unwrap();
        drop(conn);
        let migrations = dir.path().join("migrations");
        std::fs::create_dir(&migrations).unwrap();
        std::fs::write(
            migrations.join("29990101000000_add_ops.sql"),
            "CREATE TABLE ws18_ops(id INTEGER);",
        )
        .unwrap();
        assert_eq!(
            migrate(&database, &migrations).unwrap(),
            "MIGRATED: 29990101000000\nMIGRATIONS: 1 applied\n"
        );
        assert_eq!(migrate(&database, &migrations).unwrap(), "MIGRATIONS: 0 applied\n");
    }

    #[test]
    fn verifier_and_usage_exit_codes_match_reference() {
        assert_eq!(execute(&["verify-additive-sqlite-migration".into()]).unwrap_err().0, 2);
        assert_eq!(execute(&["db-check".into()]).unwrap_err().0, 2);
        assert!(run(&["server".into()]).is_none());
        let dir = tempfile::tempdir().unwrap();
        let before = dir.path().join("before.sqlite3");
        let after = dir.path().join("after.sqlite3");
        Connection::open(&before)
            .unwrap()
            .execute_batch("CREATE TABLE sentinel(id INTEGER); INSERT INTO sentinel VALUES(1);")
            .unwrap();
        std::fs::copy(&before, &after).unwrap();
        Connection::open(&after)
            .unwrap()
            .execute_batch("DELETE FROM sentinel;")
            .unwrap();
        let args = [
            "verify-additive-sqlite-migration".into(),
            before.display().to_string(),
            after.display().to_string(),
        ];
        assert_eq!(execute(&args).unwrap_err().0, 1);
    }
}
