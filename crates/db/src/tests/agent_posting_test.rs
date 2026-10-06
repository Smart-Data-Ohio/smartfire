use super::channel_thread_test::frozen;
use super::*;
use crate::models::agent_posting::{self as posting, Cap, PostingOutcome};
use crate::{ChannelThread, Error, Message, NewChannelThread, NewMessage, Room};
use rusqlite::params;

#[test]
fn ws11_daily_budget_windows_match_rails_across_dst_folds_and_midnight() {
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../vectors/agents_posting_budget_contract.json"
    ))
    .unwrap();
    for case in vectors["reset"].as_array().unwrap() {
        let now = crate::Timestamp::from_jiff(case["now"].as_str().unwrap().parse().unwrap());
        let zone = crate::slash_commands::time_parser::zone(case["zone"].as_str().unwrap());
        let window = posting::daily_window(now, &zone).unwrap();
        assert_eq!(window.day, case["day"].as_str().unwrap());
        assert_eq!(
            window.start.jiff(),
            case["start"]
                .as_str()
                .unwrap()
                .parse::<jiff::Timestamp>()
                .unwrap()
        );
        assert_eq!(
            window.end.jiff(),
            case["end"]
                .as_str()
                .unwrap()
                .parse::<jiff::Timestamp>()
                .unwrap()
        );
        assert_eq!(window.retry_after, case["retry_after"].as_i64().unwrap());
    }
}

#[test]
fn ws11_posting_budget_replays_before_counting_and_rolls_notices_back() {
    let t = frozen();
    t.write(|tx| {
        let attrs = || NewMessage {
            room_id: id("watercooler"),
            creator_id: id("david"),
            client_message_id: Some("ws11-idempotency".into()),
            body: Some("Original".into()),
            ..Default::default()
        };
        tx.conn().execute(
            "UPDATE agents SET daily_message_cap=1 WHERE id=?",
            [id("bender_agent")],
        )?;
        let PostingOutcome::Created(original) = posting::post(tx, id("bender_agent"), attrs())?
        else {
            panic!("first post was denied")
        };
        assert_eq!(
            original.creator_id,
            id("bender"),
            "submitted creator impersonated a human"
        );
        let PostingOutcome::Replay(replay) = posting::post(tx, id("bender_agent"), attrs())? else {
            panic!("budget swallowed replay")
        };
        assert_eq!(original.id, replay.id);
        assert_eq!(
            tx.conn()
                .query_row("SELECT COUNT(*) FROM agent_budget_notices", [], |r| r
                    .get::<_, i64>(0))?,
            0
        );
        Ok(())
    });
    let failed: Result<()> = t.try_write(|tx| {
        assert!(posting::check_budget(tx, id("bender_agent"), Cap::Messages)?.is_some());
        Err(Error::Other("rollback".into()))
    });
    assert!(failed.is_err());
    t.read(|conn| {
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM agent_budget_notices", [], |r| r
                .get::<_, i64>(0))?,
            0
        );
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM activity_items WHERE source_type='AgentBudgetNotice'",
                [],
                |r| r.get::<_, i64>(0)
            )?,
            0
        );
        Ok(())
    });
    t.write(|tx| {
        for _ in 0..3 {
            assert!(posting::check_budget(tx, id("bender_agent"), Cap::Messages)?.is_some());
        }
        assert_eq!(
            tx.conn()
                .query_row("SELECT COUNT(*) FROM agent_budget_notices", [], |r| r
                    .get::<_, i64>(0))?,
            1
        );
        assert_eq!(
            tx.conn().query_row(
                "SELECT COUNT(*) FROM activity_items WHERE source_type='AgentBudgetNotice'",
                [],
                |r| r.get::<_, i64>(0)
            )?,
            1
        );
        Ok(())
    });
}

