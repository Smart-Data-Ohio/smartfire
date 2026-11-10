use super::{TestDb, id};
use crate::models::announcement::{self, SyncJob};
use crate::{Event, Message, MessageChanges, NewMessage, Room};

fn setup() -> (TestDb, i64, i64, i64, Message) {
    let t = TestDb::new();
    let (source, first, second) = (id("designers"), id("watercooler"), id("hq"));
    let message = t.write(move |tx| {
        announcement::set_flag(tx, source, true)?;
        announcement::follow(tx, source, first, id("david"))?;
        announcement::follow(tx, source, second, id("david"))?;
        Message::create(
            tx,
            NewMessage {
                room_id: source,
                creator_id: id("david"),
                body: Some("<p>News</p>".into()),
                ..Default::default()
            },
        )
    });
    (t, source, first, second, message)
}

#[test]
fn announcement_delivery_snapshots_followers_and_duplicate_jobs_do_not_double_post() {
    let (t, source, first, second, message) = setup();
    let publication = t.write(move |tx| announcement::publish(tx, &message, id("david")));
    t.write(move |tx| announcement::follow(tx, source, id("pets"), id("david")));
    for _ in 0..2 {
        t.write(move |tx| announcement::deliver(tx, publication));
    }
    let copies = t.read(move |c| announcement::copies(c, publication));
    assert_eq!(copies.len(), 2);
    let mut expected = vec![first, second];
    expected.sort();
    assert_eq!(
        copies.iter().map(|m| m.room_id).collect::<Vec<_>>(),
        expected
    );
    for copy in copies {
        assert_eq!(copy.creator_id, id("david"));
        assert_eq!(
            t.read(move |c| copy.body_html(c)),
            Some("<p>News</p>".into())
        );
    }
    assert_eq!(
        t.sink
            .events()
            .iter()
            .filter(|event| event.as_job::<SyncJob>().is_some())
            .count(),
        1
    );
}

#[test]
fn announcement_edits_and_deletes_enqueue_and_propagate_once() {
    let (t, _, _, _, mut message) = setup();
    let original = message.clone();
    let source_id = original.id;
    let publication = t.write(move |tx| announcement::publish(tx, &original, id("david")));
    t.write(move |tx| announcement::deliver(tx, publication));
    t.write(move |tx| {
        message.edit(
            tx,
            MessageChanges {
                body: Some("<p>Updated</p>".into()),
                ..Default::default()
            },
        )
    });
    for _ in 0..2 {
        t.write(move |tx| announcement::deliver(tx, publication));
    }
    let copies = t.read(move |c| announcement::copies(c, publication));
    assert_eq!(copies.len(), 2);
    for copy in copies.clone() {
        assert_eq!(
            t.read(move |c| copy.body_html(c)),
            Some("<p>Updated</p>".into())
        );
    }
    assert_eq!(
        t.sink
            .events()
            .iter()
            .filter(|event| event.as_job::<SyncJob>().is_some())
            .count(),
        2
    );
    t.write(move |tx| Message::find(tx.conn(), source_id)?.destroy(tx));
    let job = t
        .sink
        .events()
        .iter()
        .find_map(|event| event.as_job::<announcement::DeleteCopiesJob>())
        .unwrap();
    assert_eq!(job.message_ids.len(), 2);
    for _ in 0..2 {
        let ids = job.message_ids.clone();
        t.write(move |tx| announcement::delete_copies(tx, publication, &ids));
    }
    for copy in copies {
        assert!(t.read(move |c| Message::find_by_id(c, copy.id)).is_none());
    }
    t.write(move |tx| announcement::deliver(tx, publication));
}

