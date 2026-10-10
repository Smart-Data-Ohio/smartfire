//! Full-app broadcasts across a real concurrent venue destroy and one failed note.
use crate::controllers::spa::api_tests::{Sync, serve};
use campfire_api_types as api;
use crate::controllers::presenters::test_support::{DAVID, TestApp};
use campfire_db::{Connection, Database, Room};
use serde_json::{Value, json};
use std::ffi::{CStr, c_void};
use std::sync::{
    Arc, Barrier, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

struct Gate {
    missing_venue: bool,
    seen_events: AtomicBool,
    interrupt: AtomicBool,
    fired: AtomicBool,
    ready: mpsc::Sender<()>,
    resume: Mutex<mpsc::Receiver<()>>,
}
unsafe extern "C" fn trace(_: u32, data: *mut c_void, stmt: *mut c_void, _: *mut c_void) -> i32 {
    let gate = unsafe { &*data.cast::<Gate>() };
    let expanded = unsafe { rusqlite::ffi::sqlite3_expanded_sql(stmt.cast()) };
    if expanded.is_null() {
        return 0;
    }
    let sql = unsafe { CStr::from_ptr(expanded) }
        .to_string_lossy()
        .into_owned();
    unsafe { rusqlite::ffi::sqlite3_free(expanded.cast()) };
    let late_events =
        sql.starts_with("SELECT r.message_id,e.* FROM events") && sql.contains("9000000034");
    gate.interrupt
        .store(!gate.missing_venue && late_events, Ordering::SeqCst);
    if late_events {
        gate.seen_events.store(true, Ordering::SeqCst);
    }
    if gate.missing_venue
        && gate.seen_events.load(Ordering::SeqCst)
        && sql.starts_with("SELECT * FROM \"rooms\" WHERE id IN (")
        && sql.contains("974900000")
        && !gate.fired.swap(true, Ordering::SeqCst)
    {
        gate.ready.send(()).unwrap();
        gate.resume
            .lock()
            .unwrap()
            .recv_timeout(std::time::Duration::from_secs(30))
            .unwrap();
    }
    0
}
unsafe extern "C" fn progress(data: *mut c_void) -> i32 {
    i32::from(
        unsafe { &*data.cast::<Gate>() }
            .interrupt
            .swap(false, Ordering::SeqCst),
    )
}
fn install(conn: &Connection, data: usize) {
    // The state outlives all exclusively checked-out connections and is removed
    // before assertions. The trace blocks only the venue SELECT, after events load.
    unsafe {
        assert_eq!(
            rusqlite::ffi::sqlite3_trace_v2(
                conn.handle(),
                if data == 0 {
                    0
                } else {
                    rusqlite::ffi::SQLITE_TRACE_STMT
                },
                if data == 0 { None } else { Some(trace) },
                data as *mut c_void
            ),
            rusqlite::ffi::SQLITE_OK
        );
        rusqlite::ffi::sqlite3_progress_handler(
            conn.handle(),
            if data == 0 { 0 } else { 1 },
            if data == 0 { None } else { Some(progress) },
            data as *mut c_void,
        );
    }
}
async fn readers(db: &Database, count: usize, data: usize) {
    let barrier = Arc::new(Barrier::new(count));
    let mut tasks = Vec::new();
    for _ in 0..count {
        let db = db.clone();
        let b = barrier.clone();
        tasks.push(tokio::spawn(async move {
            db.read(move |c| {
                install(c, data);
                b.wait();
                Ok(())
            })
            .await
            .unwrap()
        }));
    }
    for task in tasks {
        task.await.unwrap();
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn review_pr206_round2_digest_missing_venue() {
    check_digest_kind("missing-venue").await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn review_pr206_round2_digest_failed_broadcast() {
    check_digest_kind("failed-broadcast").await;
}
async fn check_digest_kind(kind: &str) {
    let golden: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/board_automation_review_round2_digests.json"
    ))
    .unwrap();
    for row in golden["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["kind"] == kind)
    {
        let app = TestApp::boot_frozen_with_env(&[("APP_URL", "http://example.com"), ("SPA_ENABLED", "1")])
            .await
            .unwrap()
            .without_job_runner()
            .await;
        let setup = row["setup"].clone();
        app.db()
            .write(move |tx| {
                for sql in setup.as_array().unwrap() {
                    tx.conn().execute_batch(sql.as_str().unwrap())?;
                }
                Ok(())
            })
            .await
            .unwrap();
        let (addr, server) = serve(&app).await;
        let browser = app.sign_in(DAVID).await;
        let topics = (974000000..974000034).map(|id| format!("room:{id}")).collect::<Vec<_>>();
        let mut client = Sync::connect(addr, &browser.cookie_header(), &topics).await;
        client.welcome().await;
        let (ready_tx, ready_rx) = mpsc::channel();
        let (resume_tx, resume_rx) = mpsc::channel();
        let gate = Box::new(Gate {
            missing_venue: row["kind"] == "missing-venue",
            seen_events: AtomicBool::new(false),
            interrupt: AtomicBool::new(false),
            fired: AtomicBool::new(false),
            ready: ready_tx,
            resume: Mutex::new(resume_rx),
        });
        let data = (&*gate as *const Gate) as usize;
        readers(app.db(), app.booted.app.config.db_readers, data).await;
        let db = app.db().clone();
        let now = db.env().now();
        let sweep = tokio::spawn(async move {
            campfire_db::models::board_automations::dispatch_digests(&db, now).await
        });
        if gate.missing_venue {
            tokio::task::spawn_blocking(move || {
                ready_rx
                    .recv_timeout(std::time::Duration::from_secs(30))
                    .unwrap()
            })
            .await
            .unwrap();
            let mut config = campfire_db::Config::new(app.db().path());
            config.prepare = false;
            config.readers = 1;
            let mut env = app.db().env().clone();
            env.sink = Arc::new(campfire_db::NullSink);
            let other = Database::open(config, env).unwrap();
            other
                .write(|tx| Room::find(tx.conn(), 974900000)?.begin_destroy(tx))
                .await
                .unwrap();
            campfire_db::room_delete::perform_with_config(
                &other,
                974900000,
                campfire_db::room_delete::HuddleConfig::from_env(),
            )
            .await
            .unwrap();
            assert!(
                other
                    .read(|c| Room::find_by_id(c, 974900000))
                    .await
                    .unwrap()
                    .is_none()
            );
            resume_tx.send(()).unwrap();
        }
        let stats = sweep.await.unwrap().unwrap();
        readers(app.db(), app.booted.app.config.db_readers, 0).await;
        assert_eq!(
            stats.failed_ids,
            if gate.missing_venue {
                vec![]
            } else {
                vec![974000033]
            }
        );
        let actual=app.db().read(|c|Ok(c.prepare("SELECT room_id,message_id IS NOT NULL FROM board_stale_digests ORDER BY room_id")?.query_map([],|r|Ok(json!([r.get::<_,i64>(0)?,r.get::<_,bool>(1)?])))?.collect::<rusqlite::Result<Vec<_>>>()?)).await.unwrap();
        assert_eq!(json!(actual), row["claims"], "{} claim state", row["kind"]);
        let ids = app.db().read(|conn| Ok(conn.prepare("SELECT message_id FROM board_stale_digests WHERE message_id IS NOT NULL ORDER BY message_id")?.query_map([], |row| row.get::<_, i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?)).await.unwrap();
        assert_eq!(ids.len(), row["frames"].as_array().unwrap().len());
        let mut received = Vec::new();
        for _ in &ids {
            let event = client.until(|event| matches!(event.payload, api::SyncPayload::MessageCreated(_)), |_| false).await;
            let api::SyncPayload::MessageCreated(message) = event.payload else { unreachable!() };
            assert!(message.system_note);
            received.push(message.id);
        }
        received.sort();
        assert_eq!(received, ids, "each successful digest ID arrived over JSON");
        app.booted.app.broadcasts.settle_sync().await;
        campfire_app::cable::sync::publish(&app.booted.app.cable, campfire_cable::sync::Audience::User(DAVID), &api::SyncPayload::RoomRead(api::RoomRead { room_id: -999 }));
        client.until(|event| matches!(&event.payload, api::SyncPayload::RoomRead(read) if read.room_id == -999), |event| matches!(event.payload, api::SyncPayload::MessageCreated(_))).await;
        assert_eq!(stats.notes, row["notes"].as_u64().unwrap() as usize);
        let repeat = campfire_db::models::board_automations::dispatch_digests(app.db(), now)
            .await
            .unwrap();
        assert_eq!((repeat.claims, repeat.notes), (0, 0));
        server.abort();
        println!(
            "PR206 round2 {}: notes={}; published={}; claims match Rails; repeats=0",
            row["kind"],
            stats.notes,
            row["frames"].as_array().unwrap().len()
        );
    }
}
