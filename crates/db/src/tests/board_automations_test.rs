//! Pinned Rails rule, sweep, digest, clock and query-growth contracts.
use super::*;
use crate::models::board_automations::{dispatch_digests, dispatch_sla};
use crate::models::notification_push::BoardNudgeJob;
use crate::{BoardSlaNudge, BoardSlaRule, BoardStaleDigest, Message, NewBoardSlaRule, Timestamp};
use rusqlite::{
    params,
    trace::{TraceEvent, TraceEventCodes},
};
use serde_json::{Value, json};
use std::cell::Cell;

fn oracle() -> Value {
    serde_json::from_str(include_str!("../../../../vectors/board_automations.json")).unwrap()
}
fn stamp(value: &Value) -> Timestamp {
    Timestamp::parse_db(value.as_str().unwrap()).unwrap()
}
fn input(row: &Value) -> NewBoardSlaRule {
    NewBoardSlaRule {
        room_id: row["room_id"].as_i64().unwrap(),
        work_status: row["work_status"].as_str().map(str::to_owned),
        nudge_after_minutes: row["nudge_after_minutes"].as_str().map(str::to_owned),
        escalate_after_minutes: row["escalate_after_minutes"].as_str().map(str::to_owned),
    }
}
fn fixture(setup: &Value) -> TestDb {
    let t = TestDb::with_clock(TestClock::frozen_at(stamp(&oracle()["now"])), 4);
    let setup = setup.clone();
    t.write(move |tx| {
        for s in setup.as_array().unwrap() {
            tx.conn().execute_batch(s.as_str().unwrap())?;
        }
        Ok(())
    });
    t.sink.take();
    t
}
#[test]
fn board_automation_rule_models_match_rails_errors_and_dirty_updates() {
    let golden = oracle();
    let setup = &golden["sla"][0]["setup"];
    for row in golden["models"].as_array().unwrap() {
        let t = fixture(setup);
        let row = row.clone();
        t.write(move |tx| {
            tx.conn().execute("DELETE FROM board_sla_rules", [])?;
            if row["duplicate"] == true {
                let r = BoardSlaRule::create(tx, input(&oracle()["models"][0]["input"]))?;
                tx.conn()
                    .execute("UPDATE board_sla_rules SET id=0 WHERE id=?", [r.id])?;
            }
            let attributes = input(&row["input"]);
            let errors =
                BoardSlaRule::validate(tx.conn(), &attributes, row["existing_id"].as_i64())?;
            let mut actual = serde_json::Map::new();
            for (key, message) in &errors.0 {
                actual
                    .entry(key.to_string())
                    .or_insert(json!([]))
                    .as_array_mut()
                    .unwrap()
                    .push(json!(message));
            }
            assert_eq!(json!(actual), row["errors"], "{}", row["name"]);
            assert_eq!(
                json!(errors.full_messages()),
                row["full_messages"],
                "{}",
                row["name"]
            );
            if row["valid"] == true && row["existing_id"].is_null() {
                let mut saved = BoardSlaRule::create(tx, attributes)?;
                assert_eq!(json!(saved.nudge_after_minutes.to_string()), row["nudge"]);
                assert_eq!(json!(saved.escalate_after_minutes.to_string()), row["escalate"]);
                if saved.nudge_after_minutes < 43199 {
                    let mut stale = saved.clone();
                    saved.update(tx, Some("90".into()), Some("43200".into()))?;
                    stale.update(
                        tx,
                        Some(stale.nudge_after_minutes.to_string()),
                        Some("43199".into()),
                    )?;
                    let refreshed = BoardSlaRule::find_by_id(tx.conn(), saved.id)?.unwrap();
                    assert_eq!(refreshed.nudge_after_minutes, 90);
                    assert_eq!(refreshed.escalate_after_minutes, 43199);
                }
                assert_eq!(BoardSlaRule::for_room(tx.conn(), saved.room_id)?.len(), 1);
                saved.destroy(tx)?;
                assert!(BoardSlaRule::find_by_id(tx.conn(), saved.id)?.is_none());
            }
            Ok(())
        });
        assert!(
            t.events().is_empty(),
            "bare rule writes have no audit/notification callback"
        );
    }
}
fn sla_facts(t: &TestDb) -> Value {
    let (claims,items)=t.read(|conn| {
        let claims=crate::sql::query_all(conn,"SELECT channel_thread_id,work_status,stage,recipient_id,status_entered_at FROM board_sla_nudges ORDER BY id",[],|r|Ok(json!([r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,i64>(3)?,r.get::<_,Timestamp>(4)?.jiff().strftime("%Y-%m-%d %H:%M:%S.%6f").to_string()])))?;
        let items=crate::sql::query_all(conn,"SELECT user_id,event_type FROM activity_items WHERE source_type='BoardSlaNudge' ORDER BY id",[],|r|Ok(json!([r.get::<_,i64>(0)?,r.get::<_,String>(1)?])))?;
        Ok((claims,items))
    });
    let pushes = t
        .events()
        .iter()
        .filter_map(|event| event.as_job::<BoardNudgeJob>())
        .map(|job| t.read(|conn| Ok(BoardSlaNudge::find(conn, job.nudge_id)?.recipient_id)))
        .collect::<Vec<_>>();
    json!({"claims":claims,"items":items,"pushes":pushes})
}
#[test]
fn board_automation_sla_sweeps_match_rails_thresholds_recipients_dedupe_and_crossings() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    for row in oracle()["sla"].as_array().unwrap() {
        let t = fixture(&row["setup"]);
        for run in row["runs"].as_array().unwrap() {
            let extra = run["setup"].clone();
            t.write(move |tx| {
                for sql in extra.as_array().unwrap() {
                    tx.conn().execute_batch(sql.as_str().unwrap())?;
                }
                Ok(())
            });
            let stats = runtime
                .block_on(dispatch_sla(&t.db, stamp(&run["now"])))
                .unwrap();
            assert!(stats.failed_ids.is_empty(), "{} {stats:?}", row["name"]);
            assert_eq!(sla_facts(&t), run["facts"], "{}", row["name"]);
        }
    }
}
fn digest_facts(t: &TestDb, before: i64) -> Value {
    t.read(move |conn| {
        let claims=BoardStaleDigest::for_room(conn,486777696)?;
        let mut digests=Vec::new();
        for claim in claims {
            let message=Message::find(conn,claim.message_id.unwrap())?;
            digests.push(json!({"room_id":claim.room_id,"on":claim.digest_on.to_string(),"body":message.body_html(conn)?.unwrap(),"plain":message.plain_text_body(conn,&BasicRichText)?,"system_note":message.system_note,"thread_id":message.thread_id,"streaming":message.streaming}));
        }
        let unread:i64=conn.query_row("SELECT COUNT(*) FROM memberships WHERE room_id=486777696 AND unread_at IS NOT NULL",[],|r|r.get(0))?;
        let items:i64=conn.query_row("SELECT COUNT(*) FROM activity_items",[],|r|r.get(0))?;
        let jobs=t.events().iter().filter_map(|e|match e { Event::Job(j)=>Some(j.class),_=>None }).collect::<Vec<_>>();
        Ok(json!({"digests":digests,"message_delta":Message::count(conn)?-before,"items":items,"unread":unread,"jobs":jobs}))
    })
}
#[test]
fn board_automation_digests_match_rails_daily_claims_quiet_notes_order_and_escaping() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    for row in oracle()["digests"].as_array().unwrap() {
        let t = fixture(&row["setup"]);
        let before = t.read(Message::count);
        for run in row["runs"].as_array().unwrap() {
            let stats = runtime
                .block_on(dispatch_digests(&t.db, stamp(&run["now"])))
                .unwrap();
            assert!(stats.failed_ids.is_empty(), "{} {stats:?}", row["name"]);
            assert_eq!(digest_facts(&t, before), run["facts"], "{}", row["name"]);
        }
        let emitted = t
            .events()
            .iter()
            .filter_map(|event| match event {
                Event::Broadcast(request) => request.decode::<crate::models::board_automations::DigestNotes>().map(|notes| notes.unwrap()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            emitted.iter().map(|notes| notes.message_ids.len()).sum::<usize>(),
            (t.read(Message::count) - before) as usize,
            "one append per quiet note"
        );
    }
}
#[test]
fn board_automation_digest_claim_keeps_rails_failed_post_claim_and_skips_retry() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let t = fixture(&oracle()["digests"][4]["setup"]);
    t.write(|tx| {tx.conn().execute_batch("CREATE TRIGGER reject_digest_note BEFORE INSERT ON messages WHEN NEW.system_note=1 BEGIN SELECT RAISE(ABORT,'reject digest note'); END")?;Ok(())});
    let now = t.now();
    let stats = runtime.block_on(dispatch_digests(&t.db, now)).unwrap();
    assert_eq!(stats.failed_ids, vec![486777696]);
    assert_eq!(stats.claims, 1);
    assert_eq!(stats.notes, 0);
    t.read(|conn| {
        let claims = BoardStaleDigest::for_room(conn, 486777696)?;
        assert_eq!(claims.len(), 1);
        assert!(claims[0].message_id.is_none());
        assert!(BoardStaleDigest::find_by_id(conn, claims[0].id)?.is_some());
        Ok(())
    });
    t.write(|tx| {
        tx.conn().execute_batch("DROP TRIGGER reject_digest_note")?;
        Ok(())
    });
    assert_eq!(
        runtime
            .block_on(dispatch_digests(&t.db, now))
            .unwrap()
            .notes,
        0
    );
}
thread_local! { static READS: Cell<usize> = const { Cell::new(0) }; static WRITES: Cell<usize> = const { Cell::new(0) }; }
fn trace(event: TraceEvent<'_>) {
    if let TraceEvent::Stmt(_, sql) = event {
        let sql = sql.trim_start().to_ascii_uppercase();
        if sql.starts_with("SELECT") {
            READS.with(|n| n.set(n.get() + 1));
        }
        if [
            "BEGIN",
            "COMMIT",
            "ROLLBACK",
            "INSERT",
            "UPDATE",
            "DELETE",
            "SAVEPOINT",
        ]
        .iter()
        .any(|s| sql.starts_with(s))
        {
            WRITES.with(|n| n.set(n.get() + 1));
        }
    }
}
// Trace every reader plus the real writer, not merely the preload helper.
async fn writer_counts(t: &TestDb, reset: bool) -> (usize, usize) {
    t.db.write(move |tx| {
        tx.conn()
            .trace_v2(TraceEventCodes::SQLITE_TRACE_STMT, Some(trace));
        let counts = (READS.with(Cell::get), WRITES.with(Cell::get));
        if reset {
            READS.with(|n| n.set(0));
            WRITES.with(|n| n.set(0));
        }
        Ok(counts)
    })
    .await
    .unwrap()
}
#[test]
fn board_automation_dispatch_reads_at_two_sizes_and_repeat_sweeps_do_not_write() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut measured = Vec::new();
    for kind in ["sla", "digest"] {
        let mut counts = Vec::new();
        for size in [10, 100] {
            let t = fixture(&oracle()["sla"][0]["setup"]);
            t.write(move |tx| {
                tx.conn().execute("DELETE FROM channel_threads WHERE id=970000001",[])?;
                tx.conn().execute("DELETE FROM board_sla_rules",[])?;
                for i in 0..size {
                    let room=980000000+i;
                    tx.conn().execute("INSERT INTO rooms(id,name,type,creator_id,created_at,updated_at) VALUES(?,'Sweep board','Rooms::Board',127326141,?,?)",params![room,tx.now(),tx.now()])?;
                    for user in [127326141,149087659] {tx.conn().execute("INSERT INTO memberships(room_id,user_id,created_at,updated_at) VALUES(?,?,?,?)",params![room,user,tx.now(),tx.now()])?;}
                    tx.conn().execute("INSERT INTO board_sla_rules(room_id,work_status,nudge_after_minutes,escalate_after_minutes,created_at,updated_at) VALUES(?,'in_progress',60,240,?,?)",params![room,tx.now(),tx.now()])?;
                    tx.conn().execute("INSERT INTO channel_threads(room_id,creator_id,name,work_status,work_owner_id,work_status_changed_at,created_at,updated_at,last_activity_at) VALUES(?,127326141,'Sweep post','in_progress',149087659,'2026-03-02 14:00:00',?,?,?)",params![room,tx.now(),tx.now(),tx.now()])?;
                } Ok(())
            });
            runtime.block_on(writer_counts(&t, true));
            let log = t.db.capture_read_queries();
            let stats = if kind == "sla" {
                runtime.block_on(dispatch_sla(&t.db, t.now()))
            } else {
                runtime.block_on(dispatch_digests(&t.db, t.now()))
            }
            .unwrap();
            assert_eq!(stats.claims, size as usize);
            assert!(stats.failed_ids.is_empty());
            t.db.stop_capturing_read_queries();
            let count = runtime.block_on(writer_counts(&t, false)).0 + log.lock().unwrap().len();
            counts.push(count);
            println!("WS12 automation {kind} boards={size}: {count} SELECTs");
            runtime.block_on(writer_counts(&t, true));
            let log = t.db.capture_read_queries();
            let repeat = if kind == "sla" {
                runtime.block_on(dispatch_sla(&t.db, t.now()))
            } else {
                runtime.block_on(dispatch_digests(&t.db, t.now()))
            }
            .unwrap();
            assert_eq!(repeat.claims, 0);
            t.db.stop_capturing_read_queries();
            let (writer_reads, writes) = runtime.block_on(writer_counts(&t, false));
            let count = writer_reads + log.lock().unwrap().len();
            println!("WS12 automation {kind} repeat boards={size}: {count} SELECTs");
            assert!(count <= 2);
            // Ignore tracing's own final empty writer transaction; the sweep must open none.
            assert_eq!(
                writes, 2,
                "only the tracing setup commit and measurement begin may write"
            );
        }
        let golden = oracle();
        let rails = |size| {
            golden["query_counts"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["kind"] == kind && r["size"] == size && r["repeat"] == false)
                .unwrap()["reads"]
                .as_u64()
                .unwrap() as usize
        };
        measured.push((kind, counts, rails(100) - rails(10)));
    }
    for (kind, counts, rail_growth) in measured {
        assert!(
            counts[1].saturating_sub(counts[0]) <= rail_growth,
            "{kind}: {counts:?}; Rails grows {rail_growth}"
        );
    }
}