#[test]
fn announcement_bodyless_publications_still_propagate_after_room_flag_is_disabled() {
    for mutation in ["update_body", "edit", "destroy"] {
        let (t, source, _, _, _) = setup();
        let message = t.write(move |tx| {
            Message::create(
                tx,
                NewMessage {
                    room_id: source,
                    creator_id: id("david"),
                    ..Default::default()
                },
            )
        });
        let source_id = message.id;
        let publication = t.write(move |tx| announcement::publish(tx, &message, id("david")));
        t.write(move |tx| announcement::deliver(tx, publication));
        let copies = t.read(move |conn| announcement::copies(conn, publication));
        assert_eq!(copies.len(), 2);
        for copy in &copies {
            assert_eq!(t.read(|conn| copy.body_html(conn)), None);
        }
        t.write(move |tx| announcement::set_flag(tx, source, false));
        t.sink.take();
        t.write(move |tx| {
            let mut message = Message::find(tx.conn(), source_id)?;
            match mutation {
                "update_body" => message.update_body(tx, "<p>Updated</p>"),
                "edit" => message.edit(
                    tx,
                    MessageChanges {
                        body: Some("<p>Updated</p>".into()),
                        ..Default::default()
                    },
                ),
                "destroy" => message.destroy(tx),
                _ => unreachable!(),
            }
        });
        let events = t.events();
        if mutation == "destroy" {
            let jobs = events
                .iter()
                .filter_map(|event| event.as_job::<announcement::DeleteCopiesJob>())
                .collect::<Vec<_>>();
            assert_eq!(jobs.len(), 1);
            let job = jobs.into_iter().next().unwrap();
            assert_eq!(job.message_ids.len(), 2);
            t.write(move |tx| {
                announcement::delete_copies(tx, job.publication_id, &job.message_ids)
            });
            for copy in copies {
                assert!(
                    t.read(move |conn| Message::find_by_id(conn, copy.id))
                        .is_none()
                );
            }
        } else {
            let jobs = events
                .iter()
                .filter_map(|event| event.as_job::<SyncJob>())
                .collect::<Vec<_>>();
            assert_eq!(jobs.len(), 1);
            let job = jobs.into_iter().next().unwrap();
            assert_eq!(job.publication_id, publication);
            t.write(move |tx| announcement::deliver(tx, job.publication_id));
            for copy in copies {
                assert_eq!(
                    t.read(move |conn| copy.body_html(conn)),
                    Some("<p>Updated</p>".into())
                );
            }
        }
    }
}

#[test]
fn announcement_unfollow_stops_pending_and_future_deliveries() {
    let (t, source, first, second, message) = setup();
    let original = message.clone();
    let publication = t.write(move |tx| {
        let publication = announcement::publish(tx, &original, id("david"))?;
        announcement::unfollow(tx, source, first)?;
        Ok(publication)
    });
    t.write(move |tx| announcement::deliver(tx, publication));
    assert_eq!(
        t.read(move |c| announcement::copies(c, publication))
            .iter()
            .map(|m| m.room_id)
            .collect::<Vec<_>>(),
        [second]
    );
    let next = t.write(move |tx| {
        Message::create(
            tx,
            NewMessage {
                room_id: source,
                creator_id: id("david"),
                body: Some("Next".into()),
                ..Default::default()
            },
        )
    });
    let publication = t.write(move |tx| announcement::publish(tx, &next, id("david")));
    t.write(move |tx| announcement::deliver(tx, publication));
    assert_eq!(
        t.read(move |c| announcement::copies(c, publication))
            .iter()
            .map(|m| m.room_id)
            .collect::<Vec<_>>(),
        [second]
    );
}

#[test]
fn announcement_room_deletion_cascades_follow_and_delivery_rows() {
    let (t, source, first, _, message) = setup();
    let publication = t.write(move |tx| announcement::publish(tx, &message, id("david")));
    t.write(move |tx| announcement::deliver(tx, publication));
    t.write(move |tx| Room::find(tx.conn(), first)?.destroy(tx));
    assert_eq!(t.read(move |c| announcement::followers(c, source)).len(), 1);
    assert_eq!(
        t.read(move |c| announcement::copies(c, publication)).len(),
        1
    );
    t.write(move |tx| Room::find(tx.conn(), source)?.destroy(tx));
    assert!(
        t.read(move |c| announcement::following(c, first))
            .is_empty()
    );
    let job = t
        .sink
        .events()
        .iter()
        .find_map(|event| event.as_job::<announcement::DeleteCopiesJob>())
        .unwrap();
    t.write(move |tx| announcement::delete_copies(tx, job.publication_id, &job.message_ids));
    assert!(
        t.read(move |c| announcement::copies(c, publication))
            .is_empty()
    );
    assert_eq!(
        t.read(move |c| Ok(c.query_row(
            "SELECT count(*) FROM announcement_publications WHERE id=?",
            [publication],
            |r| r.get::<_, i64>(0)
        )?)),
        0
    );
}

