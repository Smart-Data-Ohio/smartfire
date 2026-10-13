use crate::models::{
    agent_context,
    agent_delivery::{AgentEvent, NewEvent},
    agent_event_access, message_quote, room_files,
    search_query::SearchQuery,
};
use crate::tests::{TestDb, id};
use crate::{
    ActivityItem, ChannelThread, Message, NewChannelThread, NewMessage, Room, RoomType, SavedItem,
    ThreadMembership, User,
};
use rusqlite::params;
use serde_json::json;

fn fixture() -> (TestDb, i64, i64, i64) {
    let db = TestDb::new();
    let (room, thread, message) = db.write(|tx| {
        tx.conn().execute("DELETE FROM agent_grants", [])?;
        let room = Room::create_for(tx, RoomType::Open, Some("Reader privacy"), id("david"), &[id("david"), id("kevin"), id("bender")])?;
        let thread = ChannelThread::create(tx, NewChannelThread { room_id: room.id, creator_id: id("david"), name: Some("readerprivacy".into()), work_status: Some("planned".into()), visibility: crate::channel_thread::ThreadVisibility::Private, ..Default::default() })?;
        let message = Message::create(tx, NewMessage { room_id: room.id, thread_id: Some(thread.id), creator_id: id("david"), markdown_source: Some("readerprivacy".into()), ..Default::default() })?;
        thread.add_access_member(tx, id("david"), id("kevin"))?;
        ThreadMembership::join(tx, thread.id, id("kevin"))?;
        SavedItem::save_for(tx, id("kevin"), message.id, None)?;
        ActivityItem::refresh_unread(tx, id("kevin"), "Message", message.id, "mention")?;
        AgentEvent::create(tx, NewEvent { agent_id: id("bender_agent"), room_id: Some(room.id), message_id: Some(message.id), event_type: "mention".into(), outcome: Some("pending".into()), metadata: json!({"thread_id": thread.id}), ..Default::default() })?;
        tx.conn().execute("INSERT INTO active_storage_blobs(key,filename,content_type,metadata,service_name,byte_size,created_at) VALUES('private-reader-file','secret.txt','text/plain','{}','local',1,?)", [tx.now()])?;
        let blob = tx.conn().last_insert_rowid();
        tx.conn().execute("INSERT INTO active_storage_attachments(name,record_type,record_id,blob_id,created_at) VALUES('attachment','Message',?,?,?)", params![message.id, blob, tx.now()])?;
        thread.remove_access_member(tx, id("david"), id("kevin"))?;
        Ok((room.id, thread.id, message.id))
    });
    (db, room, thread, message)
}

fn grant(db: &TestDb, thread: i64, user: i64) {
    db.write(move |tx| {
        tx.conn().execute("INSERT INTO thread_access_memberships(thread_id,user_id,added_by_id,created_at) VALUES(?,?,?,?)", params![thread, user, id("david"), tx.now()])?;
        Ok(())
    });
}

#[test]
fn private_thread_followers_do_not_reveal_search_saved_activity_or_files() {
    let (db, room, thread, message) = fixture();
    let read = |visible: bool| {
        db.read(|conn| {
            let viewer = User::find(conn, id("kevin"))?;
            let query = SearchQuery::parse("readerprivacy");
            let hits = query
                .messages_for_user(conn, viewer.id, jiff::tz::TimeZone::UTC, None)?
                .messages;
            assert_eq!(hits.iter().any(|row| row.id == message), visible);
            assert_eq!(
                query
                    .sections_for_user(conn, viewer.id)?
                    .iter()
                    .any(|section| section.records.iter().any(|row| row.id == thread)),
                visible
            );
            assert_eq!(
                SavedItem::accessible_to(conn, viewer.id)?
                    .iter()
                    .any(|row| row.message_id == message),
                visible
            );
            assert_eq!(
                ActivityItem::accessible_to(conn, &viewer)?
                    .iter()
                    .any(|row| row.source_id == message),
                visible
            );
            assert_eq!(
                ActivityItem::unread_count(conn, &viewer, db.now())?,
                i64::from(visible)
            );
            assert_eq!(
                crate::ChannelThread::find(conn, thread)?.work_viewable_by(conn, &viewer)?,
                visible
            );
            assert_eq!(
                crate::ChannelThread::visible_work_threads(conn, &viewer, "all")?
                    .iter()
                    .any(|row| row.id == thread),
                visible
            );
            assert_eq!(
                room_files::uploads_for_viewer(conn, room, viewer.id, "all", "", 10)?
                    .iter()
                    .any(|row| row.message_id == message),
                visible
            );
            assert_eq!(
                message_quote::visible(conn, &Message::find(conn, message)?, viewer.id)?,
                visible
            );
            Ok(())
        })
    };
    read(false);
    grant(&db, thread, id("kevin"));
    read(true);
    db.write(move |tx| {
        tx.conn().execute(
            "DELETE FROM thread_access_memberships WHERE thread_id=? AND user_id=?",
            params![thread, id("kevin")],
        )?;
        Ok(())
    });
    read(false);
}

