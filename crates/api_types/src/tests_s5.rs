//! Wire shapes of the S5 types: huddles, voice rooms, moderation and stage rooms.

use serde_json::json;

use crate::tests::{assert_wire, user};
use crate::*;

fn participant() -> HuddleParticipant {
    HuddleParticipant {
        user_id: 7,
        membership_id: 31,
        identities: vec![
            "campfire-participant-0f3a9c".into(),
            "campfire-participant-77be10".into(),
        ],
        server_muted: false,
    }
}

fn participant_json() -> serde_json::Value {
    json!({
        "userId": 7,
        "membershipId": 31,
        "identities": ["campfire-participant-0f3a9c", "campfire-participant-77be10"],
        "serverMuted": false,
    })
}

fn presence() -> HuddlePresence {
    HuddlePresence {
        room_id: 12,
        participants: vec![participant()],
        live: false,
    }
}

fn presence_json() -> serde_json::Value {
    json!({"roomId": 12, "participants": [participant_json()], "live": false})
}

fn stage() -> StageState {
    StageState {
        room_id: 40,
        members: vec![
            StageMember {
                membership_id: 31,
                user_id: 7,
                role: StageRole::Host,
                hand_raised_at: None,
                server_muted: false,
            },
            StageMember {
                membership_id: 32,
                user_id: 8,
                role: StageRole::Listener,
                hand_raised_at: Some("2026-10-06T11:02:03.456Z".into()),
                server_muted: true,
            },
        ],
        live: Some(StageStream {
            id: 5,
            membership_id: 31,
            user_id: 7,
            identity: Some("campfire-participant-0f3a9c".into()),
            quality: StreamQuality::P1080Fps15,
            started_at: "2026-10-06T11:00:00.000Z".into(),
        }),
    }
}

fn stage_json() -> serde_json::Value {
    json!({
        "roomId": 40,
        "members": [
            {"membershipId": 31, "userId": 7, "role": "host", "handRaisedAt": null, "serverMuted": false},
            {"membershipId": 32, "userId": 8, "role": "listener", "handRaisedAt": "2026-10-06T11:02:03.456Z", "serverMuted": true},
        ],
        "live": {
            "id": 5,
            "membershipId": 31,
            "userId": 7,
            "identity": "campfire-participant-0f3a9c",
            "quality": "1080p15",
            "startedAt": "2026-10-06T11:00:00.000Z",
        },
    })
}

#[test]
fn the_presence_list_carries_each_live_room_and_its_people() {
    let list = HuddlePresenceList {
        rooms: vec![presence()],
        users: vec![user()],
    };
    let wire = json!({
        "rooms": [presence_json()],
        "users": [serde_json::to_value(user()).unwrap()],
    });
    assert_wire(&list, wire);

    let mut stage_room = presence();
    stage_room.room_id = 40;
    stage_room.live = true;
    stage_room.participants[0].server_muted = true;
    stage_room.participants[0].identities.truncate(1);
    assert_wire(
        &stage_room,
        json!({"roomId": 40, "participants": [{
            "userId": 7,
            "membershipId": 31,
            "identities": ["campfire-participant-0f3a9c"],
            "serverMuted": true,
        }], "live": true}),
    );
    assert_wire(
        &HuddlePresence {
            room_id: 12,
            participants: vec![],
            live: false,
        },
        json!({"roomId": 12, "participants": [], "live": false}),
    );
}

#[test]
fn a_room_call_and_its_credentials() {
    assert_wire(
        &HuddleDetail {
            room_name: "Grace Hopper".into(),
            presence: presence(),
            users: vec![user()],
        },
        json!({
            "roomName": "Grace Hopper",
            "presence": presence_json(),
            "users": [serde_json::to_value(user()).unwrap()],
        }),
    );
    assert_wire(
        &HuddleCredentials {
            url: "wss://huddles.example.com".into(),
            token: "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ4In0.c2ln".into(),
            identity: "campfire-participant-0f3a9c".into(),
            grant_id: 901,
            room_id: 12,
            room_name: "Grace Hopper".into(),
            can_publish: true,
        },
        json!({
            "url": "wss://huddles.example.com",
            "token": "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ4In0.c2ln",
            "identity": "campfire-participant-0f3a9c",
            "grantId": 901,
            "roomId": 12,
            "roomName": "Grace Hopper",
            "canPublish": true,
        }),
    );
}

#[test]
fn moderation_names_the_member_and_the_action() {
    for (action, name) in [
        (HuddleModeration::Mute, "mute"),
        (HuddleModeration::Unmute, "unmute"),
        (HuddleModeration::Disconnect, "disconnect"),
    ] {
        assert_wire(
            &ModerateHuddle {
                membership_id: 32,
                action,
            },
            json!({"membershipId": 32, "action": name}),
        );
    }
}

#[test]
fn a_stage_lists_every_member_and_the_live_stream() {
    assert_wire(&stage(), stage_json());
    assert_wire(
        &StageDetail {
            stage: stage(),
            users: vec![user()],
        },
        json!({"stage": stage_json(), "users": [serde_json::to_value(user()).unwrap()]}),
    );
    let mut quiet = stage();
    quiet.live = None;
    let mut wire = stage_json();
    wire["live"] = json!(null);
    assert_wire(&quiet, wire);

    let mut lost = stage();
    lost.live.as_mut().unwrap().identity = None;
    assert_eq!(
        serde_json::to_value(&lost).unwrap()["live"]["identity"],
        json!(null)
    );
}

