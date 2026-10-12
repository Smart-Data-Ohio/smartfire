use crate::fixtures::identify;
use crate::models::huddle_effects::AfkMoved;
use crate::models::huddle_grant::HuddleGrant;
use crate::models::room_delete::HuddleConfig;
use crate::tests::TestDb;
use crate::{Account, Broadcast, CachedStatements, Error, Event, Membership, Room, RoomType, Session, StageRole};

fn config() -> HuddleConfig {
    HuddleConfig { api_secret: Some("afk-test-secret".into()), admin_configured: false }
}

fn policy(db: &TestDb, room_id: Option<i64>, minutes: i64) -> crate::Result<()> {
    db.try_write(move |tx| {
        Account::first_in(tx)?.update(tx, None, None, Some(&[("afk_policy", &serde_json::json!({
            "roomId": room_id, "timeoutMinutes": minutes,
        }).to_string())]))
    })
}

fn setup(kind: RoomType, accessible: bool) -> (TestDb, HuddleGrant, i64) {
    let db = TestDb::new();
    let (grant, destination) = db.write(move |tx| {
        let david = identify("david");
        let kevin = identify("kevin");
        let source = Room::create_for(tx, kind, Some("Source"), david, &[david, kevin])?;
        let members = if accessible { vec![david, kevin] } else { vec![david] };
        let mut destination = Room::create_for(tx, RoomType::Voice, Some("AFK"), david, &members)?;
        destination.update_voice_user_limit(tx, Some(1))?;
        let member = Membership::find_by_room_and_user(tx.conn(), source.id, kevin)?.unwrap();
        let session = Session::start(tx, kevin, None, None)?;
        let mut grant = HuddleGrant::issue(tx, session.id, member.id, source.id, &config())?;
        grant.record_seen(tx)?;
        Ok((grant, destination.id))
    });
    policy(&db, Some(destination), 1).unwrap();
    (db, grant, destination)
}

fn tick(db: &TestDb, grant_id: i64, active: bool) -> bool {
    db.travel(10);
    db.write(move |tx| {
        let mut grant = HuddleGrant::find_by_id(tx.conn(), grant_id)?.unwrap();
        if active { grant.record_activity(tx)?; }
        grant.record_seen(tx)?;
        grant.move_if_idle(tx, &config())
    })
}

#[test]
fn afk_policy_validates_rooms_and_timeouts_and_keeps_unknown_settings() {
    let (db, _, destination) = setup(RoomType::Voice, true);
    db.write(|tx| {
        tx.conn().execute_cached("UPDATE accounts SET settings=json_set(settings,'$.future_policy',json('{\"keep\":true}'))", [])?;
        Ok(())
    });
    for minutes in [1, 5, 15, 30, 60] {
        policy(&db, Some(destination), minutes).unwrap();
    }
    for minutes in [0, 2, 10, 61] {
        assert!(matches!(policy(&db, Some(destination), minutes), Err(Error::RecordInvalid(_))));
    }
    assert!(matches!(policy(&db, Some(-1), 5), Err(Error::RecordInvalid(_))));
    let text_room = db.write(|tx| Room::create_for(tx, RoomType::Closed, Some("Text"), identify("david"), &[])).id;
    assert!(matches!(policy(&db, Some(text_room), 5), Err(Error::RecordInvalid(_))));
    db.write(move |tx| {
        Account::first_in(tx)?.update(tx, None, None, Some(&[("afk_policy", &serde_json::json!({"roomId": destination}).to_string())]))
    });
    db.read(|conn| {
        let settings = Account::first(conn)?.unwrap().settings();
        assert_eq!(settings.afk_policy().timeout_minutes, 5);
        assert_eq!(settings.get("future_policy"), Some(&serde_json::json!({"keep": true})));
        Ok(())
    });
    db.write(move |tx| { tx.conn().execute_cached("UPDATE rooms SET deleted_at=? WHERE id=?", rusqlite::params![tx.now(), destination])?; Ok(()) });
    assert!(matches!(policy(&db, Some(destination), 5), Err(Error::RecordInvalid(_))));
}

