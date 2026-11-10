use super::*;

#[test]
fn ignore_non_message_notification_sources_suppress_current_and_queued_delivery() {
    use crate::models::{notification_push, user_block::UserBlock};
    use crate::{CalendarEvent, NewCalendarEvent, PushPayload};
    let t = TestDb::new();
    let (viewer, author) = (id("jason"), id("david"));
    let event = t.write(move |tx| {
        CalendarEvent::create(
            tx,
            NewCalendarEvent {
                room_id: id("designers"),
                organizer_id: author,
                title: "Ignored event".into(),
                starts_at: Some(tx.now().since(jiff::SignedDuration::from_mins(10))),
                time_zone: "UTC".into(),
                ..Default::default()
            },
        )
    });
    let event_id = event.id;
    t.write(move |tx| {
        CalendarEvent::respond(tx, event_id, viewer, "going", false)?;
        UserBlock::set(tx, viewer, author, true)?;
        Ok(())
    });
    t.read(|conn| {
        assert!(!event.notification_recipient_ids(conn)?.contains(&viewer));
        assert!(
            !CalendarEvent::reminder_push_source(conn, event_id, t.now())?
                .unwrap()
                .recipient_ids
                .contains(&viewer)
        );
        Ok(())
    });
    t.write(move |tx| {
        let payload = PushPayload::new(
            "Ignored huddle".into(),
            "Ignored sender".into(),
            "/rooms/1".into(),
            None,
        );
        assert!(!notification_push::enqueue_huddle_invitation(
            tx,
            id("designers"),
            viewer,
            author,
            payload
        )?);
        Ok(())
    });
}

#[test]
fn ignore_work_activity_filters_old_items_and_stops_new_notifications() {
    use crate::models::{channel_thread::WorkChanges, user_block::UserBlock};
    use crate::{
        ActivityItem, ChannelThread, NewChannelThread, ThreadInvolvement, ThreadMembership, User,
    };
    let t = TestDb::new();
    let (viewer, author) = (id("jason"), id("david"));
    let thread = t.write(move |tx| {
        let thread = ChannelThread::create(
            tx,
            NewChannelThread {
                room_id: id("designers"),
                creator_id: author,
                name: Some("Ignore work events".into()),
                work_status: Some("planned".into()),
                ..Default::default()
            },
        )?;
        ThreadMembership::join(tx, thread.id, viewer)?
            .update_involvement(tx, ThreadInvolvement::Everything)?;
        Ok(thread)
    });
    let thread_id = thread.id;
    t.write(move |tx| {
        ChannelThread::find(tx.conn(), thread_id)?.update_work(
            tx,
            &User::find(tx.conn(), author)?,
            WorkChanges {
                status: Some(Some("in_progress".into())),
                ..Default::default()
            },
        )
    });
    t.read(|conn| {
        assert_eq!(
            ActivityItem::unread_count(conn, &User::find(conn, viewer)?, t.now())?,
            1
        );
        Ok(())
    });
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE activity_items SET read_at=? WHERE user_id=?",
            rusqlite::params![tx.now(), viewer],
        )?;
        UserBlock::set(tx, viewer, author, true)
    });
    t.read(|conn| {
        let user = User::find(conn, viewer)?;
        assert!(ActivityItem::accessible_to(conn, &user)?.is_empty());
        assert_eq!(ActivityItem::unread_count(conn, &user, t.now())?, 0);
        Ok(())
    });
    let before = t.events().len();
    t.write(move |tx| {
        ChannelThread::find(tx.conn(), thread_id)?.update_work(
            tx,
            &User::find(tx.conn(), author)?,
            WorkChanges {
                status: Some(Some("done".into())),
                ..Default::default()
            },
        )
    });
    assert!(t.events()[before..].iter().any(|event| {
        matches!(event, crate::Event::Broadcast(request) if request.decode::<crate::models::activity_item::ActivityItemRecorded>()
            .is_some_and(|recorded| recorded.unwrap().user_id == viewer && request.arguments["id"] == 0))
    }), "{:?}", &t.events()[before..]);
    t.read(|conn| {
        assert_eq!(
            ActivityItem::unread_count(conn, &User::find(conn, viewer)?, t.now())?,
            0,
            "the canonical publication must leave the blocker's visible inbox unchanged"
        );
        Ok(())
    });
}

