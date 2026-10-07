//! The S4 work endpoints on `/api/v1` (`campfire_api::work`): the work facts on every thread,
//! the work section of a thread's detail, the work list, a work change and a handoff, each with
//! `channel_threads#update`'s and `work_threads#create_handoff`'s checks; `thread.updated` on
//! every work change; and the board's classic frames byte for byte the same with the sync
//! engine on.

use std::time::Duration;

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use campfire_db::{NewWorkThreadLink, WorkThreadLink};
use serde_json::json;

use super::api_tests::{Sync, app, get, json_body, parse, serve, tag};
use crate::controllers::presenters::test_support::{BENDER, DAVID, JASON, KEVIN, Reply, TestApp};

/// Bender's agent.
const AGENT: i64 = 773018776;
/// A closed room David created; Kevin and JZ are plain members.
const DESIGNERS: i64 = 654632876;
/// Designers' untracked thread, which David started.
const THREAD: i64 = 1;
/// David's board: David, Jason and Bender. Its posts 4 (planned, David's), 5 (in progress,
/// David's), 6 (blocked, Bender's) and 7 (done, David's) are all tracked.
const BOARD: i64 = 699448332;
const PLANNED_POST: i64 = 4;
const BLOCKED_POST: i64 = 6;
const DONE_POST: i64 = 7;
/// A plain member of Designers.
const JZ: i64 = 773523953;
/// A member in no room at all.
const LOU: i64 = 773523958;
/// An event in Designers, and the seed's pull request.
const DESIGNERS_EVENT: i64 = 390339825;
const PULL_REQUEST: i64 = 1;

fn envelope(reply: &Reply) -> api::ApiError {
    parse::<api::ApiErrorResponse>(reply).error
}

fn validation_fields(reply: &Reply) -> Vec<String> {
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.text()
    );
    let api::ApiError::Validation { fields, .. } = envelope(reply) else {
        panic!("{}", reply.text())
    };
    fields.into_keys().collect()
}

/// Lets Bender's agent post, manage threads and read messages everywhere.
async fn grant_bender(a: &TestApp) {
    a.db()
        .write(|tx| {
            for capability in ["post_messages", "manage_threads", "read_messages"] {
                tx.conn().execute(
                    "INSERT INTO agent_grants (agent_id, capability, granted_by_id, created_at, updated_at) VALUES (?, ?, ?, '2026-03-02 15:00:00', '2026-03-02 15:00:00')",
                    rusqlite::params![AGENT, capability, DAVID],
                )?;
            }
            Ok(())
        })
        .await
        .unwrap();
}

async fn sql(a: &TestApp, statements: &'static [&'static str]) {
    a.db()
        .write(move |tx| {
            for statement in statements {
                tx.conn().execute_batch(statement)?;
            }
            Ok(())
        })
        .await
        .unwrap();
}

fn thread_updated(thread_id: i64) -> impl Fn(&api::SyncEvent) -> bool {
    move |event| matches!(&event.payload, api::SyncPayload::ThreadUpdated(thread) if thread.id == thread_id)
}

