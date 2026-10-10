use super::channel_thread_test::frozen;
use super::*;
use crate::{Error, Membership, Room, RoomCategory, WorkspaceCategory};

fn placements(t: &TestDb, category_id: Option<i64>) -> Vec<(i64, Option<i64>)> {
    t.read(move |conn| WorkspaceCategory::rooms(conn, category_id))
        .into_iter()
        .map(|room| (room.room_id, room.position))
        .collect()
}

#[test]
fn workspace_categories_create_rename_reorder_and_validate() {
    let t = frozen();
    let first = t.write(|tx| WorkspaceCategory::create(tx, "First"));
    let second = t.write(|tx| WorkspaceCategory::create(tx, "Second"));
    assert_eq!((first.position, second.position), (0, 1));
    t.travel(60);
    let original = first.clone();
    let renamed = t.write(move |tx| original.rename(tx, "Team"));
    assert_eq!(renamed.updated_at, t.now());
    assert_eq!(renamed.created_at, first.created_at);
    for name in [" ".to_string(), "é".repeat(51)] {
        assert!(matches!(
            t.try_write(move |tx| WorkspaceCategory::create(tx, &name)),
            Err(Error::RecordInvalid(_))
        ));
    }
    let ordered = t
        .write(move |tx| WorkspaceCategory::reorder(tx, &[second.id, first.id]))
        .unwrap();
    assert_eq!(
        ordered
            .iter()
            .map(|row| (row.id, row.position))
            .collect::<Vec<_>>(),
        [(second.id, 0), (first.id, 1)]
    );
    for ids in [
        vec![first.id],
        vec![first.id, first.id],
        vec![first.id, second.id, -1],
    ] {
        assert!(
            t.write(move |tx| WorkspaceCategory::reorder(tx, &ids))
                .is_none()
        );
    }
    assert_eq!(t.read(WorkspaceCategory::ordered), ordered);
}

#[test]
fn workspace_rooms_move_within_and_across_categories_with_dense_positions() {
    let t = frozen();
    let first = t.write(|tx| WorkspaceCategory::create(tx, "First"));
    let second = t.write(|tx| WorkspaceCategory::create(tx, "Second"));
    for room in ["hq", "pets", "designers"] {
        t.write(move |tx| WorkspaceCategory::move_room(tx, id(room), Some(first.id), i64::MAX));
    }
    assert_eq!(
        placements(&t, Some(first.id)),
        [
            (id("hq"), Some(0)),
            (id("pets"), Some(1)),
            (id("designers"), Some(2))
        ]
    );
    t.write(move |tx| WorkspaceCategory::move_room(tx, id("designers"), Some(first.id), -1));
    assert_eq!(
        placements(&t, Some(first.id)),
        [
            (id("designers"), Some(0)),
            (id("hq"), Some(1)),
            (id("pets"), Some(2))
        ]
    );
    t.write(move |tx| WorkspaceCategory::move_room(tx, id("hq"), Some(second.id), 0));
    assert_eq!(
        placements(&t, Some(first.id)),
        [(id("designers"), Some(0)), (id("pets"), Some(1))]
    );
    assert_eq!(placements(&t, Some(second.id)), [(id("hq"), Some(0))]);
    t.write(move |tx| WorkspaceCategory::move_room(tx, id("pets"), None, 0));
    assert_eq!(placements(&t, Some(first.id)), [(id("designers"), Some(0))]);
    assert!(
        placements(&t, None)
            .iter()
            .enumerate()
            .all(|(i, (_, pos))| *pos == Some(i as i64))
    );
}

