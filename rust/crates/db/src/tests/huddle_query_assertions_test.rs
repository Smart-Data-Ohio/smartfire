//! Capture actual executed SQLite statements, including transaction state, at
//! the boundaries where the pinned Rails declarations assert query counts.
use super::{TestDb, huddle_notices_test::insert};
use crate::models::{huddle_grant::HuddleGrant, huddle_notices, room_delete::HuddleConfig};
use crate::{Membership, StageRole, Timestamp};
use serde_json::{Value, json};
use std::ffi::{CStr, c_void};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
struct Statement {
    sql: String,
    transaction: bool,
}
type Statements = Arc<Mutex<Vec<Statement>>>;
struct Trace {
    db: *mut rusqlite::ffi::sqlite3,
    statements: Statements,
}
unsafe extern "C" fn capture(
    _: u32,
    context: *mut c_void,
    statement: *mut c_void,
    _: *mut c_void,
) -> i32 {
    // sqlite3_trace_v2 invokes this synchronously on the connection's own
    // thread. Installation and removal keep the boxed context alive throughout.
    let trace = unsafe { &*(context as *const Trace) };
    let sql = unsafe { rusqlite::ffi::sqlite3_sql(statement.cast()) };
    if !sql.is_null() {
        trace.statements.lock().unwrap().push(Statement {
            sql: unsafe { CStr::from_ptr(sql) }
                .to_string_lossy()
                .into_owned(),
            transaction: unsafe { rusqlite::ffi::sqlite3_get_autocommit(trace.db) == 0 },
        });
    }
    0
}
fn install(conn: &crate::Connection, statements: Statements) -> usize {
    let db = unsafe { conn.handle() };
    let context = Box::into_raw(Box::new(Trace { db, statements }));
    let result = unsafe {
        rusqlite::ffi::sqlite3_trace_v2(
            db,
            rusqlite::ffi::SQLITE_TRACE_STMT,
            Some(capture),
            context.cast(),
        )
    };
    assert_eq!(result, rusqlite::ffi::SQLITE_OK);
    context as usize
}
fn remove(conn: &crate::Connection, context: usize) {
    unsafe {
        assert_eq!(
            rusqlite::ffi::sqlite3_trace_v2(conn.handle(), 0, None, std::ptr::null_mut()),
            rusqlite::ffi::SQLITE_OK
        );
        drop(Box::from_raw(context as *mut Trace));
    }
}
fn normalized(sql: &str) -> String {
    sql.replace('"', "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
fn selects<'a>(statements: &'a [Statement], table: &str) -> Vec<&'a Statement> {
    statements
        .iter()
        .filter(|s| {
            let sql = normalized(&s.sql);
            sql.starts_with("select ") && sql.contains(&format!("from {table} "))
        })
        .collect()
}

