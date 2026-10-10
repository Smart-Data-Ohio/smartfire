//! Work read policies and candidates use the merged WS11 agent model APIs.
use super::*;
use crate::{
    AgentGrant, ChannelThread, Membership, NewChannelThread, NewGrant, Room, RoomType, User,
};

fn fixture(t: &TestDb) -> (Room, ChannelThread) {
    t.write(move |tx| {
        let room = Room::create_for(
            tx,
            RoomType::Board,
            Some("Work choices"),
            id("david"),
            &[id("david"), id("jz"), id("kevin"), id("bender")],
        )?;
        let post = ChannelThread::create(
            tx,
            NewChannelThread {
                room_id: room.id,
                creator_id: id("jz"),
                name: Some("Post".into()),
                work_status: Some("planned".into()),
                ..Default::default()
            },
        )?;
        Ok((room, post))
    })
}

#[test]
fn workspace_work_relation_rechecks_membership_is_unbounded_and_rejects_inactive_or_bot_viewers() {
    let t=channel_thread_test::frozen();
    let (room,hidden,ids)=t.write(|tx| {
        tx.conn().execute("UPDATE channel_threads SET work_status=NULL",[])?;
        let room=Room::create_for(tx,RoomType::Open,Some("Visible"),id("david"),&[id("david"),id("jz")])?;
        let hidden=Room::create_for(tx,RoomType::Closed,Some("Hidden"),id("david"),&[id("david")])?;
        let mut ids=Vec::new();
        for i in 0..105 {
            ids.push(ChannelThread::create(tx,NewChannelThread {room_id:room.id,creator_id:id("jz"),name:Some(format!("Work {i}")),work_status:Some("planned".into()),..Default::default()})?.id);
        }
        ChannelThread::create(tx,NewChannelThread {room_id:hidden.id,creator_id:id("david"),name:Some("Hidden work".into()),work_status:Some("planned".into()),..Default::default()})?;
        Ok((room.id,hidden.id,ids))
    });
    t.read(|conn| {
        let viewer=User::find(conn,id("jz"))?;
        let rows=ChannelThread::visible_work_threads(conn,&viewer,"all")?;
        assert_eq!(rows.iter().map(|row|row.id).collect::<Vec<_>>(),ids.iter().rev().copied().collect::<Vec<_>>());
        assert!(!rows.iter().any(|row|row.room_id==hidden));
        let mut inactive=viewer.clone();inactive.status=crate::models::user::Status::Deactivated;
        assert!(ChannelThread::visible_work_threads(conn,&inactive,"all")?.is_empty());
        assert!(ChannelThread::visible_work_threads(conn,&User::find(conn,id("bender"))?,"all")?.is_empty());
        Ok(())
    });
    t.write(move |tx| { Membership::find_by_room_and_user(tx.conn(),room,id("jz"))?.unwrap().destroy(tx)?;Ok(()) });
    t.read(|conn| {
        assert!(ChannelThread::visible_work_threads(conn,&User::find(conn,id("jz"))?,"all")?.is_empty());Ok(())
    });
}
#[test]
fn work_policy_separates_managers_owners_and_parent_members() {
    let t = channel_thread_test::frozen();
    let (room, post) = fixture(&t);
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE channel_threads SET work_owner_id=? WHERE id=?",
            (id("kevin"), post.id),
        )?;
        Ok(())
    });
    t.read(|conn| {
        let post = ChannelThread::find(conn, post.id)?;
        for user in ["david", "jz"] {
            let user = User::find(conn, id(user))?;
            assert!(post.work_manageable_by(conn, &user)?);
            assert!(post.work_assignment_manageable_by(conn, &user)?);
        }
        let owner = User::find(conn, id("kevin"))?;
        assert!(post.work_manageable_by(conn, &owner)?);
        assert!(!post.work_assignment_manageable_by(conn, &owner)?);
        let bot = User::find(conn, id("bender"))?;
        assert!(!post.work_viewable_by(conn, &bot)?);
        let outsider = User::find(conn, id("jason"))?;
        assert!(!post.work_manageable_by(conn, &outsider)?);
        Ok(())
    });
    t.write(move |tx| {
        Membership::find_by_room_and_user(tx.conn(), room.id, id("kevin"))?
            .unwrap()
            .destroy(tx)
    });
    t.read(|conn| {
        let owner = User::find(conn, id("kevin"))?;
        let post = ChannelThread::find(conn, post.id)?;
        assert!(!post.work_viewable_by(conn, &owner)?);
        assert!(!post.work_manageable_by(conn, &owner)?);
        Ok(())
    });
}
#[test]
fn work_candidates_follow_agent_grants_suspension_and_membership() {
    let t = channel_thread_test::frozen();
    let (room, _) = fixture(&t);
    t.write(move |tx| {
        tx.conn().execute("DELETE FROM agent_grants", [])?;
        Ok(())
    });
    let candidates = || t.read(|conn| ChannelThread::work_owner_candidates_for(conn, room.id));
    let (humans, agents) = candidates();
    assert_eq!(
        humans.iter().map(|u| u.name.as_str()).collect::<Vec<_>>(),
        ["David", "JZ", "Kevin"]
    );
    assert_eq!(
        agents.iter().map(|u| u.id).collect::<Vec<_>>(),
        [id("bender")]
    );
    let mut grant = t.write(move |tx| {
        AgentGrant::create(
            tx,
            NewGrant {
                agent_id: id("bender_agent"),
                granted_by_id: id("david"),
                room_id: Some(room.id),
                capability: "post_messages".into(),
                ..Default::default()
            },
        )
    });
    t.write(move |tx| grant.revoke(tx));
    assert!(candidates().1.is_empty());
    t.write(move |tx| {
        tx.conn().execute("DELETE FROM agent_grants", [])?;
        tx.conn().execute(
            "UPDATE agents SET suspended_at=? WHERE id=?",
            (tx.now(), id("bender_agent")),
        )?;
        Ok(())
    });
    assert!(candidates().1.is_empty());
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE agents SET suspended_at=NULL WHERE id=?",
            [id("bender_agent")],
        )?;
        Ok(())
    });
    assert_eq!(candidates().1.len(), 1);
    t.write(move |tx| {
        Membership::find_by_room_and_user(tx.conn(), room.id, id("bender"))?
            .unwrap()
            .destroy(tx)
    });
    assert!(candidates().1.is_empty());
}
#[test]
fn board_owner_availability_matches_every_owner_kind_after_access_changes() {
    let t = channel_thread_test::frozen();
    let (room, human) = fixture(&t);
    let agent = t.write(move |tx| {
        ChannelThread::create(
            tx,
            NewChannelThread {
                room_id: room.id,
                creator_id: id("jz"),
                name: Some("Agent".into()),
                work_status: Some("planned".into()),
                ..Default::default()
            },
        )
    });
    t.write(move |tx| {
        tx.conn().execute("DELETE FROM agent_grants", [])?;
        tx.conn().execute(
            "UPDATE channel_threads SET work_owner_id=? WHERE id=?",
            (id("kevin"), human.id),
        )?;
        tx.conn().execute(
            "UPDATE channel_threads SET work_owner_id=? WHERE id=?",
            (id("bender"), agent.id),
        )?;
        Ok(())
    });
    let availability = || {
        t.read(|conn| {
            let posts = [
                ChannelThread::find(conn, human.id)?,
                ChannelThread::find(conn, agent.id)?,
            ];
            ChannelThread::board_owner_active_map(conn, room.id, &posts)
        })
    };
    assert_eq!(availability().get(&id("kevin")), Some(&true));
    assert_eq!(availability().get(&id("bender")), Some(&true));
    let mut grant = t.write(move |tx| {
        AgentGrant::create(
            tx,
            NewGrant {
                agent_id: id("bender_agent"),
                granted_by_id: id("david"),
                room_id: Some(room.id),
                capability: "post_messages".into(),
                ..Default::default()
            },
        )
    });
    t.write(move |tx| {
        Membership::find_by_room_and_user(tx.conn(), room.id, id("kevin"))?
            .unwrap()
            .destroy(tx)?;
        grant.revoke(tx)
    });
    assert_eq!(availability().get(&id("kevin")), Some(&false));
    assert_eq!(availability().get(&id("bender")), Some(&false));
    assert!(
        t.read(|conn| ChannelThread::board_owner_active_map(conn, room.id, &[]))
            .is_empty()
    );
}