#[test]
fn ignore_existing_room_and_thread_unread_and_push_badges_follow_policy() {
    use crate::models::user_block::UserBlock;
    use crate::{ChannelThread, Membership, NewMessage, PushSubscription, ThreadMembership};
    let t = TestDb::new();
    let (viewer, author, room) = (id("jason"), id("david"), id("designers"));
    t.write(move |tx| {
        for mut membership in Membership::for_user(tx.conn(), viewer)? {
            membership.read(tx)?;
        }
        Ok(())
    });
    let thread =
        channel_thread_test::create_thread(&t, "designers", "david", None, Some("Unread ignore"));
    let thread_id = thread.id;
    t.write(move |tx| {
        ThreadMembership::join(tx, thread_id, viewer)?.read(tx)?;
        crate::Message::create(
            tx,
            NewMessage {
                room_id: room,
                creator_id: author,
                body: Some("Unread root".into()),
                ..Default::default()
            },
        )?;
        ChannelThread::find(tx.conn(), thread_id)?.post_message(
            tx,
            author,
            NewMessage {
                body: Some("Unread reply".into()),
                ..Default::default()
            },
        )?;
        Ok(())
    });
    let check = |unread| {
        t.read(|conn| {
            assert_eq!(
                ChannelThread::find(conn, thread_id)?.unread_for(conn, viewer)?,
                unread
            );
            assert_eq!(
                ThreadMembership::unread_for_user(conn, viewer)?
                    .iter()
                    .any(|m| m.thread_id == thread_id),
                unread
            );
            assert_eq!(
                ThreadMembership::for_user_threads(conn, viewer, &[thread_id])?[0].unread(),
                unread
            );
            assert_eq!(
                Membership::unread_count(conn, viewer, t.now())?,
                i64::from(unread)
            );
            assert_eq!(
                Membership::find_by_room_and_user(conn, room, viewer)?
                    .unwrap()
                    .unread(),
                unread
            );
            assert_eq!(
                PushSubscription::for_user(conn, viewer)?[0].badge(conn, t.now())?,
                i64::from(unread)
            );
            Ok(())
        })
    };
    check(true);
    t.write(move |tx| UserBlock::set(tx, viewer, author, true));
    check(false);
    t.write(move |tx| UserBlock::set(tx, viewer, author, false));
    check(true);
    t.write(move |tx| {
        UserBlock::set(tx, viewer, author, true)?;
        crate::Message::create(
            tx,
            NewMessage {
                room_id: room,
                creator_id: id("kevin"),
                body: Some("Visible root".into()),
                ..Default::default()
            },
        )?;
        ChannelThread::find(tx.conn(), thread_id)?.post_message(
            tx,
            id("kevin"),
            NewMessage {
                body: Some("Visible reply".into()),
                ..Default::default()
            },
        )?;
        Ok(())
    });
    check(true);
}

#[test]
fn ignore_saved_reminder_fires_without_message_text_or_unread_count() {
    use crate::models::user_block::UserBlock;
    use crate::{ActivityItem, Message, NewMessage, NewSavedItem, SavedItem, User};
    let t = TestDb::new();
    let (viewer, author) = (id("jason"), id("david"));
    let saved = t.write(move |tx| {
        let message = Message::create(
            tx,
            NewMessage {
                room_id: id("designers"),
                creator_id: author,
                body: Some("Private ignored text".into()),
                ..Default::default()
            },
        )?;
        SavedItem::create(
            tx,
            NewSavedItem {
                user_id: viewer,
                message_id: message.id,
                remind_at: Some(tx.now().since(jiff::SignedDuration::from_mins(1))),
                ..Default::default()
            },
        )
    });
    t.write(move |tx| UserBlock::set(tx, viewer, author, true));
    t.clock
        .travel_to(t.now().since(jiff::SignedDuration::from_mins(2)));
    t.write(move |tx| SavedItem::dispatch_reminder(tx, saved.id, tx.now()));
    t.read(|conn| {
        let user = User::find(conn, viewer)?;
        assert_eq!(
            ActivityItem::accessible_to(conn, &user)?.len(),
            1,
            "explicit reminder survives"
        );
        assert_eq!(ActivityItem::unread_count(conn, &user, t.now())?, 0);
        assert_eq!(
            ActivityItem::unread_snapshot(conn, viewer, t.now())?.count,
            0
        );
        let push = SavedItem::reminder_push(conn, &BasicRichText, saved.id, &|_| true)?.unwrap();
        assert_eq!(push.payload.body, "Reminder: a saved message");
        Ok(())
    });
}

