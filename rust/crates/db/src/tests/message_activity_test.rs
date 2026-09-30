//! Named Rails message/keyword recorder sequences over real fixtures and SQLite.
use super::*;
use crate::{ActivityItem, KeywordAlert, Message, NewMessage};
#[test]
fn ws17_keyword_room_records_matching_keyword() {
    let t = TestDb::new();
    let message = t.write(|tx| {
        KeywordAlert::create(tx, id("david"), "deploy")?;
        Message::create(
            tx,
            NewMessage {
                room_id: id("designers"),
                creator_id: id("jz"),
                body: Some("Shipping the DEPLOY now".into()),
                ..Default::default()
            },
        )
    });
    assert_eq!(
        t.read(move |c| ActivityItem::find_by_user_and_source(
            c,
            id("david"),
            "Message",
            message.id
        ))
        .unwrap()
        .event_type,
        "keyword_alert"
    );
}

#[test]
fn ws17_message_activity_matches_actual_rails_callbacks_and_scoped_candidates() {
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("ws17_message_activity.json")).unwrap();
    for row in vectors["rows"].as_array().unwrap() {
        let t = TestDb::new();
        let row = row.clone();
        let name = row["name"].as_str().unwrap().to_string();
        let expected = row["event_type"].as_str().map(str::to_string);
        let expected_candidates = row["candidate_ids"].clone();
        let (message, members)=t.write(move|tx| {
            let recipient=id("david");let author=id("jz");let room=id("designers");
            let status=if row["inactive"].as_bool()==Some(true) {1} else {0};let role=if row["bot"].as_bool()==Some(true) {2} else {0};
            tx.conn().execute("UPDATE users SET status=?,role=?,dnd_enabled=?,quiet_hours_enabled=?,quiet_hours_start_minute=900,quiet_hours_end_minute=1020,time_zone='UTC',ooo_until=? WHERE id=?",rusqlite::params![status,role,row["attrs"]["dnd_enabled"].as_bool().unwrap_or(false),row["attrs"]["quiet_hours_enabled"].as_bool().unwrap_or(false),row["attrs"]["ooo_until"].as_str(),recipient])?;
            tx.conn().execute("UPDATE memberships SET involvement=? WHERE room_id=? AND user_id=?",rusqlite::params![if row["involvement"].as_str()==Some("missing") {Some("mentions")} else if row.get("involvement").is_some() {row["involvement"].as_str()} else {Some("mentions")},room,recipient])?;
            KeywordAlert::create(tx,recipient,"deploy")?;
            let thread=if let Some(involvement)=row["thread_involvement"].as_str() {
                let thread=crate::ChannelThread::create(tx,crate::NewChannelThread {room_id:room,creator_id:author,name:Some("Activity thread".into()),..Default::default()})?;
                crate::ThreadMembership::join(tx,thread.id,author)?;
                if involvement!="missing" {crate::ThreadMembership::join(tx,thread.id,recipient)?.update_involvement(tx,crate::ThreadInvolvement::from_name(involvement).unwrap())?;}
                Some(thread.id)
            } else {None};
            if row["involvement"].as_str()==Some("missing") {tx.conn().execute("DELETE FROM memberships WHERE room_id=? AND user_id=?",rusqlite::params![room,recipient])?;}
            let source=if row["reply"].as_bool()==Some(true) {Some(Message::create(tx,NewMessage {room_id:room,creator_id:recipient,thread_id:thread,body:Some("Original".into()),..Default::default()})?.id)} else {None};
            let message=Message::create(tx,NewMessage {room_id:room,creator_id:if row["self"].as_bool()==Some(true) {recipient} else {author},thread_id:thread,body:Some(row["body"].as_str().unwrap().into()),reply_to_message_id:source,reply_notify_author:Some(row["reply_notify_author"].as_bool().unwrap_or(true)),streaming:row["streaming"].as_bool().unwrap_or(false),system_note:row["system_note"].as_bool().unwrap_or(false),..Default::default()})?;
            let candidates=crate::models::activity_item::message_recorder::candidates(tx.conn(),tx.rich_text(),&message,tx.now())?;
            Ok((message,candidates.room_member_ids))
        });
        assert_eq!(
            t.read(move |c| ActivityItem::find_by_user_and_source(
                c,
                id("david"),
                "Message",
                message.id
            ))
            .map(|i| i.event_type),
            expected,
            "{name}"
        );
        assert_eq!(
            serde_json::to_value(members).unwrap(),
            expected_candidates,
            "{name} candidates"
        );
    }
}
#[test]
fn ws17_keyword_thread_only_members_and_muted_follower_priority() {
    let t = TestDb::new();
    t.write(|tx| {
        KeywordAlert::create(tx, id("david"), "deploy")?;
        KeywordAlert::create(tx, id("kevin"), "deploy")?;
        let thread = crate::ChannelThread::create(
            tx,
            crate::NewChannelThread {
                room_id: id("designers"),
                creator_id: id("jz"),
                name: Some("Keyword thread".into()),
                ..Default::default()
            },
        )?;
        crate::ThreadMembership::join(tx, thread.id, id("jz"))?;
        let mut member = crate::ThreadMembership::join(tx, thread.id, id("david"))?;
        for (involvement, expected) in [
            (crate::ThreadInvolvement::Mentions, Some("keyword_alert")),
            (crate::ThreadInvolvement::Nothing, None),
            (
                crate::ThreadInvolvement::Everything,
                Some("thread_activity"),
            ),
        ] {
            member.update_involvement(tx, involvement)?;
            let message = Message::create(
                tx,
                NewMessage {
                    room_id: id("designers"),
                    creator_id: id("jz"),
                    thread_id: Some(thread.id),
                    body: Some("Deploy now".into()),
                    ..Default::default()
                },
            )?;
            assert_eq!(
                ActivityItem::find_by_user_and_source(
                    tx.conn(),
                    id("david"),
                    "Message",
                    message.id
                )?
                .as_ref()
                .map(|i| i.event_type.as_str()),
                expected
            );
            assert!(
                ActivityItem::find_by_user_and_source(
                    tx.conn(),
                    id("kevin"),
                    "Message",
                    message.id
                )?
                .is_none()
            );
        }
        Ok(())
    });
}
#[test]
fn ws17_ten_thread_updates_group_unread_and_handled_starts_new_item() {
    let t = TestDb::new();
    let (thread,item)=t.write(|tx| {
        let thread=crate::ChannelThread::create(tx,crate::NewChannelThread {room_id:id("designers"),creator_id:id("jz"),name:Some("Grouped thread".into()),..Default::default()})?;
        crate::ThreadMembership::join(tx,thread.id,id("jz"))?;crate::ThreadMembership::join(tx,thread.id,id("david"))?.update_involvement(tx,crate::ThreadInvolvement::Everything)?;
        let mut last=None;
        for n in 0..10 {last=Some(Message::create(tx,NewMessage {room_id:id("designers"),creator_id:id("jz"),thread_id:Some(thread.id),body:Some(format!("Update {n}")),reply_to_message_id:last, ..Default::default()})?.id);}
        let item=ActivityItem::find_by_user_and_source(tx.conn(),id("david"),"Message",last.unwrap())?.unwrap();
        let count:i64=tx.conn().query_row("SELECT count(*) FROM activity_items WHERE user_id=? AND source_type='Message' AND source_id IN (SELECT id FROM messages WHERE thread_id=?)",rusqlite::params![id("david"),thread.id],|r|r.get(0))?;assert_eq!(count,1);assert!(item.unread());
        tx.conn().execute("UPDATE activity_items SET read_at=? WHERE id=?",rusqlite::params![tx.now(),item.id])?;Ok((thread.id,item.id))
    });
    t.travel(60);
    t.write(move |tx| {
        let message = Message::create(
            tx,
            NewMessage {
                room_id: id("designers"),
                creator_id: id("jz"),
                thread_id: Some(thread),
                body: Some("Update 10".into()),
                ..Default::default()
            },
        )?;
        let refreshed =
            ActivityItem::find_by_user_and_source(tx.conn(), id("david"), "Message", message.id)?
                .unwrap();
        assert_eq!(refreshed.id, item);
        assert!(refreshed.unread());
        assert!(refreshed.updated_at >= message.created_at);
        tx.conn().execute(
            "UPDATE activity_items SET handled_at=? WHERE id=?",
            rusqlite::params![tx.now(), item],
        )?;
        let message = Message::create(
            tx,
            NewMessage {
                room_id: id("designers"),
                creator_id: id("jz"),
                thread_id: Some(thread),
                body: Some("New item".into()),
                ..Default::default()
            },
        )?;
        assert_ne!(
            ActivityItem::find_by_user_and_source(tx.conn(), id("david"), "Message", message.id)?
                .unwrap()
                .id,
            item
        );
        Ok(())
    });
}
#[test]
fn ws17_same_message_recording_keeps_type_read_and_handled_state() {
    let t = TestDb::new();
    t.write(|tx| {
        KeywordAlert::create(tx, id("david"), "deploy")?;
        let message = Message::create(
            tx,
            NewMessage {
                room_id: id("designers"),
                creator_id: id("jz"),
                body: Some("Deploy".into()),
                ..Default::default()
            },
        )?;
        let item =
            ActivityItem::find_by_user_and_source(tx.conn(), id("david"), "Message", message.id)?
                .unwrap();
        tx.conn().execute(
            "UPDATE activity_items SET read_at=?,handled_at=? WHERE id=?",
            rusqlite::params![tx.now(), tx.now(), item.id],
        )?;
        let before = ActivityItem::find(tx.conn(), item.id)?;
        for _ in 0..2 {
            ActivityItem::record_message(tx, &message)?;
        }
        assert_eq!(ActivityItem::find(tx.conn(), item.id)?, before);
        Ok(())
    });
}
#[test]
fn ws17_activity_insert_failure_rolls_back_message_jobs_and_broadcasts() {
    let t = TestDb::new();
    t.write(|tx| {KeywordAlert::create(tx,id("david"),"deploy")?;tx.conn().execute_batch("CREATE TRIGGER ws17_reject_activity BEFORE INSERT ON activity_items BEGIN SELECT RAISE(ABORT,'ws17 rejected activity'); END")?;Ok(())});
    t.sink.take();
    let before = t.read(Message::count);
    assert!(
        t.try_write(|tx| Message::create(
            tx,
            NewMessage {
                room_id: id("designers"),
                creator_id: id("jz"),
                body: Some("Deploy".into()),
                ..Default::default()
            }
        ))
        .is_err()
    );
    assert_eq!(t.read(Message::count), before);
    assert!(t.events().is_empty());
}

