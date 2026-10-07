//! The S4 agent endpoints on `/api/v1` (`campfire_api::agents`): the directory, a profile, the
//! approvals page and a decision, each with the classic page's audience; the agent identity on
//! `User`; and the twins `agent.status`, `agent.steps` and `approval.updated`, with the classic
//! frames of the broadcasts they sit beside byte for byte the same with the sync engine on.

use std::time::Duration;

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use campfire_db::models::agent_step::{AgentStep, NewAgentStep, StepParentChange};
use campfire_db::{Agent, AgentApproval, Event, NewApproval};
use serde_json::json;

use super::api_tests::{Sync, app, get, json_body, parse, serve, tag};
use crate::controllers::presenters::test_support::{
    ALL_TALK, BENDER, DAVID, JASON, KEVIN, TestApp,
};

/// Bender's agent: a workspace agent David owns. Bender is in All Talk, Archive, the release
/// board and a direct room with Kevin.
const AGENT: i64 = 773018776;
const ARCHIVE: i64 = 699448327;
const DESIGNERS: i64 = 654632876;
/// The seed's pending request (in Designers), with inbox items for David and Jason.
const SEEDED_APPROVAL: i64 = 1;
/// A member in no room at all.
const LOU: i64 = 773523958;
/// Designers' thread.
const THREAD: i64 = 1;
/// One of Bender's own messages, in All Talk.
const BENDERS_MESSAGE: i64 = 935961918;

/// Creates a pending request for Bender's agent, overdue when `overdue`, answering its id.
async fn approval(a: &TestApp, action: &'static str, overdue: bool) -> i64 {
    a.db()
        .write(move |tx| {
            let approval = AgentApproval::create(
                tx,
                NewApproval {
                    agent_id: AGENT,
                    room_id: Some(ALL_TALK),
                    action: action.into(),
                    summary: format!("Run {action}"),
                    ..Default::default()
                },
            )?;
            if overdue {
                tx.conn().execute(
                    "UPDATE agent_approvals SET expires_at = ? WHERE id = ?",
                    rusqlite::params![
                        tx.now().ago(jiff::SignedDuration::from_mins(1)),
                        approval.id
                    ],
                )?;
            }
            Ok(approval.id)
        })
        .await
        .unwrap()
}

fn validation_fields(reply: &crate::controllers::presenters::test_support::Reply) -> Vec<String> {
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.text()
    );
    let api::ApiError::Validation { fields, .. } = parse::<api::ApiErrorResponse>(reply).error
    else {
        panic!("{}", reply.text())
    };
    fields.into_keys().collect()
}

#[tokio::test]
async fn the_directory_and_users_carry_the_agent_identity() {
    let Some(a) = app(true).await else { return };
    let mut kevin = a.sign_in(KEVIN).await;
    let response = kevin.send(get("/api/v1/agents")).await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    assert_eq!(response.header("cache-control"), Some("no-store"));
    let directory: api::AgentDirectory = parse(&response);
    let row = directory
        .agents
        .iter()
        .find(|row| row.agent_id == AGENT)
        .expect("Bender's agent");
    assert_eq!(
        (row.user_id, row.owner_id, row.kind, row.suspended),
        (BENDER, Some(DAVID), api::AgentKind::Workspace, false)
    );
    let bender = directory
        .users
        .iter()
        .find(|user| user.id == BENDER)
        .expect("the bot user");
    let badge = bender.agent.as_ref().expect("an agent badge");
    assert_eq!(badge.agent_id, AGENT);
    assert!(directory.users.iter().any(|user| user.id == DAVID));
    // A person who isn't an agent has no badge.
    let david = directory
        .users
        .iter()
        .find(|user| user.id == DAVID)
        .unwrap();
    assert_eq!(david.agent, None);

    // The badge rides on every `User`, so a member list shows it too.
    let members: api::MemberList = parse(
        &kevin
            .send(get(&format!("/api/v1/rooms/{ARCHIVE}/members")))
            .await,
    );
    let bender = members.users.iter().find(|user| user.id == BENDER).unwrap();
    assert_eq!(
        bender.agent.as_ref().map(|badge| badge.agent_id),
        Some(AGENT)
    );
}