#[test]
fn page_agent_capabilities_match_single_checks_after_each_policy_change() {
    let t = channel_thread_test::frozen();
    let (room, _) = fixture(&t);
    let agent = id("bender_agent");
    let requests = [
        (agent, None),
        (agent, Some(room.id)),
        (agent, Some(id("watercooler"))),
        (agent, Some(-1)),
        (-1, Some(room.id)),
    ];
    let check = || {
        t.read(|conn| {
            for capability in ["post_messages", "external_action", "unknown"] {
                let batch = crate::Agent::capabilities_for_rooms(conn, capability, &requests)?;
                for &(agent, room) in &requests {
                    assert_eq!(
                        batch[&(agent, room)],
                        crate::models::agent_access::capability_for_agent(
                            conn, agent, capability, room
                        )?,
                        "{capability}, {agent}, {room:?}"
                    );
                }
            }
            Ok(())
        })
    };
    t.write(|tx| {
        tx.conn().execute("DELETE FROM agent_grants", [])?;
        Ok(())
    });
    check(); // Legacy capability set, absent agent and absent room.
    let mut grant = t.write(move |tx| {
        AgentGrant::create(
            tx,
            NewGrant {
                agent_id: agent,
                granted_by_id: id("david"),
                capability: "post_messages".into(),
                room_id: Some(room.id),
                ..Default::default()
            },
        )
    });
    check(); // A room grant does not authorize other rooms or the global scope.
    t.write(move |tx| grant.revoke(tx));
    check(); // A revoked grant never restores legacy access.
    t.write(move |tx| {
        AgentGrant::create(
            tx,
            NewGrant {
                agent_id: agent,
                granted_by_id: id("david"),
                capability: "post_messages".into(),
                ..Default::default()
            },
        )?;
        Ok(())
    });
    check(); // A workspace grant covers the global and room scopes.
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE rooms SET deleted_at=? WHERE id=?",
            (tx.now(), room.id),
        )?;
        Ok(())
    });
    check();
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE agents SET suspended_at=? WHERE id=?",
            (tx.now(), agent),
        )?;
        Ok(())
    });
    check();
    t.write(move |tx| {
        tx.conn()
            .execute("UPDATE agents SET suspended_at=NULL WHERE id=?", [agent])?;
        tx.conn()
            .execute("UPDATE users SET status=1 WHERE id=?", [id("bender")])?;
        Ok(())
    });
    check();
}

