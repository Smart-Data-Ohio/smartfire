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
use crate::controllers::presenters::test_support::{
    BENDER, Browser, DAVID, JASON, KEVIN, Reply, Req, TestApp,
};

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

#[path = "spa_api_work_revision_tests.rs"]
mod revisions;

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

fn board_topics() -> [String; 2] {
    // Boards require replies inside a post. Keep marker replies away from the tested post.
    [format!("room:{BOARD}"), format!("thread:{DONE_POST}")]
}

/// Settle prior deferred publications before the marker's synchronous `message.created`.
/// They share the ordered sync ring, so draining to the marker catches every earlier frame.
async fn no_more_thread_updates(
    a: &TestApp,
    browser: &mut Browser<'_>,
    sync: &mut Sync,
    room_id: i64,
    thread_id: i64,
    marker: &str,
) {
    tokio::time::timeout(
        Duration::from_secs(5),
        a.booted.app.broadcasts.settle_sync(),
    )
    .await
    .expect("all deferred sync publications settled");
    let path = if room_id == BOARD {
        assert_ne!(thread_id, DONE_POST, "the marker uses a different post");
        format!("/rooms/{room_id}/threads/{DONE_POST}/messages")
    } else {
        format!("/rooms/{room_id}/messages")
    };
    let reply = browser
        .write(
            Req::new(Method::POST, &path)
                .header("accept", "text/vnd.turbo-stream.html")
                .header("content-type", "application/json")
                .body(
                    json!({"message": {"markdown_source": marker, "client_message_id": marker}})
                        .to_string(),
                ),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    sync.until(
        |event| {
            matches!(&event.payload, api::SyncPayload::MessageCreated(message)
                if message.room_id == room_id && message.client_message_id == marker)
        },
        thread_updated(thread_id),
    )
    .await;
}

/// The existing writer-queue interleaving pattern, without a production pause hook.
async fn hold_writer(a: &TestApp) -> (std::sync::mpsc::Sender<()>, tokio::task::JoinHandle<()>) {
    let (entered, ready) = tokio::sync::oneshot::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let db = a.db().clone();
    let blocker = tokio::spawn(async move {
        db.write(move |_| {
            entered.send(()).unwrap();
            wait.recv_timeout(Duration::from_secs(10)).unwrap();
            Ok(())
        })
        .await
        .unwrap();
    });
    ready.await.unwrap();
    (release, blocker)
}

async fn wait_for_queued_writes(a: &TestApp, count: usize) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while a.db().queued_writes() < count {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the competing HTTP writes queued behind the held writer");
}

#[tokio::test]
async fn deferred_sync_publications_settle_after_reader_jobs_and_per_app() {
    use campfire_app::cable::sync::SyncRenderer;

    let a = app(true).await.expect("the frozen default seed");
    let b = app(true).await.expect("the frozen default seed");
    let renderer =
        campfire_api::sync::Renderer::new(&a.booted.app, tokio::runtime::Handle::current());
    let other = campfire_api::sync::Renderer::new(&b.booted.app, tokio::runtime::Handle::current());
    let (entered, ready) = tokio::sync::oneshot::channel();
    let (release, held) = std::sync::mpsc::channel();
    renderer.defer(Box::new(move |_| {
        entered.send(()).unwrap();
        held.recv_timeout(Duration::from_secs(10)).unwrap();
    }));
    tokio::time::timeout(Duration::from_secs(5), ready)
        .await
        .expect("the deferred reader started")
        .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(100), renderer.settle())
            .await
            .is_err(),
        "settle must include a reader that is still publishing"
    );
    tokio::time::timeout(Duration::from_secs(1), other.settle())
        .await
        .expect("another app has no outstanding deferred reads");
    release.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(5), renderer.settle())
        .await
        .expect("settle completes once the deferred reader finishes");
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
    no_more_thread_updates(
        &a,
        &mut david,
        &mut kevins_tab,
        DESIGNERS,
        THREAD,
        "work-start-marker",
    )
    .await;

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
    no_more_thread_updates(
        &a,
        &mut david,
        &mut kevins_tab,
        DESIGNERS,
        THREAD,
        "work-result-marker",
    )
    .await;
    let reply = kevin
        .write(json_body(Method::PATCH, &path, &json!({"ownerId": JZ})))
        .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.text());

    // The classic validations, on the request's fields.
    let reply = david
        .write(json_body(Method::PATCH, &path, &json!({"ownerId": LOU})))
        .await;
    assert_eq!(validation_fields(&reply), vec!["ownerId".to_string()]);
    // The message reads as the classic one, by the column's name.
    assert_eq!(
        envelope(&reply),
        api::ApiError::Validation {
            message: "Work owner must be an active human member of the parent room".into(),
            fields: [(
                "ownerId".to_string(),
                vec!["must be an active human member of the parent room".to_string()]
            )]
            .into(),
        }
    );
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
    no_more_thread_updates(
        &a,
        &mut david,
        &mut kevins_tab,
        DESIGNERS,
        THREAD,
        "work-untrack-marker",
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
    let mut david = a.sign_in(DAVID).await;
    no_more_thread_updates(
        &a,
        &mut david,
        &mut kevins_tab,
        DESIGNERS,
        THREAD,
        "event-link-marker",
    )
    .await;

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
    no_more_thread_updates(
        &a,
        &mut david,
        &mut kevins_tab,
        DESIGNERS,
        THREAD,
        "link-removal-marker",
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
    no_more_thread_updates(
        &a,
        &mut david,
        &mut kevins_tab,
        DESIGNERS,
        THREAD,
        "pull-request-link-marker",
    )
    .await;
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
    let mut jasons_tab = Sync::connect(addr, &jason.cookie_header(), &board_topics()).await;
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
    no_more_thread_updates(
        &a,
        &mut david,
        &mut jasons_tab,
        BOARD,
        PLANNED_POST,
        "handoff-marker",
    )
    .await;
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

#[tokio::test]
async fn a_classic_thread_update_publishes_thread_updated_once() {
    let Some(a) = app(true).await else { return };
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    david.authenticity_token().await;
    let kevin = a.sign_in(KEVIN).await;
    let mut kevins_tab =
        Sync::connect(addr, &kevin.cookie_header(), &[format!("room:{DESIGNERS}")]).await;
    kevins_tab.welcome().await;
    let path = format!("/rooms/{DESIGNERS}/threads/{THREAD}");
    let mut marker = 0;

    // A work change (the model publishes it), a work change with a rename beside it, and a
    // rename alone (the controller publishes it): one `thread.updated` each, then nothing up to
    // a message posted after it.
    for form in [
        vec![("thread[work_status]", "planned")],
        vec![
            ("thread[work_status]", "in_progress"),
            ("thread[name]", "Work and a name"),
        ],
        vec![("thread[name]", "Only a name")],
    ] {
        let reply = david
            .write(
                Req::new(Method::PATCH, &path)
                    .header("accept", "text/html")
                    .form(&form),
            )
            .await;
        assert!(reply.status.is_redirection(), "{form:?}: {}", reply.text());
        kevins_tab.until(thread_updated(THREAD), |_| false).await;
        marker += 1;
        no_more_thread_updates(
            &a,
            &mut david,
            &mut kevins_tab,
            DESIGNERS,
            THREAD,
            &format!("classic-update-marker-{marker}"),
        )
        .await;
    }
    server.abort();
}

async fn a_classic_tag_update_auto_assigns_once(status: Option<&'static str>) {
    use campfire_db::{BoardTagAssignment, ChannelThread, NewBoardTagAssignment, NewChannelThread};

    let a = app(true).await.expect("the frozen default seed");
    let post = a
        .db()
        .write(|tx| {
            BoardTagAssignment::create(
                tx,
                NewBoardTagAssignment {
                    room_id: BOARD,
                    tag: "release".into(),
                    assignee_id: JASON,
                    created_by_id: DAVID,
                },
            )?;
            ChannelThread::create(
                tx,
                NewChannelThread {
                    room_id: BOARD,
                    creator_id: DAVID,
                    name: Some("Unassigned release".into()),
                    work_status: Some("planned".into()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    assert_eq!(post.work_owner_id, None);
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut tab = Sync::connect(addr, &david.cookie_header(), &board_topics()).await;
    tab.welcome().await;
    let mut form = vec![("thread[tags]", "release")];
    if let Some(status) = status {
        form.push(("thread[work_status]", status));
    }
    let reply = david
        .write(
            Req::new(
                Method::PATCH,
                &format!("/rooms/{BOARD}/threads/{}", post.id),
            )
            .header("accept", "text/html")
            .form(&form),
        )
        .await;
    assert!(reply.status.is_redirection(), "{}", reply.text());
    let event = tab.until(thread_updated(post.id), |_| false).await;
    let api::SyncPayload::ThreadUpdated(thread) = event.payload else {
        unreachable!()
    };
    let work = thread.work.unwrap();
    assert_eq!(
        work.status,
        if status.is_some() {
            api::WorkStatus::InProgress
        } else {
            api::WorkStatus::Planned
        },
        "the single publication includes the requested status"
    );
    assert_eq!(
        work.owner.unwrap().id,
        JASON,
        "the tag callback committed the assignment"
    );
    let (stored, history) = a
        .db()
        .read(move |conn| {
            Ok((
                ChannelThread::find(conn, post.id)?,
                campfire_db::WorkThreadEvent::for_thread(conn, post.id)?,
            ))
        })
        .await
        .unwrap();
    assert_eq!(stored.work_owner_id, Some(JASON));
    assert!(
        history.iter().any(|entry| {
            entry.event_type == "work_assignment"
                && entry.from_owner_id.is_none()
                && entry.to_owner_id == Some(JASON)
                && entry.metadata["note"] == "Auto-assigned by board tag rule"
        }),
        "the auto-assignment keeps its history: {history:?}"
    );
    if status.is_some() {
        assert!(
            history.iter().any(|entry| {
                entry.event_type == "work_update"
                    && entry.from_status.as_deref() == Some("planned")
                    && entry.to_status.as_deref() == Some("in_progress")
            }),
            "the status change keeps its history: {history:?}"
        );
    }
    no_more_thread_updates(
        &a,
        &mut david,
        &mut tab,
        BOARD,
        post.id,
        "tag-assignment-marker",
    )
    .await;
    server.abort();
}

#[tokio::test]
async fn a_tag_only_classic_update_auto_assigns_and_publishes_thread_updated_once() {
    a_classic_tag_update_auto_assigns_once(None).await;
}

#[tokio::test]
async fn a_combined_tag_and_work_classic_update_auto_assigns_and_publishes_thread_updated_once() {
    a_classic_tag_update_auto_assigns_once(Some("in_progress")).await;
}

#[tokio::test]
async fn agent_token_work_writes_publish_thread_updated_once() {
    use campfire_db::{Agent, AgentGrant, NewAgent, NewGrant, User};

    let a = app(true).await.expect("the frozen default seed");
    crate::controllers::agent_http_tests::initialize(&a).await;
    grant_bender(&a).await;
    let receiver = a
        .db()
        .write(|tx| {
            let user = User::create_bot(tx, "Release receiver", None)?;
            tx.conn().execute(
                "INSERT INTO memberships (room_id, user_id, created_at, updated_at) VALUES (?, ?, ?, ?)",
                rusqlite::params![BOARD, user.id, tx.now(), tx.now()],
            )?;
            let agent = Agent::create(
                tx,
                NewAgent {
                    user_id: user.id,
                    owner_id: Some(DAVID),
                    ..Default::default()
                },
            )?;
            for capability in ["post_messages", "manage_threads", "read_messages"] {
                AgentGrant::create(
                    tx,
                    NewGrant {
                        agent_id: agent.id,
                        room_id: Some(BOARD),
                        capability: capability.into(),
                        granted_by_id: DAVID,
                        ..Default::default()
                    },
                )?;
            }
            Ok(agent)
        })
        .await
        .unwrap();
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut tab = Sync::connect(addr, &david.cookie_header(), &board_topics()).await;
    tab.welcome().await;
    let authorization = format!("Bearer {}", crate::controllers::agent_http_tests::SECRET);
    for (index, (method, suffix, body, status)) in [
        (
            Method::PATCH,
            "",
            json!({"work_status": "in_progress"}),
            StatusCode::OK,
        ),
        (
            Method::PATCH,
            "",
            json!({"run_url": "https://ci.example/runs/agent"}),
            StatusCode::OK,
        ),
        (
            Method::PUT,
            "/result",
            json!({"markdown": "Release **complete**"}),
            StatusCode::OK,
        ),
        (
            Method::POST,
            "/handoff",
            json!({"receiver_agent_id": receiver.id, "summary": "Take over"}),
            StatusCode::CREATED,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let reply = a
            .anonymous()
            .send(
                json_body(
                    method,
                    &format!("/agents/work/{BLOCKED_POST}{suffix}"),
                    &body,
                )
                .header("authorization", &authorization),
            )
            .await;
        assert_eq!(reply.status, status, "write {index}: {}", reply.text());
        let event = tab.until(thread_updated(BLOCKED_POST), |_| false).await;
        let api::SyncPayload::ThreadUpdated(thread) = event.payload else {
            unreachable!()
        };
        let work = thread.work.unwrap();
        match index {
            0 => assert_eq!(work.status, api::WorkStatus::InProgress),
            1 => assert_eq!(
                work.run_url.as_deref(),
                Some("https://ci.example/runs/agent")
            ),
            2 => {
                assert!(work.result_updated_at.is_some());
                let detail: api::ThreadDetail = parse(
                    &david
                        .send(get(&format!("/api/v1/threads/{BLOCKED_POST}")))
                        .await,
                );
                assert_eq!(
                    detail.work.unwrap().result_markdown.as_deref(),
                    Some("Release **complete**")
                );
            }
            3 => assert_eq!(work.owner.unwrap().id, receiver.user_id),
            _ => unreachable!(),
        }
        no_more_thread_updates(
            &a,
            &mut david,
            &mut tab,
            BOARD,
            BLOCKED_POST,
            &format!("agent-write-marker-{index}"),
        )
        .await;
    }
    server.abort();
}

#[tokio::test]
async fn overlapping_handoffs_commit_once_and_publish_thread_updated_once() {
    let a = app(true).await.expect("the frozen default seed");
    grant_bender(&a).await;
    let (addr, server) = serve(&a).await;
    let path = format!("/api/v1/threads/{PLANNED_POST}/work/handoff");
    let body = json!({"receiverAgentId": AGENT, "summary": "Take the release", "links": [], "openQuestions": []});
    let mut first = a.sign_in(DAVID).await;
    let mut second = a.sign_in(DAVID).await;
    first.authenticity_token().await;
    second.authenticity_token().await;
    let mut tab = Sync::connect(addr, &first.cookie_header(), &board_topics()).await;
    tab.welcome().await;
    let (accepted, refused) = {
        let (release, blocker) = hold_writer(&a).await;
        let request = first.write(json_body(Method::POST, &path, &body));
        tokio::pin!(request);
        tokio::select! {
            _ = &mut request => panic!("the handoff ended while the writer was held"),
            () = wait_for_queued_writes(&a, 1) => {}
        }
        let release = async {
            wait_for_queued_writes(&a, 2).await;
            release.send(()).unwrap();
        };
        let (accepted, refused, ()) = tokio::join!(
            request,
            second.write(json_body(Method::POST, &path, &body)),
            release,
        );
        blocker.await.unwrap();
        (accepted, refused)
    };
    assert_eq!(accepted.status, StatusCode::CREATED, "{}", accepted.text());
    assert_eq!(
        validation_fields(&refused),
        vec!["receiverAgentId".to_string()]
    );
    let api::ApiError::Validation { message, .. } = envelope(&refused) else {
        unreachable!()
    };
    assert_eq!(message, "Receiver is already the owner of this work");
    let committed = a
        .db()
        .read(|conn| Ok(campfire_db::WorkHandoff::for_thread(conn, PLANNED_POST)?.len()))
        .await
        .unwrap();
    assert_eq!(committed, 1);
    let event = tab.until(thread_updated(PLANNED_POST), |_| false).await;
    let api::SyncPayload::ThreadUpdated(thread) = event.payload else {
        unreachable!()
    };
    assert_eq!(thread.work.unwrap().owner.unwrap().id, BENDER);
    no_more_thread_updates(
        &a,
        &mut first,
        &mut tab,
        BOARD,
        PLANNED_POST,
        "concurrent-handoff-marker",
    )
    .await;
    server.abort();
}

#[tokio::test]
async fn overlapping_api_work_and_classic_rename_publish_once_per_write() {
    // Exercise both queue orders: a later work write cannot suppress the classic rename's
    // publication, and a rename cannot overwrite the work columns from an earlier API write.
    for classic_first in [true, false] {
        let a = app(true).await.expect("the frozen default seed");
        let (addr, server) = serve(&a).await;
        let mut classic = a.sign_in(DAVID).await;
        let mut spa = a.sign_in(DAVID).await;
        classic.authenticity_token().await;
        spa.authenticity_token().await;
        let mut tab = Sync::connect(
            addr,
            &classic.cookie_header(),
            &[format!("room:{DESIGNERS}")],
        )
        .await;
        tab.welcome().await;
        let (renamed, changed) = {
            let (release, blocker) = hold_writer(&a).await;
            let classic_request = classic.write(
                Req::new(
                    Method::PATCH,
                    &format!("/rooms/{DESIGNERS}/threads/{THREAD}"),
                )
                .header("accept", "text/html")
                .form(&[("thread[name]", "Concurrent release")]),
            );
            let work_request = spa.write(json_body(
                Method::PATCH,
                &format!("/api/v1/threads/{THREAD}/work"),
                &json!({"status": "in_progress", "ownerId": KEVIN}),
            ));
            tokio::pin!(classic_request, work_request);
            if classic_first {
                tokio::select! {
                    _ = &mut classic_request => panic!("the classic edit ended while the writer was held"),
                    () = wait_for_queued_writes(&a, 1) => {}
                }
            } else {
                tokio::select! {
                    _ = &mut work_request => panic!("the API edit ended while the writer was held"),
                    () = wait_for_queued_writes(&a, 1) => {}
                }
            }
            let release = async {
                wait_for_queued_writes(&a, 2).await;
                release.send(()).unwrap();
            };
            let (renamed, changed, ()) = tokio::join!(classic_request, work_request, release);
            blocker.await.unwrap();
            (renamed, changed)
        };
        assert!(renamed.status.is_redirection(), "{}", renamed.text());
        assert_eq!(changed.status, StatusCode::OK, "{}", changed.text());
        tab.until(thread_updated(THREAD), |_| false).await;
        let last = tab.until(thread_updated(THREAD), |_| false).await;
        let api::SyncPayload::ThreadUpdated(thread) = last.payload else {
            unreachable!()
        };
        assert_eq!(thread.name, "Concurrent release");
        let work = thread.work.unwrap();
        assert_eq!(work.status, api::WorkStatus::InProgress);
        assert_eq!(work.owner.unwrap().id, KEVIN);
        let detail: api::ThreadDetail = parse(
            &classic
                .send(get(&format!("/api/v1/threads/{THREAD}")))
                .await,
        );
        assert_eq!(detail.thread.name, "Concurrent release");
        let work = detail.thread.work.unwrap();
        assert_eq!(work.status, api::WorkStatus::InProgress);
        assert_eq!(work.owner.unwrap().id, KEVIN);
        no_more_thread_updates(
            &a,
            &mut classic,
            &mut tab,
            DESIGNERS,
            THREAD,
            &format!("concurrent-rename-marker-{classic_first}"),
        )
        .await;
        server.abort();
    }
}

#[tokio::test]
async fn overlapping_api_work_and_classic_rename_cannot_publish_a_stale_snapshot_last() {
    let a = app(true).await.expect("the frozen default seed");
    let (addr, server) = serve(&a).await;
    let mut classic = a.sign_in(DAVID).await;
    let mut spa = a.sign_in(DAVID).await;
    classic.authenticity_token().await;
    spa.authenticity_token().await;
    let mut tab = Sync::connect(
        addr,
        &classic.cookie_header(),
        &[format!("room:{DESIGNERS}")],
    )
    .await;
    tab.welcome().await;

    // The API's reader has the old name. Commit the rename while that snapshot is held,
    // giving its separate reader a chance to overtake the earlier publication.
    let hold = campfire_api::test_hooks::hold_after_thread_snapshot(a.db().path(), THREAD);
    let changed = spa
        .write(json_body(
            Method::PATCH,
            &format!("/api/v1/threads/{THREAD}/work"),
            &json!({"status": "in_progress", "ownerId": KEVIN}),
        ))
        .await;
    assert_eq!(changed.status, StatusCode::OK, "{}", changed.text());
    tokio::time::timeout(Duration::from_secs(5), hold.reached)
        .await
        .expect("the work snapshot was rendered")
        .unwrap();
    let renamed = classic
        .write(
            Req::new(
                Method::PATCH,
                &format!("/rooms/{DESIGNERS}/threads/{THREAD}"),
            )
            .header("accept", "text/html")
            .form(&[("thread[name]", "Concurrent release")]),
        )
        .await;
    assert!(renamed.status.is_redirection(), "{}", renamed.text());
    let overtaking = tokio::time::timeout(
        Duration::from_millis(100),
        tab.until(thread_updated(THREAD), |_| false),
    )
    .await;
    hold.release.send(()).unwrap();
    let first = match overtaking {
        Ok(event) => event,
        Err(_) => tab.until(thread_updated(THREAD), |_| false).await,
    };
    let last = tab.until(thread_updated(THREAD), |_| false).await;
    let api::SyncPayload::ThreadUpdated(first) = first.payload else {
        unreachable!()
    };
    let api::SyncPayload::ThreadUpdated(last) = last.payload else {
        unreachable!()
    };
    assert_eq!(last.name, "Concurrent release");
    assert_eq!(first.name, "Launch review");
    for thread in [first, last] {
        let work = thread.work.unwrap();
        assert_eq!(work.status, api::WorkStatus::InProgress);
        assert_eq!(work.owner.unwrap().id, KEVIN);
    }
    no_more_thread_updates(
        &a,
        &mut classic,
        &mut tab,
        DESIGNERS,
        THREAD,
        "held-work-snapshot-marker",
    )
    .await;
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
        let mut sync = Sync::connect(addr, &david.cookie_header(), &board_topics()).await;
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
        sync.until(thread_updated(PLANNED_POST), |_| false).await;
        let event = sync.until(thread_updated(PLANNED_POST), |_| false).await;
        let api::SyncPayload::ThreadUpdated(thread) = event.payload else {
            unreachable!()
        };
        let work = thread.work.unwrap();
        assert_eq!(work.status, api::WorkStatus::Blocked);
        assert_eq!(work.links.len(), 1);
        no_more_thread_updates(
            &a,
            &mut david,
            &mut sync,
            BOARD,
            PLANNED_POST,
            "board-frame-marker",
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