#[test]
fn board_automation_digest_batches_survive_a_lowered_parameter_limit_with_distinct_creators() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let t = fixture(&oracle()["sla"][0]["setup"]);
    t.write(|tx| {
        tx.conn().execute("DELETE FROM board_sla_rules",[])?;
        for i in 0..100 {
            let id=990000000+i;
            tx.conn().execute("INSERT INTO users(id,name,created_at,updated_at) VALUES(?,'Distinct creator',?,?)",params![id,tx.now(),tx.now()])?;
            tx.conn().execute("INSERT INTO rooms(id,name,type,creator_id,created_at,updated_at) VALUES(?,'Distinct board','Rooms::Board',?,?,?)",params![id,id,tx.now(),tx.now()])?;
            tx.conn().execute("INSERT INTO board_sla_rules(room_id,work_status,nudge_after_minutes,escalate_after_minutes,created_at,updated_at) VALUES(?,'planned',60,240,?,?)",params![id,tx.now(),tx.now()])?;
            tx.conn().execute("INSERT INTO channel_threads(room_id,creator_id,name,work_status,work_status_changed_at,created_at,updated_at,last_activity_at) VALUES(?,?,'Distinct post','planned','2026-03-02 14:00:00',?,?,?)",params![id,id,tx.now(),tx.now(),tx.now()])?;
        }
        assert!(tx.conn().set_limit(rusqlite::limits::Limit::SQLITE_LIMIT_VARIABLE_NUMBER,64)?>=64);
        Ok(())
    });
    runtime.block_on(async {
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let mut handles = Vec::new();
        for _ in 0..2 {
            let db = t.db.clone();
            let barrier = barrier.clone();
            handles.push(tokio::spawn(async move {
                db.read(move |conn| {
                    conn.set_limit(rusqlite::limits::Limit::SQLITE_LIMIT_VARIABLE_NUMBER, 64)?;
                    barrier.wait();
                    Ok(())
                })
                .await
                .unwrap()
            }));
        }
        for handle in handles {
            handle.await.unwrap();
        }
    });
    let stats = runtime.block_on(dispatch_digests(&t.db, t.now())).unwrap();
    assert_eq!(stats.claims, 100);
    assert_eq!(stats.notes, 100);
    assert!(stats.failed_ids.is_empty());
    assert_eq!(
        runtime
            .block_on(dispatch_digests(&t.db, t.now()))
            .unwrap()
            .claims,
        0
    );
    println!(
        "WS12 digest batches: 100 distinct boards and creators, SQLite variable limit 64; 100 notes"
    );
}

