use super::*;
use crate::{
    AgentGrant, BoardTagAssignment, ChannelThread, Error, NewBoardTagAssignment, NewChannelThread,
    NewGrant, Room, RoomType, User, WorkThreadEvent,
};
use rusqlite::params;
use serde_json::{Value, json};

fn setup() -> (TestDb, Room) {
    let t = channel_thread_test::frozen();
    let room = t.write(|tx| {
        let room = Room::create_for(
            tx,
            RoomType::Board,
            Some("Rules"),
            id("david"),
            &[id("david"), id("jz"), id("bender")],
        )?;
        tx.conn().execute("DELETE FROM agent_grants", [])?;
        for cap in ["read_messages", "post_messages"] {
            AgentGrant::create(
                tx,
                NewGrant {
                    agent_id: id("bender_agent"),
                    room_id: Some(room.id),
                    capability: cap.into(),
                    granted_by_id: id("david"),
                    ..Default::default()
                },
            )?;
        }
        Ok(room)
    });
    (t, room)
}
fn rule(t: &TestDb, room: i64, tag: &str, assignee: &str) -> BoardTagAssignment {
    let tag = tag.to_owned();
    let assignee = id(assignee);
    t.write(move |tx| {
        BoardTagAssignment::create(
            tx,
            NewBoardTagAssignment {
                room_id: room,
                tag,
                assignee_id: assignee,
                created_by_id: id("david"),
            },
        )
    })
}
fn post(t: &TestDb, room: i64, tags: Vec<String>, owner: Option<i64>) -> ChannelThread {
    t.write(move |tx| {
        ChannelThread::create(
            tx,
            NewChannelThread {
                room_id: room,
                creator_id: id("david"),
                name: Some("Fix it".into()),
                work_status: Some("planned".into()),
                work_owner_id: owner,
                tag_names: Some(tags),
                ..Default::default()
            },
        )
    })
}

#[test]
fn ws12_tag_assignment_validations_match_rails() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../vectors/board_tag_assignments_contract.json"
    ))
    .unwrap();
    for row in oracle["rows"].as_array().unwrap() {
        let (t, board) = setup();
        let input = row.clone();
        let actual = t.write(move |tx| {
            let flags = input["flags"].as_array().unwrap();
            let has = |flag: &str| flags.contains(&json!(flag));
            if has("duplicate") || has("other_board") {
                BoardTagAssignment::create(
                    tx,
                    NewBoardTagAssignment {
                        room_id: board.id,
                        tag: "bug".into(),
                        assignee_id: id("jz"),
                        created_by_id: id("david"),
                    },
                )?;
            }
            if has("deactivated") {
                tx.conn()
                    .execute("UPDATE users SET status=1 WHERE id=?", [id("jz")])?;
            }
            if has("suspended") {
                tx.conn().execute(
                    "UPDATE agents SET suspended_at=? WHERE id=?",
                    params![tx.now(), id("bender_agent")],
                )?;
            }
            for (flag, cap) in [("no_post", "post_messages"), ("no_read", "read_messages")] {
                if has(flag) {
                    tx.conn().execute(
                        "UPDATE agent_grants SET revoked_at=? WHERE agent_id=? AND capability=?",
                        params![tx.now(), id("bender_agent"), cap],
                    )?;
                }
            }
            let room_id = if has("other_board") {
                Room::create_for(
                    tx,
                    RoomType::Board,
                    Some("Other"),
                    id("david"),
                    &[id("david"), id("jz")],
                )?
                .id
            } else if has("channel") {
                id("watercooler")
            } else if has("missing_room") {
                0
            } else {
                board.id
            };
            let tag = input["tag"].as_str().unwrap().to_owned();
            let normalized = rails_compat::unicode::downcase(campfire_richtext::ruby::strip(&tag));
            match BoardTagAssignment::create(
                tx,
                NewBoardTagAssignment {
                    room_id,
                    tag,
                    assignee_id: input["assignee_id"].as_i64().unwrap(),
                    created_by_id: if has("missing_creator") {
                        0
                    } else {
                        id("david")
                    },
                },
            ) {
                Ok(rule) => Ok(json!({"valid":true,"tag":rule.tag,"errors":[]})),
                Err(Error::RecordInvalid(errors)) => {
                    Ok(json!({"valid":false,"tag":normalized,"errors":errors.full_messages()}))
                }
                Err(error) => Err(error),
            }
        });
        assert_eq!(actual, row["expected"], "{}", row["name"]);
    }
}