thread_local! {static SQL: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };}
fn trace(event: rusqlite::trace::TraceEvent<'_>) {
    if let rusqlite::trace::TraceEvent::Stmt(_, sql) = event {
        SQL.with(|queries| queries.borrow_mut().push(sql.into()));
    }
}
fn record_queries(t: &TestDb, message: Message, needle: &'static str) -> usize {
    t.write(move |tx| {
        tx.conn().execute(
            "DELETE FROM activity_items WHERE source_type='Message' AND source_id=?",
            [message.id],
        )?;
        SQL.with(|queries| queries.borrow_mut().clear());
        tx.conn().trace_v2(
            rusqlite::trace::TraceEventCodes::SQLITE_TRACE_STMT,
            Some(trace),
        );
        let result = ActivityItem::record_message(tx, &message);
        tx.conn()
            .trace_v2(rusqlite::trace::TraceEventCodes::empty(), None);
        result?;
        Ok(SQL.with(|queries| {
            queries
                .borrow()
                .iter()
                .filter(|q| q.to_lowercase().contains(needle))
                .count()
        }))
    })
}
fn followers(t: &TestDb, thread: Option<i64>, count: i64, offset: i64) {
    t.write(move |tx| {
        for n in 0..count {
            let user = crate::User::create(
                tx,
                crate::NewUser {
                    name: format!("Follower {}", n + offset),
                    ..Default::default()
                },
            )?;
            crate::Room::find(tx.conn(), id("designers"))?.grant_to(tx, &[user.id])?;
            if let Some(thread) = thread {
                crate::ThreadMembership::join(tx, thread, user.id)?
                    .update_involvement(tx, crate::ThreadInvolvement::Mentions)?;
                KeywordAlert::create(tx, user.id, "deploy")?;
            }
        }
        Ok(())
    });
}
#[test]
fn ws17_root_candidates_load_only_keyword_holders_not_bystanders() {
    let t = TestDb::new();
    followers(&t, None, 30, 0);
    t.write(|tx| {
        KeywordAlert::create(tx, id("david"), "deploy")?;
        KeywordAlert::create(tx, id("kevin"), "deploy")?;
        let message = Message::create(
            tx,
            NewMessage {
                room_id: id("designers"),
                creator_id: id("jz"),
                body: Some("Deploy now".into()),
                ..Default::default()
            },
        )?;
        let actual = crate::models::activity_item::message_recorder::candidates(
            tx.conn(),
            tx.rich_text(),
            &message,
            tx.now(),
        )?;
        let mut expected = vec![id("david"), id("kevin")];
        expected.sort_unstable();
        assert_eq!(actual.room_member_ids, expected);
        Ok(())
    });
}
#[test]
fn ws17_room_candidate_queries_stay_flat_as_roster_grows() {
    let t = TestDb::new();
    let message = t.write(|tx| {
        KeywordAlert::create(tx, id("david"), "deploy")?;
        KeywordAlert::create(tx, id("kevin"), "deploy")?;
        Message::create(
            tx,
            NewMessage {
                room_id: id("designers"),
                creator_id: id("jz"),
                body: Some("Deploy now".into()),
                ..Default::default()
            },
        )
    });
    followers(&t, None, 5, 0);
    let small = record_queries(&t, message.clone(), "membership");
    let keyword = record_queries(&t, message.clone(), "keyword_alert");
    followers(&t, None, 25, 5);
    let large = record_queries(&t, message.clone(), "membership");
    assert!(small > 0);
    assert_eq!(small, large);
    assert_eq!(keyword, record_queries(&t, message, "keyword_alert"));
}
#[test]
fn ws17_thread_keyword_and_membership_queries_stay_flat_as_followers_grow() {
    let t = TestDb::new();
    let thread = t.write(|tx| {
        let thread = crate::ChannelThread::create(
            tx,
            crate::NewChannelThread {
                room_id: id("designers"),
                creator_id: id("jz"),
                name: Some("Ceiling".into()),
                ..Default::default()
            },
        )?;
        crate::ThreadMembership::join(tx, thread.id, id("jz"))?;
        Ok(thread.id)
    });
    followers(&t, Some(thread), 5, 0);
    let message = t.write(move |tx| {
        Message::create(
            tx,
            NewMessage {
                room_id: id("designers"),
                creator_id: id("jz"),
                thread_id: Some(thread),
                body: Some("Deploy now".into()),
                ..Default::default()
            },
        )
    });
    let small = record_queries(&t, message.clone(), "keyword_alert");
    let members = record_queries(&t, message.clone(), "membership");
    followers(&t, Some(thread), 25, 5);
    assert!(small > 0);
    assert_eq!(small, record_queries(&t, message.clone(), "keyword_alert"));
    assert_eq!(members, record_queries(&t, message, "membership"));
}