#[test]
fn ignore_fanouts_load_blockers_once_for_any_roster_size() {
    use crate::{
        ChannelThread, Message, NewMessage, NewUser, Room, RoomType, ThreadInvolvement,
        ThreadMembership, User,
    };
    for size in [3, 30] {
        let t = TestDb::new();
        let (room, thread, message) = t.write(move |tx| {
            let mut users = vec![id("david")];
            for n in 0..size {
                users.push(
                    User::create(
                        tx,
                        NewUser {
                            name: format!("Recipient {n}"),
                            ..Default::default()
                        },
                    )?
                    .id,
                );
            }
            let room = Room::create_for(
                tx,
                RoomType::Closed,
                Some("Policy queries"),
                users[0],
                &users,
            )?;
            let thread = ChannelThread::create(
                tx,
                crate::NewChannelThread {
                    room_id: room.id,
                    creator_id: users[0],
                    name: Some("Fanout".into()),
                    ..Default::default()
                },
            )?;
            for &user in &users[1..] {
                ThreadMembership::join(tx, thread.id, user)?
                    .update_involvement(tx, ThreadInvolvement::Everything)?;
            }
            let body = users[1..]
                .iter()
                .map(|id| crate::rich_text::mention_attachment_for(*id))
                .collect::<Vec<_>>()
                .join(" ");
            let message = Message::create(
                tx,
                NewMessage {
                    room_id: room.id,
                    creator_id: users[0],
                    thread_id: Some(thread.id),
                    body: Some(body),
                    ..Default::default()
                },
            )?;
            Ok((room, thread, message))
        });
        for lane in ["room", "activity", "thread", "push"] {
            let queries = t.db.capture_queries();
            let message = message.clone();
            let thread = thread.clone();
            t.write(move |tx| {
                match lane {
                    "room" => crate::models::agent_posting::broadcast_unread_room(tx, &message)?,
                    "activity" => {
                        crate::models::activity_item::message_recorder::candidates(
                            tx.conn(),
                            tx.rich_text(),
                            &message,
                            tx.now(),
                        )?;
                    }
                    "thread" => ChannelThread::receive(tx, thread.id, &message)?,
                    "push" => {
                        ChannelThread::push_recipients(
                            tx.conn(),
                            tx.rich_text(),
                            thread.id,
                            message.id,
                            &|_| true,
                        )?;
                    }
                    _ => unreachable!(),
                }
                Ok(())
            });
            t.db.stop_capturing_queries();
            let count = queries
                .lock()
                .unwrap()
                .iter()
                .filter(|sql| sql.contains("FROM user_blocks"))
                .count();
            let expected = usize::from(matches!(lane, "thread" | "push"));
            assert_eq!(
                count, expected,
                "{lane}: {size} recipients in room {}",
                room.id
            );
        }
    }
}

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
        let (_, pushes) = PushSubscription::pushes_for(conn, &BasicRichText, &message, t.now())?;
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

#[test]
fn ignore_reading_a_filtered_thread_persists_the_read_cursor() {
    use crate::models::user_block::UserBlock;
    use crate::{ChannelThread, NewMessage, ThreadMembership};
    let t = TestDb::new();
    let (viewer, author) = (id("jason"), id("david"));
    let thread = channel_thread_test::create_thread(
        &t,
        "designers",
        "david",
        None,
        Some("Read hidden thread"),
    );
    let thread_id = thread.id;
    let message = t.write(move |tx| {
        ThreadMembership::join(tx, thread_id, viewer)?;
        ChannelThread::find(tx.conn(), thread_id)?.post_message(
            tx,
            author,
            NewMessage {
                body: Some("Unread before block".into()),
                ..Default::default()
            },
        )
    });
    t.write(move |tx| {
        UserBlock::set(tx, viewer, author, true)?;
        let mut membership =
            ThreadMembership::find_by_thread_and_user(tx.conn(), thread_id, viewer)?.unwrap();
        assert!(!membership.unread());
        membership.read(tx)?;
        UserBlock::set(tx, viewer, author, false)
    });
    t.read(|conn| {
        let membership =
            ThreadMembership::find_by_thread_and_user(conn, thread_id, viewer)?.unwrap();
        assert!(
            !membership.unread(),
            "reading while blocked must survive unblocking"
        );
        assert!(
            membership
                .last_read_message_id
                .is_some_and(|id| id >= message.id)
        );
        Ok(())
    });
}

#[test]
fn ignore_board_unread_filters_only_the_authors_content() {
    use crate::models::user_block::UserBlock;
    use crate::{ChannelThread, Membership, NewChannelThread, Room, RoomType};
    let t = TestDb::new();
    let (viewer, author, unrelated) = (id("jason"), id("david"), id("kevin"));
    let room = t.write(move |tx| {
        Room::create_for(
            tx,
            RoomType::Board,
            Some("Ignored board"),
            author,
            &[viewer, author, unrelated],
        )
    });
    let room_id = room.id;
    t.write(move |tx| {
        ChannelThread::create_board_post(
            tx,
            NewChannelThread {
                room_id,
                creator_id: author,
                name: Some("Visible post".into()),
                work_status: Some("planned".into()),
                ..Default::default()
            },
            Some("Board opener".into()),
        )?;
        Ok(())
    });
    for (blocked, unread) in [(unrelated, true), (author, false)] {
        t.write(move |tx| UserBlock::set(tx, viewer, blocked, true));
        t.read(|conn| {
            assert_eq!(
                Membership::find_by_room_and_user(conn, room_id, viewer)?
                    .unwrap()
                    .unread(),
                unread
            );
            assert_eq!(Membership::unread_count(conn, viewer, t.now())? > 0, unread);
            Ok(())
        });
    }
    t.write(move |tx| {
        Membership::find_by_room_and_user(tx.conn(), room_id, viewer)?
            .unwrap()
            .read(tx)?;
        UserBlock::set(tx, viewer, author, false)
    });
    t.read(|conn| {
        assert!(
            !Membership::find_by_room_and_user(conn, room_id, viewer)?
                .unwrap()
                .unread()
        );
        Ok(())
    });
}

