//! Wire shapes of the S4 contract B types: the agent event ledger and work tracking.

use serde_json::json;

use crate::tests::{assert_wire, user};
use crate::tests_s4::{agent_user, step, step_wire};
use crate::*;

fn ledger_event() -> AgentLedgerEvent {
    AgentLedgerEvent {
        id: 501,
        event_type: AgentLedgerEventType::GithubActionCompleted,
        outcome: Some(AgentDeliveryOutcome::Delivered),
        created_at: "2026-10-06T09:30:00.000Z".into(),
        room_id: Some(12),
        room_name: Some("general".into()),
        actor_id: Some(7),
        message_id: Some(9001),
        hop: 1,
        detail: None,
        webhook_status: AgentWebhookStatus::Failed,
        webhook_attempts: 2,
        webhook_last_error: Some("HTTP 500".into()),
        external: Some(AgentExternalResult {
            action: Some("merge_pull_request".into()),
            status: Some("succeeded".into()),
            message: None,
        }),
        handoff_summary: None,
        content: Some("Merging #42 now".into()),
    }
}

fn ledger_event_wire() -> serde_json::Value {
    json!({
        "id": 501,
        "eventType": "github_action_completed",
        "outcome": "delivered",
        "createdAt": "2026-10-06T09:30:00.000Z",
        "roomId": 12,
        "roomName": "general",
        "actorId": 7,
        "messageId": 9001,
        "hop": 1,
        "detail": null,
        "webhookStatus": "failed",
        "webhookAttempts": 2,
        "webhookLastError": "HTTP 500",
        "external": {"action": "merge_pull_request", "status": "succeeded", "message": null},
        "handoffSummary": null,
        "content": "Merging #42 now",
    })
}

#[test]
fn ledger_page_round_trips() {
    assert_wire(
        &AgentLedgerPage {
            events: vec![ledger_event()],
            users: vec![user()],
            next_cursor: Some("NTAx".into()),
        },
        json!({
            "events": [ledger_event_wire()],
            "users": [serde_json::to_value(user()).unwrap()],
            "nextCursor": "NTAx",
        }),
    );
}

#[test]
fn ledger_entry_without_a_delivery_has_nulls() {
    let posted = AgentLedgerEvent {
        id: 502,
        event_type: AgentLedgerEventType::Posted,
        outcome: None,
        room_id: None,
        room_name: None,
        actor_id: None,
        message_id: None,
        hop: 0,
        webhook_status: AgentWebhookStatus::None,
        webhook_attempts: 0,
        webhook_last_error: None,
        external: None,
        content: None,
        ..ledger_event()
    };
    let wire = serde_json::to_value(&posted).unwrap();
    assert_eq!(wire["eventType"], "posted");
    assert_eq!(wire["outcome"], json!(null));
    assert_eq!(wire["webhookStatus"], "none");
    assert_eq!(wire["external"], json!(null));
    assert_wire(&posted, wire);
}

#[test]
fn ledger_enums_are_the_classic_strings() {
    let types = [
        (AgentLedgerEventType::Mention, "mention"),
        (AgentLedgerEventType::DirectMessage, "direct_message"),
        (AgentLedgerEventType::Reply, "reply"),
        (AgentLedgerEventType::ApprovalDecided, "approval_decided"),
        (
            AgentLedgerEventType::GithubActionCompleted,
            "github_action_completed",
        ),
        (
            AgentLedgerEventType::FizzyActionCompleted,
            "fizzy_action_completed",
        ),
        (AgentLedgerEventType::WorkAssigned, "work_assigned"),
        (AgentLedgerEventType::WorkUnassigned, "work_unassigned"),
        (AgentLedgerEventType::WorkHandedOff, "work_handed_off"),
        (AgentLedgerEventType::SlashCommand, "slash_command"),
        (AgentLedgerEventType::Posted, "posted"),
        (
            AgentLedgerEventType::DeliverySuppressedRateLimit,
            "delivery_suppressed_rate_limit",
        ),
        (
            AgentLedgerEventType::DeliverySuppressedHopLimit,
            "delivery_suppressed_hop_limit",
        ),
        (
            AgentLedgerEventType::DeliverySuppressedRevoked,
            "delivery_suppressed_revoked",
        ),
    ];
    for (value, wire) in types {
        assert_wire(&value, json!(wire));
    }
    for (value, wire) in [
        (AgentWebhookStatus::None, "none"),
        (AgentWebhookStatus::Pending, "pending"),
        (AgentWebhookStatus::Delivered, "delivered"),
        (AgentWebhookStatus::Failed, "failed"),
    ] {
        assert_wire(&value, json!(wire));
    }
    for (value, wire) in [
        (AgentDeliveryOutcome::Pending, "pending"),
        (AgentDeliveryOutcome::Delivered, "delivered"),
        (AgentDeliveryOutcome::Acknowledged, "acknowledged"),
        (AgentDeliveryOutcome::Suppressed, "suppressed"),
    ] {
        assert_wire(&value, json!(wire));
    }
}

