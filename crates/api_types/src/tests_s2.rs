//! Wire shapes of the S2 types: message actions, composer, threads, panes, direct messages and
//! the switcher.

use serde_json::json;

use crate::tests::{assert_wire, message, user};
use crate::*;

#[test]
fn a_message_carries_its_play_sound() {
    let mut sound_message = message();
    sound_message.sound = Some(MessageSound {
        name: "bell".into(),
        url: "/assets/bell-digest.mp3".into(),
        presentation: SoundPresentation::Text {
            text: "🔔".into()
        },
    });
    let wire = serde_json::to_value(&sound_message).unwrap();
    assert_eq!(
        wire["sound"],
        json!({
            "name": "bell", "url": "/assets/bell-digest.mp3",
            "presentation": { "kind": "text", "text": "🔔" },
        })
    );
    assert_eq!(
        serde_json::from_value::<MessageDTO>(wire).unwrap(),
        sound_message
    );
    assert_wire(
        &SoundPresentation::Image {
            url: "/assets/56k.webp".into(),
            width: 79,
            height: 33,
        },
        json!({ "kind": "image", "url": "/assets/56k.webp", "width": 79, "height": 33 }),
    );
}

fn attachment() -> Attachment {
    Attachment {
        filename: "roadmap.png".into(),
        content_type: "image/png".into(),
        byte_size: 48_213,
        width: Some(1600),
        height: Some(900),
        preview: AttachmentPreview::Image,
        url: "/rails/active_storage/blobs/redirect/eyJf--1/roadmap.png".into(),
        download_url:
            "/rails/active_storage/blobs/redirect/eyJf--1/roadmap.png?disposition=attachment".into(),
        thumbnail_url: Some(
            "/rails/active_storage/representations/redirect/eyJf--1/eyJr--2/roadmap.png".into(),
        ),
    }
}

fn reaction() -> Reaction {
    Reaction {
        content: "🎉".into(),
        title: "Party popper".into(),
        image_url: None,
        reactor_ids: vec![7, 8],
    }
}

fn boost() -> Boost {
    Boost {
        id: 55,
        booster_id: 8,
        content: "nice work".into(),
        created_at: "2026-10-06T09:16:00.000Z".into(),
    }
}

fn thread() -> Thread {
    Thread {
        id: 88,
        room_id: 12,
        parent_message_id: Some(9001),
        creator_id: 7,
        name: "Hello there".into(),
        status: ThreadStatus::Active,
        reply_count: 3,
        last_activity_at: "2026-10-06T10:00:00.000Z".into(),
        auto_archive_after_minutes: 4320,
        created_at: "2026-10-06T09:20:00.000Z".into(),
        work: None,
    }
}

fn thread_membership() -> ThreadMembership {
    ThreadMembership {
        thread_id: 88,
        involvement: ThreadInvolvement::Everything,
        unread_at: Some("2026-10-06T10:00:00.000Z".into()),
        joined_at: "2026-10-06T09:20:00.000Z".into(),
    }
}

fn indicator() -> ThreadIndicator {
    ThreadIndicator {
        thread_id: 88,
        reply_count: 3,
        last_reply_at: "2026-10-06T10:00:00.000Z".into(),
        replier_ids: vec![8, 7],
    }
}

