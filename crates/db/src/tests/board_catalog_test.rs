use super::*;
use crate::{BoardTag, BoardTagPolicy, ChannelThread, NewChannelThread, Room, RoomType};

fn board(t: &TestDb) -> i64 {
    t.write(|tx| Room::create_for(tx, RoomType::Board, Some("Catalog"), id("david"), &[id("david"), id("jz")])).id
}

fn post(t: &TestDb, room_id: i64, tags: &[&str]) -> Result<ChannelThread> {
    let tags = tags.iter().map(|name| (*name).to_string()).collect();
    t.try_write(move |tx| ChannelThread::create(tx, NewChannelThread {
        room_id, creator_id: id("jz"), name: Some("Catalog post".into()),
        work_status: Some("planned".into()), tag_names: Some(tags), ..Default::default()
    }))
}

fn names(t: &TestDb, post: &ChannelThread) -> Vec<String> {
    t.read(|conn| post.tag_names(conn))
}

#[test]
fn board_catalog_crud_limits_and_reorder() {
    let t = channel_thread_test::frozen();
    let room_id = board(&t);
    for name in [" ".to_string(), "é".repeat(21)] {
        assert!(t.try_write(move |tx| BoardTag::create(tx, room_id, &name, None)).is_err());
    }
    let tag = t.write(move |tx| BoardTag::create(tx, room_id, "  Bug Report  ", Some("🐛")));
    assert_eq!(tag.name, "Bug Report");
    assert_eq!(tag.position, 0);
    assert!(t.try_write(move |tx| BoardTag::create(tx, room_id, "bug report", None)).is_err());
    let mut ids = vec![tag.id];
    for index in 1..20 {
        ids.push(t.write(move |tx| BoardTag::create(tx, room_id, &format!("tag-{index}"), None)).id);
    }
    assert!(t.try_write(move |tx| BoardTag::create(tx, room_id, "overflow", None)).is_err());
    ids.reverse();
    let order = ids.clone();
    t.write(move |tx| BoardTag::reorder(tx, room_id, &order));
    assert_eq!(t.read(|conn| BoardTag::for_room(conn, room_id)).iter().map(|tag| tag.id).collect::<Vec<_>>(), ids);
    assert!(t.try_write(move |tx| BoardTag::reorder(tx, room_id, &[tag.id,tag.id])).is_err());
    let room2 = board(&t);
    assert!(t.try_write(move |tx| BoardTagPolicy::update(tx, room2, true, Some(tag.id))).is_err());
}

#[test]
fn board_catalog_rename_delete_merge_names_only_in_own_board() {
    let t = channel_thread_test::frozen();
    let room_id = board(&t);
    let other = board(&t);
    let post = post(&t, room_id, &["bug", "new-name", "legacy"]).unwrap();
    let other_post = self::post(&t, other, &["bug"]).unwrap();
    let tag = t.write(move |tx| BoardTag::create(tx, room_id, "bug", None));
    let from = t.events().len();
    t.write(move |tx| BoardTag::update(tx, room_id, tag.id, "new-name", Some("✨")));
    assert_eq!(names(&t, &post), ["legacy", "new-name"]);
    assert_eq!(names(&t, &other_post), ["bug"]);
    let changes = t.events()[from..].iter().filter(|event| matches!(event, Event::Broadcast(request) if request.decode::<crate::models::channel_thread::ThreadWorkChange>().is_some())).count();
    assert_eq!(changes, 1, "renaming publishes the final post snapshot once");
    let counts = t.read(|conn| ChannelThread::board_tag_counts(conn, room_id));
    assert_eq!(counts, [("legacy".into(),1),("new-name".into(),1)]);
    t.write(move |tx| BoardTagPolicy::update(tx, room_id, true, Some(tag.id)));
    let from = t.events().len();
    t.write(move |tx| BoardTag::destroy(tx, room_id, tag.id));
    assert_eq!(names(&t, &post), ["legacy"]);
    assert_eq!(names(&t, &other_post), ["bug"]);
    assert_eq!(t.read(|conn| BoardTagPolicy::for_room(conn, room_id)).default_board_tag_id, None);
    let changes = t.events()[from..].iter().filter(|event| matches!(event, Event::Broadcast(request) if request.decode::<crate::models::channel_thread::ThreadWorkChange>().is_some())).count();
    assert_eq!(changes, 1, "deleting publishes the final post snapshot once");
}

