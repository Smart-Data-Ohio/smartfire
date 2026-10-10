//! Wire shapes of the S3 types: the activity inbox, saved items, scheduled messages, search,
//! sidebar organisation, polls, events and link cards.

use serde_json::json;

use crate::tests::{assert_wire, message, user};
use crate::*;

fn activity_item() -> ActivityItem {
    ActivityItem {
        id: 301,
        event_type: ActivityEventType::Mention,
        state: ActivityState::Unread,
        read_at: None,
        handled_at: None,
        created_at: "2026-10-06T09:15:01.000Z".into(),
        updated_at: "2026-10-06T09:15:01.000Z".into(),
        source: ActivitySource {
            source_type: ActivitySourceType::Message,
            source_id: 9001,
            room_id: Some(12),
            thread_id: None,
            message_id: Some(9001),
            event_id: None,
            creator_id: Some(7),
            title: "general".into(),
            body: "Hello there @Grace".into(),
            occurred_at: "2026-10-06T09:15:00.123Z".into(),
            approval_status: None,
            budget_cap: None,
            path: "/rooms/12/@9001".into(),
        },
    }
}

fn activity_item_wire() -> serde_json::Value {
    json!({
        "id": 301,
        "eventType": "mention",
        "state": "unread",
        "readAt": null,
        "handledAt": null,
        "createdAt": "2026-10-06T09:15:01.000Z",
        "updatedAt": "2026-10-06T09:15:01.000Z",
        "source": {
            "sourceType": "message",
            "sourceId": 9001,
            "roomId": 12,
            "threadId": null,
            "messageId": 9001,
            "eventId": null,
            "creatorId": 7,
            "title": "general",
            "body": "Hello there @Grace",
            "occurredAt": "2026-10-06T09:15:00.123Z",
            "approvalStatus": null,
            "budgetCap": null,
            "path": "/rooms/12/@9001",
        },
    })
}

fn conversation() -> ConversationName {
    ConversationName {
        room_id: 12,
        thread_id: Some(88),
        room_kind: RoomKind::Open,
        room_name: "general".into(),
        room_icon_name: Some("campfire".into()),
        thread_name: Some("Hello there".into()),
    }
}

fn conversation_wire() -> serde_json::Value {
    json!({
        "roomId": 12,
        "threadId": 88,
        "roomKind": "open",
        "roomName": "general",
        "roomIconName": "campfire",
        "threadName": "Hello there",
    })
}

fn saved_item() -> SavedItem {
    SavedItem {
        id: 41,
        message_id: 9001,
        status: SavedStatus::InProgress,
        remind_at: Some("2026-10-07T09:00:00.000Z".into()),
        reminded_at: None,
        created_at: "2026-10-06T09:20:00.000Z".into(),
    }
}

fn saved_item_wire() -> serde_json::Value {
    json!({
        "id": 41,
        "messageId": 9001,
        "status": "in_progress",
        "remindAt": "2026-10-07T09:00:00.000Z",
        "remindedAt": null,
        "createdAt": "2026-10-06T09:20:00.000Z",
    })
}

fn poll() -> Poll {
    Poll {
        id: 5,
        message_id: 9001,
        as_of: "2026-10-06T09:30:00.000Z".into(),
        multiple: false,
        anonymous: false,
        closes_at: Some("2026-10-08T17:00:00.000Z".into()),
        closed_at: None,
        closed: false,
        total_votes: 3,
        options: vec![
            PollOption {
                id: 51,
                label: "Tacos".into(),
                votes: 2,
                voter_ids: vec![7, 8],
                media: None,
            },
            PollOption {
                id: 52,
                label: "Pizza".into(),
                votes: 1,
                voter_ids: vec![9],
                media: None,
            },
        ],
    }
}

fn poll_wire() -> serde_json::Value {
    json!({
        "id": 5,
        "messageId": 9001,
        "asOf": "2026-10-06T09:30:00.000Z",
        "multiple": false,
        "anonymous": false,
        "closesAt": "2026-10-08T17:00:00.000Z",
        "closedAt": null,
        "closed": false,
        "totalVotes": 3,
        "options": [
            {"id": 51, "label": "Tacos", "votes": 2, "voterIds": [7, 8]},
            {"id": 52, "label": "Pizza", "votes": 1, "voterIds": [9]},
        ],
    })
}

fn scheduled() -> ScheduledMessage {
    ScheduledMessage {
        id: 4,
        room_id: 12,
        thread_id: None,
        reply_to_message_id: None,
        reply_target: None,
        markdown_source: "Standup in 5".into(),
        excerpt: "Standup in 5".into(),
        send_at: "2026-10-07T13:55:00.000Z".into(),
        state: ScheduledMessageState::Dropped,
        sendable: false,
        sent_at: None,
        sent_message_id: None,
        dropped_at: Some("2026-10-07T13:55:12.000Z".into()),
        drop_reason: Some("its room was deleted".into()),
        created_at: "2026-10-06T11:00:00.000Z".into(),
    }
}

