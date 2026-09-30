use super::channel_thread_test::frozen;
use super::*;
use crate::models::direct_room::{self, LeaveOutcome};
use crate::{Message, NewUser, Room, RoomType, User};
fn group(t: &TestDb) -> Room {
    t.write(|tx| {
        Room::find_or_create_direct_for(tx, &[id("david"), id("jason"), id("kevin")], id("david"))
    })
}
fn notes(t: &TestDb, room: i64) -> Vec<Message> {
    t.read(|c| {
        Ok(Message::for_room(c, room)?
            .into_iter()
            .filter(|m| m.system_note)
            .collect())
    })
}
fn rename(t: &TestDb, mut room: Room, name: &str) -> Room {
    let name = name.to_owned();
    t.write(move |tx| {
        room.rename_direct(tx, &name, id("david"))?;
        Room::find(tx.conn(), room.id)
    })
}

#[test]
fn direct_display_names_match_rails_vectors() {
    let t = frozen();
    let template = t.read(|c| User::find(c, id("david")));
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("ws8_direct_vectors.json")).unwrap();
    for row in cases.as_array().unwrap() {
        let members: Vec<User> = row["names"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, n)| {
                let mut user = template.clone();
                user.id = i as i64 + 1;
                user.name = n.as_str().unwrap().into();
                user
            })
            .collect();
        let viewer = row["viewer"].as_u64().map(|i| &members[i as usize]);
        assert_eq!(
            serde_json::to_value(direct_room::display_name(
                row["name"].as_str(),
                viewer,
                &members
            ))
            .unwrap(),
            row["display"],
            "{row}"
        );
    }
}
#[test]
fn review_direct_default_and_preloaded_order_match_rails() {
    let t = frozen();
    let cases: serde_json::Value = serde_json::from_str(include_str!("ws8_review_vectors.json")).unwrap();
    for row in cases["direct"].as_array().unwrap() {
        let names = row["names"].as_array().unwrap().clone();
        let (room, viewer) = t.write(move |tx| {
            let users = names.iter().map(|name| User::create(tx, NewUser {
                name: name.as_str().unwrap().into(), ..Default::default()
            })).collect::<Result<Vec<_>>>()?;
            let ids = users.iter().map(|u| u.id).collect::<Vec<_>>();
            Ok((Room::create_for(tx, RoomType::Direct, None, id("david"), &ids)?, users[0].clone()))
        });
        let actual = t.read(|conn| {
            let members = room.users(conn)?;
            Ok(serde_json::json!({
                "default": room.direct_display_name(conn, None, None)?,
                "preloaded": room.direct_display_name(conn, None, Some(&members))?,
                "viewer_default": room.direct_display_name(conn, Some(&viewer), None)?,
                "viewer_preloaded": room.direct_display_name(conn, Some(&viewer), Some(&members))?
            }))
        });
        for key in ["default", "preloaded", "viewer_default", "viewer_preloaded"] {
            assert_eq!(actual[key], row[key], "{row}");
        }
    }
}
#[test]
fn group_rename_validates_and_limits_notes_to_one_per_minute() {
    let t = frozen();
    let room = group(&t);
    let room = rename(&t, room, "  First  ");
    assert_eq!(room.name.as_deref(), Some("First"));
    assert_eq!(notes(&t, room.id).len(), 1);
    let room = rename(&t, room, "Second");
    assert_eq!(room.name.as_deref(), Some("Second"));
    assert_eq!(notes(&t, room.id).len(), 1);
    t.clock.travel(jiff::SignedDuration::from_secs(61));
    let room = rename(&t, room, "");
    assert!(room.name.is_none());
    assert_eq!(notes(&t, room.id).len(), 2);
    let mut pair = t.read(|c| Room::find(c, id("david_and_jason")));
    assert!(
        t.try_write(move |tx| pair.rename_direct(tx, "bad", id("david")))
            .is_err()
    );
    let mut room = room.clone();
    assert!(matches!(
        t.try_write(move |tx| room.rename_direct(tx, &"é".repeat(101), id("david"))),
        Err(crate::Error::RecordInvalid(_))
    ));
}
#[test]
fn direct_name_validation_applies_to_every_create_and_update_path() {
    let t = frozen();
    assert!(matches!(
        t.try_write(|tx| Room::create_for(
            tx,
            RoomType::Direct,
            Some(&"x".repeat(101)),
            id("david"),
            &[id("david")]
        )),
        Err(crate::Error::RecordInvalid(_))
    ));
    let mut room = group(&t);
    assert!(matches!(
        t.try_write(move |tx| room.update(tx, Some(Some(&"x".repeat(101))), None)),
        Err(crate::Error::RecordInvalid(_))
    ));
}
#[test]
fn adding_members_enforces_cap_skips_existing_and_refreshes_keys() {
    let t = frozen();
    let room = group(&t);
    let before = room.direct_member_key.clone();
    let copied = room.clone();
    assert_eq!(
        t.write(move |tx| copied.add_direct_members(tx, &[id("jz"), id("jason")], id("david"))),
        [id("jz")]
    );
    let updated = t.read(|c| Room::find(c, room.id));
    assert_ne!(updated.direct_member_key, before);
    let copied = room.clone();
    assert!(
        t.write(move |tx| copied.add_direct_members(tx, &[id("jz")], id("david")))
            .is_empty()
    );
    let extras: Vec<i64> = (0..7)
        .map(|n| {
            t.write(move |tx| {
                User::create(
                    tx,
                    NewUser {
                        name: format!("Extra {n}"),
                        ..Default::default()
                    },
                )
                .map(|u| u.id)
            })
        })
        .collect();
    let copied = room.clone();
    let six = extras[..6].to_vec();
    t.write(move |tx| copied.add_direct_members(tx, &six, id("david")));
    let copied = room.clone();
    let overflow = extras[6];
    assert!(
        t.try_write(move |tx| copied.add_direct_members(tx, &[overflow], id("david")))
            .is_err()
    );
    assert_eq!(t.read(|c| room.user_ids(c)).len(), 10);
}
#[test]
fn named_shrunken_groups_keep_history_and_cannot_squat_pair_keys() {
    let t = frozen();
    let room = rename(&t, group(&t), "Named");
    let copied = room.clone();
    assert_eq!(
        t.write(move |tx| copied.leave_direct(tx, id("kevin"))),
        LeaveOutcome::Left
    );
    assert_eq!(t.read(|c| room.user_ids(c)).len(), 2);
    let current = t.read(|c| Room::find(c, room.id));
    assert!(
        current
            .direct_member_key
            .unwrap()
            .ends_with(&format!("#{}", room.id))
    );
    let pair =
        t.write(|tx| Room::find_or_create_direct_for(tx, &[id("david"), id("jason")], id("david")));
    assert_ne!(pair.id, room.id);
    let copied = room.clone();
    t.write(move |tx| copied.leave_direct(tx, id("jason")));
    let copied = room.clone();
    assert_eq!(
        t.write(move |tx| copied.leave_direct(tx, id("david"))),
        LeaveOutcome::Destroyed
    );
    assert!(t.read(|c| Room::find(c, room.id)).deleted());
}
#[test]
fn notes_are_literal_quiet_and_directory_events_are_per_member() {
    let t = frozen();
    let from = t.events().len();
    let room = rename(&t, group(&t), "<img src=x>");
    let note = notes(&t, room.id).pop().unwrap();
    assert!(t.read(|c| note.body_html(c)).unwrap().contains("&lt;img"));
    assert!(
        !t.events()[from..]
            .iter()
            .any(|e| matches!(e, crate::Event::PushMessage { .. } | crate::Event::Job(_)))
    );
    assert!(
        t.read(|c| room.memberships(c))
            .iter()
            .all(|m| m.unread_at.is_none())
    );
    let source = t.events().len();
    let copy = room.clone();
    t.write(move |tx| copy.add_direct_members(tx, &[id("jz")], id("david")));
    let broadcast_count = t.events()[source..]
        .iter()
        .filter(|e| e.as_broadcast().is_some())
        .count();
    assert_eq!(broadcast_count, 9);
}
