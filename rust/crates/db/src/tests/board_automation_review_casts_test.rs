use super::*;
use crate::{BoardSlaRule, NewBoardSlaRule, Timestamp};
use serde_json::{Value, json};
#[test]
fn review_pr206_decimal_casts_and_errors() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../../vectors/board_automation_review_casts.json"
    ))
    .unwrap();
    let t = TestDb::with_clock(
        TestClock::frozen_at(Timestamp::parse_db("2026-03-02 16:00:00").unwrap()),
        4,
    );
    t.write(|tx| {
        tx.conn().execute_batch(
            "UPDATE rooms SET type='Rooms::Board' WHERE id=486777696;DELETE FROM board_sla_rules;",
        )?;
        Ok(())
    });
    let mut differences = Vec::new();
    for row in data["rows"].as_array().unwrap() {
        let input = row["input"].as_str().map(str::to_owned);
        let copied = input.clone();
        let (cast, errors) = t.read(move |conn| {
            let errors = BoardSlaRule::validate(
                conn,
                &NewBoardSlaRule {
                    room_id: 486777696,
                    work_status: Some("planned".into()),
                    nudge_after_minutes: copied.clone(),
                    escalate_after_minutes: Some("10".into()),
                },
                None,
            )?;
            Ok((BoardSlaRule::cast_threshold(copied.as_deref()), errors))
        });
        let mut fields = serde_json::Map::new();
        for (key, message) in &errors.0 {
            fields
                .entry(key.to_string())
                .or_insert(json!([]))
                .as_array_mut()
                .unwrap()
                .push(json!(message));
        }
        let actual = json!({"cast":cast,"valid":errors.is_empty(),"errors":fields,"full_messages":errors.full_messages()});
        let expected = json!({"cast":row["cast"],"valid":row["valid"],"errors":row["errors"],"full_messages":row["full_messages"]});
        if actual != expected {
            let label = input
                .as_deref()
                .unwrap_or("<nil>")
                .chars()
                .take(60)
                .collect::<String>();
            println!("REVIEW decimal input={label:?}: Rust={actual}; Rails={expected}");
            differences.push(label);
        }
    }
    println!(
        "REVIEW decimal casts: {} inputs; {} differences",
        data["rows"].as_array().unwrap().len(),
        differences.len()
    );
    assert!(
        differences.is_empty(),
        "decimal mismatches: {differences:?}"
    );
}
