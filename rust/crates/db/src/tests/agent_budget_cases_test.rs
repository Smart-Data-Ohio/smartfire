//! AgentBudgetsTest through persisted model rows and the shared domain preflight.
use super::*;
use crate::models::agent_posting::{self as budgets, Cap};
use crate::models::agent_service::ServiceResult;
use crate::{
    Agent, AgentApproval, AgentChanges, ChannelThread, Message, NewApproval, NewChannelThread,
    NewMessage, Room, RoomType,
};
use serde_json::Value;
fn oracle(key: &str) -> Value {
    let v: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agents_budget_usage_contract.json"
    ))
    .unwrap();
    v[key].clone()
}
fn setup() -> TestDb {
    let t = super::channel_thread_test::frozen();
    t.clock
        .travel_to(crate::Timestamp::parse_db("2026-03-02 16:00:00").unwrap());
    t.write(|tx|{
        tx.conn().execute("UPDATE messages SET created_at='2000-01-01 00:00:00' WHERE creator_id=?",[id("bender")])?;
        tx.conn().execute("UPDATE channel_threads SET created_at='2000-01-01 00:00:00' WHERE creator_id=?",[id("bender")])?;
        tx.conn().execute("DELETE FROM agent_approvals",[])?;
        tx.conn().execute("DELETE FROM agent_budget_notices",[])?;
        tx.conn().execute("DELETE FROM activity_items WHERE source_type='AgentBudgetNotice'",[])?;
        tx.conn().execute("UPDATE agents SET daily_message_cap=NULL,daily_board_post_cap=NULL,daily_external_action_cap=NULL WHERE id=?",[id("bender_agent")])?;
        tx.conn().execute("UPDATE users SET time_zone='UTC' WHERE id=?",[id("bender")])?;
        Ok(())
    });
    t
}
fn message(tx: &mut Tx<'_>) -> Result<Message> {
    Message::create(
        tx,
        NewMessage {
            room_id: id("watercooler"),
            creator_id: id("bender"),
            markdown_source: Some("One".into()),
            ..Default::default()
        },
    )
}
fn board(tx: &mut Tx<'_>, opener: bool) -> Result<ChannelThread> {
    let room = Room::create_for(
        tx,
        RoomType::Board,
        Some("Budget Board"),
        id("david"),
        &[id("david"), id("bender")],
    )?;
    let mut thread = ChannelThread::create(
        tx,
        NewChannelThread {
            room_id: room.id,
            creator_id: id("bender"),
            name: Some("Post".into()),
            work_status: Some("planned".into()),
            ..Default::default()
        },
    )?;
    if opener {
        thread.post_message(
            tx,
            id("bender"),
            NewMessage {
                markdown_source: Some("Opening words".into()),
                board_post_opener: true,
                ..Default::default()
            },
        )?;
    }
    Ok(thread)
}
fn cap(tx: &mut Tx<'_>, changes: AgentChanges) -> Result<()> {
    Agent::find(tx.conn(), id("bender_agent"))?
        .unwrap()
        .update(tx, changes)
}
fn usage(t: &TestDb) -> Value {
    let now = t.now();
    t.read(move |c| budgets::usage(c, id("bender_agent"), now))
}
#[test]
fn ws11_budget_case_usage_counts_persisted_rows() {
    let t = setup();
    t.write(|tx| {
        message(tx)?;
        board(tx, false)?;
        AgentApproval::create(
            tx,
            NewApproval {
                agent_id: id("bender_agent"),
                action: "deploy".into(),
                summary: "Ship it".into(),
                ..Default::default()
            },
        )?;
        Ok(())
    });
    assert_eq!(usage(&t), oracle("all"));
}
#[test]
fn ws11_budget_case_board_opener_counts_only_as_post() {
    let t = setup();
    t.write(|tx| {
        board(tx, true)?;
        Ok(())
    });
    assert_eq!(usage(&t), oracle("opener"));
}
#[test]
fn ws11_budget_case_opener_passes_reply_exhausts() {
    let t = setup();
    t.write(|tx| {
        cap(
            tx,
            AgentChanges {
                daily_message_cap: Some(Some(1)),
                ..Default::default()
            },
        )?;
        let mut thread = board(tx, true)?;
        assert!(budgets::check_budget(tx, id("bender_agent"), Cap::Messages)?.is_none());
        thread.post_message(
            tx,
            id("bender"),
            NewMessage {
                markdown_source: Some("A reply".into()),
                ..Default::default()
            },
        )?;
        assert_eq!(
            ServiceResult::budget(
                budgets::check_budget(tx, id("bender_agent"), Cap::Messages)?.unwrap()
            )
            .status,
            429
        );
        Ok(())
    });
}
#[test]
fn ws11_budget_case_yesterday_is_excluded() {
    let t = setup();
    t.write(|tx| {
        let m = message(tx)?;
        tx.conn().execute(
            "UPDATE messages SET created_at=? WHERE id=?",
            rusqlite::params![tx.now().ago(jiff::SignedDuration::from_hours(24)), m.id],
        )?;
        Ok(())
    });
    assert_eq!(usage(&t)["messages"], oracle("yesterday")["messages"]);
}
#[test]
fn ws11_budget_case_unlimited_caps_pass() {
    let t = setup();
    t.write(|tx| {
        for c in [Cap::Messages, Cap::BoardPosts, Cap::ExternalActions] {
            assert!(budgets::check_budget(tx, id("bender_agent"), c)?.is_none());
        }
        Ok(())
    });
}
#[test]
fn ws11_budget_case_denial_notifies_owner_once_per_day() {
    let t = setup();
    let first = t.write(|tx| {
        cap(
            tx,
            AgentChanges {
                daily_message_cap: Some(Some(1)),
                ..Default::default()
            },
        )?;
        message(tx)?;
        Ok(ServiceResult::budget(
            budgets::check_budget(tx, id("bender_agent"), Cap::Messages)?.unwrap(),
        ))
    });
    assert!(!first.is_ok());
    assert_eq!(first.status, 429);
    assert_eq!(
        first.error.as_deref(),
        Some("Daily message budget exceeded (1/day)")
    );
    let p = first.payload.unwrap();
    assert_eq!(p["cap"], "messages");
    assert_eq!(p["limit"], 1);
    assert!(p["retry_after"].as_i64().unwrap() > 0);
    for _ in 0..3 {
        t.write(|tx| {
            assert!(budgets::check_budget(tx, id("bender_agent"), Cap::Messages)?.is_some());
            Ok(())
        });
    }
    t.read(|c| {
        let owner = Agent::find(c, id("bender_agent"))?
            .unwrap()
            .owner_id
            .unwrap();
        assert_eq!(
            c.query_row(
                "SELECT COUNT(*) FROM activity_items WHERE event_type='agent_budget_exceeded'",
                [],
                |r| r.get::<_, i64>(0)
            )?,
            1
        );
        assert_eq!(
            c.query_row(
                "SELECT user_id FROM activity_items WHERE event_type='agent_budget_exceeded'",
                [],
                |r| r.get::<_, i64>(0)
            )?,
            owner
        );
        Ok(())
    });
    t.travel(24 * 3600);
    t.write(|tx| {
        message(tx)?;
        assert!(budgets::check_budget(tx, id("bender_agent"), Cap::Messages)?.is_some());
        assert_eq!(
            tx.conn().query_row(
                "SELECT COUNT(*) FROM activity_items WHERE event_type='agent_budget_exceeded'",
                [],
                |r| r.get::<_, i64>(0)
            )?,
            2
        );
        Ok(())
    });
}
#[test]
fn ws11_budget_case_each_cap_notifies_separately() {
    let t = setup();
    t.write(|tx| {
        cap(
            tx,
            AgentChanges {
                daily_message_cap: Some(Some(1)),
                daily_board_post_cap: Some(Some(1)),
                ..Default::default()
            },
        )?;
        message(tx)?;
        board(tx, false)?;
        for c in [Cap::Messages, Cap::BoardPosts] {
            assert!(budgets::check_budget(tx, id("bender_agent"), c)?.is_some());
        }
        assert_eq!(
            tx.conn().query_row(
                "SELECT COUNT(*) FROM activity_items WHERE event_type='agent_budget_exceeded'",
                [],
                |r| r.get::<_, i64>(0)
            )?,
            2
        );
        Ok(())
    });
}
#[test]
fn ws11_budget_case_ownerless_notifies_active_human_admins() {
    let t = setup();
    t.write(|tx|{tx.conn().execute("UPDATE agents SET owner_id=NULL WHERE id=?",[id("bender_agent")])?;cap(tx,AgentChanges{daily_message_cap:Some(Some(1)),..Default::default()})?;message(tx)?;budgets::check_budget(tx,id("bender_agent"),Cap::Messages)?;let list=|sql|->Result<Vec<i64>>{let mut q=tx.conn().prepare(sql)?;Ok(q.query_map([],|r|r.get(0))?.collect::<std::result::Result<_,_>>()?)};let actual=list("SELECT user_id FROM activity_items WHERE event_type='agent_budget_exceeded' ORDER BY user_id")?;assert!(!actual.is_empty());assert_eq!(actual,list("SELECT id FROM users WHERE role=1 AND status=0 ORDER BY id")?);Ok(())});
}
#[test]
fn ws11_budget_case_caps_require_positive_integer() {
    let t = setup();
    for n in [0, -3] {
        assert!(matches!(
            t.try_write(move |tx| cap(
                tx,
                AgentChanges {
                    daily_message_cap: Some(Some(n)),
                    ..Default::default()
                }
            )),
            Err(crate::Error::RecordInvalid(_))
        ));
    }
    t.write(|tx| {
        cap(
            tx,
            AgentChanges {
                daily_message_cap: Some(None),
                ..Default::default()
            },
        )
    });
}
