//! `test/models/room_test.rb`, `rooms/direct_test.rb`, `rooms/open_test.rb`

use super::*;
use crate::{Involvement, Membership, Message, Room, RoomType, StageRole, User};

fn member_ids(t: &TestDb, room_id: i64) -> Vec<i64> {
    t.read(|c| Room::find(c, room_id)?.user_ids(c))
}

fn room(t: &TestDb, label: &str) -> Room {
    let room_id = id(label);
    t.read(|c| Room::find(c, room_id))
}

#[test]
fn grant_membership_to_user() {
    let t = TestDb::new();
    let watercooler = room(&t, "watercooler");
    t.write(move |tx| watercooler.grant_to(tx, &[id("kevin")]));
    assert!(member_ids(&t, id("watercooler")).contains(&id("kevin")));
}

#[test]
fn revoke_membership_from_user() {
    let t = TestDb::new();
    let watercooler = room(&t, "watercooler");
    t.write(move |tx| watercooler.revoke_from(tx, &[id("david")]));
    assert!(!member_ids(&t, id("watercooler")).contains(&id("david")));
    assert_eq!(
        t.events(),
        vec![
            Event::broadcast(&crate::RoomRemovalBroadcast {
                user_id: id("david"),
                room_id: id("watercooler"),
                room_class: "Rooms::Closed".into(),
            }),
            Event::DisconnectUser { user_id: id("david"), reconnect: true },
        ]
    );
}

#[test]
fn revise_memberships() {
    let t = TestDb::new();
    let watercooler = room(&t, "watercooler");
    t.write(move |tx| watercooler.revise(tx, &[id("kevin")], &[id("david")]));
    let members = member_ids(&t, id("watercooler"));
    assert!(members.contains(&id("kevin")));
    assert!(!members.contains(&id("david")));
}

#[test]
fn create_for_users_by_giving_them_immediate_membership() {
    let t = TestDb::new();
    let room = t.write(|tx| {
        Room::create_for(
            tx,
            RoomType::Closed,
            Some("Hello!"),
            id("david"),
            &[id("kevin"), id("david")],
        )
    });
    let members = member_ids(&t, room.id);
    assert!(members.contains(&id("kevin")) && members.contains(&id("david")));
}

#[test]
fn type_predicates() {
    let t = TestDb::new();
    assert!(room(&t, "pets").open());
    assert!(!room(&t, "pets").direct());
    assert!(room(&t, "david_and_jason").direct());
    assert!(room(&t, "designers").closed());
}

#[test]
fn default_involvement_for_new_users() {
    let t = TestDb::new();
    let room = t.write(|tx| {
        Room::create_for(
            tx,
            RoomType::Closed,
            Some("Hello!"),
            id("david"),
            &[id("kevin"), id("david")],
        )
    });
    let memberships = t.read(|c| room.memberships(c));
    assert_eq!(memberships.len(), 2);
    assert!(
        memberships
            .iter()
            .all(|m| m.involved_in(Involvement::Mentions))
    );
}

#[test]
fn granted_memberships_are_stamped_by_sqlite() {
    // insert_all lets SQLite fill the timestamps: STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW').
    let t = TestDb::new();
    let room = t.write(|tx| {
        Room::create_for(
            tx,
            RoomType::Closed,
            Some("Hello!"),
            id("david"),
            &[id("kevin")],
        )
    });
    let created_at: String = t.read(|c| {
        Ok(c.query_row(
            "SELECT created_at FROM memberships WHERE room_id = ?",
            [room.id],
            |r| r.get(0),
        )?)
    });
    assert_eq!(
        created_at.len(),
        "2026-09-26 12:25:26.826".len(),
        "{created_at}"
    );
}

#[test]
fn direct_rooms_keep_their_type() {
    let t = TestDb::new();
    let mut direct = room(&t, "david_and_jason");
    let result = t.try_write(move |tx| direct.update(tx, None, Some(RoomType::Open)));
    match result {
        Err(crate::Error::RecordInvalid(errors)) => {
            assert_eq!(errors.on("type"), ["can't be changed for a direct room"])
        }
        other => panic!("expected invalid, got {other:?}"),
    }
}

