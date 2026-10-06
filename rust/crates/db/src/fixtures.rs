//! Loads the reference's `test/fixtures/*.yml` (the port's copy is `rust/fixtures/`) the way `ActiveRecord::FixtureSet` does, so Rust
//! tests run against the same rows as the Ruby tests:
//!
//! - ids are `Zlib.crc32(label) % (2**30 - 1)` unless given;
//! - `belongs_to` values are labels (`creator: :david`), polymorphic ones name the class
//!   (`record: first (Message)`); the foreign key is `<association>_id` unless the model names
//!   another ([`custom_foreign_key`]);
//! - enum names become their stored values (`role: administrator` is 1);
//! - `$LABEL` in a string value becomes the fixture's label;
//! - `created_at`/`updated_at` default to one `now` per fixture file;
//! - columns a fixture leaves out get the column default;
//! - hashes and arrays are stored as JSON (the `json` columns);
//! - each table is emptied, then filled, with foreign keys checked at commit.
//!
//! ERB is evaluated for the forms the fixtures use: `<%= N.<unit>.ago %>` and `.from_now`, plus
//! `+ N.<unit>` and `.to_fs(:db)`, `BCrypt::Password.create("...")` assigned to a local,
//! `Digest::SHA256.hexdigest("...")` (optionally sliced, `[0, 4]`) and `User.generate_bot_token`.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use jiff::SignedDuration;
use rusqlite::Connection;
use rusqlite::types::Value;
use serde_yaml::Value as Yaml;

use crate::error::{Error, Result};
use crate::models::{Involvement, Role, Status, user};
use crate::time::Timestamp;

/// `ActiveRecord::FixtureSet::MAX_ID`
pub const MAX_ID: u32 = (1 << 30) - 1;

/// `ActiveRecord::FixtureSet.identify(label)`
pub fn identify(label: &str) -> i64 {
    i64::from(crc32fast::hash(label.as_bytes()) % MAX_ID)
}

/// The port's own copy of the reference Rails app's static inputs: `rust/web/`, laid out like the
/// Rails app (`app/assets`, `app/javascript`, `vendor/javascript`, `public/`, `config/importmap.rb`,
/// `script/livekit-gateway`, ...). `CAMPFIRE_REFERENCE` at compile time names a reference Rails
/// app's root to read them, and the fixtures, from instead.
pub fn reference_root() -> PathBuf {
    match option_env!("CAMPFIRE_REFERENCE") {
        Some(root) => PathBuf::from(root),
        None => rust_root().join("web"),
    }
}

/// The reference's `test/fixtures`: `rust/fixtures/`.
pub fn reference_dir() -> PathBuf {
    match option_env!("CAMPFIRE_REFERENCE") {
        Some(root) => Path::new(root).join("test/fixtures"),
        None => rust_root().join("fixtures"),
    }
}

/// A file named by its path in the reference Rails app (`public/500.html`,
/// `test/fixtures/files/earth.png`, `script/livekit-gateway/package.json`), in the port's copy.
pub fn reference_path(path: &str) -> PathBuf {
    match path.strip_prefix("test/fixtures/") {
        Some(fixture) => reference_dir().join(fixture),
        None => reference_root().join(path),
    }
}

/// The Rails app itself (`CAMPFIRE_REFERENCE`, else the repository root that contains `rust/`), for
/// the checks that compare the port with Rails source it doesn't copy: `db/schema.rb`,
/// `db/migrate`, `app/models`, `app/views`, `config/icons.yml`.
pub fn rails_root() -> PathBuf {
    match option_env!("CAMPFIRE_REFERENCE") {
        Some(root) => PathBuf::from(root),
        None => rust_root().join(".."),
    }
}

/// `rust/` (crates/db -> crates -> rust).
fn rust_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[derive(Debug, Clone)]
pub struct Options {
    /// Base time for `N.minutes.ago` and the default timestamps.
    pub now: Timestamp,
    /// BCrypt cost for `BCrypt::Password.create` (bcrypt-ruby's default is 12).
    pub bcrypt_cost: u32,
}

/// What was loaded: table -> label -> id.
#[derive(Debug, Clone, Default)]
pub struct Loaded {
    pub ids: BTreeMap<String, BTreeMap<String, i64>>,
}

