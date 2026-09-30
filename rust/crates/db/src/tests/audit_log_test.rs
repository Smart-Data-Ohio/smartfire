use super::*;
use crate::models::audit_log::{self as audit, Actor, AuditLog, Context, NewAuditLog, Target};
use crate::{NewUser, Room, Timestamp, User};
use serde_json::{Value, json};
fn golden() -> Value {
    serde_json::from_str(include_str!("ws8_audit_vectors.json")).unwrap()
}
fn fixture() -> TestDb {
    TestDb::with_clock(
        TestClock::frozen_at(Timestamp::parse_db("2026-03-10 12:00:00").unwrap()),
        4,
    )
}
#[test]
fn action_vocabulary_matches_rails() {
    assert_eq!(json!(audit::actions()), golden()["actions"]);
}
#[test]
fn nested_redaction_and_scalar_wrapping_match_rails() {
    for row in golden()["filters"].as_array().unwrap() {
        assert_eq!(
            audit::filter_secrets((!row["input"].is_null()).then_some(&row["input"])),
            row["output"]
        );
    }
}
#[test]
fn failure_labels_match_rails_without_preserving_mistyped_passwords() {
    let t = fixture();
    t.write(|tx| {
        User::create(
            tx,
            NewUser {
                name: "Dotless".into(),
                email_address: Some("dotless@intranet".into()),
                ..Default::default()
            },
        )
    });
    for row in golden()["labels"].as_array().unwrap() {
        let actual = t.read(|c| audit::failure_actor_label(c, row["input"].as_str()));
        assert_eq!(json!(actual), row["label"]);
    }
}
#[test]
fn webhook_origins_and_digests_match_rails() {
    for row in golden()["origins"].as_array().unwrap() {
        assert_eq!(
            json!(audit::webhook_origin_summary(row["input"].as_str()).unwrap()),
            row["summary"],
            "{row}"
        );
    }
}
fn records(t: &TestDb) -> Vec<AuditLog> {
    t.write(|tx|{
 let admin=User::find(tx.conn(),id("david"))?;let member=User::find(tx.conn(),id("kevin"))?;let room=Room::find(tx.conn(),id("watercooler"))?;let context=Context{actor:Some(Actor::from(&admin)),..Default::default()};
 Ok(vec![AuditLog::record(tx,NewAuditLog{action:"user.role.change".into(),actor:Some(Actor::from(&admin)),target:Some(Target::from(&member)),changes:Some(json!({"role":audit::pair(json!("member"),json!("administrator")),"token":"secret"})),ip_address:Some("203.0.113.7".into()),user_agent:Some("a".repeat(600)),..Default::default()},&context)?,AuditLog::record(tx,NewAuditLog{action:"room.destroy".into(),target:Some(Target::from(&room)),..Default::default()},&context)?,AuditLog::record(tx,NewAuditLog{action:"future.action".into(),actor_label:Some("Override".into()),target_label:Some("Target".into()),changes:Some(json!("context")),..Default::default()},&context)?])
 })
}
#[test]
fn persisted_snapshots_and_current_context_match_rails() {
    let t = fixture();
    let entries = records(&t);
    assert_eq!(
        json!(entries.iter().map(AuditLog::snapshot).collect::<Vec<_>>()),
        golden()["records"]
    );
    let stored = entries[0].id;
    t.write(|tx| {
        tx.conn()
            .execute("UPDATE users SET name='Renamed' WHERE id=?", [id("kevin")])?;
        Ok(())
    });
    assert_eq!(
        t.read(|c| AuditLog::find(c, stored)).snapshot(),
        golden()["records"][0]
    );
}
#[test]
fn persisted_rows_refuse_all_mutations_like_rails() {
    let t = fixture();
    let entry = records(&t)[0].clone();
    let result = t.write(move |tx| {
        Ok(vec![
            entry.save(tx),
            entry.destroy(tx),
            entry.delete(tx),
            entry.update(tx, "user.unban"),
        ]
        .into_iter()
        .map(|r| if r.is_err() { "readonly" } else { "allowed" })
        .collect::<Vec<_>>())
    });
    assert_eq!(json!(result), golden()["refusals"]);
}
#[test]
fn action_presence_validation_matches_rails() {
    let t = fixture();
    for row in golden()["validation"].as_array().unwrap() {
        let action = row["action"].as_str().unwrap().to_string();
        let valid = t
            .try_write(move |tx| {
                AuditLog::record(
                    tx,
                    NewAuditLog {
                        action,
                        ..Default::default()
                    },
                    &Context::default(),
                )
            })
            .is_ok();
        assert_eq!(json!(valid), row["valid"]);
    }
}
#[test]
fn sign_in_failure_throttle_matches_rails_at_window_boundaries() {
    let t = fixture();
    let base = t.now();
    let ctx = Context {
        ip_address: Some("198.51.100.9".into()),
        user_agent: Some("Audit browser".into()),
        ..Default::default()
    };
    for row in golden()["failures"].as_array().unwrap() {
        t.clock
            .travel_to(base.since(jiff::SignedDuration::from_secs(
                row["offset"].as_i64().unwrap(),
            )));
        let ctx = ctx.clone();
        let email = row["email"].as_str().unwrap().to_string();
        let result =
            t.write(move |tx| AuditLog::record_sign_in_failure(tx, &email, "password", &ctx));
        let output=result.map(|e|json!({"action":e.action,"actor_id":e.actor_id,"actor_label":e.actor_label,"details":e.details,"ip_address":e.ip_address,"user_agent":e.user_agent}));
        assert_eq!(json!(output), row["result"], "{row}");
        let count = t.read(|c| {
            crate::sql::count(
                c,
                "SELECT COUNT(*) FROM audit_logs WHERE action='session.sign_in.failure'",
                [],
            )
        });
        assert_eq!(json!(count), row["count"]);
    }
}
