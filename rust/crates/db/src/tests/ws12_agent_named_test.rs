//! Complete assertion sets from test/models/channel_thread_agent_assignment_test.rb.
use super::*;
use crate::models::channel_thread::{AgentWorkChanges, WorkChanges};
use crate::{
    Agent, ChannelThread, Error, NewChannelThread, Room, ThreadMembership, User, WorkThreadEvent,
};
use rusqlite::params;
use serde_json::{Value, json};

fn bot(tx: &mut Tx<'_>, user_id: i64, agent_id: Option<i64>, name: &str, room: i64) -> Result<()> {
    tx.conn().execute(
        "INSERT INTO users(id,name,role,status,created_at,updated_at) VALUES(?,?,2,0,?,?)",
        params![user_id, name, tx.now(), tx.now()],
    )?;
    Room::find(tx.conn(), room)?.grant_to(tx, &[user_id])?;
    if let Some(agent_id) = agent_id {
        // Exact identifiers make the producer's actor/owner assertions comparable.
        tx.conn().execute("INSERT INTO agents(id,user_id,owner_id,kind,status,created_at,updated_at) VALUES(?,?,?,'workspace','idle',?,?)",params![agent_id,user_id,id("david"),tx.now(),tx.now()])?;
    }
    Ok(())
}
fn grant(tx: &mut Tx<'_>, agent: i64, room: i64, capability: &str) -> Result<()> {
    tx.conn().execute("INSERT INTO agent_grants(agent_id,room_id,capability,granted_by_id,created_at,updated_at) VALUES(?,?,?,?,?,?)",params![agent,room,capability,id("david"),tx.now(),tx.now()])?;
    Ok(())
}
fn attempt(result: Result<()>) -> Value {
    match result {
        Ok(()) => Value::Null,
        Err(Error::RecordInvalid(errors)) => {
            json!({"kind":"invalid","messages":errors.full_messages()})
        }
        Err(Error::RecordNotFound(_)) => json!({"kind":"not_found"}),
        Err(error) => panic!("unexpected mutation failure: {error}"),
    }
}
fn run(key: &'static str) {
    let vectors: Value =
        serde_json::from_str(include_str!("../../../../vectors/ws12_agent_named.json")).unwrap();
    let rows = vectors["rows"].as_array().unwrap();
    let index = rows.iter().position(|row| row["key"] == key).unwrap();
    let expected = rows[index]["facts"].clone();
    let t = channel_thread_test::frozen();
    t.clock
        .travel_to(crate::Timestamp::parse_db("2026-03-02 16:00:00").unwrap());
    let thread_id = 901831000 + index as i64;
    let (room, manager, agent_id, bot_id) = if key == "status_inbox" {
        (id("designers"), id("jz"), 901830002, 901830001)
    } else {
        (
            id("watercooler"),
            id("david"),
            id("bender_agent"),
            id("bender"),
        )
    };
    t.write(move |tx| {
        tx.conn().execute("DELETE FROM agent_grants WHERE agent_id=?",[id("bender_agent")])?;
        if key=="status_inbox" {bot(tx,bot_id,Some(agent_id),"Inbox Worker Bot",room)?;}
        let creator=User::find(tx.conn(),manager)?;
        let thread=ChannelThread::create(tx,NewChannelThread {room_id:room,creator_id:manager,name:Some(if key=="status_inbox" {"Inbox agent work"}else{"Agent work"}.into()),..Default::default()})?;
        tx.conn().execute("UPDATE channel_threads SET id=? WHERE id=?",params![thread_id,thread.id])?;
        ThreadMembership::join(tx,thread_id,manager)?;
        if key=="status_inbox" {
            ThreadMembership::join(tx,thread_id,id("david"))?;
            tx.conn().execute("UPDATE thread_memberships SET involvement='everything' WHERE thread_id=? AND user_id=?",params![thread_id,id("david")])?;
        }
        let mut thread=ChannelThread::find(tx.conn(),thread_id)?;
        thread.update_work(tx,&creator,WorkChanges {status:Some(Some("planned".into())),..Default::default()})?;
        Ok(())
    });
    let mut errors = Vec::new();
    let mut transitions = Vec::new();
    let mut other_owner = Value::Null;
    if [
        "active",
        "legacy",
        "suspended",
        "nonmember",
        "no_post",
        "outside_human",
        "plain_bot",
    ]
    .contains(&key)
    {
        t.write(move |tx| {
            if key != "legacy" {
                grant(
                    tx,
                    agent_id,
                    room,
                    if key == "no_post" {
                        "read_messages"
                    } else {
                        "post_messages"
                    },
                )?;
            }
            if key == "suspended" {
                tx.conn().execute(
                    "UPDATE agents SET suspended_at=? WHERE id=?",
                    params![tx.now(), agent_id],
                )?;
            }
            if key == "nonmember" {
                tx.conn().execute(
                    "DELETE FROM memberships WHERE room_id=? AND user_id=?",
                    params![room, bot_id],
                )?;
            }
            if key == "plain_bot" {
                bot(tx, 901830003, None, "No Agent Bot", room)?;
            }
            Ok(())
        });
        errors.push(attempt(t.try_write(move |tx| {
            let mut thread = ChannelThread::find(tx.conn(), thread_id)?;
            thread.update_work(
                tx,
                &User::find(tx.conn(), manager)?,
                WorkChanges {
                    owner_id: Some(json!(match key {
                        "outside_human" => id("kevin"),
                        "plain_bot" => 901830003,
                        _ => bot_id,
                    })),
                    ..Default::default()
                },
            )
        })));
    } else if key == "unavailable" {
        t.write(move |tx| {
            grant(tx, agent_id, room, "post_messages")?;
            let manager = User::find(tx.conn(), manager)?;
            let mut thread = ChannelThread::find(tx.conn(), thread_id)?;
            thread.update_work(
                tx,
                &manager,
                WorkChanges {
                    owner_id: Some(json!(bot_id)),
                    ..Default::default()
                },
            )?;
            bot(tx, 901830004, Some(901830005), "Suspended Owner Bot", room)?;
            grant(tx, 901830005, room, "post_messages")?;
            let thread = ChannelThread::create(
                tx,
                NewChannelThread {
                    room_id: room,
                    creator_id: manager.id,
                    name: Some("Suspended work".into()),
                    ..Default::default()
                },
            )?;
            tx.conn().execute(
                "UPDATE channel_threads SET id=901832000 WHERE id=?",
                [thread.id],
            )?;
            ThreadMembership::join(tx, 901832000, manager.id)?;
            let mut other = ChannelThread::find(tx.conn(), 901832000)?;
            other.update_work(
                tx,
                &manager,
                WorkChanges {
                    status: Some(Some("planned".into())),
                    owner_id: Some(json!(901830004)),
                },
            )?;
            Ok(())
        });
        let availability = || {
            t.read(|conn| {
                let threads = [
                    ChannelThread::find(conn, thread_id)?,
                    ChannelThread::find(conn, 901832000)?,
                ];
                let owners = ChannelThread::work_owners(conn, &threads)?;
                Ok(json!(
                    threads
                        .iter()
                        .map(|thread| owners.available(thread))
                        .collect::<Vec<_>>()
                ))
            })
        };
        transitions.push(availability());
        t.write(move |tx| {
            tx.conn().execute(
                "DELETE FROM memberships WHERE room_id=? AND user_id=?",
                params![room, bot_id],
            )?;
            tx.conn().execute(
                "UPDATE agents SET suspended_at=? WHERE id=901830005",
                [tx.now()],
            )?;
            Ok(())
        });
        transitions.push(availability());
        other_owner = t.read(|conn| {
            Ok(json!(ChannelThread::find(conn, 901832000)?.work_owner_id))
        });
    } else {
        t.write(move |tx| {
            grant(tx, agent_id, room, "post_messages")?;
            let mut thread = ChannelThread::find(tx.conn(), thread_id)?;
            thread.update_work(
                tx,
                &User::find(tx.conn(), manager)?,
                WorkChanges {
                    owner_id: Some(json!(if key.starts_with("unowned_") {
                        id("jason")
                    } else {
                        bot_id
                    })),
                    ..Default::default()
                },
            )
        });
        let inputs: Vec<AgentWorkChanges> = match key {
            "status_note" => vec![AgentWorkChanges {
                work_status: Some(json!("in_progress")),
                note: Some(json!("Digging in")),
                ..Default::default()
            }],
            "status_inbox" => vec![AgentWorkChanges {
                work_status: Some(json!("in_progress")),
                note: Some(json!("On it")),
                ..Default::default()
            }],
            "tags" => vec![AgentWorkChanges {
                tags: Some(json!("API, launch")),
                ..Default::default()
            }],
            "invalid_fields" => vec![
                AgentWorkChanges {
                    tags: Some(json!("one, two, three, four, five, six")),
                    ..Default::default()
                },
                AgentWorkChanges {
                    run_url: Some(json!("http://example.com/runs/1")),
                    ..Default::default()
                },
                AgentWorkChanges::default(),
            ],
            "unowned_status" => vec![AgentWorkChanges {
                work_status: Some(json!("in_progress")),
                ..Default::default()
            }],
            "invalid_status_note" => vec![
                AgentWorkChanges {
                    work_status: Some(json!("shipped")),
                    ..Default::default()
                },
                AgentWorkChanges {
                    work_status: Some(json!("in_progress")),
                    note: Some(json!("x".repeat(501))),
                    ..Default::default()
                },
            ],
            _ => Vec::new(),
        };
        for changes in inputs {
            errors.push(attempt(t.try_write(move |tx| {
                let mut thread = ChannelThread::find(tx.conn(), thread_id)?;
                thread.update_work_by_agent(
                    tx,
                    &Agent::find(tx.conn(), agent_id)?.unwrap(),
                    changes,
                )
            })));
        }
        if key == "result" || key == "unowned_result" {
            errors.push(attempt(t.try_write(move |tx| {
                let mut thread = ChannelThread::find(tx.conn(), thread_id)?;
                thread.update_result_by_agent(
                    tx,
                    &Agent::find(tx.conn(), agent_id)?.unwrap(),
                    &json!(if key == "result" {
                        "## Agent outcome"
                    } else {
                        "Hijacked"
                    }),
                )
            })));
        }
    }
    let actual=t.read(move |conn| {
        let thread=ChannelThread::find(conn,thread_id)?;
        let mut events=WorkThreadEvent::for_thread(conn,thread_id)?;
        events.sort_by_key(|e|e.id);
        let latest=events.last().unwrap().id;
        let history=events.iter().map(|e|json!({"kind":e.event_type,"actor":e.actor_id,"from_owner":e.from_owner_id,"to_owner":e.to_owner_id,"from_status":e.from_status,"to_status":e.to_status,"note":e.metadata.get("note"),"metadata":e.metadata})).collect::<Vec<_>>();
        let mut statement=conn.prepare("SELECT user_id,event_type,source_id,read_at,handled_at FROM activity_items WHERE source_type='WorkThreadEvent' AND source_id IN (SELECT id FROM work_thread_events WHERE channel_thread_id=?) ORDER BY user_id,id")?;
        let inbox=statement.query_map([thread_id],|row|Ok(json!({"user":row.get::<_,i64>(0)?,"event_type":row.get::<_,String>(1)?,"latest":row.get::<_,i64>(2)?==latest,"unread":row.get::<_,Option<String>>(3)?.is_none()&&row.get::<_,Option<String>>(4)?.is_none()})))?.collect::<std::result::Result<Vec<_>,_>>()?;
        let owners=ChannelThread::work_owners(conn,std::slice::from_ref(&thread))?;
        Ok(json!({"errors":errors,"owner":thread.work_owner_id,"status":thread.work_status,"tags":thread.tag_names(conn)?,"result":thread.result_markdown,"result_actor":thread.result_updated_by_id,"run_url":thread.run_url,"available":owners.available(&thread),"transitions":transitions,"other_owner":other_owner,"history":history,"inbox":inbox}))
    });
    assert_eq!(
        actual, expected,
        "{key}: every pinned original assertion and persisted fact"
    );
}
macro_rules! cases {($($test:ident=>$key:literal),*$(,)?)=>{$(#[test]fn $test(){run($key);})*};}
cases! {
    ws12_agent_named_active_member_post_owner=>"active",
    ws12_agent_named_legacy_post_owner=>"legacy",
    ws12_agent_named_suspended_owner_validation=>"suspended",
    ws12_agent_named_nonmember_owner_validation=>"nonmember",
    ws12_agent_named_missing_post_owner_validation=>"no_post",
    ws12_agent_named_outside_human_owner_validation=>"outside_human",
    ws12_agent_named_bot_without_agent_validation=>"plain_bot",
    ws12_agent_named_owner_unavailable_after_access_change=>"unavailable",
    ws12_agent_named_status_event_with_note=>"status_note",
    ws12_agent_named_status_inbox_human_path=>"status_inbox",
    ws12_agent_named_tag_set_preserves_status=>"tags",
    ws12_agent_named_tags_run_url_missing_fields_validation=>"invalid_fields",
    ws12_agent_named_result_event_agent_actor=>"result",
    ws12_agent_named_result_requires_ownership=>"unowned_result",
    ws12_agent_named_status_requires_ownership=>"unowned_status",
    ws12_agent_named_status_and_note_validation=>"invalid_status_note",
}
