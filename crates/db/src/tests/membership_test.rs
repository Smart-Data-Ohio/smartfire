//! `test/models/membership_test.rb`

use super::*;
use crate::Membership;
use rusqlite::params;

const TTL_PLUS_ONE: i64 = 61;

fn membership(t: &TestDb) -> Membership {
    t.read(|c| Membership::find(c, id("david_watercooler")))
}

/// Runs a Connectable method and returns the updated in-memory membership.
fn run(
    t: &TestDb,
    membership: Membership,
    f: fn(&mut Membership, &mut Tx<'_>) -> Result<()>,
) -> Membership {
    t.write(move |tx| {
        let mut m = membership;
        f(&mut m, tx)?;
        Ok(m)
    })
}

fn connected(t: &TestDb, m: Membership) -> Membership {
    run(t, m, |m, tx| m.connected(tx))
}

fn disconnected(t: &TestDb, m: Membership) -> Membership {
    run(t, m, |m, tx| m.disconnected(tx))
}

fn connected_exists(t: &TestDb, m: &Membership) -> bool {
    let now = t.now();
    t.read(|c| Membership::connected_exists(c, m.id, now))
}

fn disconnected_exists(t: &TestDb, m: &Membership) -> bool {
    let now = t.now();
    t.read(|c| Membership::disconnected_exists(c, m.id, now))
}

#[test]
fn connected_scope() {
    let t = TestDb::new();
    let m = connected(&t, membership(&t));
    assert!(connected_exists(&t, &m));

    let m = disconnected(&t, m);
    assert!(!connected_exists(&t, &m));

    t.travel(TTL_PLUS_ONE);
    assert!(!connected_exists(&t, &m));
}

#[test]
fn disconnected_scope() {
    let t = TestDb::new();
    let m = disconnected(&t, membership(&t));
    assert!(disconnected_exists(&t, &m));

    let m = connected(&t, m);
    assert!(!disconnected_exists(&t, &m));

    t.travel(TTL_PLUS_ONE);
    assert!(disconnected_exists(&t, &m));
}

#[test]
fn connected_is_false_when_connection_is_stale() {
    let t = TestDb::new();
    let m = connected(&t, membership(&t));
    t.travel(TTL_PLUS_ONE);
    assert!(!m.is_connected(t.now()));
}

#[test]
fn connecting() {
    let t = TestDb::new();
    let m = connected(&t, membership(&t));
    assert!(m.is_connected(t.now()));
    assert_eq!(m.connections, 1);

    let m = connected(&t, m);
    assert_eq!(m.connections, 2);
    assert_eq!(t.read(|c| Membership::find(c, m.id)).connections, 2);
}

#[test]
fn connecting_resets_stale_connection_count() {
    let t = TestDb::new();
    let m = connected(&t, connected(&t, membership(&t)));
    assert_eq!(m.connections, 2);

    t.travel(TTL_PLUS_ONE);
    let m = connected(&t, m);
    assert_eq!(m.connections, 1);
}

#[test]
fn disconnecting() {
    let t = TestDb::new();
    let m = connected(&t, connected(&t, membership(&t)));

    let m = disconnected(&t, m);
    assert!(m.is_connected(t.now()));
    assert_eq!(m.connections, 1);

    let m = disconnected(&t, m);
    assert!(!m.is_connected(t.now()));
    assert_eq!(m.connections, 0);
    assert_eq!(t.read(|c| Membership::find(c, m.id)).connected_at, None);
}

#[test]
fn disconnecting_resets_stale_connection_count() {
    let t = TestDb::new();
    let m = connected(&t, connected(&t, membership(&t)));
    assert_eq!(m.connections, 2);

    t.travel(TTL_PLUS_ONE);
    let m = disconnected(&t, m);
    assert_eq!(m.connections, 0);
}

#[test]
fn refreshing_the_connection() {
    let t = TestDb::new();
    let m = connected(&t, membership(&t));

    t.travel(TTL_PLUS_ONE);
    assert!(!m.is_connected(t.now()));

    let m = run(&t, m, |m, tx| m.refresh_connection(tx));
    assert!(m.is_connected(t.now()));
}

#[test]
fn present_marks_read_and_counts_connections() {
    let t = TestDb::new();
    let m = t.write(|tx| {
        tx.conn().execute(
            "UPDATE memberships SET unread_at = '2026-01-01 00:00:00' WHERE id = ?",
            [id("david_watercooler")],
        )?;
        let mut m = Membership::find(tx.conn(), id("david_watercooler"))?;
        m.present(tx)?;
        Ok(m)
    });
    let reloaded = t.read(|c| Membership::find(c, m.id));
    assert_eq!(reloaded.connections, 1);
    assert_eq!(reloaded.unread_at, None);
    assert_eq!(
        reloaded.updated_at, m.updated_at,
        "Membership.connect doesn't touch updated_at"
    );
}

#[test]
fn disconnect_all_resets_connected_memberships() {
    let t = TestDb::new();
    let m = connected(&t, membership(&t));
    t.write(Membership::disconnect_all);
    let reloaded = t.read(|c| Membership::find(c, m.id));
    assert_eq!((reloaded.connections, reloaded.connected_at), (0, None));
}

#[test]
fn removing_a_membership_resets_the_users_connections() {
    let t = TestDb::new();
    let m = membership(&t);
    t.write(move |tx| m.destroy(tx));
    assert_eq!(
        t.events(),
        vec![
            Event::broadcast(&crate::models::activity_item::ActivityItemsRemoved {
                items: Vec::new(),
                recipient_ids: Some(vec![id("david")]),
                room_id: Some(id("watercooler")),
            }),
            Event::broadcast(&crate::RoomRemovalBroadcast {
                user_id: id("david"),
                room_id: id("watercooler"),
                room_class: "Rooms::Closed".into(),
            }),
            Event::DisconnectUser {
                user_id: id("david"),
                reconnect: true
            },
        ]
    );
}

/// `read` writes nothing once the member is read and the pointer is on the newest root message.
#[test]
fn reading_is_a_noop_when_already_read() {
    let t = TestDb::new();
    let m = run(&t, membership(&t), |m, tx| m.read(tx));
    let before = t.read(|c| Membership::find(c, m.id)).updated_at;
    t.travel(5);
    let m = run(&t, m, |m, tx| m.read(tx));
    assert_eq!(t.read(|c| Membership::find(c, m.id)).updated_at, before);
}

/// `membership_navigation_test.rb`: "read clears unread and points at the newest root message".
#[test]
fn read_clears_unread_and_points_at_the_newest_root_message() {
    let t = TestDb::new();
    let designers = crate::Timeline::Room(id("designers"));
    let messages = t.read(|c| crate::Message::first_page(c, designers));
    let (first, second) = (messages[0].clone(), messages[1].clone());
    let membership_id = id("david_designers");
    let (first_id, second_at) = (first.id, second.created_at);
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE memberships SET unread_at = ?, last_read_message_id = ? WHERE id = ?",
            rusqlite::params![second_at, first_id, membership_id],
        )?;
        Ok(())
    });
    let m = t.read(|c| Membership::find(c, membership_id));
    assert!(m.unread());
    let m = run(&t, m, |m, tx| m.read(tx));
    let newest = t
        .read(|c| crate::Message::last_page(c, designers))
        .last()
        .unwrap()
        .id;
    let stored = t.read(|c| Membership::find(c, m.id));
    assert!(!stored.unread());
    assert_eq!(stored.last_read_message_id, Some(newest));
    assert_eq!(m, stored);
}