#[test]
fn added_tag_later_assigns_but_unchanged_tags_and_existing_owners_do_not() {
    let (t, room) = setup();
    let initial = post(&t, room.id, vec!["bug".into()], None);
    rule(&t, room.id, "bug", "jz");
    let thread_id = initial.id;
    t.write(move |tx| {
        ChannelThread::find(tx.conn(), thread_id)?.update_metadata(
            tx,
            None,
            None,
            Some(&["bug".into()]),
        )
    });
    assert_eq!(
        t.read(move |conn| ChannelThread::find(conn, thread_id))
            .work_owner_id,
        None
    );
    t.write(move |tx| {
        ChannelThread::find(tx.conn(), thread_id)?.update_metadata(tx, None, None, Some(&[]))
    });
    t.travel(3600);
    t.write(move |tx| {
        ChannelThread::find(tx.conn(), thread_id)?.update_metadata(
            tx,
            None,
            None,
            Some(&["bug".into()]),
        )
    });
    let assigned = t.read(move |conn| ChannelThread::find(conn, thread_id));
    assert_eq!(assigned.work_owner_id, Some(id("jz")));
    assert_eq!(
        assigned.work_status_changed_at,
        initial.work_status_changed_at
    );
    for owner in [id("david"), id("bender")] {
        let owned = post(&t, room.id, vec!["bug".into()], Some(owner));
        assert_eq!(
            t.read(move |conn| ChannelThread::find(conn, owned.id))
                .work_owner_id,
            Some(owner)
        );
        assert!(
            t.read(move |conn| WorkThreadEvent::for_thread(conn, owned.id))
                .is_empty()
        );
    }
}

#[test]
fn assignment_uses_first_lexical_rule_and_does_not_try_another_after_revocation() {
    let (t, room) = setup();
    rule(&t, room.id, "z-last", "david");
    rule(&t, room.id, "a-first", "jz");
    let first = post(&t, room.id, vec!["z-last".into(), "a-first".into()], None);
    assert_eq!(
        t.read(move |conn| ChannelThread::find(conn, first.id))
            .work_owner_id,
        Some(id("jz"))
    );
    t.write(move |tx| {
        tx.conn().execute(
            "DELETE FROM memberships WHERE room_id=? AND user_id=?",
            params![room.id, id("jz")],
        )?;
        Ok(())
    });
    let next = post(&t, room.id, vec!["z-last".into(), "a-first".into()], None);
    assert_eq!(
        t.read(move |conn| ChannelThread::find(conn, next.id))
            .work_owner_id,
        None
    );
}

#[test]
fn auto_assignment_rechecks_agent_membership_activity_and_both_grants() {
    for flag in ["outside", "suspended", "inactive", "no_post", "no_read"] {
        let (t, room) = setup();
        rule(&t, room.id, "bug", "bender");
        t.write(move |tx| {
            match flag {
                "outside" => {
                    tx.conn().execute(
                        "DELETE FROM memberships WHERE room_id=? AND user_id=?",
                        params![room.id, id("bender")],
                    )?;
                }
                "suspended" => {
                    tx.conn().execute(
                        "UPDATE agents SET suspended_at=? WHERE id=?",
                        params![tx.now(), id("bender_agent")],
                    )?;
                }
                "inactive" => {
                    tx.conn()
                        .execute("UPDATE users SET status=1 WHERE id=?", [id("bender")])?;
                }
                _ => {
                    tx.conn().execute(
                        "UPDATE agent_grants SET revoked_at=? WHERE agent_id=? AND capability=?",
                        params![
                            tx.now(),
                            id("bender_agent"),
                            if flag == "no_post" {
                                "post_messages"
                            } else {
                                "read_messages"
                            }
                        ],
                    )?;
                }
            }
            Ok(())
        });
        let post = post(&t, room.id, vec!["bug".into()], None);
        assert_eq!(
            t.read(move |conn| ChannelThread::find(conn, post.id))
                .work_owner_id,
            None,
            "{flag}"
        );
        assert!(
            t.read(move |conn| WorkThreadEvent::for_thread(conn, post.id))
                .is_empty(),
            "{flag}"
        );
    }
}

#[test]
fn auto_assignment_uses_ws11_ledger_and_failure_keeps_the_primary_tag_write() {
    let (t, room) = setup();
    rule(&t, room.id, "bug", "bender");
    let assigned = post(&t, room.id, vec!["bug".into()], None);
    let events=t.read(move |conn|crate::sql::query_all(conn,"SELECT event_type,actor_id,webhook_status FROM agent_events WHERE json_extract(metadata,'$.thread_id')=?",[assigned.id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,Option<i64>>(1)?,r.get::<_,String>(2)?))));
    assert_eq!(events, [("work_assigned".into(), None, "pending".into())]);
    t.write(|tx|{tx.conn().execute_batch("CREATE TEMP TRIGGER reject_auto_audit BEFORE INSERT ON work_thread_events WHEN NEW.actor_id IS NULL BEGIN SELECT RAISE(ABORT,'rejected auto audit'); END")?;Ok(())});
    let kept = post(&t, room.id, vec!["bug".into()], None);
    let (owner, tags, events) = t.read(move |conn| {
        let thread = ChannelThread::find(conn, kept.id)?;
        Ok((
            thread.work_owner_id,
            thread.tag_names(conn)?,
            WorkThreadEvent::for_thread(conn, kept.id)?,
        ))
    });
    assert_eq!(owner, None);
    assert_eq!(tags, ["bug"]);
    assert!(events.is_empty());
}

