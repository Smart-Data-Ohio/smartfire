use super::*;

#[test]
fn ignore_pair_is_unique_and_cannot_reference_self() {
    let t = TestDb::new();
    let (viewer, author) = (id("david"), id("kevin"));
    t.write(move |tx| {
        tx.conn().execute(
            "INSERT INTO user_blocks(blocker_id,blocked_id,created_at) VALUES (?,?,?)",
            rusqlite::params![viewer, author, tx.now()],
        )?;
        Ok(())
    });
    let duplicate = t
        .try_write(move |tx| {
            tx.conn().execute(
                "INSERT INTO user_blocks(blocker_id,blocked_id,created_at) VALUES (?,?,?)",
                rusqlite::params![viewer, author, tx.now()],
            )?;
            Ok(())
        })
        .unwrap_err();
    assert!(duplicate.is_record_not_unique());
    assert!(
        t.try_write(move |tx| {
            tx.conn().execute(
                "INSERT INTO user_blocks(blocker_id,blocked_id,created_at) VALUES (?,?,?)",
                rusqlite::params![viewer, viewer, tx.now()],
            )?;
            Ok(())
        })
        .is_err()
    );
}

#[test]
fn ignore_suppresses_message_mentions_and_pushes_only_for_blocker() {
    use crate::{ActivityItem, Message, NewMessage, PushSubscription, User};
    let t = TestDb::new();
    let (viewer, sender) = (id("jason"), id("david"));
    t.write(move |tx| {
        tx.conn().execute(
            "INSERT INTO user_blocks(blocker_id,blocked_id,created_at) VALUES (?,?,?)",
            rusqlite::params![viewer, sender, tx.now()],
        )?;
        tx.conn().execute(
            "UPDATE memberships SET unread_at=NULL, last_read_message_id=NULL WHERE room_id=?",
            [id("designers")],
        )?;
        Ok(())
    });
    let message = t.write(move |tx| {
        Message::create(
            tx,
            NewMessage {
                room_id: id("designers"),
                creator_id: sender,
                body: Some(format!(
                    "Ignore regression {}",
                    crate::rich_text::mention_attachment_for(viewer)
                )),
                ..Default::default()
            },
        )
    });
    t.write(move |tx| {
        ActivityItem::record_message(tx, &message)?;
        Ok(())
    });
    t.read(|conn| {
        let user = User::find(conn, viewer)?;
        assert_eq!(ActivityItem::unread_count(conn, &user, t.now())?, 0);
        let message = Message::last_page(conn, crate::Timeline::Room(id("designers")))?
            .pop()
            .unwrap();
        let (_, pushes, _) = PushSubscription::pushes_for(conn, &BasicRichText, &message, t.now())?;
        assert!(pushes.iter().all(|s| s.user_id != viewer));
        assert!(
            !pushes.is_empty(),
            "other recipients still get notifications"
        );
        assert!(
            crate::Membership::find_by_room_and_user(conn, id("designers"), viewer)?
                .unwrap()
                .unread_at
                .is_none()
        );
        Ok(())
    });
}

#[test]
fn ignore_model_is_idempotent_and_private_and_rejects_self() {
    use crate::models::{ignore_policy, user_block::UserBlock};
    let t = TestDb::new();
    let (viewer, author, other) = (id("david"), id("bender"), id("kevin"));
    let first = t.write(move |tx| UserBlock::set(tx, viewer, author, true));
    assert_eq!(
        t.write(move |tx| UserBlock::set(tx, viewer, author, true)),
        first
    );
    t.read(|conn| {
        assert_eq!(UserBlock::for_viewer(conn, viewer)?.len(), 1);
        assert!(ignore_policy::ignored(conn, viewer, author)?);
        assert!(!ignore_policy::ignored(conn, other, author)?);
        Ok(())
    });
    let second = t.write(move |tx| UserBlock::set(tx, viewer, author, false));
    assert!(second > first);
    assert_eq!(
        t.write(move |tx| UserBlock::set(tx, viewer, author, false)),
        second
    );
    assert!(
        t.try_write(move |tx| UserBlock::set(tx, viewer, viewer, true))
            .is_err()
    );
    assert!(
        t.events().is_empty(),
        "preference changes enqueue no jobs or public broadcasts"
    );
}