#[tokio::test]
async fn threads_carry_their_work_facts_and_the_detail_its_work_section() {
    let Some(a) = app(true).await else { return };
    grant_bender(&a).await;
    // A run URL that isn't `https://`, a result, and links: a pull request whose stored URL is
    // a script (left out), a Drive file, a Drive file at a script URL (left out), and an event.
    sql(
        &a,
        &[
            "UPDATE channel_threads SET run_url = 'javascript:alert(1)', result_markdown = 'Shipped **it**', result_updated_at = '2026-03-02 15:30:00', result_updated_by_id = 127326141 WHERE id = 6",
            "UPDATE github_pull_requests SET html_url = 'javascript:alert(2)' WHERE id = 1",
            "INSERT INTO work_thread_links (channel_thread_id, created_by_id, kind, github_pull_request_id, created_at, updated_at) VALUES (6, 127326141, 'pull_request', 1, '2026-03-02 15:00:00', '2026-03-02 15:00:00')",
            "INSERT INTO work_thread_links (channel_thread_id, created_by_id, kind, url, title, created_at, updated_at) VALUES (6, 127326141, 'drive_file', 'https://docs.google.com/document/d/1', 'Plan', '2026-03-02 15:00:00', '2026-03-02 15:00:00')",
            "INSERT INTO work_thread_links (channel_thread_id, created_by_id, kind, url, created_at, updated_at) VALUES (6, 127326141, 'drive_file', 'javascript:alert(3)', '2026-03-02 15:00:00', '2026-03-02 15:00:00')",
        ],
    )
    .await;
    let mut david = a.sign_in(DAVID).await;
    let path = format!("/api/v1/threads/{BLOCKED_POST}");
    let reply = david.send(get(&path)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let detail: api::ThreadDetail = parse(&reply);
    let facts = detail.thread.work.as_ref().expect("a tracked thread");
    assert_eq!(facts.status, api::WorkStatus::Blocked);
    let owner = facts.owner.as_ref().expect("Bender owns it");
    assert_eq!(owner.id, BENDER);
    assert_eq!(
        owner.agent.as_ref().map(|badge| badge.agent_id),
        Some(AGENT)
    );
    assert!(facts.owner_active, "Bender is on the board and may post");
    assert_eq!(facts.run_url, None, "only an https:// run URL");
    assert!(facts.result_updated_at.is_some());
    assert_eq!(
        facts
            .links
            .iter()
            .map(|link| (link.kind, link.url.as_str(), link.label.as_str()))
            .collect::<Vec<_>>(),
        vec![(
            api::WorkLinkKind::DriveFile,
            "https://docs.google.com/document/d/1",
            "Plan"
        )]
    );
    // David is the board's creator: every work action but removing a board post's tracking.
    let permissions = detail.permissions;
    assert_eq!(
        (
            permissions.can_convert_work,
            permissions.can_manage_work,
            permissions.can_update_work_status,
            permissions.can_assign_work,
            permissions.can_remove_work
        ),
        (false, true, true, true, false)
    );
    let work = detail.work.as_ref().expect("the work section");
    assert_eq!(work.result_markdown.as_deref(), Some("Shipped **it**"));
    assert!(
        work.result_html
            .as_deref()
            .is_some_and(|html| html.contains("<strong>it</strong>")),
        "{:?}",
        work.result_html
    );
    assert_eq!(work.result_updated_by_id, Some(DAVID));
    // Owner candidates: the board's active humans, then its agents allowed to post.
    let candidates = work
        .owner_candidates
        .iter()
        .map(|candidate| candidate.user_id)
        .collect::<Vec<_>>();
    assert!(
        candidates.contains(&DAVID) && candidates.contains(&JASON) && candidates.contains(&BENDER),
        "{candidates:?}"
    );
    assert_eq!(candidates.last(), Some(&BENDER), "agents come after people");
    // Bender already owns it, so nobody to hand it to.
    assert_eq!(work.handoff_receivers, vec![]);
    let users = detail.users.iter().map(|user| user.id).collect::<Vec<_>>();
    assert!(
        users.contains(&JASON) && users.contains(&BENDER),
        "{users:?}"
    );

    // An https:// run URL shows, and the thread list carries the same facts.
    sql(
        &a,
        &["UPDATE channel_threads SET run_url = 'https://ci.example/runs/1' WHERE id = 6"],
    )
    .await;
    let list: api::ThreadList = parse(
        &david
            .send(get(&format!("/api/v1/rooms/{BOARD}/threads?state=all")))
            .await,
    );
    let row = list
        .threads
        .iter()
        .find(|summary| summary.thread.id == BLOCKED_POST)
        .unwrap();
    let facts = row.thread.work.as_ref().unwrap();
    assert_eq!(facts.run_url.as_deref(), Some("https://ci.example/runs/1"));
    assert_eq!(facts.links.len(), 1);

    // An untracked thread has no work, and its creator may start tracking it.
    let detail: api::ThreadDetail =
        parse(&david.send(get(&format!("/api/v1/threads/{THREAD}"))).await);
    assert_eq!((&detail.thread.work, &detail.work), (&None, &None));
    assert!(detail.permissions.can_convert_work);
    // Kevin isn't on the board.
    let mut kevin = a.sign_in(KEVIN).await;
    assert_eq!(kevin.send(get(&path)).await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn the_work_list_filters_as_the_classic_page_does() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    fn ids(reply: Reply) -> Vec<i64> {
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        let list: api::WorkList = parse(&reply);
        for row in &list.threads {
            assert!(row.thread.work.is_some());
            assert_eq!((row.room_name.as_str(), row.board), ("Release board", true));
        }
        let mut ids: Vec<i64> = list.threads.iter().map(|row| row.thread.id).collect();
        if !ids.is_empty() {
            assert!(list.users.iter().any(|user| user.id == DAVID));
        }
        ids.sort();
        ids
    }
    for (state, expected) in [
        ("", vec![4, 5, 6]),
        ("?state=open", vec![4, 5, 6]),
        ("?state=whatever", vec![4, 5, 6]),
        ("?state=done", vec![DONE_POST]),
        ("?state=agents", vec![BLOCKED_POST]),
        ("?state=boards", vec![4, 5, 6, 7]),
        ("?state=all", vec![4, 5, 6, 7]),
    ] {
        let reply = david.send(get(&format!("/api/v1/work{state}"))).await;
        assert_eq!(ids(reply), expected, "{state}");
    }
    // Kevin isn't on the board: nothing.
    let mut kevin = a.sign_in(KEVIN).await;
    let reply = kevin.send(get("/api/v1/work?state=all")).await;
    assert_eq!(ids(reply), Vec::<i64>::new());
}

#[tokio::test]
async fn work_changes_take_the_classic_checks_and_publish_thread_updated() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let path = format!("/api/v1/threads/{THREAD}/work");
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let mut kevins_tab =
        Sync::connect(addr, &kevin.cookie_header(), &[format!("room:{DESIGNERS}")]).await;
    kevins_tab.welcome().await;

    // Kevin may not start tracking David's thread.
    let reply = kevin
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"status": "planned"}),
        ))
        .await;
    assert_eq!(
        (reply.status, tag(&reply)),
        (StatusCode::FORBIDDEN, "Forbidden".into()),
        "{}",
        reply.text()
    );
    // David may, and assign it to Kevin in the same change.
    let reply = david
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"status": "planned", "ownerId": KEVIN}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let detail: api::ThreadDetail = parse(&reply);
    let facts = detail.thread.work.as_ref().unwrap();
    assert_eq!(
        (
            facts.status,
            facts.owner.as_ref().map(|owner| owner.id),
            facts.owner_active
        ),
        (api::WorkStatus::Planned, Some(KEVIN), true)
    );
    let work = detail.work.as_ref().unwrap();
    assert_eq!(
        work.history
            .iter()
            .map(|entry| (entry.kind, entry.to_status, entry.actor_id))
            .collect::<Vec<_>>(),
        vec![(
            api::WorkHistoryKind::Update,
            Some(api::WorkStatus::Planned),
            Some(DAVID)
        )]
    );
    assert_eq!(
        work.history[0]
            .to_owner
            .as_ref()
            .and_then(|owner| owner.user_id),
        Some(KEVIN)
    );
    // Every room member hears of it, with the facts.
    let event = kevins_tab.until(thread_updated(THREAD), |_| false).await;
    let api::SyncPayload::ThreadUpdated(thread) = event.payload else {
        unreachable!()
    };
    assert_eq!(
        thread.work.as_ref().map(|work| work.status),
        Some(api::WorkStatus::Planned)
    );

    // Kevin owns it now: he may write its result and move it, not reassign it.
    let reply = kevin
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"resultMarkdown": "Done *well*", "status": "in_progress"}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let detail: api::ThreadDetail = parse(&reply);
    assert_eq!(
        detail.thread.work.as_ref().map(|work| work.status),
        Some(api::WorkStatus::InProgress)
    );
    let work = detail.work.unwrap();
    assert_eq!(work.result_markdown.as_deref(), Some("Done *well*"));
    assert_eq!(work.result_updated_by_id, Some(KEVIN));
    assert_eq!(
        work.owner_candidates,
        vec![],
        "only an assigner sees the candidates"
    );
    let kinds = work
        .history
        .iter()
        .map(|entry| entry.kind)
        .collect::<Vec<_>>();
    assert!(kinds.contains(&api::WorkHistoryKind::Result), "{kinds:?}");
    let event = kevins_tab
        .until(
            |event| {
                matches!(&event.payload, api::SyncPayload::ThreadUpdated(thread)
                    if thread.work.as_ref().is_some_and(|work| work.status == api::WorkStatus::InProgress))
            },
            |_| false,
        )
        .await;
    assert_eq!(event.topic, format!("room:{DESIGNERS}"));
    let reply = kevin
        .write(json_body(Method::PATCH, &path, &json!({"ownerId": JZ})))
        .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.text());

    // The classic validations, on the request's fields.
    let reply = david
        .write(json_body(Method::PATCH, &path, &json!({"ownerId": LOU})))
        .await;
    assert_eq!(validation_fields(&reply), vec!["ownerId".to_string()]);
    let reply = david
        .write(json_body(Method::PATCH, &path, &json!({"status": null})))
        .await;
    assert_eq!(validation_fields(&reply), vec!["ownerId".to_string()]);
    let long = "x".repeat(20_001);
    let reply = david
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"resultMarkdown": long}),
        ))
        .await;
    assert_eq!(
        validation_fields(&reply),
        vec!["resultMarkdown".to_string()]
    );
    let reply = david
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"status": "someday"}),
        ))
        .await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.text()
    );

    // Stopping tracking needs the owner cleared too.
    let reply = david
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"status": null, "ownerId": null}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let detail: api::ThreadDetail = parse(&reply);
    assert_eq!((&detail.thread.work, &detail.work), (&None, &None));
    assert!(detail.permissions.can_convert_work);
    kevins_tab
        .until(
            |event| {
                matches!(&event.payload, api::SyncPayload::ThreadUpdated(thread)
                    if thread.id == THREAD && thread.work.is_none())
            },
            |_| false,
        )
        .await;
    server.abort();
}

