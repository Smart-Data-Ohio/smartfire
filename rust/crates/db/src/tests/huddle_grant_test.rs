use crate::models::huddle_grant::HuddleGrant;
use crate::models::room_delete::HuddleConfig;
use crate::tests::TestDb;
use crate::{CachedStatements, Membership, Session, User};
use crate::{Room, RoomType, StageRole, Timestamp};
use rusqlite::params;

fn config() -> HuddleConfig {
    HuddleConfig {
        api_secret: Some("ws13-fixture-api-secret".into()),
        admin_configured: false,
    }
}

fn issue(db: &TestDb, membership: i64, session: i64) -> HuddleGrant {
    db.write(move |tx| {
        HuddleGrant::issue(
            tx,
            session,
            membership,
            Membership::find(tx.conn(), membership)?.room_id,
            &config(),
        )
    })
}
fn clear_grants(db: &TestDb) {
    db.write(|tx| Ok(tx.conn().execute("DELETE FROM huddle_grants", [])?));
}

fn setup(db: &TestDb) -> (i64, i64, i64) {
    db.write(|tx| {
        let user = User::find_by_id(tx.conn(), crate::fixtures::identify("david"))?.unwrap();
        let membership = Membership::for_user(tx.conn(), user.id)?.into_iter().next().unwrap();
        let session = Session::start(tx, user.id, None, None)?;
        let now = tx.now();
        let id = tx.conn().query_row_cached("INSERT INTO huddle_grants(identity,room_name,session_id,user_id,membership_id,room_id,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?) RETURNING id", params!["ws13-security-participant", "ws13-security-room", session.id, user.id, membership.id, membership.room_id, now, now], |r| r.get(0))?;
        Ok((id, membership.id, session.id))
    })
}

fn revoked(db: &TestDb, id: i64) -> bool {
    db.read(|conn| {
        Ok(conn.query_row(
            "SELECT revoked_at IS NOT NULL FROM huddle_grants WHERE id=?",
            [id],
            |r| r.get(0),
        )?)
    })
}

#[test]
fn huddle_revocation_and_leave_emit_presence_only_after_commit() {
    let db = TestDb::new();
    let (grant, _, _) = setup(&db);
    db.sink.take();
    let sink = db.sink.clone();
    db.write(move |tx| {
        let mut grant = HuddleGrant::find_by_id(tx.conn(), grant)?.unwrap();
        grant.record_seen(tx)?;
        grant.mark_out_of_call(tx, None)?;
        assert!(sink.events().is_empty(), "broadcast escaped before commit");
        Ok(())
    });
    assert!(db.sink.take().iter().any(|event| matches!(event, crate::Event::Broadcast(request) if request.kind == "HuddleGrant#broadcast_voice_presence")), "leave did not emit committed presence");
    db.write(move |tx| {
        HuddleGrant::find_by_id(tx.conn(), grant)?
            .unwrap()
            .revoke(tx, false, &config())
    });
    assert!(db.sink.take().iter().any(|event| matches!(event, crate::Event::Broadcast(request) if request.kind == "HuddleGrant#broadcast_voice_presence")), "revocation did not emit committed presence");
}

#[test]
fn huddle_membership_removal_revokes_before_deleting() {
    let db = TestDb::new();
    let (grant, membership, _) = setup(&db);
    db.write(move |tx| Membership::find(tx.conn(), membership)?.destroy(tx));
    assert!(
        revoked(&db, grant),
        "removed member retained an active grant"
    );
}

#[test]
fn huddle_sign_out_revokes_before_deleting_session() {
    let db = TestDb::new();
    let (grant, _, session) = setup(&db);
    db.write(move |tx| Session::find(tx.conn(), session)?.destroy(tx));
    assert!(
        revoked(&db, grant),
        "signed-out session retained an active grant"
    );
}

#[test]
fn huddle_ban_revokes_all_user_grants() {
    let db = TestDb::new();
    let (grant, _, _) = setup(&db);
    db.write(|tx| User::find(tx.conn(), crate::fixtures::identify("david"))?.ban(tx));
    assert!(revoked(&db, grant), "banned user retained an active grant");
}