#[test]
fn ignore_thread_recording_preserves_visible_group_and_never_creates_or_bumps_hidden_activity() {
    use crate::models::user_block::UserBlock;
    use crate::{
        ActivityItem, ChannelThread, NewMessage, ThreadInvolvement, ThreadMembership, User,
    };
    let t = TestDb::new();
    let (viewer, author, other) = (id("jason"), id("david"), id("kevin"));
    let thread =
        channel_thread_test::create_thread(&t, "designers", "david", None, Some("Group ignore"));
    let thread_id = thread.id;
    t.write(move |tx| {
        ThreadMembership::join(tx, thread_id, viewer)?
            .update_involvement(tx, ThreadInvolvement::Everything)?;
        UserBlock::set(tx, viewer, author, true)?;
        Ok(())
    });
    let post = |creator| {
        t.write(move |tx| {
            let message = ChannelThread::find(tx.conn(), thread_id)?.post_message(
                tx,
                creator,
                NewMessage {
                    body: Some("Reply".into()),
                    ..Default::default()
                },
            )?;
            ActivityItem::record_message(tx, &message)?;
            Ok(message.id)
        })
    };
    post(author);
    t.read(|conn| {
        assert!(ActivityItem::accessible_to(conn, &User::find(conn, viewer)?)?.is_empty());
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM activity_items WHERE user_id=? AND event_type='thread_activity'", [viewer], |row| row.get::<_, i64>(0))?, 0);
        Ok(())
    });
    let visible = post(other);
    let before = t
        .read(|conn| ActivityItem::find_by_user_and_source(conn, viewer, "Message", visible))
        .unwrap();
    t.clock.travel(jiff::SignedDuration::from_secs(1));
    post(author);
    t.read(|conn| {
        assert_eq!(ActivityItem::find(conn, before.id)?, before);
        assert_eq!(
            ActivityItem::accessible_to(conn, &User::find(conn, viewer)?)?,
            vec![before.clone()]
        );
        Ok(())
    });
    t.write(move |tx| before.mark_read(tx));
    let read = t
        .read(|conn| ActivityItem::find_by_user_and_source(conn, viewer, "Message", visible))
        .unwrap();
    post(author);
    t.read(|conn| {
        assert_eq!(ActivityItem::find(conn, read.id)?, read);
        Ok(())
    });
}

#[test]
fn ignore_sla_activity_maps_the_board_posts_author_for_all_read_paths() {
    use crate::models::activity_item::ActivitySource;
    use crate::models::user_block::UserBlock;
    use crate::{ActivityItem, BoardSlaNudge, NewBoardSlaNudge, User};
    let t = TestDb::new();
    let (viewer, author) = (id("jason"), id("david"));
    let thread =
        channel_thread_test::create_thread(&t, "designers", "david", None, Some("SLA ignore"));
    let thread_id = thread.id;
    let item = t.write(move |tx| {
        let nudge = BoardSlaNudge::create(
            tx,
            NewBoardSlaNudge {
                room_id: id("designers"),
                channel_thread_id: thread_id,
                recipient_id: viewer,
                work_status: Some("planned".into()),
                stage: Some("nudge".into()),
                status_entered_at: Some(tx.now()),
            },
        )?;
        ActivityItem::record(
            tx,
            viewer,
            ActivitySource::BoardSlaNudge(nudge.id),
            "work_sla",
            false,
        )?
        .ok_or(crate::Error::RecordNotFound("ActivityItem"))
    });
    t.write(move |tx| UserBlock::set(tx, viewer, author, true));
    t.read(|conn| {
        let user = User::find(conn, viewer)?;
        assert!(ActivityItem::find_accessible(conn, &user, item.id)?.is_none());
        assert!(ActivityItem::accessible_to(conn, &user)?.is_empty());
        assert_eq!(ActivityItem::unread_count(conn, &user, t.now())?, 0);
        assert_eq!(
            ActivityItem::unread_snapshot(conn, viewer, t.now())?.count,
            0
        );
        Ok(())
    });
}