#[test]
fn a_message_carries_its_attachment_reactions_boosts_pin_and_thread() {
    let mut full = message();
    full.forwarded_at = Some("2026-10-06T09:15:30.000Z".into());
    full.forward_note = Some("FYI".into());
    full.attachment = Some(attachment());
    full.reactions = vec![reaction()];
    full.boosts = vec![boost()];
    full.pinned = true;
    full.thread = Some(indicator());

    let wire = serde_json::to_value(&full).unwrap();

    assert_eq!(wire["forwardedAt"], "2026-10-06T09:15:30.000Z");
    assert_eq!(wire["forwardNote"], "FYI");
    assert_eq!(
        wire["attachment"],
        json!({
            "filename": "roadmap.png",
            "contentType": "image/png",
            "byteSize": 48213,
            "width": 1600,
            "height": 900,
            "preview": "image",
            "url": "/rails/active_storage/blobs/redirect/eyJf--1/roadmap.png",
            "downloadUrl": "/rails/active_storage/blobs/redirect/eyJf--1/roadmap.png?disposition=attachment",
            "thumbnailUrl": "/rails/active_storage/representations/redirect/eyJf--1/eyJr--2/roadmap.png",
        })
    );
    assert_eq!(
        wire["reactions"],
        json!([{"content": "🎉", "title": "Party popper", "imageUrl": null, "reactorIds": [7, 8]}])
    );
    assert_eq!(
        wire["boosts"],
        json!([{"id": 55, "boosterId": 8, "content": "nice work", "createdAt": "2026-10-06T09:16:00.000Z"}])
    );
    assert_eq!(wire["pinned"], true);
    assert_eq!(
        wire["thread"],
        json!({"threadId": 88, "replyCount": 3, "lastReplyAt": "2026-10-06T10:00:00.000Z", "replierIds": [8, 7]})
    );
    assert_wire(&full, wire);
}

#[test]
fn attachment_previews_are_lowercase() {
    for (preview, wire) in [
        (AttachmentPreview::Image, "image"),
        (AttachmentPreview::Video, "video"),
        (AttachmentPreview::File, "file"),
    ] {
        assert_wire(&preview, json!(wire));
    }
}

#[test]
fn message_edits_and_sources_round_trip() {
    assert_wire(
        &UpdateMessage {
            markdown_source: "Hello **again**".into(),
        },
        json!({"markdownSource": "Hello **again**"}),
    );
    assert_wire(
        &MessageSource {
            message_id: 9001,
            markdown_source: "Hello **there**".into(),
        },
        json!({"messageId": 9001, "markdownSource": "Hello **there**"}),
    );
}

#[test]
fn uploads_round_trip() {
    assert_wire(
        &CreateUpload {
            filename: "roadmap.png".into(),
            byte_size: 48_213,
            checksum: "1B2M2Y8AsgTpgAmY7PhCfg==".into(),
            content_type: "image/png".into(),
        },
        json!({
            "filename": "roadmap.png",
            "byteSize": 48213,
            "checksum": "1B2M2Y8AsgTpgAmY7PhCfg==",
            "contentType": "image/png",
        }),
    );
    assert_wire(
        &DirectUpload {
            signed_id: "eyJfcmFpbHMiOnt9--abc".into(),
            upload_url: "/rails/active_storage/disk/eyJ0b2tlbiI6MX0".into(),
        },
        json!({
            "signedId": "eyJfcmFpbHMiOnt9--abc",
            "uploadUrl": "/rails/active_storage/disk/eyJ0b2tlbiI6MX0",
        }),
    );
    assert_wire(
        &CreateMessage {
            client_message_id: "0192f0c4-7e8a-7b3c-9d0a-6f3b2d1e8c11".into(),
            markdown_source: String::new(),
            reply_to_message_id: None,
            reply_notify_author: None,
            attachment_signed_id: Some("eyJfcmFpbHMiOnt9--abc".into()),
        },
        json!({
            "clientMessageId": "0192f0c4-7e8a-7b3c-9d0a-6f3b2d1e8c11",
            "markdownSource": "",
            "replyToMessageId": null,
            "replyNotifyAuthor": null,
            "attachmentSignedId": "eyJfcmFpbHMiOnt9--abc",
        }),
    );
}

#[test]
fn reactions_round_trip() {
    assert_wire(
        &CreateBoost {
            content: ":tada:".into(),
        },
        json!({"content": ":tada:"}),
    );
    assert_wire(
        &MessageReactions {
            message_id: 9001,
            room_id: 12,
            thread_id: None,
            reactions: vec![Reaction {
                content: ":github:".into(),
                title: "GitHub".into(),
                image_url: Some("/assets/icons/brands/github.svg".into()),
                reactor_ids: vec![7],
            }],
            boosts: vec![boost()],
            updated_at: "2026-10-06T09:16:00.000Z".into(),
        },
        json!({
            "messageId": 9001,
            "roomId": 12,
            "threadId": null,
            "reactions": [{
                "content": ":github:",
                "title": "GitHub",
                "imageUrl": "/assets/icons/brands/github.svg",
                "reactorIds": [7],
            }],
            "boosts": [{"id": 55, "boosterId": 8, "content": "nice work", "createdAt": "2026-10-06T09:16:00.000Z"}],
            "updatedAt": "2026-10-06T09:16:00.000Z",
        }),
    );
}