#[test]
fn huddle_authorization_relations_match_rails() {
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("../models/huddle_grant_vectors.json")).unwrap();
    for case in vectors["authorization"].as_array().unwrap() {
        let db = TestDb::new();
        let (id, _, _) = setup(&db);
        let name = case["name"].as_str().unwrap().to_string();
        let mutation = name.clone();
        db.try_write(move |tx| {
            let grant = HuddleGrant::find_by_id(tx.conn(), id)?.unwrap();
            match mutation.as_str() {
                "active" => {}
                "revoked" => {
                    tx.conn().execute(
                        "UPDATE huddle_grants SET revoked_at=? WHERE id=?",
                        params![tx.now(), id],
                    )?;
                }
                "missing_session" => {
                    tx.conn()
                        .execute("DELETE FROM sessions WHERE id=?", [grant.session_id])?;
                }
                "wrong_session_user" => {
                    tx.conn().execute(
                        "UPDATE sessions SET user_id=? WHERE id=?",
                        params![crate::fixtures::identify("jason"), grant.session_id],
                    )?;
                }
                "removed_membership" => {
                    tx.conn()
                        .execute("DELETE FROM memberships WHERE id=?", [grant.membership_id])?;
                }
                "wrong_membership_user" => {
                    tx.conn().execute(
                        "UPDATE memberships SET user_id=999999 WHERE id=?",
                        [grant.membership_id],
                    )?;
                }
                "wrong_membership_room" => {
                    tx.conn().execute(
                        "UPDATE memberships SET room_id=999999 WHERE id=?",
                        [grant.membership_id],
                    )?;
                }
                "banned_user" | "deactivated_user" | "bot_user" => {
                    let sql = match mutation.as_str() {
                        "banned_user" => "UPDATE users SET status=2 WHERE id=?",
                        "deactivated_user" => "UPDATE users SET status=1 WHERE id=?",
                        _ => "UPDATE users SET role=2 WHERE id=?",
                    };
                    tx.conn().execute(sql, [grant.user_id])?;
                }
                "missing_room" => {
                    let empty =
                        Room::create(tx, RoomType::Closed, Some("Disposable"), grant.user_id)?;
                    tx.conn().execute(
                        "UPDATE memberships SET room_id=? WHERE id=?",
                        params![empty.id, grant.membership_id],
                    )?;
                    tx.conn().execute(
                        "UPDATE huddle_grants SET room_id=? WHERE id=?",
                        params![empty.id, id],
                    )?;
                    tx.conn()
                        .execute("DELETE FROM rooms WHERE id=?", [empty.id])?;
                }
                "deleted_room" => {
                    tx.conn().execute(
                        "UPDATE rooms SET deleted_at=? WHERE id=?",
                        params![tx.now(), grant.room_id],
                    )?;
                }
                "server_mute_mismatch" | "server_mute_match" => {
                    tx.conn().execute(
                        "UPDATE memberships SET server_muted_at=? WHERE id=?",
                        params![tx.now(), grant.membership_id],
                    )?;
                    if mutation == "server_mute_match" {
                        tx.conn()
                            .execute("UPDATE huddle_grants SET server_muted=1 WHERE id=?", [id])?;
                    }
                }
                "stage_role_mismatch" | "stage_role_match" => {
                    tx.conn().execute(
                        "UPDATE rooms SET type='Rooms::Stage' WHERE id=?",
                        [grant.room_id],
                    )?;
                    tx.conn().execute(
                        "UPDATE memberships SET stage_role='listener' WHERE id=?",
                        [grant.membership_id],
                    )?;
                    tx.conn().execute(
                        "UPDATE huddle_grants SET stage_role=? WHERE id=?",
                        params![
                            if mutation == "stage_role_match" {
                                "listener"
                            } else {
                                "speaker"
                            },
                            id
                        ],
                    )?;
                }
                _ => panic!("unknown vector {mutation}"),
            }
            Ok(())
        })
        .unwrap_or_else(|error| panic!("{name}: {error}"));
        let authorized =
            db.read(|conn| HuddleGrant::find_by_id(conn, id)?.unwrap().authorized(conn));
        assert_eq!(authorized, case["authorized"].as_bool().unwrap(), "{name}");
        let enforced = db
            .write(move |tx| HuddleGrant::authorize_or_revoke(tx, id, &config()))
            .is_some();
        assert_eq!(enforced, authorized, "{name}");
        assert_eq!(revoked(&db, id), !authorized, "{name}");
        // A second check never reactivates a revoked row or creates a second cleanup.
        db.write(move |tx| HuddleGrant::authorize_or_revoke(tx, id, &config()));
        assert_eq!(
            db.read(|conn| Ok(conn.query_row(
                "SELECT count(*) FROM huddle_cleanups",
                [],
                |r| r.get::<_, i64>(0)
            )?)),
            i64::from(!authorized && name != "revoked")
        );
    }
}