#[test]
fn destroying_a_room_destroys_its_messages_and_memberships() {
    let t = TestDb::new();
    let watercooler = room(&t, "watercooler");
    t.write(move |tx| watercooler.destroy(tx));
    assert!(
        t.read(|c| Message::for_room(c, id("watercooler")))
            .is_empty()
    );
    assert!(
        t.read(|c| Membership::for_room(c, id("watercooler")))
            .is_empty()
    );
    assert!(t.read(|c| Room::find_by_id(c, id("watercooler"))).is_none());
    assert_eq!(
        t.read(|c| crate::sql::count(
            c,
            "SELECT COUNT(*) FROM boosts WHERE message_id IN (?, ?)",
            [id("thirteenth"), id("fourth")]
        )),
        0
    );
}

// Rooms::Direct

#[test]
fn create_direct_room_for_same_users() {
    let t = TestDb::new();
    let room =
        t.write(|tx| Room::find_or_create_direct_for(tx, &[id("jz"), id("kevin")], id("jz")));
    let members = member_ids(&t, room.id);
    assert!(members.contains(&id("jz")) && members.contains(&id("kevin")));
    assert!(!members.contains(&id("jason")));
}

#[test]
fn only_one_direct_room_will_exist_for_the_same_users() {
    let t = TestDb::new();
    let room1 =
        t.write(|tx| Room::find_or_create_direct_for(tx, &[id("jz"), id("kevin")], id("jz")));
    let room2 =
        t.write(|tx| Room::find_or_create_direct_for(tx, &[id("kevin"), id("jz")], id("kevin")));
    assert_eq!(room1.id, room2.id);

    let existing =
        t.write(|tx| Room::find_or_create_direct_for(tx, &[id("david"), id("kevin")], id("david")));
    assert_eq!(existing.id, id("david_and_kevin"));
}

#[test]
fn direct_default_involvement_for_new_users() {
    let t = TestDb::new();
    let room =
        t.write(|tx| Room::find_or_create_direct_for(tx, &[id("jz"), id("kevin")], id("jz")));
    assert!(
        t.read(|c| room.memberships(c))
            .iter()
            .all(|m| m.involved_in(Involvement::Everything))
    );
}

// Rooms::Open

#[test]
fn open_room_grants_access_to_all_users_after_creation() {
    let t = TestDb::new();
    let room = t.write(|tx| {
        Room::create(
            tx,
            RoomType::Open,
            Some("My open room with everyone!"),
            id("david"),
        )
    });
    assert_eq!(member_ids(&t, room.id).len() as i64, t.read(User::count));
}

#[test]
fn open_room_grants_access_to_all_users_after_becoming_open() {
    let t = TestDb::new();
    let mut watercooler = room(&t, "watercooler");
    t.write(move |tx| watercooler.update(tx, None, Some(RoomType::Open)));
    assert_eq!(
        member_ids(&t, id("watercooler")).len() as i64,
        t.read(User::count)
    );
    assert_eq!(room(&t, "watercooler").room_type, RoomType::Open);
    let stored: String = t.read(|c| {
        Ok(c.query_row(
            "SELECT type FROM rooms WHERE id = ?",
            [id("watercooler")],
            |r| r.get(0),
        )?)
    });
    assert_eq!(stored, "Rooms::Open");
}

#[test]
fn user_room_scopes() {
    let t = TestDb::new();
    let david = id("david");
    assert_eq!(
        t.read(|c| Room::for_user_of_type(c, david, RoomType::Direct))
            .len(),
        2
    );
    assert_eq!(
        t.read(|c| Room::for_user_without_directs(c, david)).len(),
        4
    );
    assert!(
        t.read(|c| Room::find_for_user(c, id("kevin"), id("pets")))
            .is_none()
    );
    let ordered = t.read(|c| Membership::visible_with_ordered_room(c, david));
    let names: Vec<_> = ordered.iter().map(|(_, r)| r.name.clone()).collect();
    assert_eq!(
        names[names.len() - 4..],
        [
            Some("All Pets".into()),
            Some("All Talk".into()),
            Some("Designers".into()),
            Some("HQ".into())
        ]
    );
}