/// The pointer is the newest *root* message: thread replies don't move it.
#[test]
fn latest_root_message_id_skips_thread_messages_and_breaks_ties_by_id() {
    let t = TestDb::new();
    t.clock.travel_to(t.now());
    let room_id = id("designers");
    let (root_a, root_b, reply) = t.write(move |tx| {
        let thread_id = super::message_test::create_thread(tx, room_id, id("david"))?;
        let root = |tx: &mut Tx<'_>| {
            crate::Message::create(
                tx,
                crate::NewMessage {
                    room_id,
                    creator_id: id("david"),
                    ..Default::default()
                },
            )
        };
        let root_a = root(tx)?;
        let root_b = root(tx)?;
        let reply = crate::Message::create(
            tx,
            crate::NewMessage {
                room_id,
                creator_id: id("david"),
                thread_id: Some(thread_id),
                ..Default::default()
            },
        )?;
        Ok((root_a, root_b, reply))
    });
    assert_eq!(root_a.created_at, reply.created_at, "same instant");
    assert!(reply.id > root_b.id && root_b.id > root_a.id);
    let m = t.read(|c| Membership::find(c, id("kevin_designers")));
    assert_eq!(t.read(|c| m.latest_root_message_id(c)), Some(root_b.id));
}

/// `Membership.connect` also moves the read pointer to the newest root message.
#[test]
fn present_moves_the_read_pointer() {
    let t = TestDb::new();
    let m = membership(&t);
    assert_eq!(m.last_read_message_id, None);
    let m = run(&t, m, |m, tx| m.present(tx));
    let newest = t.read(|c| m.latest_root_message_id(c));
    assert!(newest.is_some());
    assert_eq!(
        t.read(|c| Membership::find(c, m.id)).last_read_message_id,
        newest
    );
}