#[test]
fn huddle_issuance_reuses_active_grants_and_never_revives_revoked_rows() {
    let db = TestDb::new();
    db.clock.travel_to(Timestamp::from_second(1_767_268_800));
    let (_, membership, session) = setup(&db);
    clear_grants(&db);
    let first = issue(&db, membership, session);
    assert_eq!(first.identity.len(), "campfire-participant-".len() + 64);
    assert!(
        first.identity["campfire-participant-".len()..]
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    );
    db.travel(1);
    let reused = issue(&db, membership, session);
    assert_eq!(first.id, reused.id);
    assert_eq!(first.identity, reused.identity);
    assert_eq!(reused.last_issued_at, Some(db.now()));
    db.write(move |tx| {
        HuddleGrant::find_by_id(tx.conn(), first.id)?
            .unwrap()
            .revoke(tx, true, &config())
    });
    let fresh = issue(&db, membership, session);
    assert_ne!(fresh.id, reused.id);
    assert_ne!(fresh.identity, reused.identity);
    db.write(move |tx| Session::find(tx.conn(), session)?.destroy(tx));
    assert!(
        db.try_write(move |tx| HuddleGrant::issue(
            tx,
            session,
            membership,
            Membership::find(tx.conn(), membership)?.room_id,
            &config()
        ))
        .is_err()
    );
}

#[test]
fn huddle_concurrent_issuance_obeys_partial_unique_index() {
    let db = TestDb::new();
    let (_, membership, session) = setup(&db);
    clear_grants(&db);
    let other = db.another_process();
    let barrier = std::sync::Barrier::new(2);
    let (a, b) = std::thread::scope(|scope| {
        let a = scope.spawn(|| {
            barrier.wait();
            issue(&db, membership, session)
        });
        let b = scope.spawn(|| {
            barrier.wait();
            other
                .write_blocking(move |tx| {
                    HuddleGrant::issue(
                        tx,
                        session,
                        membership,
                        Membership::find(tx.conn(), membership)?.room_id,
                        &config(),
                    )
                })
                .unwrap()
        });
        (a.join().unwrap(), b.join().unwrap())
    });
    assert_eq!(a.id, b.id);
    assert_eq!(a.identity, b.identity);
}

#[test]
fn huddle_liveness_touch_and_first_sighting_jobs_match_rails() {
    let db = TestDb::new();
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("../models/huddle_grant_vectors.json")).unwrap();
    db.clock
        .travel_to(Timestamp::from_second(vectors["now"].as_i64().unwrap()));
    let (id, _, _) = setup(&db);
    assert!(!db.read(|conn| {
        Ok(HuddleGrant::find_by_id(conn, id)?
            .unwrap()
            .in_call(db.now()))
    }));
    let created = db.now();
    for case in vectors["liveness"].as_array().unwrap() {
        db.clock
            .travel_to(Timestamp::from_second(case["at"].as_i64().unwrap()));
        let count = db.events().len();
        let grant = db.write(move |tx| {
            let mut grant = HuddleGrant::find_by_id(tx.conn(), id)?.unwrap();
            grant.record_seen(tx)?;
            Ok(grant)
        });
        assert_eq!(grant.last_seen_at.unwrap().as_second(), case["last_seen"]);
        assert_eq!(grant.in_call(db.now()), case["in_call"]);
        assert_eq!(grant.updated_at, created);
        let jobs: Vec<&str> = db.events()[count..]
            .iter()
            .filter_map(|e| match e {
                crate::Event::Job(j) if j.class == "Huddle::BroadcastPresenceJob" => {
                    Some("presence")
                }
                crate::Event::Job(j) if j.class == "Huddle::JoinNoticeJob" => Some("join"),
                _ => None,
            })
            .collect();
        assert_eq!(serde_json::json!(jobs), case["jobs"]);
    }
    let seen = db.read(|conn| {
        Ok(HuddleGrant::find_by_id(conn, id)?
            .unwrap()
            .last_seen_at
            .unwrap())
    });
    assert!(!db.write(move |tx| {
        HuddleGrant::find_by_id(tx.conn(), id)?
            .unwrap()
            .mark_out_of_call(tx, Some(seen.ago(jiff::SignedDuration::from_secs(1))))
    }));
    assert!(db.write(move |tx| {
        HuddleGrant::find_by_id(tx.conn(), id)?
            .unwrap()
            .mark_out_of_call(tx, Some(seen))
    }));
    assert!(!db.write(move |tx| {
        HuddleGrant::find_by_id(tx.conn(), id)?
            .unwrap()
            .mark_out_of_call(tx, None)
    }));
    assert!(!revoked(&db, id));
}

