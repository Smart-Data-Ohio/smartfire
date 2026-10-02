use super::*;
use crate::tests::TestDb;
use serde_json::json;

fn claims(subject: &str, email: &str) -> Map<String, Value> {
    json!({"sub":subject,"email":email,"hd":" SmartData.NET "})
        .as_object()
        .unwrap()
        .clone()
}

#[test]
fn identity_validations_match_presence_association_and_unique_subject_and_user() {
    let db = TestDb::new();
    db.clock.travel_to(db.now());
    db.write(|tx| {
        let first = user(tx, "first@smartdata.net")?;
        let second = user(tx, "second@smartdata.net")?;
        for result in [
            GoogleIdentity::create(tx, first.id, " ", "first@smartdata.net", "smartdata.net"),
            GoogleIdentity::create(tx, first.id, "first-sub", " ", "smartdata.net"),
            GoogleIdentity::create(tx, -1, "first-sub", "first@smartdata.net", "smartdata.net"),
        ] {
            assert!(matches!(result, Err(Error::RecordInvalid(_))));
        }
        GoogleIdentity::create(
            tx,
            first.id,
            "first-sub",
            "first@smartdata.net",
            "smartdata.net",
        )?;
        assert!(matches!(
            GoogleIdentity::create(
                tx,
                second.id,
                "first-sub",
                "second@smartdata.net",
                "smartdata.net"
            ),
            Err(Error::RecordInvalid(_))
        ));
        assert!(matches!(
            GoogleIdentity::create(
                tx,
                first.id,
                "second-sub",
                "second@smartdata.net",
                "smartdata.net"
            ),
            Err(Error::RecordInvalid(_))
        ));
        Ok(())
    });
}