/// `enum :involvement` includes `muted`; `stage_role` reads back as its enum.
#[test]
fn muted_involvement_and_stage_role_round_trip() {
    let t = TestDb::new();
    let m = run(&t, membership(&t), |m, tx| {
        m.update_involvement(tx, crate::Involvement::Muted)
    });
    let stored = t.read(|c| Membership::find(c, m.id));
    assert_eq!(stored.involvement, Some(crate::Involvement::Muted));
    let raw: String = t.read(|c| {
        Ok(c.query_row(
            "SELECT involvement FROM memberships WHERE id = ?",
            [m.id],
            |r| r.get(0),
        )?)
    });
    assert_eq!(raw, "muted");
    assert_eq!(
        stored.stage_role, None,
        "non-stage rooms leave the stage columns nil"
    );
}

fn badge_with_membership(involvement: Option<&str>, deleted: bool) -> i64 {
    let t = TestDb::new();
    let viewer = id("david");
    let room = id("watercooler");
    let visible = !deleted && involvement.is_some_and(|level| level != "invisible");
    let involvement = involvement.map(str::to_owned);
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE memberships SET unread_at=NULL WHERE user_id=?",
            [viewer],
        )?;
        tx.conn().execute(
            "UPDATE memberships SET involvement=?1,unread_at=?2 WHERE user_id=?3 AND room_id=?4",
            params![involvement, tx.now(), viewer, room],
        )?;
        if deleted {
            tx.conn().execute(
                "UPDATE rooms SET deleted_at=? WHERE id=?",
                params![tx.now(), room],
            )?;
        }
        Ok(())
    });
    t.read(|conn| {
        assert_eq!(
            Membership::visible_with_ordered_room(conn, viewer)?
                .iter()
                .any(|(membership, row)| row.id == room && membership.unread()),
            visible,
        );
        crate::PushSubscription::for_user(conn, viewer)?[0].badge(conn, t.now())
    })
}

#[test]
fn app_badge_excludes_invisible_memberships() {
    assert_eq!(badge_with_membership(Some("invisible"), false), 0);
}

#[test]
fn app_badge_excludes_null_involvement_memberships() {
    assert_eq!(badge_with_membership(None, false), 0);
}

#[test]
fn app_badge_excludes_deleted_rooms() {
    assert_eq!(badge_with_membership(Some("mentions"), true), 0);
}

#[test]
fn app_badge_keeps_visible_unread_memberships() {
    for involvement in ["nothing", "muted", "mentions", "everything"] {
        assert_eq!(
            badge_with_membership(Some(involvement), false),
            1,
            "{involvement}"
        );
    }
}