fn scheduled_wire() -> serde_json::Value {
    json!({
        "id": 4,
        "roomId": 12,
        "threadId": null,
        "replyToMessageId": null,
        "replyTarget": null,
        "markdownSource": "Standup in 5",
        "excerpt": "Standup in 5",
        "sendAt": "2026-10-07T13:55:00.000Z",
        "state": "dropped",
        "sendable": false,
        "sentAt": null,
        "sentMessageId": null,
        "droppedAt": "2026-10-07T13:55:12.000Z",
        "dropReason": "its room was deleted",
        "createdAt": "2026-10-06T11:00:00.000Z",
    })
}

fn category() -> RoomCategory {
    RoomCategory {
        id: 3,
        name: "Projects".into(),
        collapsed: false,
        position: 1,
    }
}

#[test]
fn activity_enums_are_snake_case() {
    let event_types = [
        (ActivityEventType::Mention, "mention"),
        (ActivityEventType::Reply, "reply"),
        (ActivityEventType::ThreadActivity, "thread_activity"),
        (ActivityEventType::KeywordAlert, "keyword_alert"),
        (ActivityEventType::WorkUpdate, "work_update"),
        (ActivityEventType::WorkAssignment, "work_assignment"),
        (ActivityEventType::WorkSla, "work_sla"),
        (ActivityEventType::HuddleStarted, "huddle_started"),
        (ActivityEventType::HuddleMissed, "huddle_missed"),
        (ActivityEventType::EventInvitation, "event_invitation"),
        (ActivityEventType::EventUpdate, "event_update"),
        (ActivityEventType::EventCancelled, "event_cancelled"),
        (ActivityEventType::EventReminder, "event_reminder"),
        (ActivityEventType::PrReviewRequest, "pr_review_request"),
        (
            ActivityEventType::AgentApprovalRequest,
            "agent_approval_request",
        ),
        (
            ActivityEventType::AgentBudgetExceeded,
            "agent_budget_exceeded",
        ),
        (ActivityEventType::MessageReminder, "message_reminder"),
        (
            ActivityEventType::ScheduledMessageDropped,
            "scheduled_message_dropped",
        ),
        (ActivityEventType::TwoFactorLockout, "two_factor_lockout"),
        (ActivityEventType::NewSignIn, "new_sign_in"),
    ];
    for (value, wire) in event_types {
        assert_wire(&value, json!(wire));
    }
    let sources = [
        (ActivitySourceType::Message, "message"),
        (ActivitySourceType::SavedItem, "saved_item"),
        (ActivitySourceType::WorkThreadEvent, "work_thread_event"),
        (ActivitySourceType::BoardSlaNudge, "board_sla_nudge"),
        (ActivitySourceType::HuddleGrant, "huddle_grant"),
        (ActivitySourceType::Event, "event"),
        (ActivitySourceType::AgentApproval, "agent_approval"),
        (ActivitySourceType::AgentBudgetNotice, "agent_budget_notice"),
        (ActivitySourceType::ScheduledMessage, "scheduled_message"),
        (
            ActivitySourceType::TwoFactorCredential,
            "two_factor_credential",
        ),
        (ActivitySourceType::Session, "session"),
    ];
    for (value, wire) in sources {
        assert_wire(&value, json!(wire));
    }
    let tabs = [
        (ActivityTab::All, "all"),
        (ActivityTab::Mentions, "mentions"),
        (ActivityTab::Threads, "threads"),
        (ActivityTab::Events, "events"),
        (ActivityTab::Agents, "agents"),
        (ActivityTab::Github, "github"),
        (ActivityTab::Huddles, "huddles"),
        (ActivityTab::Reminders, "reminders"),
        (ActivityTab::Security, "security"),
    ];
    for (value, wire) in tabs {
        assert_wire(&value, json!(wire));
    }
}

