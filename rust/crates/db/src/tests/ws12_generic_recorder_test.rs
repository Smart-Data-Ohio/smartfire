//! Real persisted source rows and complete recorder facts produced by pinned Rails.
use super::*;
use crate::models::activity_item::{
    ActivityEventType, ActivityRecordingFacts, ActivityRecordingSource, ActivitySource,
    AgentBudgetNoticeActivityReader, SourceAuthorization,
};
use crate::{ActivityItem, Connection, Error, Result, Room};
use rusqlite::OptionalExtension;
use serde_json::{Value, json};

fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../vectors/ws12_generic_recorder.json"
    ))
    .unwrap()
}
fn prepare(setup: &Value) -> TestDb {
    let t = channel_thread_test::frozen();
    t.clock.travel_to(crate::Timestamp::from_second(1772467200));
    let setup = setup.clone();
    t.write(move |tx| {
        tx.conn().execute_batch("PRAGMA defer_foreign_keys=ON")?;
        for (table, rows) in setup.as_object().unwrap() {
            for row in rows.as_array().unwrap() {
                let fields = row.as_object().unwrap();
                let columns = fields.keys().cloned().collect::<Vec<_>>();
                let values = fields
                    .values()
                    .map(|value| match value {
                        Value::Null => rusqlite::types::Value::Null,
                        Value::Bool(value) => rusqlite::types::Value::Integer(i64::from(*value)),
                        Value::Number(value) => {
                            rusqlite::types::Value::Integer(value.as_i64().unwrap())
                        }
                        Value::String(value) => rusqlite::types::Value::Text(value.clone()),
                        _ => panic!("unexpected persisted SQLite field"),
                    })
                    .collect::<Vec<_>>();
                let sql = format!(
                    "INSERT INTO {table} ({}) VALUES ({}) ON CONFLICT(id) DO UPDATE SET {}",
                    columns.join(","),
                    vec!["?"; columns.len()].join(","),
                    columns
                        .iter()
                        .map(|c| format!("{c}=excluded.{c}"))
                        .collect::<Vec<_>>()
                        .join(",")
                );
                tx.conn()
                    .execute(&sql, rusqlite::params_from_iter(values))?;
            }
        }
        tx.conn().execute("DELETE FROM activity_items", [])?;
        let broken: Vec<(String, i64, String)> =
            crate::sql::query_all(tx.conn(), "PRAGMA foreign_key_check", [], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })?;
        assert!(broken.is_empty(), "source setup foreign keys: {broken:?}");
        Ok(())
    });
    t
}
fn items(conn: &Connection) -> Result<Value> {
    let mut stmt =
        conn.prepare("SELECT * FROM activity_items ORDER BY user_id,source_type,source_id")?;
    let stamp = |value: Option<String>| {
        value.map(|value| crate::Timestamp::parse_db(&value).unwrap().as_second())
    };
    let rows = stmt.query_map([], |row| Ok(json!({
        "user":row.get::<_,i64>("user_id")?, "source_type":row.get::<_,String>("source_type")?,
        "source_id":row.get::<_,i64>("source_id")?, "event":row.get::<_,String>("event_type")?,
        "read_at":stamp(row.get("read_at")?), "handled_at":stamp(row.get("handled_at")?),
        "created_at":stamp(row.get("created_at")?), "updated_at":stamp(row.get("updated_at")?),
    })))?.collect::<std::result::Result<Vec<_>,_>>()?;
    Ok(json!(rows))
}