#[test]
fn board_automation_destroy_job_matches_rails_cleanup_with_digests_and_scheduled_poll_sources() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    for row in oracle()["cleanup"].as_array().unwrap() {
        let t = fixture(&row["setup"]);
        runtime.block_on(dispatch_sla(&t.db, t.now())).unwrap();
        runtime.block_on(dispatch_digests(&t.db, t.now())).unwrap();
        let extra = if row["with_scheduled"] == true {
            Some(t.write(|tx| {
                let reply = Message::create(
                    tx,
                    crate::NewMessage {
                        room_id: 486777696,
                        creator_id: 127326141,
                        thread_id: Some(970000001),
                        markdown_source: Some("Which option?".into()),
                        ..Default::default()
                    },
                )?;
                let mut poll = crate::Poll::create_for_message(
                    tx,
                    &reply,
                    crate::NewPoll {
                        labels: vec!["A".into(), "B".into()],
                        ..Default::default()
                    },
                )?;
                let option = poll.options(tx.conn())?[0].id;
                poll.cast_vote(tx, 149087659, &[option])?;
                let pending = crate::ScheduledMessage::create(
                    tx,
                    crate::NewScheduledMessage {
                        user_id: 127326141,
                        room_id: 486777696,
                        thread_id: Some(970000001),
                        reply_to_message_id: None,
                        markdown_source: "Threaded nudge".into(),
                        send_at: tx.now().since(jiff::SignedDuration::from_secs(3600)),
                    },
                )?;
                let mut dropped = crate::ScheduledMessage::create(
                    tx,
                    crate::NewScheduledMessage {
                        user_id: 127326141,
                        room_id: 486777696,
                        thread_id: None,
                        reply_to_message_id: None,
                        markdown_source: "Root post".into(),
                        send_at: tx.now().since(jiff::SignedDuration::from_secs(3600)),
                    },
                )?;
                dropped.drop(tx, Some("test"), tx.now())?;
                Ok((poll.id, pending.id, dropped.id))
            }))
        } else {
            None
        };
        t.write(|tx| crate::Room::find(tx.conn(), 486777696)?.begin_destroy(tx));
        runtime
            .block_on(crate::models::room_delete::perform_with_config(
                &t.db,
                486777696,
                crate::models::room_delete::HuddleConfig::default(),
            ))
            .unwrap();
        let counts=t.read(move |conn| {
            let mut counts=serde_json::Map::new();
            for table in ["rooms","messages","channel_threads","board_tag_assignments","board_sla_rules","board_sla_nudges","board_stale_digests","scheduled_messages"] {
                let column=if table=="rooms" {"id"}else{"room_id"};
                let count:i64=conn.query_row(&format!("SELECT COUNT(*) FROM {table} WHERE {column}=486777696"),[],|r|r.get(0))?;
                counts.insert(table.into(),json!(count));
            }
            counts.insert("sla_items".into(),json!(conn.query_row("SELECT COUNT(*) FROM activity_items WHERE event_type='work_sla'",[],|r|r.get::<_,i64>(0))?));
            if let Some((poll,pending,dropped))=extra {
                for table in ["polls","poll_options","poll_votes"] {let col=if table=="polls" {"id"}else{"poll_id"};counts.insert(table.into(),json!(conn.query_row(&format!("SELECT COUNT(*) FROM {table} WHERE {col}=?"),[poll],|r|r.get::<_,i64>(0))?));}
                counts.insert("scheduled_items".into(),json!(conn.query_row("SELECT COUNT(*) FROM activity_items WHERE source_type='ScheduledMessage' AND source_id IN (?,?)",[pending,dropped],|r|r.get::<_,i64>(0))?));
            }
            Ok(json!(counts))
        });
        assert_eq!(counts, row["counts"]);
    }
}