#[test]
fn stage_requests() {
    for (role, name) in [
        (StageRole::Listener, "listener"),
        (StageRole::Speaker, "speaker"),
        (StageRole::Host, "host"),
    ] {
        assert_wire(&ChangeStageRole { role }, json!({"role": name}));
    }
    assert!(serde_json::from_value::<ChangeStageRole>(json!({"role": "owner"})).is_err());
    assert_wire(
        &LowerHand {
            membership_id: None,
        },
        json!({"membershipId": null}),
    );
    assert_wire(
        &LowerHand {
            membership_id: Some(32),
        },
        json!({"membershipId": 32}),
    );
    for (quality, name) in [
        (StreamQuality::P720Fps15, "720p15"),
        (StreamQuality::P1080Fps15, "1080p15"),
        (StreamQuality::P1080Fps30, "1080p30"),
    ] {
        assert_wire(&StartStageStream { quality }, json!({"quality": name}));
    }
    assert!(serde_json::from_value::<StartStageStream>(json!({"quality": "4k60"})).is_err());
    assert_wire(
        &StopStageStream { stream_id: Some(5) },
        json!({"streamId": 5}),
    );
    assert_wire(
        &StopStageStream { stream_id: None },
        json!({"streamId": null}),
    );
}

#[test]
fn an_unconfigured_server_is_a_503() {
    let error = ApiError::Unavailable {
        message: "Huddles are not configured".into(),
    };
    assert_eq!(error.status(), 503);
    assert_wire(
        &ApiErrorResponse { error },
        json!({"error": {"_tag": "Unavailable", "message": "Huddles are not configured"}}),
    );
}

#[test]
fn notices_are_tagged_by_kind() {
    assert_wire(
        &HuddleNotice::Joined {
            room_id: 12,
            room_name: "general".into(),
            user_id: 8,
            user_name: "Grace Hopper".into(),
            in_call: true,
            rejoin: false,
        },
        json!({
            "kind": "joined",
            "roomId": 12,
            "roomName": "general",
            "userId": 8,
            "userName": "Grace Hopper",
            "inCall": true,
            "rejoin": false,
        }),
    );
    assert_wire(
        &HuddleNotice::Left {
            room_id: 12,
            room_name: "general".into(),
            user_id: 8,
            user_name: "Grace Hopper".into(),
        },
        json!({"kind": "left", "roomId": 12, "roomName": "general", "userId": 8, "userName": "Grace Hopper"}),
    );
    assert_wire(
        &HuddleNotice::Ended { room_id: 12 },
        json!({"kind": "ended", "roomId": 12}),
    );
}

#[test]
fn every_s5_event_has_its_type_and_data() {
    let cases = vec![
        (
            "user",
            SyncPayload::HuddlePresence(presence()),
            json!({"type": "huddle.presence", "data": presence_json()}),
        ),
        (
            "user",
            SyncPayload::HuddleRole(HuddleRoleChanged {
                room_id: 40,
                stage_role: Some(StageRole::Speaker),
                server_muted: false,
            }),
            json!({"type": "huddle.role", "data": {"roomId": 40, "stageRole": "speaker", "serverMuted": false}}),
        ),
        (
            "user",
            SyncPayload::HuddleRole(HuddleRoleChanged {
                room_id: 12,
                stage_role: None,
                server_muted: true,
            }),
            json!({"type": "huddle.role", "data": {"roomId": 12, "stageRole": null, "serverMuted": true}}),
        ),
        (
            "user",
            SyncPayload::HuddleNotice(HuddleNotice::Ended { room_id: 12 }),
            json!({"type": "huddle.notice", "data": {"kind": "ended", "roomId": 12}}),
        ),
        (
            "user",
            SyncPayload::HuddleRing(HuddleRing {
                activity_item_id: Some(77),
                event: HuddleRingEvent::Started,
                state: HuddleRingState::Unread,
                room_id: 12,
                room_name: "Grace Hopper".into(),
                caller_name: "Grace Hopper".into(),
                silent: true,
            }),
            json!({"type": "huddle.ring", "data": {
                "activityItemId": 77,
                "event": "started",
                "state": "unread",
                "roomId": 12,
                "roomName": "Grace Hopper",
                "callerName": "Grace Hopper",
                "silent": true,
            }}),
        ),
        (
            "user",
            SyncPayload::HuddleRing(HuddleRing {
                activity_item_id: None,
                event: HuddleRingEvent::Ended,
                state: HuddleRingState::Handled,
                room_id: 12,
                room_name: "general".into(),
                caller_name: "Grace Hopper".into(),
                silent: false,
            }),
            json!({"type": "huddle.ring", "data": {
                "activityItemId": null,
                "event": "ended",
                "state": "handled",
                "roomId": 12,
                "roomName": "general",
                "callerName": "Grace Hopper",
                "silent": false,
            }}),
        ),
        (
            "room:40",
            SyncPayload::StageUpdated(stage()),
            json!({"type": "stage.updated", "data": stage_json()}),
        ),
        (
            "user",
            SyncPayload::StageStreamStopped(StageStreamStopped { room_id: 40 }),
            json!({"type": "stage.stream.stopped", "data": {"roomId": 40}}),
        ),
    ];
    for (topic, payload, wire) in cases {
        let event = SyncEvent {
            seq: 9,
            topic: topic.into(),
            payload,
        };
        let mut expected = wire;
        expected["seq"] = json!(9);
        expected["topic"] = json!(topic);
        assert_wire(&event, expected);
    }
}
