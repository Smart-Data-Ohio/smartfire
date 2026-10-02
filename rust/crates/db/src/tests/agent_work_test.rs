//! Real service/model writes compared to the pinned Rails producer, independent of HTTP.
use super::*;
use crate::models::channel_thread::AgentWorkChanges;
use crate::models::{agent_delivery::AgentEvent, agent_work as service, audit_log};
use crate::{
    Agent, AgentGrant, ChannelThread, HandoffPackage, NewChannelThread, NewGrant, Room, Timestamp,
    WorkThreadEvent,
};
use rusqlite::params;
use serde_json::{Value, json};

const BOARD: i64 = 900082020;
const RECEIVER_USER: i64 = 900082030;
const RECEIVER: i64 = 900082031;

fn setup() -> TestDb {
    let t = channel_thread_test::frozen();
    t.clock
        .travel_to(Timestamp::parse_db("2026-03-02 16:00:00").unwrap());
    t.write(|tx| {
        tx.conn().execute("INSERT INTO rooms(id,type,name,creator_id,created_at,updated_at) VALUES (?,'Rooms::Board','Service board',?,?,?)",params![BOARD,id("david"),tx.now(),tx.now()])?;
        tx.conn().execute("INSERT INTO users(id,name,role,status,created_at,updated_at) VALUES (?,'Receiver',2,0,?,?)",params![RECEIVER_USER,tx.now(),tx.now()])?;
        tx.conn().execute("INSERT INTO agents(id,user_id,owner_id,kind,status,created_at,updated_at) VALUES (?,?,?,'workspace','idle',?,?)",params![RECEIVER,RECEIVER_USER,id("david"),tx.now(),tx.now()])?;
        crate::Webhook::create(tx,RECEIVER_USER,Some("https://receiver.example.test/hook"))?;
        Room::find(tx.conn(),BOARD)?.grant_to(tx,&[id("david"),id("kevin"),id("bender"),RECEIVER_USER])?;
        Ok(())
    });
    t
}
fn grants(tx: &mut Tx<'_>) -> Result<()> {
    tx.conn().execute("DELETE FROM agent_grants", [])?;
    tx.conn().execute(
        "UPDATE agents SET suspended_at=NULL,daily_board_post_cap=NULL WHERE id IN (?,?)",
        params![id("bender_agent"), RECEIVER],
    )?;
    tx.conn()
        .execute("UPDATE users SET status=0 WHERE id=?", [id("bender")])?;
    Room::find(tx.conn(), BOARD)?.grant_to(tx, &[id("bender"), RECEIVER_USER])?;
    for agent_id in [id("bender_agent"), RECEIVER] {
        for cap in ["read_messages", "post_messages", "manage_threads"] {
            AgentGrant::create(
                tx,
                NewGrant {
                    agent_id,
                    capability: cap.into(),
                    room_id: Some(BOARD),
                    granted_by_id: id("david"),
                    ..Default::default()
                },
            )?;
        }
    }
    Ok(())
}
fn flags(tx: &mut Tx<'_>, thread: Option<i64>, flags: &[Value]) -> Result<()> {
    for flag in flags {
        match flag.as_str().unwrap() {
            "nonmember" => {
                tx.conn().execute(
                    "DELETE FROM memberships WHERE room_id=? AND user_id=?",
                    params![BOARD, id("bender")],
                )?;
            }
            "unreadable" | "no_manage" => {
                tx.conn().execute(
                    "UPDATE agent_grants SET revoked_at=? WHERE agent_id=? AND capability=?",
                    params![
                        tx.now(),
                        id("bender_agent"),
                        if flag == "unreadable" {
                            "read_messages"
                        } else {
                            "manage_threads"
                        }
                    ],
                )?;
            }
            "other_owner" => {
                tx.conn().execute(
                    "UPDATE channel_threads SET work_owner_id=? WHERE id=?",
                    params![id("david"), thread.unwrap()],
                )?;
            }
            "untracked" => {
                tx.conn().execute(
                    "UPDATE channel_threads SET work_status=NULL WHERE id=?",
                    [thread.unwrap()],
                )?;
            }
            "suspended" => {
                tx.conn().execute(
                    "UPDATE agents SET suspended_at=? WHERE id=?",
                    params![tx.now(), id("bender_agent")],
                )?;
            }
            "budget" => {
                tx.conn().execute(
                    "UPDATE agents SET daily_board_post_cap=1 WHERE id=?",
                    [id("bender_agent")],
                )?;
            }
            "global_read" | "read_other_room" => {
                tx.conn().execute(
                    "DELETE FROM agent_grants WHERE agent_id=? AND capability='read_messages'",
                    [id("bender_agent")],
                )?;
                AgentGrant::create(
                    tx,
                    NewGrant {
                        agent_id: id("bender_agent"),
                        capability: "read_messages".into(),
                        room_id: if flag == "global_read" {
                            None
                        } else {
                            Some(id("watercooler"))
                        },
                        granted_by_id: id("david"),
                        ..Default::default()
                    },
                )?;
            }
            "receiver_outside" => {
                tx.conn().execute(
                    "DELETE FROM memberships WHERE room_id=? AND user_id=?",
                    params![BOARD, RECEIVER_USER],
                )?;
            }
            "receiver_suspended" => {
                tx.conn().execute(
                    "UPDATE agents SET suspended_at=? WHERE id=?",
                    params![tx.now(), RECEIVER],
                )?;
            }
            "receiver_no_post" | "receiver_no_manage" | "receiver_no_read" => {
                let cap = match flag.as_str().unwrap() {
                    "receiver_no_post" => "post_messages",
                    "receiver_no_manage" => "manage_threads",
                    _ => "read_messages",
                };
                tx.conn().execute(
                    "UPDATE agent_grants SET revoked_at=? WHERE agent_id=? AND capability=?",
                    params![tx.now(), RECEIVER, cap],
                )?;
            }
            "receiver_self" | "receiver_missing" => (),
            flag => panic!("unknown fixture flag {flag}"),
        }
    }
    Ok(())
}
fn thread_fields(conn: &Connection, thread: &ChannelThread) -> Result<Value> {
    let messages = crate::sql::query_all(
        conn,
        "SELECT creator_id,markdown_source,board_post_opener FROM messages WHERE thread_id=? ORDER BY id",
        [thread.id],
        |r| {
            Ok(
                json!({"creator_id":r.get::<_,i64>(0)?,"markdown":r.get::<_,Option<String>>(1)?,"opener":r.get::<_,bool>(2)?}),
            )
        },
    )?;
    Ok(
        json!({"title":thread.name,"creator_id":thread.creator_id,"work_status":thread.work_status,"owner":thread.work_owner_id,"tags":thread.tag_names(conn)?,
        "run_url":thread.run_url,"result":thread.result_markdown.as_ref().map(|s|s.chars().take(200).collect::<String>()),"result_length":thread.result_markdown.as_ref().map(|s|s.chars().count()),
        "result_updated_by":thread.result_updated_by_id,"result_updated_at":thread.result_updated_at.map(crate::models::agent_payloads::json_time),"updated_at":crate::models::agent_payloads::json_time(thread.updated_at),
        "work_status_changed_at":thread.work_status_changed_at.map(crate::models::agent_payloads::json_time),"messages":messages}),
    )
}
fn history(conn: &Connection, thread_id: i64, after: i64) -> Result<Vec<Value>> {
    let mut events = WorkThreadEvent::for_thread(conn, thread_id)?;
    events.sort_by_key(|e| e.id);
    Ok(events.into_iter().filter(|e|e.id>after).map(|mut e| {
        if e.metadata.get("handoff_id").is_some() { e.metadata["handoff_id"]=json!("handoff"); }
        json!({"kind":e.event_type,"actor":e.actor_id,"from_owner":e.from_owner_id,"to_owner":e.to_owner_id,"metadata":e.metadata})
    }).collect())
}
fn ledger(conn: &Connection, after: i64) -> Result<Vec<Value>> {
    let ids = crate::sql::query_all(
        conn,
        "SELECT id FROM agent_events WHERE id>? ORDER BY id",
        [after],
        |r| r.get::<_, i64>(0),
    )?;
    ids.into_iter().map(|id| {
        let e = AgentEvent::find(conn,id)?.unwrap();
        assert!(uuid::Uuid::parse_str(e.chain_id.as_deref().unwrap()).is_ok());
        let mut package = e.metadata.get("handoff").cloned().unwrap_or(Value::Null);
        if !package.is_null() { package["id"]=json!("handoff"); }
        Ok(json!({"agent":e.agent_id,"kind":e.event_type,"actor":e.actor_id,"outcome":e.outcome,
            "title":e.metadata.get("title"),"status":e.metadata.get("work_status"),"hop":e.hop(),"handoff":package,"webhook_status":e.webhook_status}))
    }).collect()
}
fn outcome<T>(result: service::Outcome<T>) -> (u16, Option<String>, Value, Option<T>) {
    match result {
        service::Outcome::Success { payload, status } => (status, None, Value::Null, Some(payload)),
        service::Outcome::Denied(result) => (
            result.status,
            result.error.clone(),
            result.failure_body(),
            None,
        ),
    }
}

