//! Wire shapes of the S7 bot types.

use serde_json::json;

use crate::tests::assert_wire;
use crate::*;

fn agent() -> BotAgent {
    BotAgent {
        id: 11,
        provider: Some("anthropic".into()),
        runtime: None,
        description: Some("Bends things".into()),
        daily_message_cap: Some(50),
        daily_board_post_cap: None,
        daily_external_action_cap: Some(5),
        usage: "2/50 messages · 0 board posts · 0/5 external actions".into(),
        suspended: false,
        ledger_url: "/agents/11/events".into(),
        approvals_url: "/agents/11/approvals".into(),
    }
}

#[test]
fn the_list_and_a_bot() {
    assert_wire(
        &BotList {
            bots: vec![BotSummary {
                id: 9,
                name: "Bender".into(),
                avatar_url: "/users/9/avatar?v=1".into(),
                icon: Some(BotIcon::Emoji {
                    title: "Robot".into(),
                    character: "🤖".into(),
                }),
                ownership: "Workspace agent · Owned by Grace".into(),
                rooms: vec![BotRoom {
                    id: 3,
                    name: "Ops".into(),
                    message_command: "curl -d 'Hello!' https://chat.example/rooms/3/BOT_KEY/messages"
                        .into(),
                    attachment_command:
                        "curl -F \"attachment=@/path/to/file\" https://chat.example/rooms/3/BOT_KEY/messages"
                            .into(),
                }],
            }],
        },
        json!({"bots": [{
            "id": 9,
            "name": "Bender",
            "avatarUrl": "/users/9/avatar?v=1",
            "icon": {"kind": "emoji", "title": "Robot", "character": "🤖"},
            "ownership": "Workspace agent · Owned by Grace",
            "rooms": [{
                "id": 3,
                "name": "Ops",
                "messageCommand": "curl -d 'Hello!' https://chat.example/rooms/3/BOT_KEY/messages",
                "attachmentCommand": "curl -F \"attachment=@/path/to/file\" https://chat.example/rooms/3/BOT_KEY/messages"
            }]
        }]}),
    );
    assert_wire(
        &BotChange {
            bot: Bot {
                id: 9,
                name: "Bender".into(),
                avatar_url: "/users/9/avatar?v=1".into(),
                avatar_attached: false,
                icon_name: Some("acme".into()),
                icon: Some(BotIcon::Image {
                    title: "Acme".into(),
                    url: "/icons/acme.svg".into(),
                }),
                webhook_url: Some("https://example.com/hook".into()),
                can_administer: true,
                agent: Some(agent()),
                signing_secret: None,
                github: Some(BotGithub {
                    login: "bender-bot".into(),
                    usable: false,
                    disconnected_reason: Some("Bad credentials".into()),
                }),
            },
            notice: Some("Agent suspended; 0 approvals cancelled.".into()),
        },
        json!({
            "bot": {
                "id": 9,
                "name": "Bender",
                "avatarUrl": "/users/9/avatar?v=1",
                "avatarAttached": false,
                "iconName": "acme",
                "icon": {"kind": "image", "title": "Acme", "url": "/icons/acme.svg"},
                "webhookUrl": "https://example.com/hook",
                "canAdminister": true,
                "agent": {
                    "id": 11,
                    "provider": "anthropic",
                    "runtime": null,
                    "description": "Bends things",
                    "dailyMessageCap": 50,
                    "dailyBoardPostCap": null,
                    "dailyExternalActionCap": 5,
                    "usage": "2/50 messages · 0 board posts · 0/5 external actions",
                    "suspended": false,
                    "ledgerUrl": "/agents/11/events",
                    "approvalsUrl": "/agents/11/approvals"
                },
                "signingSecret": null,
                "github": {"login": "bender-bot", "usable": false, "disconnectedReason": "Bad credentials"}
            },
            "notice": "Agent suspended; 0 approvals cancelled."
        }),
    );
    assert_wire(&BotRemoved { id: 9 }, json!({"id": 9}));
}