fn stage(db: &TestDb) -> (i64, i64, i64) {
    db.write(|tx| {
        let david = crate::fixtures::identify("david");
        let jason = crate::fixtures::identify("jason");
        let room = Room::create_for(tx, RoomType::Stage, Some("Stage"), david, &[david, jason])?;
        let membership = Membership::find_by_room_and_user(tx.conn(), room.id, jason)?.unwrap();
        let session = Session::start(tx, jason, None, None)?;
        Ok((room.id, membership.id, session.id))
    })
}
#[test]
fn huddle_role_boundaries_and_server_mute_revoke_transactionally() {
    let db = TestDb::new();
    let (_, membership, session) = stage(&db);
    let listener = issue(&db, membership, session);
    db.write(move |tx| {
        Membership::find(tx.conn(), membership)?.change_stage_role(tx, StageRole::Speaker)
    });
    assert!(revoked(&db, listener.id));
    let speaker = issue(&db, membership, session);
    db.write(move |tx| {
        Membership::find(tx.conn(), membership)?.change_stage_role(tx, StageRole::Host)
    });
    let promoted = db.read(|conn| Ok(HuddleGrant::find_by_id(conn, speaker.id)?.unwrap()));
    assert!(!promoted.revoked());
    assert_eq!(promoted.stage_role.as_deref(), Some("host"));
    assert_eq!(promoted.updated_at, speaker.updated_at);
    assert!(db.write(move |tx| Membership::find(tx.conn(), membership)?.server_mute(tx)));
    assert!(revoked(&db, speaker.id));
    let muted = issue(&db, membership, session);
    assert!(muted.server_muted);
    assert!(!db.write(move |tx| Membership::find(tx.conn(), membership)?.server_mute(tx)));
    assert!(!revoked(&db, muted.id));
    assert!(db.write(move |tx| Membership::find(tx.conn(), membership)?.server_unmute(tx)));
    assert!(revoked(&db, muted.id));
}

#[test]
fn huddle_last_host_validation_preserves_grants_and_streams() {
    let db = TestDb::new();
    let (room, _, _) = stage(&db);
    let (membership, session) = db.write(move |tx| {
        let david = crate::fixtures::identify("david");
        Ok((
            Membership::find_by_room_and_user(tx.conn(), room, david)?
                .unwrap()
                .id,
            Session::start(tx, david, None, None)?.id,
        ))
    });
    let host = issue(&db, membership, session);
    assert!(
        db.try_write(move |tx| Membership::find(tx.conn(), membership)?
            .change_stage_role(tx, StageRole::Listener))
            .is_err()
    );
    assert!(!revoked(&db, host.id));
    assert!(
        db.try_write(move |tx| Membership::find(tx.conn(), membership)?
            .change_stage_role(tx, StageRole::Speaker))
            .is_err()
    );
    assert!(!revoked(&db, host.id));
}

