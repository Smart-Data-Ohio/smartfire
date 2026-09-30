//! The conditional-UPDATE claim pattern our Ruby uses on its own tables, for the domain code that
//! ports it:
//!
//! - a claim is one `UPDATE ... WHERE <still claimable>`, won when it changed exactly one row
//!   ([`ConditionalUpdate::claim`]): `Agent::DeliveryJob`'s `WHERE outcome = 'pending'`,
//!   `Agent::EventWebhookJob`'s `WHERE webhook_attempts = n`, the GitHub/Fizzy claims'
//!   `json_extract(metadata, '$.status') = 'running'` rewrite;
//! - a stuck-claim sweep reclaims the rows whose claim is older than a cutoff in one statement
//!   and returns them, so the caller re-enqueues their jobs in the same write
//!   ([`ConditionalUpdate::returning_ids`] with [`unclaimed_before`]): `Room::DestroyJob.reenqueue_stuck!`;
//! - a JSON lease keeps a stamp and a token in a JSON column, taken over once the stamp goes
//!   stale ([`JsonLease`]): `SlackImport#acquire_step_lease!` and friends.
//!
//! Every statement runs in the caller's write, on the single writer, so claims never interleave.
//! Table and column names are the caller's constants, never input.

use rusqlite::ToSql;
use rusqlite::types::Value;

use campfire_db::{Connection, Result, Timestamp};

/// `UPDATE <table> SET ... WHERE ...`, built from SQL fragments with `?` placeholders and their
/// values, in order.
#[derive(Debug, Clone)]
pub struct ConditionalUpdate {
    table: &'static str,
    sets: Vec<String>,
    set_values: Vec<Value>,
    conditions: Vec<String>,
    condition_values: Vec<Value>,
}

impl ConditionalUpdate {
    pub fn table(table: &'static str) -> Self {
        Self { table, sets: Vec::new(), set_values: Vec::new(), conditions: Vec::new(), condition_values: Vec::new() }
    }

    /// `column = value`
    pub fn set(self, column: &'static str, value: impl Into<Value>) -> Self {
        self.set_sql(column, "?", [value.into()])
    }

    /// `column = <expression>`, whose `?`s take `values`: `json_set("state", '$.key', ?)`.
    pub fn set_sql(mut self, column: &'static str, expression: &str, values: impl IntoIterator<Item = Value>) -> Self {
        self.sets.push(format!("{} = {expression}", quote(column)));
        self.set_values.extend(values);
        self
    }

