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
    let grant_id = db.write(|tx| {
        let caller = crate::fixtures::identify("david");
        let room = crate::fixtures::identify("david_and_jason");
        let session = crate::Session::start(tx, caller, None, None)?;
        let member = crate::Membership::find_by_room_and_user(tx.conn(), room, caller)?.unwrap();
        Ok(tx.conn().query_row("INSERT INTO huddle_grants(identity,room_name,session_id,user_id,membership_id,room_id,last_issued_at,created_at,updated_at) VALUES('ws13b-policy-source','ws13b-policy-room',?,?,?,?,?,?,?) RETURNING id",rusqlite::params![session.id,caller,member.id,room,tx.now(),tx.now(),tx.now()],|r|r.get::<_,i64>(0))?)
    });
    for outcome in case["outcomes"].as_array().unwrap() {
        db.clock.travel_to(Timestamp::from_second(
            outcome["context"]["now"].as_i64().unwrap(),
        ));
        let mut invitation = outcome["broadcast"]["payload"]["huddleInvitation"].clone();
        invitation.as_object_mut().unwrap().remove("silent");
        let request = RingRequest {
            recipient_id: outcome["context"]["recipient"]["id"].as_i64().unwrap(),
            sender_id: outcome["sender_id"].as_i64().unwrap(),
            grant_id: Some(grant_id),
            invitation,
        };
        let setup = outcome["setup_sql"].as_str().unwrap().to_owned();
        let sender = outcome["sender_id"].as_i64().unwrap();
        db.write(move |tx| {
            tx.conn().execute_batch(&setup)?;
            tx.conn().execute("UPDATE huddle_grants SET user_id=? WHERE id=?", rusqlite::params![sender, grant_id])?;
            Ok(())
        });
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
