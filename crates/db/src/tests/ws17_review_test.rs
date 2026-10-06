//! Independent-review regressions, replayable unchanged on 1021be6a.
use super::*;
use crate::{Error, KeywordAlert};
use serde_json::Value;

fn unicode() -> Value {
    serde_json::from_str(include_str!("../../../../vectors/ws17_unicode.json")).unwrap()
}

#[test]
fn ws17_review_keyword_replacements_follow_ruby_downcase() {
    let t = TestDb::new();
    for row in unicode()["replacements"].as_array().unwrap() {
        let lines = vec![row["lines"].as_str().unwrap().to_owned()];
        let result = t.try_write(move |tx| {
            crate::models::user_status_settings::replace_keyword_alerts(tx, id("david"), &lines)
        });
        assert_eq!(result.is_ok(), row["success"].as_bool().unwrap(), "{row}");
        let actual = t.read(|c| {
            Ok(KeywordAlert::for_user(c, id("david"))?
                .into_iter()
                .map(|a| a.phrase)
                .collect::<Vec<_>>())
        });
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            row["phrases"],
            "{row}"
        );
    }
}

#[test]
fn ws17_review_keyword_create_and_update_follow_ruby_sql_validation() {
    let t = TestDb::new();
    for row in unicode()["validations"].as_array().unwrap() {
        let existing = row["existing"].as_str().unwrap().to_owned();
        t.write(move |tx| {
            tx.conn()
                .execute("DELETE FROM keyword_alerts WHERE user_id=?", [id("david")])?;
            KeywordAlert::create(tx, id("david"), &existing)
        });
        for update in [false, true] {
            let candidate = row["candidate"].as_str().unwrap().to_owned();
            let result = t.try_write(move |tx| {
                tx.savepoint(|tx| {
                    if update {
                        let mut alert = KeywordAlert::create(tx, id("david"), "other-keyword")?;
                        alert.update(tx, &candidate)?;
                    } else {
                        KeywordAlert::create(tx, id("david"), &candidate)?;
                    }
                    // Roll back successful probes so the create and update see the same rows.
                    Err::<(), _>(Error::Other("successful probe".into()))
                })
            });
            let errors = match result.unwrap_err() {
                Error::RecordInvalid(errors) => errors.full_messages(),
                Error::Other(message) if message == "successful probe" => Vec::new(),
                other => panic!("{other}"),
            };
            assert_eq!(
                serde_json::to_value(errors).unwrap(),
                row["errors"],
                "update={update} {row}"
            );
        }
    }
}

#[test]
fn ws17_review_keyword_matcher_follows_rails_unicode_corpus() {
    let v = unicode();
    let mut differences = Vec::new();
    for row in v["matcher"].as_array().unwrap() {
        let phrases = vec![(17, vec![row["phrase"].as_str().unwrap().to_owned()])];
        let actual = crate::models::keyword_alert::matching_user_ids(
            &phrases,
            row["text"].as_str().unwrap(),
        )
        .unwrap();
        if serde_json::to_value(&actual).unwrap() != row["users"] {
            differences.push(format!("actual={actual:?} {row}"));
        }
    }
    assert!(
        differences.is_empty(),
        "{} Unicode differences: {}",
        differences.len(),
        differences.join("\n")
    );
}

#[test]
fn ws17_review_board_tag_matches_fixed_rails_without_oracle_repair() {
    let v: Value = serde_json::from_str(include_str!(
        "../../../../vectors/ws17_notification_push.json"
    ))
    .unwrap();
    let row = v["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "board_nudge")
        .unwrap();
    let t = TestDb::new();
    let sql = row["setup_sql"].as_str().unwrap().to_owned();
    t.write(move |tx| {
        tx.conn().execute_batch(&sql)?;
        Ok(())
    });
    let nudge = row["nudge_id"].as_i64().unwrap();
    let actual =
        t.read(|c| crate::models::notification_push::BoardNudgeSource::find(c, nudge)?.payload(c));
    assert_eq!(
        serde_json::to_value(actual.tag).unwrap(),
        row["deliveries"][0]["payload"]["tag"]
    );
}

#[test]
fn ws17_review_board_humanize_preserves_ruby_casing_and_acronyms() {
    let push: Value = serde_json::from_str(include_str!(
        "../../../../vectors/ws17_notification_push.json"
    ))
    .unwrap();
    let row = push["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "board_nudge")
        .unwrap();
    let t = TestDb::new();
    let sql = row["setup_sql"].as_str().unwrap().to_owned();
    t.write(move |tx| {
        tx.conn().execute_batch(&sql)?;
        Ok(())
    });
    let nudge = row["nudge_id"].as_i64().unwrap();
    let prefix = row["deliveries"][0]["payload"]["body"]
        .as_str()
        .unwrap()
        .split_once(" sitting in ")
        .unwrap()
        .0;
    for case in unicode()["humanized"].as_array().unwrap() {
        let status = case["value"].as_str().unwrap().to_owned();
        t.write(move |tx| {
            tx.conn().execute(
                "UPDATE board_sla_nudges SET work_status=? WHERE id=?",
                rusqlite::params![status, nudge],
            )?;
            Ok(())
        });
        let actual = t.read(|c| {
            crate::models::notification_push::BoardNudgeSource::find(c, nudge)?.payload(c)
        });
        assert_eq!(
            actual.body,
            format!(
                "{prefix} sitting in {}",
                case["humanized"].as_str().unwrap()
            ),
            "{case}"
        );
    }
}

#[test]
fn ws17_review_board_tag_normalization_follows_ruby_strip_and_downcase() {
    let v = unicode();
    let names: Vec<String> = serde_json::from_value(v["tags"].clone()).unwrap();
    let actual = crate::models::channel_thread::normalize_tag_names(&names);
    assert_eq!(serde_json::to_value(actual).unwrap(), v["normalized_tags"]);
}