fn pull_request_link() -> WorkLink {
    WorkLink {
        id: 31,
        kind: WorkLinkKind::PullRequest,
        label: "basecamp/once-campfire#42".into(),
        url: "https://github.com/basecamp/once-campfire/pull/42".into(),
        pull_request_state: Some(WorkPullRequestState::Merged),
        title: Some("Speed up search".into()),
        event_starts_at: None,
        event_time_zone: None,
        event_cancelled: false,
    }
}

fn work_facts() -> WorkFacts {
    WorkFacts {
        status: WorkStatus::InProgress,
        owner: Some(agent_user()),
        owner_active: true,
        run_url: Some("https://ci.example.com/runs/7".into()),
        result_updated_at: None,
        links: vec![pull_request_link()],
        tags: vec![],
        message_count: 4,
    }
}

fn work_facts_wire() -> serde_json::Value {
    json!({
        "status": "in_progress",
        "owner": serde_json::to_value(agent_user()).unwrap(),
        "ownerActive": true,
        "runUrl": "https://ci.example.com/runs/7",
        "resultUpdatedAt": null,
        "links": [{
            "id": 31,
            "kind": "pull_request",
            "label": "basecamp/once-campfire#42",
            "url": "https://github.com/basecamp/once-campfire/pull/42",
            "pullRequestState": "merged",
            "title": "Speed up search",
            "eventStartsAt": null,
            "eventTimeZone": null,
            "eventCancelled": false,
        }],
        "tags": [],
        "messageCount": 4,
    })
}

fn work_thread() -> Thread {
    Thread {
        id: 88,
        room_id: 12,
        parent_message_id: Some(9001),
        creator_id: 7,
        name: "Speed up search".into(),
        status: ThreadStatus::Active,
        reply_count: 3,
        last_activity_at: "2026-10-06T10:00:00.000Z".into(),
        auto_archive_after_minutes: 4320,
        created_at: "2026-10-06T09:20:00.000Z".into(),
        work: Some(work_facts()),
    }
}

#[test]
fn a_tracked_thread_carries_its_work_facts() {
    let wire = serde_json::to_value(work_thread()).unwrap();
    assert_eq!(wire["work"], work_facts_wire());
    assert_wire(&work_thread(), wire);

    let event_link = WorkLink {
        id: 32,
        kind: WorkLinkKind::Event,
        label: "Launch review".into(),
        url: "/rooms/12/events/5".into(),
        pull_request_state: None,
        title: None,
        event_starts_at: Some("2026-10-08T15:00:00.000Z".into()),
        event_time_zone: Some("America/New_York".into()),
        event_cancelled: true,
    };
    let wire = serde_json::to_value(&event_link).unwrap();
    assert_eq!(wire["kind"], "event");
    assert_eq!(wire["eventCancelled"], true);
    assert_wire(&event_link, wire);
    assert_wire(&WorkLinkKind::DriveFile, json!("drive_file"));
    for (value, wire) in [
        (WorkPullRequestState::Open, "open"),
        (WorkPullRequestState::Draft, "draft"),
        (WorkPullRequestState::Merged, "merged"),
        (WorkPullRequestState::Closed, "closed"),
    ] {
        assert_wire(&value, json!(wire));
    }
}

#[test]
fn an_unassigned_thread_and_an_unnamed_owner_snapshot_are_nulls() {
    let facts = WorkFacts {
        owner: None,
        owner_active: false,
        run_url: None,
        links: vec![],
        ..work_facts()
    };
    let wire = serde_json::to_value(&facts).unwrap();
    assert_eq!(wire["owner"], serde_json::Value::Null);
    assert_eq!(wire["runUrl"], serde_json::Value::Null);
    assert_wire(&facts, wire);
    assert_wire(
        &WorkOwnerSnapshot {
            user_id: Some(40),
            name: None,
        },
        json!({"userId": 40, "name": null}),
    );
}