#[test]
fn ws12_agent_work_writes_and_denials_match_rails_services() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agents_work_services_contract.json"
    ))
    .unwrap();
    let t = setup();
    for row in oracle["rows"].as_array().unwrap() {
        t.clock
            .travel_to(Timestamp::parse_db("2026-03-02 16:00:00").unwrap());
        let create = row["operation"] == "create";
        let thread = t.write(move |tx| {
            grants(tx)?;
            if create {
                return Ok(None);
            }
            ChannelThread::create_board_post(
                tx,
                NewChannelThread {
                    room_id: BOARD,
                    creator_id: id("david"),
                    name: Some("Owned".into()),
                    work_status: Some("planned".into()),
                    work_owner_id: Some(id("bender")),
                    tag_names: Some(vec!["seed".into()]),
                    run_url: Some("https://example.test/initial".into()),
                    ..Default::default()
                },
                None,
            )
            .map(Some)
        });
        let thread_id = thread.as_ref().map(|t| t.id);
        let case_flags = row["flags"].as_array().unwrap().clone();
        t.write(move |tx| flags(tx, thread_id, &case_flags));
        let (before_history, before_ledger, before_threads) = t.read(move |conn| {
            Ok((
                conn.query_row(
                    "SELECT COALESCE(MAX(id),0) FROM work_thread_events WHERE channel_thread_id=?",
                    [thread_id],
                    |r| r.get::<_, i64>(0),
                )?,
                conn.query_row("SELECT COALESCE(MAX(id),0) FROM agent_events", [], |r| {
                    r.get::<_, i64>(0)
                })?,
                ChannelThread::count(conn)?,
            ))
        });
        t.travel(60);
        let input = row["input"].clone();
        let operation = row["operation"].as_str().unwrap().to_owned();
        let case_flags = row["flags"].clone();
        let (status, error, failure, created) = t.write(move |tx| {
            let agent = Agent::find(tx.conn(), id("bender_agent"))?.unwrap();
            match operation.as_str() {
                "create" => Ok(outcome(service::create_board_post(
                    tx,
                    &agent,
                    &Room::find(tx.conn(), BOARD)?,
                    service::BoardPostInput {
                        title: input["title"].as_str().map(str::to_owned),
                        body: input["body"].as_str().map(str::to_owned),
                        tags: input.get("tags").cloned(),
                        work_status: input["work_status"].as_str().map(str::to_owned),
                        run_url: input["run_url"].as_str().map(str::to_owned),
                        owner_id: input.get("owner_id").cloned(),
                    },
                )?)),
                "update" => Ok(outcome(service::update_work(
                    tx,
                    &agent,
                    thread_id.unwrap(),
                    AgentWorkChanges {
                        work_status: input.get("work_status").cloned(),
                        note: input.get("note").cloned(),
                        tags: input.get("tags").cloned(),
                        run_url: input.get("run_url").cloned(),
                    },
                )?)),
                "result" => Ok(outcome(service::set_result(
                    tx,
                    &agent,
                    thread_id.unwrap(),
                    input.get("markdown"),
                )?)),
                "handoff" => {
                    let receiver = if case_flags
                        .as_array()
                        .unwrap()
                        .contains(&json!("receiver_self"))
                    {
                        agent.id
                    } else if case_flags
                        .as_array()
                        .unwrap()
                        .contains(&json!("receiver_missing"))
                    {
                        0
                    } else {
                        RECEIVER
                    };
                    let (status, error, failure, payload) = outcome(service::handoff_work(
                        tx,
                        &agent,
                        thread_id.unwrap(),
                        receiver,
                        HandoffPackage {
                            summary: input["summary"].as_str().unwrap_or_default().into(),
                            links: input["links"].clone(),
                            open_questions: input["open_questions"].clone(),
                        },
                        &audit_log::Context::default(),
                    )?);
                    Ok((status, error, failure, payload.map(|p| p.thread)))
                }
                _ => panic!("unknown operation"),
            }
        });
        let target = thread_id.or(created.map(|t| t.id));
        let actual = t.read(move |conn| {
            let fields = target.map(|id|ChannelThread::find(conn,id).and_then(|thread|thread_fields(conn,&thread))).transpose()?;
            Ok(json!({"status":status,"error":error,"failure":failure,"added_threads":ChannelThread::count(conn)?-before_threads,"fields":fields,
                "history":target.map(|id|history(conn,id,before_history)).transpose()?.unwrap_or_default(),"ledger":ledger(conn,before_ledger)?}))
        });
        assert_eq!(
            actual, row["expected"],
            "{}:{}",
            row["operation"], row["name"]
        );
    }
}