#[test]
fn ws17_recorder_current_preferences_and_existing_items_match_named_sequences() {
    let t = TestDb::new();
    t.write(|tx| {
        let room = id("designers");
        let recipient = id("david");
        let author = id("jz");
        let mut membership =
            crate::Membership::find_by_room_and_user(tx.conn(), room, recipient)?.unwrap();
        let mention = crate::rich_text::mention_attachment_for(recipient);
        for involvement in [crate::Involvement::Nothing, crate::Involvement::Invisible] {
            membership.update_involvement(tx, involvement)?;
            let message = Message::create(
                tx,
                NewMessage {
                    room_id: room,
                    creator_id: author,
                    body: Some(format!("Hey {mention}")),
                    ..Default::default()
                },
            )?;
            let item =
                ActivityItem::find_by_user_and_source(tx.conn(), recipient, "Message", message.id)?;
            assert_eq!(
                item.as_ref().map(|i| i.event_type.as_str()),
                (involvement != crate::Involvement::Invisible).then_some("mention")
            );
            if let Some(item) = item {
                let before = item.clone();
                membership.update_involvement(tx, crate::Involvement::Invisible)?;
                assert_eq!(ActivityItem::find(tx.conn(), item.id)?, before);
            }
        }
        let source = Message::create(
            tx,
            NewMessage {
                room_id: room,
                creator_id: recipient,
                body: Some("Original".into()),
                ..Default::default()
            },
        )?;
        for involvement in [crate::Involvement::Nothing, crate::Involvement::Mentions] {
            membership.update_involvement(tx, involvement)?;
            let message = Message::create(
                tx,
                NewMessage {
                    room_id: room,
                    creator_id: author,
                    body: Some("Reply".into()),
                    reply_to_message_id: Some(source.id),
                    ..Default::default()
                },
            )?;
            assert_eq!(
                ActivityItem::find_by_user_and_source(tx.conn(), recipient, "Message", message.id)?
                    .as_ref()
                    .map(|i| i.event_type.as_str()),
                (involvement == crate::Involvement::Mentions).then_some("reply")
            );
        }
        let thread = crate::ChannelThread::create(
            tx,
            crate::NewChannelThread {
                room_id: room,
                creator_id: author,
                name: Some("Preferences".into()),
                ..Default::default()
            },
        )?;
        crate::ThreadMembership::join(tx, thread.id, author)?;
        let mut follower = crate::ThreadMembership::join(tx, thread.id, recipient)?;
        for (involvement, room_involvement, expected) in [
            (
                crate::ThreadInvolvement::Mentions,
                crate::Involvement::Mentions,
                None,
            ),
            (
                crate::ThreadInvolvement::Everything,
                crate::Involvement::Mentions,
                Some("thread_activity"),
            ),
            (
                crate::ThreadInvolvement::Everything,
                crate::Involvement::Nothing,
                None,
            ),
            (
                crate::ThreadInvolvement::Everything,
                crate::Involvement::Mentions,
                Some("thread_activity"),
            ),
        ] {
            follower.update_involvement(tx, involvement)?;
            membership.update_involvement(tx, room_involvement)?;
            let message = Message::create(
                tx,
                NewMessage {
                    room_id: room,
                    creator_id: author,
                    thread_id: Some(thread.id),
                    body: Some("Update".into()),
                    ..Default::default()
                },
            )?;
            assert_eq!(
                ActivityItem::find_by_user_and_source(tx.conn(), recipient, "Message", message.id)?
                    .as_ref()
                    .map(|i| i.event_type.as_str()),
                expected
            );
        }
        Ok(())
    });
}
#[test]
fn ws17_root_candidates_only_load_mentionee_and_reply_author_memberships() {
    let t = TestDb::new();
    followers(&t, None, 30, 0);
    t.write(|tx| {
        let source = Message::create(
            tx,
            NewMessage {
                room_id: id("designers"),
                creator_id: id("david"),
                body: Some("Original".into()),
                ..Default::default()
            },
        )?;
        let message = Message::create(
            tx,
            NewMessage {
                room_id: id("designers"),
                creator_id: id("jz"),
                body: Some(format!(
                    "Hey {}",
                    crate::rich_text::mention_attachment_for(id("david"))
                )),
                reply_to_message_id: Some(source.id),
                ..Default::default()
            },
        )?;
        let candidates = crate::models::activity_item::message_recorder::candidates(
            tx.conn(),
            tx.rich_text(),
            &message,
            tx.now(),
        )?;
        assert_eq!(candidates.room_member_ids, [id("david")]);
        Ok(())
    });
}

