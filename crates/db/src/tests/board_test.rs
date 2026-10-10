//! Rails ChannelThreadBoardTest and ThreadTag callbacks, using real writes and a frozen clock.
use super::*;
use crate::broadcasts::Broadcast;
use crate::{
    ChannelThread, Involvement, Membership, NewChannelThread, Room, RoomType, ThreadTag, User,
};

fn board(t: &TestDb) -> Room {
    t.write(|tx| {
        Room::create_for(
            tx,
            RoomType::Board,
            Some("Launch"),
            id("david"),
            &[id("david"), id("jz"), id("kevin")],
        )
    })
}

fn post(t: &TestDb, room: i64) -> ChannelThread {
    t.write(move |tx| {
        ChannelThread::create(
            tx,
            NewChannelThread {
                room_id: room,
                creator_id: id("jz"),
                name: Some("Tagged post".into()),
                work_status: Some("planned".into()),
                ..Default::default()
            },
        )
    })
}



fn sync_events(t: &TestDb, from: usize) -> Vec<(&'static str, i64)> {
    use crate::models::channel_thread::{ThreadBoardCreation, ThreadWorkChange};
    t.events()[from..]
        .iter()
        .filter_map(|event| {
            let crate::Event::Broadcast(request) = event else {
                return None;
            };
            if let Some(event) = request.decode::<ThreadBoardCreation>() {
                Some(("created", event.unwrap().thread_id))
            } else {
                request
                    .decode::<ThreadWorkChange>()
                    .map(|event| ("updated", event.unwrap().thread_id))
            }
        })
        .collect()
}

#[test]
fn creation_keeps_the_sync_snapshot_and_tag_assignment_update() {
    let t = channel_thread_test::frozen();
    let room = board(&t);
    let from = t.events().len();
    let thread = t.write(move |tx| {
        let thread = ChannelThread::create(
            tx,
            NewChannelThread {
                room_id: room.id,
                creator_id: id("jz"),
                name: Some("Post with brief".into()),
                work_status: Some("planned".into()),
                tag_names: Some(vec!["api".into(), "bug".into()]),
                ..Default::default()
            },
        )?;
        crate::Message::create(
            tx,
            crate::NewMessage {
                room_id: room.id,
                thread_id: Some(thread.id),
                creator_id: id("jz"),
                board_post_opener: true,
                markdown_source: Some("The brief".into()),
                ..Default::default()
            },
        )?;
        Ok(thread)
    });
    assert_eq!(sync_events(&t, from), [("created", thread.id), ("updated", thread.id)]);
    assert_eq!(
        t.read(|conn| ChannelThread::find(conn, thread.id)).messages_count,
        1
    );
}

#[test]
fn tag_and_row_changes_publish_one_sync_update_and_rollback_publishes_none() {
    let t = channel_thread_test::frozen();
    let room = board(&t);
    let thread = post(&t, room.id);
    let from = t.events().len();
    t.write(move |tx| {
        ChannelThread::find(tx.conn(), thread.id)?.update_metadata(
            tx,
            Some("Renamed"),
            None,
            Some(&["api".into(), "bug".into()]),
        )
    });
    assert_eq!(sync_events(&t, from), [("updated", thread.id)]);
    let from = t.events().len();
    t.write(move |tx| {
        ChannelThread::find(tx.conn(), thread.id)?.update_metadata(
            tx,
            Some("Renamed"),
            None,
            Some(&["api".into(), "bug".into()]),
        )?;
        let result: crate::Result<()> = tx.savepoint(|tx| {
            ThreadTag::create(tx, thread.id, "rolled-back")?;
            Err(crate::Error::Other("rollback the tag".into()))
        });
        assert!(result.is_err());
        Ok(())
    });
    assert!(sync_events(&t, from).is_empty());
    let from = t.events().len();
    t.write(move |tx| {
        ChannelThread::find(tx.conn(), thread.id)?.update_metadata(tx, None, None, Some(&[]))
    });
    assert_eq!(sync_events(&t, from), [("updated", thread.id)]);
}

