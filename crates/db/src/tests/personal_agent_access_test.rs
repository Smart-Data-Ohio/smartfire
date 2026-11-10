use super::*;
use crate::models::{
    agent_access as access,
    agent_delivery::{AgentEvent, NewEvent},
    agent_direct_messages as dms, agent_event_access as events,
};
use crate::{AgentGrant, ChannelThread, NewChannelThread, NewGrant, Room};
use rusqlite::params;

fn personal() -> TestDb {
    let t = super::channel_thread_test::frozen();
    t.write(|tx| {
        tx.conn().execute(
            "DELETE FROM agent_grants WHERE agent_id=?",
            [id("bender_agent")],
        )?;
        tx.conn().execute(
            "UPDATE agents SET kind='personal',owner_id=? WHERE id=?",
            params![id("david"), id("bender_agent")],
        )?;
        Room::find(tx.conn(), id("watercooler"))?.grant_to(tx, &[id("bender"), id("david")])?;
        Room::find(tx.conn(), id("designers"))?.grant_to(tx, &[id("bender"), id("david")])?;
        Ok(())
    });
    t
}
fn grant(tx: &Tx<'_>, cap: &str, room: Option<i64>) -> Result<AgentGrant> {
    AgentGrant::create(
        tx,
        NewGrant {
            agent_id: id("bender_agent"),
            granted_by_id: id("david"),
            capability: cap.into(),
            room_id: room,
            ..Default::default()
        },
    )
}

#[test]
fn personal_agent_model_rejects_non_chat_and_roomless_grants() {
    let t = personal();
    t.write(|tx| {
        for (cap, room) in [
            ("external_action", Some(id("watercooler"))),
            ("fizzy", None),
            ("dm_anyone", None),
            ("read_messages", None),
        ] {
            assert!(
                matches!(grant(tx, cap, room), Err(crate::Error::RecordInvalid(_))),
                "{cap} {room:?}"
            );
        }
        Ok(())
    });
}

#[test]
fn personal_agent_database_rejects_non_chat_grants() {
    let t = personal();
    t.write(|tx| {
        assert!(tx.conn().execute("INSERT INTO agent_grants(agent_id,capability,room_id,granted_by_id,created_at,updated_at) VALUES(?,'external_action',?,?,?,?)",params![id("bender_agent"),id("watercooler"),id("david"),tx.now(),tx.now()]).is_err());
        Ok(())
    });
}

#[test]
fn personal_agent_no_legacy_and_live_owner_room_ceiling() {
    let t = personal();
    t.write(|tx| {
        assert!(!access::capability_for_agent(
            tx.conn(),
            id("bender_agent"),
            "read_messages",
            Some(id("watercooler"))
        )?);
        for cap in ["read_messages", "post_messages", "react", "manage_threads"] {
            grant(tx, cap, Some(id("watercooler")))?;
            assert!(access::capability_for_agent(
                tx.conn(),
                id("bender_agent"),
                cap,
                Some(id("watercooler"))
            )?);
        }
        tx.conn().execute(
            "DELETE FROM memberships WHERE room_id=? AND user_id=?",
            params![id("watercooler"), id("david")],
        )?;
        for cap in ["read_messages", "post_messages", "react", "manage_threads"] {
            assert!(!access::capability_for_agent(
                tx.conn(),
                id("bender_agent"),
                cap,
                Some(id("watercooler"))
            )?);
        }
        Ok(())
    });
}

#[test]
fn personal_agent_owner_thread_access_and_demotion_are_live() {
    let t = personal();
    t.write(|tx| {
        let thread = ChannelThread::create(
            tx,
            NewChannelThread {
                room_id: id("designers"),
                creator_id: id("jz"),
                name: Some("Personal secret".into()),
                visibility: crate::channel_thread::ThreadVisibility::Private,
                ..Default::default()
            },
        )?;
        thread.add_access_member(tx, id("jz"), id("bender"))?;
        thread.add_access_member(tx, id("jz"), id("david"))?;
        assert!(thread.readable_by(tx.conn(), id("bender"))?);
        tx.conn().execute(
            "DELETE FROM thread_access_memberships WHERE thread_id=? AND user_id=?",
            params![thread.id, id("david")],
        )?;
        assert!(thread.readable_by(tx.conn(), id("bender"))?);
        tx.conn()
            .execute("UPDATE users SET role=0 WHERE id=?", [id("david")])?;
        assert!(!thread.readable_by(tx.conn(), id("bender"))?);
        Ok(())
    });
}