fn owned(t: &TestDb) -> ChannelThread {
    t.write(|tx| {
        grants(tx)?;
        ChannelThread::create_board_post(
            tx,
            NewChannelThread {
                room_id: BOARD,
                creator_id: id("david"),
                name: Some("Owned".into()),
                work_status: Some("planned".into()),
                work_owner_id: Some(id("bender")),
                ..Default::default()
            },
            None,
        )
    })
}

#[test]
fn agent_model_rechecks_stale_ownership_for_writes_and_keeps_the_rails_result_noop() {
    let t = setup();
    let initial = owned(&t);
    let mut status_stale = initial.clone();
    let mut result_stale = initial.clone();
    let mut handoff_stale = initial.clone();
    let mut noop = initial.clone();
    t.write(move |tx| {
        ChannelThread::find(tx.conn(), initial.id)?.update_work(
            tx,
            &crate::User::find(tx.conn(), id("david"))?,
            crate::models::channel_thread::WorkChanges {
                owner_id: Some(json!(id("david"))),
                ..Default::default()
            },
        )
    });
    let before = t
        .read(move |conn| WorkThreadEvent::for_thread(conn, initial.id))
        .len();
    assert!(matches!(
        t.try_write(move |tx| status_stale.update_work_by_agent(
            tx,
            &Agent::find(tx.conn(), id("bender_agent"))?.unwrap(),
            AgentWorkChanges {
                work_status: Some(json!("done")),
                tags: Some(json!("api")),
                ..Default::default()
            }
        )),
        Err(crate::Error::RecordNotFound(_))
    ));
    assert!(matches!(
        t.try_write(move |tx| result_stale.update_result_by_agent(
            tx,
            &Agent::find(tx.conn(), id("bender_agent"))?.unwrap(),
            &json!("Late")
        )),
        Err(crate::Error::RecordNotFound(_))
    ));
    assert!(matches!(
        t.try_write(move |tx| handoff_stale.hand_off(
            tx,
            &crate::User::find(tx.conn(), id("bender"))?,
            &Agent::find(tx.conn(), RECEIVER)?.unwrap(),
            HandoffPackage {
                summary: "Late".into(),
                ..Default::default()
            },
            &audit_log::Context::default()
        )),
        Err(crate::Error::RecordNotFound(_))
    ));
    t.write(move |tx| {
        noop.update_result_by_agent(
            tx,
            &Agent::find(tx.conn(), id("bender_agent"))?.unwrap(),
            &Value::Null,
        )
    });
    assert_eq!(
        t.read(move |conn| WorkThreadEvent::for_thread(conn, initial.id))
            .len(),
        before
    );
    let fields = t.read(move |conn| ChannelThread::find(conn, initial.id));
    assert_eq!(fields.work_status.as_deref(), Some("planned"));
    assert_eq!(fields.work_owner_id, Some(id("david")));
    assert_eq!(fields.result_markdown, None);
}