#[test]
fn activity_round_trips() {
    assert_wire(
        &ActivityList {
            items: vec![activity_item()],
            users: vec![user()],
            unread_count: 4,
            unread_revision: 17,
            evaluated_at: "2026-10-10T12:00:00.000000000Z".into(),
            next_cursor: Some("MjAyNi0xMC0wNlQwOToxNTowMS4wMDBafDMwMQ".into()),
        },
        json!({
            "items": [activity_item_wire()],
            "users": [serde_json::to_value(user()).unwrap()],
            "unreadCount": 4, "unreadRevision": 17, "evaluatedAt": "2026-10-10T12:00:00.000000000Z",
            "nextCursor": "MjAyNi0xMC0wNlQwOToxNTowMS4wMDBafDMwMQ",
        }),
    );

    let mut approval = activity_item();
    approval.event_type = ActivityEventType::AgentApprovalRequest;
    approval.state = ActivityState::Handled;
    approval.read_at = Some("2026-10-06T10:00:00.000Z".into());
    approval.handled_at = Some("2026-10-06T10:00:00.000Z".into());
    approval.source.source_type = ActivitySourceType::AgentApproval;
    approval.source.approval_status = Some(AgentApprovalStatus::Approved);
    let wire = serde_json::to_value(&approval).unwrap();
    assert_eq!(wire["source"]["approvalStatus"], "approved");
    assert_eq!(wire["state"], "handled");
    assert_wire(&approval, wire);
    let statuses = [
        (AgentApprovalStatus::Pending, "pending"),
        (AgentApprovalStatus::Approved, "approved"),
        (AgentApprovalStatus::Denied, "denied"),
        (AgentApprovalStatus::Cancelled, "cancelled"),
        (AgentApprovalStatus::Expired, "expired"),
    ];
    for (value, wire) in statuses {
        assert_wire(&value, json!(wire));
    }
    let caps = [
        (AgentBudgetCap::Messages, "messages"),
        (AgentBudgetCap::BoardPosts, "board_posts"),
        (AgentBudgetCap::ExternalActions, "external_actions"),
    ];
    for (value, wire) in caps {
        assert_wire(&value, json!(wire));
    }

    assert_wire(
        &ActivityUnreadCount {
            unread_count: 4,
            unread_revision: 17,
            evaluated_at: "2026-10-10T12:00:00.000000000Z".into(),
        },
        json!({"unreadCount": 4, "unreadRevision": 17, "evaluatedAt": "2026-10-10T12:00:00.000000000Z"}),
    );
    assert_wire(
        &UpdateActivityItem {
            action: ActivityAction::Handled,
        },
        json!({"action": "handled"}),
    );
    let actions = [
        (ActivityAction::Read, "read"),
        (ActivityAction::Unread, "unread"),
        (ActivityAction::Handled, "handled"),
        (ActivityAction::Unhandled, "unhandled"),
    ];
    for (value, wire) in actions {
        assert_wire(&value, json!(wire));
    }
    assert_wire(
        &ActivityItemChanged {
            item: activity_item(),
            unread_count: 3,
            unread_revision: 17,
            evaluated_at: "2026-10-10T12:00:00.000000000Z".into(),
        },
        json!({"item": activity_item_wire(), "unreadCount": 3, "unreadRevision": 17, "evaluatedAt": "2026-10-10T12:00:00.000000000Z"}),
    );
    assert_wire(
        &ActivityItemRemoved {
            id: 301,
            unread_count: 2,
            unread_revision: 17,
            evaluated_at: "2026-10-10T12:00:00.000000000Z".into(),
        },
        json!({"id": 301, "unreadCount": 2, "unreadRevision": 17, "evaluatedAt": "2026-10-10T12:00:00.000000000Z"}),
    );
}

#[test]
fn saved_items_list_and_update() {
    assert_wire(&SavedFilter::All, json!("all"));
    assert_wire(&SavedFilter::InProgress, json!("in_progress"));
    assert_wire(&SavedFilter::Done, json!("done"));

    let message_wire = serde_json::to_value(message()).unwrap();
    assert_wire(
        &SavedItemList {
            items: vec![saved_item()],
            messages: vec![message()],
            users: vec![user()],
            conversations: vec![conversation()],
            next_cursor: None,
        },
        json!({
            "items": [saved_item_wire()],
            "messages": [message_wire],
            "users": [serde_json::to_value(user()).unwrap()],
            "conversations": [conversation_wire()],
            "nextCursor": null,
        }),
    );
    assert_wire(
        &UpdateSavedItem {
            status: SavedStatus::Done,
        },
        json!({"status": "done"}),
    );
    assert_wire(
        &SavedChanged {
            message_id: 9001,
            item: Some(saved_item()),
        },
        json!({"messageId": 9001, "item": saved_item_wire()}),
    );
}

