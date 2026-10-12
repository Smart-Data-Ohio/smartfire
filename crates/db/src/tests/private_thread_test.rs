use super::*;
use crate::models::channel_thread::{ThreadAccessError, ThreadVisibility};
use crate::{
    ChannelThread, Error, Message, NewChannelThread, NewMessage, ThreadInvolvement,
    ThreadMembership,
};

fn private(t: &TestDb, room: &str, creator: &str) -> ChannelThread {
    let attributes = NewChannelThread {
        room_id: id(room),
        creator_id: id(creator),
        name: Some("Private discussion".into()),
        visibility: ThreadVisibility::Private,
        ..Default::default()
    };
    t.write(move |tx| ChannelThread::create(tx, attributes))
}

#[test]
fn private_thread_access_requires_parent_and_explicit_grant_or_administrator() {
    let t = TestDb::new();
    let thread = private(&t, "designers", "jz");
    assert!(t.read(|c| thread.readable_by(c, id("jz"))));
    assert!(t.read(|c| thread.readable_by(c, id("david"))));
    assert!(!t.read(|c| thread.readable_by(c, id("kevin"))));
    assert!(!t.read(|c| thread.readable_by(c, id("bender"))));
    t.write(|tx| {
        tx.conn().execute(
            "DELETE FROM memberships WHERE room_id=? AND user_id=?",
            [id("designers"), id("david")],
        )?;
        Ok(())
    });
    assert!(
        !t.read(|c| thread.readable_by(c, id("david"))),
        "admin still needs parent access"
    );
    let members = t.read(|c| thread.access_members(c));
    assert_eq!(
        members.iter().map(|m| m.user_id).collect::<Vec<_>>(),
        [id("jz")]
    );
}

#[test]
fn public_threads_keep_default_and_visibility_cannot_be_changed() {
    let t = TestDb::new();
    let thread = t.write(|tx| {
        ChannelThread::create(
            tx,
            NewChannelThread {
                room_id: id("designers"),
                creator_id: id("jz"),
                name: Some("Public".into()),
                ..Default::default()
            },
        )
    });
    assert_eq!(thread.visibility, ThreadVisibility::Public);
    assert!(t.read(|c| thread.readable_by(c, id("kevin"))));
    assert!(matches!(
        t.read(|c| Ok(thread.access_members(c).unwrap_err())),
        Error::ThreadAccess(ThreadAccessError::NotPrivate)
    ));
    let thread_id = thread.id;
    assert!(
        t.try_write(move |tx| {
            tx.conn().execute(
                "UPDATE channel_threads SET visibility='private' WHERE id=?",
                [thread_id],
            )?;
            Ok(())
        })
        .is_err()
    );
    assert_eq!(
        t.read(|c| ChannelThread::find(c, thread_id)).visibility,
        ThreadVisibility::Public
    );
}

#[test]
fn invitation_roles_parent_access_and_self_leave_are_enforced() {
    let t = TestDb::new();
    let thread = private(&t, "designers", "jz");
    let thread_id = thread.id;
    let add = |actor, target| {
        t.try_write(move |tx| {
            ChannelThread::find(tx.conn(), thread_id)?.add_access_member(tx, id(actor), id(target))
        })
    };
    assert!(matches!(
        add("kevin", "kevin"),
        Err(Error::ThreadAccess(ThreadAccessError::InviteForbidden))
    ));
    assert!(matches!(
        add("jz", "bender"),
        Err(Error::ThreadAccess(ThreadAccessError::ParentRoomRequired))
    ));
    add("jz", "kevin").unwrap();
    add("kevin", "jason").unwrap();
    add("kevin", "jason").unwrap();
    assert_eq!(
        t.read(|c| thread.access_members(c)).len(),
        3,
        "invitations are idempotent"
    );
    let remove = |actor, target| {
        t.try_write(move |tx| {
            ChannelThread::find(tx.conn(), thread_id)?.remove_access_member(
                tx,
                id(actor),
                id(target),
            )
        })
    };
    assert!(matches!(
        remove("kevin", "jz"),
        Err(Error::ThreadAccess(ThreadAccessError::RemoveForbidden))
    ));
    remove("kevin", "kevin").unwrap();
    assert!(!t.read(|c| thread.readable_by(c, id("kevin"))));
    add("david", "kevin").unwrap();
    remove("jz", "kevin").unwrap();
    remove("david", "jason").unwrap();
    remove("jz", "jz").unwrap();
    assert!(
        !t.read(|c| thread.readable_by(c, id("jz"))),
        "creator loses access on leaving"
    );
}