#[tokio::test]
async fn a_profile_shows_management_and_grants_only_to_its_managers() {
    let Some(a) = app(true).await else { return };
    let path = format!("/api/v1/agents/{AGENT}");
    let mut kevin = a.sign_in(KEVIN).await;
    let response = kevin.send(get(&path)).await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    let profile: api::AgentProfile = parse(&response);
    assert_eq!(
        (profile.agent.agent_id, profile.agent.user_id),
        (AGENT, BENDER)
    );
    assert_eq!((&profile.grants, &profile.management), (&None, &None));
    // Only the rooms Kevin shares, and a count of the others.
    let rooms = profile
        .rooms
        .iter()
        .map(|room| room.room_id)
        .collect::<Vec<_>>();
    assert!(rooms.contains(&ARCHIVE), "{rooms:?}");
    assert!(!rooms.contains(&ALL_TALK), "{rooms:?}");
    assert!(profile.hidden_room_count >= 1, "{profile:?}");
    assert!(profile.users.iter().any(|user| user.id == BENDER));
    assert!(profile.users.iter().any(|user| user.id == DAVID));

    let mut david = a.sign_in(DAVID).await;
    let profile: api::AgentProfile = parse(&david.send(get(&path)).await);
    let management = profile.management.expect("the owner manages it");
    assert_eq!(management.budget_usage.len(), 3);
    assert!(profile.grants.is_some());
    assert!(profile.rooms.iter().any(|room| room.room_id == ALL_TALK));

    // Jason administers the workspace: he manages it too.
    let mut jason = a.sign_in(JASON).await;
    let profile: api::AgentProfile = parse(&jason.send(get(&path)).await);
    assert!(profile.management.is_some() && profile.grants.is_some());

    for missing in ["/api/v1/agents/999999", "/api/v1/agents/nope"] {
        let response = kevin.send(get(missing)).await;
        assert_eq!(response.status, StatusCode::NOT_FOUND, "{missing}");
        assert_eq!(tag(&response), "NotFound");
    }
}

#[tokio::test]
async fn the_approvals_page_settles_overdue_requests_and_pages_by_cursor() {
    let Some(a) = app(true).await else { return };
    let path = format!("/api/v1/agents/{AGENT}/approvals");
    // Kevin neither administers nor owns it: as in classic, a 404.
    let mut kevin = a.sign_in(KEVIN).await;
    let response = kevin.send(get(&path)).await;
    assert_eq!(
        response.status,
        StatusCode::NOT_FOUND,
        "{}",
        response.text()
    );

    let overdue = approval(&a, "deploy.overdue", true).await;
    let mut jason = a.sign_in(JASON).await;
    let response = jason.send(get(&path)).await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    let page: api::AgentApprovalPage = parse(&response);
    assert_eq!(page.next_cursor, None);
    let ids = page
        .approvals
        .iter()
        .map(|card| card.id)
        .collect::<Vec<_>>();
    assert_eq!(ids, vec![overdue, SEEDED_APPROVAL], "newest first");
    let card = &page.approvals[0];
    assert_eq!(card.status, api::AgentApprovalStatus::Expired);
    assert_eq!((card.agent_user_id, card.room_id), (BENDER, Some(ALL_TALK)));
    assert_eq!(card.room_name.as_deref(), Some("All Talk"));
    assert!(page.users.iter().any(|user| user.id == BENDER));
    // Listing wrote the expiry: the row and its inbox items are settled.
    let (status, open_items) = a
        .db()
        .read(move |conn| {
            Ok((
                conn.query_row(
                    "SELECT status FROM agent_approvals WHERE id = ?",
                    [overdue],
                    |row| row.get::<_, String>(0),
                )?,
                conn.query_row(
                    "SELECT COUNT(*) FROM activity_items WHERE source_type = 'AgentApproval' AND source_id = ? AND handled_at IS NULL",
                    [overdue],
                    |row| row.get::<_, i64>(0),
                )?,
            ))
        })
        .await
        .unwrap();
    assert_eq!((status.as_str(), open_items), ("expired", 0));

    // The status filter.
    let pending: api::AgentApprovalPage =
        parse(&jason.send(get(&format!("{path}?status=pending"))).await);
    assert_eq!(
        pending
            .approvals
            .iter()
            .map(|card| card.id)
            .collect::<Vec<_>>(),
        vec![SEEDED_APPROVAL]
    );
    // A pending card: Jason may approve and deny; the seeded one isn't admin-only.
    let card = &pending.approvals[0];
    assert_eq!(
        (card.approvable, card.deniable, card.admin_only),
        (true, true, false)
    );

    // 50 a page, and the cursor picks up after the last.
    for _ in 0..50 {
        approval(&a, "deploy.more", false).await;
    }
    let first: api::AgentApprovalPage = parse(&jason.send(get(&path)).await);
    assert_eq!(first.approvals.len(), 50);
    let cursor = first.next_cursor.clone().expect("a second page");
    let second: api::AgentApprovalPage =
        parse(&jason.send(get(&format!("{path}?before={cursor}"))).await);
    assert_eq!(second.next_cursor, None);
    let mut all = first
        .approvals
        .iter()
        .chain(&second.approvals)
        .map(|card| card.id)
        .collect::<Vec<_>>();
    assert_eq!(all.len(), 52);
    assert!(all.windows(2).all(|pair| pair[0] > pair[1]), "{all:?}");
    all.dedup();
    assert_eq!(all.len(), 52);

    let response = jason.send(get(&format!("{path}?before=garbage"))).await;
    assert_eq!(validation_fields(&response), vec!["before".to_string()]);
}