#[test]
fn personal_agent_direct_room_set_rejects_other_humans_and_groups() {
    let t = personal();
    t.write(|tx| {
        assert!(
            Room::find_or_create_direct_for(tx, &[id("bender"), id("jason")], id("jason")).is_err()
        );
        assert!(
            Room::find_or_create_direct_for(
                tx,
                &[id("bender"), id("david"), id("jason")],
                id("david")
            )
            .is_err()
        );
        let own = Room::find_or_create_direct_for(tx, &[id("bender"), id("david")], id("david"))?;
        assert!(access::capability_for_agent(
            tx.conn(),
            id("bender_agent"),
            "post_messages",
            Some(own.id)
        )?);
        assert!(!dms::allowed(
            tx,
            &crate::Agent::find(tx.conn(), id("bender_agent"))?.unwrap(),
            id("jason")
        )?);
        Ok(())
    });
}

#[test]
fn personal_agent_polling_and_ack_share_live_room_ceiling() {
    let t = personal();
    t.write(|tx| {
        tx.conn().execute("DELETE FROM agent_events", [])?;
        grant(tx, "read_messages", Some(id("watercooler")))?;
        grant(tx, "read_messages", Some(id("designers")))?;
        let event = AgentEvent::create(
            tx,
            NewEvent {
                agent_id: id("bender_agent"),
                event_type: "mention".into(),
                room_id: Some(id("watercooler")),
                message_id: Some(id("fourth")),
                outcome: Some("pending".into()),
                ..Default::default()
            },
        )?;
        assert!(
            events::readable_page(tx.conn(), id("bender_agent"), 0, None)?
                .iter()
                .any(|e| e.id == event.id)
        );
        tx.conn().execute(
            "DELETE FROM memberships WHERE room_id=? AND user_id=?",
            params![id("watercooler"), id("david")],
        )?;
        assert!(
            !events::readable_page(tx.conn(), id("bender_agent"), 0, None)?
                .iter()
                .any(|e| e.id == event.id)
        );
        assert!(!matches!(
            events::acknowledge(tx, id("bender_agent"), event.id)?,
            events::Acknowledgment::Acknowledged { .. }
        ));
        Ok(())
    });
}

#[test]
fn personal_agent_trigger_guards_every_insert_update_and_kind_flip() {
    let t = personal();
    t.write(|tx| {
        let own=Room::find_or_create_direct_for(tx,&[id("bender"),id("david")],id("david"))?;
        for (cap,room) in [("external_action",Some(id("watercooler"))),("fizzy",None),("dm_anyone",None),("read_messages",None),("react",Some(own.id))] {
            assert!(tx.conn().execute("INSERT INTO agent_grants(agent_id,capability,room_id,granted_by_id,created_at,updated_at) VALUES(?,?,?,?,?,?)",params![id("bender_agent"),cap,room,id("david"),tx.now(),tx.now()]).is_err());
            assert!(matches!(grant(tx,cap,room),Err(crate::Error::RecordInvalid(_))));
        }
        let valid=grant(tx,"read_messages",Some(id("watercooler")))?;
        for (cap,room) in [("external_action",Some(id("watercooler"))),("fizzy",None),("dm_anyone",None),("read_messages",None),("read_messages",Some(own.id))] {
            assert!(tx.conn().execute("UPDATE agent_grants SET capability=?,room_id=? WHERE id=?",params![cap,room,valid.id]).is_err());
            let mut model=AgentGrant::find(tx.conn(),valid.id)?.unwrap();
            assert!(matches!(model.update(tx,crate::GrantChanges{capability:Some(cap.into()),room_id:Some(room),..Default::default()}),Err(crate::Error::RecordInvalid(_))));
        }
        tx.conn().execute("UPDATE agents SET kind='workspace' WHERE id=?",[id("bender_agent")])?;
        let other=grant(tx,"fizzy",None)?;
        assert!(tx.conn().execute("UPDATE agents SET kind='personal' WHERE id=?",[id("bender_agent")]).is_err());
        let mut model=crate::Agent::find(tx.conn(),id("bender_agent"))?.unwrap();
        assert!(matches!(model.update(tx,crate::AgentChanges{kind:Some(crate::AgentKind::Personal),..Default::default()}),Err(crate::Error::RecordInvalid(_))));
        other.destroy(tx)?;
        tx.conn().execute("UPDATE agents SET kind='personal' WHERE id=?",[id("bender_agent")])?;
        Ok(())
    });
}

