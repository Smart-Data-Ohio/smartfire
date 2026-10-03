use super::*;
use crate::models::board_automations::{dispatch_digests, dispatch_sla};
use crate::{Room, Timestamp};
use rusqlite::trace::{TraceEvent, TraceEventCodes};
use serde_json::Value;
use std::sync::{Mutex, OnceLock, mpsc};
fn setup() -> TestDb {
    let g: Value =
        serde_json::from_str(include_str!("../../../../vectors/board_automations.json")).unwrap();
    let t = TestDb::with_clock(
        TestClock::frozen_at(Timestamp::parse_db("2026-03-02 16:00:00").unwrap()),
        4,
    );
    let s = g["sla"][0]["setup"].clone();
    t.write(move|tx|{for sql in s.as_array().unwrap(){tx.conn().execute_batch(sql.as_str().unwrap())?;} tx.conn().execute("UPDATE channel_threads SET work_status_changed_at='2026-03-02 12:00:00' WHERE id=970000001",[])?;Ok(())});
    t.sink.take();
    t
}
fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap()
}
#[test]
fn review_pr206_concurrent_sweeps() {
    let t = setup();
    let other = t.another_process();
    let rt = runtime();
    let now = t.now();
    let (a, b) =
        rt.block_on(async { tokio::join!(dispatch_sla(&t.db, now), dispatch_sla(&other, now)) });
    let (a, b) = (a.unwrap(), b.unwrap());
    assert!(a.failed_ids.is_empty() && b.failed_ids.is_empty());
    assert_eq!(a.claims + b.claims, 2);
    let counts = t.read(|c| {
        Ok((
            c.query_row("SELECT COUNT(*) FROM board_sla_nudges", [], |r| {
                r.get::<_, i64>(0)
            })?,
            c.query_row(
                "SELECT COUNT(*) FROM activity_items WHERE source_type='BoardSlaNudge'",
                [],
                |r| r.get::<_, i64>(0),
            )?,
        ))
    });
    assert_eq!(counts, (2, 2));
    assert_eq!(rt.block_on(dispatch_sla(&other, now)).unwrap().claims, 0);
    let (a, b) = rt.block_on(async {
        tokio::join!(dispatch_digests(&t.db, now), dispatch_digests(&other, now))
    });
    let (a, b) = (a.unwrap(), b.unwrap());
    assert!(a.failed_ids.is_empty() && b.failed_ids.is_empty());
    assert_eq!((a.claims + b.claims, a.notes + b.notes), (1, 1));
    assert_eq!(
        rt.block_on(dispatch_digests(&other, now)).unwrap().claims,
        0
    );
    println!(
        "PR206 independent concurrent sweeps: 2 database handles; SLA claims=2 inbox=2; digest claims=1 notes=1; repeats=0; failures=0"
    );
}
struct Gate {
    prefix: &'static str,
    ready: mpsc::Sender<()>,
    resume: mpsc::Receiver<()>,
}
static GATE: OnceLock<Mutex<Option<Gate>>> = OnceLock::new();
fn trace(event: TraceEvent<'_>) {
    if let TraceEvent::Stmt(_, sql) = event {
        let mut gate = GATE.get().unwrap().lock().unwrap();
        if gate
            .as_ref()
            .is_some_and(|gate| sql.starts_with(gate.prefix))
        {
            let gate = gate.take().unwrap();
            gate.ready.send(()).unwrap();
            gate.resume
                .recv_timeout(std::time::Duration::from_secs(30))
                .unwrap();
        }
    }
}
#[test]
fn review_pr206_deleted_board_races_keep_healthy_boards_running() {
    let rails: Value = serde_json::from_str(include_str!(
        "../../../../vectors/board_automation_review_deletion_races.json"
    ))
    .unwrap();
    for (kind, prefix) in [
        (
            "digest",
            "SELECT * FROM rooms WHERE id IN (SELECT value FROM json_each(",
        ),
        (
            "digest",
            "SELECT * FROM users WHERE id IN (SELECT creator_id FROM rooms",
        ),
        ("sla", "SELECT n.channel_thread_id"),
    ] {
        let t = setup();
        t.write(|tx| {
            tx.conn().execute_batch("INSERT INTO rooms(id,type,name,creator_id,created_at,updated_at) VALUES(980900001,'Rooms::Board','Healthy board',127326141,'2026-03-02 16:00:00','2026-03-02 16:00:00'); INSERT INTO memberships(room_id,user_id,created_at,updated_at) VALUES(980900001,127326141,'2026-03-02 16:00:00','2026-03-02 16:00:00'); INSERT INTO board_sla_rules(room_id,work_status,nudge_after_minutes,escalate_after_minutes,created_at,updated_at) VALUES(980900001,'planned',60,240,'2026-03-02 16:00:00','2026-03-02 16:00:00'); INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,work_status_changed_at,created_at,updated_at,last_activity_at) VALUES(980900002,980900001,127326141,'Healthy post','planned','2026-03-02 12:00:00','2026-03-02 12:00:00','2026-03-02 12:00:00','2026-03-02 12:00:00')")?;
            Ok(())
        });
        let other = t.another_process();
        other
            .read_blocking(|conn| {
                conn.trace_v2(TraceEventCodes::SQLITE_TRACE_STMT, Some(trace));
                Ok(())
            })
            .unwrap();
        let (ready_tx, ready_rx) = mpsc::channel();
        let (resume_tx, resume_rx) = mpsc::channel();
        *GATE.get_or_init(|| Mutex::new(None)).lock().unwrap() = Some(Gate {
            prefix,
            ready: ready_tx,
            resume: resume_rx,
        });
        let rt = runtime();
        let now = t.now();
        let handle = rt.spawn(async move {
            if kind == "sla" {
                dispatch_sla(&other, now).await
            } else {
                dispatch_digests(&other, now).await
            }
        });
        ready_rx
            .recv_timeout(std::time::Duration::from_secs(30))
            .unwrap();
        let board = t.read(|c| {
            Ok(c.query_row(
                "SELECT room_id FROM channel_threads WHERE id=970000001",
                [],
                |r| r.get::<_, i64>(0),
            )?)
        });
        t.write(move |tx| Room::find(tx.conn(), board)?.begin_destroy(tx));
        rt.block_on(crate::room_delete::perform_with_config(
            &t.db,
            board,
            crate::room_delete::HuddleConfig::from_env(),
        ))
        .unwrap();
        assert!(t.read(move |c| Room::find_by_id(c, board)).is_none());
        resume_tx.send(()).unwrap();
        let stats = rt
            .block_on(handle)
            .expect("dispatcher must not panic")
            .unwrap();
        let healthy = t.read(move |c| Ok(c.query_row(if kind == "sla" {
            "SELECT COUNT(*) FROM board_sla_nudges WHERE room_id=980900001"
        } else {
            "SELECT COUNT(*) FROM board_stale_digests WHERE room_id=980900001 AND message_id IS NOT NULL"
        }, [], |r| r.get::<_, i64>(0))?));
        let expected = rails["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["kind"] == kind)
            .unwrap();
        assert_eq!(
            healthy,
            expected["healthy_claims"].as_i64().unwrap(),
            "{prefix}"
        );
        assert_eq!(expected["destroyed"], true);
        assert!(!stats.failed_ids.contains(&980900001));
        println!("PR206 delete race {kind} before {prefix}: no panic; healthy claims={healthy}");
    }
}
