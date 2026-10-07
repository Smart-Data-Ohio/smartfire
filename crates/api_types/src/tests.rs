use std::collections::BTreeMap;
use std::fmt::Debug;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::*;

/// `value` serializes to exactly `wire`, and `wire` deserializes back to `value`.
#[track_caller]
pub(crate) fn assert_wire<T>(value: &T, wire: Value)
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    assert_eq!(serde_json::to_value(value).unwrap(), wire);
    assert_eq!(&serde_json::from_value::<T>(wire).unwrap(), value);
}

pub(crate) fn user() -> User {
    User {
        id: 7,
        name: "Ada Lovelace".into(),
        role: UserRole::Administrator,
        status: UserStatus::Active,
        bio: None,
        avatar_url: "/users/7/avatar?v=1700000000".into(),
        has_avatar: true,
        custom_status: Some(CustomStatus {
            emoji: Some("🌴".into()),
            text: Some("On a beach".into()),
            expires_at: None,
        }),
        avatar_icon: None,
        agent: None,
        created_at: "2026-09-26T12:26:46.848Z".into(),
    }
}

pub(crate) fn message() -> MessageDTO {
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
        forwarded_at: None,
        forward_note: None,
        edited_at: None,
        attachment: None,
        reactions: vec![],
        boosts: vec![],
        pinned: false,
        thread: None,
        poll: None,
        cards: vec![],
        cards_as_of: "2026-10-06T09:15:00.200Z".into(),
        steps: vec![],
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
            "hasAvatar": true,
            "customStatus": {"emoji": "🌴", "text": "On a beach", "expiresAt": null},
            "avatarIcon": null,
            "agent": null,
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
        last_room_id: Some(12),
    };
    let wire = serde_json::to_value(&me).unwrap();
    assert_eq!(wire["lastRoomId"], 12);
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
            "forwardedAt": null,
            "forwardNote": null,
            "editedAt": null,
            "attachment": null,
            "reactions": [],
            "boosts": [],
            "pinned": false,
            "thread": null,
            "poll": null,
            "cards": [],
            "cardsAsOf": "2026-10-06T09:15:00.200Z",
            "steps": [],
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
            ApiError::InvalidAuthenticityToken {
                message: "Can't verify CSRF token authenticity".into(),
            },
            json!({"_tag": "InvalidAuthenticityToken", "message": "Can't verify CSRF token authenticity"}),
            422,
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
        (
            ClientFrame::Hb { active: true },
            json!({"t": "hb", "active": true}),
        ),
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
    assert_wire(&ServerFrame::Ping, json!({"t": "ping"}));

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

fn room() -> Room {
    Room {
        id: 12,
        kind: RoomKind::Open,
        name: Some("general".into()),
        icon_name: None,
        creator_id: 7,
        created_at: "2026-01-01T00:00:00.000Z".into(),
        updated_at: "2026-10-06T09:15:00.123Z".into(),
    }
}

fn membership() -> Membership {
    Membership {
        id: 40,
        room_id: 12,
        user_id: 7,
        involvement: Involvement::Everything,
        unread_at: Some("2026-10-06T09:15:00.123Z".into()),
        last_read_message_id: Some(8999),
        room_category_id: Some(3),
        favorite_position: None,
        stage_role: None,
    }
}

pub(crate) fn row() -> SidebarRow {
    SidebarRow {
        room: room(),
        membership: membership(),
        display_name: "general".into(),
        direct_member_ids: vec![],
        unread_count: 4,
        mention_count: 1,
    }
}

#[test]
fn room_detail_round_trips() {
    let detail = RoomDetail {
        room: room(),
        membership: membership(),
        display_name: "general".into(),
        member_count: 23,
        pins_count: 2,
        direct_member_ids: vec![],
        member_preview_ids: vec![7],
        users: vec![user()],
        unread: Some(UnreadDivider {
            first_unread_message_id: 9000,
            count: 4,
        }),
    };
    let wire = serde_json::to_value(&detail).unwrap();
    assert_eq!(wire["room"], serde_json::to_value(room()).unwrap());
    assert_eq!(wire["membership"]["roomCategoryId"], 3);
    assert_eq!(wire["displayName"], "general");
    assert_eq!(wire["memberCount"], 23);
    assert_eq!(wire["pinsCount"], 2);
    assert_eq!(wire["directMemberIds"], json!([]));
    assert_eq!(wire["memberPreviewIds"], json!([7]));
    assert_eq!(wire["users"][0]["name"], "Ada Lovelace");
    assert_eq!(
        wire["unread"],
        json!({"firstUnreadMessageId": 9000, "count": 4})
    );
    assert_wire(&detail, wire);

    let read = RoomDetail {
        unread: None,
        ..detail
    };
    assert_eq!(serde_json::to_value(&read).unwrap()["unread"], Value::Null);
}

#[test]
fn message_page_carries_its_authors_and_cursors() {
    let page = MessagePage {
        messages: vec![message()],
        users: vec![user()],
        before: Some(9001),
        after: None,
        saved: vec![SavedMark {
            message_id: 9001,
            saved_item_id: 31,
        }],
    };
    let wire = serde_json::to_value(&page).unwrap();
    assert_eq!(
        wire["messages"][0],
        serde_json::to_value(message()).unwrap()
    );
    assert_eq!(wire["users"][0]["id"], 7);
    assert_eq!(wire["before"], 9001);
    assert_eq!(wire["after"], Value::Null);
    assert_eq!(
        wire["saved"],
        json!([{"messageId": 9001, "savedItemId": 31}])
    );
    assert_wire(&page, wire);
}