#[test]
fn huddle_last_active_stage_grant_ends_stream_inside_revocation() {
    let db = TestDb::new();
    let (room, membership, session) = stage(&db);
    let first = issue(&db, membership, session);
    let second_session = db.write(move |tx| Session::start(tx, first.user_id, None, None));
    let second = issue(&db, membership, second_session.id);
    db.write(move |tx| Ok(tx.conn().execute("INSERT INTO streams(room_id,membership_id,user_id,quality,started_at,created_at,updated_at) VALUES(?,?,?,'1080p15',?,?,?)", params![room, membership, first.user_id, tx.now(), tx.now(), tx.now()])?));
    let live = || {
        db.read(|conn| {
            Ok(conn.query_row(
                "SELECT ended_at IS NULL FROM streams WHERE room_id=?",
                [room],
                |r| r.get::<_, bool>(0),
            )?)
        })
    };
    db.write(move |tx| {
        HuddleGrant::find_by_id(tx.conn(), first.id)?
            .unwrap()
            .revoke(tx, true, &config())
    });
    assert!(live());
    db.write(move |tx| {
        HuddleGrant::find_by_id(tx.conn(), second.id)?
            .unwrap()
            .revoke(tx, true, &config())
    });
    assert!(!live());
}

#[test]
fn huddle_room_deletion_revokes_without_per_participant_cleanup() {
    let db = TestDb::new();
    let (id, _, _) = setup(&db);
    let room = db.read(|conn| Ok(HuddleGrant::find_by_id(conn, id)?.unwrap().room_id));
    db.write(move |tx| Room::find(tx.conn(), room)?.begin_destroy(tx));
    assert!(revoked(&db, id));
    assert_eq!(
        db.read(|conn| Ok(conn.query_row(
            "SELECT count(*) FROM huddle_cleanups WHERE operation='delete_room'",
            [],
            |r| r.get::<_, i64>(0)
        )?)),
        1
    );
    assert_eq!(
        db.read(|conn| Ok(conn.query_row(
            "SELECT count(*) FROM huddle_cleanups WHERE operation='remove_participant'",
            [],
            |r| r.get::<_, i64>(0)
        )?)),
        0
    );
}

#[test]
fn huddle_issuance_rejects_cross_user_and_stale_room_coordinates() {
    let db = TestDb::new();
    let (_, membership, session) = setup(&db);
    clear_grants(&db);
    let other = db.write(|tx| Session::start(tx, crate::fixtures::identify("jason"), None, None));
    assert!(
        db.try_write(move |tx| HuddleGrant::issue(
            tx,
            other.id,
            membership,
            Membership::find(tx.conn(), membership)?.room_id,
            &config()
        ))
        .is_err()
    );
    let room = db.read(|conn| Ok(Membership::find(conn, membership)?.room_id));
    assert!(
        db.try_write(move |tx| HuddleGrant::issue(tx, session, membership, room + 1, &config()))
            .is_err()
    );
    assert_eq!(
        db.read(|conn| Ok(
            conn.query_row("SELECT count(*) FROM huddle_grants", [], |r| r
                .get::<_, i64>(0))?
        )),
        0
    );
}

#[test]
fn huddle_issuance_retries_three_real_unique_conflicts_without_leaking_rows() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let db = TestDb::new();
    let (_, membership, session) = setup(&db);
    clear_grants(&db);
    db.write(|tx| Ok(tx.conn().execute_batch("CREATE TRIGGER collide_grant BEFORE INSERT ON huddle_grants BEGIN INSERT INTO huddle_grants(identity,room_name,session_id,user_id,membership_id,room_id,created_at,updated_at) VALUES(NEW.identity,NEW.room_name,NEW.session_id,NEW.user_id,NEW.membership_id,NEW.room_id,NEW.created_at,NEW.updated_at); END;")?));
    struct CountingClock {
        now: Timestamp,
        calls: Arc<AtomicUsize>,
    }
    impl crate::Clock for CountingClock {
        fn now(&self) -> Timestamp {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.now
        }
    }
    let calls = Arc::new(AtomicUsize::new(0));
    let mut env = db.db.env().clone();
    env.clock = Arc::new(CountingClock {
        now: db.now(),
        calls: calls.clone(),
    });
    let mut cfg = crate::Config::new(db.db.path());
    cfg.prepare = false;
    let competing = crate::Database::open(cfg, env).unwrap();
    let result = competing.write_blocking(move |tx| {
        HuddleGrant::issue(
            tx,
            session,
            membership,
            Membership::find(tx.conn(), membership)?.room_id,
            &config(),
        )
    });
    assert!(result.unwrap_err().is_record_not_unique());
    // Each attempt asks for the in-call cutoff and then the INSERT timestamp.
    assert_eq!(calls.load(Ordering::SeqCst), 6);
    assert_eq!(
        db.read(|conn| Ok(
            conn.query_row("SELECT count(*) FROM huddle_grants", [], |r| r
                .get::<_, i64>(0))?
        )),
        0
    );
}

