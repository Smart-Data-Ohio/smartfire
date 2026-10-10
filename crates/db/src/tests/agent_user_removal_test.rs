use super::*;
use crate::models::user::removal::DependencyPhase;
use crate::{
    Agent, AgentCredential, AgentGrant, AgentKind, Boost, ChannelThread, Message, MessagePin,
    NewAgent, NewChannelThread, NewCredential, NewGrant, NewMessage, Room, SavedItem, User,
};
use rusqlite::params;
use serde_json::{Value, json};
struct Rows {
    user: User,
    agent: i64,
    credential: i64,
    grant: i64,
    own: i64,
    other: i64,
    thread: i64,
    survivor: i64,
}
fn setup(t: &TestDb) -> Rows {
    let rows = t.write(|tx| {
        let mut user = User::create_email_bot(tx)?;
        user.update(
            tx,
            crate::UserChanges {
                name: Some("Cleanup Bot".into()),
                ..Default::default()
            },
        )?;
        let room = Room::find(tx.conn(), id("watercooler"))?;
        room.grant_to(tx, &[user.id])?;
        let agent = Agent::create(
            tx,
            NewAgent {
                user_id: user.id,
                owner_id: Some(id("david")),
                kind: AgentKind::Workspace,
                ..Default::default()
            },
        )?;
        let credential = AgentCredential::create(
            tx,
            NewCredential {
                agent_id: agent.id,
                created_by_id: id("david"),
                name: "Keep".into(),
                token_digest: "ws11-user-delete-digest".into(),
                token_last_four: "disp".into(),
                ..Default::default()
            },
        )?;
        let grant = AgentGrant::create(
            tx,
            NewGrant {
                agent_id: agent.id,
                capability: "post_messages".into(),
                granted_by_id: id("david"),
                ..Default::default()
            },
        )?;
        crate::Webhook::create(tx, user.id, Some("https://example.test/hook"))?;
        let own = Message::create(
            tx,
            NewMessage {
                room_id: room.id,
                creator_id: user.id,
                markdown_source: Some("Remove me".into()),
                client_message_id: Some("ws11-remove-own".into()),
                ..Default::default()
            },
        )?;
        let other = Message::create(
            tx,
            NewMessage {
                room_id: room.id,
                creator_id: id("david"),
                markdown_source: Some("Survives".into()),
                client_message_id: Some("ws11-remove-other".into()),
                ..Default::default()
            },
        )?;
        MessagePin::create(tx, &own, room.id, user.id)?;
        MessagePin::create(tx, &other, room.id, user.id)?;
        Boost::create(tx, other.id, user.id, "Yes")?;
        SavedItem::save_for(tx, user.id, other.id, None)?;
        let thread = ChannelThread::create(
            tx,
            NewChannelThread {
                room_id: room.id,
                creator_id: user.id,
                name: Some("Remove thread".into()),
                ..Default::default()
            },
        )?;
        let survivor = ChannelThread::create(
            tx,
            NewChannelThread {
                room_id: room.id,
                creator_id: id("david"),
                name: Some("Keep owner work".into()),
                work_status: Some("planned".into()),
                ..Default::default()
            },
        )?;
        tx.conn().execute(
            "UPDATE channel_threads SET work_owner_id=? WHERE id=?",
            params![user.id, survivor.id],
        )?;
        Ok(Rows {
            user,
            agent: agent.id,
            credential: credential.id,
            grant: grant.id,
            own: own.id,
            other: other.id,
            thread: thread.id,
            survivor: survivor.id,
        })
    });
    t.sink.take();
    rows
}
#[test]
fn ws11_hard_user_removal_matches_rails_dependent_delete_and_pin_callbacks() {
    let t = super::channel_thread_test::frozen();
    let rows = setup(&t);
    let uid = rows.user.id;
    t.write(move |tx| rows.user.destroy(tx));
    t.read(move|conn| {
        let count=|table:&str,field:&str|->Result<i64>{Ok(conn.query_row(&format!("SELECT COUNT(*) FROM {table} WHERE {field}=?"),[uid],|r|r.get(0))?)};
        let result=json!({"user":User::find_by_id(conn,uid)?.is_some(),"agent":Agent::find(conn,rows.agent)?.is_some(),"webhook":count("webhooks","user_id")?,"credential":AgentCredential::find(conn,rows.credential)?.is_some(),"grant":AgentGrant::find(conn,rows.grant)?.is_some(),"grant_revoked":AgentGrant::find(conn,rows.grant)?.unwrap().revoked_at.is_some(),"own_message":Message::find_by_id(conn,rows.own)?.is_some(),"other_message":Message::find_by_id(conn,rows.other)?.is_some(),"pins":count("message_pins","pinner_id")?,"boosts":count("boosts","booster_id")?,"saves":count("saved_items","user_id")?,"thread":ChannelThread::find_by_id(conn,rows.thread)?.is_some(),"survivor_owner":ChannelThread::find(conn,rows.survivor)?.work_owner_id,"memberships":count("memberships","user_id")?});
        let gold:Value=serde_json::from_str(include_str!("../../../../vectors/agents_user_removal_contract.json")).unwrap();assert_eq!(result,gold["results"]);
        Ok(())
    });
    let badges: Vec<_> = t
        .sink
        .events()
        .iter()
        .filter_map(|e| e.as_broadcast())
        .filter_map(|b| match b {
            crate::broadcasts::Broadcast::MessagePinned { message_id } => Some(message_id),
            _ => None,
        })
        .collect();
    assert_eq!(
        badges.len(),
        2,
        "one callback per pin, including owned-message cascade"
    );
}
#[test]
fn ws11_hard_user_dependency_failure_restores_rows_grants_and_broadcasts_when_rescued() {
    let t = super::channel_thread_test::frozen();
    let rows = setup(&t);
    t.write(move |tx| {
        let error = rows.user.destroy_with_dependencies(tx, |_, _, phase| {
            if phase == DependencyPhase::Connections {
                Err(crate::Error::Other("WS11 dependency refusal".into()))
            } else {
                Ok(())
            }
        });
        assert!(error.is_err());
        assert!(Agent::find(tx.conn(), rows.agent)?.is_some());
        assert!(
            AgentGrant::find(tx.conn(), rows.grant)?
                .unwrap()
                .revoked_at
                .is_none()
        );
        assert!(Message::find_by_id(tx.conn(), rows.own)?.is_some());
        Ok(())
    });
    assert!(t.sink.events().is_empty());
}
#[test]
fn ws11_hard_user_delete_keeps_agent_command_fk_failure_atomic() {
    let t = super::channel_thread_test::frozen();
    let rows = setup(&t);
    t.write(move|tx| {
        tx.conn().execute("INSERT INTO agent_slash_commands(agent_id,room_id,name,description,created_at,updated_at) VALUES (?,?,'keep','keep',?,?)",params![rows.agent,id("watercooler"),tx.now(),tx.now()])?;
        assert!(rows.user.destroy(tx).is_err());
        assert!(User::find_by_id(tx.conn(),rows.user.id)?.is_some());
        assert!(AgentGrant::find(tx.conn(),rows.grant)?.unwrap().revoked_at.is_none());
        Ok(())
    });
    assert!(t.sink.events().is_empty());
}