#[test]
fn work_detail_round_trips() {
    let detail = WorkDetail {
        result_markdown: Some("Shipped in **v2.1**".into()),
        result_html: Some("<p>Shipped in <strong>v2.1</strong></p>".into()),
        result_updated_by_id: Some(7),
        steps: vec![AgentStep {
            message_id: None,
            thread_id: Some(88),
            ..step()
        }],
        history: vec![
            WorkHistoryEntry {
                id: 71,
                kind: WorkHistoryKind::Handoff,
                created_at: "2026-10-06T09:40:00.000Z".into(),
                actor_id: Some(7),
                from_status: Some(WorkStatus::InProgress),
                to_status: Some(WorkStatus::InProgress),
                from_owner: Some(WorkOwnerSnapshot {
                    user_id: Some(7),
                    name: Some("Ada Lovelace".into()),
                }),
                to_owner: Some(WorkOwnerSnapshot {
                    user_id: Some(40),
                    name: Some("Scout".into()),
                }),
                note: None,
                handoff: Some(WorkHistoryHandoff {
                    summary: "Finish the index rebuild".into(),
                    link_count: 1,
                    question_count: 0,
                }),
            },
            WorkHistoryEntry {
                id: 70,
                kind: WorkHistoryKind::Update,
                created_at: "2026-10-06T09:20:00.000Z".into(),
                actor_id: None,
                from_status: None,
                to_status: Some(WorkStatus::Planned),
                from_owner: None,
                to_owner: None,
                note: Some("Picked up from triage".into()),
                handoff: None,
            },
        ],
        owner_candidates: vec![
            WorkOwnerCandidate {
                user_id: 7,
                provider: None,
                description: None,
            },
            WorkOwnerCandidate {
                user_id: 40,
                provider: Some("Anthropic".into()),
                description: Some("Triage and fixes".into()),
            },
        ],
        handoff_receivers: vec![WorkHandoffReceiver {
            agent_id: 3,
            user_id: 40,
        }],
    };
    let mut thread_step = step_wire();
    thread_step["messageId"] = json!(null);
    thread_step["threadId"] = json!(88);
    assert_wire(
        &detail,
        json!({
            "resultMarkdown": "Shipped in **v2.1**",
            "resultHtml": "<p>Shipped in <strong>v2.1</strong></p>",
            "resultUpdatedById": 7,
            "steps": [thread_step],
            "history": [
                {
                    "id": 71,
                    "kind": "handoff",
                    "createdAt": "2026-10-06T09:40:00.000Z",
                    "actorId": 7,
                    "fromStatus": "in_progress",
                    "toStatus": "in_progress",
                    "fromOwner": {"userId": 7, "name": "Ada Lovelace"},
                    "toOwner": {"userId": 40, "name": "Scout"},
                    "note": null,
                    "handoff": {"summary": "Finish the index rebuild", "linkCount": 1, "questionCount": 0},
                },
                {
                    "id": 70,
                    "kind": "update",
                    "createdAt": "2026-10-06T09:20:00.000Z",
                    "actorId": null,
                    "fromStatus": null,
                    "toStatus": "planned",
                    "fromOwner": null,
                    "toOwner": null,
                    "note": "Picked up from triage",
                    "handoff": null,
                },
            ],
            "ownerCandidates": [
                {"userId": 7, "provider": null, "description": null},
                {"userId": 40, "provider": "Anthropic", "description": "Triage and fixes"},
            ],
            "handoffReceivers": [{"agentId": 3, "userId": 40}],
        }),
    );
    assert_wire(&WorkHistoryKind::Assignment, json!("assignment"));
    assert_wire(&WorkHistoryKind::Result, json!("result"));
}

#[test]
fn work_list_round_trips() {
    let list = WorkList {
        threads: vec![WorkListRow {
            thread: work_thread(),
            room_name: "general".into(),
            board: false,
            updated_at: "2026-10-06T10:00:00.000Z".into(),
        }],
        users: vec![user()],
    };
    assert_wire(
        &list,
        json!({
            "threads": [{
                "thread": serde_json::to_value(work_thread()).unwrap(),
                "roomName": "general",
                "board": false,
                "updatedAt": "2026-10-06T10:00:00.000Z",
            }],
            "users": [serde_json::to_value(user()).unwrap()],
        }),
    );
    for (value, wire) in [
        (WorkFilter::Open, "open"),
        (WorkFilter::Done, "done"),
        (WorkFilter::All, "all"),
        (WorkFilter::Agents, "agents"),
        (WorkFilter::Boards, "boards"),
    ] {
        assert_wire(&value, json!(wire));
    }
}

#[test]
fn update_work_tells_a_null_from_a_missing_key() {
    let empty = UpdateWork {
        status: None,
        owner_id: None,
        result_markdown: None,
        tags: None,
    };
    assert_wire(&empty, json!({}));

    // Stop tracking and unassign; leave the result alone.
    assert_wire(
        &UpdateWork {
            status: Some(None),
            owner_id: Some(None),
            result_markdown: None,
            tags: None,
        },
        json!({"status": null, "ownerId": null}),
    );

    assert_wire(
        &UpdateWork {
            status: Some(Some(WorkStatus::Blocked)),
            owner_id: Some(Some(40)),
            result_markdown: Some(Some("Waiting on review".into())),
            tags: None,
        },
        json!({"status": "blocked", "ownerId": 40, "resultMarkdown": "Waiting on review"}),
    );

    // A board post's tags replace the set; `[]` clears them.
    assert_wire(
        &UpdateWork {
            status: None,
            owner_id: None,
            result_markdown: None,
            tags: Some(vec!["api".into(), "bug".into()]),
        },
        json!({"tags": ["api", "bug"]}),
    );

    assert!(serde_json::from_value::<UpdateWork>(json!({"status": "started"})).is_err());
}

#[test]
fn create_work_handoff_round_trips() {
    assert_wire(
        &CreateWorkHandoff {
            receiver_agent_id: 3,
            summary: "Finish the index rebuild".into(),
            links: vec!["https://github.com/basecamp/once-campfire/pull/42".into()],
            open_questions: vec![],
        },
        json!({
            "receiverAgentId": 3,
            "summary": "Finish the index rebuild",
            "links": ["https://github.com/basecamp/once-campfire/pull/42"],
            "openQuestions": [],
        }),
    );
}
