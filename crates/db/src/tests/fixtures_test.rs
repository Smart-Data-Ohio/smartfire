//! The fixture loader against facts from a Ruby-loaded fixtures database, and row for row against
//! the frozen rows the reference loaded.

use super::*;
use crate::{Membership, Message, Role, Room, RoomType, Session, User};
use std::collections::BTreeMap;

#[test]
fn ids_are_label_crcs() {
    let t = TestDb::new();
    assert_eq!(t.read(|c| User::find(c, 127326141)).name, "David");
    assert_eq!(
        t.read(|c| Room::find(c, 654632876)).name.as_deref(),
        Some("Designers")
    );
}

#[test]
fn associations_enums_and_defaults() {
    let t = TestDb::new();
    let pets = t.read(|c| Room::find(c, id("pets")));
    assert_eq!(
        (pets.room_type, pets.creator_id),
        (RoomType::Open, id("david"))
    );

    assert_eq!(
        t.read(|c| User::find(c, id("david"))).role,
        Role::Administrator
    );
    assert_eq!(t.read(|c| User::find(c, id("jz"))).role, Role::Member);
    let bender = t.read(|c| User::find(c, id("bender")));
    // `Digest::SHA256.hexdigest("BenderToken1")`, evaluated by the fixture's ERB.
    assert_eq!(
        (bender.role, bender.bot_token_digest.as_deref()),
        (Role::Bot, Some(BENDER_TOKEN_DIGEST))
    );

    let kevin_designers = t.read(|c| Membership::find(c, id("kevin_designers")));
    assert_eq!(
        kevin_designers.involvement,
        Some(crate::Involvement::Mentions),
        "column default"
    );
    assert_eq!(kevin_designers.connections, 0);

    let rich_text: (i64, String) = t.read(|c| {
        Ok(c.query_row(
            "SELECT record_id, record_type FROM action_text_rich_texts WHERE id = ?",
            [id("first")],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?)
    });
    assert_eq!(rich_text, (id("first"), "Message".into()));
}

#[test]
fn erb_times_are_whole_seconds_and_default_timestamps_are_now() {
    let t = TestDb::new();
    let first = t.read(|c| Message::find(c, id("first")));
    assert_eq!(first.created_at.subsec_microsecond(), 0);
    assert!(first.updated_at > first.created_at);
    let session = t.read(|c| Session::find(c, id("david_safari")));
    assert_eq!(session.last_active_at.subsec_microsecond(), 0);
    assert!(
        (session.created_at.as_second() - session.last_active_at.as_second() - 7200).abs() <= 1
    );
}

#[test]
fn passwords_are_bcrypt_of_secret123456() {
    let t = TestDb::new();
    let david = t.read(|c| User::find(c, id("david")));
    assert!(david.authenticate("secret123456"));
    assert!(david.password_digest.unwrap().starts_with("$2a$"));
}

/// Every row of a table, sorted (some FTS shadow tables have no rowid), exactly, except the salted BCrypt digests: those are
/// their cost prefix (`$2a$12$`) and whether they verify `secret123456`, the fixtures' password.
fn dump(conn: &Connection, table: &str) -> Vec<String> {
    let mut stmt = conn.prepare(&format!("SELECT * FROM \"{table}\"")).unwrap();
    let names: Vec<String> = stmt.column_names().into_iter().map(String::from).collect();
    let mut rows = stmt.query([]).unwrap();
    let mut out = Vec::new();
    while let Some(row) = rows.next().unwrap() {
        // These rows pin the Rails-era schema, before port-owned migrations.
        if table == "schema_migrations" {
            let version: String = row.get("version").unwrap();
            if !crate::schema::baseline_versions().any(|baseline| baseline == version) {
                continue;
            }
        }
        let fields: Vec<String> = names
            .iter()
            .enumerate()
            .filter(|(i, name)| {
                if table == "rooms" && name.as_str() == "client_room_id" {
                    let key: Option<String> = row.get(*i).unwrap();
                    assert_eq!(key, None, "classic fixtures never set API creation keys");
                    false
                } else {
                    true
                }
            })
            // Port-only counter for the SPA activity badge that Rails doesn't have.
            .filter(|(_, name)| !(table == "users" && name.as_str() == "activity_revision"))
            .map(|(i, name)| {
                let value: rusqlite::types::Value = row.get(i).unwrap();
                let rendered = match (name.as_str(), value) {
                    ("password_digest", rusqlite::types::Value::Text(s)) => format!(
                        "bcrypt:{}:{}",
                        &s[..7],
                        bcrypt::verify("secret123456", &s).unwrap()
                    ),
                    (_, v) => format!("{v:?}"),
                };
                format!("{name}={rendered}")
            })
            .collect();
        out.push(fields.join(" "));
    }
    out.sort();
    out
}

fn tables(conn: &Connection) -> Vec<String> {
    crate::sql::query_all(
        conn,
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name <> 'ar_internal_metadata' ORDER BY name",
        [],
        |r| r.get(0),
    )
    .unwrap()
}

/// The fixture set names `db:fixtures:load` loads: every `.yml` under test/fixtures but `files/`.
fn fixture_sets() -> Vec<String> {
    fn walk(dir: &std::path::Path, root: &std::path::Path, out: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                if path != root.join("files") {
                    walk(&path, root, out);
                }
            } else if path.extension().is_some_and(|e| e == "yml") {
                let relative = path.strip_prefix(root).unwrap().with_extension("");
                out.push(relative.to_string_lossy().into_owned());
            }
        }
    }
    let root = crate::fixtures::reference_dir();
    let mut out = Vec::new();
    walk(&root, &root, &mut out);
    out.sort();
    out
}