#[test]
fn ws17_mentions_exclude_author_bots_and_involvement_changes_leave_items_untouched() {
    let t = TestDb::new();
    t.write(|tx| {
        let room = id("designers");
        let author = id("jz");
        let recipient = id("david");
        let body = format!(
            "Hey {} {} {}",
            crate::rich_text::mention_attachment_for(recipient),
            crate::rich_text::mention_attachment_for(author),
            crate::rich_text::mention_attachment_for(id("bender"))
        );
        let message = Message::create(
            tx,
            NewMessage {
                room_id: room,
                creator_id: author,
                body: Some(body),
                ..Default::default()
            },
        )?;
        let item =
            ActivityItem::find_by_user_and_source(tx.conn(), recipient, "Message", message.id)?
                .unwrap();
        assert_eq!(item.event_type, "mention");
        assert!(item.unread());
        for user in [author, id("bender")] {
            assert!(
                ActivityItem::find_by_user_and_source(tx.conn(), user, "Message", message.id)?
                    .is_none()
            );
        }
        let mut member =
            crate::Membership::find_by_room_and_user(tx.conn(), room, recipient)?.unwrap();
        for involvement in [crate::Involvement::Nothing, crate::Involvement::Invisible] {
            member.update_involvement(tx, involvement)?;
            assert_eq!(ActivityItem::find(tx.conn(), item.id)?, item);
        }
        Ok(())
    });
}