#[test]
fn page_work_owners_keep_membership_and_grants_scoped_to_each_room() {
    let t = channel_thread_test::frozen();
    let (room, first) = fixture(&t);
    let (other, second, mut grant) = t.write(move |tx| {
        let other = Room::create_for(
            tx,
            RoomType::Board,
            Some("Other work"),
            id("david"),
            &[id("david"), id("bender")],
        )?;
        let second = ChannelThread::create(
            tx,
            NewChannelThread {
                room_id: other.id,
                creator_id: id("david"),
                name: Some("Other post".into()),
                work_status: Some("planned".into()),
                ..Default::default()
            },
        )?;
        tx.conn().execute("DELETE FROM agent_grants", [])?;
        for post in [first.id, second.id] {
            tx.conn().execute(
                "UPDATE channel_threads SET work_owner_id=? WHERE id=?",
                (id("bender"), post),
            )?;
        }
        let grant = AgentGrant::create(
            tx,
            NewGrant {
                agent_id: id("bender_agent"),
                granted_by_id: id("david"),
                capability: "post_messages".into(),
                room_id: Some(room.id),
                ..Default::default()
            },
        )?;
        Ok((other, second, grant))
    });
    let available = || {
        t.read(|conn| {
            let posts = ChannelThread::for_ids(conn, &[first.id, second.id])?;
            let owners = ChannelThread::work_owners(conn, &posts)?;
            Ok([first.id, second.id]
                .map(|id| owners.available(posts.iter().find(|post| post.id == id).unwrap())))
        })
    };
    assert_eq!(available(), [true, false]);
    t.write(move |tx| {
        grant.revoke(tx)?;
        AgentGrant::create(
            tx,
            NewGrant {
                agent_id: id("bender_agent"),
                granted_by_id: id("david"),
                capability: "post_messages".into(),
                ..Default::default()
            },
        )?;
        Ok(())
    });
    assert_eq!(available(), [true, true]);
    t.write(move |tx| {
        Membership::find_by_room_and_user(tx.conn(), other.id, id("bender"))?
            .unwrap()
            .destroy(tx)
    });
    assert_eq!(available(), [true, false]);
}