#[test]
fn board_catalog_required_default_create_edit_and_legacy() {
    let t = channel_thread_test::frozen();
    let room_id = board(&t);
    let legacy = post(&t, room_id, &["legacy"]).unwrap();
    let tag = t.write(move |tx| BoardTag::create(tx, room_id, "Bug Report", None));
    t.write(move |tx| BoardTagPolicy::update(tx, room_id, true, None));
    assert!(post(&t, room_id, &[]).is_err());
    assert!(post(&t, room_id, &["legacy"]).is_err());
    let curated = post(&t, room_id, &[" bug report ","BUG REPORT","legacy"]).unwrap();
    assert_eq!(names(&t, &curated), ["Bug Report","legacy"]);
    t.write(move |tx| ChannelThread::find(tx.conn(), legacy.id)?.update_metadata(tx, Some("Legacy renamed"), None, None));
    assert!(t.try_write(move |tx| ChannelThread::find(tx.conn(), curated.id)?.update_metadata(tx, None, None, Some(&[]))).is_err());
    t.write(move |tx| BoardTagPolicy::update(tx, room_id, true, Some(tag.id)));
    let defaulted = post(&t, room_id, &[]).unwrap();
    assert_eq!(names(&t, &defaulted), ["Bug Report"]);
    t.write(move |tx| ChannelThread::find(tx.conn(), curated.id)?.update_metadata(tx, None, None, Some(&["legacy".into()])));
    assert_eq!(names(&t, &curated), ["Bug Report","legacy"]);
    assert!(post(&t, room_id, &["a","b","c","d","e"]).is_err(), "default cannot exceed the five-tag limit");
}

#[test]
fn board_catalog_names_allow_unicode_and_filter_case_insensitively() {
    let t = channel_thread_test::frozen();
    let room_id = board(&t);
    t.write(move |tx| BoardTag::create(tx, room_id, "Équipe produit", None));
    let post = post(&t, room_id, &[" ÉQUIPE PRODUIT "]).unwrap();
    assert_eq!(names(&t, &post), ["Équipe produit"]);
    assert_eq!(t.read(|conn| ChannelThread::board_posts_for(conn, room_id, "all", "anyone", "équipe produit", None, 1))[0].id, post.id);
}

#[test]
fn board_catalog_transaction_rollback_preserves_posts_and_events() {
    let t = channel_thread_test::frozen();
    let room_id = board(&t);
    let tag = t.write(move |tx| BoardTag::create(tx, room_id, "bug", None));
    let post = post(&t, room_id, &["bug"]).unwrap();
    let from = t.events().len();
    let result: Result<()> = t.try_write(move |tx| {
        BoardTag::update(tx, room_id, tag.id, "Renamed tag", None)?;
        Err(crate::Error::Other("rollback".into()))
    });
    assert!(result.is_err());
    assert_eq!(names(&t, &post), ["bug"]);
    assert_eq!(t.events().len(), from);
}

#[test]
fn board_catalog_room_destruction_cleans_up_catalog_and_default() {
    let t = channel_thread_test::frozen();
    let room_id = board(&t);
    let tag = t.write(move |tx| BoardTag::create(tx, room_id, "bug", None));
    t.write(move |tx| BoardTagPolicy::update(tx, room_id, true, Some(tag.id)));
    post(&t, room_id, &[]).unwrap();
    t.write(move |tx| Room::find(tx.conn(), room_id)?.destroy(tx));
    assert!(t.read(|conn| BoardTag::for_room(conn, room_id)).is_empty());
    assert!(t.read(|conn| Room::find_by_id(conn, room_id)).is_none());
}

#[test]
fn board_catalog_default_preserves_case_insensitive_tag_auto_assignment() {
    let t = channel_thread_test::frozen();
    let room_id = board(&t);
    let tag = t.write(move |tx| BoardTag::create(tx, room_id, "Bug", None));
    t.write(move |tx| crate::BoardTagAssignment::create(tx, crate::NewBoardTagAssignment {
        room_id, tag: "bug".into(), assignee_id: id("jz"), created_by_id: id("david"),
    }));
    t.write(move |tx| BoardTagPolicy::update(tx, room_id, true, Some(tag.id)));
    let post = post(&t, room_id, &[]).unwrap();
    assert_eq!(t.read(|conn| ChannelThread::find(conn, post.id)).work_owner_id, Some(id("jz")));
}

#[test]
fn board_catalog_filter_includes_legacy_spelling_of_catalog_name() {
    let t = channel_thread_test::frozen();
    let room_id = board(&t);
    let legacy = post(&t, room_id, &["rust"]).unwrap();
    t.write(move |tx| BoardTag::create(tx, room_id, "Rust", None));
    let curated = post(&t, room_id, &["rust"]).unwrap();
    assert_eq!(names(&t, &legacy), ["rust"]);
    assert_eq!(names(&t, &curated), ["Rust"]);
    let filtered = t.read(|conn| ChannelThread::board_posts_for(conn, room_id, "all", "anyone", "RUST", None, 1));
    assert_eq!(filtered.iter().map(|post| post.id).collect::<Vec<_>>(), [curated.id, legacy.id]);
}