#[tokio::test]
async fn a_decision_goes_through_the_classic_checks_and_tells_the_other_deciders() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let path = format!("/api/v1/agent_approvals/{SEEDED_APPROVAL}");
    let jason = a.sign_in(JASON).await;
    let mut jasons_tab = Sync::connect(addr, &jason.cookie_header(), &[]).await;
    jasons_tab.welcome().await;

    // Kevin may not decide it: a 404, as in classic.
    let mut kevin = a.sign_in(KEVIN).await;
    let response = kevin
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"decision": "approved"}),
        ))
        .await;
    assert_eq!(
        response.status,
        StatusCode::NOT_FOUND,
        "{}",
        response.text()
    );

    let mut david = a.sign_in(DAVID).await;
    let response = david
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"decision": "maybe"}),
        ))
        .await;
    assert_eq!(validation_fields(&response), vec!["decision".to_string()]);

    let response = david
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"decision": "approved", "note": "Ship it"}),
        ))
        .await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    let card: api::AgentApproval = parse(&response);
    assert_eq!(
        (
            card.status,
            card.decided_by_id,
            card.decision_note.as_deref()
        ),
        (
            api::AgentApprovalStatus::Approved,
            Some(DAVID),
            Some("Ship it")
        )
    );

    // Jason received the request's inbox item: his card updates, built for him.
    let event = jasons_tab
        .until(
            |event| matches!(&event.payload, api::SyncPayload::ApprovalUpdated(_)),
            |_| false,
        )
        .await;
    assert_eq!(event.topic, "user");
    let api::SyncPayload::ApprovalUpdated(updated) = event.payload else {
        unreachable!()
    };
    assert_eq!(updated.approval.id, SEEDED_APPROVAL);
    assert_eq!(updated.approval.status, api::AgentApprovalStatus::Approved);
    assert_eq!(updated.approval.room_name.as_deref(), Some("Designers"));
    let people = updated.users.iter().map(|user| user.id).collect::<Vec<_>>();
    assert!(
        people.contains(&BENDER) && people.contains(&DAVID),
        "{people:?}"
    );

    // Deciding again fails the classic way.
    let response = david
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"decision": "denied"}),
        ))
        .await;
    assert_eq!(validation_fields(&response), vec!["base".to_string()]);
    server.abort();
}

#[tokio::test]
async fn only_an_administrator_approves_a_github_action() {
    let Some(a) = app(true).await else { return };
    // Make Kevin the owner: he may deny but not approve an admin-only action.
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE agents SET owner_id = ? WHERE id = ?",
                [KEVIN, AGENT],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let id = approval(&a, "github.comment", false).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let page: api::AgentApprovalPage = parse(
        &kevin
            .send(get(&format!(
                "/api/v1/agents/{AGENT}/approvals?status=pending"
            )))
            .await,
    );
    let card = page.approvals.iter().find(|card| card.id == id).unwrap();
    assert_eq!(
        (card.admin_only, card.approvable, card.deniable),
        (true, false, true)
    );
    // Kevin isn't in All Talk: no room name for him.
    assert_eq!(card.room_name, None);
    let path = format!("/api/v1/agent_approvals/{id}");
    let response = kevin
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"decision": "approved"}),
        ))
        .await;
    assert_eq!(
        response.status,
        StatusCode::FORBIDDEN,
        "{}",
        response.text()
    );
    assert_eq!(tag(&response), "Forbidden");
    let response = kevin
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"decision": "denied"}),
        ))
        .await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    assert_eq!(
        parse::<api::AgentApproval>(&response).status,
        api::AgentApprovalStatus::Denied
    );
}

fn agent_status(event: &api::SyncEvent) -> bool {
    matches!(&event.payload, api::SyncPayload::AgentStatus(changed) if changed.agent_id == AGENT)
}