#[test]
fn notification_memberships_never_grant_access_and_unfollowing_preserves_it() {
    let t = TestDb::new();
    let thread = private(&t, "designers", "jz");
    let thread_id = thread.id;
    assert!(
        t.try_write(move |tx| ThreadMembership::join(tx, thread_id, id("kevin")))
            .is_err()
    );
    assert!(
        t.try_write(move |tx| ThreadMembership::create(
            tx,
            thread_id,
            id("kevin"),
            ThreadInvolvement::Everything
        ))
        .is_err()
    );
    t.write(move |tx| {
        ChannelThread::find(tx.conn(), thread_id)?.add_access_member(tx, id("jz"), id("kevin"))?;
        let mut membership = ThreadMembership::join(tx, thread_id, id("kevin"))?;
        membership.update_involvement(tx, ThreadInvolvement::Nothing)?;
        Ok(())
    });
    assert!(t.read(|c| thread.readable_by(c, id("kevin"))));
    t.write(move |tx| {
        tx.conn().execute(
            "DELETE FROM thread_memberships WHERE thread_id=? AND user_id=?",
            [thread_id, id("kevin")],
        )?;
        Ok(())
    });
    assert!(t.read(|c| thread.readable_by(c, id("kevin"))));
}

#[test]
fn public_thread_followers_keep_historical_authors_without_granting_private_access() {
    let t = TestDb::new();
    let private = private(&t, "designers", "jz");
    t.write(move |tx| {
        private.add_access_member(tx, id("jz"), id("kevin"))?;
        let public = ChannelThread::create(tx, NewChannelThread {
            room_id: id("designers"), creator_id: id("jz"), name: Some("Imported public thread".into()),
            ..Default::default()
        })?;
        tx.conn().execute("UPDATE users SET status=2 WHERE id=?", [id("kevin")])?;
        let member = ThreadMembership::join(tx, public.id, id("kevin"))?;
        assert_eq!(member.thread_id, public.id);
        assert_eq!(member.user_id, id("kevin"));
        assert!(!public.readable_by(tx.conn(), id("kevin"))?);
        assert!(ThreadMembership::join(tx, private.id, id("kevin")).is_err());
        assert!(!ThreadMembership::validate(tx.conn(), private.id, id("kevin"))?.is_empty());
        Ok(())
    });
}

#[test]
fn mentions_invite_before_delivery_and_removal_filters_followers_unread_push_and_messages() {
    let t = TestDb::new();
    let thread = private(&t, "designers", "jz");
    let thread_id = thread.id;
    let first = t.write(move |tx| {
        ChannelThread::find(tx.conn(), thread_id)?.post_message(
            tx,
            id("jz"),
            NewMessage {
                markdown_source: Some("Hello @[Kevin]".into()),
                ..Default::default()
            },
        )
    });
    assert!(t.read(|c| thread.readable_by(c, id("kevin"))));
    t.write(move |tx| {
        ThreadMembership::join(tx, thread_id, id("kevin"))?;
        Ok(())
    });
    t.write(move |tx| {
        ChannelThread::find(tx.conn(), thread_id)?.remove_access_member(tx, id("jz"), id("kevin"))
    });
    let second = t.write(move |tx| {
        ChannelThread::find(tx.conn(), thread_id)?.post_message(
            tx,
            id("jz"),
            NewMessage {
                markdown_source: Some("Confidential followup".into()),
                ..Default::default()
            },
        )
    });
    assert!(
        t.read(|c| ChannelThread::followed_by(c, id("kevin")))
            .is_empty()
    );
    assert!(
        t.read(|c| ThreadMembership::unread_for_user(c, id("kevin")))
            .is_empty()
    );
    assert!(t.read(|c| Ok(Message::find_reachable(c, id("kevin"), first.id).is_err())));
    let candidates =
        t.read(|c| ChannelThread::push_candidates(c, &BasicRichText, &thread, &second));
    assert!(
        !candidates
            .iter()
            .any(|candidate| candidate.recipient.id == id("kevin"))
    );
    assert!(
        t.try_write(
            move |tx| ChannelThread::find(tx.conn(), thread_id)?.post_message(
                tx,
                id("kevin"),
                NewMessage {
                    markdown_source: Some("Denied".into()),
                    ..Default::default()
                }
            )
        )
        .is_err()
    );
}

#[test]
fn bots_need_explicit_access_and_private_deletion_targets_only_readers() {
    let t = TestDb::new();
    let thread = private(&t, "watercooler", "david");
    assert!(!t.read(|c| thread.readable_by(c, id("bender"))));
    let thread_id = thread.id;
    t.write(move |tx| {
        ChannelThread::find(tx.conn(), thread_id)?.add_access_member(
            tx,
            id("david"),
            id("bender"),
        )?;
        Ok(())
    });
    assert!(t.read(|c| thread.readable_by(c, id("bender"))));
    let from = t.events().len();
    t.write(move |tx| ChannelThread::find(tx.conn(), thread_id)?.destroy(tx));
    let changes: Vec<crate::channel_thread::ThreadAccessChange> = t.events()[from..]
        .iter()
        .filter_map(|event| match event {
            crate::Event::Broadcast(request) => request.decode().and_then(Result::ok),
            _ => None,
        })
        .collect();
    assert!(
        changes
            .iter()
            .any(|change| change.user_id == id("bender") && !change.added)
    );
    assert_eq!(
        t.read(|c| Ok(c.query_row(
            "SELECT COUNT(*) FROM thread_access_memberships WHERE thread_id=?",
            [thread_id],
            |row| row.get::<_, i64>(0)
        )?)),
        0
    );
}