#[test]
fn ws11_message_budget_excludes_board_openers_and_counts_local_days() {
    let t = frozen();
    let now: jiff::Timestamp = "2026-03-09T01:00:00Z".parse().unwrap();
    t.clock.travel_to(crate::Timestamp::from_jiff(now));
    t.write(|tx| {
        tx.conn().execute(
            "UPDATE agents SET daily_message_cap=1 WHERE id=?",
            [id("bender_agent")],
        )?;
        tx.conn().execute(
            "UPDATE users SET time_zone='Eastern Time (US & Canada)' WHERE id=?",
            [id("bender")],
        )?;
        tx.conn().execute(
            "UPDATE messages SET created_at='2026-01-01 00:00:00' WHERE creator_id=?",
            [id("bender")],
        )?;
        let opener = Message::create(
            tx,
            NewMessage {
                room_id: id("watercooler"),
                creator_id: id("bender"),
                body: Some("Opener".into()),
                board_post_opener: true,
                ..Default::default()
            },
        )?;
        assert!(posting::check_budget(tx, id("bender_agent"), Cap::Messages)?.is_none());
        let counted = Message::create(
            tx,
            NewMessage {
                room_id: id("watercooler"),
                creator_id: id("bender"),
                body: Some("Earlier local day".into()),
                ..Default::default()
            },
        )?;
        tx.conn().execute(
            "UPDATE messages SET created_at='2026-03-08 06:00:00' WHERE id=?",
            [counted.id],
        )?;
        let payload = posting::check_budget(tx, id("bender_agent"), Cap::Messages)?.unwrap();
        assert_eq!(payload["retry_after"], 10799);
        tx.conn().execute(
            "UPDATE messages SET created_at='2026-03-08 04:59:59.999999' WHERE id=?",
            [counted.id],
        )?;
        assert!(posting::check_budget(tx, id("bender_agent"), Cap::Messages)?.is_none());
        assert!(Message::find(tx.conn(), opener.id)?.board_post_opener);
        Ok(())
    });
}

#[test]
fn ws11_board_and_external_budgets_have_separate_notices_and_admin_fallback() {
    let t = frozen();
    t.write(|tx| {
        tx.conn().execute("UPDATE agents SET owner_id=NULL,daily_board_post_cap=1,daily_external_action_cap=1 WHERE id=?", [id("bender_agent")])?;
        tx.conn().execute("UPDATE rooms SET type='Rooms::Board' WHERE id=?", [id("watercooler")])?;
        Room::find(tx.conn(), id("watercooler"))?.grant_to(tx, &[id("bender")])?;
        assert!(posting::check_budget(tx, id("bender_agent"), Cap::BoardPosts)?.is_none());
        ChannelThread::create(tx, NewChannelThread { room_id: id("watercooler"), creator_id: id("bender"), name: Some("Post".into()), work_status: Some("planned".into()), ..Default::default() })?;
        assert!(posting::check_budget(tx, id("bender_agent"), Cap::BoardPosts)?.is_some());
        assert!(posting::check_budget(tx, id("bender_agent"), Cap::ExternalActions)?.is_none());
        tx.conn().execute("INSERT INTO agent_approvals (agent_id,action,summary,created_at,updated_at,expires_at) VALUES (?,'deploy','Deploy',?,?,?)", params![id("bender_agent"), tx.now(), tx.now(), tx.now()])?;
        assert!(posting::check_budget(tx, id("bender_agent"), Cap::ExternalActions)?.is_some());
        assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM agent_budget_notices WHERE agent_id=?", [id("bender_agent")], |r| r.get::<_, i64>(0))?, 2);
        let admins: i64 = tx.conn().query_row("SELECT COUNT(*) FROM users WHERE role=1 AND status=0", [], |r| r.get(0))?;
        assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM activity_items WHERE source_type='AgentBudgetNotice'", [], |r| r.get::<_, i64>(0))?, 2 * admins);
        Ok(())
    });
}

#[test]
fn ws11_budget_error_json_matches_rails_bytes() {
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../vectors/agents_posting_budget_contract.json"
    ))
    .unwrap();
    let t = frozen();
    t.clock.travel_to(crate::Timestamp::from_jiff(
        "2026-03-02T16:00:00Z".parse().unwrap(),
    ));
    let payload = t.write(|tx| {
        tx.conn().execute(
            "UPDATE agents SET daily_message_cap=1 WHERE id=?",
            [id("bender_agent")],
        )?;
        tx.conn().execute(
            "UPDATE users SET time_zone='UTC' WHERE id=?",
            [id("bender")],
        )?;
        Message::create(
            tx,
            NewMessage {
                room_id: id("watercooler"),
                creator_id: id("bender"),
                body: Some("Spent".into()),
                ..Default::default()
            },
        )?;
        Ok(posting::check_budget(tx, id("bender_agent"), Cap::Messages)?.unwrap())
    });
    assert_eq!(
        payload.to_string(),
        vectors["overflow"]["body"].as_str().unwrap()
    );
}
