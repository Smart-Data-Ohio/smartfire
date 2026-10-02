//! Rails BoardSlaNudge validations, microsecond clock boundaries and recorder authorization.
use super::*;
use crate::models::activity_item::ActivitySource;
use crate::{ActivityItem, BoardSlaNudge, NewBoardSlaNudge, Timestamp};
use serde_json::{Value, json};

fn oracle() -> Value {
    serde_json::from_str(include_str!("../../../../vectors/board_sla_nudge.json")).unwrap()
}
fn input(value: &Value) -> NewBoardSlaNudge {
    NewBoardSlaNudge {
        room_id: value["room_id"].as_i64().unwrap(),
        channel_thread_id: value["channel_thread_id"].as_i64().unwrap(),
        recipient_id: value["recipient_id"].as_i64().unwrap(),
        work_status: value["work_status"].as_str().map(str::to_owned),
        stage: value["stage"].as_str().map(str::to_owned),
        status_entered_at: value["status_entered_at"]
            .as_str()
            .and_then(Timestamp::parse_db),
    }
}
fn fixture() -> TestDb {
    let t = TestDb::with_clock(TestClock::frozen_at(Timestamp::from_second(1772467200)), 4);
    let setup = oracle()["setup"].as_array().unwrap().clone();
    t.write(move |tx| {
        for sql in setup {
            tx.conn().execute_batch(sql.as_str().unwrap())?;
        }
        Ok(())
    });
    t.sink.take();
    t
}

#[test]
fn board_sla_nudge_models_match_rails_validations_and_wait_boundaries() {
    for row in oracle()["models"].as_array().unwrap() {
        let t = fixture();
        let row = row.clone();
        t.write(move |tx| {
            if row["duplicate"] == true {
                BoardSlaNudge::create(tx, input(&oracle()["models"][0]["input"]))?;
            }
            let attributes = input(&row["input"]);
            let errors = BoardSlaNudge::validate(tx.conn(), &attributes, None)?;
            let mut actual = serde_json::Map::new();
            for (key, message) in &errors.0 {
                actual
                    .entry(key.to_string())
                    .or_insert(json!([]))
                    .as_array_mut()
                    .unwrap()
                    .push(json!(message));
            }
            assert_eq!(json!(actual), row["errors"], "{}", row["name"]);
            assert_eq!(
                json!(errors.full_messages()),
                row["full_messages"],
                "{}",
                row["name"]
            );
            assert_eq!(errors.is_empty(), row["valid"] == true);
            let before: i64 =
                tx.conn()
                    .query_row("SELECT COUNT(*) FROM activity_items", [], |r| r.get(0))?;
            match BoardSlaNudge::create(tx, attributes) {
                Ok(nudge) => {
                    assert_eq!(row["valid"], true);
                    assert_eq!(
                        json!(nudge.waited_minutes(tx.now())),
                        row["waited"],
                        "{}",
                        row["name"]
                    );
                    assert_eq!(json!(nudge.activity_recipient_ids()), row["recipients"]);
                    assert_eq!(nudge.created_at, tx.now());
                    assert_eq!(BoardSlaNudge::find(tx.conn(), nudge.id)?, nudge);
                    assert_eq!(BoardSlaNudge::for_ids(tx.conn(), &[nudge.id])?, vec![nudge]);
                }
                Err(crate::Error::RecordInvalid(actual)) => assert_eq!(actual, errors),
                Err(error) => return Err(error),
            }
            let after: i64 =
                tx.conn()
                    .query_row("SELECT COUNT(*) FROM activity_items", [], |r| r.get(0))?;
            assert_eq!(json!(after - before), row["activity_delta"]);
            Ok(())
        });
        assert!(
            t.events().is_empty(),
            "bare claim creation has no recorder/job callbacks"
        );
    }
}

#[test]
fn board_sla_nudge_recorder_matches_rails_recipient_and_idempotency_facts() {
    for row in oracle()["recorder"].as_array().unwrap() {
        let t = fixture();
        let row = row.clone();
        t.write(move |tx| {
            let mut attributes = input(&oracle()["models"][0]["input"]);
            attributes.recipient_id = row["recipient"].as_i64().unwrap();
            if row["name"] == "inactive" { tx.conn().execute("UPDATE users SET status=1 WHERE id=?", [attributes.recipient_id])?; }
            let nudge = BoardSlaNudge::create(tx, attributes)?;
            let caller = row["caller"].as_i64().unwrap();
            let item = ActivityItem::record(tx, caller, ActivitySource::BoardSlaNudge(nudge.id), "work_sla", false)?;
            assert_eq!(item.is_some(), !row["result"].is_null(), "{}",row["name"]);
            if row["name"] == "handled-repeated" { item.as_ref().unwrap().mark_handled(tx)?; }
            if row["name"] == "repeated" || row["name"] == "handled-repeated" {
                let again = ActivityItem::record(tx, caller, ActivitySource::BoardSlaNudge(nudge.id), "work_sla", false)?.unwrap();
                assert_eq!(again.id, item.as_ref().unwrap().id);
                assert_eq!(json!(again.state()), row["state"]);
            }
            let count: i64 = tx.conn().query_row("SELECT COUNT(*) FROM activity_items WHERE source_type='BoardSlaNudge' AND source_id=?", [nudge.id], |r|r.get(0))?;
            assert_eq!(json!(count), row["count"]);
            nudge.destroy(tx)?;
            let remaining: i64 = tx.conn().query_row("SELECT COUNT(*) FROM activity_items WHERE source_type='BoardSlaNudge' AND source_id=?", [nudge.id], |r|r.get(0))?;
            assert_eq!(json!(remaining), row["remaining_items"]);
            assert!(BoardSlaNudge::find_by_id(tx.conn(), nudge.id)?.is_none());
            assert!(ActivityItem::find_by_user_and_source(tx.conn(), caller, "BoardSlaNudge", nudge.id)?.is_none());
            Ok(())
        });
    }
}

#[test]
fn board_sla_nudge_new_crossing_or_stage_is_a_distinct_claim() {
    let t = fixture();
    let first = t.write(|tx| BoardSlaNudge::create(tx, input(&oracle()["models"][0]["input"])));
    t.clock.travel(jiff::SignedDuration::from_secs(60));
    assert_eq!(first.waited_minutes(t.now()), 121);
    assert!(
        t.try_write(|tx| BoardSlaNudge::create(tx, input(&oracle()["models"][0]["input"])))
            .is_err()
    );
    let second = t.write(|tx| {
        let mut attributes = input(&oracle()["models"][0]["input"]);
        attributes.status_entered_at = Some(tx.now());
        BoardSlaNudge::create(tx, attributes)
    });
    assert_ne!(first.id, second.id);
    assert_eq!(second.waited_minutes(t.now()), 0);
    assert!(t.read(|conn| BoardSlaNudge::for_ids(conn, &[])).is_empty());
}