#[test]
fn pins_round_trip() {
    let state = PinState {
        message_id: 9001,
        room_id: 12,
        pinned: true,
        pin_count: 4,
    };
    assert_wire(
        &state,
        json!({"messageId": 9001, "roomId": 12, "pinned": true, "pinCount": 4}),
    );
    let mut pinned = message();
    pinned.pinned = true;
    let list = PinList {
        pins: vec![Pin {
            message_id: 9001,
            pinner_id: 8,
            pinned_at: "2026-10-06T11:00:00.000Z".into(),
        }],
        messages: vec![pinned.clone()],
        users: vec![user()],
    };
    let wire = serde_json::to_value(&list).unwrap();
    assert_eq!(
        wire["pins"],
        json!([{"messageId": 9001, "pinnerId": 8, "pinnedAt": "2026-10-06T11:00:00.000Z"}])
    );
    assert_eq!(wire["messages"][0], serde_json::to_value(pinned).unwrap());
    assert_wire(&list, wire);
}

#[test]
fn saved_items_round_trip() {
    assert_wire(&SavedStatus::InProgress, json!("in_progress"));
    assert_wire(&SavedStatus::Done, json!("done"));
    assert_wire(
        &SaveMessage {
            message_id: 9001,
            remind_at: None,
        },
        json!({"messageId": 9001, "remindAt": null}),
    );
    assert_wire(
        &SavedItem {
            id: 31,
            message_id: 9001,
            status: SavedStatus::InProgress,
            remind_at: Some("2026-10-07T09:00:00.000Z".into()),
            reminded_at: None,
            created_at: "2026-10-06T11:00:00.000Z".into(),
        },
        json!({
            "id": 31,
            "messageId": 9001,
            "status": "in_progress",
            "remindAt": "2026-10-07T09:00:00.000Z",
            "remindedAt": null,
            "createdAt": "2026-10-06T11:00:00.000Z",
        }),
    );
}

#[test]
fn forwards_round_trip() {
    assert_wire(
        &ForwardDestinationList {
            destinations: vec![ForwardDestination {
                room_id: 3,
                name: "engineering".into(),
                direct: false,
                threads: vec![ForwardThread {
                    id: 88,
                    name: "Hello there".into(),
                    status: ThreadStatus::Active,
                }],
            }],
        },
        json!({"destinations": [{
            "roomId": 3,
            "name": "engineering",
            "direct": false,
            "threads": [{"id": 88, "name": "Hello there", "status": "active"}],
        }]}),
    );
    assert_wire(
        &CreateForwards {
            note: Some("FYI".into()),
            destinations: vec![
                ForwardTarget {
                    room_id: 3,
                    thread_id: None,
                },
                ForwardTarget {
                    room_id: 4,
                    thread_id: Some(88),
                },
            ],
        },
        json!({
            "note": "FYI",
            "destinations": [{"roomId": 3, "threadId": null}, {"roomId": 4, "threadId": 88}],
        }),
    );
    let result = ForwardResult {
        forwards: vec![message()],
    };
    assert_wire(
        &result,
        json!({"forwards": [serde_json::to_value(message()).unwrap()]}),
    );
}