#[tokio::test]
async fn agent_status_tells_everyone_and_working_presence_only_its_room_mates() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let kevin = a.sign_in(KEVIN).await;
    let lou = a.sign_in(LOU).await;
    let mut kevins = Sync::connect(addr, &kevin.cookie_header(), &[]).await;
    kevins.welcome().await;
    let mut lous = Sync::connect(addr, &lou.cookie_header(), &[]).await;
    lous.welcome().await;

    a.db()
        .write(|tx| {
            let mut agent = Agent::find(tx.conn(), AGENT)?.unwrap();
            agent.set_working_presence(tx, Some("Reviewing the deploy"))
        })
        .await
        .unwrap();
    let api::SyncPayload::AgentStatus(seen) = kevins.until(agent_status, |_| false).await.payload
    else {
        unreachable!()
    };
    assert_eq!(
        seen.working_presence.as_deref(),
        Some("Reviewing the deploy")
    );
    assert!(seen.working_presence_expires_at.is_some());
    let event = lous.until(agent_status, |_| false).await;
    assert_eq!(event.topic, "user");
    let api::SyncPayload::AgentStatus(seen) = event.payload else {
        unreachable!()
    };
    assert_eq!(
        (seen.working_presence, seen.working_presence_expires_at),
        (None, None)
    );
    assert_eq!(seen.user_id, BENDER);

    // Suspension has no classic frame, but an `agent.status` all the same.
    a.db()
        .write(|tx| {
            tx.conn()
                .execute("DELETE FROM agent_grants WHERE agent_id = ?", [AGENT])?;
            let mut agent = Agent::find(tx.conn(), AGENT)?.unwrap();
            agent.suspended_at = Some(tx.now());
            agent.save(tx)
        })
        .await
        .unwrap();
    let api::SyncPayload::AgentStatus(seen) = kevins.until(agent_status, |_| false).await.payload
    else {
        unreachable!()
    };
    assert!(seen.suspended);
    server.abort();
}

#[tokio::test]
async fn agent_steps_reach_the_parents_conversation_and_the_message() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let topics = [format!("room:{ALL_TALK}"), format!("thread:{THREAD}")];
    let mut sync = Sync::connect(addr, &david.cookie_header(), &topics).await;
    sync.welcome().await;
    // Steps go on the agent's own messages, and on work it owns.
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE channel_threads SET work_status = 'in_progress', work_owner_id = ? WHERE id = ?",
                [BENDER, THREAD],
            )?;
            Ok(())
        })
        .await
        .unwrap();

    for (message_id, thread_id) in [(Some(BENDERS_MESSAGE), None), (None, Some(THREAD))] {
        a.db()
            .write(move |tx| {
                AgentStep::create(
                    tx,
                    NewAgentStep {
                        agent_id: AGENT,
                        message_id,
                        channel_thread_id: thread_id,
                        name: "Fetch the logs".into(),
                        status: "done".into(),
                        duration_ms: Some(420),
                        ..Default::default()
                    },
                )?;
                tx.emit_after_commit(Event::broadcast(&StepParentChange {
                    message_id,
                    thread_id,
                }));
                Ok(())
            })
            .await
            .unwrap();
        let event = sync
            .until(
                |event| matches!(&event.payload, api::SyncPayload::AgentSteps(_)),
                |_| false,
            )
            .await;
        let api::SyncPayload::AgentSteps(changed) = event.payload else {
            unreachable!()
        };
        let expected_topic = match thread_id {
            Some(thread) => format!("thread:{thread}"),
            None => format!("room:{ALL_TALK}"),
        };
        assert_eq!(event.topic, expected_topic);
        assert_eq!(
            (changed.message_id, changed.thread_id),
            (message_id, thread_id)
        );
        let step = changed.steps.last().expect("the new step");
        assert_eq!(
            (step.name.as_str(), step.status, step.duration_ms),
            ("Fetch the logs", api::AgentStepStatus::Done, Some(420))
        );
        if thread_id.is_some() {
            assert_eq!(changed.room_id, DESIGNERS);
        }
    }

    // The message carries its steps on every read.
    let page: api::MessagePage = parse(
        &david
            .send(get(&format!(
                "/api/v1/rooms/{ALL_TALK}/messages?around={BENDERS_MESSAGE}"
            )))
            .await,
    );
    let message = page
        .messages
        .iter()
        .find(|message| message.id == BENDERS_MESSAGE)
        .unwrap();
    assert!(
        message
            .steps
            .iter()
            .any(|step| step.name == "Fetch the logs")
    );
    server.abort();
}