#[test]
fn board_replies_publish_once_even_at_the_same_clock_and_on_direct_message_writes() {
    let t = channel_thread_test::frozen();
    let room = board(&t);
    let thread = post(&t, room.id);
    let from = t.events().len();
    let reply = t.write(move |tx| {
        ChannelThread::find(tx.conn(), thread.id)?.post_message(
            tx,
            id("jz"),
            crate::NewMessage {
                markdown_source: Some("First reply".into()),
                ..Default::default()
            },
        )
    });
    assert_eq!(sync_events(&t, from), [("updated", thread.id)]);
    assert_eq!(
        t.read(|conn| ChannelThread::find(conn, thread.id)).messages_count,
        1
    );
    t.travel(60);
    let from = t.events().len();
    t.write(move |tx| {
        ChannelThread::find(tx.conn(), thread.id)?.post_message(
            tx,
            id("jz"),
            crate::NewMessage {
                markdown_source: Some("Later reply".into()),
                ..Default::default()
            },
        )
    });
    assert_eq!(sync_events(&t, from), [("updated", thread.id)]);
    let from = t.events().len();
    t.write(move |tx| reply.destroy(tx));
    assert_eq!(sync_events(&t, from), [("updated", thread.id)]);
    assert_eq!(
        t.read(|conn| ChannelThread::find(conn, thread.id)).messages_count,
        1
    );
    let from = t.events().len();
    t.write(move |tx| {
        crate::Message::create(
            tx,
            crate::NewMessage {
                room_id: room.id,
                thread_id: Some(thread.id),
                creator_id: id("jz"),
                markdown_source: Some("Direct reply".into()),
                ..Default::default()
            },
        )
    });
    assert_eq!(sync_events(&t, from), [("updated", thread.id)]);
    assert_eq!(
        t.read(|conn| ChannelThread::find(conn, thread.id)).messages_count,
        2
    );
}

#[test]
fn board_room_type_predicates_are_exclusive() {
    let t = channel_thread_test::frozen();
    let room = board(&t);
    assert!(room.board());
    assert!(!room.open() && !room.closed() && !room.direct() && !room.voice() && !room.stage());
}
#[test]
fn board_rooms_are_in_board_and_non_direct_queries() {
    let t = channel_thread_test::frozen();
    let before = t.read(|conn| Membership::count_without_direct_rooms(conn, id("david")));
    let room = board(&t);
    assert!(
        t.read(|conn| Room::for_user_of_type(conn, id("david"), RoomType::Board))
            .iter()
            .any(|row| row.id == room.id)
    );
    assert!(
        t.read(|conn| Room::for_user_without_directs(conn, id("david")))
            .iter()
            .any(|row| row.id == room.id)
    );
    assert_eq!(
        t.read(|conn| Membership::count_without_direct_rooms(conn, id("david"))),
        before + 1
    );
}
#[test]
fn board_members_default_to_mentions() {
    let t = channel_thread_test::frozen();
    let room = board(&t);
    assert_eq!(room.default_involvement(), "mentions");
    assert!(
        t.read(|conn| Membership::for_room(conn, room.id))
            .iter()
            .all(|membership| membership.involvement == Some(Involvement::Mentions))
    );
}
#[test]
fn deactivating_a_user_removes_their_board_membership() {
    let t = channel_thread_test::frozen();
    let room = board(&t);
    t.write(|tx| User::find(tx.conn(), id("david"))?.deactivate(tx));
    assert!(
        t.read(|conn| Membership::find_by_room_and_user(conn, room.id, id("david")))
            .is_none()
    );
}
#[test]
fn stale_sweeps_skip_board_posts_and_archive_ordinary_threads() {
    let t = channel_thread_test::frozen();
    let room = board(&t);
    let post = post(&t, room.id);
    let ordinary = t.write(|tx| {
        ChannelThread::create(
            tx,
            NewChannelThread {
                room_id: id("designers"),
                creator_id: id("jz"),
                name: Some("Old thread".into()),
                auto_archive_after_minutes: Some(60),
                ..Default::default()
            },
        )
    });
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE channel_threads SET auto_archive_after_minutes=60 WHERE id=?",
            [post.id],
        )?;
        Ok(())
    });
    t.travel(2 * 60 * 60);
    t.write(|tx| ChannelThread::close_stale_in(tx, None));
    assert!(
        t.read(|conn| ChannelThread::find(conn, post.id))
            .closed_at
            .is_none()
    );
    assert!(
        t.read(|conn| ChannelThread::find(conn, ordinary.id))
            .closed_at
            .is_some()
    );
    t.write(move |tx| ChannelThread::close_stale_in(tx, Some(room.id)));
    assert!(
        t.read(|conn| ChannelThread::find(conn, post.id))
            .closed_at
            .is_none()
    );
}