#[test]
fn scheduled_messages_carry_their_state() {
    let states = [
        (ScheduledMessageState::Pending, "pending"),
        (ScheduledMessageState::Sending, "sending"),
        (ScheduledMessageState::Sent, "sent"),
        (ScheduledMessageState::Dropped, "dropped"),
    ];
    for (value, wire) in states {
        assert_wire(&value, json!(wire));
    }
    assert_wire(&ScheduledMessageFilter::Pending, json!("pending"));
    assert_wire(&ScheduledMessageFilter::Past, json!("past"));

    assert_wire(
        &ScheduledMessageList {
            scheduled_messages: vec![scheduled()],
            conversations: vec![ConversationName {
                thread_id: None,
                thread_name: None,
                ..conversation()
            }],
            next_cursor: Some("MjAyNi0xMC0wN1QxMzo1NTowMC4wMDBafDQ".into()),
        },
        json!({
            "scheduledMessages": [scheduled_wire()],
            "conversations": [{
                "roomId": 12,
                "threadId": null,
                "roomKind": "open",
                "roomName": "general",
                "roomIconName": "campfire",
                "threadName": null,
            }],
            "nextCursor": "MjAyNi0xMC0wN1QxMzo1NTowMC4wMDBafDQ",
        }),
    );
    assert_wire(
        &UpdateScheduledMessage {
            markdown_source: None,
            send_at: Some("2026-10-07T14:00:00.000Z".into()),
            reply_to_message_id: None,
        },
        json!({"sendAt": "2026-10-07T14:00:00.000Z"}),
    );
    let explicit_null: UpdateScheduledMessage =
        serde_json::from_value(json!({"markdownSource": null, "sendAt": null})).unwrap();
    assert_eq!(explicit_null.markdown_source, None);
    assert_eq!(explicit_null.send_at, None);
    assert_wire(
        &ScheduledMessageRemoved { id: 4, room_id: 12 },
        json!({"id": 4, "roomId": 12}),
    );
}

#[test]
fn search_round_trips() {
    let mut reply = message();
    reply.thread_id = Some(88);
    let reply_wire = serde_json::to_value(&reply).unwrap();
    assert_wire(
        &SearchResults {
            query: "launch from:@ada has:file".into(),
            chips: vec![
                SearchChip {
                    operator: SearchOperator::From,
                    value: "ada".into(),
                    token: "from:@ada".into(),
                    label: "from: ada".into(),
                    remove_query: "launch has:file".into(),
                },
                SearchChip {
                    operator: SearchOperator::Has,
                    value: "file".into(),
                    token: "has:file".into(),
                    label: "has: file".into(),
                    remove_query: "launch from:@ada".into(),
                },
            ],
            messages: vec![reply],
            users: vec![user()],
            conversations: vec![conversation()],
            next_cursor: Some("MjAyNi0xMC0wNlQwOToxNTowMC4xMjNafDkwMDE".into()),
            sections: vec![SearchSection {
                kind: SearchSectionKind::WorkThreads,
                rows: vec![SearchSectionRow {
                    id: 88,
                    room_id: 12,
                    room_kind: RoomKind::Open,
                    title: "Launch checklist".into(),
                    time: "2026-10-06T10:00:00.000Z".into(),
                    work_status: Some(WorkStatus::InProgress),
                    cancelled: false,
                }],
            }],
        },
        json!({
            "query": "launch from:@ada has:file",
            "chips": [
                {"operator": "from", "value": "ada", "token": "from:@ada", "label": "from: ada", "removeQuery": "launch has:file"},
                {"operator": "has", "value": "file", "token": "has:file", "label": "has: file", "removeQuery": "launch from:@ada"},
            ],
            "messages": [reply_wire],
            "users": [serde_json::to_value(user()).unwrap()],
            "conversations": [conversation_wire()],
            "nextCursor": "MjAyNi0xMC0wNlQwOToxNTowMC4xMjNafDkwMDE",
            "sections": [{"kind": "work_threads", "rows": [{
                "id": 88,
                "roomId": 12,
                "roomKind": "open",
                "title": "Launch checklist",
                "time": "2026-10-06T10:00:00.000Z",
                "workStatus": "in_progress",
                "cancelled": false,
            }]}],
        }),
    );
    let operators = [
        (SearchOperator::From, "from"),
        (SearchOperator::In, "in"),
        (SearchOperator::Has, "has"),
        (SearchOperator::Before, "before"),
        (SearchOperator::After, "after"),
        (SearchOperator::On, "on"),
        (SearchOperator::Is, "is"),
    ];
    for (value, wire) in operators {
        assert_wire(&value, json!(wire));
    }
    assert_wire(&SearchSectionKind::BoardPosts, json!("board_posts"));
    assert_wire(&SearchSectionKind::Events, json!("events"));

    assert_wire(
        &RecentSearchList {
            searches: vec![RecentSearch {
                id: 6,
                query: "launch".into(),
                searched_at: "2026-10-06T10:05:00.000Z".into(),
            }],
        },
        json!({"searches": [{"id": 6, "query": "launch", "searchedAt": "2026-10-06T10:05:00.000Z"}]}),
    );
    assert_wire(
        &RecordSearch {
            query: "launch".into(),
        },
        json!({"query": "launch"}),
    );
}