fn run(name: &str) {
    let vectors: Value =
        serde_json::from_str(include_str!("huddle_query_assertions.json")).unwrap();
    assert_eq!(vectors["reference_pin"], &include_str!("../../../../parity/reference.sha").trim()[..8]);
    assert_eq!(vectors["cases"].as_array().unwrap().len(), 8);
    let case = vectors["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == name)
        .unwrap();
    let db = TestDb::new();
    db.clock
        .travel_to(Timestamp::from_second(vectors["now"].as_i64().unwrap()));
    let input = case["input"].clone();
    db.write(move |tx| {
        tx.conn().execute_batch("DELETE FROM huddle_cleanups; DELETE FROM activity_items; DELETE FROM huddle_grants; DELETE FROM streams;")?;
        let room = &input["room"];
        let room_id = room["id"].as_i64().unwrap();
        tx.conn().execute("DELETE FROM memberships WHERE room_id=?", [room_id])?;
        // Fixture rooms are shared with seeded messages, so update rather than delete.
        if crate::Room::find_by_id(tx.conn(), room_id)?.is_some() {
            tx.conn().execute("UPDATE rooms SET name=?,type=?,deleted_at=NULL WHERE id=?", rusqlite::params![room["name"].as_str(), room["type"].as_str(), room_id])?;
        } else {
            insert(tx, "rooms", room)?;
        }
        for user in input["users"].as_array().unwrap() {
            tx.conn().execute("INSERT INTO users(id,name,role,status,inbox_preferences,created_at,updated_at) VALUES(?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET name=excluded.name,role=excluded.role,status=excluded.status,inbox_preferences=excluded.inbox_preferences", rusqlite::params![user["id"].as_i64(),user["name"].as_str(),crate::Role::from_name(user["role"].as_str().unwrap()).unwrap(),crate::Status::from_name(user["status"].as_str().unwrap()).unwrap(),user["inbox_preferences"].to_string(),tx.now(),tx.now()])?;
        }
        for member in input["memberships"].as_array().unwrap() { insert(tx, "memberships", member)?; }
        // Session IDs coincide with Rails fixture IDs; extra sessions are inserted.
        for session in input["sessions"].as_array().unwrap() {
            if !crate::sql::exists(tx.conn(), "SELECT 1 FROM sessions WHERE id=?", [session["id"].as_i64().unwrap()])? { insert(tx, "sessions", session)?; }
        }
        for table in ["huddle_grants", "activity_items", "streams"] {
            let field = match table { "huddle_grants" => "grants", "activity_items" => "items", _ => table };
            for row in input[field].as_array().unwrap() { insert(tx, table, row)?; }
        }
        Ok(())
    });
    let statements = Arc::new(Mutex::new(Vec::new()));
    if name.starts_with("preloaded") || name == "unloaded_live" {
        db.read(|conn| {
            let mut room = crate::models::stage::StageRoom::find(conn, 9001)?.unwrap();
            if name != "unloaded_live" {
                room.preload_live_streams(conn)?;
            }
            assert_eq!(
                json!(
                    room.loaded_live_streams()
                        .map(|v| v.iter().map(|s| s.id).collect::<Vec<_>>())
                ),
                case["preloaded_ids"]
            );
            let context = install(conn, statements.clone());
            let actual = room.live_stream(conn);
            remove(conn, context);
            assert_eq!(json!(actual?.map(|s| s.id)), case["value"]);
            Ok(())
        });
        let rows = statements.lock().unwrap();
        assert_eq!(
            rows.len(),
            case["queries"].as_array().unwrap().len(),
            "{name}: {rows:?}"
        );
        if name == "unloaded_live" {
            assert_eq!(selects(&rows, "streams").len(), 1);
            assert!(normalized(&rows[0].sql).contains("ended_at is null"));
        }
        return;
    }
    let trace_rows = statements.clone();
    let context = db.write(move |tx| Ok(install(tx.conn(), trace_rows)));
    statements.lock().unwrap().clear();
    let grant_id = case["input"]["grants"][0]["id"]
        .as_i64()
        .unwrap_or_default();
    let operation = name.to_owned();
    let result = db.try_write(move |tx| match operation.as_str() {
        "notifier_4" | "notifier_8" => huddle_notices::notify_join(tx, grant_id),
        "demotion_lock" => Membership::find(tx.conn(), 9011)?.change_stage_role_with_config(
            tx,
            StageRole::Listener,
            &HuddleConfig::default(),
        ),
        "departure_lock" => Membership::find(tx.conn(), 9011)?.destroy(tx),
        "nonstage_no_stream_query" => HuddleGrant::find_by_id(tx.conn(), grant_id)?
            .unwrap()
            .revoke(tx, true, &HuddleConfig::default()),
        _ => panic!("unknown case {operation}"),
    });
    let rows = statements.lock().unwrap().clone();
    db.write(move |tx| {
        remove(tx.conn(), context);
        Ok(())
    });
    if name == "demotion_lock" {
        let crate::Error::RecordInvalid(errors) = result.unwrap_err() else {
            panic!("expected last-host validation");
        };
        assert_eq!(errors.to_string(), case["error"].as_str().unwrap());
    } else {
        result.unwrap();
    }
    if name.starts_with("notifier") {
        let users = selects(&rows, "users");
        let rings = selects(&rows, "activity_items");
        assert_eq!(
            users.len(),
            case["user_selects"].as_array().unwrap().len(),
            "{name}: {rows:?}"
        );
        assert_eq!(
            rings.len(),
            case["ring_selects"].as_array().unwrap().len(),
            "{name}: {rows:?}"
        );
        let bulk = normalized(&users[0].sql);
        assert!(bulk.contains("id in ("), "{name}: {bulk}");
        assert_eq!(
            bulk.matches('?').count(),
            case["user_selects"][0]["sql"]
                .as_str()
                .unwrap()
                .matches('?')
                .count()
        );
        assert!(normalized(&users[1].sql).contains("join memberships"));
        assert!(normalized(&rings[0].sql).contains("join huddle_grants"));
    } else if name.ends_with("lock") {
        let begin = rows
            .iter()
            .position(|s| normalized(&s.sql).starts_with("begin immediate"))
            .expect("immediate writer transaction must acquire the room lock");
        let room = rows
            .iter()
            .position(|s| normalized(&s.sql).contains("from rooms "))
            .unwrap();
        let check = rows
            .iter()
            .position(|s| {
                normalized(&s.sql).contains("stage_role='host'")
                    && normalized(&s.sql).starts_with("select 1 from memberships")
            })
            .unwrap();
        assert!(begin < room && room < check, "{name}: {rows:?}");
        assert!(rows[room].transaction && rows[check].transaction);
        assert_eq!(json!(["lock", "check_in_transaction"]), case["events"]);
    } else {
        assert!(selects(&rows, "streams").is_empty(), "{name}: {rows:?}");
    }
}
macro_rules! cases { ($($name:ident),* $(,)?) => {$ (
    #[test] fn $name() { run(stringify!($name)); }
)*}; }
cases!(
    notifier_4,
    notifier_8,
    demotion_lock,
    departure_lock,
    preloaded_live,
    preloaded_ended,
    unloaded_live,
    nonstage_no_stream_query
);