#[test]
fn personal_agent_planted_forbidden_grants_fail_all_read_functions_and_approvals() {
    let t = personal();
    t.write(|tx| {
        tx.conn().execute_batch("DROP TRIGGER personal_agent_grant_insert;DROP TRIGGER personal_agent_grant_update;")?;
        for cap in ["external_action","fizzy","dm_anyone"] {
            tx.conn().execute("INSERT INTO agent_grants(agent_id,capability,granted_by_id,created_at,updated_at) VALUES(?,?,?,?,?)",params![id("bender_agent"),cap,id("david"),tx.now(),tx.now()])?;
            for room in [None,Some(id("watercooler"))] {
                assert!(!access::capability_for_agent(tx.conn(),id("bender_agent"),cap,room)?);
                assert!(!access::capabilities_for_agents(tx.conn(),cap,&[(id("bender_agent"),room)])?[&(id("bender_agent"),room)]);
            }
            assert!(!access::has_capability_anywhere(tx.conn(),id("bender_agent"),cap)?);
            assert_eq!(access::capability_for_user(tx.conn(),id("bender"),cap,id("watercooler"))?,Some(false));
            assert_eq!(access::capabilities_for_user_in_room(tx.conn(),id("bender"),id("watercooler"),&[cap])?[cap],Some(false));
            assert!(access::capabilities_for_users_in_rooms(tx.conn(),&[id("bender")],&[id("watercooler")],cap)?.is_empty());
        }
        Ok(())
    });
}