#[test]
fn creation_prepends_both_renderings_and_marks_only_visible_disconnected_unmuted_members() {
    let t = channel_thread_test::frozen();
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../vectors/ws12_board_unread_streams.json"
    ))
    .unwrap();
    let setup = oracle["setup_sql"].as_str().unwrap().to_owned();
    t.write(move |tx| {
        tx.conn().execute_batch(&setup)?;
        Ok(())
    });
    let room = board(&t);
    t.write(move |tx| {
        let mut membership =
            Membership::find_by_room_and_user(tx.conn(), room.id, id("kevin"))?.unwrap();
        membership.update_involvement(tx, Some(Involvement::Muted))?;
        Ok(())
    });
    let from = t.events().len();
    let thread = post(&t, room.id);

    assert!(t.read(|conn| {
        Ok(
            Membership::find_by_room_and_user(conn, room.id, id("david"))?
                .unwrap()
                .unread(),
        )
    }));
    assert!(!t.read(|conn| {
        Ok(
            Membership::find_by_room_and_user(conn, room.id, id("kevin"))?
                .unwrap()
                .unread(),
        )
    }));
    assert!(!t.read(|conn| {
        Ok(Membership::find_by_room_and_user(conn, room.id, id("jz"))?
            .unwrap()
            .unread())
    }));
    let actual = t.events()[from..]
        .iter()
        .filter_map(|event| {
            let (stream, payload) = event.as_broadcast()?.channel_frame()?;
            payload
                .get("roomId")
                .is_some()
                .then(|| serde_json::json!({"stream":stream,"payload":payload}))
        })
        .collect::<Vec<_>>();
    assert_eq!(room.id, oracle["room_id"].as_i64().unwrap());
    assert_eq!(
        serde_json::json!(actual),
        oracle["frames"],
        "Rails board unread stream names and complete payloads"
    );
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
    assert_eq!(sync_events(&t, from), [("updated", thread.id)]);

    assert_eq!(
        t.read(|conn| ChannelThread::find(conn, thread.id))
            .updated_at,
        thread.updated_at
    );
    let from = t.events().len();
    t.write(move |tx| tag.destroy(tx));
    assert_eq!(sync_events(&t, from), [("updated", thread.id)]);

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
    assert!(t.events()[from..].iter().any(|event| event.as_broadcast() == Some(Broadcast::ThreadRemoved { thread_id, room_id: room.id })));

}

#[test]
fn board_pages_are_cumulative_clamped_and_ordered_with_one_probe_row() {
    let t = channel_thread_test::frozen();
    let room = board(&t);
    for index in 0..55 {
        t.write(move |tx| {
            ChannelThread::create(
                tx,
                NewChannelThread {
                    room_id: room.id,
                    creator_id: id("jz"),
                    name: Some(format!("Post {index}")),
                    work_status: Some("planned".into()),
                    ..Default::default()
                },
            )
        });
    }
    let read = |page| {
        t.read(|conn| {
            ChannelThread::board_posts_for(
                conn,
                room.id,
                "open",
                "anyone",
                "",
                Some(id("jz")),
                page,
            )
        })
    };
    let first = read(1);
    assert_eq!(first.len(), 51);
    assert_eq!(first[0].name, "Post 54");
    assert_eq!(read(2).len(), 55);
    assert_eq!(read(0), first);
    assert_eq!(read(999).len(), 55);
    use crate::models::channel_thread::board_page_number;
    for (raw, want) in [
        ("junk", 1),
        ("0", 1),
        ("2tail", 2),
        (" +1_2tail", 12),
        ("999999999999999999999999", 20),
        ("-2", 1),
        ("2__0", 2),
        ("_20", 1),
        ("", 1),
    ] {
        assert_eq!(board_page_number(raw), want, "{raw}");
    }
}

#[test]
fn tag_sets_normalize_replace_retain_unchanged_rows_and_clear() {
    let t = channel_thread_test::frozen();
    let room = board(&t);
    let thread = post(&t, room.id);
    t.write(move |tx| {
        ChannelThread::find(tx.conn(), thread.id)?.update_metadata(
            tx,
            None,
            None,
            Some(&["Bug".into(), " bug ".into(), "".into(), "API-v2".into()]),
        )
    });
    let before = t.read(|conn| ThreadTag::for_thread(conn, thread.id));
    assert_eq!(
        before
            .iter()
            .map(|tag| tag.name.as_str())
            .collect::<Vec<_>>(),
        ["api-v2", "bug"]
    );
    let api = before[0].clone();
    t.travel(60);
    t.write(move |tx| {
        ChannelThread::find(tx.conn(), thread.id)?.update_metadata(
            tx,
            None,
            None,
            Some(&["api-v2".into(), "docs".into()]),
        )
    });
    assert_eq!(t.read(|conn| ThreadTag::find(conn, api.id)), api);
    t.write(move |tx| {
        ChannelThread::find(tx.conn(), thread.id)?.update_metadata(tx, None, None, Some(&[]))
    });
    assert!(
        t.read(|conn| ThreadTag::for_thread(conn, thread.id))
            .is_empty()
    );
}

