use crate::fixtures::identify;
use crate::models::huddle_grant::{HuddleGrant, IN_CALL_WINDOW};
use crate::models::room_delete::HuddleConfig;
use crate::tests::TestDb;
use crate::{Error, Membership, Room, RoomType, Session};

fn config() -> HuddleConfig {
    HuddleConfig {
        api_secret: Some("user-limit-secret".into()),
        admin_configured: false,
    }
}

fn room(db: &TestDb, limit: Option<i64>) -> i64 {
    db.clock.travel_to(db.now());
    db.write(move |tx| {
        let mut room = Room::create_for(
            tx,
            RoomType::Voice,
            Some("Limited"),
            identify("david"),
            &[
                identify("david"),
                identify("kevin"),
                identify("jz"),
                identify("jason"),
            ],
        )?;
        room.update_voice_user_limit(tx, limit)?;
        Ok(room.id)
    })
}

fn caller(db: &TestDb, room_id: i64, name: &'static str) -> (i64, i64) {
    db.write(move |tx| {
        let user_id = identify(name);
        let membership = Membership::find_by_room_and_user(tx.conn(), room_id, user_id)?.unwrap();
        let session = Session::start(tx, user_id, None, None)?;
        Ok((membership.id, session.id))
    })
}

fn issue(db: &TestDb, room: i64, caller: (i64, i64)) -> crate::Result<HuddleGrant> {
    db.try_write(move |tx| HuddleGrant::issue(tx, caller.1, caller.0, room, &config()))
}

#[test]
fn voice_user_limit_validates_round_trips_and_clears_without_jobs() {
    let db = TestDb::new();
    let room = room(&db, None);
    assert_eq!(
        db.read(|conn| Room::find(conn, room)).voice_user_limit,
        None
    );
    let events = db.events();
    for limit in [1, 5, 99] {
        db.write(move |tx| Room::find(tx.conn(), room)?.update_voice_user_limit(tx, Some(limit)));
        assert_eq!(
            db.read(|conn| Room::find(conn, room)).voice_user_limit,
            Some(limit)
        );
        let joined = db.read(|conn| Membership::visible_with_ordered_room(conn, identify("david")));
        assert_eq!(
            joined
                .iter()
                .find(|(_, r)| r.id == room)
                .unwrap()
                .1
                .voice_user_limit,
            Some(limit)
        );
    }
    let before = db.read(|conn| Room::find(conn, room));
    for limit in [-1, 0, 100, i64::MAX] {
        let error = db
            .try_write(move |tx| {
                Room::find(tx.conn(), room)?.update_voice_user_limit(tx, Some(limit))
            })
            .unwrap_err();
        let Error::RecordInvalid(errors) = error else {
            panic!("expected validation")
        };
        assert_eq!(errors.on("voice_user_limit"), ["must be between 1 and 99"]);
        assert_eq!(db.read(|conn| Room::find(conn, room)), before);
    }
    db.write(move |tx| Room::find(tx.conn(), room)?.update_voice_user_limit(tx, None));
    assert_eq!(
        db.read(|conn| Room::find(conn, room)).voice_user_limit,
        None
    );
    assert_eq!(db.events(), events);
}

#[test]
fn voice_user_limit_reserves_pending_joins_and_releases_abandoned_or_left_slots() {
    let db = TestDb::new();
    let room = room(&db, Some(1));
    let kevin = caller(&db, room, "kevin");
    let jz = caller(&db, room, "jz");
    let grant = issue(&db, room, kevin).unwrap();
    db.write(move |tx| {
        let floor = tx.now().ago(jiff::SignedDuration::from_secs(1));
        HuddleGrant::find_by_id(tx.conn(), grant.id)?
            .unwrap()
            .mark_out_of_call(tx, Some(floor))
    });
    assert!(matches!(issue(&db, room, jz), Err(Error::HuddleRoomFull)));
    db.travel(IN_CALL_WINDOW);
    let other = issue(&db, room, jz).unwrap();
    db.write(move |tx| {
        HuddleGrant::find_by_id(tx.conn(), other.id)?
            .unwrap()
            .mark_out_of_call(tx, None)
    });
    let refreshed = issue(&db, room, kevin).unwrap();
    assert_eq!(refreshed.id, grant.id);
    db.write(move |tx| {
        let mut grant = HuddleGrant::find_by_id(tx.conn(), refreshed.id)?.unwrap();
        grant.record_seen(tx)?;
        grant.mark_out_of_call(tx, None)?;
        Ok(())
    });
    assert!(issue(&db, room, jz).is_ok());
}

#[test]
fn voice_user_limit_counts_users_allows_rejoins_and_admins_and_does_not_kick() {
    let db = TestDb::new();
    let room = room(&db, Some(2));
    let kevin = caller(&db, room, "kevin");
    let kevin_device = caller(&db, room, "kevin");
    let jz = caller(&db, room, "jz");
    let admin = caller(&db, room, "jason");
    let creator = caller(&db, room, "david");
    let first = issue(&db, room, kevin).unwrap();
    issue(&db, room, kevin_device).unwrap();
    let second = issue(&db, room, jz).unwrap();
    db.write(move |tx| {
        HuddleGrant::find_by_id(tx.conn(), first.id)?
            .unwrap()
            .record_seen(tx)?;
        HuddleGrant::find_by_id(tx.conn(), second.id)?
            .unwrap()
            .record_seen(tx)?;
        Room::find(tx.conn(), room)?.update_voice_user_limit(tx, Some(1))
    });
    assert_eq!(issue(&db, room, kevin).unwrap().id, first.id);
    assert!(issue(&db, room, caller(&db, room, "kevin")).is_ok());
    assert!(issue(&db, room, admin).is_ok());
    assert!(issue(&db, room, creator).is_ok());
    assert_eq!(
        db.read(|conn| HuddleGrant::participants_for(conn, room, db.now()))
            .len(),
        2
    );
    assert!(
        !db.read(|conn| HuddleGrant::find_by_id(conn, first.id))
            .unwrap()
            .revoked()
    );
    // Creators have the same room-administration permission as the settings API.
    db.write(move |tx| {
        tx.conn()
            .execute("UPDATE users SET role=0 WHERE id=?", [identify("david")])?;
        Ok(())
    });
    assert!(issue(&db, room, creator).is_ok());
}

#[test]
fn voice_user_limit_admits_only_one_of_two_concurrent_joins_for_the_last_slot() {
    let db = TestDb::new();
    let room = room(&db, Some(1));
    let kevin = caller(&db, room, "kevin");
    let jz = caller(&db, room, "jz");
    let other_process = db.another_process();
    let barrier = std::sync::Barrier::new(2);
    let (a, b) = std::thread::scope(|scope| {
        let a = scope.spawn(|| {
            barrier.wait();
            issue(&db, room, kevin)
        });
        let b = scope.spawn(|| {
            barrier.wait();
            other_process
                .write_blocking(move |tx| HuddleGrant::issue(tx, jz.1, jz.0, room, &config()))
        });
        (a.join().unwrap(), b.join().unwrap())
    });
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert!(matches!(
        (&a, &b),
        (Ok(_), Err(Error::HuddleRoomFull)) | (Err(Error::HuddleRoomFull), Ok(_))
    ));
    assert_eq!(
        db.read(|conn| Ok(conn.query_row(
            "SELECT COUNT(*) FROM huddle_grants WHERE room_id=? AND revoked_at IS NULL",
            [room],
            |row| row.get::<_, i64>(0)
        )?)),
        1
    );
}