#[test]
fn personal_agent_batch_matches_single_for_live_owner_grants_and_rooms() {
    let t = personal();
    t.write(|tx| {
        for cap in access::PERSONAL_CAPABILITIES {
            grant(tx, cap, Some(id("watercooler")))?;
        }
        let dm = Room::find_or_create_direct_for(tx, &[id("bender"), id("david")], id("david"))?;
        let rooms = [id("watercooler"), id("designers"), dm.id];
        let mut messages = Vec::new();
        for room_id in rooms {
            messages.push(crate::Message::create(
                tx,
                crate::NewMessage {
                    room_id,
                    creator_id: id("david"),
                    markdown_source: Some("Capability matrix event".into()),
                    ..Default::default()
                },
            )?);
        }
        tx.conn().execute("DELETE FROM agent_events", [])?;
        for message in messages {
            AgentEvent::create(
                tx,
                NewEvent {
                    agent_id: id("bender_agent"),
                    event_type: if message.room_id == dm.id {
                        "direct_message"
                    } else {
                        "mention"
                    }
                    .into(),
                    room_id: Some(message.room_id),
                    message_id: Some(message.id),
                    outcome: Some("pending".into()),
                    ..Default::default()
                },
            )?;
        }
        for (status, member, revoked) in [
            (0, true, false),
            (0, false, false),
            (0, true, true),
            (1, true, false),
        ] {
            tx.conn().execute(
                "UPDATE users SET status=? WHERE id=?",
                params![status, id("david")],
            )?;
            if member {
                Room::find(tx.conn(), id("watercooler"))?.grant_to(tx, &[id("david")])?;
            } else {
                tx.conn().execute(
                    "DELETE FROM memberships WHERE user_id=? AND room_id=?",
                    params![id("david"), id("watercooler")],
                )?;
            }
            tx.conn().execute(
                "UPDATE agent_grants SET revoked_at=? WHERE agent_id=?",
                params![revoked.then(|| tx.now()), id("bender_agent")],
            )?;
            for cap in access::CAPABILITIES {
                let requests = rooms
                    .iter()
                    .map(|room| (id("bender_agent"), Some(*room)))
                    .chain([(id("bender_agent"), None)])
                    .collect::<Vec<_>>();
                let batch = access::capabilities_for_agents(tx.conn(), cap, &requests)?;
                let users = access::capabilities_for_users_in_rooms(
                    tx.conn(),
                    &[id("bender")],
                    &rooms,
                    cap,
                )?;
                for &room in &rooms {
                    let single = access::capability_for_agent(
                        tx.conn(),
                        id("bender_agent"),
                        cap,
                        Some(room),
                    )?;
                    let expected = status == 0
                        && if room == dm.id {
                            ["read_messages", "post_messages", "react"].contains(&cap)
                        } else {
                            room == id("watercooler")
                                && member
                                && !revoked
                                && access::PERSONAL_CAPABILITIES.contains(&cap)
                        };
                    assert_eq!(single, expected, "{cap} {room} {status}/{member}/{revoked}");
                    assert_eq!(batch[&(id("bender_agent"), Some(room))], expected);
                    assert_eq!(users.contains(&(id("bender"), room)), expected);
                    assert_eq!(
                        access::capability_for_user(tx.conn(), id("bender"), cap, room)?,
                        Some(expected)
                    );
                    assert_eq!(
                        access::capabilities_for_user_in_room(
                            tx.conn(),
                            id("bender"),
                            room,
                            &[cap]
                        )?[cap],
                        Some(expected)
                    );
                }
                assert!(!batch[&(id("bender_agent"), None)]);
                let anywhere = status == 0
                    && (["read_messages", "post_messages", "react"].contains(&cap)
                        || (cap == "manage_threads" && member && !revoked));
                assert_eq!(
                    access::has_capability_anywhere(tx.conn(), id("bender_agent"), cap)?,
                    anywhere
                );
            }
            let selected = events::readable_page(tx.conn(), id("bender_agent"), 0, None)?
                .into_iter()
                .map(|event| event.room_id.unwrap())
                .collect::<std::collections::HashSet<_>>();
            let mut expected = std::collections::HashSet::new();
            if status == 0 {
                expected.insert(dm.id);
                if member && !revoked {
                    expected.insert(id("watercooler"));
                }
            }
            assert_eq!(selected, expected, "polling {status}/{member}/{revoked}");
        }
        Ok(())
    });
}

#[test]
fn personal_agent_creation_has_no_key_or_auto_join_and_blocks_key_reset_webhooks() {
    let t = super::channel_thread_test::frozen();
    let agent = t.write(|tx| {
        crate::models::personal_agents::create(
            tx,
            crate::NewUser {
                name: "Personal helper".into(),
                ..Default::default()
            },
            id("david"),
        )
    });
    t.write(move |tx| {
        let mut bot = crate::User::find(tx.conn(), agent.user_id)?;
        assert!(bot.bot_token_digest.is_none());
        assert!(Room::for_user(tx.conn(), bot.id)?.is_empty());
        assert_eq!(agent.daily_message_cap, Some(200));
        assert!(bot.reset_bot_key(tx).is_err());
        assert!(crate::Webhook::create(tx, bot.id, Some("https://example.test/hook")).is_err());
        Room::create(
            tx,
            crate::RoomType::Open,
            Some("New open room"),
            id("david"),
        )?;
        Ok(())
    });
    t.read(move |conn| {
        assert!(Room::for_user(conn, agent.user_id)?.is_empty());
        Ok(())
    });
}