#[test]
fn autocomplete_round_trips() {
    let wire = serde_json::to_value(UserSuggestionList {
        suggestions: vec![UserSuggestion {
            user: user(),
            mention_token: Some("@[Ada Lovelace]".into()),
        }],
    })
    .unwrap();
    assert_eq!(wire["suggestions"][0]["mentionToken"], "@[Ada Lovelace]");
    assert_eq!(
        wire["suggestions"][0]["user"],
        serde_json::to_value(user()).unwrap()
    );

    assert_wire(
        &IconList {
            icons: vec![
                Icon {
                    name: "tada".into(),
                    title: "Tada".into(),
                    kind: IconKind::Emoji,
                    character: Some("🎉".into()),
                    image_url: None,
                },
                Icon {
                    name: "shipit".into(),
                    title: "Ship it".into(),
                    kind: IconKind::Custom,
                    character: None,
                    image_url: Some("/icons/shipit".into()),
                },
            ],
        },
        json!({"icons": [
            {"name": "tada", "title": "Tada", "kind": "emoji", "character": "🎉", "imageUrl": null},
            {"name": "shipit", "title": "Ship it", "kind": "custom", "character": null, "imageUrl": "/icons/shipit"},
        ]}),
    );
    assert_wire(&IconKind::Brand, json!("brand"));
}

#[test]
fn slash_commands_round_trip() {
    assert_wire(
        &SlashCommandList {
            commands: vec![SlashCommand {
                name: "remind".into(),
                description: "Post and remind yourself about it later".into(),
                arg_hint: "<when> <text>".into(),
                takes_arguments: true,
                agent_name: None,
            }],
        },
        json!({"commands": [{
            "name": "remind",
            "description": "Post and remind yourself about it later",
            "argHint": "<when> <text>",
            "takesArguments": true,
            "agentName": null,
        }]}),
    );
    assert_wire(
        &RunSlashCommand {
            text: "/shrug fine".into(),
            thread_id: None,
        },
        json!({"text": "/shrug fine", "threadId": null}),
    );
    let cases = [
        (
            SlashCommandResult::Posted {
                message_id: 9002,
                notice: Some("Reminder set for tomorrow at 9:00 AM".into()),
            },
            json!({"status": "posted", "messageId": 9002, "notice": "Reminder set for tomorrow at 9:00 AM"}),
        ),
        (
            SlashCommandResult::Ephemeral {
                message: "Status set".into(),
            },
            json!({"status": "ephemeral", "message": "Status set"}),
        ),
        (
            SlashCommandResult::Error {
                message: "Unknown command".into(),
            },
            json!({"status": "error", "message": "Unknown command"}),
        ),
        (
            SlashCommandResult::OpenUrl {
                url: "/rooms/12/events/new?title=Standup".into(),
            },
            json!({"status": "open_url", "url": "/rooms/12/events/new?title=Standup"}),
        ),
        (SlashCommandResult::OpenPoll, json!({"status": "open_poll"})),
        (
            SlashCommandResult::StartHuddle {
                room_id: 12,
                room_name: "general".into(),
            },
            json!({"status": "start_huddle", "roomId": 12, "roomName": "general"}),
        ),
    ];
    for (result, wire) in cases {
        assert_wire(&result, wire);
    }
}

#[test]
fn preview_and_scheduled_messages_round_trip() {
    assert_wire(
        &PreviewMessage {
            markdown_source: "**hi**".into(),
        },
        json!({"markdownSource": "**hi**"}),
    );
    assert_wire(
        &MessagePreview {
            body_html: "<p><strong>hi</strong></p>".into(),
        },
        json!({"bodyHtml": "<p><strong>hi</strong></p>"}),
    );
    assert_wire(
        &CreateScheduledMessage {
            markdown_source: "Standup in 5".into(),
            send_at: "2026-10-07T13:55:00.000Z".into(),
            thread_id: None,
            reply_to_message_id: None,
        },
        json!({
            "markdownSource": "Standup in 5",
            "sendAt": "2026-10-07T13:55:00.000Z",
            "threadId": null,
            "replyToMessageId": null,
        }),
    );
    assert_wire(
        &UpdateScheduledMessage {
            markdown_source: Some("Standup in 10".into()),
            send_at: None,
        },
        json!({"markdownSource": "Standup in 10"}),
    );
    assert_wire(
        &ScheduledMessageList {
            scheduled_messages: vec![ScheduledMessage {
                id: 4,
                room_id: 12,
                thread_id: Some(88),
                reply_to_message_id: None,
                markdown_source: "Standup in 5".into(),
                send_at: "2026-10-07T13:55:00.000Z".into(),
                state: ScheduledMessageState::Pending,
                sendable: true,
                sent_at: None,
                sent_message_id: None,
                dropped_at: None,
                drop_reason: None,
                created_at: "2026-10-06T11:00:00.000Z".into(),
            }],
            conversations: vec![],
            next_cursor: None,
        },
        json!({"scheduledMessages": [{
            "id": 4,
            "roomId": 12,
            "threadId": 88,
            "replyToMessageId": null,
            "markdownSource": "Standup in 5",
            "sendAt": "2026-10-07T13:55:00.000Z",
            "state": "pending",
            "sendable": true,
            "sentAt": null,
            "sentMessageId": null,
            "droppedAt": null,
            "dropReason": null,
            "createdAt": "2026-10-06T11:00:00.000Z",
        }], "conversations": [], "nextCursor": null}),
    );
}

