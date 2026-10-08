//! Wire shapes of the S6 board types.

use serde_json::json;

use crate::tests::{assert_wire, user};
use crate::*;

#[test]
fn work_link_form_and_create_round_trip() {
    assert_wire(
        &WorkLinkForm {
            events: vec![WorkLinkEventCandidate {
                id: 9,
                title: "Review".into(),
                starts_at: "2026-10-08T14:00:00.000Z".into(),
                time_zone: "America/New_York".into(),
            }],
        },
        json!({"events": [{"id":9,"title":"Review","startsAt":"2026-10-08T14:00:00.000Z","timeZone":"America/New_York"}]}),
    );
    for (kind, input) in [
        (
            WorkLinkKind::PullRequest,
            json!({"kind":"pull_request","pullRequestUrl":"https://github.com/owner/repo/pull/123"}),
        ),
        (WorkLinkKind::Event, json!({"kind":"event","eventId":9})),
        (
            WorkLinkKind::DriveFile,
            json!({"kind":"drive_file","driveUrl":"https://docs.google.com/document/d/abc/edit"}),
        ),
    ] {
        let decoded: CreateWorkLink = serde_json::from_value(input.clone()).unwrap();
        assert_eq!(decoded.kind, kind);
        assert_wire(&decoded, input);
    }
    let empty: CreateWorkLink =
        serde_json::from_value(json!({"kind":"event","eventId":null})).unwrap();
    assert_eq!(empty.event_id, None);
}

fn post() -> Thread {
    Thread {
        id: 301,
        room_id: 40,
        parent_message_id: None,
        creator_id: 7,
        name: "Fix the login redirect".into(),
        status: ThreadStatus::Active,
        reply_count: 2,
        last_activity_at: "2026-10-07T09:30:00.000Z".into(),
        auto_archive_after_minutes: 10080,
        created_at: "2026-10-06T09:30:00.000Z".into(),
        work: Some(WorkFacts {
            status: WorkStatus::Planned,
            owner: None,
            owner_active: false,
            run_url: None,
            result_updated_at: None,
            links: vec![],
            tags: vec!["api".into(), "bug".into()],
            message_count: 3,
        }),
    }
}

#[test]
fn a_board_listing_round_trips() {
    let thread = serde_json::to_value(post()).unwrap();
    assert_eq!(thread["work"]["tags"], json!(["api", "bug"]));
    assert_eq!(thread["work"]["messageCount"], json!(3));
    assert_wire(
        &BoardListing {
            room_id: 40,
            status: BoardStatusFilter::Open,
            owner: "anyone".into(),
            tag: String::new(),
            page: 1,
            posts: vec![ThreadSummary {
                thread: post(),
                membership: None,
            }],
            has_more: false,
            any_posts: true,
            owner_options: vec![BoardOwnerOption {
                user_id: 7,
                agent: false,
            }],
            tag_counts: vec![BoardTagCount {
                name: "api".into(),
                count: 1,
            }],
            digest: Some(BoardDigest {
                date: "2026-10-07".into(),
                text: "Stale work digest: 1 post past its SLA".into(),
            }),
            can_administer: true,
            users: vec![user()],
        },
        json!({
            "roomId": 40,
            "status": "open",
            "owner": "anyone",
            "tag": "",
            "page": 1,
            "posts": [{"thread": thread, "membership": null}],
            "hasMore": false,
            "anyPosts": true,
            "ownerOptions": [{"userId": 7, "agent": false}],
            "tagCounts": [{"name": "api", "count": 1}],
            "digest": {"date": "2026-10-07", "text": "Stale work digest: 1 post past its SLA"},
            "canAdminister": true,
            "users": [serde_json::to_value(user()).unwrap()],
        }),
    );
    for (value, wire) in [
        (BoardStatusFilter::Open, "open"),
        (BoardStatusFilter::Done, "done"),
        (BoardStatusFilter::All, "all"),
    ] {
        assert_wire(&value, json!(wire));
    }
}

#[test]
fn the_new_post_form_and_create_round_trip() {
    assert_wire(
        &BoardPostForm {
            owner_candidates: vec![WorkOwnerCandidate {
                user_id: 7,
                provider: None,
                description: None,
            }],
            tag_suggestions: vec!["api".into()],
            users: vec![user()],
        },
        json!({
            "ownerCandidates": [{"userId": 7, "provider": null, "description": null}],
            "tagSuggestions": ["api"],
            "users": [serde_json::to_value(user()).unwrap()],
        }),
    );
    assert_wire(
        &CreateBoardPost {
            name: "Fix the login redirect".into(),
            status: WorkStatus::Planned,
            owner_id: None,
            tags: vec!["bug".into()],
            client_post_id: Some("0192a3b4-0000-7000-8000-000000000001".into()),
            message: Some(CreateMessage {
                client_message_id: "0192a3b4-0000-7000-8000-000000000001".into(),
                markdown_source: "Steps to reproduce…".into(),
                reply_to_message_id: None,
                reply_notify_author: None,
                attachment_signed_id: None,
            }),
        },
        json!({
            "name": "Fix the login redirect",
            "status": "planned",
            "ownerId": null,
            "tags": ["bug"],
            "clientPostId": "0192a3b4-0000-7000-8000-000000000001",
            "message": {
                "clientMessageId": "0192a3b4-0000-7000-8000-000000000001",
                "markdownSource": "Steps to reproduce…",
                "replyToMessageId": null,
                "replyNotifyAuthor": null,
                "attachmentSignedId": null,
            },
        }),
    );
}

#[test]
fn board_automations_round_trip() {
    assert_wire(
        &BoardAutomations {
            room_id: 40,
            tag_rules: vec![BoardTagRule {
                id: 3,
                tag: "bug".into(),
                assignee_id: 7,
            }],
            sla_timers: vec![BoardSlaTimer {
                status: WorkStatus::InProgress,
                nudge_after_minutes: 60,
                escalate_after_minutes: 240,
            }],
            candidates: vec![7],
            users: vec![user()],
        },
        json!({
            "roomId": 40,
            "tagRules": [{"id": 3, "tag": "bug", "assigneeId": 7}],
            "slaTimers": [{"status": "in_progress", "nudgeAfterMinutes": 60, "escalateAfterMinutes": 240}],
            "candidates": [7],
            "users": [serde_json::to_value(user()).unwrap()],
        }),
    );
    assert_wire(
        &CreateBoardTagRule {
            tag: "bug".into(),
            assignee_id: None,
        },
        json!({"tag": "bug", "assigneeId": null}),
    );
    let off = BoardSlaTimerInput {
        nudge_after_minutes: None,
        escalate_after_minutes: None,
    };
    assert_wire(
        &UpdateBoardSlaTimers {
            planned: BoardSlaTimerInput {
                nudge_after_minutes: Some(30),
                escalate_after_minutes: Some(90),
            },
            in_progress: off,
            blocked: off,
        },
        json!({
            "planned": {"nudgeAfterMinutes": 30, "escalateAfterMinutes": 90},
            "inProgress": {"nudgeAfterMinutes": null, "escalateAfterMinutes": null},
            "blocked": {"nudgeAfterMinutes": null, "escalateAfterMinutes": null},
        }),
    );
}