#[test]
fn huddle_never_seen_disconnect_is_silent_and_keeps_authorization() {
    let db = TestDb::new();
    let (id, _, _) = setup(&db);
    db.sink.take();
    assert!(!db.write(move |tx| {
        HuddleGrant::find_by_id(tx.conn(), id)?
            .unwrap()
            .mark_out_of_call(tx, None)
    }));
    assert!(db.events().is_empty(), "never-seen disconnect emitted work");
    db.read(|conn| {
        let grant = HuddleGrant::find_by_id(conn, id)?.unwrap();
        assert!(grant.authorized(conn)?);
        assert!(grant.last_seen_at.is_none());
        assert!(!grant.revoked());
        Ok(())
    });
}

fn review_direct_setup(db: &TestDb) -> (i64, i64, i64) {
    db.write(|tx| {
        let room = crate::fixtures::identify("david_and_jason");
        let user = crate::fixtures::identify("david");
        let membership = Membership::find_by_room_and_user(tx.conn(), room, user)?.unwrap();
        let session = Session::start(tx, user, None, None)?;
        Ok((room, membership.id, session.id))
    })
}

fn review_invitation_insert_failure(operation: &str) {
    let oracle: serde_json::Value = serde_json::from_str(include_str!("ws13b_review_fixes.json")).unwrap();
    for case in oracle["failures"].as_array().unwrap().iter().filter(|case| case["operation"] == operation) {
        let db = TestDb::new();
        db.clock.travel_to(Timestamp::parse_db("2026-01-01 12:00:00").unwrap());
        let (room, membership, session) = review_direct_setup(&db);
        let group_recipient = crate::fixtures::identify("kevin");
        if operation == "group" {
            db.write(move |tx| {
                Membership::create_default(tx, room, group_recipient)?;
                Ok(())
            });
        }
        if case["operation"] == "reuse" {
            db.write(move |tx| HuddleGrant::issue(tx, session, membership, room, &config()));
            db.write(|tx| Ok(tx.conn().execute("DELETE FROM activity_items WHERE source_type='HuddleGrant'", [])?));
            db.travel(180);
        }
        db.sink.take();
        let trigger = if operation == "group" {
            format!("CREATE TRIGGER reject_huddle_item BEFORE INSERT ON activity_items WHEN NEW.event_type='huddle_started' AND NEW.user_id={group_recipient} BEGIN SELECT RAISE(ABORT,'review invitation failure'); END")
        } else {
            "CREATE TRIGGER reject_huddle_item BEFORE INSERT ON activity_items WHEN NEW.event_type='huddle_started' BEGIN SELECT RAISE(ABORT,'review invitation failure'); END".to_string()
        };
        db.write(move |tx| Ok(tx.conn().execute_batch(&trigger)?));
        assert!(db.try_write(move |tx| HuddleGrant::issue(tx, session, membership, room, &config())).is_err());
        let pushes = db.sink.take().iter().filter(|event| event.as_job::<crate::models::huddle_notices::PushInvitationJob>().is_some()).count();
        assert_eq!(serde_json::json!(pushes), case["push_jobs"]);
        db.read(move |conn| {
            let count: i64 = conn.query_row("SELECT COUNT(*) FROM huddle_grants WHERE session_id=?", [session], |r| r.get(0))?;
            assert_eq!(serde_json::json!(count), case["grants"], "{}: Rails commits issuance before invitation failure", case["operation"]);
            let issued: Timestamp = conn.query_row("SELECT last_issued_at FROM huddle_grants WHERE session_id=?", [session], |r| r.get(0))?;
            let expected: jiff::Timestamp = case["last_issued_at"].as_str().unwrap().parse().unwrap();
            assert_eq!(issued, Timestamp::from_jiff(expected));
            let count: i64 = conn.query_row("SELECT COUNT(*) FROM activity_items WHERE source_type='HuddleGrant'", [], |r| r.get(0))?;
            assert_eq!(serde_json::json!(count), case["items"]);
            Ok(())
        });
    }
}