#[test]
fn announcement_follow_refuses_long_cycles_and_direct_rooms_and_copy_publication() {
    let (t, source, first, second, message) = setup();
    t.write(move |tx| {
        announcement::set_flag(tx, first, true)?;
        announcement::set_flag(tx, second, true)?;
        announcement::follow(tx, first, second, id("david"))
    });
    assert!(
        t.try_write(move |tx| announcement::follow(tx, second, source, id("david")))
            .is_err()
    );
    assert!(
        t.try_write(move |tx| announcement::follow(tx, source, source, id("david")))
            .is_err()
    );
    let direct = t.write(|tx| {
        tx.conn().execute(
            "UPDATE rooms SET type='Rooms::Direct' WHERE id=?",
            [id("pets")],
        )?;
        Ok(id("pets"))
    });
    assert!(
        t.try_write(move |tx| announcement::follow(tx, source, direct, id("david")))
            .is_err()
    );
    assert!(
        t.try_write(move |tx| announcement::set_flag(tx, direct, true))
            .is_err()
    );
    let publication = t.write(move |tx| announcement::publish(tx, &message, id("david")));
    t.write(move |tx| announcement::deliver(tx, publication));
    let copy = t
        .read(move |c| announcement::copies(c, publication))
        .remove(0);
    assert!(
        t.try_write(move |tx| announcement::publish(tx, &copy, id("david")))
            .is_err()
    );
}

#[test]
fn announcement_follow_refuses_boards_and_accepts_shared_timeline_rooms() {
    let (t, source, _, _, _) = setup();
    let target = id("pets");
    t.write(move |tx| {
        tx.conn()
            .execute("UPDATE rooms SET type='Rooms::Board' WHERE id=?", [target])?;
        Ok(())
    });
    let error = t
        .try_write(move |tx| announcement::follow(tx, source, target, id("david")))
        .unwrap_err();
    let crate::Error::RecordInvalid(errors) = error else {
        panic!("{error:?}")
    };
    assert_eq!(
        errors.on("targetRoomId"),
        ["must show plain timeline messages; boards are not supported"]
    );
    assert!(
        t.read(move |c| announcement::following(c, target))
            .is_empty()
    );
    for kind in [
        "Rooms::Open",
        "Rooms::Closed",
        "Rooms::Voice",
        "Rooms::Stage",
    ] {
        t.write(move |tx| {
            tx.conn()
                .execute("UPDATE rooms SET type=? WHERE id=?", (kind, target))?;
            announcement::follow(tx, source, target, id("david"))?;
            announcement::unfollow(tx, source, target)
        });
    }
}

#[test]
fn announcement_delivery_skips_legacy_board_follows_and_reserved_board_targets() {
    let (t, source, first, second, message) = setup();
    let target = id("pets");
    let publication = t.write(move |tx| {
        tx.conn().execute("UPDATE rooms SET type='Rooms::Board' WHERE id=?", [first])?;
        announcement::follow(tx, source, target, id("david"))?;
        let publication = announcement::publish(tx, &message, id("david"))?;
        let reserved: i64 = tx.conn().query_row("SELECT count(*) FROM announcement_deliveries WHERE publication_id=? AND target_room_id=?", (publication, first), |r| r.get(0))?;
        assert_eq!(reserved, 0, "legacy board follows must not reserve deliveries");
        tx.conn().execute("UPDATE rooms SET type='Rooms::Board' WHERE id=?", [second])?;
        // A reservation made by an earlier release must also be skipped.
        tx.conn().execute("INSERT INTO announcement_deliveries(publication_id,target_room_id) VALUES (?,?)", (publication, first))?;
        Ok(publication)
    });
    for _ in 0..2 {
        let delivered = t.write(move |tx| announcement::deliver(tx, publication));
        assert_eq!(
            delivered
                .iter()
                .map(|(message, _)| message.room_id)
                .collect::<Vec<_>>(),
            [target]
        );
    }
    let copies = t.read(move |c| announcement::copies(c, publication));
    assert_eq!(
        copies
            .iter()
            .map(|message| message.room_id)
            .collect::<Vec<_>>(),
        [target]
    );
}

#[test]
fn announcement_hidden_source_reference_is_null_for_target_only_members() {
    let (t, source, _, _, message) = setup();
    let original = message.clone();
    let publication = t.write(move |tx| announcement::publish(tx, &original, id("david")));
    t.write(move |tx| announcement::deliver(tx, publication));
    let copy = t
        .read(move |c| announcement::copies(c, publication))
        .remove(0);
    assert_eq!(
        t.read(move |c| announcement::source_for_viewer(c, copy.id, id("david")))
            .unwrap()
            .message_id,
        message.id
    );
    t.write(move |tx| {
        tx.conn().execute(
            "DELETE FROM memberships WHERE room_id=? AND user_id=?",
            (source, id("david")),
        )?;
        Ok(())
    });
    assert!(
        t.read(move |c| announcement::source_for_viewer(c, copy.id, id("david")))
            .is_none()
    );
    assert!(
        t.sink
            .events()
            .iter()
            .any(|event| matches!(event, Event::PushMessage { .. }))
    );
}