#[test]
fn threads_round_trip() {
    let thread_wire = json!({
        "id": 88,
        "roomId": 12,
        "parentMessageId": 9001,
        "creatorId": 7,
        "name": "Hello there",
        "status": "active",
        "replyCount": 3,
        "lastActivityAt": "2026-10-06T10:00:00.000Z",
        "autoArchiveAfterMinutes": 4320,
        "createdAt": "2026-10-06T09:20:00.000Z",
        "work": null,
    });
    let membership_wire = json!({
        "threadId": 88,
        "involvement": "everything",
        "unreadAt": "2026-10-06T10:00:00.000Z",
        "joinedAt": "2026-10-06T09:20:00.000Z",
    });
    assert_wire(&thread(), thread_wire.clone());
    assert_wire(&thread_membership(), membership_wire.clone());
    assert_wire(
        &ThreadList {
            threads: vec![ThreadSummary {
                thread: thread(),
                membership: None,
            }],
            users: vec![],
        },
        json!({"threads": [{"thread": thread_wire, "membership": null}], "users": []}),
    );

    let detail = ThreadDetail {
        thread: thread(),
        membership: Some(thread_membership()),
        parent_message: Some(message()),
        permissions: ThreadPermissions {
            can_rename: true,
            can_close: true,
            can_reopen: false,
            can_lock: false,
            can_unlock: false,
            can_delete: false,
            can_convert_work: true,
            can_manage_work: false,
            can_update_work_status: false,
            can_assign_work: false,
            can_remove_work: false,
        },
        work: None,
        users: vec![user()],
    };
    let wire = serde_json::to_value(&detail).unwrap();
    assert_eq!(wire["membership"], membership_wire);
    assert_eq!(
        wire["permissions"],
        json!({
            "canRename": true,
            "canClose": true,
            "canReopen": false,
            "canLock": false,
            "canUnlock": false,
            "canDelete": false,
            "canConvertWork": true,
            "canManageWork": false,
            "canUpdateWorkStatus": false,
            "canAssignWork": false,
            "canRemoveWork": false,
        })
    );
    assert_wire(&detail, wire.clone());

    let created = ThreadCreated {
        detail: detail.clone(),
        message: message(),
    };
    assert_wire(
        &created,
        json!({"detail": wire, "message": serde_json::to_value(message()).unwrap()}),
    );

    assert_wire(
        &CreateThread {
            parent_message_id: 9001,
            name: None,
            message: CreateMessage {
                client_message_id: "0192f0c4-7e8a-7b3c-9d0a-6f3b2d1e8c11".into(),
                markdown_source: "First!".into(),
                reply_to_message_id: None,
                reply_notify_author: None,
                attachment_signed_id: None,
            },
        },
        json!({
            "parentMessageId": 9001,
            "name": null,
            "message": {
                "clientMessageId": "0192f0c4-7e8a-7b3c-9d0a-6f3b2d1e8c11",
                "markdownSource": "First!",
                "replyToMessageId": null,
                "replyNotifyAuthor": null,
                "attachmentSignedId": null,
            },
        }),
    );
    assert_wire(
        &UpdateThread {
            name: None,
            status: Some(ThreadStatus::Locked),
        },
        json!({"name": null, "status": "locked"}),
    );
    assert_wire(
        &JoinThread {
            involvement: Some(ThreadInvolvement::Mentions),
        },
        json!({"involvement": "mentions"}),
    );
    assert_wire(
        &ThreadMembershipState {
            membership: thread_membership(),
        },
        json!({"membership": membership_wire}),
    );
    for (filter, wire) in [
        (ThreadFilter::Active, "active"),
        (ThreadFilter::Closed, "closed"),
        (ThreadFilter::Locked, "locked"),
        (ThreadFilter::All, "all"),
    ] {
        assert_wire(&filter, json!(wire));
    }
    assert_wire(&ThreadInvolvement::Nothing, json!("nothing"));
}