#[test]
fn the_bot_writes() {
    assert_wire(
        &CreateBot {
            name: "Robo".into(),
            icon_name: None,
            webhook_url: Some("https://example.com/robo".into()),
            avatar: Some("signed-blob".into()),
        },
        json!({"name": "Robo", "iconName": null, "webhookUrl": "https://example.com/robo", "avatar": "signed-blob"}),
    );
    assert_wire(
        &BotKey {
            id: 9,
            name: "Robo".into(),
            key: "9-abc".into(),
            example_command: "curl -d 'Hello!' https://chat.example/rooms/ROOM_ID/9-abc/messages"
                .into(),
        },
        json!({
            "id": 9,
            "name": "Robo",
            "key": "9-abc",
            "exampleCommand": "curl -d 'Hello!' https://chat.example/rooms/ROOM_ID/9-abc/messages"
        }),
    );
    assert_wire(
        &UpdateBot {
            name: Some("Robo".into()),
            icon_name: Some(String::new()),
            webhook_url: None,
            avatar: None,
            agent: Some(UpdateBotAgent {
                daily_message_cap: Some("lots".into()),
                ..Default::default()
            }),
        },
        json!({
            "name": "Robo",
            "iconName": "",
            "webhookUrl": null,
            "avatar": null,
            "agent": {
                "provider": null,
                "runtime": null,
                "description": null,
                "dailyMessageCap": "lots",
                "dailyBoardPostCap": null,
                "dailyExternalActionCap": null
            }
        }),
    );
    assert_wire(
        &ConnectGithub {
            access_token: "ghp_x".into(),
        },
        json!({"accessToken": "ghp_x"}),
    );
}

#[test]
fn credentials_and_grants() {
    assert_wire(
        &CredentialCreated {
            secret: "cfa_secret".into(),
            credentials: CredentialList {
                bot_id: 9,
                bot_name: "Bender".into(),
                can_issue: true,
                credentials: vec![Credential {
                    id: 4,
                    name: "ci".into(),
                    last_four: "cret".into(),
                    created_by: "Grace Hopper".into(),
                    created_at: "2026-03-02T16:00:00.000Z".into(),
                    last_used_at: None,
                    expires_at: Some("2030-01-02T03:04:00-05:00".into()),
                    state: CredentialState::Expired,
                }],
            },
        },
        json!({
            "secret": "cfa_secret",
            "credentials": {
                "botId": 9,
                "botName": "Bender",
                "canIssue": true,
                "credentials": [{
                    "id": 4,
                    "name": "ci",
                    "lastFour": "cret",
                    "createdBy": "Grace Hopper",
                    "createdAt": "2026-03-02T16:00:00.000Z",
                    "lastUsedAt": null,
                    "expiresAt": "2030-01-02T03:04:00-05:00",
                    "state": "expired"
                }]
            }
        }),
    );
    assert_wire(
        &CreateCredential {
            name: "ci".into(),
            expires_at: Some("2026-10-31T17:00".into()),
        },
        json!({"name": "ci", "expiresAt": "2026-10-31T17:00"}),
    );
    assert_wire(
        &GrantList {
            bot_id: 9,
            bot_name: "Bender".into(),
            can_grant: false,
            legacy: false,
            grants: vec![Grant {
                id: 2,
                capability: "react".into(),
                room_name: "Workspace-wide".into(),
                granted_by: "Grace Hopper".into(),
                created_at: "2026-03-02T16:00:00.000Z".into(),
                revoked: true,
            }],
            capabilities: vec!["read_messages".into(), "react".into()],
            rooms: vec![GrantRoom {
                id: 3,
                name: "Ops".into(),
            }],
        },
        json!({
            "botId": 9,
            "botName": "Bender",
            "canGrant": false,
            "legacy": false,
            "grants": [{
                "id": 2,
                "capability": "react",
                "roomName": "Workspace-wide",
                "grantedBy": "Grace Hopper",
                "createdAt": "2026-03-02T16:00:00.000Z",
                "revoked": true
            }],
            "capabilities": ["read_messages", "react"],
            "rooms": [{"id": 3, "name": "Ops"}]
        }),
    );
    assert_wire(
        &CreateGrant {
            capability: "react".into(),
            room_id: None,
        },
        json!({"capability": "react", "roomId": null}),
    );
}