impl Loaded {
    pub fn id(&self, table: &str, label: &str) -> Option<i64> {
        self.ids.get(table)?.get(label).copied()
    }
}

/// `belongs_to` associations whose foreign key isn't `<association>_id`: the `foreign_key:`
/// option in the model.
fn custom_foreign_key(table: &str, association: &str) -> Option<&'static str> {
    match (table, association) {
        // app/models/twitter/post_reference.rb: `belongs_to :post, foreign_key: :twitter_post_id`
        ("twitter_post_references", "post") => Some("twitter_post_id"),
        // app/models/event.rb: `belongs_to :venue, foreign_key: :venue_room_id`
        ("events", "venue") => Some("venue_room_id"),
        _ => None,
    }
}

/// A fixture key that names a `belongs_to` rather than a column.
enum Association {
    BelongsTo { column: String },
    Polymorphic { id: String, type_column: String },
}

/// `TableRow#resolve_sti_reflections`, from the schema: a key that isn't a column but has an
/// `_id` column (and, for polymorphic ones, a `_type` column) is an association.
fn association(table: &str, key: &str, columns: &[Column]) -> Option<Association> {
    let has = |name: &str| columns.iter().any(|c| c.name == name);
    if has(key) {
        return None;
    }
    if let Some(column) = custom_foreign_key(table, key) {
        return Some(Association::BelongsTo {
            column: column.into(),
        });
    }
    let id = format!("{key}_id");
    let type_column = format!("{key}_type");
    match (has(&id), has(&type_column)) {
        (true, true) => Some(Association::Polymorphic { id, type_column }),
        (true, false) => Some(Association::BelongsTo { column: id }),
        _ => None,
    }
}

/// Enum attributes: name -> stored value.
fn resolve_enum(table: &str, column: &str, value: &str) -> Option<Value> {
    match (table, column) {
        ("users", "role") => Role::from_name(value).map(|r| Value::Integer(r as i64)),
        ("users", "status") => Status::from_name(value).map(|s| Value::Integer(s as i64)),
        // String-backed enums (`index_by(&:itself)`) store the name itself; this one is checked
        // because the Rust enum must know every value the fixtures use.
        ("memberships", "involvement") => {
            Involvement::from_name(value).map(|i| Value::Text(i.name().into()))
        }
        _ => None,
    }
}

/// Loads every `*.yml` under `dir` into `conn`, inside the caller's transaction or its own.
pub fn load(conn: &Connection, dir: &Path, options: &Options) -> Result<Loaded> {
    let mut files = Vec::new();
    collect_yaml_files(dir, dir, &mut files)?;
    files.sort();

    let autocommit = conn.is_autocommit();
    if autocommit {
        conn.execute_batch("BEGIN IMMEDIATE TRANSACTION")?;
    }
    let result = load_files(conn, dir, &files, options);
    if autocommit {
        match &result {
            Ok(_) => conn.execute_batch("COMMIT TRANSACTION")?,
            Err(_) => conn.execute_batch("ROLLBACK TRANSACTION")?,
        }
    }
    result
}