#[test]
fn personal_agent_owner_dm_first_message_followups_and_group_addition() {
    let t = personal();
    t.write(|tx| {
        use crate::models::{agent_posting::DriveInput,audit_log::Context};
        for text in ["Hello owner","Follow-up"] {
            assert!(matches!(dms::open_and_post(tx,id("bender_agent"),id("david"),crate::NewMessage{markdown_source:Some(text.into()),..Default::default()},DriveInput::Absent,&Context::default())?,dms::DirectMessageResult::Posted {..}));
        }
        let room=Room::find_direct_for(tx.conn(),&[id("bender"),id("david")])?.unwrap();
        assert!(room.add_direct_members(tx,&[id("jason")],id("david")).is_err());
        assert!(!access::capability_for_agent(tx.conn(),id("bender_agent"),"manage_threads",Some(room.id))?);
        AgentEvent::create(tx,NewEvent{agent_id:id("bender_agent"),actor_id:Some(id("jason")),event_type:"mention".into(),..Default::default()})?;
        assert!(matches!(dms::open_and_post(tx,id("bender_agent"),id("jason"),crate::NewMessage::default(),DriveInput::Absent,&Context::default())?,dms::DirectMessageResult::Denied(r) if r.status==403));
        Ok(())
    });
}

#[test]
fn personal_agent_owner_deactivation_disables_auth_and_legacy_bot_key() {
    let t = personal();
    t.write(|tx| {
        let (_, secret) = crate::AgentCredential::create_with_secret(
            tx,
            id("bender_agent"),
            "personal",
            id("david"),
            None,
        )?;
        assert!(access::authenticate_identity(tx, &secret, "127.0.0.1")?.is_some());
        tx.conn()
            .execute("UPDATE users SET status=1 WHERE id=?", [id("david")])?;
        assert!(access::authenticate_identity(tx, &secret, "127.0.0.1")?.is_none());
        assert!(
            access::verify_identity(tx.conn(), &secret, tx.now(), &tx.env().default_url_origin,)?
                .is_none()
        );
        assert!(
            !crate::Agent::find(tx.conn(), id("bender_agent"))?
                .unwrap()
                .active(tx.conn())?
        );
        Ok(())
    });
}

#[test]
fn personal_agent_migration_rehearsal_preserves_background_jobs() {
    let dir = tempfile::tempdir().unwrap();
    let seed = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../parity/.seed/default/db/production.sqlite3");
    let db = dir.path().join("rehearsal.sqlite3");
    std::fs::copy(seed, &db).unwrap();
    let mut conn = crate::Connection::open(db).unwrap();
    conn.execute_batch("DROP TRIGGER IF EXISTS personal_agent_grant_insert;DROP TRIGGER IF EXISTS personal_agent_grant_update;DROP TRIGGER IF EXISTS personal_agent_kind_update;DELETE FROM schema_migrations WHERE version='20261012220000';").unwrap();
    let jobs = |conn: &crate::Connection| {
        conn.query_row("SELECT COALESCE(json_group_array(json(row)), '[]') FROM (SELECT json_object('id',id,'class',job_class,'arguments',arguments,'run_at',run_at) row FROM background_jobs ORDER BY id)",[],|row|row.get::<_,String>(0)).unwrap()
    };
    let before = jobs(&conn);
    crate::migrations::migrate_preserving_existing_foreign_key_violations(&mut conn).unwrap();
    assert_eq!(jobs(&conn), before);
    assert!(conn.query_row("SELECT COUNT(*)=3 FROM sqlite_master WHERE type='trigger' AND name LIKE 'personal_agent_%'",[],|row|row.get::<_,bool>(0)).unwrap());
}

