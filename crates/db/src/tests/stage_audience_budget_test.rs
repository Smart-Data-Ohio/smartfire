use crate::fixtures::identify;
use crate::models::huddle_grant::{HuddleGrant, IN_CALL_WINDOW};
use crate::models::room_delete::HuddleConfig;
use crate::tests::TestDb;
use crate::{Account, Error, Membership, Room, RoomType, Session, StageRole};

fn config() -> HuddleConfig {
    HuddleConfig {
        api_secret: Some("stage-budget-secret".into()),
        admin_configured: false,
    }
}

fn budget(db: &TestDb, budget: i64) {
    db.write(move |tx| {
        Account::first_in(tx)?.update(
            tx,
            None,
            None,
            Some(&[("stage_audience_budget", &budget.to_string())]),
        )
    });
}

fn room(db: &TestDb, limit: i64) -> i64 {
    db.clock.travel_to(db.now());
    budget(db, limit);
    db.write(|tx| {
        Room::create_for(
            tx,
            RoomType::Stage,
            Some("Town Hall"),
            identify("david"),
            &[
                identify("david"),
                identify("kevin"),
                identify("jz"),
                identify("jason"),
            ],
        )
        .map(|room| room.id)
    })
}

fn caller(db: &TestDb, room: i64, name: &'static str) -> (i64, i64) {
    db.write(move |tx| {
        let user = identify(name);
        let member = Membership::find_by_room_and_user(tx.conn(), room, user)?.unwrap();
        Ok((member.id, Session::start(tx, user, None, None)?.id))
    })
}

fn issue(db: &TestDb, room: i64, caller: (i64, i64)) -> crate::Result<HuddleGrant> {
    db.try_write(move |tx| HuddleGrant::issue(tx, caller.1, caller.0, room, &config()))
}

fn admit(tx: &mut crate::Tx<'_>, id: i64) -> crate::Result<bool> {
    HuddleGrant::authorize_or_revoke(tx, id, &config())?
        .unwrap()
        .record_seen(tx)
}

