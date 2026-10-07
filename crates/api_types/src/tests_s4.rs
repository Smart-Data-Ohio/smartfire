//! Wire shapes of the S4 contract A types: agent identity, steps and approvals.

use serde_json::json;

use crate::tests::{assert_wire, message, user};
use crate::*;

fn badge() -> AgentBadge {
    AgentBadge {
        agent_id: 3,
        kind: AgentKind::Personal,
        status: AgentStatus::Working,
        suspended: false,
    }
}

fn agent_user() -> User {
    User {
        id: 40,
        name: "Scout".into(),
        role: UserRole::Bot,
        bio: None,
        avatar_url: "/users/40/avatar?v=1700000001".into(),
        custom_status: None,
        avatar_icon: Some(Icon {
            name: "github".into(),
            title: "GitHub".into(),
            kind: IconKind::Brand,
            character: None,
            image_url: Some("/assets/icons/github.svg".into()),
        }),
        agent: Some(badge()),
        ..user()
    }
}

fn directory_row() -> AgentDirectoryRow {
    AgentDirectoryRow {
        agent_id: 3,
        user_id: 40,
        kind: AgentKind::Personal,
        owner_id: Some(7),
        status: AgentStatus::Working,
        status_note: Some("Triaging the queue".into()),
        suspended: false,
        created_at: "2026-09-01T10:00:00.000Z".into(),
        status_changed_at: Some("2026-10-06T09:00:00.000Z".into()),
        last_seen_at: None,
    }
}

fn directory_row_wire() -> serde_json::Value {
    json!({
        "agentId": 3,
        "userId": 40,
        "kind": "personal",
        "ownerId": 7,
        "status": "working",
        "statusNote": "Triaging the queue",
        "suspended": false,
        "createdAt": "2026-09-01T10:00:00.000Z",
        "statusChangedAt": "2026-10-06T09:00:00.000Z",
        "lastSeenAt": null,
    })
}

fn step() -> AgentStep {
    AgentStep {
        id: 11,
        message_id: Some(9001),
        thread_id: None,
        name: "Search the docs".into(),
        status: AgentStepStatus::Done,
        input_summary: Some("rate limits".into()),
        output_summary: None,
        duration_ms: Some(1250),
        position: 0,
        created_at: "2026-10-06T09:15:01.000Z".into(),
        updated_at: "2026-10-06T09:15:02.250Z".into(),
    }
}

fn step_wire() -> serde_json::Value {
    json!({
        "id": 11,
        "messageId": 9001,
        "threadId": null,
        "name": "Search the docs",
        "status": "done",
        "inputSummary": "rate limits",
        "outputSummary": null,
        "durationMs": 1250,
        "position": 0,
        "createdAt": "2026-10-06T09:15:01.000Z",
        "updatedAt": "2026-10-06T09:15:02.250Z",
    })
}

fn approval() -> AgentApproval {
    AgentApproval {
        id: 21,
        agent_id: 3,
        agent_user_id: 40,
        room_id: Some(12),
        room_name: Some("general".into()),
        action: "github.merge_pull_request".into(),
        summary: "Merge #42 into main".into(),
        status: AgentApprovalStatus::Pending,
        expires_at: "2026-10-07T09:15:00.000Z".into(),
        created_at: "2026-10-06T09:15:00.000Z".into(),
        decided_by_id: None,
        decided_at: None,
        decision_note: None,
        github_login: Some("scout-bot".into()),
        fizzy_user_name: None,
        admin_only: true,
        approvable: false,
        deniable: true,
    }
}

fn approval_wire() -> serde_json::Value {
    json!({
        "id": 21,
        "agentId": 3,
        "agentUserId": 40,
        "roomId": 12,
        "roomName": "general",
        "action": "github.merge_pull_request",
        "summary": "Merge #42 into main",
        "status": "pending",
        "expiresAt": "2026-10-07T09:15:00.000Z",
        "createdAt": "2026-10-06T09:15:00.000Z",
        "decidedById": null,
        "decidedAt": null,
        "decisionNote": null,
        "githubLogin": "scout-bot",
        "fizzyUserName": null,
        "adminOnly": true,
        "approvable": false,
        "deniable": true,
    })
}

