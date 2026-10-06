use std::collections::BTreeMap;
use std::fmt::Debug;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::*;

/// `value` serializes to exactly `wire`, and `wire` deserializes back to `value`.
#[track_caller]
fn assert_wire<T>(value: &T, wire: Value)
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    assert_eq!(serde_json::to_value(value).unwrap(), wire);
    assert_eq!(&serde_json::from_value::<T>(wire).unwrap(), value);
}

fn user() -> User {
    User {
        id: 7,
        name: "Ada Lovelace".into(),
        role: UserRole::Administrator,
        status: UserStatus::Active,
        bio: None,
        avatar_url: "/users/7/avatar?v=1700000000".into(),
        custom_status: Some(CustomStatus {
            emoji: Some("🌴".into()),
            text: Some("On a beach".into()),
            expires_at: None,
        }),
        created_at: "2026-09-26T12:26:46.848Z".into(),
    }
}

fn message() -> MessageDTO {
    MessageDTO {
        id: 9001,
        room_id: 12,
        thread_id: None,
        creator_id: 7,
        client_message_id: "4f1c7a0e-5b0e-4c55-9d0a-6f3b2d1e8c11".into(),
        body_html: "<p>Hello <strong>there</strong></p>".into(),
        markdown_source: Some("Hello **there**".into()),
        system_note: false,
        action: false,
        streaming: false,
        embeds_suppressed: false,
        reply_to_message_id: Some(8999),
        forwarded_from_message_id: None,
        edited_at: None,
        created_at: "2026-10-06T09:15:00.123Z".into(),
        updated_at: "2026-10-06T09:15:00.123Z".into(),
    }
}

#[test]
fn user_is_camel_case_with_explicit_nulls() {
    assert_wire(
        &user(),
        json!({
            "id": 7,
            "name": "Ada Lovelace",
            "role": "administrator",
            "status": "active",
            "bio": null,
            "avatarUrl": "/users/7/avatar?v=1700000000",
            "customStatus": {"emoji": "🌴", "text": "On a beach", "expiresAt": null},
            "createdAt": "2026-09-26T12:26:46.848Z",
        }),
    );
}

#[test]
fn absent_values_are_sent_as_null_not_omitted() {
    // The generated types say `bio: string | null`, so the key must always be there.
    let wire = serde_json::to_value(user()).unwrap();
    assert_eq!(wire.get("bio"), Some(&Value::Null));
    assert_eq!(wire["customStatus"].get("expiresAt"), Some(&Value::Null));
}

#[test]
fn me_round_trips() {
    let me = Me {
        user: user(),
        email_address: Some("ada@example.com".into()),
        preferences: Preferences {
            theme: Theme::System,
            text_size: TextSize::Default,
            time_zone: Some("America/New_York".into()),
            time_zone_explicit: false,
            tour_completed: true,
            voice_mode: VoiceMode::PushToTalk,
            push_to_talk_key: "`".into(),
        },
        presence_setting: PresenceSetting::Dnd,
        do_not_disturb: DoNotDisturb {
            enabled: true,
            until: Some("2026-10-06T17:00:00.000Z".into()),
        },
        quiet_hours: Some(QuietHours {
            start_minute: 1320,
            end_minute: 420,
        }),
        out_of_office: None,
    };
    let wire = serde_json::to_value(&me).unwrap();
    assert_eq!(wire["emailAddress"], "ada@example.com");
    assert_eq!(
        wire["preferences"],
        json!({
            "theme": "system",
            "textSize": "default",
            "timeZone": "America/New_York",
            "timeZoneExplicit": false,
            "tourCompleted": true,
            "voiceMode": "push_to_talk",
            "pushToTalkKey": "`",
        })
    );
    assert_eq!(wire["presenceSetting"], "dnd");
    assert_eq!(
        wire["doNotDisturb"],
        json!({"enabled": true, "until": "2026-10-06T17:00:00.000Z"})
    );
    assert_eq!(
        wire["quietHours"],
        json!({"startMinute": 1320, "endMinute": 420})
    );
    assert_eq!(wire["outOfOffice"], Value::Null);
    assert_wire(&me, wire);

    let away = OutOfOffice {
        until: "2026-10-10T00:00:00.000Z".into(),
        note: None,
        keep_notifications: true,
    };
    assert_wire(
        &away,
        json!({"until": "2026-10-10T00:00:00.000Z", "note": null, "keepNotifications": true}),
    );
}

