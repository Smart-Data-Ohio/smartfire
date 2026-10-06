//! `app/models/huddle_cleanup.rb`: durable LiveKit removal snapshots, enqueue leases,
//! and database-backed retries. Network I/O happens after claiming commits.
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

pub use crate::models::room_delete::CleanupJob;
use crate::{CachedStatements, Connection, Errors, Event, Result, Timestamp, Tx};

pub const ENQUEUE_LEASE_SECONDS: i64 = 60;
pub const INITIAL_RETRY_DELAY_SECONDS: i64 = 15;
pub const MAX_RETRY_DELAY_SECONDS: i64 = 15 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    RemoveParticipant,
    DeleteRoom,
}
impl Operation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RemoveParticipant => "remove_participant",
            Self::DeleteRoom => "delete_room",
        }
    }
}

#[derive(Debug, Clone)]
pub struct HuddleCleanup {
    pub id: i64,
    pub operation: Operation,
    pub huddle_grant_id: Option<i64>,
    pub room_name: String,
    pub identity: Option<String>,
    pub attempts: i64,
    pub enqueued_at: Option<Timestamp>,
    pub last_attempted_at: Option<Timestamp>,
    pub next_attempt_at: Option<Timestamp>,
    pub completed_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl HuddleCleanup {
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        let operation: String = row.get("operation")?;
        let operation = match operation.as_str() {
            "remove_participant" => Operation::RemoveParticipant,
            "delete_room" => Operation::DeleteRoom,
            _ => return Err(rusqlite::Error::InvalidQuery),
        };
        Ok(Self {
            id: row.get("id")?,
            operation,
            huddle_grant_id: row.get("huddle_grant_id")?,
            room_name: row.get("room_name")?,
            identity: row.get("identity")?,
            attempts: row.get("attempts")?,
            enqueued_at: row.get("enqueued_at")?,
            last_attempted_at: row.get("last_attempted_at")?,
            next_attempt_at: row.get("next_attempt_at")?,
            completed_at: row.get("completed_at")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    pub fn find_by_id(conn: &Connection, id: i64) -> Result<Option<Self>> {
        Ok(conn
            .query_row_cached(
                "SELECT * FROM huddle_cleanups WHERE id=?",
                [id],
                Self::from_row,
            )
            .optional()?)
    }

    /// Presence validations are the same as Rails. Enum validity is enforced by the input type;
    /// the two Rails partial unique indexes enforce the idempotence keys.
    pub fn create(
        tx: &mut Tx<'_>,
        operation: Operation,
        room_name: &str,
        identity: Option<&str>,
        grant_id: Option<i64>,
        admin_configured: bool,
    ) -> Result<Self> {
        let mut errors = Errors::default();
        if room_name.chars().all(char::is_whitespace) {
            errors.add("room_name", "can't be blank");
        }
        if operation == Operation::RemoveParticipant
            && identity.is_none_or(|s| s.chars().all(char::is_whitespace))
        {
            errors.add("identity", "can't be blank");
        }
        errors.into_result()?;
        let now = tx.now();
        let id = tx.conn().query_row_cached(
            "INSERT INTO huddle_cleanups(operation,room_name,identity,huddle_grant_id,created_at,updated_at) VALUES(?,?,?,?,?,?) RETURNING id",
            params![operation.as_str(), room_name, identity, grant_id, now, now], |row| row.get(0),
        )?;
        Self::enqueue(tx, id, admin_configured)?;
        Ok(Self::find_by_id(tx.conn(), id)?.expect("created cleanup"))
    }

    pub fn create_participant_removal(
        tx: &mut Tx<'_>,
        grant_id: i64,
        room_name: &str,
        identity: &str,
        admin_configured: bool,
    ) -> Result<Self> {
        let existing = tx.conn().query_row_cached(
            "SELECT * FROM huddle_cleanups WHERE operation='remove_participant' AND huddle_grant_id=?", [grant_id], Self::from_row,
        ).optional()?;
        match existing {
            Some(row) => Ok(row),
            None => Self::create(
                tx,
                Operation::RemoveParticipant,
                room_name,
                Some(identity),
                Some(grant_id),
                admin_configured,
            ),
        }
    }

    pub fn create_room_deletion(
        tx: &mut Tx<'_>,
        room_name: &str,
        admin_configured: bool,
    ) -> Result<Self> {
        let existing = tx
            .conn()
            .query_row_cached(
                "SELECT * FROM huddle_cleanups WHERE operation='delete_room' AND room_name=?",
                [room_name],
                Self::from_row,
            )
            .optional()?;
        match existing {
            Some(row) => Ok(row),
            None => Self::create(
                tx,
                Operation::DeleteRoom,
                room_name,
                None,
                None,
                admin_configured,
            ),
        }
    }

    /// Persist the job and lease with the triggering write (decision 2). An enqueue failure
    /// rolls back the triggering transaction; no cleanup or lease is stranded.
    pub fn enqueue(tx: &mut Tx<'_>, id: i64, admin_configured: bool) -> Result<bool> {
        if !admin_configured {
            return Ok(false);
        }
        let now = tx.now();
        let claimed = tx.conn().execute_cached(
            "UPDATE huddle_cleanups SET enqueued_at=?,next_attempt_at=? WHERE id=? AND completed_at IS NULL AND enqueued_at IS NULL AND (next_attempt_at IS NULL OR next_attempt_at<=?)",
            params![now, now.since(jiff::SignedDuration::from_secs(ENQUEUE_LEASE_SECONDS)), id, now],
        )? == 1;
        if claimed {
            tx.emit_after_commit(Event::job(&CleanupJob { cleanup_id: id }));
        }
        Ok(claimed)
    }

    pub fn due_ids(conn: &Connection, now: Timestamp, limit: usize) -> Result<Vec<i64>> {
        let mut statement = conn.prepare_cached(
            "SELECT id FROM huddle_cleanups WHERE completed_at IS NULL AND (next_attempt_at IS NULL OR next_attempt_at<=?) ORDER BY id LIMIT ?",
        )?;
        Ok(statement
            .query_map(params![now, limit as i64], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn claim(
        tx: &mut Tx<'_>,
        id: i64,
        from_queue: bool,
        admin_configured: bool,
    ) -> Result<Option<Self>> {
        if !admin_configured {
            return Ok(None);
        }
        let Some(row) = Self::find_by_id(tx.conn(), id)? else {
            return Ok(None);
        };
        let now = tx.now();
        if row.completed_at.is_some()
            || if from_queue {
                row.enqueued_at.is_none()
            } else {
                row.next_attempt_at.is_some_and(|at| at > now)
            }
        {
            return Ok(None);
        }
        let attempt = row
            .attempts
            .checked_add(1)
            .ok_or(rusqlite::Error::InvalidQuery)?;
        let delay_micros = (INITIAL_RETRY_DELAY_SECONDS as f64
            * 1_000_000.0
            * 2f64.powf(attempt.saturating_sub(1) as f64))
        .min(MAX_RETRY_DELAY_SECONDS as f64 * 1_000_000.0) as i64;
        tx.conn().execute_cached(
            "UPDATE huddle_cleanups SET attempts=?,enqueued_at=NULL,last_attempted_at=?,next_attempt_at=? WHERE id=?",
            params![attempt, now, now.since(jiff::SignedDuration::from_micros(delay_micros)), id],
        )?;
        Self::find_by_id(tx.conn(), id)
    }

    pub fn complete(tx: &mut Tx<'_>, id: i64) -> Result<()> {
        let Some(row) = Self::find_by_id(tx.conn(), id)? else {
            return Ok(());
        };
        let mut errors = Errors::default();
        if row.room_name.chars().all(char::is_whitespace) {
            errors.add("room_name", "can't be blank");
        }
        if row.operation == Operation::RemoveParticipant
            && row
                .identity
                .as_deref()
                .is_none_or(|s| s.chars().all(char::is_whitespace))
        {
            errors.add("identity", "can't be blank");
        }
        errors.into_result()?;
        let now = tx.now();
        tx.conn().execute_cached("UPDATE huddle_cleanups SET completed_at=?,next_attempt_at=NULL,updated_at=? WHERE id=?", params![now, now, id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestDb;

    fn create(db: &TestDb, configured: bool) -> i64 {
        db.write(move |tx| {
            Ok(HuddleCleanup::create(
                tx,
                Operation::RemoveParticipant,
                "opaque-room",
                Some("opaque-participant"),
                None,
                configured,
            )?
            .id)
        })
    }

    #[test]
    fn huddle_cleanup_validates_rows_and_enqueues_only_with_admin_configuration() {
        let db = TestDb::new();
        assert!(
            db.try_write(|tx| HuddleCleanup::create(
                tx,
                Operation::DeleteRoom,
                " \t",
                None,
                None,
                true
            ))
            .is_err()
        );
        assert!(
            db.try_write(|tx| HuddleCleanup::create(
                tx,
                Operation::RemoveParticipant,
                "room",
                None,
                None,
                true
            ))
            .is_err()
        );
        let id = create(&db, false);
        let row = db.read(|conn| Ok(HuddleCleanup::find_by_id(conn, id)?.unwrap()));
        assert_eq!(row.enqueued_at, None);
        assert!(db.events().is_empty());
        assert!(db.write(move |tx| HuddleCleanup::enqueue(tx, id, true)));
        assert!(!db.write(move |tx| HuddleCleanup::enqueue(tx, id, true)));
        assert_eq!(db.events().len(), 1);
    }

    #[test]
    fn huddle_cleanup_queued_lease_consumes_once_and_retries_without_a_queue() {
        let db = TestDb::new();
        db.clock.travel_to(Timestamp::from_second(1_767_268_800));
        let id = create(&db, true);
        assert!(
            db.write(move |tx| HuddleCleanup::claim(tx, id, false, true))
                .is_none()
        );
        let row = db
            .write(move |tx| HuddleCleanup::claim(tx, id, true, true))
            .expect("queued claim");
        assert_eq!(row.attempts, 1);
        assert_eq!(row.enqueued_at, None);
        assert_eq!(row.last_attempted_at, Some(db.now()));
        assert_eq!(
            row.next_attempt_at,
            Some(db.now().since(jiff::SignedDuration::from_secs(15)))
        );
        assert!(
            db.write(move |tx| HuddleCleanup::claim(tx, id, true, true))
                .is_none()
        );
        assert!(
            db.write(move |tx| HuddleCleanup::claim(tx, id, false, false))
                .is_none()
        );
        db.travel(15);
        assert_eq!(
            db.read(|conn| HuddleCleanup::due_ids(conn, db.now(), 100)),
            vec![id]
        );
        let retry = db
            .write(move |tx| HuddleCleanup::claim(tx, id, false, true))
            .unwrap();
        assert_eq!(retry.attempts, 2);
        assert_eq!(
            retry.next_attempt_at,
            Some(db.now().since(jiff::SignedDuration::from_secs(30)))
        );
        db.write(move |tx| HuddleCleanup::complete(tx, id));
        assert!(
            db.write(move |tx| HuddleCleanup::claim(tx, id, false, true))
                .is_none()
        );
        assert!(
            db.read(|conn| HuddleCleanup::due_ids(conn, db.now(), 100))
                .is_empty()
        );
    }

    #[test]
    fn huddle_cleanup_retry_schedule_and_timestamps_match_rails() {
        let db = TestDb::new();
        let v: serde_json::Value =
            serde_json::from_str(include_str!("huddle_cleanup_vectors.json")).unwrap();
        db.clock
            .travel_to(Timestamp::from_second(v["now"].as_i64().unwrap()));
        let id = create(&db, true);
        for case in v["states"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let mut result = None;
            let from_queue = name == "queued_failure" || name == "duplicate_queue_delivery";
            if name == "queued_failure"
                || name.starts_with("retry_")
                || name == "already_removed_success"
            {
                if name != "queued_failure" {
                    let due = db.read(|conn| {
                        Ok(HuddleCleanup::find_by_id(conn, id)?
                            .unwrap()
                            .next_attempt_at
                            .unwrap())
                    });
                    db.clock.travel_to(due);
                }
                assert!(
                    db.write(move |tx| HuddleCleanup::claim(tx, id, from_queue, true))
                        .is_some(),
                    "{name}"
                );
                result = Some(name == "already_removed_success");
                if name == "already_removed_success" {
                    db.write(move |tx| HuddleCleanup::complete(tx, id));
                }
            } else if name == "lease_blocks_reconcile"
                || name == "completed_noop"
                || name == "duplicate_queue_delivery"
            {
                result = Some(
                    db.write(move |tx| HuddleCleanup::claim(tx, id, from_queue, true))
                        .is_some(),
                );
            }
            assert_eq!(
                result.map_or(serde_json::Value::Null, Into::into),
                case["result"],
                "{name}"
            );
            let row = db.read(|conn| Ok(HuddleCleanup::find_by_id(conn, id)?.unwrap()));
            let stamp = |at: Option<Timestamp>| at.map(|t| t.to_db());
            let actual = serde_json::json!({ "attempts": row.attempts, "completed_at": stamp(row.completed_at),
                "created_at": row.created_at.to_db(), "enqueued_at": stamp(row.enqueued_at),
                "last_attempted_at": stamp(row.last_attempted_at), "next_attempt_at": stamp(row.next_attempt_at), "updated_at": row.updated_at.to_db() });
            assert_eq!(actual, case["row"], "{name}");
        }
    }

    #[test]
    fn huddle_cleanup_two_database_handles_cannot_claim_the_same_attempt() {
        let db = TestDb::new();
        let id = create(&db, false);
        let other = db.another_process();
        let barrier = std::sync::Barrier::new(2);
        let (first, second) = std::thread::scope(|scope| {
            let first = scope.spawn(|| {
                barrier.wait();
                db.write(move |tx| HuddleCleanup::claim(tx, id, false, true))
            });
            let second = scope.spawn(|| {
                barrier.wait();
                other
                    .write_blocking(move |tx| HuddleCleanup::claim(tx, id, false, true))
                    .unwrap()
            });
            (first.join().unwrap(), second.join().unwrap())
        });
        assert_ne!(first.is_some(), second.is_some());
    }

    #[test]
    fn huddle_cleanup_room_deletion_is_idempotent_even_after_completion() {
        let db = TestDb::new();
        let first = db.write(|tx| HuddleCleanup::create_room_deletion(tx, "opaque-room", true));
        db.write(move |tx| HuddleCleanup::complete(tx, first.id));
        let second = db.write(|tx| HuddleCleanup::create_room_deletion(tx, "opaque-room", true));
        assert_eq!(first.id, second.id);
        assert!(second.completed_at.is_some());
        assert_eq!(db.events().len(), 1);
    }
}
