//! Preservation verifier for `script/admin/verify-additive-sqlite-migration`.
use rusqlite::{
    Connection, OpenFlags,
    types::{Value, ValueRef},
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Input(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error("{0}")]
    Mismatch(String),
}

/// Both inputs are opened read-only. Output follows our Ruby verifier's MATCH/ADDITIVE contract.
/// Internal metadata and migration versions are intentionally excluded, as in the reference.
pub fn verify(before: &Path, after: &Path) -> Result<String, Error> {
    let before_meta = std::fs::metadata(before)?;
    let after_meta = std::fs::metadata(after)?;
    if !before_meta.is_file() || !after_meta.is_file() {
        return Err(Error::Input("inputs must be regular database files".into()));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if before_meta.dev() == after_meta.dev() && before_meta.ino() == after_meta.ino() {
            return Err(same_file());
        }
    }
    if std::fs::canonicalize(before)? == std::fs::canonicalize(after)? {
        return Err(same_file());
    }
    let before = readonly(before)?;
    let after = readonly(after)?;
    let mut output = String::new();
    let mut mismatches = 0;
    for (label, conn) in [("before", &before), ("after", &after)] {
        if rows(conn, "PRAGMA quick_check")? == [vec![Value::Text("ok".into())]] {
            output.push_str(&format!("{label}: integrity MATCH\n"));
        } else {
            mismatch(&mut output, &mut mismatches, label, "integrity check failed");
        }
    }
    let before_tables = table_names(&before)?;
    let after_tables = table_names(&after)?;
    for table in before_tables.iter().filter(|table| !after_tables.contains(table)) {
        mismatch(&mut output, &mut mismatches, table, "table is missing");
    }
    let mut additive_columns = 0;
    for table in &before_tables {
        if !after_tables.contains(table) {
            continue;
        }
        let columns = rows(&before, &format!("PRAGMA table_xinfo({})", quote(table)))?;
        let after_columns = rows(&after, &format!("PRAGMA table_xinfo({})", quote(table)))?;
        let mut schema_matches = columns.iter().all(|column| after_columns.contains(column));
        let before_indexes = indexes(&before, table)?;
        let after_indexes = indexes(&after, table)?;
        schema_matches &= before_indexes
            .iter()
            .all(|(name, signature)| after_indexes.get(name) == Some(signature));
        let before_foreign_keys = foreign_keys(&before, table)?;
        let after_foreign_keys = foreign_keys(&after, table)?;
        schema_matches &= before_foreign_keys
            .iter()
            .all(|(key, count)| after_foreign_keys.get(key).copied().unwrap_or(0) >= *count);
        let before_triggers = triggers(&before, table)?;
        let after_triggers = triggers(&after, table)?;
        schema_matches &= before_triggers.iter().all(|row| after_triggers.contains(row));
        if !schema_matches {
            mismatch(&mut output, &mut mismatches, table, "preexisting schema changed");
        }
        let names = columns
            .iter()
            .map(|column| match &column[1] {
                Value::Text(name) => Ok(quote(name)),
                _ => Err(Error::Input("invalid column metadata".into())),
            })
            .collect::<Result<Vec<_>, _>>()?
            .join(", ");
        let sql = format!("SELECT {names} FROM {}", quote(table));
        let old_rows = fingerprints(&before, &sql)?;
        let new_rows = fingerprints(&after, &sql);
        let data_matches = new_rows.as_ref().is_ok_and(|new| old_rows == *new);
        if !data_matches {
            mismatch(&mut output, &mut mismatches, table, "preexisting row data changed");
        }
        output.push_str(&format!(
            "{table}: rows {} -> {}, schema {}, data {}\n",
            old_rows.len(),
            new_rows.map(|rows| rows.len()).unwrap_or(0),
            verdict(schema_matches),
            verdict(data_matches)
        ));
        additive_columns += after_columns.len().saturating_sub(columns.len());
    }
    if mismatches == 0 {
        output.push_str(&format!(
            "MATCH: {} preexisting tables preserved\nADDITIVE: {} tables, {additive_columns} columns\n",
            before_tables.len(),
            after_tables.iter().filter(|t| !before_tables.contains(t)).count()
        ));
        Ok(output)
    } else {
        output.push_str(&format!("MISMATCH: {mismatches} preservation checks failed\n"));
        Err(Error::Mismatch(output))
    }
}

pub fn readonly(path: &Path) -> Result<Connection, Error> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    conn.busy_timeout(std::time::Duration::from_millis(crate::schema::BUSY_TIMEOUT_MS))?;
    conn.pragma_update(None, "query_only", true)?;
    Ok(conn)
}