#[test]
fn private_thread_review_stream_mentions_invite_at_finalize_before_notifications() {
    use crate::models::{agent_posting::PostResult, agent_streaming};
    let t = TestDb::new();
    let thread = private(&t, "designers", "jz");
    let thread_id = thread.id;
    let (message_id, receiver_id, receiver_agent_id) = t.write(move |tx| {
        let receiver = crate::User::create_bot(tx, "Stream Receiver", None)?;
        let receiver_agent = crate::Agent::create(
            tx,
            crate::NewAgent {
                user_id: receiver.id,
                owner_id: Some(id("david")),
                ..Default::default()
            },
        )?;
        crate::Room::find(tx.conn(), id("designers"))?
            .grant_to(tx, &[id("bender"), receiver.id])?;
        ChannelThread::find(tx.conn(), thread_id)?.add_access_member(
            tx,
            id("david"),
            id("bender"),
        )?;
        let PostResult::Posted(message) = agent_streaming::start(
            tx,
            id("bender_agent"),
            NewMessage {
                room_id: id("designers"),
                thread_id: Some(thread_id),
                client_message_id: Some("private-stream-mentions".into()),
                markdown_source: Some("Draft".into()),
                ..Default::default()
            },
        )?
        else {
            panic!("stream should start");
        };
        agent_streaming::update(
            tx,
            id("bender_agent"),
            message.id,
            Some(" @[Kevin] @[Stream Receiver]"),
            None,
        )?;
        Ok((message.id, receiver.id, receiver_agent.id))
    });
    assert!(!t.read(|c| thread.readable_by(c, id("kevin"))));
    assert!(!t.read(|c| thread.readable_by(c, receiver_id)));
    t.write(move |tx| agent_streaming::finalize(tx, id("bender_agent"), message_id));
    assert!(t.read(|c| thread.readable_by(c, id("kevin"))));
    assert!(t.read(|c| thread.readable_by(c, receiver_id)));
    assert_eq!(t.read(|c| Ok(c.query_row(
        "SELECT COUNT(*) FROM agent_events WHERE agent_id=? AND message_id=? AND event_type='mention' AND outcome='pending'",
        [receiver_agent_id, message_id], |row| row.get::<_, i64>(0))?)), 1);
    assert_eq!(t.read(|c| Ok(c.query_row(
        "SELECT COUNT(*) FROM activity_items WHERE user_id=? AND source_type='Message' AND source_id=? AND event_type='mention'",
        [id("kevin"), message_id], |row| row.get::<_, i64>(0))?)), 1);
    assert!(
        t.read(|c| ChannelThread::push_candidates(
            c,
            &BasicRichText,
            &thread,
            &Message::find(c, message_id)?
        ))
        .iter()
        .any(|candidate| candidate.recipient.id == id("kevin"))
    );
    t.write(move |tx| agent_streaming::finalize(tx, id("bender_agent"), message_id));
    assert_eq!(t.read(|c| Ok(c.query_row(
        "SELECT COUNT(*) FROM agent_events WHERE agent_id=? AND message_id=? AND event_type='mention'",
        [receiver_agent_id, message_id], |row| row.get::<_, i64>(0))?)), 1);
    assert_eq!(
        t.read(|c| thread.access_members(c))
            .iter()
            .filter(|member| member.user_id == id("kevin"))
            .count(),
        1
    );
}

#[test]
fn private_thread_review_edits_invite_new_mentionees_before_activity() {
    let t = TestDb::new();
    let thread = private(&t, "designers", "jz");
    let thread_id = thread.id;
    let mut message = t.write(move |tx| {
        ChannelThread::find(tx.conn(), thread_id)?.post_message(
            tx,
            id("jz"),
            NewMessage {
                markdown_source: Some("Draft".into()),
                ..Default::default()
            },
        )
    });
    let message_id = message.id;
    t.write(move |tx| {
        message.update(
            tx,
            crate::MessageChanges {
                markdown_source: Some("Hello @[Kevin]".into()),
                ..Default::default()
            },
        )
    });
    assert!(t.read(|c| thread.readable_by(c, id("kevin"))));
    assert_eq!(t.read(|c| Ok(c.query_row(
        "SELECT COUNT(*) FROM activity_items WHERE user_id=? AND source_type='Message' AND source_id=? AND event_type='mention'",
        [id("kevin"), message_id], |row| row.get::<_, i64>(0))?)), 1);
}
