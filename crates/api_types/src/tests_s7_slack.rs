//! Wire shapes of the S7 Slack importer types.

use std::collections::BTreeMap;

use serde_json::json;

use crate::tests::assert_wire;
use crate::*;

fn run() -> SlackRun {
    SlackRun {
        id: 12,
        kind: SlackRunKind::Workspace,
        mode: SlackRunMode::DryRun,
        status: SlackRunStatus::Running,
        title: "Workspace dry run #12".into(),
        started_by: "Grace".into(),
        created_at: "2026-10-06T12:00:00.000Z".into(),
        started_at: Some("2026-10-06T12:00:05.000Z".into()),
        finished_at: None,
        phase: Some("conversations".into()),
        current: Some("#general".into()),
        queued_behind: false,
        people: Some(SlackPeople {
            total: 9,
            matched: 6,
            placeholders: 2,
            deactivated: 1,
            bots: 0,
        }),
        counts: None,
        api_calls: Some(41),
        issues_count: 2,
        error: None,
        active: true,
        cancellable: true,
        undoable: false,
        undo_blocked_reason: None,
        plan_ready: false,
        catch_up: false,
        conversations: vec![],
    }
}

fn conversation() -> SlackConversation {
    SlackConversation {
        id: "C111".into(),
        name: "general".into(),
        kind: "Public channel".into(),
        archived: false,
        members: 9,
        messages: 120,
        threads: 7,
    }
}

#[test]
fn the_setup_page_and_its_writes() {
    assert_wire(
        &SlackSetup {
            client_id: Some("123.456".into()),
            configured: true,
            configured_by: Some("Grace".into()),
            team_name: Some("Acme".into()),
            team_known: true,
            connection: SlackConnectionState::Rejected {
                reason: Some("token_revoked".into()),
            },
            active_run: Some(SlackRunSummary {
                id: 12,
                kind: SlackRunKind::Workspace,
                mode: SlackRunMode::DryRun,
                status: SlackRunStatus::Queued,
            }),
            manifest: "display_information: …".into(),
            connect_path: "/slack/oauth/start?return_to=%2Faccount%2Fslack_import".into(),
        },
        json!({
            "clientId": "123.456",
            "configured": true,
            "configuredBy": "Grace",
            "teamName": "Acme",
            "teamKnown": true,
            "connection": {"state": "rejected", "reason": "token_revoked"},
            "activeRun": {"id": 12, "kind": "workspace", "mode": "dry_run", "status": "queued"},
            "manifest": "display_information: …",
            "connectPath": "/slack/oauth/start?return_to=%2Faccount%2Fslack_import"
        }),
    );
    assert_wire(&SlackConnectionState::None, json!({"state": "none"}));
    assert_wire(
        &SaveSlackCredentials {
            client_id: "123.456".into(),
            client_secret: None,
        },
        json!({"clientId": "123.456", "clientSecret": null}),
    );
    assert_wire(
        &SlackDisconnected {
            notice: "Slack disconnected.".into(),
        },
        json!({"notice": "Slack disconnected."}),
    );
}

#[test]
fn the_run_lists() {
    let row = SlackRunRow {
        id: 12,
        kind: SlackRunKind::Personal,
        mode: SlackRunMode::Import,
        status: SlackRunStatus::Undone,
        started_by: "Grace".into(),
        created_at: "2026-10-06T12:00:00.000Z".into(),
    };
    let wire = json!({
        "id": 12,
        "kind": "personal",
        "mode": "import",
        "status": "undone",
        "startedBy": "Grace",
        "createdAt": "2026-10-06T12:00:00.000Z"
    });
    assert_wire(
        &SlackRunList {
            runs: vec![row.clone()],
        },
        json!({"runs": [wire.clone()]}),
    );
    assert_wire(
        &SlackPersonal {
            team_known: true,
            connection: SlackConnectionState::Connected,
            connect_path: "/slack/oauth/start?return_to=%2Fslack%2Fimports".into(),
            runs: vec![row],
        },
        json!({
            "teamKnown": true,
            "connection": {"state": "connected"},
            "connectPath": "/slack/oauth/start?return_to=%2Fslack%2Fimports",
            "runs": [wire]
        }),
    );
}