#[test]
fn agent_enums_are_lowercase() {
    for (status, wire) in [
        (AgentStatus::Idle, "idle"),
        (AgentStatus::Working, "working"),
        (AgentStatus::Waiting, "waiting"),
        (AgentStatus::Failed, "failed"),
    ] {
        assert_wire(&status, json!(wire));
    }
    assert_wire(&AgentKind::Workspace, json!("workspace"));
    for (status, wire) in [
        (AgentStepStatus::Pending, "pending"),
        (AgentStepStatus::Running, "running"),
        (AgentStepStatus::Done, "done"),
        (AgentStepStatus::Failed, "failed"),
    ] {
        assert_wire(&status, json!(wire));
    }
    for (capability, wire) in [
        (AgentCapability::ReadMessages, "read_messages"),
        (AgentCapability::PostMessages, "post_messages"),
        (AgentCapability::React, "react"),
        (AgentCapability::ManageThreads, "manage_threads"),
        (AgentCapability::ExternalAction, "external_action"),
        (AgentCapability::Fizzy, "fizzy"),
        (AgentCapability::DmAnyone, "dm_anyone"),
    ] {
        assert_wire(&capability, json!(wire));
    }
    assert_wire(&ApprovalDecision::Approved, json!("approved"));
    assert_wire(&ApprovalDecision::Denied, json!("denied"));
}

#[test]
fn an_agent_user_carries_its_badge_and_icon() {
    assert_wire(
        &agent_user(),
        json!({
            "id": 40,
            "name": "Scout",
            "role": "bot",
            "status": "active",
            "bio": null,
            "avatarUrl": "/users/40/avatar?v=1700000001",
            "hasAvatar": true,
            "customStatus": null,
            "avatarIcon": {
                "name": "github",
                "title": "GitHub",
                "kind": "brand",
                "character": null,
                "imageUrl": "/assets/icons/github.svg",
            },
            "agent": {"agentId": 3, "kind": "personal", "status": "working", "suspended": false},
            "createdAt": "2026-09-26T12:26:46.848Z",
        }),
    );
}

#[test]
fn directory_and_profile_round_trip() {
    assert_wire(
        &AgentDirectory {
            agents: vec![directory_row()],
            users: vec![],
        },
        json!({"agents": [directory_row_wire()], "users": []}),
    );
    let profile = AgentProfile {
        agent: directory_row(),
        provider: Some("Anthropic".into()),
        runtime: None,
        description: Some("Keeps the queue tidy.".into()),
        rooms: vec![AgentProfileRoom {
            room_id: 12,
            name: "general".into(),
        }],
        hidden_room_count: 2,
        grants: Some(AgentGrants {
            legacy: false,
            grants: vec![
                AgentGrant {
                    capability: AgentCapability::PostMessages,
                    workspace_wide: false,
                    room_count: 2,
                },
                AgentGrant {
                    capability: AgentCapability::ReadMessages,
                    workspace_wide: true,
                    room_count: 0,
                },
            ],
        }),
        management: Some(AgentManagement {
            activity_summary: AgentActivitySummary {
                delivered: 4,
                acknowledged: 3,
                posted: 9,
                suppressed: 0,
            },
            budget_usage: vec![
                AgentBudgetUsage {
                    cap: AgentBudgetCap::Messages,
                    used: 9,
                    limit: Some(200),
                },
                AgentBudgetUsage {
                    cap: AgentBudgetCap::BoardPosts,
                    used: 0,
                    limit: None,
                },
            ],
        }),
        users: vec![],
    };
    assert_wire(
        &profile,
        json!({
            "agent": directory_row_wire(),
            "provider": "Anthropic",
            "runtime": null,
            "description": "Keeps the queue tidy.",
            "rooms": [{"roomId": 12, "name": "general"}],
            "hiddenRoomCount": 2,
            "grants": {
                "legacy": false,
                "grants": [
                    {"capability": "post_messages", "workspaceWide": false, "roomCount": 2},
                    {"capability": "read_messages", "workspaceWide": true, "roomCount": 0},
                ],
            },
            "management": {
                "activitySummary": {"delivered": 4, "acknowledged": 3, "posted": 9, "suppressed": 0},
                "budgetUsage": [
                    {"cap": "messages", "used": 9, "limit": 200},
                    {"cap": "board_posts", "used": 0, "limit": null},
                ],
            },
            "users": [],
        }),
    );
    // Someone who is neither an administrator nor the owner gets no grants or management.
    let wire = serde_json::to_value(AgentProfile {
        grants: None,
        management: None,
        ..profile
    })
    .unwrap();
    assert_eq!(wire["grants"], serde_json::Value::Null);
    assert_eq!(wire["management"], serde_json::Value::Null);
}