fn quote(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}
fn same_file() -> Error {
    Error::Mismatch(
        "inputs: before and after must be separate database files MISMATCH\nMISMATCH: 1 preservation check failed\n"
            .into(),
    )
}
fn verdict(value: bool) -> &'static str {
    if value { "MATCH" } else { "MISMATCH" }
}
fn mismatch(output: &mut String, count: &mut usize, scope: &str, reason: &str) {
    *count += 1;
    output.push_str(&format!("{scope}: {reason} MISMATCH\n"));
}

fn rows(conn: &Connection, sql: &str) -> rusqlite::Result<Vec<Vec<Value>>> {
    let mut stmt = conn.prepare(sql)?;
    let count = stmt.column_count();
    stmt.query_map([], |row| (0..count).map(|i| row.get(i)).collect())?
        .collect()
}

fn table_names(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    conn.prepare("SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT IN ('ar_internal_metadata', 'schema_migrations') ORDER BY name")?
        .query_map([], |r| r.get(0))?.collect()
}

fn indexes(conn: &Connection, table: &str) -> rusqlite::Result<BTreeMap<String, Vec<Vec<Value>>>> {
    let mut signatures = BTreeMap::new();
    for row in rows(conn, &format!("PRAGMA index_list({})", quote(table)))? {
        let Value::Text(name) = &row[1] else { continue };
        let mut signature = vec![row[2..].to_vec()];
        signature.extend(rows(conn, &format!("PRAGMA index_xinfo({})", quote(name)))?);
        signatures.insert(name.clone(), signature);
    }
    Ok(signatures)
}

fn foreign_keys(conn: &Connection, table: &str) -> rusqlite::Result<BTreeMap<String, usize>> {
    let mut groups: BTreeMap<i64, Vec<Vec<Value>>> = BTreeMap::new();
    for row in rows(conn, &format!("PRAGMA foreign_key_list({})", quote(table)))? {
        let Value::Integer(id) = row[0] else { continue };
        groups.entry(id).or_default().push(row[1..].to_vec());
    }
    let mut counts = BTreeMap::new();
    for mut rows in groups.into_values() {
        rows.sort_by_key(|row| match row[0] {
            Value::Integer(seq) => seq,
            _ => 0,
        });
        // Debug includes typed Value variants and complete grouped constraints.
        let key = format!("{rows:?}");
        *counts.entry(key).or_insert(0) += 1;
    }
    Ok(counts)
}