#[test]
fn announcement_attachments_share_blobs_and_keep_distinct_attachment_rows() {
    let (t, source, _, _, _) = setup();
    let blob_id = t.write(|tx| {
        Ok(crate::Blob::create(
            tx,
            &crate::Blob {
                id: 0,
                key: "announcement-file".into(),
                filename: "news.txt".into(),
                content_type: Some("text/plain".into()),
                metadata: None,
                service_name: "local".into(),
                byte_size: 4,
                checksum: None,
                created_at: tx.now(),
            },
        )?
        .id)
    });
    let message = t.write(move |tx| {
        Message::create(
            tx,
            NewMessage {
                room_id: source,
                creator_id: id("david"),
                attachment_blob_id: Some(blob_id),
                attachment_spoilers: vec![true],
                ..Default::default()
            },
        )
    });
    let original = message.clone();
    let publication = t.write(move |tx| announcement::publish(tx, &original, id("david")));
    t.write(move |tx| announcement::deliver(tx, publication));
    let source_attachment = t.read(move |c| message.attachments(c)).remove(0).0;
    for copy in t.read(move |c| announcement::copies(c, publication)) {
        let attachment = t.read(move |c| copy.attachments(c)).remove(0).0;
        assert!(attachment.spoiler);
        assert!(source_attachment.spoiler);
        assert_eq!(attachment.blob_id, blob_id);
        assert_ne!(attachment.id, source_attachment.id);
    }
}

#[test]
fn announcement_deleted_source_copies_cannot_be_republished_before_cleanup() {
    let (t, _, first, _, message) = setup();
    let source_id = message.id;
    t.write(move |tx| announcement::set_flag(tx, first, true));
    let publication = t.write(move |tx| announcement::publish(tx, &message, id("david")));
    t.write(move |tx| announcement::deliver(tx, publication));
    let copy = t
        .read(move |c| announcement::copies(c, publication))
        .into_iter()
        .find(|m| m.room_id == first)
        .unwrap();
    t.write(move |tx| Message::find(tx.conn(), source_id)?.destroy(tx));
    assert!(
        t.try_write(move |tx| announcement::publish(tx, &copy, id("david")))
            .is_err()
    );
}

#[test]
fn announcement_forwarded_content_keeps_its_note_and_note_edits() {
    let (t, source, _, _, original) = setup();
    let mut message = t.write(move |tx| {
        Message::create(
            tx,
            NewMessage {
                room_id: source,
                creator_id: id("david"),
                body: Some("<p>Forwarded news</p>".into()),
                forwarded_from_message_id: Some(original.id),
                forwarded_at: Some(tx.now()),
                forward_note: Some("Context".into()),
                ..Default::default()
            },
        )
    });
    let publishing = message.clone();
    let publication = t.write(move |tx| announcement::publish(tx, &publishing, id("david")));
    t.write(move |tx| announcement::deliver(tx, publication));
    for copy in t.read(move |c| announcement::copies(c, publication)) {
        assert_eq!(copy.forward_note.as_deref(), Some("Context"));
    }
    t.write(move |tx| {
        message.edit(
            tx,
            MessageChanges {
                forward_note: Some(Some("Revised context".into())),
                ..Default::default()
            },
        )
    });
    for _ in 0..2 {
        t.write(move |tx| announcement::deliver(tx, publication));
    }
    for copy in t.read(move |c| announcement::copies(c, publication)) {
        assert_eq!(copy.forward_note.as_deref(), Some("Revised context"));
    }
}