#[test]
fn members_and_files_round_trip() {
    assert_wire(
        &MemberList {
            members: vec![Member {
                user_id: 7,
                presence: Presence::Idle,
                status_text: Some("🌴 On a beach".into()),
                starred: true,
            }],
            users: vec![user()],
        },
        json!({
            "members": [{"userId": 7, "presence": "idle", "statusText": "🌴 On a beach", "starred": true}],
            "users": [serde_json::to_value(user()).unwrap()],
        }),
    );
    assert_wire(
        &StarState {
            user_id: 8,
            starred: false,
        },
        json!({"userId": 8, "starred": false}),
    );
    for (kind, wire) in [
        (FileType::All, "all"),
        (FileType::Images, "images"),
        (FileType::Videos, "videos"),
        (FileType::Documents, "documents"),
        (FileType::Other, "other"),
    ] {
        assert_wire(&kind, json!(wire));
    }
    let files = FileList {
        files: vec![RoomFile {
            message_id: 9001,
            thread_id: None,
            creator_id: 7,
            attachment: attachment(),
            created_at: "2026-10-06T09:15:00.123Z".into(),
        }],
        users: vec![],
        next_page: Some(2),
    };
    let wire = serde_json::to_value(&files).unwrap();
    assert_eq!(wire["nextPage"], 2);
    assert_eq!(wire["files"][0]["messageId"], 9001);
    assert_eq!(
        wire["files"][0]["attachment"],
        serde_json::to_value(attachment()).unwrap()
    );
    assert_wire(&files, wire);
}

#[test]
fn direct_messages_round_trip() {
    assert_wire(
        &DirectCandidateList {
            candidates: vec![DirectCandidate {
                user_id: 9,
                agent: true,
                starred: false,
            }],
            users: vec![],
        },
        json!({"candidates": [{"userId": 9, "agent": true, "starred": false}], "users": []}),
    );
    assert_wire(
        &CreateDirect {
            user_ids: vec![8, 9],
        },
        json!({"userIds": [8, 9]}),
    );
    assert_wire(
        &AddDirectMembers { user_ids: vec![10] },
        json!({"userIds": [10]}),
    );
    assert_wire(
        &RenameDirect {
            name: Some("Launch crew".into()),
        },
        json!({"name": "Launch crew"}),
    );
}

#[test]
fn the_switcher_round_trips() {
    assert_wire(
        &Switcher {
            rooms: vec![SwitcherRoom {
                room_id: 11,
                name: "Maya Chen and Jonah Park".into(),
                kind: SwitcherRoomKind::Group,
                icon_name: None,
                unread: true,
                muted: false,
                favorite: false,
            }],
            people: vec![SwitcherPerson {
                user_id: 8,
                direct_room_id: Some(9),
            }],
            threads: vec![SwitcherThread {
                thread_id: 88,
                name: "Hello there".into(),
                room_id: 12,
                room_name: Some("general".into()),
            }],
            users: vec![],
        },
        json!({
            "rooms": [{
                "roomId": 11,
                "name": "Maya Chen and Jonah Park",
                "kind": "group",
                "iconName": null,
                "unread": true,
                "muted": false,
                "favorite": false,
            }],
            "people": [{"userId": 8, "directRoomId": 9}],
            "threads": [{"threadId": 88, "name": "Hello there", "roomId": 12, "roomName": "general"}],
            "users": [],
        }),
    );
    for (kind, wire) in [
        (SwitcherRoomKind::Channel, "channel"),
        (SwitcherRoomKind::Dm, "dm"),
        (SwitcherRoomKind::Voice, "voice"),
        (SwitcherRoomKind::Stage, "stage"),
        (SwitcherRoomKind::Board, "board"),
    ] {
        assert_wire(&kind, json!(wire));
    }
}