#[test]
fn auto_assignment_skips_channels_and_nested_rollback_discards_pending_callbacks() {
    let (t, room) = setup();
    rule(&t, room.id, "bug", "jz");
    let ordinary = t.write(|tx| {
        ChannelThread::create(
            tx,
            NewChannelThread {
                room_id: id("watercooler"),
                creator_id: id("david"),
                name: Some("Chat".into()),
                work_status: Some("planned".into()),
                tag_names: Some(vec!["bug".into()]),
                ..Default::default()
            },
        )
    });
    assert_eq!(
        t.read(move |conn| ChannelThread::find(conn, ordinary.id))
            .work_owner_id,
        None
    );
    let post = post(&t, room.id, vec![], None);
    let thread_id = post.id;
    t.write(move |tx| {
        let rollback = tx.savepoint(|tx| {
            ChannelThread::find(tx.conn(), thread_id)?.update_metadata(
                tx,
                None,
                None,
                Some(&["bug".into()]),
            )?;
            Err::<(), _>(Error::Other("rollback".into()))
        });
        assert!(rollback.is_err());
        Ok(())
    });
    let row = t.read(move |conn| ChannelThread::find(conn, thread_id));
    assert_eq!(row.work_owner_id, None);
    assert!(t.read(move |conn| row.tag_names(conn)).is_empty());
}

#[test]
fn rule_update_and_destroy_preserve_rails_timestamps_and_validate_new_assignees() {
    let (t, room) = setup();
    let mut rule = rule(&t, room.id, "bug", "jz");
    let initial = rule.updated_at;
    t.travel(60);
    rule = t.write(move |tx| {
        rule.update(tx, " BUG ", id("jz"))?;
        Ok(rule)
    });
    assert_eq!(rule.updated_at, initial);
    let mut invalid = rule.clone();
    assert!(matches!(
        t.try_write(move |tx| invalid.update(tx, "bug", id("kevin"))),
        Err(Error::RecordInvalid(_))
    ));
    rule = t.write(move |tx| {
        rule.update(tx, "feature", id("david"))?;
        Ok(rule)
    });
    assert_eq!(rule.updated_at, t.now());
    let id = rule.id;
    t.write(move |tx| rule.destroy(tx));
    assert!(
        t.read(move |conn| BoardTagAssignment::find(conn, id))
            .is_none()
    );
    assert!(
        t.read(|conn| User::find_by_id(conn, super::id("jz")))
            .is_some()
    );
}

#[test]
fn newly_added_tag_assigns_after_commit_and_records_the_system_actor() {
    let t = channel_thread_test::frozen();
    let room = t.write(|tx| {
        let room = Room::create_for(tx, RoomType::Board, Some("Launch"), id("david"), &[id("david"), id("jz")])?;
        tx.conn().execute("INSERT INTO board_tag_assignments(room_id,tag,assignee_id,created_by_id,created_at,updated_at) VALUES (?,?,?,?,?,?)",params![room.id,"bug",id("jz"),id("david"),tx.now(),tx.now()])?;
        Ok(room)
    });
    let post = t.write(move |tx| {
        ChannelThread::create(
            tx,
            NewChannelThread {
                room_id: room.id,
                creator_id: id("david"),
                name: Some("Fix it".into()),
                work_status: Some("planned".into()),
                tag_names: Some(vec![" Bug ".into()]),
                ..Default::default()
            },
        )
    });
    let fresh = t.read(move |conn| ChannelThread::find(conn, post.id));
    assert_eq!(fresh.work_owner_id, Some(id("jz")));
    let events = t.read(move |conn| WorkThreadEvent::for_thread(conn, post.id));
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].actor_id, None);
    assert_eq!(
        events[0].metadata["note"],
        "Auto-assigned by board tag rule"
    );
}

#[test]
fn stale_rule_edits_preserve_the_other_writers_dirty_columns() {
    let (t, room) = setup();
    let mut tag_writer = rule(&t, room.id, "bug", "jz");
    let mut owner_writer = tag_writer.clone();
    let rule_id = tag_writer.id;
    t.write(move |tx| tag_writer.update(tx, "feature", id("jz")));
    t.write(move |tx| owner_writer.update(tx, "bug", id("david")));
    let fresh = t
        .read(move |conn| BoardTagAssignment::find(conn, rule_id))
        .unwrap();
    assert_eq!(fresh.tag, "feature");
    assert_eq!(fresh.assignee_id, id("david"));
    let mut tag_writer = fresh.clone();
    let mut owner_writer = fresh;
    t.write(move |tx| owner_writer.update(tx, "feature", id("jz")));
    t.write(move |tx| tag_writer.update(tx, "urgent", id("david")));
    let fresh = t
        .read(move |conn| BoardTagAssignment::find(conn, rule_id))
        .unwrap();
    assert_eq!(fresh.tag, "urgent");
    assert_eq!(fresh.assignee_id, id("jz"));
}
