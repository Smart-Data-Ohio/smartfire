//! Rails ChannelThreadBoardTest and ThreadTag callbacks, using real writes and a frozen clock.
use super::*;
use crate::{ChannelThread, Involvement, Membership, NewChannelThread, Room, RoomType, ThreadTag};
use crate::broadcasts::{Broadcast, TurboAction};

fn board(t: &TestDb) -> Room {
    t.write(|tx| Room::create_for(tx, RoomType::Board, Some("Launch"), id("david"), &[id("david"), id("jz"), id("kevin")]))
}

fn post(t: &TestDb, room: i64) -> ChannelThread {
    t.write(move |tx| ChannelThread::create(tx, NewChannelThread {
        room_id: room, creator_id: id("jz"), name: Some("Tagged post".into()), work_status: Some("planned".into()), ..Default::default()
    }))
}

fn rows(t: &TestDb, from: usize) -> Vec<(TurboAction, String)> {
    t.events()[from..].iter().filter_map(|event| match event.as_broadcast()? {
        Broadcast::Turbo(frame) if frame.target.starts_with("board_") => Some((frame.action, frame.target)),
        _ => None,
    }).collect()
}

#[test]
fn creation_prepends_both_renderings_and_marks_only_visible_disconnected_unmuted_members() {
    let t = channel_thread_test::frozen();
    let room = board(&t);
    t.write(move |tx| {
        let mut membership = Membership::find_by_room_and_user(tx.conn(), room.id, id("kevin"))?.unwrap();
        membership.update_involvement(tx, Some(Involvement::Muted))?;
        Ok(())
    });
    let from = t.events().len();
    let thread = post(&t, room.id);
    assert_eq!(rows(&t, from), [(TurboAction::Prepend, "board_posts".into()), (TurboAction::Prepend, "board_column_planned".into())]);
    assert!(t.read(|conn| Ok(Membership::find_by_room_and_user(conn, room.id, id("david"))?.unwrap().unread())));
    assert!(!t.read(|conn| Ok(Membership::find_by_room_and_user(conn, room.id, id("kevin"))?.unwrap().unread())));
    assert!(!t.read(|conn| Ok(Membership::find_by_room_and_user(conn, room.id, id("jz"))?.unwrap().unread())));
    assert!(t.events()[from..].iter().any(|event| matches!(event.as_broadcast(), Some(Broadcast::Cable {stream, payload}) if stream == format!("user_{}_unread_rooms", id("david")) && payload == serde_json::json!({"roomId": room.id}))));
    assert_eq!(thread.work_status_changed_at, Some(t.now()));
}

#[test]
fn direct_tag_create_and_destroy_replace_both_rows_without_touching_the_thread() {
    let t = channel_thread_test::frozen();
    let room = board(&t);
    let thread = post(&t, room.id);
    let from = t.events().len();
    t.travel(60);
    let tag = t.write(move |tx| ThreadTag::create(tx, thread.id, "bug"));
    assert_eq!(rows(&t, from), [(TurboAction::Replace, format!("board_row_channel_thread_{}", thread.id)), (TurboAction::Replace, format!("board_column_row_channel_thread_{}", thread.id))]);
    assert_eq!(t.read(|conn| ChannelThread::find(conn, thread.id)).updated_at, thread.updated_at);
    let from = t.events().len();
    t.write(move |tx| tag.destroy(tx));
    assert_eq!(rows(&t, from).len(), 2);
}

#[test]
fn deleting_a_post_removes_both_rows_without_replacing_each_dependent_tag() {
    let t = channel_thread_test::frozen();
    let room = board(&t);
    let thread = post(&t, room.id);
    t.write(move |tx| ThreadTag::create(tx, thread.id, "bug"));
    let from = t.events().len();
    let thread_id = thread.id;
    t.write(move |tx| thread.destroy(tx));
    assert_eq!(rows(&t, from), [(TurboAction::Remove, format!("board_row_channel_thread_{thread_id}")), (TurboAction::Remove, format!("board_column_row_channel_thread_{thread_id}"))]);
}
