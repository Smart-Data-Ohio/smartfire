use super::*;
use crate::Room;
use crate::models::room_delete::{self, HuddleConfig};
use serde_json::Value;

pub(super) fn golden() -> Value {
    serde_json::from_str(include_str!("ws8_deletion_vectors.json")).unwrap()
}
pub(super) fn setup(g: &Value) -> TestDb {
    let clock =
        TestClock::frozen_at(crate::Timestamp::parse_db(g["now"].as_str().unwrap()).unwrap());
    let t = TestDb::with_clock(clock, 4);
    let sql = g["setup_sql"].as_str().unwrap().to_owned();
    t.write(move |tx| {
        tx.conn().execute_batch("PRAGMA defer_foreign_keys=ON")?;
        tx.conn().execute_batch(&sql)?;
        Ok(())
    });
    t
}
pub(super) fn assert_checks(t: &TestDb, checks: &Value) {
    for check in checks.as_array().unwrap() {
        let sql = check["sql"].as_str().unwrap();
        let rows = t.read(|c| {
            let mut stmt = c.prepare(sql)?;
            let columns = stmt.column_count();
            Ok(stmt
                .query_map([], |r| {
                    (0..columns)
                        .map(|i| {
                            Ok(match r.get_ref(i)? {
                                rusqlite::types::ValueRef::Null => Value::Null,
                                rusqlite::types::ValueRef::Integer(i) => Value::from(i),
                                rusqlite::types::ValueRef::Real(f) => Value::from(f),
                                rusqlite::types::ValueRef::Text(s) => {
                                    Value::from(std::str::from_utf8(s).unwrap())
                                }
                                rusqlite::types::ValueRef::Blob(_) => panic!("unexpected blob"),
                            })
                        })
                        .collect::<rusqlite::Result<Vec<Value>>>()
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?)
        });
        assert_eq!(serde_json::to_value(rows).unwrap(), check["rows"], "{sql}");
    }
}
#[test]
fn room_begin_destroy_matches_rails() {
    let g = golden();
    let t = setup(&g);
    let rid = g["room_id"].as_i64().unwrap();
    t.write(move |tx| {
        room_delete::begin_destroy(tx, &Room::find(tx.conn(), rid)?, &HuddleConfig::default())
    });
    assert_checks(&t, &g["begun"]);
    assert!(
        t.events()
            .iter()
            .any(|e| matches!(e,Event::Job(j) if j.class=="Room::DestroyJob"))
    );
}
#[test]
fn room_destroy_cascades_match_rails() {
    let g = golden();
    let t = setup(&g);
    let rid = g["room_id"].as_i64().unwrap();
    t.write(move |tx| {
        room_delete::begin_destroy(tx, &Room::find(tx.conn(), rid)?, &HuddleConfig::default())
    });
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(room_delete::perform_with_config(&t.db, rid, HuddleConfig{api_secret:Some("fixture-secret".into()),admin_configured:false}))
        .unwrap();
    assert_checks(&t, &g["finished"]);
    let calendar: Vec<_> = t
        .events()
        .iter()
        .filter_map(|e| match e {
            Event::Job(j) if j.class == "Calendar::RemoteDeleteJob" => Some(j.arguments.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(serde_json::to_value(calendar).unwrap(), g["calendar_jobs"]);
}
#[test]
fn stuck_room_claims_match_rails_and_competing_writers() {
    let g = golden();
    let g = &g["recovery"];
    let t = setup(g);
    let n = t.write(|tx| room_delete::reenqueue_stuck(tx, 600));
    assert_eq!(n, g["ids"].as_array().unwrap().len());
    assert_checks(&t, &g["checks"]);
    let ids: Vec<_> = t
        .events()
        .iter()
        .filter_map(|e| match e {
            Event::Job(j) if j.class == "Room::DestroyJob" => j.arguments["room_id"].as_i64(),
            _ => None,
        })
        .collect();
    assert_eq!(serde_json::to_value(ids).unwrap(), g["ids"]);
    let other = t.another_process();
    assert_eq!(
        other
            .write_blocking(|tx| room_delete::reenqueue_stuck(tx, 600))
            .unwrap(),
        g["repeat_ids"].as_array().unwrap().len()
    );
    t.travel(601);
    let writers: Vec<_> = (0..4).map(|_| t.another_process()).collect();
    let claimed: usize = std::thread::scope(|s| {
        writers
            .iter()
            .map(|db| {
                s.spawn(|| {
                    db.write_blocking(|tx| room_delete::reenqueue_stuck(tx, 600))
                        .unwrap()
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|h| h.join().unwrap())
            .sum()
    });
    assert_eq!(claimed, g["expired_claim_count"].as_u64().unwrap() as usize);
}
#[test]
fn room_workers_ignore_live_missing_and_already_destroyed_rooms() {
    let g = golden();
    let t = setup(&g["live"]);
    let rid = g["live"]["room_id"].as_i64().unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(room_delete::perform_with_config(&t.db, rid, HuddleConfig{api_secret:Some("fixture-secret".into()),admin_configured:false})).unwrap();
    runtime.block_on(room_delete::perform(&t.db, -1)).unwrap();
    assert_checks(&t, &g["live"]["checks"]);
    t.write(move |tx| Room::find(tx.conn(), rid)?.begin_destroy(tx));
    runtime.block_on(room_delete::perform_with_config(&t.db, rid, HuddleConfig{api_secret:Some("fixture-secret".into()),admin_configured:false})).unwrap();
    runtime.block_on(room_delete::perform_with_config(&t.db, rid, HuddleConfig{api_secret:Some("fixture-secret".into()),admin_configured:false})).unwrap();
    assert!(t.read(|c| Room::find_by_id(c, rid)).is_none());
}
#[test]
fn room_destroy_commits_progress_and_refreshes_claim_before_retry() {
    let g = golden();
    let t = setup(&g);
    let rid = g["room_id"].as_i64().unwrap();
    t.write(move |tx| { Room::find(tx.conn(),rid)?.begin_destroy(tx)?; tx.conn().execute_batch("CREATE TRIGGER fail_ws8_room BEFORE DELETE ON rooms BEGIN SELECT RAISE(ABORT,'retry'); END")?; Ok(()) });
    t.travel(17);
    let runtime = tokio::runtime::Runtime::new().unwrap();
    assert!(runtime.block_on(room_delete::perform_with_config(&t.db, rid, HuddleConfig{api_secret:Some("fixture-secret".into()),admin_configured:false})).is_err());
    let r = t.read(|c| Room::find(c, rid));
    assert_eq!(r.destroy_enqueued_at, Some(t.now()));
    // Rails oracle captures the same failure after child transactions and before final commit.
    assert_checks(&t, &g["progress"]);
    t.write(|tx| {
        tx.conn().execute_batch("DROP TRIGGER fail_ws8_room")?;
        Ok(())
    });
    runtime.block_on(room_delete::perform_with_config(&t.db, rid, HuddleConfig{api_secret:Some("fixture-secret".into()),admin_configured:false})).unwrap();
    assert_checks(&t, &g["finished"]);
}

#[test]
fn room_directory_scopes_exclude_deleted_rooms() {
    let g = golden();
    let t = setup(&g["recovery"]);
    let mut all = t
        .read(crate::Room::all)
        .into_iter()
        .map(|r| r.id)
        .collect::<Vec<_>>();
    all.sort();
    assert_eq!(serde_json::to_value(all).unwrap(), g["readers"]["all"]);
    let mut closed = t
        .read(|c| Room::of_type(c, crate::RoomType::Closed))
        .into_iter()
        .map(|r| r.id)
        .collect::<Vec<_>>();
    closed.sort();
    assert_eq!(
        serde_json::to_value(closed).unwrap(),
        g["readers"]["closed"]
    );
    assert_eq!(
        t.read(|c| Room::count_of_type(c, crate::RoomType::Closed)),
        g["readers"]["closed"].as_array().unwrap().len() as i64
    );
    assert_eq!(
        t.read(crate::Room::original).unwrap().id,
        g["readers"]["original"].as_i64().unwrap()
    );
}