/// The instant `fixtures_rails_rows.json` was recorded at: nonzero microseconds keep Rails'
/// six-digit timestamp representation.
const FROZEN_FIXTURES_NOW: &str = "2026-03-02 16:00:00.123456";

/// Compares `actual` (table name to rows) with the frozen file `src/tests/NAME`, or rewrites the
/// file from it when `CAMPFIRE_FIXTURES_DUMP=write` (after a deliberate fixture or schema change).
pub(super) fn assert_frozen_rows(name: &str, actual: &BTreeMap<String, Vec<String>>) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/tests")
        .join(name);
    if std::env::var("CAMPFIRE_FIXTURES_DUMP").as_deref() == Ok("write") {
        std::fs::write(&path, serde_json::to_string_pretty(actual).unwrap() + "\n").unwrap();
        return;
    }
    let expected: BTreeMap<String, Vec<String>> =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let names = |rows: &BTreeMap<String, Vec<String>>| rows.keys().cloned().collect::<Vec<_>>();
    assert_eq!(names(actual), names(&expected), "tables differ from {name}");
    let mismatches: Vec<String> = expected
        .iter()
        .filter(|(table, rows)| &actual[*table] != *rows)
        .map(|(table, rows)| {
            let only_frozen: Vec<_> = rows.iter().filter(|r| !actual[table].contains(r)).collect();
            let only_rust: Vec<_> = actual[table].iter().filter(|r| !rows.contains(r)).collect();
            format!("{table}:\n  frozen only: {only_frozen:#?}\n  rust only: {only_rust:#?}")
        })
        .collect();
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

/// `fixtures_match_ruby_row_for_row` without the reference: the rows the reference's
/// `db:fixtures:load` wrote at `FROZEN_FIXTURES_NOW` (pinned Rails, recorded 2026-10-05 when
/// that test last passed), every table but `ar_internal_metadata`.
#[test]
fn fixtures_match_frozen_rails_rows() {
    let now = crate::Timestamp::parse_db(FROZEN_FIXTURES_NOW).unwrap();
    // bcrypt-ruby's default cost, which the fixtures' `BCrypt::Password.create` uses.
    let t = TestDb::with_clock(crate::TestClock::frozen_at(now), 12);
    let rows = t.read(|c| {
        Ok(tables(c)
            .into_iter()
            .map(|table| (table.clone(), dump(c, &table)))
            .collect())
    });
    assert_frozen_rows("fixtures_rails_rows.json", &rows);
    assert_eq!(fixture_sets().len(), 20);
}