/// The classic frames of an agent's status change, an approval decision and a step change.
async fn classic_agent_frames(spa: bool) -> Option<Vec<(String, String)>> {
    use crate::controllers::presenters::test_support::SEED_NOW;
    use campfire_kit::clock::FrozenClock;
    let clock = std::sync::Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
    let env: &[(&str, &str)] = if spa { &[("SPA_ENABLED", "1")] } else { &[] };
    let a = TestApp::boot_seed_with_env("default", clock, env).await?;
    // The classic pages open: the agents directory and David's inbox.
    let (mut client, cable) =
        crate::controllers::messages::attachment_processing_tests::subscribe(&a).await;
    for channel in ["AgentsChannel", "ActivityChannel"] {
        let identifier = crate::channels::tests::support::identifier(json!({ "channel": channel }));
        client.confirm(&identifier).await;
    }
    let mut david = a.sign_in(DAVID).await;
    david.authenticity_token().await;
    let (_sync, server) = if spa {
        let (addr, server) = serve(&a).await;
        let topics = [format!("room:{ALL_TALK}")];
        let mut sync = Sync::connect(addr, &david.cookie_header(), &topics).await;
        sync.welcome().await;
        (Some(sync), Some(server))
    } else {
        (None, None)
    };
    assert_eq!(a.booted.app.cable.sync_wanted(), spa);
    let capture = a.publications();
    capture.take();

    a.db()
        .write(|tx| {
            let mut agent = Agent::find(tx.conn(), AGENT)?.unwrap();
            agent.status = "working".into();
            agent.status_note = Some("Frame parity".into());
            agent.save(tx)?;
            agent.set_working_presence(tx, Some("Parity presence"))?;
            AgentStep::create(
                tx,
                NewAgentStep {
                    agent_id: AGENT,
                    message_id: Some(BENDERS_MESSAGE),
                    name: "Parity step".into(),
                    ..Default::default()
                },
            )?;
            tx.emit_after_commit(Event::broadcast(&StepParentChange {
                message_id: Some(BENDERS_MESSAGE),
                thread_id: None,
            }));
            Ok(())
        })
        .await
        .unwrap();
    let reply = david
        .write(
            crate::controllers::presenters::test_support::Req::new(
                Method::PATCH,
                &format!("/agent_approvals/{SEEDED_APPROVAL}"),
            )
            .header("accept", "text/html")
            .form(&[("decision", "denied")]),
        )
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.text());

    let mut frames = Vec::new();
    let mut quiet = 0;
    while quiet < 10 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let more = capture.take();
        quiet = if more.is_empty() { quiet + 1 } else { 0 };
        frames.extend(more);
    }
    cable.abort();
    if let Some(server) = server {
        server.abort();
    }
    Some(frames)
}

#[tokio::test]
async fn the_agent_frames_are_the_same_with_the_sync_engine_on() {
    let (Some(off), Some(on)) = (
        classic_agent_frames(false).await,
        classic_agent_frames(true).await,
    ) else {
        return;
    };
    let has = |needle: &str| off.iter().any(|(_, frame)| frame.contains(needle));
    assert!(
        has(&format!("status_badge_agent_{AGENT}")),
        "the badge: {off:#?}"
    );
    assert!(
        has(&format!("directory_row_agent_{AGENT}")),
        "the row: {off:#?}"
    );
    assert!(has("Parity step"), "the step's message: {off:#?}");
    assert!(
        off.iter()
            .any(|(stream, _)| *stream == format!("user_{DAVID}_activity")),
        "the decided request's inbox item: {off:#?}"
    );
    assert_eq!(off.len(), on.len(), "off: {off:#?}\non: {on:#?}");
    for (index, (off, on)) in off.iter().zip(&on).enumerate() {
        assert_eq!(off, on, "frame {index}");
    }
}