// Test-side consumer of WS11's narrow reader contract, using actual notice/agent/user rows.
struct BudgetReader;
impl AgentBudgetNoticeActivityReader for BudgetReader {
    fn recording_recipient_ids(&self, conn: &Connection, notice: i64) -> Result<Option<Vec<i64>>> {
        let owner: Option<Option<i64>> = conn.query_row("SELECT u.id FROM agent_budget_notices n JOIN agents a ON a.id=n.agent_id LEFT JOIN users u ON u.id=a.owner_id WHERE n.id=?", [notice], |r| r.get(0)).optional()?;
        let Some(owner) = owner else { return Ok(None) };
        if let Some(owner) = owner {
            return Ok(Some(vec![owner]));
        }
        Ok(Some(crate::sql::query_all(
            conn,
            "SELECT id FROM users WHERE status=0 AND role=1 ORDER BY id",
            [],
            |r| r.get(0),
        )?))
    }
}
struct RoomSource(i64);
impl ActivityRecordingSource for RoomSource {
    fn recording_facts(&self, conn: &Connection) -> Result<Option<ActivityRecordingFacts>> {
        Ok(
            Room::find_by_id(conn, self.0)?.map(|room| ActivityRecordingFacts {
                source_type: "Room",
                source_id: room.id,
                creator_id: Some(room.creator_id),
                // A custom source cannot smuggle a thread into the grouping contract.
                thread_id: Some(91),
                recipient_ids: Vec::new(),
            }),
        )
    }
}
fn record(tx: &mut crate::Tx<'_>, row: &Value) -> Result<Option<ActivityItem>> {
    let user = row["recipient"].as_i64().unwrap();
    let id = row["source_id"].as_i64().unwrap();
    let event = row["event"].as_str().unwrap();
    let skip = row["skip"].as_bool().unwrap();
    let authorization = if skip {
        SourceAuthorization::CallerAuthorized
    } else {
        SourceAuthorization::SourceRecipients
    };
    let source = match row["source_type"].as_str().unwrap() {
        "Message" => ActivitySource::Message(id),
        "SavedItem" => ActivitySource::SavedItem(id),
        "Event" => ActivitySource::CalendarEvent(id),
        "HuddleGrant" => ActivitySource::HuddleGrant(id),
        "AgentApproval" => ActivitySource::AgentApproval(id),
        "AgentBudgetNotice" => {
            return ActivityItem::record_with_budget_notice_reader(
                tx,
                user,
                ActivitySource::AgentBudgetNotice(id),
                ActivityEventType::parse(event)?,
                authorization,
                &BudgetReader,
            );
        }
        "ScheduledMessage" => ActivitySource::ScheduledMessage(id),
        "Session" => ActivitySource::Session(id),
        "TwoFactorCredential" => ActivitySource::TwoFactorCredential(id),
        "Room" => {
            return ActivityItem::record_from_source(
                tx,
                user,
                &RoomSource(id),
                ActivityEventType::parse(event)?,
                authorization,
            );
        }
        other => panic!("unexpected source {other}"),
    };
    ActivityItem::record(tx, user, source, event, skip)
}
#[test]
fn ws12_generic_recorder_matches_rails_persisted_sources_recipients_and_authorization() {
    let vector = oracle();
    let mut differences = Vec::new();
    for row in vector["rows"].as_array().unwrap() {
        let t = prepare(&vector["setup"]);
        let changes = row["sql"].as_array().cloned().unwrap_or_default();
        t.write(move |tx| {
            for sql in changes {
                tx.conn().execute(sql.as_str().unwrap(), [])?;
            }
            Ok(())
        });
        let input = row.clone();
        let result = t.try_write(move |tx| record(tx, &input));
        let facts = t.read(items);
        let actual = match result {
            Ok(item) => json!({"recorded":item.is_some(),"items":facts}),
            Err(Error::Other(error)) => json!({"error":error,"items":facts}),
            Err(error) => panic!("unexpected recorder error: {error}"),
        };
        if actual != row["expected"] {
            differences.push((row["name"].clone(), actual, row["expected"].clone()));
        }
    }
    assert!(
        differences.is_empty(),
        "complete Rails recorder facts differ: {differences:?}"
    );
}
#[test]
fn ws12_generic_recorder_preserves_handled_source_event_and_timestamps() {
    let vector = oracle();
    let t = prepare(&vector["setup"]);
    let source = vector["idempotent"]["source_id"].as_i64().unwrap();
    let user = vector["idempotent"]["recipient"].as_i64().unwrap();
    let item = t
        .write(move |tx| {
            ActivityItem::record(
                tx,
                user,
                ActivitySource::SavedItem(source),
                "message_reminder",
                true,
            )
        })
        .unwrap();
    t.write(move |tx| item.mark_handled(tx));
    t.travel(1);
    let recorded = t
        .write(move |tx| {
            ActivityItem::record(tx, user, ActivitySource::SavedItem(source), "mention", true)
        })
        .is_some();
    assert_eq!(
        json!({"recorded":recorded,"items":t.read(items)}),
        vector["idempotent"]["expected"]
    );
}