#[test]
fn a_run_and_its_issues() {
    let mut run = run();
    run.counts = Some(SlackCounts {
        rooms_created: 2,
        rooms_merged: 1,
        messages: 300,
        replies: 40,
        threads: 8,
        reactions: 12,
        pins: 1,
        files_linked: 3,
        skipped: 0,
    });
    run.conversations = vec![conversation()];
    assert_wire(
        &SlackRunPage {
            run,
            issues: vec![SlackIssue {
                level: "warning".into(),
                slack_ref: Some("C111".into()),
                message: "No access".into(),
            }],
            next_page: Some(2),
        },
        json!({
            "run": {
                "id": 12,
                "kind": "workspace",
                "mode": "dry_run",
                "status": "running",
                "title": "Workspace dry run #12",
                "startedBy": "Grace",
                "createdAt": "2026-10-06T12:00:00.000Z",
                "startedAt": "2026-10-06T12:00:05.000Z",
                "finishedAt": null,
                "phase": "conversations",
                "current": "#general",
                "queuedBehind": false,
                "people": {"total": 9, "matched": 6, "placeholders": 2, "deactivated": 1, "bots": 0},
                "counts": {
                    "roomsCreated": 2, "roomsMerged": 1, "messages": 300, "replies": 40,
                    "threads": 8, "reactions": 12, "pins": 1, "filesLinked": 3, "skipped": 0
                },
                "apiCalls": 41,
                "issuesCount": 2,
                "error": null,
                "active": true,
                "cancellable": true,
                "undoable": false,
                "undoBlockedReason": null,
                "planReady": false,
                "catchUp": false,
                "conversations": [{
                    "id": "C111", "name": "general", "kind": "Public channel", "archived": false,
                    "members": 9, "messages": 120, "threads": 7
                }]
            },
            "issues": [{"level": "warning", "slackRef": "C111", "message": "No access"}],
            "nextPage": 2
        }),
    );
}

#[test]
fn the_plan_and_the_starts() {
    assert_wire(
        &SlackPlan {
            run_id: 12,
            conversations: vec![SlackPlanConversation {
                conversation: conversation(),
                target: "4".into(),
            }],
            rooms: vec![SlackRoomTarget {
                id: 4,
                name: "General".into(),
            }],
            samples: vec![SlackSample {
                conversation: "general".into(),
                slack_text: "*hi*".into(),
                html: "<p><strong>hi</strong></p>".into(),
            }],
            default_oldest: "2026-09-22".into(),
        },
        json!({
            "runId": 12,
            "conversations": [{
                "conversation": {
                    "id": "C111", "name": "general", "kind": "Public channel", "archived": false,
                    "members": 9, "messages": 120, "threads": 7
                },
                "target": "4"
            }],
            "rooms": [{"id": 4, "name": "General"}],
            "samples": [{
                "conversation": "general",
                "slackText": "*hi*",
                "html": "<p><strong>hi</strong></p>"
            }],
            "defaultOldest": "2026-09-22"
        }),
    );
    assert_wire(
        &StartSlackDryRun {
            include_private: true,
            oldest: Some("2026-01-01".into()),
            latest: None,
        },
        json!({"includePrivate": true, "oldest": "2026-01-01", "latest": null}),
    );
    assert_wire(
        &StartSlackImport {
            conversation_ids: vec!["C111".into()],
            room_targets: BTreeMap::from([("C111".into(), "new".into())]),
            preset: SlackPreset::Test,
            oldest: None,
            latest: None,
        },
        json!({
            "conversationIds": ["C111"],
            "roomTargets": {"C111": "new"},
            "preset": "test",
            "oldest": null,
            "latest": null
        }),
    );
    assert_wire(
        &StartPersonalSlackImport {
            mode: SlackRunMode::Import,
            dry_run_id: Some(12),
            conversation_ids: vec!["D111".into()],
        },
        json!({"mode": "import", "dryRunId": 12, "conversationIds": ["D111"]}),
    );
    assert_wire(
        &SlackRunChange {
            run: run(),
            notice: "Dry run started.".into(),
        },
        json!({"run": serde_json::to_value(run()).unwrap(), "notice": "Dry run started."}),
    );
}
