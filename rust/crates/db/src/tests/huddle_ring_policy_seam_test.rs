//! Eight original RingPolicy declarations, executable at the unchanged WS17
//! publication seam. Each depends on WS17 to derive sound_allowed from the
//! captured real policy context. These tests prove publication, not that adapter.
use crate::Timestamp;
use crate::models::huddle_invitations::{RingRequest, publish_ring};
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
    assert_eq!(
        case["depends_on"],
        "WS17 Notifications::Policy and RingPolicy quiet-check adapter (not merged)"
    );
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
        // Explicit oracle decision: the unmerged WS17 adapter is the sole
        // remaining dependency, never an assumed policy default.
        let sound_allowed = outcome["sound_allowed"].as_bool().unwrap();
        db.sink.take();
        db.write(move |tx| publish_ring(tx, &request, sound_allowed));
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
            "WS17-dependent {name}: {}",
            outcome["context"]
        );
    }
}
macro_rules! cases { ($($name:ident => $case:literal),* $(,)?) => {$ (
    #[test] fn $name() { run($case); }
)*}; }
cases!(ws17_input_default => "default", ws17_input_dnd => "dnd", ws17_input_allowed_dnd => "allowed_dnd", ws17_input_quiet_override => "quiet_override", ws17_input_meeting => "meeting", ws17_input_allowed_meeting => "allowed_meeting", ws17_input_ooo => "ooo", ws17_input_allowed_ooo => "allowed_ooo");