#[test]
fn s2_events_match_the_protocol() {
    let cases = [
        (
            "room:12",
            SyncPayload::MessageReactions(MessageReactions {
                message_id: 9001,
                room_id: 12,
                thread_id: None,
                reactions: vec![reaction()],
                boosts: vec![],
                updated_at: "2026-10-06T09:16:00.000Z".into(),
            }),
            json!({"type": "message.reactions", "data": {
                "messageId": 9001,
                "roomId": 12,
                "threadId": null,
                "reactions": [{"content": "🎉", "title": "Party popper", "imageUrl": null, "reactorIds": [7, 8]}],
                "boosts": [],
                "updatedAt": "2026-10-06T09:16:00.000Z",
            }}),
        ),
        (
            "room:12",
            SyncPayload::MessagePinned(PinState {
                message_id: 9001,
                room_id: 12,
                pinned: false,
                pin_count: 3,
            }),
            json!({"type": "message.pinned", "data": {"messageId": 9001, "roomId": 12, "pinned": false, "pinCount": 3}}),
        ),
        (
            "room:12",
            SyncPayload::ThreadIndicator(ThreadIndicatorChanged {
                room_id: 12,
                parent_message_id: 9001,
                thread: Some(indicator()),
            }),
            json!({"type": "thread.indicator", "data": {
                "roomId": 12,
                "parentMessageId": 9001,
                "thread": {"threadId": 88, "replyCount": 3, "lastReplyAt": "2026-10-06T10:00:00.000Z", "replierIds": [8, 7]},
            }}),
        ),
        (
            "room:12",
            SyncPayload::ThreadIndicator(ThreadIndicatorChanged {
                room_id: 12,
                parent_message_id: 9001,
                thread: None,
            }),
            json!({"type": "thread.indicator", "data": {"roomId": 12, "parentMessageId": 9001, "thread": null}}),
        ),
        (
            "room:12",
            SyncPayload::ThreadCreated(thread()),
            json!({"type": "thread.created", "data": serde_json::to_value(thread()).unwrap()}),
        ),
        (
            "thread:88",
            SyncPayload::ThreadUpdated(thread()),
            json!({"type": "thread.updated", "data": serde_json::to_value(thread()).unwrap()}),
        ),
        (
            "thread:88",
            SyncPayload::ThreadRemoved(ThreadRemoved {
                thread_id: 88,
                room_id: 12,
            }),
            json!({"type": "thread.removed", "data": {"threadId": 88, "roomId": 12}}),
        ),
        (
            "user",
            SyncPayload::ThreadUnread(ThreadUnread {
                thread_id: 88,
                room_id: 12,
                refresh_only: false,
            }),
            json!({"type": "thread.unread", "data": {"threadId": 88, "roomId": 12, "refreshOnly": false}}),
        ),
        (
            "user",
            SyncPayload::ThreadRead(ThreadRead {
                thread_id: 88,
                room_id: 12,
            }),
            json!({"type": "thread.read", "data": {"threadId": 88, "roomId": 12}}),
        ),
        (
            "user",
            SyncPayload::SavedChanged(SavedChanged {
                message_id: 9001,
                item: None,
            }),
            json!({"type": "saved.changed", "data": {"messageId": 9001, "item": null}}),
        ),
    ];
    for (topic, payload, wire) in cases {
        let event = SyncEvent {
            seq: 6,
            topic: topic.into(),
            payload,
        };
        let mut expected = wire;
        expected["seq"] = json!(6);
        expected["topic"] = json!(topic);
        assert_wire(&event, expected);
    }
}

#[test]
fn thread_typing_uses_the_thread_topic() {
    assert_wire(
        &ClientFrame::Typing {
            conv: "thread:88".into(),
            on: true,
        },
        json!({"t": "typing", "conv": "thread:88", "on": true}),
    );
}
