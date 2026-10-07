use super::*;
use crate::{CachedStatements, Room, RoomType};

#[test]
fn room_creation_keys_are_nullable_viewer_scoped_and_unique() {
    let t = TestDb::new();
    let creator = id("david");
    let other = id("kevin");
    let (first, classic, independent) = t.write(move |tx| {
        let first = Room::create_for(tx, RoomType::Closed, Some("First"), creator, &[creator])?;
        let classic = Room::create_for(tx, RoomType::Closed, Some("Classic"), creator, &[creator])?;
        let independent = Room::create_for(tx, RoomType::Closed, Some("Other"), other, &[other])?;
        for room in [&first, &independent] {
            tx.conn().execute_cached(
                "UPDATE rooms SET client_room_id='once' WHERE id=?",
                [room.id],
            )?;
        }
        Ok((first, classic, independent))
    });
    t.read(|conn| {
        assert_eq!(
            Room::find_by_creation_key(conn, creator, "once")?
                .unwrap()
                .id,
            first.id
        );
        assert_eq!(
            Room::find_by_creation_key(conn, other, "once")?.unwrap().id,
            independent.id
        );
        assert!(Room::find_by_creation_key(conn, creator, "missing")?.is_none());
        let key: Option<String> = conn.query_row_cached(
            "SELECT client_room_id FROM rooms WHERE id=?",
            [classic.id],
            |row| row.get(0),
        )?;
        assert_eq!(key, None, "classic creation leaves the key unset");
        Ok(())
    });
    let counts = || {
        t.read(|conn| {
            Ok((
                conn.query_row_cached("SELECT COUNT(*) FROM rooms", [], |row| {
                    row.get::<_, i64>(0)
                })?,
                conn.query_row_cached("SELECT COUNT(*) FROM memberships", [], |row| {
                    row.get::<_, i64>(0)
                })?,
            ))
        })
    };
    let before = counts();
    let events = t.events();
    let error =
        t.db.write_blocking(move |tx| {
            // An open room has queued membership callbacks; the conflict must discard those too.
            let room = Room::create_for(tx, RoomType::Open, Some("Conflict"), creator, &[creator])?;
            tx.conn().execute_cached(
                "UPDATE rooms SET client_room_id='once' WHERE id=?",
                [room.id],
            )?;
            Ok(())
        })
        .unwrap_err();
    assert!(error.is_record_not_unique(), "{error}");
    assert_eq!(counts(), before);
    assert_eq!(t.events(), events);
    let first_id = first.id;
    t.write(move |tx| {
        tx.conn().execute_cached(
            "UPDATE rooms SET deleted_at=? WHERE id=?",
            rusqlite::params![tx.now(), first_id],
        )?;
        assert_eq!(
            Room::find_by_creation_key(tx.conn(), creator, "once")?
                .unwrap()
                .id,
            first_id
        );
        Ok(())
    });
}