#[test]
fn private_thread_agent_context_and_polling_require_explicit_access() {
    let (db, _, thread, message) = fixture();
    let read = |visible: bool| {
        db.read(|conn| {
            let context = agent_context::build(
                conn,
                id("bender_agent"),
                Some(message),
                None,
                None,
                db.now(),
                |record| Ok(json!({"id":record.id})),
            )?;
            assert_eq!(context.status, if visible { 200 } else { 404 });
            let events = agent_event_access::readable_page(conn, id("bender_agent"), 0, Some(100))?;
            assert_eq!(
                events.iter().any(|event| event.message_id == Some(message)),
                visible
            );
            Ok(())
        })
    };
    read(false);
    grant(&db, thread, id("bender"));
    read(true);
    db.write(move |tx| {
        tx.conn().execute(
            "DELETE FROM thread_access_memberships WHERE thread_id=? AND user_id=?",
            params![thread, id("bender")],
        )?;
        Ok(())
    });
    read(false);
}

#[test]
fn private_thread_event_pages_keep_public_deleted_room_events_and_filter_before_limit() {
    let (db, room, private, message) = fixture();
    grant(&db, private, id("bender"));
    db.write(move |tx| {
        tx.conn().execute("DELETE FROM agent_events", [])?;
        let public = ChannelThread::create(
            tx,
            NewChannelThread {
                room_id: room,
                creator_id: id("david"),
                name: Some("Public event context".into()),
                ..Default::default()
            },
        )?;
        let public_message = Message::create(
            tx,
            NewMessage {
                room_id: room,
                thread_id: Some(public.id),
                creator_id: id("david"),
                markdown_source: Some("Public event message".into()),
                ..Default::default()
            },
        )?;
        let mut pairs = Vec::new();
        for kind in ["mention", "work_assigned", "slash_command"] {
            let mut ids = Vec::new();
            for (thread, source, visibility) in [
                (private, message, "private"),
                (public.id, public_message.id, "public"),
            ] {
                ids.push(
                    AgentEvent::create(
                        tx,
                        NewEvent {
                            agent_id: id("bender_agent"),
                            room_id: Some(room),
                            message_id: (kind == "mention").then_some(source),
                            event_type: kind.into(),
                            outcome: Some("delivered".into()),
                            metadata: json!({"thread_id": thread, "thread_visibility": visibility}),
                            ..Default::default()
                        },
                    )?
                    .id,
                );
            }
            pairs.push((ids[0], ids[1]));
        }
        for &(hidden, _) in &pairs {
            assert_eq!(
                agent_event_access::readable_page(
                    tx.conn(),
                    id("bender_agent"),
                    hidden - 1,
                    Some(1)
                )?[0]
                    .id,
                hidden
            );
        }
        tx.conn().execute(
            "UPDATE rooms SET deleted_at=? WHERE id=?",
            params![tx.now(), room],
        )?;
        for &(hidden, visible) in &pairs {
            let page = agent_event_access::readable_page(
                tx.conn(),
                id("bender_agent"),
                hidden - 1,
                Some(1),
            )?;
            assert_eq!(
                page.iter().map(|event| event.id).collect::<Vec<_>>(),
                [visible]
            );
        }
        tx.conn()
            .execute("UPDATE rooms SET deleted_at=NULL WHERE id=?", [room])?;
        let mut read = crate::AgentGrant::create(
            tx,
            crate::NewGrant {
                agent_id: id("bender_agent"),
                room_id: Some(room),
                granted_by_id: id("david"),
                capability: "read_messages".into(),
                ..Default::default()
            },
        )?;
        tx.conn().execute(
            "UPDATE rooms SET deleted_at=? WHERE id=?",
            params![tx.now(), room],
        )?;
        assert!(!crate::models::agent_access::capability_for_agent(
            tx.conn(),
            id("bender_agent"),
            "read_messages",
            Some(room)
        )?);
        for &(hidden, visible) in &pairs {
            let page = agent_event_access::readable_page(
                tx.conn(),
                id("bender_agent"),
                hidden - 1,
                Some(1),
            )?;
            assert_eq!(
                page.iter().map(|event| event.id).collect::<Vec<_>>(),
                [visible]
            );
        }
        tx.conn().execute(
            "UPDATE agents SET kind='personal',owner_id=? WHERE id=?",
            params![id("david"), id("bender_agent")],
        )?;
        assert!(
            agent_event_access::readable_page(tx.conn(), id("bender_agent"), 0, None)?.is_empty()
        );
        assert!(matches!(
            agent_event_access::acknowledge(tx, id("bender_agent"), pairs[0].1)?,
            agent_event_access::Acknowledgment::Forbidden
        ));
        tx.conn().execute(
            "UPDATE agents SET kind='workspace' WHERE id=?",
            [id("bender_agent")],
        )?;
        read.revoke(tx)?;
        assert!(
            agent_event_access::readable_page(tx.conn(), id("bender_agent"), 0, None)?.is_empty()
        );
        let history = agent_event_access::history_page_for_viewer(
            tx.conn(),
            id("bender_agent"),
            id("bender"),
            id("david"),
            None,
            None,
        )?;
        assert_eq!(
            history.iter().map(|event| event.id).collect::<Vec<_>>(),
            pairs
                .iter()
                .rev()
                .map(|&(_, visible)| visible)
                .collect::<Vec<_>>()
        );
        Ok(())
    });
}