#[test]
fn a_message_carries_its_steps() {
    let message = MessageDTO {
        steps: vec![step()],
        ..message()
    };
    let wire = serde_json::to_value(&message).unwrap();
    assert_eq!(wire["steps"], json!([step_wire()]));
    assert_eq!(serde_json::from_value::<MessageDTO>(wire).unwrap(), message);
}

#[test]
fn approvals_round_trip() {
    assert_wire(
        &AgentApprovalPage {
            approvals: vec![approval()],
            users: vec![],
            next_cursor: Some("MjE".into()),
        },
        json!({"approvals": [approval_wire()], "users": [], "nextCursor": "MjE"}),
    );
    assert_wire(
        &DecideApproval {
            decision: ApprovalDecision::Denied,
            note: Some("Not on a Friday".into()),
        },
        json!({"decision": "denied", "note": "Not on a Friday"}),
    );
    // The note is an optional key: left out when there's none, and missing reads as none.
    assert_wire(
        &DecideApproval {
            decision: ApprovalDecision::Approved,
            note: None,
        },
        json!({"decision": "approved"}),
    );
}

#[test]
fn s4_sync_events_round_trip() {
    let cases = [
        (
            "user",
            SyncPayload::AgentStatus(AgentStatusChanged {
                agent_id: 3,
                user_id: 40,
                status: AgentStatus::Working,
                status_note: None,
                status_changed_at: Some("2026-10-06T09:00:00.000Z".into()),
                suspended: false,
                working_presence: Some("Reviewing #42".into()),
                working_presence_expires_at: Some("2026-10-06T09:20:00.000Z".into()),
            }),
            json!({"type": "agent.status", "data": {
                "agentId": 3,
                "userId": 40,
                "status": "working",
                "statusNote": null,
                "statusChangedAt": "2026-10-06T09:00:00.000Z",
                "suspended": false,
                "workingPresence": "Reviewing #42",
                "workingPresenceExpiresAt": "2026-10-06T09:20:00.000Z",
            }}),
        ),
        (
            "room:12",
            SyncPayload::AgentSteps(AgentStepsChanged {
                room_id: 12,
                message_id: Some(9001),
                thread_id: None,
                steps: vec![step()],
            }),
            json!({"type": "agent.steps", "data": {"roomId": 12, "messageId": 9001, "threadId": null, "steps": [step_wire()]}}),
        ),
        (
            "thread:88",
            SyncPayload::AgentSteps(AgentStepsChanged {
                room_id: 12,
                message_id: None,
                thread_id: Some(88),
                steps: vec![],
            }),
            json!({"type": "agent.steps", "data": {"roomId": 12, "messageId": null, "threadId": 88, "steps": []}}),
        ),
        (
            "user",
            SyncPayload::ApprovalUpdated(ApprovalUpdated {
                approval: approval(),
                users: vec![],
            }),
            json!({"type": "approval.updated", "data": {"approval": approval_wire(), "users": []}}),
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