#[test]
fn sidebar_organisation_requests_round_trip() {
    assert_wire(
        &CreateRoomCategory {
            name: "Projects".into(),
            collapsed: None,
        },
        json!({"name": "Projects"}),
    );
    assert_wire(
        &UpdateRoomCategory {
            name: None,
            collapsed: Some(true),
        },
        json!({"collapsed": true}),
    );
    assert_wire(
        &ReorderRoomCategories {
            category_ids: vec![4, 3],
        },
        json!({"categoryIds": [4, 3]}),
    );
    assert_wire(
        &RoomCategoryList {
            categories: vec![category()],
        },
        json!({"categories": [{"id": 3, "name": "Projects", "collapsed": false, "position": 1}]}),
    );
    assert_wire(&RoomCategoryRemoved { id: 3 }, json!({"id": 3}));
    assert_wire(
        &AssignRoomCategory {
            room_category_id: None,
        },
        json!({"roomCategoryId": null}),
    );
    assert_wire(&MoveFavorite { position: 0 }, json!({"position": 0}));
    assert_wire(&FavoriteList { rows: vec![] }, json!({"rows": []}));
    assert_wire(
        &UpdateInvolvement {
            involvement: Involvement::Muted,
        },
        json!({"involvement": "muted"}),
    );
}

#[test]
fn polls_round_trip() {
    let mut question = message();
    question.poll = Some(poll());
    let wire = serde_json::to_value(&question).unwrap();
    assert_eq!(wire["poll"], poll_wire());
    assert_wire(&question, wire);

    let mut anonymous = poll();
    anonymous.anonymous = true;
    for option in &mut anonymous.options {
        option.voter_ids.clear();
    }
    let mut anonymous_wire = poll_wire();
    anonymous_wire["anonymous"] = json!(true);
    anonymous_wire["options"][0]["voterIds"] = json!([]);
    anonymous_wire["options"][1]["voterIds"] = json!([]);
    assert_wire(
        &PollResults {
            poll: anonymous,
            my_option_ids: vec![51],
        },
        json!({"poll": anonymous_wire, "myOptionIds": [51]}),
    );
    assert_wire(
        &CreatePoll {
            client_message_id: "0192f0c4-7e8a-7b3c-9d0a-6f3b2d1e8c12".into(),
            thread_id: None,
            question: "Lunch?".into(),
            options: vec!["Tacos".into(), "Pizza".into()],
            multiple: false,
            anonymous: false,
            closes_at: None,
            option_media: None,
        },
        json!({"clientMessageId": "0192f0c4-7e8a-7b3c-9d0a-6f3b2d1e8c12", "threadId": null, "question": "Lunch?", "options": ["Tacos", "Pizza"], "multiple": false, "anonymous": false, "closesAt": null}),
    );
    assert_wire(&VotePoll { option_ids: vec![] }, json!({"optionIds": []}));
}

