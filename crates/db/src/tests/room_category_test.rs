//! Model-side scenarios from room_categories / categories / favorites controller tests.
//! HTTP ownership and room-type restrictions are WS8b's.
use super::channel_thread_test::frozen;
use super::*;
use crate::{Error, Membership, RoomCategory};

fn member(t: &TestDb, room: &str) -> Membership {
    t.read(|c| Membership::find_by_room_and_user(c, id(room), id("david")))
        .unwrap()
}
fn category(t: &TestDb, user: &str, name: &str, position: i64) -> RoomCategory {
    let (user, name) = (id(user), name.to_string());
    t.write(move |tx| RoomCategory::create(tx, user, &name, position, false))
}

#[test]
fn categories_are_per_user_and_order_by_position_then_id() {
    let t = frozen();
    let second = category(&t, "david", "Second", 2);
    let first = category(&t, "david", "First", 1);
    let tied = category(&t, "david", "Tied", 1);
    category(&t, "jason", "Theirs", 0);
    assert_eq!(
        t.read(|c| RoomCategory::ordered_for_user(c, id("david"))),
        [first, tied, second]
    );
}

#[test]
fn categories_append_at_the_end_and_update_their_own_timestamp() {
    let t = frozen();
    assert_eq!(
        t.read(|c| RoomCategory::next_position_for(c, id("david"))),
        1
    );
    category(&t, "david", "First", 1);
    let mut appended = t.write(|tx| {
        RoomCategory::create(
            tx,
            id("david"),
            "Team",
            RoomCategory::next_position_for(tx.conn(), id("david"))?,
            false,
        )
    });
    assert_eq!(appended.position, 2);
    let original = appended.clone();
    t.travel(60);
    let updated = t.write(move |tx| {
        appended.update(tx, "Squad", 2, true)?;
        Ok(appended)
    });
    assert_eq!(updated.name, "Squad");
    assert!(updated.collapsed);
    assert_eq!(updated.created_at, original.created_at);
    assert_eq!(updated.updated_at, t.now());
}

#[test]
fn categories_validate_required_user_name_and_character_limit() {
    let t = frozen();
    for (user, name) in [
        (id("david"), " ".into()),
        (id("david"), "é".repeat(51)),
        (0, "Team".into()),
    ] {
        assert!(matches!(
            t.try_write(move |tx| RoomCategory::create(tx, user, &name, 0, false)),
            Err(Error::RecordInvalid(_))
        ));
    }
    assert_eq!(
        category(&t, "david", &"é".repeat(50), 0)
            .name
            .chars()
            .count(),
        50
    );
}

#[test]
fn assigning_and_unassigning_categories_validates_the_member_owner() {
    let t = frozen();
    let own = category(&t, "david", "Team", 1);
    let other = category(&t, "jason", "Theirs", 1);
    let mut membership = member(&t, "designers");
    membership = t.write(move |tx| {
        membership.update_category(tx, Some(own.id))?;
        Ok(membership)
    });
    assert_eq!(membership.room_category_id, Some(own.id));
    let mut failed = membership.clone();
    let Error::RecordInvalid(errors) = t
        .try_write(move |tx| failed.update_category(tx, Some(other.id)))
        .unwrap_err()
    else {
        panic!()
    };
    assert_eq!(errors.on("room_category"), ["must belong to the member"]);
    t.write(move |tx| membership.update_category(tx, None));
    assert_eq!(member(&t, "designers").room_category_id, None);
}

#[test]
fn deleting_a_category_nullifies_memberships_without_touching_them() {
    let t = frozen();
    let category = category(&t, "david", "Team", 1);
    let mut membership = member(&t, "designers");
    t.write(move |tx| membership.update_category(tx, Some(category.id)));
    let before = member(&t, "designers").updated_at;
    t.travel(60);
    t.write(move |tx| category.destroy(tx));
    let membership = member(&t, "designers");
    assert_eq!(membership.room_category_id, None);
    assert_eq!(membership.updated_at, before);
    assert!(
        t.read(|c| RoomCategory::ordered_for_user(c, id("david")))
            .is_empty()
    );
}

#[test]
fn favorites_append_idempotently_and_unfavorite_without_reordering_others() {
    let t = frozen();
    let mut first = member(&t, "designers");
    let mut second = member(&t, "hq");
    let second = t.write(move |tx| {
        first.favorite(tx)?;
        second.favorite(tx)?;
        Ok(second)
    });
    assert_eq!(second.favorite_position, Some(1));
    t.travel(60);
    let same = second.clone();
    let mut second = t.write(move |tx| {
        let mut second = second;
        second.favorite(tx)?;
        Ok(second)
    });
    assert_eq!(second, same);
    t.write(move |tx| second.unfavorite(tx));
    assert!(!member(&t, "hq").favorited());
    assert_eq!(member(&t, "designers").favorite_position, Some(0));
}

#[test]
fn favorites_move_to_absolute_positions_and_clamp_both_ends() {
    let t = frozen();
    let mut members = [
        member(&t, "designers"),
        member(&t, "hq"),
        member(&t, "pets"),
    ];
    t.write(move |tx| {
        for m in &mut members {
            m.favorite(tx)?;
        }
        Ok(())
    });
    let ids = |t: &TestDb| {
        t.read(|c| Membership::favorites_for_user(c, id("david")))
            .iter()
            .map(|m| m.room_id)
            .collect::<Vec<_>>()
    };
    t.travel(60);
    let mut pets = member(&t, "pets");
    t.write(move |tx| pets.move_favorite_to(tx, -5));
    assert_eq!(ids(&t), [id("pets"), id("designers"), id("hq")]);
    assert!(
        t.read(|c| Membership::favorites_for_user(c, id("david")))
            .iter()
            .all(|m| m.updated_at == t.now())
    );
    let mut pets = member(&t, "pets");
    t.write(move |tx| pets.move_favorite_to(tx, 99));
    assert_eq!(ids(&t), [id("designers"), id("hq"), id("pets")]);
    assert_eq!(member(&t, "pets").favorite_position, Some(2));
}

#[test]
fn category_and_favorite_writes_rollback_with_the_transaction() {
    let t = frozen();
    let from = t.events().len();
    let result: Result<()> = t.try_write(|tx| {
        let category = RoomCategory::create(tx, id("david"), "Team", 1, false)?;
        let mut member =
            Membership::find_by_room_and_user(tx.conn(), id("designers"), id("david"))?.unwrap();
        member.update_category(tx, Some(category.id))?;
        member.favorite(tx)?;
        Err(Error::Other("rollback".into()))
    });
    assert!(result.is_err());
    assert!(!member(&t, "designers").favorited());
    assert_eq!(member(&t, "designers").room_category_id, None);
    assert!(
        t.read(|c| RoomCategory::ordered_for_user(c, id("david")))
            .is_empty()
    );
    assert_eq!(t.events().len(), from);
}