    /// `WHERE ... AND "id" = id`
    pub fn id(self, id: i64) -> Self {
        self.condition(r#""id" = ?"#, [Value::Integer(id)])
    }

    /// `WHERE ... AND (<condition>)`, whose `?`s take `values`.
    pub fn condition(mut self, condition: &str, values: impl IntoIterator<Item = Value>) -> Self {
        self.conditions.push(format!("({condition})"));
        self.condition_values.extend(values);
        self
    }

    fn sql(&self, returning: bool) -> String {
        assert!(!self.sets.is_empty(), "an UPDATE of {} with nothing to set", self.table);
        let conditions = if self.conditions.is_empty() { "1".to_string() } else { self.conditions.join(" AND ") };
        let returning = if returning { r#" RETURNING "id""# } else { "" };
        format!("UPDATE {} SET {} WHERE {conditions}{returning}", quote(self.table), self.sets.join(", "))
    }

    fn values(&self) -> impl Iterator<Item = &dyn ToSql> {
        self.set_values.iter().chain(&self.condition_values).map(|value| value as &dyn ToSql)
    }

    /// Runs it; the number of rows it changed.
    pub fn execute(&self, conn: &Connection) -> Result<usize> {
        Ok(conn.prepare_cached(&self.sql(false))?.execute(rusqlite::params_from_iter(self.values()))?)
    }

    /// Runs it as a claim: won when it changed exactly one row (`update_all(...) == 1`).
    pub fn claim(&self, conn: &Connection) -> Result<bool> {
        Ok(self.execute(conn)? == 1)
    }

    /// Runs it, returning the ids of the rows it changed, in id order: a sweep's reclaimed rows.
    pub fn returning_ids(&self, conn: &Connection) -> Result<Vec<i64>> {
        let mut statement = conn.prepare_cached(&self.sql(true))?;
        let mut ids = statement.query_map(rusqlite::params_from_iter(self.values()), |row| row.get(0))?.collect::<rusqlite::Result<Vec<i64>>>()?;
        ids.sort_unstable();
        Ok(ids)
    }
}

/// The condition of a claim stamped in `column` that is missing or older than `cutoff`
/// (`destroy_unclaimed_before(cutoff)`), for a stuck-claim sweep that re-stamps `column`.
pub fn unclaimed_before(column: &'static str, cutoff: Timestamp) -> (String, [Value; 1]) {
    let column = quote(column);
    (format!("{column} IS NULL OR {column} < ?"), [Value::Text(cutoff.to_db())])
}

/// A lease held in a JSON column (`SlackImport`'s step lease, `app/models/slack_import.rb`): a
/// stamp (`$.step_started_at`) that ages the lease out after `stale_after`, and the random token
/// of the job holding it (`$.step_lease_token`). Acquiring is a conditional UPDATE on the stamp
/// being missing or stale; refreshing and releasing are conditional on the token, so a job never
/// extends or releases another's lease. Each touches only the lease's keys (plus the heartbeat
/// and `updated_at` columns), never other keys of the column.
#[derive(Debug, Clone, Copy)]
pub struct JsonLease {
    pub table: &'static str,
    /// The JSON column: `state`.
    pub column: &'static str,
    /// `$.step_started_at`
    pub stamp_path: &'static str,
    /// `$.step_lease_token`
    pub token_path: &'static str,
    /// A column set to now when the lease is acquired or refreshed (`heartbeat_at`), if any.
    pub heartbeat_column: Option<&'static str>,
    /// `STALE_HEARTBEAT`
    pub stale_after: jiff::SignedDuration,
}

impl JsonLease {
    /// `acquire_step_lease!`: takes the lease on row `id` when nobody holds a fresh one and
    /// `condition` (the expected status, say) holds. Returns the new token, or `None` when the
    /// claim was lost.
    pub fn acquire(&self, conn: &Connection, id: i64, condition: (&str, &[Value]), now: Timestamp) -> Result<Option<String>> {
        let token = lease_token();
        let column = quote(self.column);
        let stamp = self.extract(self.stamp_path);
        let update = self
            .touch(ConditionalUpdate::table(self.table), now)
            .set_sql(self.column, &format!("json_set({column}, ?, ?, ?, ?)"), [
                Value::Text(self.stamp_path.into()),
                Value::Text(lease_stamp(now)),
                Value::Text(self.token_path.into()),
                Value::Text(token.clone()),
            ])
            .id(id)
            .condition(&format!("{stamp} IS NULL OR {stamp} <= ?"), [Value::Text(lease_stamp(now.ago(self.stale_after)))])
            .condition(condition.0, condition.1.iter().cloned());
        Ok(update.claim(conn)?.then_some(token))
    }

    /// `refresh_step_lease!`: renews the stamp while `token` still holds the lease.
    pub fn refresh(&self, conn: &Connection, id: i64, token: &str, now: Timestamp) -> Result<bool> {
        if token.trim().is_empty() {
            return Ok(false);
        }
        let column = quote(self.column);
        self.touch(ConditionalUpdate::table(self.table), now)
            .set_sql(self.column, &format!("json_set({column}, ?, ?)"), [Value::Text(self.stamp_path.into()), Value::Text(lease_stamp(now))])
            .id(id)
            .condition(&format!("{} = ?", self.extract(self.token_path)), [Value::Text(token.into())])
            .claim(conn)
    }

    /// `release_step_lease!`: removes the stamp and token while `token` still holds the lease.
    /// Touches `updated_at` only.
    pub fn release(&self, conn: &Connection, id: i64, token: &str, now: Timestamp) -> Result<bool> {
        if token.trim().is_empty() {
            return Ok(false);
        }
        let column = quote(self.column);
        ConditionalUpdate::table(self.table)
            .set_sql(self.column, &format!("json_remove({column}, ?, ?)"), [Value::Text(self.stamp_path.into()), Value::Text(self.token_path.into())])
            .set("updated_at", now.to_db())
            .id(id)
            .condition(&format!("{} = ?", self.extract(self.token_path)), [Value::Text(token.into())])
            .claim(conn)
    }

    /// The condition that a row holds a fresh lease (`with_fresh_lease`), and its value.
    pub fn fresh(&self, now: Timestamp) -> (String, [Value; 1]) {
        (format!("{} > ?", self.extract(self.stamp_path)), [Value::Text(lease_stamp(now.ago(self.stale_after)))])
    }

    /// Whether row `id` holds a fresh lease.
    pub fn is_fresh(&self, conn: &Connection, id: i64, now: Timestamp) -> Result<bool> {
        let (fresh, values) = self.fresh(now);
        let sql = format!(r#"SELECT EXISTS (SELECT 1 FROM {} WHERE "id" = ? AND {fresh})"#, quote(self.table));
        let [cutoff] = values;
        Ok(conn.prepare_cached(&sql)?.query_row(rusqlite::params![id, cutoff], |row| row.get(0))?)
    }

    fn extract(&self, path: &str) -> String {
        format!("json_extract({}, '{path}')", quote(self.column))
    }

    fn touch(&self, update: ConditionalUpdate, now: Timestamp) -> ConditionalUpdate {
        let update = match self.heartbeat_column {
            Some(column) => update.set(column, now.to_db()),
            None => update,
        };
        update.set("updated_at", now.to_db())
    }
}

/// `SlackImport.lease_stamp`: `time.utc.iso8601(6)`, which compares lexically in SQLite (and so
/// must match Rails' to the character, for leases either app wrote).
pub fn lease_stamp(at: Timestamp) -> String {
    at.jiff().strftime("%Y-%m-%dT%H:%M:%S%.6fZ").to_string()
}

/// `SecureRandom.hex(8)`
pub fn lease_token() -> String {
    let bytes: [u8; 8] = rand::random();
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn quote(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> Timestamp {
        Timestamp::parse_db(text).unwrap()
    }

    fn runs() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"CREATE TABLE "runs" ("id" integer PRIMARY KEY, "status" varchar NOT NULL, "state" json DEFAULT '{}' NOT NULL, "heartbeat_at" datetime(6), "updated_at" datetime(6));
               INSERT INTO "runs" ("id", "status", "state") VALUES (1, 'running', '{"cursor":"c1"}'), (2, 'queued', '{}');"#,
        )
        .unwrap();
        conn
    }

    fn state(conn: &Connection, id: i64) -> serde_json::Value {
        let text: String = conn.query_row(r#"SELECT "state" FROM "runs" WHERE "id" = ?"#, [id], |row| row.get(0)).unwrap();
        serde_json::from_str(&text).unwrap()
    }

    const LEASE: JsonLease = JsonLease {
        table: "runs",
        column: "state",
        stamp_path: "$.step_started_at",
        token_path: "$.step_lease_token",
        heartbeat_column: Some("heartbeat_at"),
        stale_after: jiff::SignedDuration::from_mins(5),
    };

    /// `Time#iso8601(6)` in UTC, as Ruby prints it.
    #[test]
    fn lease_stamps_match_ruby() {
        assert_eq!(lease_stamp(at("2026-09-29 19:30:00.123456")), "2026-09-29T19:30:00.123456Z");
        assert_eq!(lease_stamp(at("2026-09-29 19:30:00")), "2026-09-29T19:30:00.000000Z");
        let token = lease_token();
        assert_eq!(token.len(), 16);
        assert!(token.bytes().all(|b| b.is_ascii_hexdigit()));
    }

    #[test]
    fn a_json_lease_is_single_flight_until_it_goes_stale() {
        let conn = runs();
        let now = at("2026-09-29 12:00:00");
        let running = (r#""status" = ?"#, &[Value::Text("running".into())][..]);

        let token = LEASE.acquire(&conn, 1, running, now).unwrap().expect("acquired");
        let held = state(&conn, 1);
        assert_eq!(held["step_lease_token"], token.as_str());
        assert_eq!(held["step_started_at"], "2026-09-29T12:00:00.000000Z");
        assert_eq!(held["cursor"], "c1", "other keys are untouched");
        assert!(LEASE.is_fresh(&conn, 1, now).unwrap());

        let later = now.since(jiff::SignedDuration::from_mins(4));
        assert_eq!(LEASE.acquire(&conn, 1, running, later).unwrap(), None, "a fresh lease blocks another claim");
        assert!(!LEASE.refresh(&conn, 1, "not-the-token", later).unwrap());
        assert!(LEASE.refresh(&conn, 1, &token, later).unwrap());
        assert_eq!(state(&conn, 1)["step_started_at"], "2026-09-29T12:04:00.000000Z");

        // Five minutes after the refresh the lease is stale, and another job takes it over.
        let stale = later.since(jiff::SignedDuration::from_mins(5));
        assert!(!LEASE.is_fresh(&conn, 1, stale).unwrap());
        let taken = LEASE.acquire(&conn, 1, running, stale).unwrap().expect("taken over");
        assert_ne!(taken, token);
        assert!(!LEASE.release(&conn, 1, &token, stale).unwrap(), "the old holder can't release it");
        assert!(LEASE.release(&conn, 1, &taken, stale).unwrap());
        assert_eq!(state(&conn, 1), serde_json::json!({"cursor": "c1"}));
    }

    #[test]
    fn a_json_lease_respects_the_callers_condition() {
        let conn = runs();
        let now = at("2026-09-29 12:00:00");
        let running = (r#""status" = ?"#, &[Value::Text("running".into())][..]);
        assert_eq!(LEASE.acquire(&conn, 2, running, now).unwrap(), None, "run 2 is queued");
        assert!(LEASE.acquire(&conn, 2, ("1", &[][..]), now).unwrap().is_some());
    }

    #[test]
    fn a_stuck_claim_sweep_restamps_and_returns_only_stuck_rows() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"CREATE TABLE "rooms" ("id" integer PRIMARY KEY, "deleted_at" datetime(6), "destroy_enqueued_at" datetime(6));
               INSERT INTO "rooms" VALUES (1, '2026-09-29 11:00:00', NULL), (2, '2026-09-29 11:00:00', '2026-09-29 11:58:00'),
                                          (3, '2026-09-29 11:00:00', '2026-09-29 11:30:00'), (4, NULL, NULL);"#,
        )
        .unwrap();
        let now = at("2026-09-29 12:00:00");
        let cutoff = now.ago(jiff::SignedDuration::from_mins(5));
        let (unclaimed, values) = unclaimed_before("destroy_enqueued_at", cutoff);
        let sweep = ConditionalUpdate::table("rooms")
            .set("destroy_enqueued_at", now.to_db())
            .condition(r#""deleted_at" < ?"#, [Value::Text(cutoff.to_db())])
            .condition(&unclaimed, values);
        assert_eq!(sweep.returning_ids(&conn).unwrap(), [1, 3]);
        assert_eq!(sweep.returning_ids(&conn).unwrap(), Vec::<i64>::new(), "reclaimed rows aren't stuck any more");
    }

    #[test]
    fn a_conditional_claim_is_won_once() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(r#"CREATE TABLE "agent_events" ("id" integer PRIMARY KEY, "metadata" json, "detail" text); INSERT INTO "agent_events" VALUES (1, '{"status":"running"}', NULL);"#).unwrap();
        let finish = ConditionalUpdate::table("agent_events")
            .set("detail", "GitHub action execution timed out".to_string())
            .set_sql("metadata", r#"json_set("metadata", '$.status', ?)"#, [Value::Text("failed".into())])
            .id(1)
            .condition(r#"json_extract("agent_events"."metadata", '$.status') = 'running'"#, []);
        assert!(finish.claim(&conn).unwrap());
        assert!(!finish.claim(&conn).unwrap(), "the first write wins");
    }
}