#[test]
fn message_cards_are_tagged_by_kind() {
    let cards = vec![
        MessageCard::Drive(DriveFileCard {
            file_id: "1AbC".into(),
            url: "https://drive.google.com/open?id=1AbC".into(),
        }),
        MessageCard::Github(GithubCardRef {
            pull_request_id: 14,
            owner: "Smart-Data-Ohio".into(),
            repo: "smartfire".into(),
            number: 280,
            url: "https://github.com/Smart-Data-Ohio/smartfire/pull/280".into(),
        }),
        MessageCard::X(XPostCard {
            fetch: CardFetch::Loaded,
            post_id: "1843000000000000001".into(),
            url: "https://x.com/ada/status/1843000000000000001".into(),
            author_name: "Ada".into(),
            author_handle: Some("ada".into()),
            author_avatar_url: Some("https://pbs.twimg.com/ada.jpg".into()),
            text: Some("Shipped".into()),
            posted_at: Some("2026-10-06T08:00:00.000Z".into()),
            replies: Some(2),
            reposts: Some(1),
            likes: Some(30),
            media: vec![XMedia {
                kind: XMediaKind::Photo,
                url: "https://pbs.twimg.com/media/1.jpg".into(),
                thumbnail_url: None,
                width: Some(1200),
                height: Some(675),
                alt: None,
            }],
            quote: Some(XQuote {
                url: None,
                author_name: Some("Grace".into()),
                author_handle: Some("grace".into()),
                text: Some("Nice".into()),
            }),
        }),
        MessageCard::Event(EventCard {
            event_id: 21,
            room_id: 12,
            title: "Retro".into(),
            organizer_id: 7,
            starts_at: "2026-10-09T15:00:00.000Z".into(),
            ends_at: Some("2026-10-09T16:00:00.000Z".into()),
            time_zone: "America/New_York".into(),
            recurring: true,
            cancelled: false,
            venue_room_id: Some(30),
            venue_name: Some("Lounge".into()),
            meet_link: None,
        }),
        MessageCard::Fizzy(FizzyCardRef {
            fizzy_card_id: 2,
            account_id: "897362094".into(),
            number: 17,
            url: "https://app.fizzy.do/897362094/cards/17".into(),
        }),
        MessageCard::Quote(QuoteCard {
            reference_id: 61,
            preview: Some(QuotePreview {
                message_id: 8990,
                room_id: 12,
                thread_id: None,
                creator_id: 8,
                author_name: "Grace Hopper".into(),
                room_label: "general".into(),
                excerpt: "The deploy is green".into(),
                created_at: "2026-10-06T08:30:00.000Z".into(),
            }),
        }),
        MessageCard::Quote(QuoteCard {
            reference_id: 62,
            preview: None,
        }),
        MessageCard::Linkedin(LinkedinCard {
            url: "https://www.linkedin.com/posts/ada_activity-1".into(),
            title: None,
            description: None,
            image_url: None,
            embed_url: Some("https://www.linkedin.com/embed/feed/update/urn:li:activity:1".into()),
        }),
        MessageCard::Link(LinkCard {
            url: "https://example.com/post".into(),
            site_name: Some("Example".into()),
            title: Some("A post".into()),
            description: None,
            image_url: Some("https://example.com/og.png".into()),
        }),
    ];
    let wire = json!([
        {"kind": "drive", "data": {"fileId": "1AbC", "url": "https://drive.google.com/open?id=1AbC"}},
        {"kind": "github", "data": {
            "pullRequestId": 14,
            "owner": "Smart-Data-Ohio",
            "repo": "smartfire",
            "number": 280,
            "url": "https://github.com/Smart-Data-Ohio/smartfire/pull/280",
        }},
        {"kind": "x", "data": {
            "fetch": "loaded",
            "postId": "1843000000000000001",
            "url": "https://x.com/ada/status/1843000000000000001",
            "authorName": "Ada",
            "authorHandle": "ada",
            "authorAvatarUrl": "https://pbs.twimg.com/ada.jpg",
            "text": "Shipped",
            "postedAt": "2026-10-06T08:00:00.000Z",
            "replies": 2,
            "reposts": 1,
            "likes": 30,
            "media": [{"kind": "photo", "url": "https://pbs.twimg.com/media/1.jpg", "thumbnailUrl": null, "width": 1200, "height": 675, "alt": null}],
            "quote": {"url": null, "authorName": "Grace", "authorHandle": "grace", "text": "Nice"},
        }},
        {"kind": "event", "data": {
            "eventId": 21,
            "roomId": 12,
            "title": "Retro",
            "organizerId": 7,
            "startsAt": "2026-10-09T15:00:00.000Z",
            "endsAt": "2026-10-09T16:00:00.000Z",
            "timeZone": "America/New_York",
            "recurring": true,
            "cancelled": false,
            "venueRoomId": 30,
            "venueName": "Lounge",
            "meetLink": null,
        }},
        {"kind": "fizzy", "data": {
            "fizzyCardId": 2,
            "accountId": "897362094",
            "number": 17,
            "url": "https://app.fizzy.do/897362094/cards/17",
        }},
        {"kind": "quote", "data": {"referenceId": 61, "preview": {
            "messageId": 8990,
            "roomId": 12,
            "threadId": null,
            "creatorId": 8,
            "authorName": "Grace Hopper",
            "roomLabel": "general",
            "excerpt": "The deploy is green",
            "createdAt": "2026-10-06T08:30:00.000Z",
        }}},
        {"kind": "quote", "data": {"referenceId": 62, "preview": null}},
        {"kind": "linkedin", "data": {
            "url": "https://www.linkedin.com/posts/ada_activity-1",
            "title": null,
            "description": null,
            "imageUrl": null,
            "embedUrl": "https://www.linkedin.com/embed/feed/update/urn:li:activity:1",
        }},
        {"kind": "link", "data": {
            "url": "https://example.com/post",
            "siteName": "Example",
            "title": "A post",
            "description": null,
            "imageUrl": "https://example.com/og.png",
        }},
    ]);
    let mut carded = message();
    carded.cards = cards.clone();
    let carded_wire = serde_json::to_value(&carded).unwrap();
    assert_eq!(carded_wire["cards"], wire);
    assert_wire(&carded, carded_wire);

    assert_wire(
        &MessageCards {
            message_id: 9001,
            room_id: 12,
            thread_id: None,
            cards,
            as_of: "2026-10-06T09:31:00.000Z".into(),
        },
        json!({"messageId": 9001, "roomId": 12, "threadId": null, "cards": wire, "asOf": "2026-10-06T09:31:00.000Z"}),
    );
    assert_wire(&QuotePreviewResult::Hidden, json!({"state": "hidden"}));
    assert_wire(
        &QuotePreviewResult::Loaded(QuotePreview {
            message_id: 70,
            room_id: 3,
            thread_id: Some(88),
            creator_id: 7,
            author_name: "Ada Lovelace".into(),
            room_label: "a direct message".into(),
            excerpt: "See you there".into(),
            created_at: "2026-10-05T18:00:00.000Z".into(),
        }),
        json!({
            "state": "loaded",
            "messageId": 70,
            "roomId": 3,
            "threadId": 88,
            "creatorId": 7,
            "authorName": "Ada Lovelace",
            "roomLabel": "a direct message",
            "excerpt": "See you there",
            "createdAt": "2026-10-05T18:00:00.000Z",
        }),
    );
    assert_wire(&CardFetch::Loading, json!("loading"));
    assert_wire(&CardFetch::Failed, json!("failed"));
    assert_wire(&XMediaKind::Gif, json!("gif"));
}