// Our room types and keys

fn stored_key(t: &TestDb, room_id: i64) -> Option<String> {
    t.read(|c| Ok(Room::find(c, room_id)?.direct_member_key))
}

fn soft_delete(t: &TestDb, room_id: i64) {
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE rooms SET deleted_at = ?, direct_member_key = NULL WHERE id = ?",
            rusqlite::params![tx.now(), room_id],
        )?;
        tx.conn().execute("DELETE FROM memberships WHERE room_id = ?", [room_id])?;
        Ok(())
    });
}

/// `Rooms::Voice`, `Rooms::Stage` and `Rooms::Board` read and write their STI class, answer
/// their predicates, default to "mentions", and are listed without the directs.
#[test]
fn voice_stage_and_board_rooms() {
    let t = TestDb::new();
    for (room_type, class_name) in [
        (RoomType::Voice, "Rooms::Voice"),
        (RoomType::Stage, "Rooms::Stage"),
        (RoomType::Board, "Rooms::Board"),
    ] {
        let room = t.write(move |tx| Room::create_for(tx, room_type, Some("Room"), id("david"), &[id("david"), id("jason")]));
        assert_eq!(room.room_type, room_type);
        let stored: String = t.read(|c| Ok(c.query_row("SELECT type FROM rooms WHERE id = ?", [room.id], |r| r.get(0))?));
        assert_eq!(stored, class_name);
        assert_eq!(RoomType::from_class_name(class_name), Some(room_type));
        assert_eq!((room.voice(), room.stage(), room.board()), (room_type == RoomType::Voice, room_type == RoomType::Stage, room_type == RoomType::Board));
        assert!(!room.open() && !room.closed() && !room.direct());
        assert!(t.read(|c| room.memberships(c)).iter().all(|m| m.involved_in(Involvement::Mentions)));
        assert!(t.read(|c| Room::for_user_without_directs(c, id("david"))).iter().any(|r| r.id == room.id));
        assert_eq!(t.read(|c| Room::of_type(c, room_type)).len(), 1);
    }
}

/// `Rooms::Stage.create_for`: the creator becomes host, every other member a listener, even when
/// the creator wasn't in the member list; other rooms leave `stage_role` nil.
#[test]
fn stage_creator_becomes_host_and_members_become_listeners() {
    let t = TestDb::new();
    let role = |t: &TestDb, room_id: i64, user: &str| {
        let user_id = id(user);
        t.read(|c| Ok(Membership::find_by_room_and_user(c, room_id, user_id)?.unwrap().stage_role))
    };
    let stage = t.write(|tx| Room::create_for(tx, RoomType::Stage, Some("Town Hall"), id("david"), &[id("david"), id("jason")]));
    assert_eq!(role(&t, stage.id, "david"), Some(StageRole::Host));
    assert_eq!(role(&t, stage.id, "jason"), Some(StageRole::Listener));

    let stage = t.write(|tx| Room::create_for(tx, RoomType::Stage, Some("Town Hall"), id("david"), &[id("jason")]));
    assert_eq!(role(&t, stage.id, "david"), Some(StageRole::Host));
    assert_eq!(role(&t, stage.id, "jason"), Some(StageRole::Listener));
    let raw: Vec<String> = t.read(|c| {
        crate::sql::query_all(c, "SELECT stage_role FROM memberships WHERE room_id = ? ORDER BY user_id", [stage.id], |r| r.get(0))
    });
    assert!(raw.iter().all(|r| r == "host" || r == "listener"), "{raw:?}");

    let voice = t.write(|tx| Room::create_for(tx, RoomType::Voice, Some("Lounge"), id("david"), &[id("david")]));
    assert_eq!(role(&t, voice.id, "david"), None);
}