#[test]
fn separate_sqlite_writers_record_one_agent_status_event_for_one_real_change() {
    let t = setup();
    let initial = owned(&t);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let mut workers = Vec::new();
    for database in [t.db.clone(), t.another_process()] {
        let barrier = barrier.clone();
        let mut stale = initial.clone();
        workers.push(std::thread::spawn(move || {
            barrier.wait();
            database.write_blocking(move |tx| {
                stale.update_work_by_agent(
                    tx,
                    &Agent::find(tx.conn(), id("bender_agent"))?.unwrap(),
                    AgentWorkChanges {
                        work_status: Some(json!("done")),
                        note: Some(json!("Finished")),
                        ..Default::default()
                    },
                )
            })
        }));
    }
    for worker in workers {
        worker.join().unwrap().unwrap();
    }
    let events = t.read(move |conn| WorkThreadEvent::for_thread(conn, initial.id));
    assert_eq!(
        events
            .iter()
            .filter(|e| e.event_type == "work_update")
            .count(),
        1
    );
    assert_eq!(events[0].metadata["note"], "Finished");
    assert_eq!(events[0].actor_id, Some(id("bender")));
}

#[test]
fn result_unchanged_value_writes_nothing_and_handoff_audit_and_ledger_keep_snapshots() {
    let t = setup();
    let thread = owned(&t);
    let thread_id = thread.id;
    t.write(move |tx| {
        service::set_result(
            tx,
            &Agent::find(tx.conn(), id("bender_agent"))?.unwrap(),
            thread_id,
            Some(&json!("## Done")),
        )
    });
    let before = t.read(move |conn| ChannelThread::find(conn, thread_id));
    let count = t
        .read(move |conn| WorkThreadEvent::for_thread(conn, thread_id))
        .len();
    t.travel(60);
    t.write(move |tx| {
        service::set_result(
            tx,
            &Agent::find(tx.conn(), id("bender_agent"))?.unwrap(),
            thread_id,
            Some(&json!("## Done")),
        )
    });
    assert_eq!(
        t.read(move |conn| ChannelThread::find(conn, thread_id))
            .updated_at,
        before.updated_at
    );
    assert_eq!(
        t.read(move |conn| WorkThreadEvent::for_thread(conn, thread_id))
            .len(),
        count
    );
    let handoff = t.write(move |tx| {
        let (status, _, _, payload) = outcome(service::handoff_work(
            tx,
            &Agent::find(tx.conn(), id("bender_agent"))?.unwrap(),
            thread_id,
            RECEIVER,
            HandoffPackage {
                summary: "é".repeat(201),
                links: json!(["https://example.test/a"]),
                open_questions: json!(["Why?"]),
            },
            &audit_log::Context {
                ip_address: Some("192.0.2.1".into()),
                user_agent: Some("WS12 fixture".into()),
                ..Default::default()
            },
        )?);
        assert_eq!(status, 201);
        Ok(payload.unwrap().handoff)
    });
    let event = t.read(move |conn| WorkThreadEvent::for_thread(conn, thread_id))[0].clone();
    assert_eq!(event.event_type, "work_handoff");
    assert_eq!(event.metadata["handoff_summary"], "é".repeat(197) + "...");
    let handoff_id = handoff.id;
    t.write(move |tx| {
        let mut sender = crate::User::find(tx.conn(), id("bender"))?;
        sender.update(
            tx,
            crate::UserChanges {
                name: Some("Renamed".into()),
                ..Default::default()
            },
        )?;
        tx.conn()
            .execute("DELETE FROM work_handoffs WHERE id=?", [handoff_id])?;
        Ok(())
    });
    let package=t.read(move |conn|crate::sql::query_one(conn,"SELECT metadata FROM agent_events WHERE event_type='work_handed_off' AND json_extract(metadata,'$.thread_id')=?",[thread_id],|r|r.get::<_,Value>(0)));
    assert_eq!(package.unwrap()["handoff"]["sender_name"], "Bender Bot");
    let audit=t.read(move |conn|crate::sql::query_one(conn,"SELECT actor_label,target_label,details,ip_address,user_agent FROM audit_logs WHERE action='work.handoff' AND target_id=?",[thread_id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,Value>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?))));
    let (actor, target, details, ip, agent) = audit.unwrap();
    assert_eq!(actor, "Bender Bot <>");
    assert_eq!(target, "Owned");
    assert_eq!(details["summary"], "é".repeat(197) + "...");
    assert_eq!(ip, "192.0.2.1");
    assert_eq!(agent, "WS12 fixture");
    let activity=t.read(move |conn|crate::sql::query_one(conn,"SELECT user_id,event_type FROM activity_items WHERE source_type='WorkThreadEvent' AND source_id=?",[event.id],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?))));
    assert_eq!(activity, Some((id("david"), "work_assignment".into())));
}