#[test]
fn afk_idle_move_revokes_the_source_and_notifies_once_at_the_timeout() {
    let (db, grant, destination) = setup(RoomType::Voice, true);
    for _ in 0..5 { assert!(!tick(&db, grant.id, false)); }
    assert!(tick(&db, grant.id, false));
    db.read(|conn| {
        let ended = HuddleGrant::find_by_id(conn, grant.id)?.unwrap();
        assert!(ended.revoked());
        assert!(!ended.authorized(conn)?);
        Ok(())
    });
    let moves: Vec<AfkMoved> = db.events().iter().filter_map(|event| match event {
        Event::Broadcast(request) if request.kind == AfkMoved::KIND => request.decode::<AfkMoved>().map(Result::unwrap),
        _ => None,
    }).collect();
    assert_eq!(moves.len(), 1);
    assert_eq!((moves[0].user_id, moves[0].room_id, moves[0].timeout_minutes), (grant.user_id, destination, 1));
    assert_eq!(moves[0].identity, grant.identity);
    db.write(move |tx| { let mut stale = grant; assert!(!stale.move_if_idle(tx, &config())?); Ok(()) });
}

#[test]
fn afk_active_and_muted_but_active_callers_stay() {
    for muted in [false, true] {
        let (db, mut grant, _) = setup(RoomType::Voice, true);
        if muted {
            grant = db.write(move |tx| {
                Membership::find(tx.conn(), grant.membership_id)?.set_server_muted(tx, true, &config())?;
                let mut fresh = HuddleGrant::issue(tx, grant.session_id, grant.membership_id, grant.room_id, &config())?;
                fresh.record_seen(tx)?;
                Ok(fresh)
            });
        }
        for _ in 0..12 { assert!(!tick(&db, grant.id, true)); }
        assert!(!db.read(|conn| HuddleGrant::find_by_id(conn, grant.id)).unwrap().revoked());
    }
}

#[test]
fn afk_unset_or_inaccessible_destination_moves_nobody() {
    for accessible in [false, true] {
        let (db, grant, _) = setup(RoomType::Voice, accessible);
        if accessible { policy(&db, None, 5).unwrap(); }
        for _ in 0..36 { assert!(!tick(&db, grant.id, false)); }
    }
}

#[test]
fn afk_stage_audiences_stay_and_speakers_move() {
    for role in [StageRole::Listener, StageRole::Speaker, StageRole::Host] {
        let (db, grant, _) = setup(RoomType::Stage, true);
        let grant = db.write(move |tx| {
            Membership::find(tx.conn(), grant.membership_id)?.change_stage_role_with_config(tx, role, &config())?;
            let mut fresh = HuddleGrant::issue(tx, grant.session_id, grant.membership_id, grant.room_id, &config())?;
            fresh.record_seen(tx)?;
            Ok(fresh)
        });
        for _ in 0..5 { assert!(!tick(&db, grant.id, false)); }
        assert_eq!(tick(&db, grant.id, false), role != StageRole::Listener);
    }
}

#[test]
fn afk_destination_ignores_its_user_limit_at_issue_and_gateway_admission() {
    let (db, grant, destination) = setup(RoomType::Voice, true);
    db.write(move |tx| {
        let member = Membership::find_by_room_and_user(tx.conn(), destination, identify("david"))?.unwrap();
        let session = Session::start(tx, member.user_id, None, None)?;
        HuddleGrant::issue(tx, session.id, member.id, destination, &config())?.record_seen(tx)?;
        let member = Membership::find_by_room_and_user(tx.conn(), destination, grant.user_id)?.unwrap();
        let mut moved = HuddleGrant::issue(tx, grant.session_id, member.id, destination, &config())?;
        assert!(moved.record_seen(tx)?);
        assert_eq!(HuddleGrant::participants_for(tx.conn(), destination, tx.now())?.len(), 2);
        assert!(!moved.move_if_idle(tx, &config())?);
        Ok(())
    });
}
