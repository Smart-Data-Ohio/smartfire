//! All eight RingPolicy declarations derive real WS17 policy from persisted Rails contexts.
use crate::Timestamp;
use crate::models::huddle_invitations::{RingRequest, ring_allowed, publish_ring_with_policy};
use crate::tests::TestDb;
use serde_json::{Value, json};

fn run(name: &str) {
    let fixture: Value =
        serde_json::from_str(include_str!("huddle_ring_policy_seam.json")).unwrap();
    assert_eq!(fixture["reference_pin"], "d7c7de92");
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 8);
    let case = fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == name)
        .unwrap();
    let db = TestDb::new();
    for outcome in case["outcomes"].as_array().unwrap() {
        db.clock.travel_to(Timestamp::from_second(
            outcome["context"]["now"].as_i64().unwrap(),
        ));
        let mut invitation = outcome["broadcast"]["payload"]["huddleInvitation"].clone();
        invitation.as_object_mut().unwrap().remove("silent");
        let request = RingRequest {
            recipient_id: outcome["context"]["recipient"]["id"].as_i64().unwrap(),
            sender_id: outcome["sender_id"].as_i64().unwrap(),
            invitation,
        };
        let setup = outcome["setup_sql"].as_str().unwrap().to_owned();
        db.write(move |tx| { tx.conn().execute_batch(&setup)?; Ok(()) });
        let quiet = |recipient: &crate::UserStatusSettings| recipient.user.id == crate::fixtures::identify("jason");
        let override_check = outcome["context"]["quiet_override"].as_bool().unwrap()
            .then_some(&quiet as &dyn Fn(&crate::UserStatusSettings) -> bool);
        let sound_allowed = db.read(|conn| ring_allowed(conn, request.recipient_id,
            outcome["context"]["caller_id"].as_i64(), db.now(), override_check));
        assert_eq!(serde_json::json!(sound_allowed), outcome["sound_allowed"], "{name} real WS17 decision");
        db.sink.take();
        let override_active = outcome["context"]["quiet_override"].as_bool().unwrap();
        db.write(move |tx| {
            let quiet = |recipient: &crate::UserStatusSettings| recipient.user.id == crate::fixtures::identify("jason");
            let check = override_active.then_some(&quiet as &dyn Fn(&crate::UserStatusSettings) -> bool);
            publish_ring_with_policy(tx, &request, check)
        });
        let frames = db
            .events()
            .iter()
            .filter_map(|event| match event.as_broadcast()? {
                crate::broadcasts::Broadcast::Cable { stream, payload } => {
                    Some(json!({"stream":stream,"payload":payload}))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            json!(frames),
            json!([outcome["broadcast"]]),
            "{name}: {}",
            outcome["context"]
        );
    }
}
macro_rules! cases { ($($name:ident => $case:literal),* $(,)?) => {$ (
    #[test] fn $name() { run($case); }
)*}; }
cases!(policy_default => "default", policy_dnd => "dnd", policy_allowed_dnd => "allowed_dnd", policy_quiet_override => "quiet_override", policy_meeting => "meeting", policy_allowed_meeting => "allowed_meeting", policy_ooo => "ooo", policy_allowed_ooo => "allowed_ooo");
