//! Named WS11 ledger comparisons. Work mutation and validation stay in WS12.
use super::*;
use crate::models::agent_delivery::EventWebhookJob;
use crate::models::channel_thread::WorkChanges;
use crate::{
    Agent, AgentGrant, AgentKind, ChannelThread, NewAgent, NewChannelThread, NewGrant, Room,
    ThreadMembership, User,
};
use serde_json::{Value, json};

const THREAD: i64 = 1901600001;
const OTHER_USER: i64 = 1901600011;
const OTHER_AGENT: i64 = 1901600012;
fn gold(name: &str) -> Value {
    let value: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agents_assignment_named.json"
    ))
    .unwrap();
    value["results"][name].clone()
}
fn setup(name: &str) -> TestDb {
    let t = super::channel_thread_test::frozen();
    t.clock
        .travel_to(crate::Timestamp::parse_db("2026-03-02 16:00:00").unwrap());
    let read = matches!(name, "nonmember" | "outer_commit");
    t.write(move |tx| {
        Room::find(tx.conn(), id("watercooler"))?.grant_to(tx, &[id("bender")])?;
        tx.conn().execute("DELETE FROM agent_grants", [])?;
        tx.conn().execute("DELETE FROM agent_events", [])?;
        for (table, sequence) in [
            ("users", OTHER_USER - 1),
            ("agents", OTHER_AGENT - 1),
            ("agent_events", 1901610000),
            ("work_thread_events", 1901620000),
        ] {
            tx.conn()
                .execute("DELETE FROM sqlite_sequence WHERE name=?", [table])?;
            tx.conn().execute(
                "INSERT INTO sqlite_sequence(name,seq) VALUES(?,?)",
                rusqlite::params![table, sequence],
            )?;
        }
        let user = User::create_bot(tx, "Second Owner Bot", None)?;
        assert_eq!(user.id, OTHER_USER);
        let other = Agent::create(
            tx,
            NewAgent {
                user_id: user.id,
                owner_id: Some(id("david")),
                kind: AgentKind::Workspace,
                ..Default::default()
            },
        )?;
        assert_eq!(other.id, OTHER_AGENT);
        Room::find(tx.conn(), id("watercooler"))?.grant_to(tx, &[user.id])?;
        for agent in [id("bender_agent"), other.id] {
            AgentGrant::create(
                tx,
                NewGrant {
                    agent_id: agent,
                    capability: "post_messages".into(),
                    room_id: Some(id("watercooler")),
                    granted_by_id: id("david"),
                    ..Default::default()
                },
            )?;
        }
        if read {
            AgentGrant::create(
                tx,
                NewGrant {
                    agent_id: id("bender_agent"),
                    capability: "read_messages".into(),
                    room_id: None,
                    granted_by_id: id("david"),
                    ..Default::default()
                },
            )?;
        }
        tx.conn().execute(
            "UPDATE sqlite_sequence SET seq=? WHERE name='channel_threads'",
            [THREAD - 1],
        )?;
        let mut thread = ChannelThread::create(
            tx,
            NewChannelThread {
                room_id: id("watercooler"),
                creator_id: id("david"),
                name: Some("Agent work".into()),
                ..Default::default()
            },
        )?;
        assert_eq!(thread.id, THREAD);
        ThreadMembership::join(tx, thread.id, id("david"))?;
        let actor = User::find(tx.conn(), id("david"))?;
        thread.update_work(
            tx,
            &actor,
            WorkChanges {
                status: Some(Some("planned".into())),
                ..Default::default()
            },
        )?;
        Ok(())
    });
    t.sink.take();
    t
}
fn change(tx: &mut Tx<'_>, owner: Option<Value>, status: Option<&str>) -> Result<()> {
    let mut thread = ChannelThread::find(tx.conn(), THREAD)?;
    let actor = User::find(tx.conn(), id("david"))?;
    thread.update_work(
        tx,
        &actor,
        WorkChanges {
            owner_id: owner,
            status: status.map(|s| Some(s.into())),
        },
    )
}
fn snapshot(t: &TestDb, error: bool, inside_jobs: Option<usize>) -> Value {
    let jobs = t
        .events()
        .iter()
        .filter_map(|event| event.as_job::<EventWebhookJob>())
        .map(|job| json!({"event_id":job.event_id}))
        .collect::<Vec<_>>();
    t.read(move |conn| {
        let thread = ChannelThread::find(conn, THREAD)?;
        let mut statement = conn.prepare("SELECT id,agent_id,event_type,outcome,message_id,room_id,actor_id,metadata,hop,detail,webhook_status,chain_id FROM agent_events ORDER BY id")?;
        let ledger = statement.query_map([], |row| {
            let chain: String = row.get(11)?;
            assert!(uuid::Uuid::parse_str(&chain).is_ok(), "generated chain must be a UUID");
            Ok(json!({"id":row.get::<_,i64>(0)?,"agent_id":row.get::<_,i64>(1)?,"event_type":row.get::<_,String>(2)?,
                "outcome":row.get::<_,Option<String>>(3)?,"message_id":row.get::<_,Option<i64>>(4)?,"room_id":row.get::<_,Option<i64>>(5)?,
                "actor_id":row.get::<_,Option<i64>>(6)?,"metadata":row.get::<_,Value>(7)?,"hop":row.get::<_,i64>(8)?,
                "detail":row.get::<_,Option<String>>(9)?,"webhook_status":row.get::<_,String>(10)?}))
        })?.collect::<std::result::Result<Vec<_>,_>>()?;
        let mut statement = conn.prepare("SELECT id,event_type,actor_id,from_owner_id,to_owner_id,from_status,to_status,metadata FROM work_thread_events WHERE channel_thread_id=? ORDER BY id")?;
        let history = statement.query_map([THREAD], |row| Ok(json!({"id":row.get::<_,i64>(0)?,"event_type":row.get::<_,String>(1)?,
            "actor_id":row.get::<_,Option<i64>>(2)?,"from_owner_id":row.get::<_,Option<i64>>(3)?,"to_owner_id":row.get::<_,Option<i64>>(4)?,
            "from_status":row.get::<_,Option<String>>(5)?,"to_status":row.get::<_,Option<String>>(6)?,"metadata":row.get::<_,Value>(7)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
        Ok(json!({"owner":thread.work_owner_id,"status":thread.work_status,"ledger":ledger,"history":history,"jobs":jobs,"error":error,"inside_jobs":inside_jobs}))
    })
}
fn case(name: &'static str) {
    // Fault tests also prove the producer works before rejecting either insert.
    if name.ends_with("failure") {
        case("assigned");
    }
    let t = setup(name);
    let mut inside = None;
    let mut error = false;
    match name {
        "assigned" => t.write(|tx| change(tx, Some(json!(id("bender"))), None)),
        "unassigned" | "nonmember" | "human" | "agent" => {
            t.write(|tx| change(tx, Some(json!(id("bender"))), None));
            if name == "nonmember" {
                t.write(|tx| {
                    tx.conn().execute(
                        "DELETE FROM memberships WHERE user_id=? AND room_id=?",
                        [id("bender"), id("watercooler")],
                    )?;
                    Ok(())
                });
            }
            t.sink.take();
            let target = match name {
                "human" => json!(id("jason")),
                "agent" => json!(OTHER_USER),
                _ => Value::Null,
            };
            t.write(move |tx| change(tx, Some(target), None));
        }
        "status_only" => {
            for (owner, status) in [
                (Some(json!(id("jason"))), None),
                (None, Some("in_progress")),
                (Some(json!(id("david"))), None),
                (Some(json!(id("bender"))), None),
                (None, Some("blocked")),
            ] {
                t.write(move |tx| change(tx, owner, status));
            }
        }
        "outer_commit" => {
            let sink = t.sink.clone();
            inside = Some(t.write(move |tx| {
                tx.savepoint(|tx| change(tx, Some(json!(id("bender"))), None))?;
                Ok(sink
                    .events()
                    .iter()
                    .filter_map(|event| event.as_job::<EventWebhookJob>())
                    .count())
            }));
        }
        "history_failure" | "ledger_failure" => {
            let table = if name == "history_failure" {
                "work_thread_events"
            } else {
                "agent_events"
            };
            t.write(move |tx| {tx.conn().execute_batch(&format!("CREATE TEMP TRIGGER next_reject BEFORE INSERT ON {table} BEGIN SELECT RAISE(ABORT,'injected named callback failure'); END"))?;Ok(())});
            error = t
                .try_write(|tx| change(tx, Some(json!(id("bender"))), None))
                .is_err();
            assert!(error, "injected failure must reach the caller");
        }
        _ => unreachable!(),
    }
    assert_eq!(
        snapshot(&t, error, inside),
        gold(name),
        "{name}: source, history, ledger and committed jobs"
    );
}
macro_rules! named_cases {
    ($($test:ident => $case:literal),* $(,)?) => {$(#[test] fn $test() {case($case);})*};
}
named_cases! {
    ws11_next_assignment_records_ledger_and_history => "assigned",
    ws11_next_assignment_unassignment_records_one_row => "unassigned",
    ws11_next_assignment_departed_owner_has_no_webhook => "nonmember",
    ws11_next_assignment_to_human_unassigns_agent => "human",
    ws11_next_assignment_between_agents_notifies_both => "agent",
    ws11_next_assignment_status_and_human_changes_emit_no_agent_rows => "status_only",
    ws11_next_assignment_webhook_waits_for_outer_commit => "outer_commit",
    ws11_next_assignment_history_failure_rolls_back_ledger => "history_failure",
    ws11_next_assignment_ledger_failure_rolls_back_history => "ledger_failure",
}