#[test]
fn scheduled_message_files_are_purged_when_their_author_is_hard_deleted() {
    let t = super::channel_thread_test::frozen();
    let (user_id, scheduled_id, blob_id) = t.write(|tx| {
        let user = User::create_email_bot(tx)?;
        let room = Room::find(tx.conn(), id("watercooler"))?;
        room.grant_to(tx, &[user.id])?;
        let blob = crate::models::active_storage::Blob::create(
            tx,
            &crate::models::active_storage::Blob {
                id: 0,
                key: format!("scheduled-removal-{}", user.id),
                filename: "plan.txt".into(),
                content_type: Some("text/plain".into()),
                metadata: Some("{}".into()),
                service_name: "local".into(),
                byte_size: 4,
                checksum: None,
                created_at: tx.now(),
            },
        )?;
        let scheduled = crate::ScheduledMessage::create_with_attachments(
            tx,
            crate::NewScheduledMessage {
                user_id: user.id,
                room_id: room.id,
                thread_id: None,
                reply_to_message_id: None,
                markdown_source: "Later".into(),
                send_at: tx.now().since(jiff::SignedDuration::from_hours(1)),
            },
            &[blob.id],
        )?;
        Ok((user.id, scheduled.id, blob.id))
    });
    t.sink.take();
    t.write(move |tx| User::find_by_id(tx.conn(), user_id)?.unwrap().destroy(tx));
    t.read(move |conn| {
        assert!(crate::ScheduledMessage::find_by_id(conn, scheduled_id)?.is_none());
        assert_eq!(
            crate::sql::count(
                conn,
                "SELECT COUNT(*) FROM active_storage_attachments WHERE record_type='ScheduledMessage' AND record_id=?",
                [scheduled_id],
            )?,
            0
        );
        Ok(())
    });
    assert!(t.events().contains(&crate::Event::PurgeBlob { blob_id }));
}
