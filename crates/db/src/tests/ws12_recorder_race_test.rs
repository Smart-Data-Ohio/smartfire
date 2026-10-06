//! Deterministic concurrent deactivation between independent fanout recipient commits.
use super::*;
use crate::models::channel_thread::WorkChanges;
use crate::{ChannelThread, EventSink, NewChannelThread, Room, RoomType, User, WorkThreadEvent};
use std::sync::atomic::{AtomicBool, Ordering};

struct ChangeAfterFirstBroadcast {
    path: std::path::PathBuf,
    events: RecordingSink,
    armed: AtomicBool,
}
impl EventSink for ChangeAfterFirstBroadcast {
    fn emit(&self, event: Event) {
        let first = matches!(event.as_broadcast(),Some(crate::broadcasts::Broadcast::Cable {stream,..}) if stream == format!("user_{}_activity",id("jason")));
        self.events.emit(event);
        if first && self.armed.swap(false, Ordering::SeqCst) {
            // The recipient write has committed before emit. This second SQLite
            // connection models another request deactivating the next recipient.
            let conn = Connection::open(&self.path).unwrap();
            conn.busy_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            conn.execute("UPDATE users SET status=1 WHERE id=?", [id("kevin")])
                .unwrap();
        }
    }
}

#[test]
fn ws12_recorder_rechecks_broadcast_recipient_status_between_commits() {
    let t = channel_thread_test::frozen();
    let thread = t.write(|tx| {
        let room = Room::create_for(
            tx,
            RoomType::Board,
            Some("Recorder race review"),
            id("david"),
            &[id("david"), id("jason"), id("kevin")],
        )?;
        tx.conn().execute(
            "UPDATE memberships SET involvement='everything' WHERE room_id=?",
            [room.id],
        )?;
        ChannelThread::create_board_post(
            tx,
            NewChannelThread {
                room_id: room.id,
                creator_id: id("jason"),
                name: Some("Plan".into()),
                work_status: Some("planned".into()),
                work_owner_id: Some(id("kevin")),
                ..Default::default()
            },
            None,
        )
    });
    t.write(move |tx| {
        tx.conn().execute("DELETE FROM activity_items", [])?;
        Ok(())
    });
    t.sink.take();
    let sink = Arc::new(ChangeAfterFirstBroadcast {
        path: t.db.path().into(),
        events: t.sink.clone(),
        armed: AtomicBool::new(true),
    });
    let mut config = Config::new(t.db.path());
    config.prepare = false;
    let mut env = t.db.env().clone();
    env.sink = sink.clone();
    let db = Database::open(config, env).unwrap();
    db.write_blocking(move |tx| {
        let actor = User::find(tx.conn(), id("david"))?;
        let mut current = ChannelThread::find(tx.conn(), thread.id)?;
        current.update_work(
            tx,
            &actor,
            WorkChanges {
                status: Some(Some("done".into())),
                ..Default::default()
            },
        )?;
        let event = WorkThreadEvent::for_thread(tx.conn(), current.id)?.remove(0);
        Ok(event.id)
    })
    .unwrap();
    assert!(
        !sink.armed.load(Ordering::SeqCst),
        "first-recipient schedule hook must have executed"
    );
    assert!(!t.read(|conn| User::find(conn, id("kevin"))).is_active());
    let later_rows = t.read(|conn| {
        crate::sql::count(
            conn,
            "SELECT COUNT(*) FROM activity_items WHERE user_id=? AND source_type='WorkThreadEvent'",
            [id("kevin")],
        )
    });
    assert_eq!(
        later_rows, 1,
        "existing per-recipient source snapshot behavior is preserved"
    );
    let events = t.sink.take();
    let later_frames = events.iter().filter(|event|matches!(event.as_broadcast(),Some(crate::broadcasts::Broadcast::Cable {stream,..}) if stream == format!("user_{}_activity",id("kevin")))).count();
    println!(
        "PR215_RECORDER_RACE later_recipient_active=false persisted_items={later_rows} activity_frames={later_frames} expected_main_rails_frames=0"
    );
    assert_eq!(
        later_frames, 0,
        "current human status must gate broadcasts after the earlier recipient commit"
    );
}