#[test]
fn page_room_names_preserve_the_ordinary_sql_order_and_viewer_fallbacks() {
    let t = channel_thread_test::frozen();
    let rooms = t.write(|tx| {
        let direct = Room::find_or_create_direct_for(
            tx,
            &[id("david"), id("jason"), id("kevin")],
            id("david"),
        )?;
        tx.conn()
            .execute("UPDATE users SET name='Élodie' WHERE id=?", [id("jason")])?;
        tx.conn()
            .execute("UPDATE users SET name='Zulu' WHERE id=?", [id("kevin")])?;
        let mut named = direct.clone();
        named.name = Some("Named <&> direct".into());
        let empty = Room::create_for(tx, RoomType::Direct, None, id("david"), &[id("david")])?;
        Ok(vec![
            direct,
            named,
            empty,
            Room::find(tx.conn(), id("watercooler"))?,
        ])
    });
    t.read(|conn| {
        let viewer = User::find(conn, id("david"))?;
        for user in [None, Some(&viewer)] {
            // The named snapshot has the same id as the unnamed room; check separately.
            for room in &rooms {
                let names = Room::display_names_for(conn, std::slice::from_ref(room), user)?;
                let expected = if room.direct() {
                    room.direct_display_name(conn, user, None)?
                        .unwrap_or_default()
                } else {
                    room.name.clone().unwrap_or_default()
                };
                assert_eq!(names[&room.id], expected);
            }
            let names = Room::display_names_for(
                conn,
                &[rooms[0].clone(), rooms[2].clone(), rooms[3].clone()],
                user,
            )?;
            for room in [&rooms[0], &rooms[2], &rooms[3]] {
                let expected = if room.direct() {
                    room.direct_display_name(conn, user, None)?
                        .unwrap_or_default()
                } else {
                    room.name.clone().unwrap_or_default()
                };
                assert_eq!(names[&room.id], expected);
            }
        }
        Ok(())
    });
}

#[test]
fn board_deletion_rows_precede_a_failing_agent_ledger_callback() {
    use crate::broadcasts::Broadcast;
    let t = channel_thread_test::frozen();
    let (_, post) = fixture(&t);
    t.write(move |tx| {
        tx.conn().execute("DELETE FROM agent_grants",[])?;
        tx.conn().execute("UPDATE channel_threads SET work_owner_id=? WHERE id=?",(id("bender"),post.id))?;
        tx.conn().execute_batch("CREATE TEMP TRIGGER ws12_reject_deleted_ledger BEFORE INSERT ON agent_events WHEN NEW.event_type='work_unassigned' BEGIN SELECT RAISE(ABORT,'WS12 rejected ledger'); END")?;
        Ok(())
    });
    let from = t.events().len();
    assert!(
        t.try_write(move |tx| ChannelThread::find(tx.conn(), post.id)?.destroy(tx))
            .is_err()
    );
    assert!(t.events()[from..].iter().any(|event| event.as_broadcast() == Some(Broadcast::ThreadRemoved { thread_id: post.id, room_id: post.room_id })));
    assert!(
        t.read(|conn| ChannelThread::find_by_id(conn, post.id))
            .is_none()
    );
}