#[test]
fn ws12_generic_recorder_nullable_huddle_involvement_matches_rails() {
    let vector = oracle();
    for name in ["HuddleGrant_null_false", "HuddleGrant_null_true"] {
        let row = vector["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["name"] == name)
            .unwrap();
        let t = prepare(&vector["setup"]);
        let changes = row["sql"].as_array().unwrap().clone();
        t.write(move |tx| {
            for sql in changes {
                tx.conn().execute(sql.as_str().unwrap(), [])?;
            }
            Ok(())
        });
        let input = row.clone();
        let recorded = t.write(move |tx| record(tx, &input)).is_some();
        assert_eq!(
            json!({"recorded": recorded, "items": t.read(items)}),
            row["expected"],
            "nullable involvement excludes only explicit nothing/invisible: {name}"
        );
    }
}

#[test]
fn ws12_generic_recorder_rolls_back_rows_and_commit_broadcasts_together() {
    let vector = oracle();
    let t = prepare(&vector["setup"]);
    t.sink.take();
    let source = vector["idempotent"]["source_id"].as_i64().unwrap();
    let user = vector["idempotent"]["recipient"].as_i64().unwrap();
    let result: Result<()> = t.try_write(move |tx| {
        assert!(
            ActivityItem::record(
                tx,
                user,
                ActivitySource::SavedItem(source),
                "message_reminder",
                true
            )?
            .is_some()
        );
        Err(Error::Other("source transaction failed".into()))
    });
    assert!(result.is_err());
    assert_eq!(t.read(items), json!([]));
    assert!(t.sink.take().is_empty());
}

#[test]
fn ws12_generic_recorder_caller_authorized_message_keeps_grouping_and_idempotency() {
    let vector = oracle();
    let t = prepare(&vector["setup"]);
    let source = vector["caller_authorized"]["source_id"].as_i64().unwrap();
    let user = vector["caller_authorized"]["recipient"].as_i64().unwrap();
    let (denied, same_id) = t.write(move |tx| {
        let denied = ActivityItem::record(
            tx,
            user,
            ActivitySource::Message(source),
            "thread_activity",
            false,
        )?
        .is_none();
        let first = ActivityItem::record(
            tx,
            user,
            ActivitySource::Message(source),
            "thread_activity",
            true,
        )?
        .unwrap();
        let second = ActivityItem::record(
            tx,
            user,
            ActivitySource::Message(source),
            "thread_activity",
            true,
        )?
        .unwrap();
        Ok((denied, first.id == second.id))
    });
    assert_eq!(
        json!({"denied":denied,"same_id":same_id,"items":t.read(items)}),
        vector["caller_authorized"]["expected"]
    );
}

#[test]
fn ws12_generic_recorder_requires_the_budget_owners_reader() {
    let vector = oracle();
    let t = prepare(&vector["setup"]);
    let result = t.try_write(|tx| {
        ActivityItem::record(
            tx,
            id("david"),
            ActivitySource::AgentBudgetNotice(901840003),
            "agent_budget_exceeded",
            true,
        )
    });
    assert!(
        matches!(result,Err(Error::Other(message)) if message == "AgentBudgetNotice requires its owning-domain activity reader")
    );
    assert_eq!(t.read(items), json!([]));
}

thread_local! { static SOURCE_READS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) }; }
fn trace(event: rusqlite::trace::TraceEvent<'_>) {
    if let rusqlite::trace::TraceEvent::Stmt(_, sql) = event {
        SOURCE_READS.with(|reads| reads.borrow_mut().push(sql.to_lowercase().replace('"', "")));
    }
}
#[test]
fn ws12_generic_recorder_batches_source_and_human_facts_at_two_sizes() {
    let vector = oracle();
    for size in [10, 100] {
        let t = prepare(&vector["setup"]);
        t.write(move |tx| {
            let ids = (0..size).map(|index|901850000+index).collect::<Vec<_>>();
            for id in &ids {
                tx.conn().execute("INSERT INTO users(id,name,role,status,created_at,updated_at) VALUES(?,'Recipient',0,0,?,?)",rusqlite::params![id,tx.now(),tx.now()])?;
            }
            SOURCE_READS.with(|reads|reads.borrow_mut().clear());
            tx.conn().trace_v2(rusqlite::trace::TraceEventCodes::SQLITE_TRACE_STMT,Some(trace));
            let recorded = ActivityItem::record_source_for_recipients(tx,&ids,&RoomSource(901840001),ActivityEventType::parse("work_update")?,SourceAuthorization::CallerAuthorized)?;
            tx.conn().trace_v2(rusqlite::trace::TraceEventCodes::empty(),None);
            assert_eq!(recorded.len(),size as usize);
            SOURCE_READS.with(|reads| {
                let reads = reads.borrow();
                let users = reads.iter().filter(|sql|sql.starts_with("select * from users ")).count();
                let sources = reads.iter().filter(|sql|sql.starts_with("select * from rooms ")).count();
                println!("WS12_RECORDER_BATCH recipients={size} source_reads={sources} user_reads={users}");
                assert_eq!((sources,users),(1,1));
            });
            Ok(())
        });
        assert_eq!(t.read(items).as_array().unwrap().len(), size as usize);
    }
}