#[test]
fn announcement_attachment_only_edits_enqueue_and_propagate() {
    let (t, _, _, _, message) = setup();
    let source_id = message.id;
    let publication = t.write(move |tx| announcement::publish(tx, &message, id("david")));
    t.write(move |tx| announcement::deliver(tx, publication));
    let mut blobs = Vec::new();
    for name in ["original", "replacement"] {
        blobs.push(t.write(move |tx| {
            Ok(crate::Blob::create(
                tx,
                &crate::Blob {
                    id: 0,
                    key: format!("announcement-{name}"),
                    filename: format!("{name}.txt"),
                    content_type: Some("text/plain".into()),
                    metadata: None,
                    service_name: "local".into(),
                    byte_size: 4,
                    checksum: None,
                    created_at: tx.now(),
                },
            )?
            .id)
        }));
    }
    for blob_id in [Some(blobs[0]), Some(blobs[1]), None] {
        t.sink.take();
        t.write(move |tx| {
            let mut source = Message::find(tx.conn(), source_id)?;
            source.replace_attachment(tx, blob_id)?;
            source.edit(tx, MessageChanges::default())
        });
        let jobs: Vec<_> = t
            .sink
            .events()
            .iter()
            .filter_map(|event| event.as_job::<SyncJob>())
            .collect();
        assert_eq!(
            jobs.len(),
            1,
            "an attachment-only edit must enqueue propagation"
        );
        for job in jobs {
            t.write(move |tx| announcement::deliver(tx, job.publication_id));
        }
        for copy in t.read(move |c| announcement::copies(c, publication)) {
            let attachments = t.read(move |c| copy.attachments(c));
            assert_eq!(
                attachments
                    .iter()
                    .map(|(_, blob)| blob.id)
                    .collect::<Vec<_>>(),
                blob_id.into_iter().collect::<Vec<_>>()
            );
        }
    }
    t.sink.take();
    t.write(move |tx| Message::find(tx.conn(), source_id)?.replace_attachment(tx, None));
    assert!(
        !t.sink
            .events()
            .iter()
            .any(|event| event.as_job::<SyncJob>().is_some())
    );
}

#[test]
fn announcement_markdown_edits_preserve_legacy_inline_attachments() {
    const ATTACHMENT: &str = r#"<action-text-attachment content-type="image/png" url="https://example.com/news.png" filename="news.png"></action-text-attachment>"#;
    let (t, _, _, _, message) = setup();
    let source_id = message.id;
    t.write(move |tx| {
        Message::find(tx.conn(), source_id)?.update_body(tx, &format!("<p>News</p>\n{ATTACHMENT}"))
    });
    let publication = t.write(move |tx| {
        announcement::publish(tx, &Message::find(tx.conn(), source_id)?, id("david"))
    });
    t.write(move |tx| announcement::deliver(tx, publication));
    for text in ["Updated news", "Revised news"] {
        t.write(move |tx| {
            Message::find(tx.conn(), source_id)?.edit(
                tx,
                MessageChanges {
                    markdown_source: Some(text.into()),
                    legacy_attachment_snapshot: Some(ATTACHMENT.into()),
                    ..Default::default()
                },
            )
        });
        for _ in 0..2 {
            t.write(move |tx| announcement::deliver(tx, publication));
        }
        let expected = format!("<p>{text}</p>\n{ATTACHMENT}");
        assert_eq!(
            t.read(move |c| Message::find(c, source_id)?.body_html(c))
                .as_deref(),
            Some(expected.as_str())
        );
        for copy in t.read(move |c| announcement::copies(c, publication)) {
            assert_eq!(copy.markdown_source.as_deref(), Some(text));
            assert_eq!(
                t.read(move |c| copy.body_html(c)).as_deref(),
                Some(expected.as_str())
            );
        }
    }
}

#[test]
fn announcement_publication_preserves_legacy_inline_attachments_in_markdown() {
    const ATTACHMENT: &str = r#"<action-text-attachment content-type="image/png" url="https://example.com/news.png" filename="news.png"></action-text-attachment>"#;
    let (t, _, _, _, message) = setup();
    let source_id = message.id;
    t.write(move |tx| {
        Message::find(tx.conn(), source_id)?.edit(
            tx,
            MessageChanges {
                markdown_source: Some("News".into()),
                legacy_attachment_snapshot: Some(ATTACHMENT.into()),
                ..Default::default()
            },
        )
    });
    let publication = t.write(move |tx| {
        announcement::publish(tx, &Message::find(tx.conn(), source_id)?, id("david"))
    });
    let expected = format!("<p>News</p>\n{ATTACHMENT}");
    for _ in 0..2 {
        t.write(move |tx| announcement::deliver(tx, publication));
        for copy in t.read(move |c| announcement::copies(c, publication)) {
            assert_eq!(copy.markdown_source.as_deref(), Some("News"));
            assert_eq!(
                t.read(move |c| copy.body_html(c)).as_deref(),
                Some(expected.as_str())
            );
        }
    }
}