#[test]
fn board_automation_digest_preserves_rails_committed_note_if_linking_the_claim_fails() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    for row in oracle()["failures"].as_array().unwrap() {
        let t = fixture(&row["setup"]);
        let trigger = row["trigger"].as_str().unwrap().to_owned();
        t.write(move |tx| {
            tx.conn().execute_batch(&trigger)?;
            Ok(())
        });
        let before = t.read(Message::count);
        let stats = runtime.block_on(dispatch_digests(&t.db, t.now())).unwrap();
        assert_eq!(stats.failed_ids, vec![486777696]);
        let (claims, attached) = t.read(|conn| {
            Ok((
                conn.query_row("SELECT COUNT(*) FROM board_stale_digests", [], |r| {
                    r.get::<_, i64>(0)
                })?,
                conn.query_row(
                    "SELECT COUNT(*) FROM board_stale_digests WHERE message_id IS NOT NULL",
                    [],
                    |r| r.get::<_, i64>(0),
                )?,
            ))
        });
        assert_eq!(json!(claims), row["claims"]);
        assert_eq!(json!(attached), row["attached"]);
        assert_eq!(
            json!(t.read(Message::count) - before),
            row["message_delta"],
            "{}",
            row["name"]
        );
        assert_eq!(
            runtime
                .block_on(dispatch_digests(&t.db, t.now()))
                .unwrap()
                .claims,
            0
        );
    }
}