#[test]
fn workspace_delete_appends_rooms_to_uncategorized_and_preserves_personal_settings() {
    let t = frozen();
    let category = t.write(|tx| WorkspaceCategory::create(tx, "Team"));
    let personal = t.write(|tx| RoomCategory::create(tx, id("david"), "Mine", 1, false));
    let membership = t.write(move |tx| {
        let mut membership =
            Membership::find_by_room_and_user(tx.conn(), id("hq"), id("david"))?.unwrap();
        membership.favorite(tx)?;
        membership.update_category(tx, Some(personal.id))?;
        Ok(membership)
    });
    t.write(move |tx| WorkspaceCategory::move_room(tx, id("hq"), Some(category.id), 0));
    let before = placements(&t, None)
        .iter()
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();
    t.write(move |tx| category.destroy(tx));
    let after = placements(&t, None);
    assert_eq!(
        after.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        [before, vec![id("hq")]].concat()
    );
    assert!(
        after
            .iter()
            .enumerate()
            .all(|(i, (_, pos))| *pos == Some(i as i64))
    );
    assert_eq!(
        t.read(|conn| Membership::find(conn, membership.id)),
        membership
    );
    assert_eq!(
        t.read(|conn| RoomCategory::find(conn, personal.id)),
        personal
    );
    assert!(t.read(WorkspaceCategory::ordered).is_empty());
}

#[test]
fn workspace_moves_roll_back_positions_and_broadcasts_on_failure() {
    let t = frozen();
    let category = t.write(|tx| WorkspaceCategory::create(tx, "Team"));
    let before = placements(&t, None);
    t.sink.take();
    assert!(
        t.try_write(move |tx| {
            WorkspaceCategory::move_room(tx, id("hq"), Some(category.id), 0)?;
            WorkspaceCategory::move_room(tx, id("pets"), Some(-1), 0)
        })
        .is_err()
    );
    assert_eq!(placements(&t, None), before);
    assert!(placements(&t, Some(category.id)).is_empty());
    assert!(t.sink.events().is_empty());
    let direct = t
        .read(|conn| {
            crate::sql::query_one(
                conn,
                "SELECT id FROM rooms WHERE type = 'Rooms::Direct' LIMIT 1",
                [],
                |row| row.get::<_, i64>(0),
            )
        })
        .unwrap();
    assert!(matches!(
        t.try_write(move |tx| WorkspaceCategory::move_room(tx, direct, Some(category.id), 0)),
        Err(Error::RecordInvalid(_))
    ));
    t.write(move |tx| WorkspaceCategory::move_room(tx, id("hq"), Some(category.id), 0));
    assert_eq!(t.sink.events().len(), 1);
    assert!(
        matches!(&t.sink.events()[0], Event::Broadcast(request) if request.kind == "WorkspaceCategory#sync_organized")
    );
}

#[test]
fn workspace_category_foreign_key_nullifies_room_assignments() {
    let t = frozen();
    let category = t.write(|tx| WorkspaceCategory::create(tx, "Team"));
    t.write(move |tx| WorkspaceCategory::move_room(tx, id("hq"), Some(category.id), 0));
    t.write(move |tx| {
        tx.conn().execute(
            "DELETE FROM workspace_categories WHERE id = ?",
            [category.id],
        )?;
        Ok(())
    });
    assert!(
        placements(&t, None)
            .iter()
            .any(|(room_id, _)| *room_id == id("hq"))
    );
}

#[test]
fn workspace_visibility_matches_the_sidebar_and_excludes_direct_messages() {
    let t = frozen();
    t.write(|tx| {
        tx.conn().execute(
            "UPDATE memberships SET involvement = 'invisible' WHERE room_id = ? AND user_id = ?",
            rusqlite::params![id("hq"), id("david")],
        )?;
        tx.conn().execute(
            "UPDATE memberships SET involvement = NULL WHERE room_id = ? AND user_id = ?",
            rusqlite::params![id("designers"), id("david")],
        )?;
        tx.conn().execute(
            "UPDATE rooms SET deleted_at = ? WHERE id = ?",
            rusqlite::params![tx.now(), id("pets")],
        )?;
        Ok(())
    });
    let mut expected = t
        .read(|conn| Membership::visible_with_ordered_room(conn, id("david")))
        .iter()
        .filter(|(_, room)| !room.direct())
        .map(|(_, room)| room.id)
        .collect::<Vec<_>>();
    let mut actual = t
        .read(|conn| WorkspaceCategory::visible_rooms(conn, id("david")))
        .iter()
        .map(|row| row.room_id)
        .collect::<Vec<_>>();
    expected.sort_unstable();
    actual.sort_unstable();
    assert_eq!(actual, expected);
}