/// Stage hosts are revoked last.
#[test]
fn revoking_puts_stage_hosts_last() {
    let t = TestDb::new();
    let stage = t.write(|tx| Room::create_for(tx, RoomType::Stage, Some("Town Hall"), id("david"), &[id("david"), id("jason"), id("kevin")]));
    t.write(move |tx| stage.revoke_from(tx, &[id("david"), id("jason"), id("kevin")]));
    let order: Vec<i64> = t.events().into_iter().filter_map(|e| match e {
        Event::DisconnectUser { user_id, .. } => Some(user_id),
        _ => None,
    }).collect();
    assert_eq!(order.last(), Some(&id("david")), "{order:?}");
}

/// `Rooms::Direct.member_key_for` against vectors from our Rails.
#[test]
fn direct_member_key_matches_rails() {
    assert_eq!(Room::direct_member_key_for(&[3, 1, 2]), "dm:8a6ae15122001229edb8866f56e342af12ae8187203c3e3b33931743e7c0c48d");
    assert_eq!(Room::direct_member_key_for(&[10, 9]), "dm:69c37d73f38de14c86035211434b780645c046f331a70bb6da9db9ba15400a7f");
    assert_eq!(Room::direct_member_key_for(&[]), "dm:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
}

/// "created rooms carry the hash of their exact member set"
#[test]
fn created_direct_rooms_carry_the_hash_of_their_exact_member_set() {
    let t = TestDb::new();
    let room = t.write(|tx| Room::find_or_create_direct_for(tx, &[id("jz"), id("david")], id("david")));
    assert_eq!(room.direct_member_key, Some(Room::direct_member_key_for(&[id("david"), id("jz")])));
    assert_eq!(Room::direct_member_key_for(&[id("david"), id("jz")]), Room::direct_member_key_for(&[id("jz"), id("david")]));
    assert_eq!(t.read(|c| Room::find_direct_for(c, &[id("david"), id("jz")])).map(|r| r.id), Some(room.id));
}

/// "lookup still finds rooms created before the member key"
#[test]
fn direct_lookup_still_finds_rooms_created_before_the_member_key() {
    let t = TestDb::new();
    assert_eq!(stored_key(&t, id("david_and_kevin")), None);
    for ids in [[id("david"), id("kevin")], [id("kevin"), id("david")]] {
        assert_eq!(t.read(|c| Room::find_direct_for(c, &ids)).map(|r| r.id), Some(id("david_and_kevin")));
    }
}

/// The partial unique index admits one alive room per member key, so two concurrent creates
/// can't both win; soft-deleted rooms don't hold their key.
#[test]
fn direct_member_keys_are_unique_among_alive_rooms() {
    let t = TestDb::new();
    let room = t.write(|tx| Room::find_or_create_direct_for(tx, &[id("jz"), id("kevin")], id("jz")));
    let key = room.direct_member_key.clone().unwrap();

    let duplicate = t.try_write({
        let key = key.clone();
        move |tx| {
            let now = tx.now();
            tx.conn().execute(
                "INSERT INTO rooms (created_at, creator_id, direct_member_key, type, updated_at) VALUES (?, ?, ?, 'Rooms::Direct', ?)",
                rusqlite::params![now, id("kevin"), key, now],
            )?;
            Ok(())
        }
    });
    assert!(duplicate.as_ref().is_err_and(crate::Error::is_record_not_unique), "{duplicate:?}");

    // The same member set again is the same room, however the ids are ordered.
    let again = t.write(|tx| Room::find_or_create_direct_for(tx, &[id("kevin"), id("jz")], id("kevin")));
    assert_eq!(again.id, room.id);

    // "a deleted room never blocks recreating its member set"
    soft_delete(&t, room.id);
    let fresh = t.write(|tx| Room::find_or_create_direct_for(tx, &[id("kevin"), id("jz")], id("kevin")));
    assert_ne!(fresh.id, room.id);
    assert_eq!(fresh.direct_member_key, Some(key));
}

/// "adding or removing a member changes the member key": `grant_to` refreshes inline, a
/// membership's destroy after commit; neither touches `updated_at`.
#[test]
fn adding_or_removing_a_direct_member_changes_the_member_key() {
    let t = TestDb::new();
    let trio = [id("david"), id("jason"), id("kevin")];
    let room = t.write(move |tx| Room::find_or_create_direct_for(tx, &trio, id("david")));
    let before = room.direct_member_key.clone();
    assert_eq!(before, Some(Room::direct_member_key_for(&trio)));

    let r = room.clone();
    t.write(move |tx| r.grant_to(tx, &[id("jz")]));
    let quartet = [id("david"), id("jason"), id("kevin"), id("jz")];
    assert_eq!(stored_key(&t, room.id), Some(Room::direct_member_key_for(&quartet)));

    let r = room.clone();
    t.write(move |tx| r.revoke_from(tx, &[id("jz")]));
    assert_eq!(stored_key(&t, room.id), before);
    assert_eq!(t.read(|c| Room::find(c, room.id)).updated_at, room.updated_at, "update_column");
}

/// "a mutated group keeps its own room when its set collides with another group"
#[test]
fn a_mutated_direct_group_keeps_its_own_room_when_its_set_collides() {
    let t = TestDb::new();
    let trio = [id("david"), id("jason"), id("kevin")];
    let reused = t.write(move |tx| Room::find_or_create_direct_for(tx, &trio, id("david")));
    let other = t.write(|tx| Room::find_or_create_direct_for(tx, &[id("david"), id("jason"), id("kevin"), id("jz")], id("david")));
    let o = other.clone();
    t.write(move |tx| o.revoke_from(tx, &[id("jz")]));

    let key = stored_key(&t, other.id).unwrap();
    assert_eq!(key, format!("{}#{}", Room::direct_member_key_for(&trio), other.id));
    assert_ne!(Some(key), reused.direct_member_key);
    assert_eq!(t.write(move |tx| Room::find_or_create_direct_for(tx, &trio, id("david"))).id, reused.id);
}

/// "a named group that shrank to two keeps a suffixed key, so Message opens a fresh one-to-one"
#[test]
fn a_named_direct_group_that_shrank_to_two_keeps_a_suffixed_key() {
    let t = TestDb::new();
    let group = t.write(|tx| Room::find_or_create_direct_for(tx, &[id("david"), id("jason"), id("kevin")], id("david")));
    let group_id = group.id;
    t.write(move |tx| {
        tx.conn().execute("UPDATE rooms SET name = 'Weekend Plans' WHERE id = ?", [group_id])?;
        Ok(())
    });
    let g = t.read(|c| Room::find(c, group_id));
    t.write(move |tx| g.revoke_from(tx, &[id("david")]));
    let pair = [id("jason"), id("kevin")];
    assert_eq!(stored_key(&t, group_id), Some(format!("{}#{group_id}", Room::direct_member_key_for(&pair))));

    let fresh = t.write(move |tx| Room::find_or_create_direct_for(tx, &pair, id("jason")));
    assert_ne!(fresh.id, group_id);
    assert_eq!(fresh.direct_member_key, Some(Room::direct_member_key_for(&pair)));
}

// Alive rooms

/// Soft-deleted rooms grant nothing: `user.rooms` and its scopes skip them.
#[test]
fn deleted_rooms_are_gone_from_user_rooms() {
    let t = TestDb::new();
    let david = id("david");
    let watercooler = id("watercooler");
    assert!(t.read(|c| Room::find_for_user(c, david, watercooler)).is_some());
    t.write(move |tx| {
        tx.conn().execute("UPDATE rooms SET deleted_at = ? WHERE id = ?", rusqlite::params![tx.now(), watercooler])?;
        Ok(())
    });
    assert!(t.read(|c| Room::find_for_user(c, david, watercooler)).is_none());
    assert!(!t.read(|c| Room::for_user(c, david)).iter().any(|r| r.id == watercooler));
    assert!(!t.read(|c| Room::for_user_without_directs(c, david)).iter().any(|r| r.id == watercooler));
    assert!(t.read(|c| Room::find(c, watercooler)).deleted());
}

/// New users join alive open rooms only; a deleted room turned open grants nobody.
#[test]
fn deleted_open_rooms_grant_nobody() {
    let t = TestDb::new();
    let pets = id("pets");
    let watercooler = id("watercooler");
    t.write(move |tx| {
        tx.conn().execute("UPDATE rooms SET deleted_at = ? WHERE id IN (?, ?)", rusqlite::params![tx.now(), pets, watercooler])?;
        Ok(())
    });
    let user = t.write(|tx| User::create(tx, crate::NewUser { name: "New".into(), email_address: Some("new@example.com".into()), ..Default::default() }));
    let rooms: Vec<i64> = t.read(|c| crate::sql::query_all(c, "SELECT room_id FROM memberships WHERE user_id = ?", [user.id], |r| r.get(0)));
    assert!(!rooms.contains(&pets));
    assert!(rooms.contains(&id("hq")), "alive open rooms are still granted: {rooms:?}");

    let mut deleted = t.read(|c| Room::find(c, watercooler));
    let members_before = member_ids(&t, watercooler).len();
    t.write(move |tx| deleted.update(tx, None, Some(RoomType::Open)));
    assert_eq!(member_ids(&t, watercooler).len(), members_before);
}

// Room#receive

fn post(t: &TestDb, room: &str, creator: &str, body: &str) -> Message {
    let attributes = crate::NewMessage { room_id: id(room), creator_id: id(creator), body: Some(body.into()), ..Default::default() };
    t.write(move |tx| Message::create(tx, attributes))
}

fn membership(t: &TestDb, label: &str) -> Membership {
    let membership_id = id(label);
    t.read(|c| Membership::find(c, membership_id))
}

fn set_involvement(t: &TestDb, label: &str, involvement: Involvement) {
    let m = membership(t, label);
    t.write(move |tx| m.clone().update_involvement(tx, involvement));
}

/// "Muted rooms go unread only when the member is mentioned."
#[test]
fn muted_members_go_unread_only_when_mentioned() {
    let t = TestDb::new();
    set_involvement(&t, "kevin_designers", Involvement::Muted);
    t.travel(1);
    post(&t, "designers", "david", "Hello");
    assert!(!membership(&t, "kevin_designers").unread());
    assert!(membership(&t, "jason_designers").unread(), "unmuted members go unread");

    t.travel(1);
    let body = format!("Hey {}", crate::rich_text::mention_attachment_for(id("kevin")));
    let message = post(&t, "designers", "david", &body);
    let kevin_designers = membership(&t, "kevin_designers");
    assert_eq!(kevin_designers.unread_at, Some(message.created_at));
}

/// Members watching live, and the author, have their read pointer advanced to the new message;
/// disconnected members keep their boundary and go unread.
#[test]
fn receiving_advances_the_read_pointer_of_live_members_and_the_author() {
    let t = TestDb::new();
    let live = membership(&t, "jason_designers");
    t.write(move |tx| live.clone().present(tx));
    let message = post(&t, "designers", "david", "Hello");

    assert_eq!(membership(&t, "david_designers").last_read_message_id, Some(message.id), "the author");
    let jason = membership(&t, "jason_designers");
    assert!(!jason.unread());
    assert_eq!(jason.last_read_message_id, Some(message.id), "watching live");
    let kevin = membership(&t, "kevin_designers");
    assert!(kevin.unread());
    assert_ne!(kevin.last_read_message_id, Some(message.id), "disconnected keeps its boundary");
}