#[test]
fn one_save_uses_one_timestamp_even_when_the_clock_advances_between_reads() {
    use std::sync::{
        Arc,
        atomic::{AtomicI64, Ordering},
    };
    struct AdvancingClock(AtomicI64);
    impl crate::Clock for AdvancingClock {
        fn now(&self) -> Timestamp {
            Timestamp::from_microsecond(self.0.fetch_add(1, Ordering::SeqCst))
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let mut config = crate::Config::new(dir.path().join("test.sqlite3"));
    config.environment = "test".into();
    let env = crate::Env {
        clock: Arc::new(AdvancingClock(AtomicI64::new(1_800_000_000_000_000))),
        ..Default::default()
    };
    let db = crate::Database::open(config, env).unwrap();
    db.write_blocking(|tx| {
        let user = user(tx, "clock@smartdata.net")?;
        let mut identity = GoogleIdentity::create(
            tx,
            user.id,
            "clock-sub",
            "clock@smartdata.net",
            "smartdata.net",
        )?;
        assert_eq!(
            identity.created_at, identity.updated_at,
            "one insert timestamp"
        );
        identity.update_claims(tx, "new-clock@smartdata.net", "smartdata.net")?;
        assert_eq!(
            GoogleIdentity::for_user(tx.conn(), user.id)?
                .unwrap()
                .updated_at,
            identity.updated_at,
            "loaded timestamp matches persisted save"
        );
        Ok(())
    })
    .unwrap();
}
fn user(tx: &mut Tx<'_>, email: &str) -> Result<User> {
    User::create(
        tx,
        NewUser {
            name: "Member".into(),
            email_address: Some(email.into()),
            ..Default::default()
        },
    )
}
fn reason<T>(result: Result<T>, expected: &str) {
    assert!(
        matches!(result, Err(Error::GoogleSignInRejected(r)) if r == expected),
        "expected {expected}"
    );
}

fn trust_fixture(tx: &Tx<'_>, id: i64) -> Result<bool> {
    Ok(tx.conn().execute("UPDATE users SET google_email_link_allowed=1,email_self_changed_at=NULL WHERE id=? AND (google_email_link_allowed!=1 OR email_self_changed_at IS NOT NULL)",[id])? == 1)
}

#[test]
fn provisions_only_members_without_passwords_and_grants_open_rooms() {
    let db = TestDb::new();
    db.clock.travel_to(db.now());
    let id = db.write(|tx| {
        let result = GoogleIdentity::resolve(tx, &claims("provision-sub", "new@smartdata.net"))?;
        assert_eq!(result.kind, ResolutionKind::Provisioned);
        assert_eq!(result.user.role, Role::Member);
        assert_eq!(result.user.name, "new");
        assert!(result.user.password_digest.is_none());
        let identity = GoogleIdentity::for_user(tx.conn(), result.user.id)?.unwrap();
        assert_eq!(identity.domain.as_deref(), Some("smartdata.net"));
        assert_eq!(identity.created_at, tx.now());
        Ok(result.user.id)
    });
    db.read(|conn| {
        let (open, memberships): (i64,i64) = conn.query_row("SELECT (SELECT count(*) FROM rooms WHERE type='Rooms::Open' AND deleted_at IS NULL),(SELECT count(*) FROM memberships WHERE user_id=?)", [id], |r| Ok((r.get(0)?,r.get(1)?)))?;
        assert!(open > 0); assert_eq!(open, memberships);
        let allowed: bool = conn.query_row("SELECT google_email_link_allowed FROM users WHERE id=?", [id], |r| r.get(0))?;
        assert!(!allowed);
        Ok(())
    });
}

#[test]
fn names_fall_back_from_name_to_given_family_to_email_with_ruby_coercions() {
    let db = TestDb::new();
    db.clock.travel_to(db.now());
    db.write(|tx| {
        for (i, extra, expected) in [
            (0, json!({"name":" Name "}), "Name"),
            (
                1,
                json!({"name":" ","given_name":" Given ","family_name":" Family "}),
                "Given Family",
            ),
            (2, json!({"given_name":"","family_name":" Solo "}), "Solo"),
            (3, json!({}), "u3"),
            (4, json!({"name":123}), "123"),
        ] {
            let mut c = claims(&format!("name-{i}"), &format!("u{i}@smartdata.net"));
            c.extend(extra.as_object().unwrap().clone());
            assert_eq!(GoogleIdentity::resolve(tx, &c)?.user.name, expected);
        }
        Ok(())
    });
}

#[test]
fn immutable_subject_wins_across_email_changes_and_updates_identity_only_when_changed() {
    let db = TestDb::new();
    db.clock.travel_to(db.now());
    let owner = db.write(|tx| {
        let owner = user(tx, "owner@smartdata.net")?;
        GoogleIdentity::link_to_user(tx, &claims("stable", "owner@smartdata.net"), owner.id)?;
        user(tx, "other@smartdata.net")?;
        tx.conn().execute("UPDATE users SET email_address='changed@smartdata.net',email_self_changed_at=? WHERE id=?", params![tx.now(),owner.id])?;
        Ok(owner.id)
    });
    db.clock.travel(jiff::SignedDuration::from_secs(60));
    db.write(move |tx| {
        let r = GoogleIdentity::resolve(tx, &claims("stable", " other@smartdata.net "))?;
        assert_eq!(r.user.id, owner);
        assert_eq!(r.kind, ResolutionKind::Existing);
        assert_eq!(
            r.user.email_address.as_deref(),
            Some("changed@smartdata.net")
        );
        let identity = GoogleIdentity::for_user(tx.conn(), owner)?.unwrap();
        assert_eq!(identity.email, "other@smartdata.net");
        assert_eq!(identity.updated_at, tx.now());
        Ok(())
    });
    db.clock.travel(jiff::SignedDuration::from_secs(60));
    db.write(move |tx| {
        let before = GoogleIdentity::for_user(tx.conn(), owner)?
            .unwrap()
            .updated_at;
        GoogleIdentity::resolve(tx, &claims("stable", "other@smartdata.net"))?;
        assert_eq!(
            GoogleIdentity::for_user(tx.conn(), owner)?
                .unwrap()
                .updated_at,
            before
        );
        Ok(())
    });
}

#[test]
fn security_self_changed_email_never_auto_links_or_provisions_a_duplicate() {
    let db = TestDb::new();
    db.clock.travel_to(db.now());
    let id = db.write(|tx| {
        let u = user(tx, "hire@smartdata.net")?;
        tx.conn().execute(
            "UPDATE users SET google_email_link_allowed=1,email_self_changed_at=? WHERE id=?",
            params![tx.now(), u.id],
        )?;
        Ok(u.id)
    });
    reason(
        db.try_write(|tx| GoogleIdentity::resolve(tx, &claims("hire-sub", "hire@smartdata.net"))),
        "admin_link_required",
    );
    db.read(move |conn| {
        assert!(GoogleIdentity::for_user(conn, id)?.is_none());
        let count: i64 = conn.query_row(
            "SELECT count(*) FROM users WHERE LOWER(email_address)='hire@smartdata.net'",
            [],
            |r| r.get(0),
        )?;
        assert_eq!(count, 1);
        Ok(())
    });
    db.write(move |tx| {
        assert!(trust_fixture(tx, id)?);
        assert!(!trust_fixture(tx, id)?);
        let r = GoogleIdentity::resolve(tx, &claims("hire-sub", "HIRE@smartdata.net"))?;
        assert_eq!(r.user.id, id);
        assert_eq!(r.kind, ResolutionKind::Linked);
        Ok(())
    });
}

#[test]
fn security_new_join_accounts_require_admin_trust_but_can_link_from_their_own_session() {
    let db = TestDb::new();
    db.clock.travel_to(db.now());
    let id = db.write(|tx| user(tx, "join@smartdata.net").map(|u| u.id));
    reason(
        db.try_write(|tx| GoogleIdentity::resolve(tx, &claims("join-sub", "join@smartdata.net"))),
        "admin_link_required",
    );
    db.write(move |tx| {
        let identity =
            GoogleIdentity::link_to_user(tx, &claims("join-sub", "google@smartdata.net"), id)?;
        assert_eq!(identity.user_id, id);
        assert_eq!(identity.email, "google@smartdata.net");
        Ok(())
    });
}

#[test]
fn ambiguous_emails_and_subject_mismatches_are_refused() {
    let db = TestDb::new();
    db.clock.travel_to(db.now());
    db.write(|tx| {
        user(tx, "same@smartdata.net")?;
        user(tx, "SAME@smartdata.net")?;
        let linked = user(tx, "linked@smartdata.net")?;
        trust_fixture(tx, linked.id)?;
        GoogleIdentity::link_to_user(tx, &claims("old-sub", "linked@smartdata.net"), linked.id)?;
        Ok(())
    });
    reason(
        db.try_write(|tx| {
            GoogleIdentity::resolve(tx, &claims("ambiguous-sub", "same@smartdata.net"))
        }),
        "ambiguous",
    );
    reason(
        db.try_write(|tx| GoogleIdentity::resolve(tx, &claims("new-sub", "linked@smartdata.net"))),
        "subject_mismatch",
    );
}

#[test]
fn security_subject_taken_and_already_linked_never_move_an_identity() {
    let db = TestDb::new();
    db.clock.travel_to(db.now());
    let (owner, other) = db.write(|tx| {
        let owner = user(tx, "owner@smartdata.net")?;
        let other = user(tx, "other@smartdata.net")?;
        GoogleIdentity::link_to_user(tx, &claims("owned", "owner@smartdata.net"), owner.id)?;
        Ok((owner.id, other.id))
    });
    reason(
        db.try_write(move |tx| {
            GoogleIdentity::link_to_user(tx, &claims("owned", "other@smartdata.net"), other)
        }),
        "subject_taken",
    );
    reason(
        db.try_write(move |tx| {
            GoogleIdentity::link_to_user(tx, &claims("replacement", "owner@smartdata.net"), owner)
        }),
        "already_linked",
    );
    db.write(move |tx| {
        assert!(GoogleIdentity::unlink(tx, owner)?);
        assert!(!GoogleIdentity::unlink(tx, owner)?);
        assert!(GoogleIdentity::for_subject(tx.conn(), "owned")?.is_none());
        Ok(())
    });
}

#[test]
fn security_deactivated_banned_bot_and_agent_owners_are_refused_by_both_linkers() {
    let db = TestDb::new();
    db.clock.travel_to(db.now());
    for (i, status, role, expected) in [
        (0, Status::Deactivated, Role::Member, "deactivated"),
        (1, Status::Banned, Role::Member, "banned"),
        (2, Status::Active, Role::Bot, "ineligible"),
    ] {
        let id = db.write(move |tx| {
            let u = user(tx, &format!("bad{i}@smartdata.net"))?;
            GoogleIdentity::link_to_user(
                tx,
                &claims(&format!("bad-{i}"), &format!("bad{i}@smartdata.net")),
                u.id,
            )?;
            tx.conn().execute(
                "UPDATE users SET status=?,role=? WHERE id=?",
                params![status, role, u.id],
            )?;
            Ok(u.id)
        });
        reason(
            db.try_write(move |tx| {
                GoogleIdentity::resolve(
                    tx,
                    &claims(&format!("bad-{i}"), &format!("bad{i}@smartdata.net")),
                )
            }),
            expected,
        );
        reason(
            db.try_write(move |tx| {
                GoogleIdentity::link_to_user(tx, &claims("fresh-sub", "fresh@smartdata.net"), id)
            }),
            expected,
        );
    }
    let id = db.write(|tx| {
        let u = user(tx, "agent@smartdata.net")?;
        tx.conn().execute(
            "INSERT INTO agents(user_id,created_at,updated_at) VALUES(?,?,?)",
            params![u.id, tx.now(), tx.now()],
        )?;
        trust_fixture(tx, u.id)?;
        Ok(u.id)
    });
    reason(
        db.try_write(|tx| GoogleIdentity::resolve(tx, &claims("agent-sub", "agent@smartdata.net"))),
        "ineligible",
    );
    reason(
        db.try_write(move |tx| {
            GoogleIdentity::link_to_user(tx, &claims("agent-sub", "agent@smartdata.net"), id)
        }),
        "ineligible",
    );
}

#[test]
fn deactivated_predecessor_match_escapes_like_wildcards() {
    let db = TestDb::new();
    db.clock.travel_to(db.now());
    db.write(|tx| {
        for email in [
            "abc-deactivated-uuid@smartdata.net",
            "100%-deactivated-uuid@smartdata.net",
            "a_b-deactivated-uuid@smartdata.net",
            "slash\\-deactivated-uuid@smartdata.net",
        ] {
            let u = user(tx, email)?;
            tx.conn()
                .execute("UPDATE users SET status=1 WHERE id=?", [u.id])?;
        }
        assert_eq!(
            GoogleIdentity::resolve(tx, &claims("underscore-sub", "a_c@smartdata.net"))?
                .user
                .email_address
                .as_deref(),
            Some("a_c@smartdata.net")
        );
        Ok(())
    });
    for (i, email) in [
        "100%@smartdata.net",
        "a_b@smartdata.net",
        "slash\\@smartdata.net",
    ]
    .into_iter()
    .enumerate()
    {
        reason(
            db.try_write(move |tx| {
                GoogleIdentity::resolve(tx, &claims(&format!("predecessor-{i}"), email))
            }),
            "deactivated",
        );
    }
}

#[test]
fn absent_or_blank_subject_and_email_are_rejected_before_any_writes() {
    let db = TestDb::new();
    db.clock.travel_to(db.now());
    for (subject, email) in [
        ("", "good@smartdata.net"),
        (" \t", "good@smartdata.net"),
        ("good", " "),
    ] {
        reason(
            db.try_write(move |tx| GoogleIdentity::resolve(tx, &claims(subject, email))),
            "bad_token",
        );
    }
}

#[test]
fn identity_insert_failure_rolls_back_provisioned_user_and_memberships() {
    let db = TestDb::new();
    db.clock.travel_to(db.now());
    let before = db.read(User::count);
    db.write(|tx| { tx.conn().execute_batch("CREATE TRIGGER fail_identity BEFORE INSERT ON google_identities BEGIN SELECT RAISE(ABORT,'identity rejected'); END;")?; Ok(()) });
    assert!(
        db.try_write(|tx| GoogleIdentity::resolve(
            tx,
            &claims("rollback-sub", "rollback@smartdata.net")
        ))
        .is_err()
    );
    assert_eq!(db.read(User::count), before);
    assert!(db.events().is_empty());
}

#[test]
fn concurrent_first_sign_ins_provision_exactly_one_owner() {
    let db = TestDb::new();
    db.clock.travel_to(db.now());
    let other = db.another_process();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let worker = |database: crate::Database, barrier: std::sync::Arc<std::sync::Barrier>| {
        std::thread::spawn(move || {
            barrier.wait();
            database
                .write_blocking(|tx| {
                    GoogleIdentity::resolve(tx, &claims("race-sub", "race@smartdata.net"))
                        .map(|r| r.user.id)
                })
                .unwrap()
        })
    };
    let a = worker(db.db.clone(), barrier.clone());
    let b = worker(other, barrier);
    assert_eq!(a.join().unwrap(), b.join().unwrap());
    db.read(|conn| {
        let count: i64 = conn.query_row(
            "SELECT count(*) FROM users WHERE email_address='race@smartdata.net'",
            [],
            |r| r.get(0),
        )?;
        assert_eq!(count, 1);
        Ok(())
    });
}