#[tokio::test]
async fn a_link_change_publishes_thread_updated() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    sql(
        &a,
        &["UPDATE channel_threads SET work_status = 'planned' WHERE id = 1"],
    )
    .await;
    let kevin = a.sign_in(KEVIN).await;
    let mut kevins_tab =
        Sync::connect(addr, &kevin.cookie_header(), &[format!("room:{DESIGNERS}")]).await;
    kevins_tab.welcome().await;
    let link = a
        .db()
        .write(|tx| {
            WorkThreadLink::create(
                tx,
                NewWorkThreadLink {
                    channel_thread_id: THREAD,
                    created_by_id: DAVID,
                    kind: Some("event".into()),
                    event_id: Some(DESIGNERS_EVENT),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let event = kevins_tab.until(thread_updated(THREAD), |_| false).await;
    let api::SyncPayload::ThreadUpdated(thread) = event.payload else {
        unreachable!()
    };
    let links = &thread.work.as_ref().unwrap().links;
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].kind, api::WorkLinkKind::Event);
    assert_eq!(
        links[0].url,
        format!("/rooms/{DESIGNERS}/events/{DESIGNERS_EVENT}")
    );
    assert!(links[0].event_starts_at.is_some());

    a.db().write(move |tx| link.destroy(tx)).await.unwrap();
    kevins_tab
        .until(
            |event| {
                matches!(&event.payload, api::SyncPayload::ThreadUpdated(thread)
                    if thread.work.as_ref().is_some_and(|work| work.links.is_empty()))
            },
            |_| false,
        )
        .await;
    // A pull request link, with its state.
    a.db()
        .write(|tx| {
            WorkThreadLink::create(
                tx,
                NewWorkThreadLink {
                    channel_thread_id: THREAD,
                    created_by_id: DAVID,
                    kind: Some("pull_request".into()),
                    github_pull_request_id: Some(PULL_REQUEST),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let event = kevins_tab
        .until(
            |event| {
                matches!(&event.payload, api::SyncPayload::ThreadUpdated(thread)
                    if thread.work.as_ref().is_some_and(|work| !work.links.is_empty()))
            },
            |_| false,
        )
        .await;
    let api::SyncPayload::ThreadUpdated(thread) = event.payload else {
        unreachable!()
    };
    let link = &thread.work.as_ref().unwrap().links[0];
    assert_eq!(
        (link.kind, link.pull_request_state),
        (
            api::WorkLinkKind::PullRequest,
            Some(api::WorkPullRequestState::Open)
        )
    );
    assert!(link.url.starts_with("https://"), "{}", link.url);
    server.abort();
}

#[tokio::test]
async fn a_handoff_takes_the_classic_checks_and_moves_the_owner() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let path = format!("/api/v1/threads/{PLANNED_POST}/work/handoff");
    let body = |receiver: i64, summary: &str| {
        json!({
            "receiverAgentId": receiver,
            "summary": summary,
            "links": ["https://example.com/spec", " https://example.com/spec "],
            "openQuestions": ["Which region?"],
        })
    };
    let mut david = a.sign_in(DAVID).await;

    // Bender may post but not manage threads yet, so isn't a receiver.
    let detail: api::ThreadDetail = parse(
        &david
            .send(get(&format!("/api/v1/threads/{PLANNED_POST}")))
            .await,
    );
    assert_eq!(detail.work.unwrap().handoff_receivers, vec![]);
    let reply = david
        .write(json_body(Method::POST, &path, &body(AGENT, "Take it")))
        .await;
    assert_eq!(
        validation_fields(&reply),
        vec!["receiverAgentId".to_string()]
    );
    let api::ApiError::Validation { message, .. } = envelope(&reply) else {
        unreachable!()
    };
    assert_eq!(
        message,
        "Receiver must hold the manage_threads capability in this room"
    );
    grant_bender(&a).await;
    let detail: api::ThreadDetail = parse(
        &david
            .send(get(&format!("/api/v1/threads/{PLANNED_POST}")))
            .await,
    );
    assert_eq!(
        detail.work.unwrap().handoff_receivers,
        vec![api::WorkHandoffReceiver {
            agent_id: AGENT,
            user_id: BENDER
        }]
    );
    let reply = david
        .write(json_body(Method::POST, &path, &body(999, "Take it")))
        .await;
    assert_eq!(
        validation_fields(&reply),
        vec!["receiverAgentId".to_string()]
    );
    let reply = david
        .write(json_body(Method::POST, &path, &body(AGENT, "  ")))
        .await;
    assert_eq!(validation_fields(&reply), vec!["summary".to_string()]);
    let reply = david
        .write(json_body(
            Method::POST,
            &path,
            &json!({"receiverAgentId": AGENT, "summary": "Go", "links": ["ftp://x"], "openQuestions": []}),
        ))
        .await;
    assert_eq!(validation_fields(&reply), vec!["links".to_string()]);

    // An untracked thread, and a member who may not manage the work.
    let reply = david
        .write(json_body(
            Method::POST,
            &format!("/api/v1/threads/{THREAD}/work/handoff"),
            &body(AGENT, "Take it"),
        ))
        .await;
    assert_eq!(validation_fields(&reply), vec!["base".to_string()]);
    sql(
        &a,
        &["INSERT INTO memberships (room_id, user_id, created_at, updated_at) VALUES (699448332, 773523953, '2026-03-02 15:00:00', '2026-03-02 15:00:00')"],
    )
    .await;
    let mut jz = a.sign_in(JZ).await;
    let reply = jz
        .write(json_body(Method::POST, &path, &body(AGENT, "Take it")))
        .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.text());
    let mut kevin = a.sign_in(KEVIN).await;
    let reply = kevin
        .write(json_body(Method::POST, &path, &body(AGENT, "Take it")))
        .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND, "{}", reply.text());

    let jason = a.sign_in(JASON).await;
    let mut jasons_tab =
        Sync::connect(addr, &jason.cookie_header(), &[format!("room:{BOARD}")]).await;
    jasons_tab.welcome().await;
    let reply = david
        .write(json_body(
            Method::POST,
            &path,
            &body(AGENT, "Take the release"),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let detail: api::ThreadDetail = parse(&reply);
    assert_eq!(
        detail
            .thread
            .work
            .as_ref()
            .and_then(|work| work.owner.as_ref())
            .map(|owner| owner.id),
        Some(BENDER)
    );
    let work = detail.work.unwrap();
    let handoff = work
        .history
        .iter()
        .find(|entry| entry.kind == api::WorkHistoryKind::Handoff)
        .expect("the handoff entry");
    assert_eq!(
        handoff.handoff,
        Some(api::WorkHistoryHandoff {
            summary: "Take the release".into(),
            link_count: 1,
            question_count: 1
        })
    );
    // Bender owns it now, so he's no longer a receiver.
    assert_eq!(work.handoff_receivers, vec![]);
    let event = jasons_tab
        .until(thread_updated(PLANNED_POST), |_| false)
        .await;
    let api::SyncPayload::ThreadUpdated(thread) = event.payload else {
        unreachable!()
    };
    assert_eq!(
        thread
            .work
            .and_then(|work| work.owner)
            .map(|owner| owner.id),
        Some(BENDER)
    );
    // Handing it to its owner again names the receiver.
    let reply = david
        .write(json_body(Method::POST, &path, &body(AGENT, "Again")))
        .await;
    assert_eq!(
        validation_fields(&reply),
        vec!["receiverAgentId".to_string()]
    );
    server.abort();
}

/// The classic frames of a board post's work change through `channel_threads#update`: its rows
/// and its column, and the `thread.updated` beside them when the sync engine is on.
async fn classic_board_frames(spa: bool) -> Option<Vec<(String, String)>> {
    use crate::controllers::presenters::test_support::SEED_NOW;
    use campfire_kit::clock::FrozenClock;
    let clock = std::sync::Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
    let env: &[(&str, &str)] = if spa { &[("SPA_ENABLED", "1")] } else { &[] };
    let a = TestApp::boot_seed_with_env("default", clock, env).await?;
    let (mut client, cable) =
        crate::controllers::messages::attachment_processing_tests::subscribe(&a).await;
    let board = a
        .db()
        .read(|conn| campfire_db::Room::find(conn, BOARD))
        .await
        .unwrap();
    let gid = campfire_app::cable::room_gid(&board).to_param();
    let signed =
        rails_compat::turbo::signed_stream_name(&a.booted.app.secrets, &[&gid, "messages"]);
    let identifier = crate::channels::tests::support::identifier(
        json!({ "channel": "RoomMessagesChannel", "signed_stream_name": signed }),
    );
    client.confirm(&identifier).await;
    let mut david = a.sign_in(DAVID).await;
    david.authenticity_token().await;
    let (sync, server) = if spa {
        let (addr, server) = serve(&a).await;
        let mut sync =
            Sync::connect(addr, &david.cookie_header(), &[format!("room:{BOARD}")]).await;
        sync.welcome().await;
        (Some(sync), Some(server))
    } else {
        (None, None)
    };
    assert_eq!(a.booted.app.cable.sync_wanted(), spa);
    let capture = a.publications();
    capture.take();

    let reply = david
        .write(
            crate::controllers::presenters::test_support::Req::new(
                Method::PATCH,
                &format!("/rooms/{BOARD}/threads/{PLANNED_POST}"),
            )
            .header("accept", "text/html")
            .form(&[("thread[work_status]", "blocked")]),
        )
        .await;
    assert!(reply.status.is_redirection(), "{}", reply.text());
    let link = a
        .db()
        .write(|tx| {
            WorkThreadLink::create(
                tx,
                NewWorkThreadLink {
                    channel_thread_id: PLANNED_POST,
                    created_by_id: DAVID,
                    kind: Some("drive_file".into()),
                    url: Some("https://docs.google.com/document/d/2".into()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    assert_eq!(link.channel_thread_id, PLANNED_POST);

    let mut frames = Vec::new();
    let mut quiet = 0;
    while quiet < 10 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let more = capture.take();
        quiet = if more.is_empty() { quiet + 1 } else { 0 };
        frames.extend(more);
    }
    if let Some(mut sync) = sync {
        // The post's new status, then its link: each a `thread.updated`.
        sync.until(
            |event| {
                matches!(&event.payload, api::SyncPayload::ThreadUpdated(thread)
                    if thread.id == PLANNED_POST && thread.work.as_ref().is_some_and(|work| work.status == api::WorkStatus::Blocked && work.links.len() == 1))
            },
            |_| false,
        )
        .await;
    }
    cable.abort();
    if let Some(server) = server {
        server.abort();
    }
    Some(frames)
}

#[tokio::test]
async fn the_board_frames_are_the_same_with_the_sync_engine_on() {
    let (Some(off), Some(on)) = (
        classic_board_frames(false).await,
        classic_board_frames(true).await,
    ) else {
        return;
    };
    let has = |needle: &str| off.iter().any(|(_, frame)| frame.contains(needle));
    assert!(
        has(&format!("board_row_channel_thread_{PLANNED_POST}")),
        "the post's row: {off:#?}"
    );
    assert!(has("board_column_blocked"), "its new column: {off:#?}");
    assert_eq!(off.len(), on.len(), "off: {off:#?}\non: {on:#?}");
    for (index, (off, on)) in off.iter().zip(&on).enumerate() {
        assert_eq!(off, on, "frame {index}");
    }
}
