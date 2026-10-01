//! Rails queue-down recovery on a persisted Rails-created tombstone. The HTTP queue
//! failure remains the fixed atomic-SQLite exception, tested separately, not relabeled.
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_kit::Clock;
use campfire_db::{Membership, Room, models::room_delete};
#[tokio::test]
async fn queue_decision_keeps_atomic_http_failure_and_recovers_a_rails_tombstone() {
    let clock = std::sync::Arc::new(campfire_kit::FrozenClock::new(SEED_NOW.parse().unwrap()));
    let mut app = TestApp::boot_with_test_clock(clock.clone()).await.expect("default seed required");
    app.booted.jobs.stop(std::time::Duration::from_secs(1)).await;
    let id = 654632876;
    let mut browser = app.david();
    app.db().write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER ws8br_queue_down BEFORE INSERT ON background_jobs WHEN NEW.job_class='Room::DestroyJob' BEGIN SELECT RAISE(ABORT,'queue unavailable'); END;")?;
        Ok(())
    }).await.unwrap();
    let reply = browser.write(Req::new(Method::DELETE, &format!("/rooms/{id}"))).await;
    assert_eq!(reply.status, StatusCode::INTERNAL_SERVER_ERROR);
    app.db().read(move |conn| {
        let room = Room::find(conn, id)?;
        assert!(room.deleted_at.is_none());
        assert!(room.destroy_enqueued_at.is_none());
        assert!(!Membership::for_room(conn, id)?.is_empty());
        Ok(())
    }).await.unwrap();
    app.db().write(move |tx| {
        tx.conn().execute_batch("DROP TRIGGER ws8br_queue_down")?;
        // Drop-in rollback compatibility: Rails can leave precisely this state
        // after its committed soft deletion and failed Redis after-commit enqueue.
        Room::find(tx.conn(), id)?.begin_destroy(tx)?;
        Ok(())
    }).await.unwrap();
    app.db().write(move |tx| {
        tx.conn().execute("DELETE FROM background_jobs WHERE job_class='Room::DestroyJob' AND json_extract(arguments,'$.room_id')=?", [id])?;
        tx.conn().execute("UPDATE rooms SET destroy_enqueued_at=NULL WHERE id=?", [id])?;
        Ok(())
    }).await.unwrap();
    assert_eq!(browser.get(&format!("/rooms/{id}")).await.status, StatusCode::FOUND);
    assert_eq!(app.db().write(|tx| room_delete::reenqueue_stuck(tx, 600)).await.unwrap(), 0);
    clock.advance(jiff::SignedDuration::from_secs(11 * 60));
    assert_eq!(app.db().write(|tx| room_delete::reenqueue_stuck(tx, 600)).await.unwrap(), 1);
    let jobs = app.db().read(move |conn| {
        let room = Room::find(conn, id)?;
        assert!(room.deleted_at.is_some());
        assert_eq!(room.destroy_enqueued_at, Some(campfire_db::Timestamp::from_jiff(clock.now())));
        Ok(conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Room::DestroyJob' AND json_extract(arguments,'$.room_id')=?", [id], |r| r.get::<_,i64>(0))?)
    }).await.unwrap();
    assert_eq!(jobs, 1);
    assert_eq!(app.db().write(|tx| room_delete::reenqueue_stuck(tx, 600)).await.unwrap(), 0);
    room_delete::perform_with_config(app.db(), id, room_delete::HuddleConfig::default()).await.unwrap();
    assert!(app.db().read(move |conn| Room::find_by_id(conn, id)).await.unwrap().is_none());
}
