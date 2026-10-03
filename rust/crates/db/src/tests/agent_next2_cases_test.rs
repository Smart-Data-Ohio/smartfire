//! Remaining WS11 names through real bot, assignment, WS12 viewer and handoff services.
use super::*;
use crate::Timestamp;
use crate::models::{agent_posting as budgets, agent_work, audit_log};
use crate::{
    ActivityItem, Agent, AgentChanges, AgentGrant, AgentKind, ChannelThread, HandoffPackage,
    Message, NewAgent, NewChannelThread, NewGrant, NewMessage, Room, RoomType, User,
};
use serde_json::{Value, json};
fn gold(name: &str) -> Value {
    let v: Value =
        serde_json::from_str(include_str!("../../../../vectors/agents_next2_models.json")).unwrap();
    v["results"][name].clone()
}
fn setup() -> TestDb {
    let t = super::channel_thread_test::frozen();
    t.clock
        .travel_to(Timestamp::parse_db("2026-03-02 16:00:00").unwrap());
    t.write(|tx| {
        tx.conn().execute("DELETE FROM agent_events", [])?;
        tx.conn().execute("DELETE FROM agent_grants", [])?;
        Room::find(tx.conn(), id("watercooler"))?.grant_to(tx, &[id("bender")])?;
        tx.conn().execute(
            "UPDATE sqlite_sequence SET seq=1901820000 WHERE name='channel_threads'",
            [],
        )?;
        Ok(())
    });
    t.sink.take();
    t
}
fn bot(tx: &mut Tx<'_>, user: i64, name: &str) -> Result<User> {
    tx.conn().execute(
        "UPDATE sqlite_sequence SET seq=? WHERE name='users'",
        [user - 1],
    )?;
    let bot = User::create_bot(tx, name, None)?;
    assert_eq!(bot.id, user);
    Ok(bot)
}
fn agent(tx: &mut Tx<'_>, user: i64, name: &str, room: i64) -> Result<Agent> {
    let bot = bot(tx, user, name)?;
    tx.conn().execute(
        "UPDATE sqlite_sequence SET seq=? WHERE name='agents'",
        [user],
    )?;
    let agent = Agent::create(
        tx,
        NewAgent {
            user_id: bot.id,
            owner_id: Some(id("david")),
            kind: AgentKind::Workspace,
            ..Default::default()
        },
    )?;
    assert_eq!(agent.id, user + 1);
    Room::find(tx.conn(), room)?.grant_to(tx, &[user])?;
    Ok(agent)
}
fn factory(reset: bool) {
    let t = setup();
    let result=t.write(move|tx|{
    let (created,calls)=crate::sql::with_fixture_alphanumeric(&["5M0aLYwQyBXOXa5Wsz6NZb11EE4tW2","R4kme9anwWRuz3sSoBXiB8Li8ioZPP"],||->Result<_>{let mut created=bot(tx,1901800001,"Bender")?;let first=created.bot_key();let second=if reset{Some(created.reset_bot_key(tx)?)}else{None};Ok((created,first,second))});
    let (created,first,second)=created?;
    let stored=User::find(tx.conn(),created.id)?;let plaintext:Option<String>=tx.conn().query_row("SELECT bot_token FROM users WHERE id=?",[created.id],|r|r.get(0))?;
    Ok(json!({"id":created.id,"name":stored.name,"role":stored.role.name(),"status":stored.status.name(),"first":first,"second":second,
        "plaintext":plaintext,"digest":stored.bot_token_digest,"stored_key":stored.bot_key(),"first_auth":User::authenticate_bot(tx.conn(),&first)?.map(|u|u.id),
        "second_auth":second.as_ref().map(|key|User::authenticate_bot(tx.conn(),key)).transpose()?.flatten().map(|u|u.id),"entropy_calls":calls}))
});
    assert_eq!(result, gold(if reset { "reset_key" } else { "create_bot" }));
}
#[test]
fn ws11_next2_bot_factory_fixed_entropy() {
    factory(false);
}
#[test]
fn ws11_next2_bot_reset_fixed_entropy() {
    factory(true);
}
fn assignments(human: bool) {
    let t = setup();
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE sqlite_sequence SET seq=1901810000 WHERE name='rooms'",
            [],
        )?;
        let room = Room::create_for(
            tx,
            RoomType::Board,
            Some("Loop Board"),
            id("david"),
            &[id("david")],
        )?;
        assert_eq!(room.id, 1901810001);
        let a = agent(tx, 1901800011, "Loop Agent A", room.id)?;
        let b = agent(tx, 1901800021, "Loop Agent B", room.id)?;
        let actors = if human {
            vec![id("david")]
        } else {
            vec![a.user_id, b.user_id, a.user_id, b.user_id]
        };
        for (i, actor) in actors.into_iter().enumerate() {
            let owner = if human || i % 2 == 1 {
                a.user_id
            } else {
                b.user_id
            };
            ChannelThread::create_board_post(
                tx,
                NewChannelThread {
                    room_id: room.id,
                    creator_id: actor,
                    name: Some(format!("Post {}", i + 1)),
                    work_status: Some("in_progress".into()),
                    work_owner_id: Some(owner),
                    ..Default::default()
                },
                None,
            )?;
        }
        Ok(())
    });
    let actual=t.read(|conn|{
    let events=crate::sql::query_all(conn,"SELECT agent_id,event_type,outcome,actor_id,hop,detail,webhook_status,metadata FROM agent_events ORDER BY id",[],|r|Ok(json!({"agent_id":r.get::<_,i64>(0)?,"event_type":r.get::<_,String>(1)?,"outcome":r.get::<_,Option<String>>(2)?,"actor_id":r.get::<_,Option<i64>>(3)?,"hop":r.get::<_,i64>(4)?,"detail":r.get::<_,Option<String>>(5)?,"webhook_status":r.get::<_,String>(6)?,"metadata":r.get::<_,Value>(7)?})))?;
    let threads=crate::sql::query_all(conn,"SELECT id,creator_id,work_owner_id FROM channel_threads WHERE id BETWEEN 1901820001 AND 1901820004 ORDER BY id",[],|r|Ok(json!([r.get::<_,i64>(0)?,r.get::<_,i64>(1)?,r.get::<_,Option<i64>>(2)?])))?;
    let history=crate::sql::query_all(conn,"SELECT channel_thread_id,event_type,actor_id,from_owner_id,to_owner_id FROM work_thread_events WHERE channel_thread_id BETWEEN 1901820001 AND 1901820004 ORDER BY id",[],|r|Ok(json!([r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<i64>>(2)?,r.get::<_,Option<i64>>(3)?,r.get::<_,Option<i64>>(4)?])))?;
    Ok(json!({"events":events,"threads":threads,"history":history,"same_chain":conn.query_row("SELECT count(DISTINCT chain_id)=1 FROM agent_events",[],|r|r.get::<_,bool>(0))?}))
});
    assert_eq!(actual, gold(if human { "human_root" } else { "hop_limit" }));
}
#[test]
fn ws11_next2_assignment_agent_hops_stop_at_limit() {
    assignments(false);
}
#[test]
fn ws11_next2_assignment_human_starts_root_zero() {
    assignments(true);
}
#[test]
fn ws11_next2_assignment_delete_no_agent_emits_nothing() {
    let t = setup();
    let thread = t.write(|tx| {
        ChannelThread::create(
            tx,
            NewChannelThread {
                room_id: id("watercooler"),
                creator_id: id("david"),
                name: Some("Agent work".into()),
                ..Default::default()
            },
        )
    });
    t.sink.take();
    let thread_id = thread.id;
    t.write(move |tx| thread.destroy(tx));
    let result=t.read(|conn|Ok(json!({"exists":ChannelThread::find_by_id(conn,thread_id)?.is_some(),"events":conn.query_row("SELECT count(*) FROM agent_events",[],|r|r.get::<_,i64>(0))?,"jobs":t.events().iter().filter(|e|matches!(e,Event::Job(job)if job.class=="Agent::EventWebhookJob")).count()})));
    assert_eq!(result, gold("delete_no_owner"));
}
fn expire_usage(tx: &Tx<'_>) -> Result<()> {
    for table in ["messages", "channel_threads"] {
        tx.conn().execute(
            &format!("UPDATE {table} SET created_at='2000-01-01 00:00:00' WHERE creator_id=?"),
            [id("bender")],
        )?;
    }
    tx.conn().execute("DELETE FROM agent_approvals", [])?;
    tx.conn().execute("DELETE FROM agent_budget_notices", [])?;
    Ok(())
}
fn one_message(tx: &mut Tx<'_>, key: &str) -> Result<()> {
    Message::create(
        tx,
        NewMessage {
            room_id: id("watercooler"),
            creator_id: id("bender"),
            markdown_source: Some("One".into()),
            client_message_id: Some(key.into()),
            ..Default::default()
        },
    )?;
    Ok(())
}
#[test]
fn ws11_next2_budget_stranger_cannot_read_owner_item() {
    let t = setup();
    let actual=t.write(|tx|{expire_usage(tx)?;Agent::find(tx.conn(),id("bender_agent"))?.unwrap().update(tx,AgentChanges{daily_message_cap:Some(Some(1)),..Default::default()})?;one_message(tx,"budget-stranger")?;let denial=budgets::check_budget(tx,id("bender_agent"),budgets::Cap::Messages)?.unwrap();
    let count=|user|->Result<usize>{Ok(ActivityItem::accessible_to(tx.conn(),&User::find(tx.conn(),user)?)?.iter().filter(|item|item.event_type=="agent_budget_exceeded").count())};
    Ok(json!({"status":"too_many_requests","error":denial["error"],"stranger":count(id("jason"))?,"owner":count(id("david"))?}))});
    assert_eq!(actual, gold("budget_viewer"));
}
#[test]
fn ws11_next2_budget_handoff_ignores_exhausted_caps() {
    let t = setup();
    let result=t.write(|tx|{
    expire_usage(tx)?;tx.conn().execute("UPDATE sqlite_sequence SET seq=1901810000 WHERE name='rooms'",[])?;
    let room=Room::create_for(tx,RoomType::Board,Some("Handoff Board"),id("david"),&[id("david"),id("bender")])?;
    let receiver=agent(tx,1901800011,"Handoff Receiver",room.id)?;
    for aid in [id("bender_agent"),receiver.id]{for cap in ["read_messages","post_messages","manage_threads"]{AgentGrant::create(tx,NewGrant{agent_id:aid,room_id:Some(room.id),capability:cap.into(),granted_by_id:id("david"),..Default::default()})?;}}
    let mut threads=vec![];for title in ["First","Second"]{threads.push(ChannelThread::create_board_post(tx,NewChannelThread{room_id:room.id,creator_id:id("david"),name:Some(title.into()),work_status:Some("in_progress".into()),work_owner_id:Some(id("bender")),..Default::default()},None)?);}
    let mut a=Agent::find(tx.conn(),id("bender_agent"))?.unwrap();
    let first=agent_work::handoff_work(tx,&a,threads[0].id,receiver.id,HandoffPackage{summary:"Halfway there".into(),..Default::default()},&audit_log::Context::default())?;
    let initial=budgets::usage(tx.conn(),a.id,tx.now())?;
    a.update(tx,AgentChanges{daily_message_cap:Some(Some(1)),daily_board_post_cap:Some(Some(1)),daily_external_action_cap:Some(Some(1)),..Default::default()})?;
    one_message(tx,"budget-handoff-exhaust")?;assert!(budgets::check_budget(tx,a.id,budgets::Cap::Messages)?.is_some());
    let second=agent_work::handoff_work(tx,&a,threads[1].id,receiver.id,HandoffPackage{summary:"Still yours".into(),..Default::default()},&audit_log::Context::default())?;
    let handoffs=crate::sql::query_all(tx.conn(),"SELECT channel_thread_id,sender_id,receiver_agent_id,summary FROM work_handoffs WHERE channel_thread_id IN (?,?) ORDER BY id",[threads[0].id,threads[1].id],|r|Ok(json!([r.get::<_,i64>(0)?,r.get::<_,i64>(1)?,r.get::<_,i64>(2)?,r.get::<_,String>(3)?])))?;
    Ok(json!({"first_ok":matches!(first,agent_work::Outcome::Success{..}),"second_ok":matches!(second,agent_work::Outcome::Success{..}),"initial_usage":initial,"final_usage":budgets::usage(tx.conn(),a.id,tx.now())?,"owner":ChannelThread::find(tx.conn(),threads[1].id)?.work_owner_id,"handoffs":handoffs}))
});
    assert_eq!(result, gold("budget_handoff"));
}