#[test]
fn personal_agent_poll_write_rechecks_grant_after_successful_preflight() {
    let t = personal();
    let g = t.write(|tx| grant(tx, "post_messages", Some(id("watercooler"))));
    assert!(t.read(|conn| access::capability_for_agent(
        conn,
        id("bender_agent"),
        "post_messages",
        Some(id("watercooler"))
    )));
    t.write(move |tx| {
        let before: i64 = tx
            .conn()
            .query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0))?;
        let polls: i64 = tx
            .conn()
            .query_row("SELECT COUNT(*) FROM polls", [], |r| r.get(0))?;
        AgentGrant::find(tx.conn(), g.id)?.unwrap().revoke(tx)?;
        let result = crate::models::agent_polls::create(
            tx,
            id("bender_agent"),
            id("watercooler"),
            "Denied question".into(),
            crate::NewPoll {
                labels: vec!["one".into(), "two".into()],
                ..Default::default()
            },
        )?;
        assert!(matches!(result,Err(denial) if denial.status==403));
        assert_eq!(
            tx.conn()
                .query_row("SELECT COUNT(*) FROM messages", [], |r| r.get::<_, i64>(0))?,
            before
        );
        assert_eq!(
            tx.conn()
                .query_row("SELECT COUNT(*) FROM polls", [], |r| r.get::<_, i64>(0))?,
            polls
        );
        Ok(())
    });
}

#[test]
fn workspace_agent_completion_polling_ack_and_delivery_require_live_external_action() {
    let t = super::channel_thread_test::frozen();
    t.write(|tx| {
        tx.conn().execute("DELETE FROM agent_events", [])?;
        tx.conn().execute("DELETE FROM agent_grants", [])?;
        Room::find(tx.conn(), id("watercooler"))?.grant_to(tx, &[id("bender")])?;
        let approval = crate::AgentApproval::create(
            tx,
            crate::NewApproval {
                agent_id: id("bender_agent"),
                room_id: Some(id("watercooler")),
                action: "github.comment".into(),
                summary: "Completion policy".into(),
                ..Default::default()
            },
        )?;
        let roomless = crate::AgentApproval::create(
            tx,
            crate::NewApproval {
                room_id: None,
                ..crate::NewApproval {
                    agent_id: id("bender_agent"),
                    action: "fizzy.comment".into(),
                    summary: "Roomless policy".into(),
                    ..Default::default()
                }
            },
        )?;
        let mut ids = Vec::new();
        for kind in [
            "approval_decided",
            "github_action_completed",
            "fizzy_action_completed",
        ] {
            ids.push(
                AgentEvent::create(
                    tx,
                    NewEvent {
                        agent_id: id("bender_agent"),
                        agent_approval_id: Some(approval.id),
                        room_id: Some(id("designers")),
                        event_type: kind.into(),
                        outcome: Some("delivered".into()),
                        ..Default::default()
                    },
                )?
                .id,
            );
        }
        let none = AgentEvent::create(
            tx,
            NewEvent {
                agent_id: id("bender_agent"),
                agent_approval_id: Some(roomless.id),
                event_type: "fizzy_action_completed".into(),
                outcome: Some("delivered".into()),
                ..Default::default()
            },
        )?;
        assert!(events::readable_page(tx.conn(), id("bender_agent"), 0, None)?.is_empty());
        for &event in &ids {
            assert!(!events::delivery_authorized(tx.conn(), event)?);
            assert!(matches!(
                events::acknowledge(tx, id("bender_agent"), event)?,
                events::Acknowledgment::Forbidden
            ));
        }
        let mut g = AgentGrant::create(
            tx,
            NewGrant {
                agent_id: id("bender_agent"),
                room_id: Some(id("watercooler")),
                capability: "external_action".into(),
                granted_by_id: id("david"),
                ..Default::default()
            },
        )?;
        let selected = events::readable_page(tx.conn(), id("bender_agent"), 0, None)?
            .iter()
            .map(|e| e.id)
            .collect::<Vec<_>>();
        assert_eq!(selected, ids);
        for &event in &ids {
            assert!(events::delivery_authorized(tx.conn(), event)?);
            assert!(matches!(
                events::acknowledge(tx, id("bender_agent"), event)?,
                events::Acknowledgment::Acknowledged { .. }
            ));
        }
        assert!(!events::delivery_authorized(tx.conn(), none.id)?);
        g.revoke(tx)?;
        AgentGrant::create(
            tx,
            NewGrant {
                agent_id: id("bender_agent"),
                room_id: Some(id("designers")),
                capability: "external_action".into(),
                granted_by_id: id("david"),
                ..Default::default()
            },
        )?;
        assert!(events::readable_page(tx.conn(), id("bender_agent"), 0, None)?.is_empty());
        AgentGrant::create(
            tx,
            NewGrant {
                agent_id: id("bender_agent"),
                capability: "external_action".into(),
                granted_by_id: id("david"),
                ..Default::default()
            },
        )?;
        assert!(events::delivery_authorized(tx.conn(), none.id)?);
        Ok(())
    });
}