#[test]
fn event_attendance_round_trips() {
    assert_wire(
        &EventAttendance {
            event_id: 21,
            response: Some(AttendanceResponse::Maybe),
            going_count: 4,
            maybe_count: 1,
            declined_count: 0,
            respondable: true,
            can_apply_to_future: true,
        },
        json!({
            "eventId": 21,
            "response": "maybe",
            "goingCount": 4,
            "maybeCount": 1,
            "declinedCount": 0,
            "respondable": true,
            "canApplyToFuture": true,
        }),
    );
    assert_wire(
        &RespondToEvent {
            response: AttendanceResponse::Declined,
            apply_to_future: false,
        },
        json!({"response": "declined", "applyToFuture": false}),
    );
    assert_wire(&AttendanceResponse::Going, json!("going"));
}

#[test]
fn github_previews_are_tagged_by_state() {
    assert_wire(&GithubPullRequestCard::Hidden, json!({"state": "hidden"}));
    assert_wire(&GithubPullRequestCard::Loading, json!({"state": "loading"}));
    assert_wire(
        &GithubPullRequestCard::Failed {
            message: "Not Found".into(),
        },
        json!({"state": "failed", "message": "Not Found"}),
    );
    assert_wire(
        &GithubPullRequestCard::Loaded(Box::new(GithubPullRequest {
            owner: "Smart-Data-Ohio".into(),
            repo: "smartfire".into(),
            number: 280,
            title: "Build the S2 client".into(),
            url: "https://github.com/Smart-Data-Ohio/smartfire/pull/280".into(),
            status: GithubPullRequestStatus::Merged,
            author_login: Some("ada".into()),
            author_avatar_url: None,
            base_branch: Some("main".into()),
            head_branch: Some("frontend/s2-ui".into()),
            review: Some(GithubReview::ChangesRequested),
            checks: Some(GithubChecks::Passing),
            github_updated_at: Some("2026-10-06T12:00:00.000Z".into()),
            discussion_thread_id: None,
            files: Some(GithubChangedFiles {
                files: vec![GithubChangedFile {
                    filename: "frontend/src/app.tsx".into(),
                    status: Some("modified".into()),
                    additions: 12,
                    deletions: 3,
                }],
                total_count: 41,
            }),
        })),
        json!({
            "state": "loaded",
            "owner": "Smart-Data-Ohio",
            "repo": "smartfire",
            "number": 280,
            "title": "Build the S2 client",
            "url": "https://github.com/Smart-Data-Ohio/smartfire/pull/280",
            "status": "merged",
            "authorLogin": "ada",
            "authorAvatarUrl": null,
            "baseBranch": "main",
            "headBranch": "frontend/s2-ui",
            "review": "changes_requested",
            "checks": "passing",
            "githubUpdatedAt": "2026-10-06T12:00:00.000Z",
            "discussionThreadId": null,
            "files": {
                "files": [{"filename": "frontend/src/app.tsx", "status": "modified", "additions": 12, "deletions": 3}],
                "totalCount": 41,
            },
        }),
    );
    assert_wire(&GithubPullRequestStatus::Draft, json!("draft"));
    assert_wire(&GithubReview::ReviewRequired, json!("review_required"));
    assert_wire(&GithubChecks::Failing, json!("failing"));
}