#[test]
fn app_badge_counts_notifying_threads_in_read_rooms_once() {
    use crate::{ChannelThread, ThreadInvolvement, ThreadMembership};
    let t = TestDb::new();
    let viewer = id("jason");
    let room = id("designers");
    let thread = channel_thread_test::create_thread(&t, "designers", "david", None, Some("Badge"));
    let thread_id = thread.id;
    t.write(move |tx| {
        let mut membership = ThreadMembership::join(tx, thread_id, viewer)?;
        membership.update_involvement(tx, ThreadInvolvement::Everything)?;
        membership.read(tx)?;
        Ok(())
    });
    channel_thread_test::post_reply(&t, thread_id, "david", "First reply");
    channel_thread_test::post_reply(&t, thread_id, "david", "Second reply");
    t.write(move |tx| {
        for mut membership in Membership::for_user(tx.conn(), viewer)? {
            membership.read(tx)?;
        }
        Ok(())
    });
    t.read(|conn| {
        assert!(ChannelThread::find(conn, thread_id)?.unread_for(conn, viewer)?);
        assert!(
            !Membership::find_by_room_and_user(conn, room, viewer)?
                .unwrap()
                .unread()
        );
        assert_eq!(Membership::unread_count(conn, viewer, t.now())?, 1);
        Ok(())
    });
    t.write(move |tx| {
        ThreadMembership::find_by_thread_and_user(tx.conn(), thread_id, viewer)?
            .unwrap()
            .read(tx)
    });
    t.read(|conn| {
        assert_eq!(Membership::unread_count(conn, viewer, t.now())?, 0);
        Ok(())
    });
}

#[test]
fn app_badge_keeps_each_visible_room_kind_and_drops_left_rooms() {
    let t = TestDb::new();
    let viewer = id("david");
    let room = id("watercooler");
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE memberships SET unread_at=NULL WHERE user_id=?",
            [viewer],
        )?;
        tx.conn().execute(
            "UPDATE memberships SET unread_at=? WHERE room_id=? AND user_id=?",
            params![tx.now(), room, viewer],
        )?;
        Ok(())
    });
    for kind in [
        crate::RoomType::Open,
        crate::RoomType::Closed,
        crate::RoomType::Direct,
        crate::RoomType::Board,
        crate::RoomType::Voice,
        crate::RoomType::Stage,
    ] {
        t.write(move |tx| {
            tx.conn()
                .execute("UPDATE rooms SET type=? WHERE id=?", params![kind, room])?;
            Ok(())
        });
        t.read(|conn| {
            assert_eq!(
                Membership::unread_count(conn, viewer, t.now())?,
                1,
                "{kind:?}"
            );
            Ok(())
        });
    }
    t.write(move |tx| {
        Membership::find_by_room_and_user(tx.conn(), room, viewer)?
            .unwrap()
            .destroy(tx)
    });
    t.read(|conn| {
        assert_eq!(Membership::unread_count(conn, viewer, t.now())?, 0);
        Ok(())
    });
}

#[test]
fn app_badge_excludes_memberships_whose_room_is_missing() {
    let t = TestDb::new();
    let viewer = id("david");
    t.write(move |tx| {
        tx.conn().execute("UPDATE memberships SET unread_at=NULL WHERE user_id=?", [viewer])?;
        // The Rails-era table has no room foreign key, so old orphaned memberships can remain.
        tx.conn().execute(
            "INSERT INTO memberships(room_id,user_id,involvement,unread_at,created_at,updated_at) VALUES(-1,?1,'everything',?2,?2,?2)",
            params![viewer, tx.now()],
        )?;
        Ok(())
    });
    t.read(|conn| {
        assert!(
            !Membership::visible_with_ordered_room(conn, viewer)?
                .iter()
                .any(|(_, room)| room.id == -1)
        );
        assert_eq!(
            crate::PushSubscription::for_user(conn, viewer)?[0].badge(conn, t.now())?,
            0
        );
        Ok(())
    });
}

#[test]
fn app_badge_respects_timed_preference_mute_until_expiry() {
    let t = TestDb::new();
    let viewer = id("david");
    let room = id("watercooler");
    let until = t.now().jiff() + jiff::SignedDuration::from_secs(60);
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE memberships SET unread_at=NULL WHERE user_id=?",
            [viewer],
        )?;
        tx.conn().execute(
            "UPDATE memberships SET unread_at=? WHERE user_id=? AND room_id=?",
            params![tx.now(), viewer, room],
        )?;
        tx.conn().execute(
            "UPDATE users SET inbox_preferences=? WHERE id=?",
            params![
                serde_json::json!({"room_mute_until": {room.to_string(): until.to_string()}})
                    .to_string(),
                viewer
            ],
        )?;
        Ok(())
    });
    t.read(|conn| {
        assert_eq!(Membership::unread_count(conn, viewer, t.now())?, 0);
        Ok(())
    });
    t.travel(60);
    t.read(|conn| {
        assert_eq!(Membership::unread_count(conn, viewer, t.now())?, 1);
        Ok(())
    });
}
