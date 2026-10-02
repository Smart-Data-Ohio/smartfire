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
    let rows = t.events()[from..]
        .iter()
        .filter_map(|event| match event.as_broadcast()? {
            Broadcast::Turbo(frame) if frame.target.starts_with("board_") => Some(frame.target),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        rows,
        [
            format!("board_row_channel_thread_{}", post.id),
            format!("board_column_row_channel_thread_{}", post.id)
        ]
    );
    assert!(
        t.read(|conn| ChannelThread::find_by_id(conn, post.id))
            .is_none()
    );
}