#[test]
fn room_and_membership_round_trip() {
    let room = Room {
        id: 12,
        kind: RoomKind::Open,
        name: Some("general".into()),
        icon_name: None,
        creator_id: 7,
        created_at: "2026-01-01T00:00:00.000Z".into(),
        updated_at: "2026-10-06T09:15:00.123Z".into(),
    };
    assert_wire(
        &room,
        json!({
            "id": 12,
            "kind": "open",
            "name": "general",
            "iconName": null,
            "creatorId": 7,
            "createdAt": "2026-01-01T00:00:00.000Z",
            "updatedAt": "2026-10-06T09:15:00.123Z",
        }),
    );
    for (kind, wire) in [
        (RoomKind::Closed, "closed"),
        (RoomKind::Direct, "direct"),
        (RoomKind::Voice, "voice"),
        (RoomKind::Stage, "stage"),
        (RoomKind::Board, "board"),
    ] {
        assert_wire(&kind, json!(wire));
    }

    let membership = Membership {
        id: 40,
        room_id: 12,
        user_id: 7,
        involvement: Involvement::Mentions,
        unread_at: Some("2026-10-06T09:15:00.123Z".into()),
        last_read_message_id: Some(8999),
        room_category_id: None,
        favorite_position: Some(0),
        stage_role: Some(StageRole::Host),
    };
    assert_wire(
        &membership,
        json!({
            "id": 40,
            "roomId": 12,
            "userId": 7,
            "involvement": "mentions",
            "unreadAt": "2026-10-06T09:15:00.123Z",
            "lastReadMessageId": 8999,
            "roomCategoryId": null,
            "favoritePosition": 0,
            "stageRole": "host",
        }),
    );
}

#[test]
fn message_round_trips() {
    assert_wire(
        &message(),
        json!({
            "id": 9001,
            "roomId": 12,
            "threadId": null,
            "creatorId": 7,
            "clientMessageId": "4f1c7a0e-5b0e-4c55-9d0a-6f3b2d1e8c11",
            "bodyHtml": "<p>Hello <strong>there</strong></p>",
            "markdownSource": "Hello **there**",
            "systemNote": false,
            "action": false,
            "streaming": false,
            "embedsSuppressed": false,
            "replyToMessageId": 8999,
            "forwardedFromMessageId": null,
            "editedAt": null,
            "createdAt": "2026-10-06T09:15:00.123Z",
            "updatedAt": "2026-10-06T09:15:00.123Z",
        }),
    );
}

#[test]
fn api_errors_are_tagged_unions_under_error() {
    let cases = [
        (
            ApiError::Unauthorized {
                message: "Sign in".into(),
            },
            json!({"_tag": "Unauthorized", "message": "Sign in"}),
            401,
        ),
        (
            ApiError::Forbidden {
                message: "No".into(),
            },
            json!({"_tag": "Forbidden", "message": "No"}),
            403,
        ),
        (
            ApiError::SudoRequired {
                message: "Confirm your password".into(),
            },
            json!({"_tag": "SudoRequired", "message": "Confirm your password"}),
            403,
        ),
        (
            ApiError::TwoFactorRequired {
                message: "Set up two-factor".into(),
            },
            json!({"_tag": "TwoFactorRequired", "message": "Set up two-factor"}),
            403,
        ),
        (
            ApiError::NotFound {
                message: "Not found".into(),
            },
            json!({"_tag": "NotFound", "message": "Not found"}),
            404,
        ),
        (
            ApiError::Conflict {
                message: "Changed".into(),
            },
            json!({"_tag": "Conflict", "message": "Changed"}),
            409,
        ),
        (
            ApiError::Validation {
                message: "Name can't be blank".into(),
                fields: BTreeMap::from([("name".into(), vec!["can't be blank".into()])]),
            },
            json!({"_tag": "Validation", "message": "Name can't be blank", "fields": {"name": ["can't be blank"]}}),
            422,
        ),
        (
            ApiError::RateLimited {
                message: "Slow down".into(),
                retry_after: 30,
            },
            json!({"_tag": "RateLimited", "message": "Slow down", "retryAfter": 30}),
            429,
        ),
    ];
    for (error, wire, status) in cases {
        assert_eq!(error.status(), status, "{error:?}");
        assert_wire(&ApiErrorResponse { error }, json!({ "error": wire }));
    }
}