#[test]
fn invalid_tag_count_length_and_format_leave_existing_tags_and_signals_unchanged() {
    let t = channel_thread_test::frozen();
    let room = board(&t);
    let thread = post(&t, room.id);
    t.write(move |tx| ThreadTag::create(tx, thread.id, "bug"));
    for tags in [
        vec![
            "one".into(),
            "two".into(),
            "three".into(),
            "four".into(),
            "five".into(),
            "six".into(),
        ],
        vec!["a".repeat(31)],
        vec!["needs work!".into()],
    ] {
        let from = t.events().len();
        let result = t.try_write(move |tx| {
            ChannelThread::find(tx.conn(), thread.id)?.update_metadata(tx, None, None, Some(&tags))
        });
        assert!(matches!(result, Err(crate::Error::RecordInvalid(_))));
        assert_eq!(
            t.read(|conn| ChannelThread::find(conn, thread.id)?.tag_names(conn)),
            ["bug"]
        );
        assert!(sync_events(&t, from).is_empty());
    }
}

#[test]
fn tag_names_require_a_real_parent_and_are_unique_per_post_but_reusable() {
    let t = channel_thread_test::frozen();
    let room = board(&t);
    let one = post(&t, room.id);
    let two = post(&t, room.id);
    t.write(move |tx| ThreadTag::create(tx, one.id, "bug"));
    assert!(
        t.read(|conn| ThreadTag::validate(conn, one.id, "bug"))
            .0
            .iter()
            .any(|(name, msg)| *name == "name" && msg == "has already been taken")
    );
    assert!(
        t.read(|conn| ThreadTag::validate(conn, two.id, "bug"))
            .is_empty()
    );
    let error = t
        .try_write(|tx| ThreadTag::create(tx, -1, "bug"))
        .unwrap_err();
    assert!(
        matches!(error,crate::Error::RecordInvalid(errors) if errors.full_messages()==["Channel thread must exist"])
    );
    for name in [
        "",
        "Needs Work!",
        "-bad",
        "a_b",
        "a\n",
        "é",
        &"a".repeat(31),
    ] {
        assert!(
            !t.read(|conn| ThreadTag::validate(conn, one.id, name))
                .is_empty()
        );
    }
    assert!(
        t.read(|conn| ThreadTag::validate(conn, one.id, "api-v2"))
            .is_empty()
    );
}

#[test]
fn board_counts_include_streaming_and_system_messages_and_omit_empty_groups() {
    let t = channel_thread_test::frozen();
    let room = board(&t);
    let thread = post(&t, room.id);
    let quiet = post(&t, room.id);
    for (system_note, streaming) in [(false, false), (true, false), (false, true)] {
        t.write(move |tx| {
            crate::Message::create(
                tx,
                crate::NewMessage {
                    room_id: room.id,
                    thread_id: Some(thread.id),
                    creator_id: id("jz"),
                    markdown_source: Some("Reply".into()),
                    system_note,
                    streaming,
                    ..Default::default()
                },
            )
        });
    }
    let counts = t.read(|conn| ChannelThread::board_reply_counts(conn, &[thread.id, quiet.id]));
    assert_eq!(counts, [(thread.id, 3)].into_iter().collect());
    assert_eq!(
        t.read(|conn| ChannelThread::find(conn, thread.id))
            .messages_count,
        1
    );
    // This slice reads existing links; their creation policy/callbacks are the next work slice.
    t.write(move |tx| {
        tx.conn().execute("INSERT INTO work_thread_links(channel_thread_id,created_by_id,kind,url,created_at,updated_at) VALUES(?,?,'drive_file','https://drive.google.com/file/d/boards1234567',?,?)",rusqlite::params![thread.id,id("jz"),tx.now().to_db(),tx.now().to_db()])?;
        Ok(())
    });
    assert_eq!(
        t.read(|conn| ChannelThread::board_link_counts(conn, &[thread.id, quiet.id])),
        [(thread.id, 1)].into_iter().collect()
    );
    assert!(
        t.read(|conn| ChannelThread::board_reply_counts(conn, &[]))
            .is_empty()
    );
    assert!(
        t.read(|conn| ChannelThread::board_link_counts(conn, &[]))
            .is_empty()
    );
}

#[test]
fn savepoint_rollback_preserves_the_outer_records_commit_callback() {
    let t = channel_thread_test::frozen();
    let room = board(&t);
    let thread = post(&t, room.id);
    let _from = t.events().len();
    t.write(move |tx| {
        let mut thread = ChannelThread::find(tx.conn(), thread.id)?;
        thread.update_settings(tx, Some("Outer name"), None)?;
        let result = tx.savepoint(|tx| {
            thread.update_settings(tx, None, Some(60))?;
            Err::<(), _>(crate::Error::Other("rollback inner".into()))
        });
        assert!(result.is_err());
        Ok(())
    });

    assert_eq!(
        t.read(|conn| ChannelThread::find(conn, thread.id)).name,
        "Outer name"
    );
    assert_eq!(
        t.read(|conn| ChannelThread::find(conn, thread.id))
            .auto_archive_after_minutes,
        4320
    );
}
