//! Real persisted source rows and complete recorder facts produced by pinned Rails.
use super::*;
use crate::models::activity_item::{
    ActivityEventType, ActivityRecordingFacts, ActivityRecordingSource, ActivitySource,
    SourceAuthorization,
};
use crate::{ActivityItem, Connection, Error, Result, Room};
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
        "AgentBudgetNotice" => ActivitySource::AgentBudgetNotice(id),
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
fn ws12_generic_budget_notice_uses_the_merged_owner_reader_without_an_adapter_argument() {
    let vector = oracle();
    let t = prepare(&vector["setup"]);
    let item = t.write(|tx| {
        ActivityItem::record(
            tx,
            id("david"),
            ActivitySource::AgentBudgetNotice(901840003),
            "agent_budget_exceeded",
            false,
        )
    });
    assert!(
        item.is_some(),
        "persisted notice authorizes its connected owner"
    );
    let absent = t.write(|tx| {
        ActivityItem::record(
            tx,
            id("david"),
            ActivitySource::AgentBudgetNotice(-1),
            "agent_budget_exceeded",
            true,
        )
    });
    assert!(
        absent.is_none(),
        "caller authorization never bypasses source persistence"
    );
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

#[test]
fn ws12_generic_recorder_batch_preserves_order_duplicates_state_and_callbacks() {
    let vector = oracle();
    let t = prepare(&vector["setup"]);
    let first = 901880001;
    let second = 901880002;
    let inactive = 901880003;
    let bot = 901880004;
    let handled = t.write(move |tx| {
        for (user, role, status) in [(first, 0, 0), (second, 0, 0), (inactive, 0, 1), (bot, 2, 0)] {
            tx.conn().execute(
                "INSERT INTO users(id,name,role,status,created_at,updated_at) VALUES(?,'Batch recipient',?,?,?,?)",
                rusqlite::params![user, role, status, tx.now(), tx.now()],
            )?;
        }
        ActivityItem::record_from_source(
            tx, id("jason"), &RoomSource(901840001),
            ActivityEventType::parse("work_update")?, SourceAuthorization::CallerAuthorized,
        )?.unwrap().mark_handled(tx)
    });
    t.sink.take();
    let before = t.read(move |conn| ActivityItem::unread_snapshot(conn, first));
    let recipients = [
        second,
        id("jason"),
        first,
        second,
        inactive,
        bot,
        id("david"),
        -1,
    ];
    let recorded = t.write(move |tx| {
        ActivityItem::record_source_for_recipients(
            tx,
            &recipients,
            &RoomSource(901840001),
            ActivityEventType::parse("work_update")?,
            SourceAuthorization::CallerAuthorized,
        )
    });
    assert_eq!(
        recorded.iter().map(|item| item.user_id).collect::<Vec<_>>(),
        [second, id("jason"), first, second]
    );
    assert_eq!(
        recorded[1], handled,
        "a duplicate keeps its handled row unchanged"
    );
    assert_eq!(
        recorded[0], recorded[3],
        "a repeated recipient returns the same row"
    );
    assert!(
        recorded[0].id < recorded[2].id,
        "insertion follows recipient order"
    );
    let frames = t
        .sink
        .take()
        .into_iter()
        .map(|event| match event.as_broadcast().unwrap() {
            crate::broadcasts::Broadcast::Cable { stream, payload } => (stream, payload),
            _ => panic!("expected activity frame"),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        frames,
        vec![
            (
                format!("user_{second}_activity"),
                json!({"activityItemId": recorded[0].id})
            ),
            (
                format!("user_{first}_activity"),
                json!({"activityItemId": recorded[2].id})
            ),
        ]
    );
    let created = t.read(move |conn| ActivityItem::unread_snapshot(conn, first));
    assert_eq!(created.revision, before.revision + 1);

    let repeated = t.write(move |tx| {
        ActivityItem::record_source_for_recipients(
            tx,
            &recipients,
            &RoomSource(901840001),
            ActivityEventType::parse("mention")?,
            SourceAuthorization::CallerAuthorized,
        )
    });
    assert_eq!(
        repeated, recorded,
        "repeats preserve state, type, timestamps and order"
    );
    let denied = t.write(move |tx| {
        ActivityItem::record_source_for_recipients(
            tx,
            &recipients,
            &RoomSource(901840001),
            ActivityEventType::parse("work_update")?,
            SourceAuthorization::SourceRecipients,
        )
    });
    assert!(
        denied.is_empty(),
        "caller authorization never leaks into source policy"
    );
    assert!(t.sink.take().is_empty());
    assert_eq!(
        t.read(move |conn| ActivityItem::unread_snapshot(conn, first)),
        created
    );

    let rollback_before = t.read(|conn| ActivityItem::unread_snapshot(conn, id("jz")));
    let failed: Result<()> = t.try_write(move |tx| {
        ActivityItem::record_source_for_recipients(
            tx,
            &[id("jz"), first],
            &RoomSource(901840001),
            ActivityEventType::parse("work_update")?,
            SourceAuthorization::CallerAuthorized,
        )?;
        Err(Error::Other("source transaction failed".into()))
    });
    assert!(failed.is_err());
    assert!(
        t.read(|conn| ActivityItem::find_by_user_and_source(conn, id("jz"), "Room", 901840001))
            .is_none()
    );
    assert_eq!(
        t.read(|conn| ActivityItem::unread_snapshot(conn, id("jz"))),
        rollback_before
    );
    assert!(
        t.sink.take().is_empty(),
        "rolled-back batches publish no frames"
    );
}

#[test]
fn ws12_budget_notice_batch_reader_and_recorder_reads_at_two_sizes() {
    let vector = oracle();
    let rails: Value = serde_json::from_str(include_str!(
        "../../../../vectors/ws12_budget_notice_reads.json"
    ))
    .unwrap();
    let mut counts = Vec::new();
    for row in rails["rows"].as_array().unwrap() {
        let size = row["size"].as_i64().unwrap();
        let t = prepare(&vector["setup"]);
        let ids=t.write(move|tx|{
            tx.conn().execute("UPDATE agents SET owner_id=NULL WHERE id=(SELECT agent_id FROM agent_budget_notices WHERE id=901840003)",[])?;
            let ids=(0..size).map(|i|901870000+i).collect::<Vec<_>>();
            for uid in &ids {tx.conn().execute("INSERT INTO users(id,name,role,status,created_at,updated_at) VALUES(?,'Budget administrator',1,0,?,?)",rusqlite::params![uid,tx.now(),tx.now()])?;}
            Ok(ids)
        });
        let queries = t.db.capture_queries();
        let records = t.write(move |tx| {
            let notice = crate::AgentBudgetNotice::find(tx.conn(), 901840003)?;
            ActivityItem::record_source_for_recipients(
                tx,
                &ids,
                &notice,
                ActivityEventType::parse("agent_budget_exceeded")?,
                SourceAuthorization::SourceRecipients,
            )
        });
        t.db.stop_capturing_queries();
        let count = queries.lock().unwrap().len();
        let facts = records
            .iter()
            .map(|r| json!([r.user_id, r.event_type, r.unread()]))
            .collect::<Vec<_>>();
        assert_eq!(
            json!(facts),
            row["items"],
            "default typed budget adapter records Rails' complete recipients and states"
        );
        println!(
            "WS12_BUDGET_NOTICE_READS recipients={size} SELECTs={count} Rails={}",
            row["reads"]
        );
        counts.push(count);
    }
    let rows = rails["rows"].as_array().unwrap();
    assert!(
        counts[1] - counts[0]
            <= (rows[1]["reads"].as_u64().unwrap() - rows[0]["reads"].as_u64().unwrap()) as usize,
        "budget notice read growth must not exceed Rails"
    );
}