fn triggers(conn: &Connection, table: &str) -> rusqlite::Result<Vec<(String, String)>> {
    conn.prepare("SELECT name, sql FROM sqlite_schema WHERE type='trigger' AND tbl_name=? ORDER BY name")?
        .query_map([table], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect()
}

fn fingerprints(conn: &Connection, sql: &str) -> rusqlite::Result<Vec<[u8; 32]>> {
    let mut stmt = conn.prepare(sql)?;
    let count = stmt.column_count();
    let mut digests: Vec<[u8; 32]> = stmt
        .query_map([], |row| {
            let mut digest = Sha256::new();
            for i in 0..count {
                // Matches the Ruby verifier's N/I/F/B/S + Q> length + typed bytes.
                let value = row.get_ref(i)?;
                let (tag, bytes): (u8, Vec<u8>) = match value {
                    ValueRef::Null => (b'N', Vec::new()),
                    ValueRef::Integer(n) => (b'I', n.to_be_bytes().to_vec()),
                    ValueRef::Real(n) => (b'F', n.to_be_bytes().to_vec()),
                    ValueRef::Text(bytes) => (b'S', bytes.to_vec()),
                    // Our pinned sqlite3 gem returns persisted BLOBs as String, not SQLite3::Blob.
                    // The Ruby verifier therefore fingerprints both TEXT and BLOB bytes as S.
                    ValueRef::Blob(bytes) => (b'S', bytes.to_vec()),
                };
                digest.update([tag]);
                digest.update((bytes.len() as u64).to_be_bytes());
                digest.update(bytes);
            }
            Ok(digest.finalize().into())
        })?
        .collect::<rusqlite::Result<_>>()?;
    digests.sort_unstable();
    Ok(digests)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Connection, SystemClock, schema};

    fn databases() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let before = dir.path().join("before.sqlite3");
        let after = dir.path().join("after.sqlite3");
        let mut conn = Connection::open(&before).unwrap();
        schema::prepare(&mut conn, "production", &SystemClock).unwrap();
        conn.execute_batch("CREATE TABLE ws18_data(id INTEGER PRIMARY KEY, text_value TEXT, blob_value BLOB, int_value INTEGER, float_value REAL); INSERT INTO ws18_data VALUES (1, 'preserve', X'00ff', 42, 1.25); CREATE INDEX ws18_index ON ws18_data(text_value); CREATE TRIGGER ws18_trigger AFTER INSERT ON ws18_data BEGIN SELECT 1; END;").unwrap();
        drop(conn);
        std::fs::copy(&before, &after).unwrap();
        (dir, before, after)
    }

    #[test]
    fn accepts_additive_tables_columns_and_rails_metadata_changes() {
        let (_dir, before, after) = databases();
        Connection::open(&after).unwrap().execute_batch("ALTER TABLE ws18_data ADD COLUMN optional TEXT; CREATE TABLE ws18_new(id INTEGER); INSERT INTO schema_migrations VALUES ('29990101000000');").unwrap();
        let output = verify(&before, &after).unwrap();
        assert!(output.contains("MATCH:"));
        assert!(output.contains("ADDITIVE: 1 tables, 1 columns"));
    }

    #[test]
    fn rejects_table_column_index_trigger_and_row_loss() {
        for sql in [
            "DROP TABLE ws18_data;",
            "ALTER TABLE ws18_data DROP COLUMN blob_value;",
            "DROP INDEX ws18_index;",
            "DROP TRIGGER ws18_trigger;",
            "UPDATE ws18_data SET text_value='changed';",
            "DELETE FROM ws18_data;",
            "INSERT INTO ws18_data VALUES (2, 'extra', NULL, 0, 0.0);",
        ] {
            let (_dir, before, after) = databases();
            Connection::open(&after).unwrap().execute_batch(sql).unwrap();
            assert!(verify(&before, &after).is_err(), "failed to reject {sql}");
        }
    }

    #[test]
    fn refuses_same_file_and_missing_database_without_creating_it() {
        let (_dir, before, after) = databases();
        assert!(verify(&before, &before).is_err());
        let alias = after.with_file_name("alias.sqlite3");
        std::fs::hard_link(&before, &alias).unwrap();
        assert!(verify(&before, &alias).is_err());
        let missing = after.with_file_name("missing.sqlite3");
        assert!(verify(&before, &missing).is_err());
        assert!(!missing.exists());
    }

    #[test]
    fn persisted_blob_and_text_bytes_share_the_reference_fingerprint() {
        let (_dir, before, after) = databases();
        Connection::open(&after)
            .unwrap()
            .execute_batch("UPDATE ws18_data SET blob_value=CAST(blob_value AS TEXT);")
            .unwrap();
        assert!(verify(&before, &after).is_ok());
    }

    #[test]
    fn foreign_keys_keep_grouping_actions_and_ignore_temporary_ids() {
        for (constraints, accepted) in [
            (
                "FOREIGN KEY (creator_id) REFERENCES users(id), FOREIGN KEY (message_id) REFERENCES messages(id)",
                true,
            ),
            (
                "FOREIGN KEY (creator_id) REFERENCES users(id) ON DELETE SET NULL",
                false,
            ),
            ("FOREIGN KEY (message_id) REFERENCES messages(id)", false),
        ] {
            let (_dir, before, after) = databases();
            for path in [&before, &after] {
                Connection::open(path).unwrap().execute_batch("CREATE TABLE ws18_children(id INTEGER PRIMARY KEY, creator_id INTEGER, message_id INTEGER, FOREIGN KEY(creator_id) REFERENCES users(id));").unwrap();
            }
            Connection::open(&after).unwrap().execute_batch(&format!("DROP TABLE ws18_children; CREATE TABLE ws18_children(id INTEGER PRIMARY KEY, creator_id INTEGER, message_id INTEGER, {constraints});")).unwrap();
            assert_eq!(verify(&before, &after).is_ok(), accepted, "{constraints}");
        }
        let (_dir, before, after) = databases();
        Connection::open(&before).unwrap().execute_batch("CREATE TABLE ws18_children(a INTEGER, b INTEGER, FOREIGN KEY(a,b) REFERENCES users(id,email_address));").unwrap();
        Connection::open(&after).unwrap().execute_batch("CREATE TABLE ws18_children(a INTEGER, b INTEGER, FOREIGN KEY(a) REFERENCES users(id), FOREIGN KEY(b) REFERENCES users(email_address));").unwrap();
        assert!(verify(&before, &after).is_err());
    }
}