#[test]
fn ignore_existing_mentions_disappear_from_badges_and_return_after_unignore() {
    use crate::models::user_block::UserBlock;
    use crate::{ActivityItem, Message, NewMessage, User};
    let t = TestDb::new();
    let (viewer, author) = (id("jason"), id("david"));
    let message = t.write(move |tx| {
        Message::create(
            tx,
            NewMessage {
                room_id: id("designers"),
                creator_id: author,
                body: Some(crate::rich_text::mention_attachment_for(viewer)),
                ..Default::default()
            },
        )
    });
    t.write(move |tx| {
        ActivityItem::record_message(tx, &message)?;
        Ok(())
    });
    let count =
        || t.read(|conn| ActivityItem::unread_count(conn, &User::find(conn, viewer)?, t.now()));
    assert_eq!(count(), 1);
    t.write(move |tx| UserBlock::set(tx, viewer, author, true));
    assert_eq!(count(), 0);
    t.write(move |tx| UserBlock::set(tx, viewer, author, false));
    assert_eq!(count(), 1);
}

#[test]
fn ignore_thread_replies_suppress_unread_mentions_and_delivery_only_for_blocker() {
    use crate::models::user_block::UserBlock;
    use crate::{ActivityItem, ChannelThread, NewMessage, ThreadInvolvement, ThreadMembership};
    let t = TestDb::new();
    let (viewer, author, other) = (id("jason"), id("david"), id("kevin"));
    let thread = super::channel_thread_test::create_thread(
        &t,
        "designers",
        "david",
        None,
        Some("Ignore test"),
    );
    let thread_id = thread.id;
    t.write(move |tx| {
        UserBlock::set(tx, viewer, author, true)?;
        for user in [viewer, other] {
            let mut member = ThreadMembership::join(tx, thread_id, user)?;
            member.update_involvement(tx, ThreadInvolvement::Everything)?;
            member.read(tx)?;
        }
        Ok(())
    });
    let message = t.write(move |tx| {
        ChannelThread::find(tx.conn(), thread_id)?.post_message(
            tx,
            author,
            NewMessage {
                body: Some(crate::rich_text::mention_attachment_for(viewer)),
                ..Default::default()
            },
        )
    });
    let message_id = message.id;
    t.write(move |tx| {
        ActivityItem::record_message(tx, &message)?;
        Ok(())
    });
    t.read(|conn| {
        assert!(!ChannelThread::find(conn, thread_id)?.unread_for(conn, viewer)?);
        assert!(ChannelThread::find(conn, thread_id)?.unread_for(conn, other)?);
        let pushes = ChannelThread::push_recipients_with_policy(
            conn,
            &BasicRichText,
            thread_id,
            message_id,
            t.now(),
        )?;
        assert!(pushes.iter().all(|push| push.user_id != viewer));
        assert!(pushes.iter().any(|push| push.user_id == other));
        Ok(())
    });
}

#[test]
fn ignore_search_sections_exclude_ignored_thread_authors() {
    use crate::models::{search_query::SearchQuery, user_block::UserBlock};
    use crate::{ChannelThread, NewChannelThread};
    let t = TestDb::new();
    let viewer = id("david");
    let (hidden, visible) = t.write(move |tx| {
        let mut ids = Vec::new();
        for author in [id("kevin"), id("jason")] {
            ids.push(
                ChannelThread::create(
                    tx,
                    NewChannelThread {
                        room_id: id("designers"),
                        creator_id: author,
                        name: Some("Ignore section witness".into()),
                        work_status: Some("planned".into()),
                        ..Default::default()
                    },
                )?
                .id,
            );
        }
        UserBlock::set(tx, viewer, id("kevin"), true)?;
        Ok((ids[0], ids[1]))
    });
    t.read(|conn| {
        let results = SearchQuery::parse_extended("Ignore section witness")
            .sections_for_user(conn, viewer)?;
        let ids: Vec<i64> = results
            .iter()
            .flat_map(|section| section.records.iter().map(|record| record.id))
            .collect();
        assert!(!ids.contains(&hidden));
        assert!(ids.contains(&visible));
        Ok(())
    });
}
