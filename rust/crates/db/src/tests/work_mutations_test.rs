//! Human work/result writes and recorder contracts from our Rails models and recorder tests.
use super::*;
use crate::models::{
    activity_item::ActivitySource,
    channel_thread::{WORK_UPDATE_FORBIDDEN, WorkChanges},
};
use crate::{
    ActivityItem, ChannelThread, Error, NewChannelThread, Room, RoomType, User, WorkThreadEvent,
};
use serde_json::json;

fn setup() -> (TestDb, ChannelThread) {
    let t = channel_thread_test::frozen();
    let thread = t.write(|tx| {
        let room = Room::create_for(
            tx,
            RoomType::Board,
            Some("Launch"),
            id("david"),
            &[id("david"), id("jz"), id("kevin"), id("bender")],
        )?;
        ChannelThread::create_board_post(
            tx,
            NewChannelThread {
                room_id: room.id,
                creator_id: id("jz"),
                name: Some("Plan".into()),
                work_status: Some("planned".into()),
                work_owner_id: Some(id("kevin")),
                ..Default::default()
            },
            None,
        )
    });
    (t, thread)
}
fn work(
    t: &TestDb,
    thread: &ChannelThread,
    actor: &str,
    changes: WorkChanges,
) -> Result<ChannelThread> {
    let mut stale = thread.clone();
    let actor = id(actor);
    t.try_write(move |tx| {
        let actor = User::find(tx.conn(), actor)?;
        stale.update_work(tx, &actor, changes)?;
        Ok(stale)
    })
}
fn events(t: &TestDb, thread: i64) -> Vec<WorkThreadEvent> {
    t.read(|conn| WorkThreadEvent::for_thread(conn, thread))
}
#[test]
fn owner_status_changes_stamp_and_snapshot_one_event_per_real_change() {
    let (t, thread) = setup();
    t.travel(60);
    let updated = work(
        &t,
        &thread,
        "kevin",
        WorkChanges {
            status: Some(Some("in_progress".into())),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(updated.work_status_changed_at, Some(t.now()));
    let before = events(&t, thread.id);
    assert_eq!(before.len(), 2);
    assert_eq!(
        before[0].metadata,
        json!({"before":{"status":"planned","owner":{"id":id("kevin"),"name":"Kevin","status":"active","role":"member"}},"after":{"status":"in_progress","owner":{"id":id("kevin"),"name":"Kevin","status":"active","role":"member"}},"actor":{"id":id("kevin"),"name":"Kevin","status":"active","role":"member"},"note":null})
    );
    t.travel(60);
    let noop = work(
        &t,
        &thread,
        "kevin",
        WorkChanges {
            status: Some(Some("in_progress".into())),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(noop.updated_at, updated.updated_at);
    assert_eq!(events(&t, thread.id).len(), 2);
}
#[test]
fn owner_cannot_reassign_even_to_self_or_remove_tracking() {
    let (t, thread) = setup();
    for changes in [
        WorkChanges {
            owner_id: Some(json!(id("kevin"))),
            ..Default::default()
        },
        WorkChanges {
            status: Some(None),
            owner_id: Some(json!(null)),
        },
    ] {
        assert!(
            matches!(work(&t,&thread,"kevin",changes),Err(Error::Other(message)) if message==WORK_UPDATE_FORBIDDEN)
        );
    }
    assert!(
        matches!(work(&t,&thread,"jz",WorkChanges {status:Some(None),owner_id:Some(json!(null))}),Err(Error::RecordInvalid(errors)) if errors.on("work_status")==["must be tracked in a board"])
    );
    assert_eq!(events(&t, thread.id).len(), 1);
}
#[test]
fn ordinary_threads_convert_and_untrack_only_through_managers() {
    let t = channel_thread_test::frozen();
    let thread =
        channel_thread_test::create_thread(&t, "designers", "jz", None, Some("Discussion"));
    assert!(matches!(
        work(
            &t,
            &thread,
            "kevin",
            WorkChanges {
                status: Some(Some("planned".into())),
                ..Default::default()
            }
        ),
        Err(Error::Other(_))
    ));
    let converted = work(
        &t,
        &thread,
        "jz",
        WorkChanges {
            status: Some(Some("planned".into())),
            owner_id: Some(json!(id("kevin"))),
        },
    )
    .unwrap();
    let removed = work(
        &t,
        &converted,
        "jz",
        WorkChanges {
            status: Some(None),
            owner_id: Some(json!(null)),
        },
    )
    .unwrap();
    assert!(!removed.work());
    assert_eq!(removed.work_owner_id, None);
    assert_eq!(events(&t, thread.id).len(), 2);
}
#[test]
fn fresh_ownership_is_rechecked_and_stale_other_columns_survive() {
    let (t, thread) = setup();
    let reassigned = work(
        &t,
        &thread,
        "jz",
        WorkChanges {
            owner_id: Some(json!(id("david"))),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(matches!(
        work(
            &t,
            &thread,
            "kevin",
            WorkChanges {
                status: Some(Some("done".into())),
                ..Default::default()
            }
        ),
        Err(Error::Other(_))
    ));
    let updated = work(
        &t,
        &thread,
        "jz",
        WorkChanges {
            status: Some(Some("done".into())),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(updated.work_owner_id, reassigned.work_owner_id);
    let mut stale = thread.clone();
    t.write(move |tx| stale.update_settings(tx, Some("Renamed"), None));
    let persisted = t.read(|conn| ChannelThread::find(conn, thread.id));
    assert_eq!(persisted.work_status.as_deref(), Some("done"));
    assert_eq!(persisted.work_owner_id, Some(id("david")));
}
#[test]
fn eligible_owner_validation_uses_ws11_grants_and_only_changed_assignments() {
    let (t, thread) = setup();
    t.write(|tx| {
        tx.conn().execute(
            "UPDATE agents SET suspended_at=? WHERE user_id=?",
            (tx.now(), id("bender")),
        )?;
        Ok(())
    });
    let rejected = work(
        &t,
        &thread,
        "jz",
        WorkChanges {
            owner_id: Some(json!(id("bender"))),
            ..Default::default()
        },
    );
    assert!(
        matches!(rejected,Err(Error::RecordInvalid(errors)) if errors.on("work_owner")==["must be an active agent member of the parent room with permission to post"])
    );
    t.write(|tx| {
        tx.conn().execute(
            "UPDATE agents SET suspended_at=NULL WHERE user_id=?",
            [id("bender")],
        )?;
        Ok(())
    });
    let assigned = work(
        &t,
        &thread,
        "jz",
        WorkChanges {
            owner_id: Some(json!(id("bender"))),
            ..Default::default()
        },
    )
    .unwrap();
    t.write(move |tx| {
        tx.conn().execute(
            "DELETE FROM memberships WHERE room_id=? AND user_id=?",
            (thread.room_id, id("bender")),
        )?;
        Ok(())
    });
    let updated = work(
        &t,
        &assigned,
        "jz",
        WorkChanges {
            status: Some(Some("done".into())),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(updated.work_owner_id, Some(id("bender")));
}
#[test]
fn rejected_audit_or_agent_ledger_rolls_back_the_whole_work_change() {
    for table in ["work_thread_events", "agent_events"] {
        let (t, thread) = setup();
        t.write(move |tx| {tx.conn().execute_batch(&format!("CREATE TRIGGER reject_work BEFORE INSERT ON {table} BEGIN SELECT RAISE(ABORT,'reject work audit'); END"))?;Ok(())});
        assert!(
            work(
                &t,
                &thread,
                "jz",
                WorkChanges {
                    owner_id: Some(json!(id("bender"))),
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert_eq!(
            t.read(|conn| ChannelThread::find(conn, thread.id))
                .work_owner_id,
            thread.work_owner_id
        );
        assert_eq!(events(&t, thread.id).len(), 1);
        assert_eq!(
            t.read(|conn| crate::sql::count(
                conn,
                "SELECT COUNT(*) FROM agent_events WHERE event_type='work_assigned'",
                []
            )),
            0
        );
    }
}

#[test]
fn agent_assignment_and_unassignment_use_ws11_ledger_even_after_access_is_revoked() {
    let (t, thread) = setup();
    let assigned = work(
        &t,
        &thread,
        "jz",
        WorkChanges {
            owner_id: Some(json!(id("bender"))),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        t.read(|conn| crate::sql::count(
            conn,
            "SELECT COUNT(*) FROM agent_events WHERE event_type='work_assigned'",
            []
        )),
        1
    );
    t.write(move |tx| {
        tx.conn().execute(
            "DELETE FROM memberships WHERE room_id=? AND user_id=?",
            (thread.room_id, id("bender")),
        )?;
        Ok(())
    });
    t.sink.take();
    let cleared = work(
        &t,
        &assigned,
        "jz",
        WorkChanges {
            owner_id: Some(json!(null)),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(cleared.work_owner_id, None);
    let ledger = t.read(|conn| {
        crate::sql::query_all(
            conn,
            "SELECT event_type,webhook_status FROM agent_events WHERE event_type='work_unassigned'",
            [],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        )
    });
    assert_eq!(ledger, [("work_unassigned".into(), "none".into())]);
    assert_eq!(events(&t, thread.id).len(), 3);
}
#[test]
fn after_commit_recipient_failure_keeps_work_and_prior_recipient_item() {
    let (t, thread) = setup();
    t.write(move |tx| {let mut follower=crate::ThreadMembership::join(tx,thread.id,id("david"))?; follower.update_involvement(tx,crate::ThreadInvolvement::Everything)?;
        tx.conn().execute_batch(&format!("CREATE TRIGGER reject_recipient BEFORE INSERT ON activity_items WHEN NEW.user_id={} AND NEW.source_type='WorkThreadEvent' BEGIN SELECT RAISE(ABORT,'reject follower'); END",id("david")))?;Ok(())});
    assert!(
        work(
            &t,
            &thread,
            "kevin",
            WorkChanges {
                status: Some(Some("done".into())),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(
        t.read(|conn| ChannelThread::find(conn, thread.id))
            .work_status
            .as_deref(),
        Some("done")
    );
    let event = events(&t, thread.id).remove(0);
    assert!(
        t.read(|conn| ActivityItem::find_by_user_and_source(
            conn,
            id("jz"),
            "WorkThreadEvent",
            event.id
        ))
        .is_some()
    );
}
#[test]
fn results_normalize_blank_enforce_permission_and_length_and_notify_creator_owner_only() {
    let (t, thread) = setup();
    t.write(move |tx| {
        let mut follower = crate::ThreadMembership::join(tx, thread.id, id("david"))?;
        follower.update_involvement(tx, crate::ThreadInvolvement::Everything)?;
        Ok(())
    });
    let mut post = thread.clone();
    let text = "é".repeat(220);
    t.write(move |tx| post.update_result(tx, &User::find(tx.conn(), id("kevin"))?, Some(text)));
    let event = events(&t, thread.id).remove(0);
    assert_eq!(event.event_type, "result_updated");
    assert_eq!(
        event.metadata["excerpt"].as_str().unwrap().chars().count(),
        200
    );
    assert_eq!(t.read(|conn| event.recipient_user_ids(conn)), [id("jz")]);
    assert!(
        t.read(|conn| ActivityItem::find_by_user_and_source(
            conn,
            id("david"),
            "WorkThreadEvent",
            event.id
        ))
        .is_none()
    );
    let current = t.read(|conn| ChannelThread::find(conn, thread.id));
    let mut noop = current.clone();
    t.write(move |tx| {
        noop.update_result(
            tx,
            &User::find(tx.conn(), id("kevin"))?,
            Some("é".repeat(220)),
        )
    });
    assert_eq!(events(&t, thread.id).len(), 2);
    let mut too_long = current.clone();
    assert!(matches!(
        t.try_write(move |tx| too_long.update_result(
            tx,
            &User::find(tx.conn(), id("kevin"))?,
            Some("é".repeat(20001))
        )),
        Err(Error::RecordInvalid(_))
    ));
    let mut unauthorized = current.clone();
    assert!(matches!(
        t.try_write(move |tx| unauthorized.update_result(
            tx,
            &User::find(tx.conn(), id("jason"))?,
            current.result_markdown.clone()
        )),
        Err(Error::Other(_))
    ));
    let mut clear = t.read(|conn| ChannelThread::find(conn, thread.id));
    t.write(move |tx| {
        clear.update_result(
            tx,
            &User::find(tx.conn(), id("kevin"))?,
            Some(" \n ".into()),
        )
    });
    assert!(
        t.read(|conn| ChannelThread::find(conn, thread.id))
            .result_markdown
            .is_none()
    );
    assert_eq!(events(&t, thread.id).len(), 3);
}
#[test]
fn concurrent_stale_instances_produce_one_event_for_identical_work_changes() {
    let (t, thread) = setup();
    let second = t.another_process();
    let actor = t.read(|conn| User::find(conn, id("kevin")));
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let results = std::thread::scope(|scope| {
        let b = barrier.clone();
        let mut first = thread.clone();
        let first_actor = actor.clone();
        let db = &t.db;
        let a = scope.spawn(move || {
            b.wait();
            db.write_blocking(move |tx| {
                first.update_work(
                    tx,
                    &first_actor,
                    WorkChanges {
                        status: Some(Some("done".into())),
                        ..Default::default()
                    },
                )
            })
        });
        let mut other = thread.clone();
        let b = scope.spawn(move || {
            barrier.wait();
            second.write_blocking(move |tx| {
                other.update_work(
                    tx,
                    &actor,
                    WorkChanges {
                        status: Some(Some("done".into())),
                        ..Default::default()
                    },
                )
            })
        });
        [a.join().unwrap(), b.join().unwrap()]
    });
    assert!(results.iter().all(Result::is_ok));
    assert_eq!(events(&t, thread.id).len(), 2);
}
#[test]
fn concurrent_recorders_are_idempotent_and_keep_handled_sources_handled() {
    let (t, thread) = setup();
    let event = events(&t, thread.id).remove(0);
    let item = t
        .read(|conn| {
            ActivityItem::find_by_user_and_source(conn, id("kevin"), "WorkThreadEvent", event.id)
        })
        .unwrap();
    t.write(move |tx| item.mark_handled(tx));
    let second = t.another_process();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    std::thread::scope(|scope| {
        let b = barrier.clone();
        let db = &t.db;
        let event_id = event.id;
        let a = scope.spawn(move || {
            b.wait();
            db.write_blocking(move |tx| {
                ActivityItem::record(
                    tx,
                    id("kevin"),
                    ActivitySource::WorkThreadEvent(event_id),
                    "work_assignment",
                    false,
                )
            })
            .unwrap()
        });
        let b = scope.spawn(move || {
            barrier.wait();
            second
                .write_blocking(move |tx| {
                    ActivityItem::record(
                        tx,
                        id("kevin"),
                        ActivitySource::WorkThreadEvent(event_id),
                        "work_assignment",
                        false,
                    )
                })
                .unwrap()
        });
        assert!(a.join().unwrap().unwrap().handled());
        assert!(b.join().unwrap().unwrap().handled());
    });
    assert_eq!(t.read(|conn|crate::sql::count(conn,"SELECT COUNT(*) FROM activity_items WHERE source_type='WorkThreadEvent' AND source_id=? AND user_id=?",[event.id,id("kevin")])),1);
}

#[test]
fn work_updates_repoint_one_unhandled_item_and_handled_updates_start_a_new_item() {
    let (t, thread) = setup();
    let a = work(
        &t,
        &thread,
        "kevin",
        WorkChanges {
            status: Some(Some("in_progress".into())),
            ..Default::default()
        },
    )
    .unwrap();
    let first = events(&t, thread.id).remove(0);
    let item = t
        .read(|conn| {
            ActivityItem::find_by_user_and_source(conn, id("jz"), "WorkThreadEvent", first.id)
        })
        .unwrap();
    let read = item.clone();
    t.write(move |tx| read.mark_read(tx));
    t.travel(60);
    let b = work(
        &t,
        &a,
        "kevin",
        WorkChanges {
            status: Some(Some("blocked".into())),
            ..Default::default()
        },
    )
    .unwrap();
    let second = events(&t, thread.id).remove(0);
    let grouped = t.read(|conn| ActivityItem::find(conn, item.id));
    assert_eq!(grouped.source_id, second.id);
    assert!(grouped.unread());
    assert_eq!(
        t.read(|conn| crate::sql::count(
            conn,
            "SELECT COUNT(*) FROM activity_items WHERE user_id=? AND event_type='work_update'",
            [id("jz")]
        )),
        1
    );
    t.write(move |tx| grouped.mark_handled(tx));
    work(
        &t,
        &b,
        "kevin",
        WorkChanges {
            status: Some(Some("done".into())),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        t.read(|conn| crate::sql::count(
            conn,
            "SELECT COUNT(*) FROM activity_items WHERE user_id=? AND event_type='work_update'",
            [id("jz")]
        )),
        2
    );
}

#[test]
fn owner_integer_coercions_match_ruby_and_bad_values_are_invalid() {
    use crate::models::channel_thread::normalize_owner_id;
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("../../../../vectors/boards_write.json")).unwrap();
    for row in oracle["coercions"].as_array().unwrap() {
        let result = normalize_owner_id(&row["input"]);
        if row["invalid"] == true {
            assert!(matches!(result, Err(Error::RecordInvalid(_))), "{row}");
        } else {
            assert_eq!(result.unwrap(), row["normalized"].as_i64(), "{row}");
        }
    }
}

#[test]
fn work_recipients_honor_current_room_thread_and_agent_assignment_preferences() {
    for (agent, preference, recorded) in [
        (true, false, false),
        (true, true, true),
        (false, false, true),
    ] {
        let (t, thread) = setup();
        t.write(move |tx| {
            tx.conn().execute(
                "UPDATE users SET inbox_preferences=? WHERE id=?",
                (json!({"agent_work":preference}), id("kevin")),
            )?;
            if !agent {
                tx.conn()
                    .execute("DELETE FROM agents WHERE user_id=?", [id("bender")])?;
            }
            Ok(())
        });
        let after = thread.clone();
        let mut before = thread.clone();
        before.work_owner_id = None;
        let event = t
            .write(move |tx| {
                WorkThreadEvent::create_for_change(
                    tx,
                    &before,
                    &after,
                    Some(&User::find(tx.conn(), id("bender"))?),
                    None,
                )
            })
            .unwrap();
        assert_eq!(
            t.read(|conn| ActivityItem::find_by_user_and_source(
                conn,
                id("kevin"),
                "WorkThreadEvent",
                event.id
            ))
            .is_some(),
            recorded
        );
    }
    for mode in [
        "nothing",
        "invisible",
        "thread-muted",
        "inactive",
        "removed",
    ] {
        let (t, thread) = setup();
        t.write(move |tx| {
            match mode {
                "thread-muted" => {
                    let mut member = crate::ThreadMembership::join(tx, thread.id, id("jz"))?;
                    member.update_involvement(tx, crate::ThreadInvolvement::Nothing)?;
                }
                "inactive" => {
                    tx.conn()
                        .execute("UPDATE users SET status=1 WHERE id=?", [id("jz")])?;
                }
                "removed" => {
                    tx.conn().execute(
                        "DELETE FROM memberships WHERE room_id=? AND user_id=?",
                        (thread.room_id, id("jz")),
                    )?;
                }
                value => {
                    tx.conn().execute(
                        "UPDATE memberships SET involvement=? WHERE room_id=? AND user_id=?",
                        (value, thread.room_id, id("jz")),
                    )?;
                }
            }
            Ok(())
        });
        work(
            &t,
            &thread,
            "kevin",
            WorkChanges {
                status: Some(Some("done".into())),
                ..Default::default()
            },
        )
        .unwrap();
        let event = events(&t, thread.id).remove(0);
        assert!(
            t.read(|conn| event.recipient_user_ids(conn)).is_empty(),
            "{mode}"
        );
    }
}