#[test]
fn ws13b_review_invitation_insert_failure_keeps_committed_new_grant() {
    review_invitation_insert_failure("create");
}
#[test]
fn ws13b_review_invitation_insert_failure_keeps_committed_reissuance() {
    review_invitation_insert_failure("reuse");
}

#[test]
fn ws13b_review_later_recipient_failure_keeps_earlier_invitation_committed() {
    review_invitation_insert_failure("group");
}

#[test]
fn reviewer_delayed_handled_job_must_not_become_a_second_retry_ring() {
    use crate::models::huddle_invitations::{RingRequest, publish_ring_with_policy};
    let db = TestDb::new();
    let (room, membership, session) = review_direct_setup(&db);
    let recipient = crate::fixtures::identify("jason");
    let grant = db.write(move |tx| HuddleGrant::issue(tx, session, membership, room, &config()));
    let first = db.sink.take().iter().filter_map(|e| e.as_job::<RingRequest>()).find(|r| r.recipient_id == recipient).unwrap();
    let item = first.invitation["activityItemId"].as_i64().unwrap();
    db.write(move |tx| publish_ring_with_policy(tx, &first, None));
    db.sink.take();
    db.write(move |tx| crate::ActivityItem::find(tx.conn(), item)?.mark_handled(tx));
    let mut backlog: Vec<_> = db.sink.take().iter().filter_map(|e| e.as_job::<RingRequest>()).collect();
    assert_eq!(backlog.len(), 1);
    assert_eq!(backlog[0].invitation["state"], "handled");
    db.travel(181);
    let retry = db.write(move |tx| HuddleGrant::issue(tx, session, membership, room, &config()));
    assert_eq!(retry.id, grant.id);
    backlog.extend(db.sink.take().iter().filter_map(|e| e.as_job::<RingRequest>()));
    assert_eq!(backlog.len(), 2, "one handled update and one retry job");
    for request in backlog { db.write(move |tx| publish_ring_with_policy(tx, &request, None)); }
    let frames: Vec<_> = db.sink.take().iter().filter_map(|e| e.as_broadcast()).filter_map(|b| match b {
        crate::broadcasts::Broadcast::Cable{payload,..} => Some(payload["huddleInvitation"].clone()), _ => None
    }).collect();
    let rings = frames.iter().filter(|p| p["eventType"]=="huddle_started" && p["state"]=="unread").count();
    assert_eq!(rings, 1, "a queued handled update was promoted into a fresh retry ring: {frames:?}");
}

#[test]
fn reviewer_issuance_and_ring_publication_stay_after_commit() {
    use crate::models::huddle_invitations::{RingRequest, publish_ring_with_policy};
    let db = TestDb::new();
    let (room, membership, session) = review_direct_setup(&db);
    db.sink.take();
    let sink = db.sink.clone();
    let result: crate::Result<()> = db.try_write(move |tx| {
        HuddleGrant::issue(tx, session, membership, room, &config())?;
        assert!(sink.events().is_empty(), "issuance emitted before commit");
        Err(crate::Error::Other("review rollback".into()))
    });
    assert!(result.is_err());
    assert!(db.sink.take().is_empty());
    let count: i64 = db.read(move |conn| Ok(conn.query_row("SELECT COUNT(*) FROM huddle_grants WHERE session_id=?", [session], |r|r.get(0))?));
    assert_eq!(count, 0);
    db.write(move |tx| HuddleGrant::issue(tx, session, membership, room, &config()));
    let request = db.sink.take().iter().filter_map(|e|e.as_job::<RingRequest>()).next().unwrap();
    let sink = db.sink.clone();
    let result: crate::Result<()> = db.try_write(move |tx| {
        publish_ring_with_policy(tx, &request, None)?;
        assert!(sink.events().is_empty(), "ring escaped before commit");
        Err(crate::Error::Other("review ring rollback".into()))
    });
    assert!(result.is_err());
    assert!(db.sink.take().is_empty());
}