/// Inserts a ledger entry for Bender's agent, `age_seconds` old, answering its id.
async fn ledger_entry(
    a: &TestApp,
    event_type: &'static str,
    room_id: Option<i64>,
    message_id: Option<i64>,
    metadata: serde_json::Value,
    age_seconds: i64,
) -> i64 {
    a.db()
        .write(move |tx| {
            let at = campfire_db::Timestamp::from_second(tx.now().as_second() - age_seconds);
            Ok(tx.conn().query_row(
                "INSERT INTO agent_events (agent_id, room_id, message_id, actor_id, event_type, outcome, metadata, detail, created_at, webhook_status, webhook_attempts) VALUES (?, ?, ?, ?, ?, 'delivered', ?, 'Room detail', ?, 'failed', 2) RETURNING id",
                rusqlite::params![AGENT, room_id, message_id, DAVID, event_type, metadata.to_string(), at],
                |row| row.get(0),
            )?)
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn the_ledger_is_for_managers_and_gates_what_a_room_outsider_reads() {
    let Some(a) = app(true).await else { return };
    let path = format!("/api/v1/agents/{AGENT}/events");
    let mut kevin = a.sign_in(KEVIN).await;
    let response = kevin.send(get(&path)).await;
    assert_eq!(
        response.status,
        StatusCode::FORBIDDEN,
        "{}",
        response.text()
    );
    assert_eq!(tag(&response), "Forbidden");
    let response = kevin.send(get("/api/v1/agents/999999/events")).await;
    assert_eq!(response.status, StatusCode::NOT_FOUND);

    let summary = "x".repeat(200);
    let outside = ledger_entry(
        &a,
        "work_handed_off",
        Some(ALL_TALK),
        Some(BENDERS_MESSAGE),
        json!({"handoff": {"summary": summary}}),
        1,
    )
    .await;
    let inside = ledger_entry(
        &a,
        "github_action_completed",
        Some(ARCHIVE),
        None,
        json!({"action": "comment", "status": "failed", "message": "Rate limited"}),
        2,
    )
    .await;
    let roomless = ledger_entry(&a, "fizzy_action_completed", None, None, json!({}), 3).await;

    // David owns it and is in every room: nothing is gated for him.
    let mut david = a.sign_in(DAVID).await;
    let response = david.send(get(&path)).await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    let page: api::AgentLedgerPage = parse(&response);
    let find = |page: &api::AgentLedgerPage, id: i64| {
        page.events
            .iter()
            .find(|event| event.id == id)
            .cloned()
            .unwrap_or_else(|| panic!("entry {id}"))
    };
    let entry = find(&page, outside);
    assert_eq!(entry.event_type, api::AgentLedgerEventType::WorkHandedOff);
    assert_eq!(entry.room_name.as_deref(), Some("All Talk"));
    assert_eq!(entry.detail.as_deref(), Some("Room detail"));
    let handoff = entry.handoff_summary.expect("the summary");
    assert_eq!(handoff.chars().count(), 140);
    assert!(handoff.ends_with("..."), "{handoff}");
    assert_eq!(
        (entry.webhook_status, entry.webhook_attempts),
        (api::AgentWebhookStatus::Failed, 2)
    );
    assert_eq!(entry.outcome, Some(api::AgentDeliveryOutcome::Delivered));
    assert!(page.users.iter().any(|user| user.id == BENDER));
    assert!(page.users.iter().any(|user| user.id == DAVID));

    // Make Kevin the owner: he isn't in All Talk, so that entry's room text is withheld.
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE agents SET owner_id = ? WHERE id = ?",
                [KEVIN, AGENT],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let page: api::AgentLedgerPage = parse(&kevin.send(get(&path)).await);
    let entry = find(&page, outside);
    assert_eq!(entry.room_id, Some(ALL_TALK));
    assert_eq!(
        (
            entry.room_name,
            entry.detail,
            entry.handoff_summary,
            entry.content
        ),
        (None, None, None, None)
    );
    let entry = find(&page, inside);
    assert_eq!(entry.room_name.as_deref(), Some("Archive"));
    assert_eq!(entry.detail.as_deref(), Some("Room detail"));
    let external = entry.external.expect("a GitHub result");
    assert_eq!(
        (
            external.action.as_deref(),
            external.status.as_deref(),
            external.message.as_deref()
        ),
        (Some("comment"), Some("failed"), Some("Rate limited"))
    );
    let entry = find(&page, roomless);
    assert_eq!(
        (entry.room_name, entry.detail.as_deref()),
        (None, Some("Room detail"))
    );

    // Newest first, 50 a page, and the cursor picks up after the last.
    for age in 10..70 {
        ledger_entry(&a, "mention", Some(ARCHIVE), None, json!({}), age).await;
    }
    let first: api::AgentLedgerPage = parse(&kevin.send(get(&path)).await);
    assert_eq!(first.events.len(), 50);
    assert!(
        first
            .events
            .windows(2)
            .all(|pair| (&pair[0].created_at, pair[0].id) > (&pair[1].created_at, pair[1].id)),
        "newest first"
    );
    let cursor = first.next_cursor.clone().expect("a second page");
    let mut ids = first
        .events
        .iter()
        .map(|event| event.id)
        .collect::<Vec<_>>();
    let mut next = Some(cursor);
    while let Some(cursor) = next {
        let page: api::AgentLedgerPage =
            parse(&kevin.send(get(&format!("{path}?before={cursor}"))).await);
        ids.extend(page.events.iter().map(|event| event.id));
        next = page.next_cursor;
    }
    let total = a
        .db()
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM agent_events WHERE agent_id = ?",
                [AGENT],
                |row| row.get::<_, i64>(0),
            )?)
        })
        .await
        .unwrap();
    let unique = ids.iter().collect::<std::collections::BTreeSet<_>>();
    assert_eq!((ids.len() as i64, unique.len() as i64), (total, total));

    // The outcome filter, and a cursor that doesn't decode.
    let pending: api::AgentLedgerPage =
        parse(&kevin.send(get(&format!("{path}?outcome=pending"))).await);
    assert!(
        pending
            .events
            .iter()
            .all(|event| event.outcome == Some(api::AgentDeliveryOutcome::Pending))
    );
    let response = kevin.send(get(&format!("{path}?before=garbage"))).await;
    assert_eq!(validation_fields(&response), vec!["before".to_string()]);
}