#[test]
fn fizzy_previews_are_tagged_by_state() {
    assert_wire(
        &FizzyCardPreview::NotConnected,
        json!({"state": "not_connected"}),
    );
    assert_wire(&FizzyCardPreview::NotFound, json!({"state": "not_found"}));
    assert_wire(&FizzyCardPreview::Loading, json!({"state": "loading"}));
    assert_wire(
        &FizzyCardPreview::Failed {
            message: "timeout".into(),
        },
        json!({"state": "failed", "message": "timeout"}),
    );
    assert_wire(
        &FizzyCardPreview::Loaded(FizzyCard {
            title: "Ship S3".into(),
            url: "https://app.fizzy.do/897362094/cards/17".into(),
            board_name: Some("Front end".into()),
            status: FizzyCardStatus::Column,
            column_name: Some("Doing".into()),
            assignees: vec![FizzyAssignee {
                name: "Ada".into(),
                avatar_url: None,
            }],
            has_more_assignees: false,
            tags: vec!["s3".into()],
            steps_total: 4,
            steps_completed: 1,
            last_active_at: None,
        }),
        json!({
            "state": "loaded",
            "title": "Ship S3",
            "url": "https://app.fizzy.do/897362094/cards/17",
            "boardName": "Front end",
            "status": "column",
            "columnName": "Doing",
            "assignees": [{"name": "Ada", "avatarUrl": null}],
            "hasMoreAssignees": false,
            "tags": ["s3"],
            "stepsTotal": 4,
            "stepsCompleted": 1,
            "lastActiveAt": null,
        }),
    );
    assert_wire(&FizzyCardStatus::Triage, json!("triage"));
}

#[test]
fn s3_sync_events_round_trip() {
    let cases = [
        (
            "user",
            SyncPayload::ActivityItem(ActivityItemChanged {
                item: activity_item(),
                unread_count: 3,
                unread_revision: 17,
            evaluated_at: "2026-10-10T12:00:00.000000000Z".into(),
            }),
            json!({"type": "activity.item", "data": {"item": activity_item_wire(), "unreadCount": 3, "unreadRevision": 17, "evaluatedAt": "2026-10-10T12:00:00.000000000Z"}}),
        ),
        (
            "user",
            SyncPayload::ActivityRemoved(ActivityItemRemoved {
                id: 301,
                unread_count: 2,
                unread_revision: 17,
            evaluated_at: "2026-10-10T12:00:00.000000000Z".into(),
            }),
            json!({"type": "activity.removed", "data": {"id": 301, "unreadCount": 2, "unreadRevision": 17, "evaluatedAt": "2026-10-10T12:00:00.000000000Z"}}),
        ),
        (
            "user",
            SyncPayload::ScheduledChanged(scheduled()),
            json!({"type": "scheduled.changed", "data": scheduled_wire()}),
        ),
        (
            "user",
            SyncPayload::ScheduledRemoved(ScheduledMessageRemoved { id: 4, room_id: 12 }),
            json!({"type": "scheduled.removed", "data": {"id": 4, "roomId": 12}}),
        ),
        (
            "user",
            SyncPayload::SidebarCategoryUpserted(category()),
            json!({"type": "sidebar.category.upserted", "data": {"id": 3, "name": "Projects", "collapsed": false, "position": 1}}),
        ),
        (
            "user",
            SyncPayload::SidebarCategoryRemoved(RoomCategoryRemoved { id: 3 }),
            json!({"type": "sidebar.category.removed", "data": {"id": 3}}),
        ),
        (
            "room:12",
            SyncPayload::PollUpdated(PollUpdated {
                room_id: 12,
                thread_id: None,
                poll: poll(),
            }),
            json!({"type": "poll.updated", "data": {"roomId": 12, "threadId": null, "poll": poll_wire()}}),
        ),
        (
            "user",
            SyncPayload::PollBallot(PollBallot {
                poll_id: 5,
                message_id: 9001,
                room_id: 12,
                thread_id: None,
                my_option_ids: vec![51],
                as_of: "2026-10-06T09:30:00.000Z".into(),
            }),
            json!({"type": "poll.ballot", "data": {"pollId": 5, "messageId": 9001, "roomId": 12, "threadId": null, "myOptionIds": [51], "asOf": "2026-10-06T09:30:00.000Z"}}),
        ),
        (
            "room:12",
            SyncPayload::MessageCards(MessageCards {
                message_id: 9001,
                room_id: 12,
                thread_id: None,
                cards: vec![],
                as_of: "2026-10-06T09:31:00.000Z".into(),
            }),
            json!({"type": "message.cards", "data": {"messageId": 9001, "roomId": 12, "threadId": null, "cards": [], "asOf": "2026-10-06T09:31:00.000Z"}}),
        ),
        (
            "user",
            SyncPayload::SavedChanged(SavedChanged {
                message_id: 9001,
                item: Some(saved_item()),
            }),
            json!({"type": "saved.changed", "data": {"messageId": 9001, "item": saved_item_wire()}}),
        ),
    ];
    for (topic, payload, wire) in cases {
        let event = SyncEvent {
            seq: 7,
            topic: topic.into(),
            payload,
        };
        let mut expected = wire;
        expected["seq"] = json!(7);
        expected["topic"] = json!(topic);
        assert_wire(&event, expected);
    }
}