#[test]
fn work_lists_filter_current_read_access_before_the_cap_and_board_filters_before_limit() {
    let t = setup();
    t.write(|tx| grants(tx));
    let ids = t.write(|tx| {
        let mut ids = Vec::new();
        for index in 0..104 {
            let thread = ChannelThread::create(
                tx,
                NewChannelThread {
                    room_id: BOARD,
                    creator_id: id("david"),
                    name: Some(format!("List {index}")),
                    work_status: Some(if index < 3 { "blocked" } else { "planned" }.into()),
                    work_owner_id: Some(id("bender")),
                    tag_names: Some(vec![if index < 3 { "api" } else { "docs" }.into()]),
                    ..Default::default()
                },
            )?;
            ids.push(thread.id);
        }
        let unreadable = Room::create_for(
            tx,
            crate::RoomType::Board,
            Some("Unreadable"),
            id("david"),
            &[id("david"), id("bender")],
        )?;
        // Grant rows exist, but read_messages covers only the other board.
        AgentGrant::create(
            tx,
            NewGrant {
                agent_id: id("bender_agent"),
                capability: "post_messages".into(),
                room_id: Some(unreadable.id),
                granted_by_id: id("david"),
                ..Default::default()
            },
        )?;
        for index in 0..102 {
            ChannelThread::create(
                tx,
                NewChannelThread {
                    room_id: unreadable.id,
                    creator_id: id("david"),
                    name: Some(format!("Hidden {index}")),
                    work_status: Some("planned".into()),
                    work_owner_id: Some(id("bender")),
                    ..Default::default()
                },
            )?;
        }
        Ok(ids)
    });
    let expected = ids[4..].iter().rev().copied().collect::<Vec<_>>();
    let visible = t.read(|conn| {
        let agent = Agent::find(conn, id("bender_agent"))?.unwrap();
        Ok(outcome(service::list_work(conn, &agent)?).3.unwrap())
    });
    assert_eq!(visible.iter().map(|t| t.id).collect::<Vec<_>>(), expected);
    let selected = t.read(|conn| {
        let agent = Agent::find(conn, id("bender_agent"))?.unwrap();
        Ok(outcome(service::list_board_posts(
            conn,
            &agent,
            &Room::find(conn, BOARD)?,
            Some(" blocked "),
            Some(" me "),
            Some(" API "),
        )?)
        .3
        .unwrap())
    });
    assert_eq!(
        selected.iter().map(|t| t.id).collect::<Vec<_>>(),
        ids[..3].iter().rev().copied().collect::<Vec<_>>()
    );
    for (status, owner, error) in [
        (
            "shipped",
            "me",
            "Status must be one of planned, in_progress, blocked, done, open, all",
        ),
        ("all", "everyone", "Owner must be a user id, me, or agents"),
        ("all", "１２", "Owner must be a user id, me, or agents"),
    ] {
        let status = status.to_owned();
        let owner = owner.to_owned();
        let result = t.read(move |conn| {
            service::list_board_posts(
                conn,
                &Agent::find(conn, id("bender_agent"))?.unwrap(),
                &Room::find(conn, BOARD)?,
                Some(&status),
                Some(&owner),
                None,
            )
        });
        let (status, message, _, _) = outcome(result);
        assert_eq!(status, 422);
        assert_eq!(message.as_deref(), Some(error));
    }
}