#[tokio::test]
async fn deleting_a_private_room_keeps_its_ledger_text_hidden_from_an_outside_owner() {
    use campfire_db::Room;
    use campfire_db::models::room_delete;

    let a = app(true)
        .await
        .expect("the frozen default seed is required")
        .without_job_runner()
        .await;
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE agents SET owner_id = ? WHERE id = ?",
                [KEVIN, AGENT],
            )?;
            assert!(Room::find(tx.conn(), ALL_TALK)?.closed());
            assert!(
                campfire_db::Membership::find_by_room_and_user(tx.conn(), ALL_TALK, KEVIN)?
                    .is_none()
            );
            Ok(())
        })
        .await
        .unwrap();
    let handoff = ledger_entry(
        &a,
        "work_handed_off",
        Some(ALL_TALK),
        Some(BENDERS_MESSAGE),
        json!({"handoff": {"summary": "Private handoff"}}),
        1,
    )
    .await;
    let github = ledger_entry(
        &a,
        "github_action_completed",
        Some(ALL_TALK),
        None,
        json!({"action": "comment", "status": "completed", "message": "Private result"}),
        2,
    )
    .await;
    let roomless = ledger_entry(
        &a,
        "fizzy_action_completed",
        None,
        None,
        json!({"action": "comment", "status": "completed", "message": "Roomless result"}),
        3,
    )
    .await;
    // These types allow optional rooms at creation, but no column records whether one was
    // deleted. Even a genuinely roomless instance follows the conservative gate.
    let roomless_github = ledger_entry(
        &a,
        "github_action_completed",
        None,
        None,
        json!({"action": "comment", "status": "completed", "message": "Ambiguous result"}),
        4,
    )
    .await;
    let roomless_approval = ledger_entry(&a, "approval_decided", None, None, json!({}), 5).await;
    let path = format!("/api/v1/agents/{AGENT}/events");
    let mut owner = a.sign_in(KEVIN).await;
    let find = |page: &api::AgentLedgerPage, id: i64| {
        page.events
            .iter()
            .find(|event| event.id == id)
            .cloned()
            .unwrap_or_else(|| panic!("entry {id}"))
    };
    let assert_hidden = |page: &api::AgentLedgerPage, room_id| {
        let entry = find(page, handoff);
        assert_eq!(entry.room_id, room_id);
        assert_eq!(
            (
                entry.room_name,
                entry.detail,
                entry.handoff_summary,
                entry.content
            ),
            (None, None, None, None)
        );
        let entry = find(page, github);
        assert_eq!(entry.room_id, room_id);
        assert_eq!(
            (entry.room_name, entry.detail, entry.content),
            (None, None, None)
        );
        let result = entry
            .external
            .expect("the result's non-private fields remain");
        assert_eq!(result.action.as_deref(), Some("comment"));
        assert_eq!(result.status.as_deref(), Some("completed"));
        assert_eq!(result.message, None);
    };
    let before: api::AgentLedgerPage = parse(&owner.send(get(&path)).await);
    assert_hidden(&before, Some(ALL_TALK));

    // Use the real asynchronous deletion lifecycle, including the final ledger unlink.
    a.db()
        .write(|tx| {
            room_delete::begin_destroy(tx, &Room::find(tx.conn(), ALL_TALK)?, &Default::default())
        })
        .await
        .unwrap();
    room_delete::perform_with_config(a.db(), ALL_TALK, Default::default())
        .await
        .unwrap();
    assert!(
        a.db()
            .read(|conn| Room::find_by_id(conn, ALL_TALK))
            .await
            .unwrap()
            .is_none()
    );
    let after: api::AgentLedgerPage = parse(&owner.send(get(&path)).await);
    assert_hidden(&after, None);
    let entry = find(&after, roomless);
    assert_eq!(entry.room_id, None);
    assert_eq!(entry.detail.as_deref(), Some("Room detail"));
    assert_eq!(
        entry.external.unwrap().message.as_deref(),
        Some("Roomless result")
    );
    let ambiguous = find(&after, roomless_github);
    assert_eq!(ambiguous.detail, None);
    assert_eq!(ambiguous.external.unwrap().message, None);
    assert_eq!(find(&after, roomless_approval).detail, None);

    let mut administrator = a.sign_in(DAVID).await;
    let page: api::AgentLedgerPage = parse(&administrator.send(get(&path)).await);
    let entry = find(&page, handoff);
    assert_eq!(entry.room_id, None);
    assert_eq!(entry.detail.as_deref(), Some("Room detail"));
    assert_eq!(entry.handoff_summary.as_deref(), Some("Private handoff"));
    let entry = find(&page, github);
    assert_eq!(entry.detail.as_deref(), Some("Room detail"));
    assert_eq!(
        entry.external.unwrap().message.as_deref(),
        Some("Private result")
    );
    assert_eq!(
        find(&page, roomless_approval).detail.as_deref(),
        Some("Room detail")
    );
}