#[test]
fn personal_agent_cannot_reuse_private_media_after_read_grant_revocation() {
    let t = personal();
    t.write(|tx| {
        let thread=ChannelThread::create(tx,NewChannelThread{room_id:id("designers"),creator_id:id("jz"),name:Some("Private media".into()),visibility:crate::channel_thread::ThreadVisibility::Private,..Default::default()})?;
        thread.add_access_member(tx,id("jz"),id("bender"))?;
        tx.conn().execute("INSERT INTO active_storage_blobs(key,filename,content_type,metadata,service_name,byte_size,checksum,created_at) VALUES('personal-private-reuse','private.txt','text/plain','{}','local',4,'fixture',?)",[tx.now()])?;
        let blob=tx.conn().last_insert_rowid();
        crate::Message::create(tx,crate::NewMessage{room_id:id("designers"),thread_id:Some(thread.id),creator_id:id("jz"),attachment_blob_id:Some(blob),..Default::default()})?;
        let mut read=grant(tx,"read_messages",Some(id("designers")))?;
        assert!(crate::Blob::reusable_by(tx.conn(),blob,id("bender"),"Message")?);
        read.revoke(tx)?;
        let own=Room::find_or_create_direct_for(tx,&[id("bender"),id("david")],id("david"))?;
        assert!(access::capability_for_agent(tx.conn(),id("bender_agent"),"post_messages",Some(own.id))?);
        assert!(!crate::Blob::reusable_by(tx.conn(),blob,id("bender"),"Message")?);
        Ok(())
    });
}

#[test]
fn personal_agent_poll_option_reuse_rechecks_source_room_after_owner_departure() {
    let t = personal();
    t.write(|tx| {
        let message=crate::Message::create(tx,crate::NewMessage{room_id:id("watercooler"),creator_id:id("david"),markdown_source:Some("Poll media".into()),..Default::default()})?;
        let poll=crate::Poll::create_for_message(tx,&message,crate::NewPoll{labels:vec!["one".into(),"two".into()],..Default::default()})?;
        let option:i64=tx.conn().query_row("SELECT id FROM poll_options WHERE poll_id=? LIMIT 1",[poll.id],|r|r.get(0))?;
        tx.conn().execute("INSERT INTO active_storage_blobs(key,filename,content_type,metadata,service_name,byte_size,checksum,created_at) VALUES('personal-poll-reuse','poll.txt','text/plain','{\"poll_media\":true}','local',4,'fixture',?)",[tx.now()])?;
        let blob=tx.conn().last_insert_rowid();
        crate::Attachment::create(tx,"PollOption",option,"media",blob)?;
        let mut post=grant(tx,"post_messages",Some(id("watercooler")))?;
        assert!(crate::Blob::reusable_by(tx.conn(),blob,id("bender"),"PollOption")?);
        post.revoke(tx)?;
        assert!(!crate::Blob::reusable_by(tx.conn(),blob,id("bender"),"PollOption")?);
        tx.conn().execute("UPDATE agent_grants SET revoked_at=NULL WHERE id=?",[post.id])?;
        tx.conn().execute("DELETE FROM memberships WHERE user_id=? AND room_id=?",params![id("david"),id("watercooler")])?;
        assert!(!crate::Blob::reusable_by(tx.conn(),blob,id("bender"),"PollOption")?);
        Ok(())
    });
}