#[test]
fn stage_audience_budget_defaults_validates_and_preserves_other_settings() {
    let db = TestDb::new();
    let mut account = db.read(|conn| Ok(Account::first(conn)?.unwrap()));
    assert_eq!(account.settings().stage_audience_budget(), 300);
    for raw in [
        "{}",
        "not json",
        r#"{"stage_audience_budget":0}"#,
        r#"{"stage_audience_budget":-1}"#,
        r#"{"stage_audience_budget":"300"}"#,
        r#"{"stage_audience_budget":1.5}"#,
        r#"{"stage_audience_budget":9007199254740992}"#,
    ] {
        account.settings_json = Some(raw.into());
        assert_eq!(account.settings().stage_audience_budget(), 300, "{raw}");
    }
    account.settings_json =
        Some(r#"{"upload_limit_bytes":131072000,"future_setting":"keep"}"#.into());
    let mut settings = account.settings();
    settings.assign(&[("stage_audience_budget", "2")]).unwrap();
    assert_eq!(settings.stage_audience_budget(), 2);
    assert_eq!(settings.upload_limit_bytes(), 131_072_000);
    assert!(settings.to_json().contains("future_setting"));
    for raw in ["0", "-1", "1.5", "bad", "9007199254740992"] {
        assert!(settings.assign(&[("stage_audience_budget", raw)]).is_err());
        assert_eq!(settings.stage_audience_budget(), 2);
    }
}

#[test]
fn stage_audience_budget_reserves_distinct_users_and_releases_left_and_expired_slots() {
    let db = TestDb::new();
    let room = room(&db, 1);
    let kevin = caller(&db, room, "kevin");
    let first = issue(&db, room, kevin).unwrap();
    let device = issue(&db, room, caller(&db, room, "kevin")).unwrap();
    assert_eq!(issue(&db, room, kevin).unwrap().id, first.id);
    let jz = caller(&db, room, "jz");
    assert!(matches!(
        issue(&db, room, jz),
        Err(Error::StageAudienceFull)
    ));
    assert_eq!(
        db.read(|conn| HuddleGrant::stage_audience_usage(conn, room, identify("kevin"), db.now())),
        (0, 1, true)
    );
    db.write(move |tx| admit(tx, first.id));
    db.write(move |tx| admit(tx, device.id));
    assert_eq!(
        db.read(|conn| HuddleGrant::stage_audience_usage(conn, room, 0, db.now())),
        (1, 1, false)
    );
    db.write(move |tx| {
        for id in [first.id, device.id] {
            HuddleGrant::find_by_id(tx.conn(), id)?
                .unwrap()
                .mark_out_of_call(tx, None)?;
        }
        Ok(())
    });
    issue(&db, room, jz).unwrap();
    db.travel(IN_CALL_WINDOW);
    assert!(issue(&db, room, kevin).is_ok());
}

#[test]
fn stage_audience_budget_exempts_speakers_hosts_and_both_kinds_of_administrators() {
    let db = TestDb::new();
    let room = room(&db, 1);
    let host = caller(&db, room, "david");
    let speaker = caller(&db, room, "kevin");
    let admin = caller(&db, room, "jason");
    db.write(move |tx| {
        tx.conn()
            .execute("UPDATE users SET role=0 WHERE id=?", [identify("david")])?;
        Membership::find(tx.conn(), speaker.0)?.change_stage_role_with_config(
            tx,
            StageRole::Speaker,
            &config(),
        )
    });
    // A stage uses its audience budget even if an old voice limit is stored on the room.
    db.write(move |tx| Room::find(tx.conn(), room)?.update_voice_user_limit(tx, Some(1)));
    let listener = issue(&db, room, caller(&db, room, "jz")).unwrap();
    db.write(move |tx| admit(tx, listener.id));
    for caller in [speaker, host, admin] {
        let grant = issue(&db, room, caller).unwrap();
        db.write(move |tx| admit(tx, grant.id));
    }
    // The creator stays exempt even if their membership is a listener.
    db.write(move |tx| {
        Membership::find(tx.conn(), speaker.0)?.change_stage_role_with_config(
            tx,
            StageRole::Host,
            &config(),
        )?;
        Membership::find(tx.conn(), host.0)?.change_stage_role_with_config(
            tx,
            StageRole::Listener,
            &config(),
        )
    });
    assert!(issue(&db, room, host).is_ok());
    assert!(issue(&db, room, speaker).is_ok());
    assert_eq!(
        db.read(|conn| HuddleGrant::stage_audience_usage(conn, room, 0, db.now())),
        (1, 1, false)
    );
}

#[test]
fn stage_audience_budget_lowering_keeps_present_users_and_second_devices() {
    let db = TestDb::new();
    let room = room(&db, 2);
    let kevin = caller(&db, room, "kevin");
    let first = issue(&db, room, kevin).unwrap();
    let second = issue(&db, room, caller(&db, room, "jz")).unwrap();
    budget(&db, 1);
    for id in [first.id, second.id] {
        db.write(move |tx| admit(tx, id));
    }
    db.travel(11);
    for id in [first.id, second.id] {
        db.write(move |tx| admit(tx, id));
    }
    assert_eq!(issue(&db, room, kevin).unwrap().id, first.id);
    let device = issue(&db, room, caller(&db, room, "kevin")).unwrap();
    assert!(db.write(move |tx| admit(tx, device.id)));
    assert_eq!(
        db.read(|conn| HuddleGrant::stage_audience_usage(conn, room, 0, db.now())),
        (2, 2, false)
    );
}

#[test]
fn stage_audience_budget_gateway_rechecks_delayed_tokens_and_reacquires_free_slots() {
    let db = TestDb::new();
    let room = room(&db, 1);
    let delayed = issue(&db, room, caller(&db, room, "kevin")).unwrap();
    db.travel(IN_CALL_WINDOW + 1);
    let joined = issue(&db, room, caller(&db, room, "jz")).unwrap();
    let events = db.events();
    assert!(matches!(
        db.try_write(move |tx| admit(tx, delayed.id)),
        Err(Error::StageAudienceFull)
    ));
    assert_eq!(db.events(), events);
    assert_eq!(
        db.read(|conn| HuddleGrant::find_by_id(conn, delayed.id))
            .unwrap()
            .last_seen_at,
        None
    );
    db.write(move |tx| {
        HuddleGrant::find_by_id(tx.conn(), joined.id)?
            .unwrap()
            .mark_out_of_call(tx, None)
    });
    assert!(db.write(move |tx| admit(tx, delayed.id)));
    assert!(matches!(
        issue(&db, room, caller(&db, room, "jz")),
        Err(Error::StageAudienceFull)
    ));
}

#[test]
fn stage_audience_budget_serializes_last_slot_issuance_and_gateway_admission_across_writers() {
    for gateway in [false, true] {
        let db = TestDb::new();
        let room = room(&db, 1);
        let first = caller(&db, room, "kevin");
        let second = caller(&db, room, "jz");
        let grants = if gateway {
            let a = issue(&db, room, first).unwrap();
            db.travel(IN_CALL_WINDOW + 1);
            let b = issue(&db, room, second).unwrap();
            db.travel(IN_CALL_WINDOW + 1);
            Some((a.id, b.id))
        } else {
            None
        };
        let other = db.another_process();
        let barrier = std::sync::Barrier::new(2);
        let (a, b) = std::thread::scope(|scope| {
            let a = scope.spawn(|| {
                barrier.wait();
                db.try_write(move |tx| match grants {
                    Some((id, _)) => admit(tx, id),
                    None => HuddleGrant::issue(tx, first.1, first.0, room, &config()).map(|_| true),
                })
            });
            let b = scope.spawn(|| {
                barrier.wait();
                other.write_blocking(move |tx| match grants {
                    Some((_, id)) => admit(tx, id),
                    None => {
                        HuddleGrant::issue(tx, second.1, second.0, room, &config()).map(|_| true)
                    }
                })
            });
            (a.join().unwrap(), b.join().unwrap())
        });
        assert!(matches!(
            (&a, &b),
            (Ok(true), Err(Error::StageAudienceFull)) | (Err(Error::StageAudienceFull), Ok(true))
        ));
        assert_eq!(
            db.read(|conn| HuddleGrant::stage_audience_usage(conn, room, 0, db.now()))
                .1,
            1
        );
    }
}

#[test]
fn stage_audience_budget_demotion_preserves_admission_but_blocks_new_listeners() {
    let db = TestDb::new();
    let room = room(&db, 1);
    let listener = caller(&db, room, "kevin");
    let speaker = caller(&db, room, "jz");
    let outsider = caller(&db, room, "jason");
    db.write(move |tx| {
        tx.conn()
            .execute("UPDATE users SET role=0 WHERE id=?", [identify("jason")])?;
        Membership::find(tx.conn(), speaker.0)?.change_stage_role_with_config(
            tx,
            StageRole::Speaker,
            &config(),
        )
    });
    for person in [listener, speaker] {
        let grant = issue(&db, room, person).unwrap();
        db.write(move |tx| admit(tx, grant.id));
    }
    db.write(move |tx| {
        Membership::find(tx.conn(), speaker.0)?.change_stage_role_with_config(
            tx,
            StageRole::Listener,
            &config(),
        )
    });
    let replacement =
        issue(&db, room, speaker).expect("existing speaker keeps admission after demotion");
    db.write(move |tx| admit(tx, replacement.id));
    assert_eq!(
        db.read(|conn| HuddleGrant::stage_audience_usage(conn, room, 0, db.now())),
        (2, 2, false)
    );
    assert!(matches!(
        issue(&db, room, outsider),
        Err(Error::StageAudienceFull)
    ));
    db.write(move |tx| {
        HuddleGrant::find_by_id(tx.conn(), replacement.id)?
            .unwrap()
            .mark_out_of_call(tx, None)
    });
    assert!(matches!(
        issue(&db, room, speaker),
        Err(Error::StageAudienceFull)
    ));
}

#[test]
fn stage_audience_budget_reservation_only_release_publishes_once_after_commit() {
    let db = TestDb::new();
    let room = room(&db, 1);
    let grant = issue(&db, room, caller(&db, room, "kevin")).unwrap();
    db.sink.take();
    let sink = db.sink.clone();
    db.write(move |tx| {
        HuddleGrant::find_by_id(tx.conn(), grant.id)?
            .unwrap()
            .mark_out_of_call(tx, None)?;
        assert!(sink.events().is_empty());
        Ok(())
    });
    let events = db.sink.take();
    assert!(events.iter().any(|event| matches!(event, crate::Event::Broadcast(request) if request.kind == "HuddleGrant#broadcast_voice_presence")), "reservation release must refresh stage capacity");
    db.write(move |tx| {
        HuddleGrant::find_by_id(tx.conn(), grant.id)?
            .unwrap()
            .mark_out_of_call(tx, None)
    });
    assert!(db.sink.take().is_empty(), "repeat leave must not republish");
}
