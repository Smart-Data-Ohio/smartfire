//! PR #187 regressions against reference-tools/boards/recorder.rb.
use super::*;
use crate::models::channel_thread::WorkChanges;
use crate::{
    ChannelThread, NewChannelThread, NewUser, Room, RoomType, ThreadInvolvement, ThreadMembership,
    User,
};

thread_local! {
    static QUERIES: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}
fn trace(event: rusqlite::trace::TraceEvent<'_>) {
    if let rusqlite::trace::TraceEvent::Stmt(_, sql) = event {
        QUERIES.with(|queries| {
            queries
                .borrow_mut()
                .push(sql.to_lowercase().replace('"', ""))
        });
    }
}
fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../../vectors/boards_recorder.json")).unwrap()
}
fn fanout(followers: usize) -> (usize, usize) {
    let t = channel_thread_test::frozen();
    let mut thread = t.write(move |tx| {
        let room = Room::create_for(
            tx,
            RoomType::Board,
            Some("Query regression"),
            id("david"),
            &[id("david")],
        )?;
        let thread = ChannelThread::create_board_post(
            tx,
            NewChannelThread {
                room_id: room.id,
                creator_id: id("david"),
                name: Some("Post".into()),
                work_status: Some("planned".into()),
                ..Default::default()
            },
            None,
        )?;
        for i in 0..followers {
            let user = User::create(
                tx,
                NewUser {
                    name: format!("Follower {i}"),
                    ..Default::default()
                },
            )?;
            room.grant_to(tx, &[user.id])?;
            ThreadMembership::join(tx, thread.id, user.id)?
                .update_involvement(tx, ThreadInvolvement::Everything)?;
        }
        Ok(thread)
    });
    t.write(move |tx| {
        QUERIES.with(|queries| queries.borrow_mut().clear());
        tx.conn().trace_v2(
            rusqlite::trace::TraceEventCodes::SQLITE_TRACE_STMT,
            Some(trace),
        );
        let actor = User::find(tx.conn(), id("david"))?;
        thread.update_work(
            tx,
            &actor,
            WorkChanges {
                status: Some(Some("done".into())),
                ..Default::default()
            },
        )
    });
    // Include after-commit fanout, which runs after the preceding write closure returns.
    let counts = t.write(|tx| {
        tx.conn()
            .trace_v2(rusqlite::trace::TraceEventCodes::empty(), None);
        Ok(QUERIES.with(|queries| {
            let queries = queries.borrow();
            let selects = queries
                .iter()
                .filter(|sql| sql.starts_with("select"))
                .collect::<Vec<_>>();
            (
                selects
                    .iter()
                    .filter(|sql| {
                        sql.contains("from memberships ")
                            || sql.contains("from thread_memberships ")
                    })
                    .count(),
                selects
                    .iter()
                    .filter(|sql| sql.contains("from users "))
                    .count(),
            )
        }))
    });
    assert_eq!(
        t.read(|conn| Ok(conn.query_row(
            "SELECT COUNT(*) FROM activity_items WHERE source_type='WorkThreadEvent'",
            [],
            |row| row.get::<_, usize>(0)
        )?)),
        followers
    );
    counts
}
#[test]
fn recorder_fanout_queries_stay_within_rails_counts() {
    let mut failures = Vec::new();
    for row in oracle()["fanout"].as_array().unwrap() {
        let followers = row["followers"].as_u64().unwrap() as usize;
        let (memberships, users) = fanout(followers);
        let rails_memberships = row["membership_selects"].as_u64().unwrap() as usize;
        let rails_users = row["user_selects"].as_u64().unwrap() as usize;
        println!(
            "Recorder fanout: followers={followers}; Rust membership/user SELECTs={memberships}/{users}; Rails={rails_memberships}/{rails_users}"
        );
        if memberships > rails_memberships || users > rails_users {
            failures.push((followers, memberships, users));
        }
    }
    assert!(
        failures.is_empty(),
        "fanout exceeds Rails query counts: {failures:?}"
    );
}
fn activity_frames(t: &TestDb, from: usize) -> usize {
    t.events()[from..].iter().filter(|event| matches!(event.as_broadcast(), Some(crate::broadcasts::Broadcast::Cable {stream, ..}) if stream == format!("user_{}_activity", id("jz")))).count()
}
#[test]
fn recorder_unread_repoint_broadcasts_only_when_read_state_changes() {
    let t = channel_thread_test::frozen();
    let thread = t.write(|tx| {
        let room = Room::create_for(
            tx,
            RoomType::Board,
            Some("Broadcast regression"),
            id("david"),
            &[id("david"), id("jz")],
        )?;
        let mut thread = ChannelThread::create_board_post(
            tx,
            NewChannelThread {
                room_id: room.id,
                creator_id: id("david"),
                name: Some("Post".into()),
                work_status: Some("planned".into()),
                ..Default::default()
            },
            None,
        )?;
        ThreadMembership::join(tx, thread.id, id("jz"))?
            .update_involvement(tx, ThreadInvolvement::Everything)?;
        let actor = User::find(tx.conn(), id("david"))?;
        thread.update_work(
            tx,
            &actor,
            WorkChanges {
                status: Some(Some("in_progress".into())),
                ..Default::default()
            },
        )?;
        Ok(thread)
    });
    assert_eq!(activity_frames(&t, 0), 1);
    let item = t.read(|conn| {
        let item_id = conn.query_row(
            "SELECT id FROM activity_items WHERE user_id=? AND event_type='work_update'",
            [id("jz")],
            |row| row.get::<_, i64>(0),
        )?;
        crate::ActivityItem::find(conn, item_id)
    });
    t.travel(60);
    let from = t.events().len();
    let updated = change_work(
        &t,
        &thread,
        "david",
        WorkChanges {
            status: Some(Some("blocked".into())),
            ..Default::default()
        },
    )
    .unwrap();
    let latest = t.read(|conn| crate::ActivityItem::find(conn, item.id));
    assert_eq!(latest.id, item.id);
    assert_ne!(latest.source_id, item.source_id);
    assert!(latest.unread());
    assert_eq!(latest.updated_at, t.now());
    let frames = activity_frames(&t, from);
    println!(
        "Unread grouped repoint: Rust activity frames={frames}; Rails={}",
        oracle()["unread_repoint_activity_frames"]
    );
    assert_eq!(
        frames as u64,
        oracle()["unread_repoint_activity_frames"].as_u64().unwrap()
    );
    t.write(move |tx| {
        crate::ActivityItem::find(tx.conn(), item.id)?.mark_read(tx)?;
        Ok(())
    });
    t.travel(60);
    let from = t.events().len();
    change_work(
        &t,
        &updated,
        "david",
        WorkChanges {
            status: Some(Some("done".into())),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        activity_frames(&t, from) as u64,
        oracle()["read_repoint_activity_frames"].as_u64().unwrap()
    );
    assert!(
        t.read(|conn| crate::ActivityItem::find(conn, item.id))
            .unread()
    );
}

fn change_work(
    t: &TestDb,
    thread: &ChannelThread,
    actor: &str,
    changes: WorkChanges,
) -> Result<ChannelThread> {
    let mut thread = thread.clone();
    let actor = id(actor);
    t.try_write(move |tx| {
        let actor = User::find(tx.conn(), actor)?;
        thread.update_work(tx, &actor, changes)?;
        Ok(thread)
    })
}