#[test]
fn a_message_read_names_its_conversation() {
    let read = MessageRead {
        message: message(),
        users: vec![user()],
        conversation: ConversationName {
            room_id: 12,
            thread_id: None,
            room_kind: RoomKind::Open,
            room_name: "general".into(),
            room_icon_name: None,
            thread_name: None,
        },
        saved: None,
    };
    let wire = serde_json::to_value(&read).unwrap();
    assert_eq!(wire["message"], serde_json::to_value(message()).unwrap());
    assert_eq!(
        wire["conversation"],
        json!({"roomId": 12, "threadId": null, "roomKind": "open", "roomName": "general",
               "roomIconName": null, "threadName": null})
    );
    assert_eq!(wire["saved"], Value::Null);
    assert_wire(&read, wire);
}

#[test]
fn create_message_requests_round_trip() {
    assert_wire(
        &CreateMessage {
            client_message_id: "0192f0c4-7e8a-7b3c-9d0a-6f3b2d1e8c11".into(),
            markdown_source: "Ship it :rocket:".into(),
            reply_to_message_id: None,
            reply_notify_author: None,
            attachment_signed_id: None,
        },
        json!({
            "clientMessageId": "0192f0c4-7e8a-7b3c-9d0a-6f3b2d1e8c11",
            "markdownSource": "Ship it :rocket:",
            "replyToMessageId": null,
            "replyNotifyAuthor": null,
            "attachmentSignedId": null,
        }),
    );
}

#[test]
fn sidebar_round_trips() {
    let sidebar = Sidebar {
        rows: vec![row()],
        categories: vec![RoomCategory {
            id: 3,
            name: "Projects".into(),
            collapsed: false,
            position: 0,
        }],
        users: vec![user()],
        direct_placeholder_user_ids: vec![7],
        can_create_rooms: true,
    };
    let wire = serde_json::to_value(&sidebar).unwrap();
    assert_eq!(
        wire["rows"][0],
        json!({
            "room": serde_json::to_value(room()).unwrap(),
            "membership": serde_json::to_value(membership()).unwrap(),
            "displayName": "general",
            "directMemberIds": [],
            "unreadCount": 4,
            "mentionCount": 1,
        })
    );
    assert_eq!(
        wire["categories"],
        json!([{"id": 3, "name": "Projects", "collapsed": false, "position": 0}])
    );
    assert_eq!(wire["directPlaceholderUserIds"], json!([7]));
    assert_eq!(wire["canCreateRooms"], true);
    assert_wire(&sidebar, wire);
}

#[test]
fn read_state_round_trips() {
    assert_wire(
        &ReadState {
            room_id: 12,
            unread: false,
            first_unread_message_id: None,
            unread_count: 0,
        },
        json!({"roomId": 12, "unread": false, "firstUnreadMessageId": null, "unreadCount": 0}),
    );
    assert_wire(
        &ReadState {
            room_id: 12,
            unread: true,
            first_unread_message_id: Some(9000),
            unread_count: 3,
        },
        json!({"roomId": 12, "unread": true, "firstUnreadMessageId": 9000, "unreadCount": 3}),
    );
    assert_wire(&MarkUnread { message_id: 9000 }, json!({"messageId": 9000}));
}

#[test]
fn users_and_presence_round_trip() {
    let list = UserList {
        users: vec![user()],
    };
    assert_wire(
        &list,
        json!({"users": [serde_json::to_value(user()).unwrap()]}),
    );
    for (presence, wire) in [
        (Presence::Online, "online"),
        (Presence::Idle, "idle"),
        (Presence::Offline, "offline"),
        (Presence::Dnd, "dnd"),
    ] {
        assert_wire(&presence, json!(wire));
    }
    assert_wire(
        &PresenceList {
            presences: vec![UserPresence {
                user_id: 7,
                presence: Presence::Idle,
                status_text: Some("In a meeting".into()),
            }],
        },
        json!({"presences": [{"userId": 7, "presence": "idle", "statusText": "In a meeting"}]}),
    );
}

#[test]
fn s1_events_match_the_protocol() {
    let cases = [
        (
            SyncPayload::RoomUnread(RoomUnread {
                room_id: 12,
                message_id: Some(9001),
                mentioned: true,
            }),
            json!({"type": "room.unread", "data": {"roomId": 12, "messageId": 9001, "mentioned": true}}),
        ),
        (
            SyncPayload::RoomUnread(RoomUnread {
                room_id: 12,
                message_id: None,
                mentioned: false,
            }),
            json!({"type": "room.unread", "data": {"roomId": 12, "messageId": null, "mentioned": false}}),
        ),
        (
            SyncPayload::RoomRead(RoomRead { room_id: 12 }),
            json!({"type": "room.read", "data": {"roomId": 12}}),
        ),
        (
            SyncPayload::SidebarRowUpserted(row()),
            json!({"type": "sidebar.row.upserted", "data": serde_json::to_value(row()).unwrap()}),
        ),
        (
            SyncPayload::SidebarRowRemoved(SidebarRowRemoved { room_id: 12 }),
            json!({"type": "sidebar.row.removed", "data": {"roomId": 12}}),
        ),
        (
            SyncPayload::Presence(UserPresence {
                user_id: 7,
                presence: Presence::Online,
                status_text: None,
            }),
            json!({"type": "presence", "data": {"userId": 7, "presence": "online", "statusText": null}}),
        ),
    ];
    for (payload, wire) in cases {
        let event = SyncEvent {
            seq: 5,
            topic: "user".into(),
            payload,
        };
        let mut expected = wire;
        expected["seq"] = json!(5);
        expected["topic"] = json!("user");
        assert_wire(&event, expected);
    }
}