#[test]
fn deleting_workspace_categories_compacts_the_remaining_order() {
    let t = frozen();
    let first = t.write(|tx| WorkspaceCategory::create(tx, "First"));
    let second = t.write(|tx| WorkspaceCategory::create(tx, "Second"));
    let third = t.write(|tx| WorkspaceCategory::create(tx, "Third"));
    t.write(move |tx| second.destroy(tx));
    assert_eq!(
        t.read(WorkspaceCategory::ordered)
            .iter()
            .map(|row| (row.id, row.position))
            .collect::<Vec<_>>(),
        [(first.id, 0), (third.id, 1)]
    );
}

fn deleting_room_compacts_workspace_positions(soft: bool) {
    for categorized in [true, false] {
        let t = frozen();
        let category = if categorized {
            Some(t.write(|tx| WorkspaceCategory::create(tx, "Team")).id)
        } else {
            None
        };
        for room in ["hq", "pets", "designers"] {
            t.write(move |tx| WorkspaceCategory::move_room(tx, id(room), category, i64::MAX));
        }
        let before = placements(&t, category);
        let expected = before
            .iter()
            .filter(|(room, _)| *room != id("pets"))
            .enumerate()
            .map(|(position, (room, _))| (*room, Some(position as i64)))
            .collect::<Vec<_>>();
        t.sink.take();
        t.write(move |tx| {
            let room = Room::find(tx.conn(), id("pets"))?;
            if soft {
                room.begin_destroy(tx)
            } else {
                room.destroy(tx)
            }
        });
        assert_eq!(placements(&t, category), expected);
        assert!(t.sink.events().iter().any(|event| matches!(event, Event::Broadcast(request) if request.kind == "WorkspaceCategory#sync_organized")));
    }
}

#[test]
fn workspace_room_soft_deletion_compacts_positions_and_publishes() {
    deleting_room_compacts_workspace_positions(true);
}

#[test]
fn workspace_room_destruction_compacts_positions_and_publishes() {
    deleting_room_compacts_workspace_positions(false);
}

#[test]
fn workspace_room_deletion_rolls_back_when_compaction_fails() {
    for soft in [true, false] {
        let t = frozen();
        let category = t.write(|tx| WorkspaceCategory::create(tx, "Team"));
        for room in ["hq", "pets", "designers"] {
            t.write(move |tx| {
                WorkspaceCategory::move_room(tx, id(room), Some(category.id), i64::MAX)
            });
        }
        let before = placements(&t, Some(category.id));
        t.write(|tx| {
            tx.conn().execute_batch("CREATE TRIGGER reject_workspace_compaction BEFORE UPDATE OF workspace_position ON rooms WHEN NEW.workspace_position != OLD.workspace_position BEGIN SELECT RAISE(ABORT, 'compaction failed'); END")?;
            Ok(())
        });
        t.sink.take();
        assert!(
            t.try_write(move |tx| {
                let room = Room::find(tx.conn(), id("pets"))?;
                if soft {
                    room.begin_destroy(tx)
                } else {
                    room.destroy(tx)
                }
            })
            .is_err()
        );
        assert_eq!(placements(&t, Some(category.id)), before);
        assert!(!t.read(|conn| Room::find(conn, id("pets"))).deleted());
        assert!(t.sink.events().is_empty());
    }
}

#[test]
fn workspace_membership_visibility_changes_publish_layout() {
    let t = frozen();
    let mut membership = t
        .read(|conn| Membership::find_by_room_and_user(conn, id("hq"), id("david")))
        .unwrap();
    for involvement in [
        Some(crate::Involvement::Invisible),
        Some(crate::Involvement::Everything),
        None,
        Some(crate::Involvement::Muted),
    ] {
        t.sink.take();
        membership = t.write(move |tx| {
            membership.update_involvement(tx, involvement)?;
            Ok(membership)
        });
        assert_eq!(t.sink.events().iter().filter(|event| matches!(event, Event::Broadcast(request) if request.kind == "WorkspaceCategory#sync_organized")).count(), 1);
    }
    for involvement in [crate::Involvement::Everything, crate::Involvement::Mentions] {
        t.sink.take();
        membership = t.write(move |tx| {
            membership.update_involvement(tx, involvement)?;
            Ok(membership)
        });
        assert!(!t.sink.events().iter().any(|event| matches!(event, Event::Broadcast(request) if request.kind == "WorkspaceCategory#sync_organized")));
    }
}