#[tokio::test]
async fn the_ledger_pages_past_unknown_types_without_losing_timestamp_ties() {
    let a = app(true)
        .await
        .expect("the frozen default seed is required")
        .without_job_runner()
        .await;
    let (expected, unknown) = a
        .db()
        .write(|tx| {
            tx.conn().execute("DELETE FROM agent_events WHERE agent_id = ?", [AGENT])?;
            // The newest 125 rows are unknown, followed by 110 recognised rows. Groups of
            // seven equal timestamps straddle both the type boundary and 50-row page cuts.
            let now = tx.now().as_second();
            for index in (0..235).rev() {
                let at = campfire_db::Timestamp::from_second(now - index / 7);
                let event_type = if index < 125 { "future_unknown" } else { "work_assigned" };
                tx.conn().execute(
                    "INSERT INTO agent_events (agent_id, room_id, event_type, outcome, created_at) VALUES (?, ?, ?, 'delivered', ?)",
                    rusqlite::params![AGENT, ARCHIVE, event_type, at],
                )?;
            }
            let mut rows = tx.conn().prepare(
                "SELECT id, event_type, created_at FROM agent_events WHERE agent_id = ? ORDER BY created_at DESC, id DESC",
            )?;
            let rows = rows
                .query_map([AGENT], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, campfire_db::Timestamp>(2)?))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            assert_eq!(rows[124].2, rows[125].2, "a tie across the type boundary");
            for boundary in [50, 100, 150, 200] {
                assert_eq!(rows[boundary - 1].2, rows[boundary].2, "a tie across page {boundary}");
            }
            let expected = rows.iter().filter(|row| row.1 == "work_assigned").map(|row| row.0).collect::<Vec<_>>();
            let unknown = rows.iter().filter(|row| row.1 == "future_unknown").map(|row| row.0).collect::<std::collections::BTreeSet<_>>();
            Ok((expected, unknown))
        })
        .await
        .unwrap();
    assert_eq!((expected.len(), unknown.len()), (110, 125));
    let mut david = a.sign_in(DAVID).await;
    let path = format!("/api/v1/agents/{AGENT}/events");
    let mut next = None;
    let mut cursors = std::collections::BTreeSet::new();
    let mut events = Vec::new();
    let mut lengths = Vec::new();
    loop {
        let url = next
            .as_ref()
            .map_or_else(|| path.clone(), |cursor| format!("{path}?before={cursor}"));
        let reply = david.send(get(&url)).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        let page: api::AgentLedgerPage = parse(&reply);
        lengths.push(page.events.len());
        for event in &page.events {
            assert_eq!(event.event_type, api::AgentLedgerEventType::WorkAssigned);
            assert!(!unknown.contains(&event.id));
        }
        events.extend(page.events);
        next = page.next_cursor;
        match &next {
            Some(cursor) => assert!(cursors.insert(cursor.clone()), "a page cursor repeated"),
            None => break,
        }
        assert!(lengths.len() < 10, "paging must terminate");
    }
    assert_eq!(lengths, vec![0, 0, 25, 50, 35]);
    assert_eq!(next, None, "the final page has no next cursor");
    let ids = events.iter().map(|event| event.id).collect::<Vec<_>>();
    assert_eq!(
        ids, expected,
        "every recognised entry appears once, in database order"
    );
    assert_eq!(
        ids.iter().collect::<std::collections::BTreeSet<_>>().len(),
        ids.len()
    );
    assert!(
        events
            .windows(2)
            .all(|pair| { (&pair[0].created_at, pair[0].id) > (&pair[1].created_at, pair[1].id) }),
        "created_at DESC, id DESC across every page"
    );
}