fn load_files(
    conn: &Connection,
    dir: &Path,
    files: &[PathBuf],
    options: &Options,
) -> Result<Loaded> {
    conn.execute_batch("PRAGMA defer_foreign_keys = ON")?;
    let mut erb = Erb::new(options);
    let mut loaded = Loaded::default();
    for file in files {
        let table = table_name(dir, file);
        let source = std::fs::read_to_string(file)
            .map_err(|e| Error::Other(format!("{}: {e}", file.display())))?;
        let yaml = erb.render(&source)?;
        let rows: Yaml = serde_yaml::from_str(&yaml)
            .map_err(|e| Error::Other(format!("{}: {e}", file.display())))?;

        conn.execute(&format!(r#"DELETE FROM "{table}""#), [])?;
        let columns = table_columns(conn, &table)?;
        let now = Timestamp::from_jiff(options.now.jiff());
        let labels = loaded.ids.entry(table.clone()).or_default();

        let Yaml::Mapping(rows) = rows else { continue };
        for (label, row) in rows {
            let label = scalar_string(&label)
                .ok_or_else(|| Error::Other(format!("bad label in {table}")))?;
            if label == "DEFAULTS" || label == "_fixture" {
                continue;
            }
            let row = fixture_row(&table, &label, row, &columns, now)?;
            let id = match row.get("id") {
                Some(Value::Integer(id)) => *id,
                _ => identify(&label),
            };
            labels.insert(label, id);
            insert_row(conn, &table, &row)?;
        }
    }
    Ok(loaded)
}

fn fixture_row(
    table: &str,
    label: &str,
    row: Yaml,
    columns: &[Column],
    now: Timestamp,
) -> Result<BTreeMap<String, Value>> {
    let mut values: BTreeMap<String, Value> = BTreeMap::new();
    let mapping = match row {
        Yaml::Mapping(m) => m,
        Yaml::Null => Default::default(),
        _ => {
            return Err(Error::Other(format!(
                "fixture {table}.{label} is not a mapping"
            )));
        }
    };

    for (key, value) in mapping {
        let key = scalar_string(&key)
            .ok_or_else(|| Error::Other(format!("bad key in {table}.{label}")))?;
        match association(table, &key, columns) {
            Some(Association::BelongsTo { column }) => {
                let target = scalar_string(&value)
                    .ok_or_else(|| Error::Other(format!("{table}.{label}.{key}")))?;
                values.insert(column, Value::Integer(identify(target.trim_start_matches(':'))));
            }
            Some(Association::Polymorphic { id, type_column }) => {
                let target = scalar_string(&value)
                    .ok_or_else(|| Error::Other(format!("{table}.{label}.{key}")))?;
                // "label (Type)"; without the type only the id is set, as in Rails.
                let target_label = match target
                    .trim()
                    .strip_suffix(')')
                    .and_then(|t| t.rsplit_once('('))
                {
                    Some((l, class)) => {
                        values.insert(type_column, Value::Text(class.to_string()));
                        l.trim().to_string()
                    }
                    None => target,
                };
                values.insert(
                    id,
                    Value::Integer(identify(target_label.trim_start_matches(':'))),
                );
            }
            None => {
                let column = columns.iter().find(|c| c.name == key).ok_or_else(|| {
                    Error::Other(format!("{table}.{label}: no column or association {key}"))
                })?;
                let value = yaml_to_sql(table, column, value, label)?;
                values.insert(key, value);
            }
        }
    }

    values
        .entry("id".into())
        .or_insert_with(|| Value::Integer(identify(label)));
    for column in ["created_at", "updated_at"] {
        if columns.iter().any(|c| c.name == column) {
            values
                .entry(column.into())
                .or_insert_with(|| Value::Text(now.to_db()));
        }
    }
    Ok(values)
}

fn yaml_to_sql(table: &str, column: &Column, value: Yaml, label: &str) -> Result<Value> {
    Ok(match value {
        Yaml::Null => Value::Null,
        Yaml::Bool(b) => Value::Integer(b as i64),
        Yaml::Number(n) => match n.as_i64() {
            Some(i) => Value::Integer(i),
            None => Value::Real(n.as_f64().unwrap_or_default()),
        },
        Yaml::String(s) => {
            // `interpolate_label`
            let s = s.replace("$LABEL", label);
            if let Some(v) = resolve_enum(table, &column.name, &s) {
                v
            } else if column.is_datetime() {
                // Time columns cast the text (e.g. ERB's "2026-09-26 11:24:38 UTC").
                match Timestamp::parse_db(&s) {
                    Some(ts) => Value::Text(ts.to_db()),
                    None => Value::Text(s),
                }
            } else {
                Value::Text(s)
            }
        }
        // A hash or array: the json type serializes it.
        other => Value::Text(
            serde_json::to_string(&other)
                .map_err(|e| Error::Other(format!("{table}.{label}.{}: {e}", column.name)))?,
        ),
    })
}

fn insert_row(conn: &Connection, table: &str, row: &BTreeMap<String, Value>) -> Result<()> {
    let columns: Vec<String> = row.keys().map(|c| format!(r#""{c}""#)).collect();
    let sql = format!(
        r#"INSERT INTO "{table}" ({}) VALUES ({})"#,
        columns.join(", "),
        crate::sql::placeholders(row.len())
    );
    conn.execute(&sql, rusqlite::params_from_iter(row.values()))?;
    Ok(())
}

/// A column's name and declared type, from `PRAGMA table_info`.
struct Column {
    name: String,
    declared_type: String,
}

impl Column {
    fn is_datetime(&self) -> bool {
        self.declared_type.starts_with("datetime")
    }
}

fn table_columns(conn: &Connection, table: &str) -> Result<Vec<Column>> {
    let mut stmt = conn.prepare(&format!(r#"PRAGMA table_info("{table}")"#))?;
    let columns = stmt
        .query_map([], |r| {
            Ok(Column {
                name: r.get("name")?,
                declared_type: r.get("type")?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if columns.is_empty() {
        return Err(Error::Other(format!("no table {table}")));
    }
    Ok(columns)
}

/// `action_text/rich_texts.yml` -> `action_text_rich_texts`, `push/subscriptions.yml` ->
/// `push_subscriptions` (the model's table name).
fn table_name(dir: &Path, file: &Path) -> String {
    let relative = file.strip_prefix(dir).unwrap_or(file).with_extension("");
    relative.to_string_lossy().replace(['/', '\\'], "_")
}

fn collect_yaml_files(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in
        std::fs::read_dir(dir).map_err(|e| Error::Other(format!("{}: {e}", dir.display())))?
    {
        let path = entry.map_err(|e| Error::Other(e.to_string()))?.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n != "files") {
                collect_yaml_files(root, &path, files)?;
            }
        } else if path.extension().is_some_and(|e| e == "yml") {
            files.push(path);
        }
    }
    let _ = root;
    Ok(())
}

fn scalar_string(value: &Yaml) -> Option<String> {
    match value {
        Yaml::String(s) => Some(s.clone()),
        Yaml::Number(n) => Some(n.to_string()),
        Yaml::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// The sliver of ERB the fixtures use.
struct Erb<'a> {
    options: &'a Options,
    locals: HashMap<String, String>,
    digests: HashMap<String, String>,
}

impl<'a> Erb<'a> {
    fn new(options: &'a Options) -> Self {
        Self {
            options,
            locals: HashMap::new(),
            digests: HashMap::new(),
        }
    }

    fn render(&mut self, source: &str) -> Result<String> {
        let mut out = String::new();
        let mut rest = source;
        while let Some(start) = rest.find("<%") {
            out.push_str(&rest[..start]);
            let after = &rest[start + 2..];
            let end = after
                .find("%>")
                .ok_or_else(|| Error::Other("unterminated ERB tag".into()))?;
            let (output, code) = match after.strip_prefix('=') {
                Some(code) => (true, &code[..end - 1]),
                None => (false, &after[..end]),
            };
            let value = self.evaluate(code.trim())?;
            if output {
                out.push_str(&value);
            }
            rest = &after[end + 2..];
            // `<% %>` on its own line leaves the newline, as ERB without trim mode does.
        }
        out.push_str(rest);
        Ok(out)
    }

    fn evaluate(&mut self, code: &str) -> Result<String> {
        if let Some((name, expression)) = code.split_once(" = ") {
            let value = self.evaluate(expression.trim())?;
            self.locals.insert(name.trim().to_string(), value);
            return Ok(String::new());
        }
        if let Some(value) = self.locals.get(code) {
            return Ok(value.clone());
        }
        if let Some(password) = code
            .strip_prefix("BCrypt::Password.create(")
            .and_then(|c| c.strip_suffix(')'))
        {
            let password = password.trim().trim_matches('"').to_string();
            if let Some(digest) = self.digests.get(&password) {
                return Ok(digest.clone());
            }
            let digest = user::password_digest(&password, self.options.bcrypt_cost)?;
            self.digests.insert(password, digest.clone());
            return Ok(digest);
        }
        if code == "User.generate_bot_token" {
            return Ok(user::generate_bot_token());
        }
        if let Some(hexdigest) = sha256_hexdigest(code) {
            return Ok(hexdigest);
        }
        let (expression, db_format) = match code.strip_suffix(".to_fs(:db)") {
            Some(expression) => (expression, true),
            None => (code, false),
        };
        if let Some(offset) = parse_relative_time(expression) {
            let at = Timestamp::from_second(self.options.now.as_second()).since(offset);
            // `to_fs(:db)` drops the zone; `TimeWithZone#to_s` in UTC keeps it. Whole seconds.
            return Ok(if db_format {
                at.to_db()
            } else {
                format!("{} UTC", at.to_db())
            });
        }
        Err(Error::Other(format!("unsupported ERB in fixture: {code}")))
    }
}

/// `Digest::SHA256.hexdigest("...")`, optionally sliced as `String#[start, length]`.
fn sha256_hexdigest(code: &str) -> Option<String> {
    use sha2::{Digest, Sha256};
    let rest = code.strip_prefix("Digest::SHA256.hexdigest(\"")?;
    let (input, rest) = rest.split_once("\")")?;
    let hexdigest = hex::encode(Sha256::digest(input.as_bytes()));
    match rest.trim() {
        "" => Some(hexdigest),
        slice => {
            let (start, length) = slice
                .strip_prefix('[')?
                .strip_suffix(']')?
                .split_once(',')?;
            let start: usize = start.trim().parse().ok()?;
            let length: usize = length.trim().parse().ok()?;
            hexdigest
                .get(start..(start + length).min(hexdigest.len()))
                .map(str::to_string)
        }
    }
}

/// `1.hour.ago`, `36.minutes.ago`, `2.days.from_now + 1.hour`...: the offset from now.
fn parse_relative_time(code: &str) -> Option<SignedDuration> {
    let mut terms = code.split(" + ");
    let base = terms.next()?.trim();
    let (duration, direction) = base.rsplit_once('.')?;
    let mut offset = match direction {
        "ago" => -parse_duration(duration)?,
        "from_now" => parse_duration(duration)?,
        _ => return None,
    };
    for term in terms {
        offset += parse_duration(term.trim())?;
    }
    Some(offset)
}

/// `1.hour`, `36.minutes`, `2.days`...
fn parse_duration(code: &str) -> Option<SignedDuration> {
    let (n, unit) = code.split_once('.')?;
    let n: i64 = n.trim().parse().ok()?;
    let seconds = match unit.trim_end_matches('s') {
        "second" => 1,
        "minute" => 60,
        "hour" => 3600,
        "day" => 86_400,
        "week" => 604_800,
        _ => return None,
    };
    Some(SignedDuration::from_secs(n * seconds))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identify_matches_rails() {
        // ActiveRecord::FixtureSet.identify, from the reference app.
        assert_eq!(identify("david"), 127326141);
        assert_eq!(identify("signal"), 873240054);
        assert_eq!(identify("designers"), 654632876);
    }

    #[test]
    fn table_names() {
        let dir = Path::new("/f");
        assert_eq!(
            table_name(dir, Path::new("/f/action_text/rich_texts.yml")),
            "action_text_rich_texts"
        );
        assert_eq!(
            table_name(dir, Path::new("/f/push/subscriptions.yml")),
            "push_subscriptions"
        );
        assert_eq!(table_name(dir, Path::new("/f/users.yml")), "users");
    }

    #[test]
    fn erb_ago_renders_whole_seconds() {
        let options = Options {
            now: Timestamp::parse_db("2026-09-26 12:24:38.211309").unwrap(),
            bcrypt_cost: 4,
        };
        let mut erb = Erb::new(&options);
        assert_eq!(
            erb.render("created_at: <%= 1.hour.ago %>").unwrap(),
            "created_at: 2026-09-26 11:24:38 UTC"
        );
        assert_eq!(
            erb.render("<% x = 5.minutes.ago %>\na: <%= x %>").unwrap(),
            "\na: 2026-09-26 12:19:38 UTC"
        );
        assert_eq!(
            erb.render("<%= 2.days.from_now + 1.hour %>|<%= 1.hour.ago.to_fs(:db) %>")
                .unwrap(),
            "2026-09-28 13:24:38 UTC|2026-09-26 11:24:38"
        );
    }

    #[test]
    fn erb_sha256_hexdigest() {
        let options = Options {
            now: Timestamp::parse_db("2026-09-26 12:24:38").unwrap(),
            bcrypt_cost: 4,
        };
        let mut erb = Erb::new(&options);
        // Digest::SHA256.hexdigest("abc")
        assert_eq!(
            erb.render(r#"<%= Digest::SHA256.hexdigest("abc") %>"#)
                .unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            erb.render(r#"<%= Digest::SHA256.hexdigest("abc")[0, 4] %>"#)
                .unwrap(),
            "ba78"
        );
    }
}
