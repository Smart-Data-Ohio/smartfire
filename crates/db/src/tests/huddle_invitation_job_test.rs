use crate::Timestamp;
use crate::models::huddle_notices::{self, PushRequest};
use crate::tests::{TestDb, huddle_notices_test};
use rusqlite::params;
use serde_json::{Value, json};

fn run(name: &str) {
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../campfire/src/huddle/huddle_job_contract_vectors.json"
    ))
    .unwrap();
    assert_eq!(vectors["invitations"].as_array().unwrap().len(), 4);
    let case = vectors["invitations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == name)
        .unwrap();
    let clock = crate::time::TestClock::new();
    clock.travel_to(Timestamp::from_second(vectors["now"].as_i64().unwrap()));
    let db = TestDb::with_clock(clock, 4);
    let input = case["input"].clone();
    let item = input["item"]["id"].as_i64().unwrap();
    db.write(move |tx|{
        tx.conn().execute_batch("DELETE FROM huddle_cleanups; DELETE FROM activity_items WHERE source_type='HuddleGrant'; DELETE FROM huddle_grants;")?;
        huddle_notices_test::insert(tx,"sessions",&input["session"])?;
        huddle_notices_test::insert(tx,"huddle_grants",&input["grant"])?;
        if input["missing"]!=true {huddle_notices_test::insert(tx,"activity_items",&input["item"])?;}
        tx.conn().execute("UPDATE memberships SET involvement=?,connected_at=? WHERE id=?",params![input["involvement"].as_str(),input["connected_at"].as_str().and_then(Timestamp::parse_db),input["membership_id"].as_i64()])?;
        Ok(())
    });
    db.sink.take();
    db.write(move |tx| huddle_notices::push_invitation(tx, item));
    let mut pushes = Vec::new();
    for request in db.events().iter().filter_map(|e| e.as_job::<PushRequest>()) {
        assert_eq!(request.kind, huddle_notices::PushKind::Huddle);
        assert_eq!(
            request.recipient_id,
            case["input"]["recipient_id"].as_i64().unwrap()
        );
        assert_eq!(
            request.sender_id,
            case["input"]["sender_id"].as_i64().unwrap()
        );
        let delivery = db
            .write(move |tx| huddle_notices::prepare_push(tx, &request, true))
            .unwrap();
        pushes.push(json!({"payload":delivery.payload,"subscription_ids":delivery.subscriptions.iter().map(|s|s.id).collect::<Vec<_>>()}));
    }
    assert_eq!(json!(pushes), case["pushes"], "{name}");
    if name == "missing" {
        assert!(db.events().is_empty());
    }
}
#[test]
fn recipient() {
    run("recipient");
}
#[test]
fn off() {
    run("off");
}
#[test]
fn connected() {
    run("connected");
}
#[test]
fn missing() {
    run("missing");
}