#[test]
fn an_unknown_error_tag_is_rejected() {
    let wire = json!({"error": {"_tag": "Teapot", "message": "418"}});
    assert!(serde_json::from_value::<ApiErrorResponse>(wire).is_err());
}

#[test]
fn client_frames_match_the_protocol() {
    let cases = [
        (
            ClientFrame::Hello {
                v: 1,
                resume: Some(ResumePoint {
                    epoch: "b7c1".into(),
                    seq: 48211,
                }),
                topics: vec!["room:12".into(), "thread:88".into()],
            },
            json!({"t": "hello", "v": 1, "resume": {"epoch": "b7c1", "seq": 48211}, "topics": ["room:12", "thread:88"]}),
        ),
        (
            ClientFrame::Hello {
                v: 1,
                resume: None,
                topics: vec![],
            },
            json!({"t": "hello", "v": 1, "resume": null, "topics": []}),
        ),
        (
            ClientFrame::Sub {
                topics: vec!["room:31".into()],
            },
            json!({"t": "sub", "topics": ["room:31"]}),
        ),
        (
            ClientFrame::Unsub {
                topics: vec!["room:12".into()],
            },
            json!({"t": "unsub", "topics": ["room:12"]}),
        ),
        (
            ClientFrame::Typing {
                conv: "room:12".into(),
                on: true,
            },
            json!({"t": "typing", "conv": "room:12", "on": true}),
        ),
        (
            ClientFrame::Present { room: 12 },
            json!({"t": "present", "room": 12}),
        ),
        (
            ClientFrame::Absent { room: 12 },
            json!({"t": "absent", "room": 12}),
        ),
        (ClientFrame::Hb, json!({"t": "hb"})),
    ];
    for (frame, wire) in cases {
        assert_wire(&frame, wire);
    }
}

#[test]
fn server_frames_match_the_protocol() {
    assert_wire(
        &ServerFrame::Welcome {
            epoch: "b7c1".into(),
            seq: 48230,
            resumed: true,
        },
        json!({"t": "welcome", "epoch": "b7c1", "seq": 48230, "resumed": true}),
    );
    assert_wire(
        &ServerFrame::Resync {
            topics: vec!["room:12".into()],
            reason: "ring_rolled_over".into(),
        },
        json!({"t": "resync", "topics": ["room:12"], "reason": "ring_rolled_over"}),
    );
    assert_wire(
        &ServerFrame::Bye {
            reconnect: true,
            reason: "server_restart".into(),
        },
        json!({"t": "bye", "reconnect": true, "reason": "server_restart"}),
    );

    let batch = ServerFrame::Batch {
        events: vec![
            SyncEvent {
                seq: 48212,
                topic: "room:12".into(),
                payload: SyncPayload::MessageCreated(message()),
            },
            SyncEvent {
                seq: 48213,
                topic: "room:12".into(),
                payload: SyncPayload::MessageRemoved(MessageRemoved {
                    id: 9000,
                    room_id: 12,
                    thread_id: None,
                }),
            },
            SyncEvent {
                seq: 48214,
                topic: "room:12".into(),
                payload: SyncPayload::Typing(Typing {
                    user_id: 7,
                    on: false,
                }),
            },
        ],
    };
    let wire = serde_json::to_value(&batch).unwrap();
    assert_eq!(wire["t"], "batch");
    assert_eq!(wire["events"][0]["seq"], 48212);
    assert_eq!(wire["events"][0]["topic"], "room:12");
    assert_eq!(wire["events"][0]["type"], "message.created");
    assert_eq!(
        wire["events"][0]["data"],
        serde_json::to_value(message()).unwrap()
    );
    assert_eq!(
        wire["events"][1],
        json!({"seq": 48213, "topic": "room:12", "type": "message.removed", "data": {"id": 9000, "roomId": 12, "threadId": null}})
    );
    assert_eq!(
        wire["events"][2],
        json!({"seq": 48214, "topic": "room:12", "type": "typing", "data": {"userId": 7, "on": false}})
    );
    assert_wire(&batch, wire);

    let updated = SyncEvent {
        seq: 1,
        topic: "room:12".into(),
        payload: SyncPayload::MessageUpdated(message()),
    };
    assert_eq!(
        serde_json::to_value(&updated).unwrap()["type"],
        "message.updated"
    );
}
