use super::{TestDb, id};
use crate::models::workspace_invite::{InviteExpiry, InviteState, WorkspaceInvite};
use crate::{NewUser, Timestamp, User};
use jiff::SignedDuration;
use sha2::{Digest, Sha256};

fn person(name: &str) -> NewUser {
    NewUser {
        name: name.into(),
        ..Default::default()
    }
}

#[test]
fn workspace_invite_stores_only_digest_and_validates_limits() {
    let t = TestDb::new();
    let (invite, token) =
        t.write(|tx| WorkspaceInvite::create(tx, id("david"), InviteExpiry::Never, None));
    assert_eq!(token.len(), 64);
    let digest = hex::encode(Sha256::digest(token.as_bytes()));
    let stored: String = t.read(move |c| {
        Ok(c.query_row(
            "SELECT token_digest FROM workspace_invites WHERE id=?",
            [invite.id],
            |r| r.get(0),
        )?)
    });
    assert_eq!(stored, digest);
    assert_ne!(stored, token);
    assert_eq!(t.read(WorkspaceInvite::list).len(), 1);
    for cap in [Some(-1), Some(0), Some(2), Some(101), Some(i64::MAX)] {
        assert!(matches!(
            t.try_write(move |tx| WorkspaceInvite::create(
                tx,
                id("david"),
                InviteExpiry::Never,
                cap
            )),
            Err(crate::Error::RecordInvalid(_))
        ));
    }
    assert!(
        t.try_write(|tx| WorkspaceInvite::create(tx, -1, InviteExpiry::Never, None))
            .is_err()
    );
    let columns: Vec<String> = t.read(|c| {
        Ok(c.prepare("PRAGMA table_info(workspace_invites)")?
            .query_map([], |r| r.get(1))?
            .collect::<Result<_, _>>()?)
    });
    assert!(!columns.iter().any(|c| c == "token" || c == "raw_token"));
    assert_eq!(
        WorkspaceInvite::digest("abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn workspace_invite_every_expiry_has_an_exclusive_boundary() {
    let t = TestDb::new();
    let now = Timestamp::from_second(1_790_000_000);
    for (expiry, seconds) in [
        (InviteExpiry::ThirtyMinutes, 1800),
        (InviteExpiry::OneHour, 3600),
        (InviteExpiry::SixHours, 21600),
        (InviteExpiry::TwelveHours, 43200),
        (InviteExpiry::OneDay, 86400),
        (InviteExpiry::SevenDays, 604800),
    ] {
        t.clock.travel_to(now);
        let (invite, token) =
            t.write(move |tx| WorkspaceInvite::create(tx, id("david"), expiry, None));
        let end = now.since(SignedDuration::from_secs(seconds));
        assert_eq!(invite.expires_at, Some(end));
        assert_eq!(
            invite.state(end.ago(SignedDuration::from_micros(1))),
            InviteState::Active
        );
        assert_eq!(invite.state(end), InviteState::Expired);
        assert_eq!(
            invite.state(end.since(SignedDuration::from_micros(1))),
            InviteState::Expired
        );
        t.clock.travel_to(end.ago(SignedDuration::from_micros(1)));
        let active_token = token.clone();
        assert!(
            t.write(move |tx| WorkspaceInvite::redeem(tx, &active_token, person("Just in time")))
                .is_some()
        );
        t.clock.travel_to(end);
        assert!(
            t.write(move |tx| WorkspaceInvite::redeem(tx, &token, person("Expired")))
                .is_none()
        );
        assert_eq!(
            t.read(move |c| WorkspaceInvite::find(c, invite.id))
                .unwrap()
                .uses,
            1
        );
    }
    let (invite, _) =
        t.write(|tx| WorkspaceInvite::create(tx, id("david"), InviteExpiry::Never, None));
    assert_eq!(invite.expires_at, None);
    assert_eq!(
        invite.state(now.since(SignedDuration::from_secs(100_000_000))),
        InviteState::Active
    );
}

#[test]
fn workspace_invite_every_cap_stops_at_its_last_use() {
    let t = TestDb::new();
    for cap in [
        Some(1),
        Some(5),
        Some(10),
        Some(25),
        Some(50),
        Some(100),
        None,
    ] {
        let (invite, token) =
            t.write(move |tx| WorkspaceInvite::create(tx, id("david"), InviteExpiry::Never, cap));
        let use_count = cap.unwrap_or(101);
        // Exercise the penultimate and final use without creating hundreds of fixture users.
        t.write(move |tx| {
            tx.conn().execute(
                "UPDATE workspace_invites SET uses=? WHERE id=?",
                [use_count - use_count.min(2), invite.id],
            )?;
            Ok(())
        });
        for _ in 0..use_count.min(2) {
            let token = token.clone();
            assert!(
                t.write(move |tx| WorkspaceInvite::redeem(tx, &token, person("Joined")))
                    .is_some()
            );
        }
        let saved = t
            .read(move |c| WorkspaceInvite::find(c, invite.id))
            .unwrap();
        assert_eq!(saved.uses, use_count);
        assert_eq!(
            saved.state(t.now()),
            if cap.is_some() {
                InviteState::Exhausted
            } else {
                InviteState::Active
            }
        );
        if cap.is_some() {
            assert!(
                t.write(move |tx| WorkspaceInvite::redeem(tx, &token, person("Too late")))
                    .is_none()
            );
        }
    }
}

#[test]
fn workspace_invite_revocation_is_idempotent_and_failed_creation_rolls_back_use() {
    let t = TestDb::new();
    let (invite, token) =
        t.write(|tx| WorkspaceInvite::create(tx, id("david"), InviteExpiry::Never, Some(1)));
    let duplicate = token.clone();
    let before = t.read(User::count);
    assert!(
        t.try_write(move |tx| WorkspaceInvite::redeem(
            tx,
            &duplicate,
            NewUser {
                name: "Duplicate".into(),
                email_address: Some("david@37signals.com".into()),
                ..Default::default()
            }
        ))
        .is_err()
    );
    assert_eq!(
        t.read(move |c| WorkspaceInvite::find(c, invite.id))
            .unwrap()
            .uses,
        0
    );
    let revoked = t
        .write(move |tx| WorkspaceInvite::revoke(tx, invite.id))
        .unwrap();
    t.travel(10);
    assert_eq!(
        t.write(move |tx| WorkspaceInvite::revoke(tx, invite.id))
            .unwrap()
            .revoked_at,
        revoked.revoked_at
    );
    assert_eq!(revoked.state(t.now()), InviteState::Revoked);
    assert!(
        t.write(move |tx| WorkspaceInvite::redeem(tx, &token, person("Revoked")))
            .is_none()
    );
    assert!(
        t.write(|tx| WorkspaceInvite::redeem(tx, "unknown", person("Unknown")))
            .is_none()
    );
    assert!(t.write(|tx| WorkspaceInvite::revoke(tx, -1)).is_none());
    assert_eq!(t.read(User::count), before);
}

#[test]
fn workspace_invite_concurrent_processes_cannot_both_redeem_the_last_use() {
    let t = TestDb::new();
    let second = t.another_process();
    let before = t.read(User::count);
    let (invite, token) =
        t.write(|tx| WorkspaceInvite::create(tx, id("david"), InviteExpiry::Never, Some(1)));
    let other = token.clone();
    let barrier = std::sync::Barrier::new(2);
    let (first, second) = std::thread::scope(|scope| {
        let first = scope.spawn(|| {
            barrier.wait();
            t.db.write_blocking(move |tx| WorkspaceInvite::redeem(tx, &token, person("First")))
        });
        let second = scope.spawn(|| {
            barrier.wait();
            second.write_blocking(move |tx| WorkspaceInvite::redeem(tx, &other, person("Second")))
        });
        (first.join().unwrap(), second.join().unwrap())
    });
    assert_eq!(
        usize::from(first.unwrap().is_some()) + usize::from(second.unwrap().is_some()),
        1
    );
    assert_eq!(t.read(User::count), before + 1);
    assert_eq!(
        t.read(move |c| WorkspaceInvite::find(c, invite.id))
            .unwrap()
            .uses,
        1
    );
}
