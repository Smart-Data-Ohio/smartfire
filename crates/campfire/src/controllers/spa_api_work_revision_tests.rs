use std::sync::Arc;

use campfire_db::{ChannelThread, NewChannelThread};
use campfire_kit::FrozenClock;

use super::*;
use crate::controllers::presenters::test_support::SEED_NOW;

async fn app_with_clock() -> (TestApp, Arc<FrozenClock>) {
    let clock = Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
    let app = TestApp::boot_seed_with_env("default", clock.clone(), &[("SPA_ENABLED", "1")])
        .await
        .expect("the frozen default seed");
    (app, clock)
}

fn change_time(clock: &FrozenClock, second: usize) -> String {
    clock.set(format!("2026-03-02T16:01:{second:02}Z").parse().unwrap());
    format!("2026-03-02T16:01:{second:02}.000Z")
}

async fn detail(browser: &mut Browser<'_>, thread_id: i64) -> api::ThreadDetail {
    let reply = browser
        .send(get(&format!("/api/v1/threads/{thread_id}")))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    parse(&reply)
}

async fn same_copies(browser: &mut Browser<'_>, sync: &mut Sync, expected: &api::Thread) {
    let read = detail(browser, expected.id).await;
    assert_eq!(read.thread.work, expected.work, "GET and write reply");
    let event = sync.until(thread_updated(expected.id), |_| false).await;
    let api::SyncPayload::ThreadUpdated(event_thread) = event.payload else {
        unreachable!()
    };
    assert_eq!(event_thread.work, expected.work, "sync and write reply");
    let list: api::WorkList = parse(&browser.send(get("/api/v1/work?state=all")).await);
    let row = list.threads.iter().find(|row| row.thread.id == expected.id);
    if let Some(work) = expected.work.as_ref() {
        let row = row.expect("the tracked thread appears in the work list");
        assert_eq!(
            &row.thread.work, &expected.work,
            "work list and write reply"
        );
        assert_eq!(
            row.updated_at, work.updated_at,
            "work list ordering is unchanged"
        );
    } else {
        assert!(row.is_none(), "untracked threads stay off the work list");
    }
}

#[tokio::test]
async fn human_work_revisions_move_on_changes_and_changes_back() {
    let (a, clock) = app_with_clock().await;
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut tab = Sync::connect(addr, &david.cookie_header(), &[format!("room:{DESIGNERS}")]).await;
    tab.welcome().await;
    let path = format!("/api/v1/threads/{THREAD}/work");
    let mut previous = String::new();
    let mut original = None;
    for (index, body) in [
        json!({"status": "planned"}),
        json!({"status": "in_progress"}),
        json!({"status": "blocked"}),
        json!({"status": "done"}),
        json!({"status": "planned"}),
        json!({"ownerId": KEVIN}),
        json!({"ownerId": null}),
        json!({"resultMarkdown": "Done **well**"}),
        json!({"resultMarkdown": null}),
        json!({"status": null}),
        json!({"status": "planned"}),
    ]
    .into_iter()
    .enumerate()
    {
        let expected = change_time(&clock, index);
        let reply = david.write(json_body(Method::PATCH, &path, &body)).await;
        assert_eq!(
            reply.status,
            StatusCode::OK,
            "change {index}: {}",
            reply.text()
        );
        let written: api::ThreadDetail = parse(&reply);
        if let Some(work) = written.thread.work.as_ref() {
            assert_eq!(work.updated_at, expected, "change {index}");
            assert!(work.updated_at > previous, "change {index} must advance");
            previous.clone_from(&work.updated_at);
            if index == 0 {
                original = Some(work.clone());
            } else if index == 4 || index == 6 || index == 10 {
                let original = original.as_ref().unwrap();
                assert_eq!(work.status, original.status);
                assert_eq!(work.owner, original.owner);
                assert_eq!(work.run_url, original.run_url);
                assert_eq!(work.links, original.links);
                assert!(
                    work.updated_at > original.updated_at,
                    "returning to the original facts advances"
                );
            }
        } else {
            assert_eq!(index, 9, "only untracking removes the work facts");
            let stamp = a
                .db()
                .read(|conn| Ok(ChannelThread::find(conn, THREAD)?.updated_at.to_wire()))
                .await
                .unwrap();
            assert_eq!(stamp, expected, "untracking still advances the source");
            assert!(stamp > previous);
            previous = stamp;
        }
        same_copies(&mut david, &mut tab, &written.thread).await;
    }
    server.abort();
}

#[tokio::test]
async fn handoff_and_agent_work_revisions_use_the_same_source_in_replies_gets_and_events() {
    let (a, clock) = app_with_clock().await;
    crate::controllers::agent_http_tests::initialize(&a).await;
    grant_bender(&a).await;
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut tab = Sync::connect(addr, &david.cookie_header(), &[format!("room:{BOARD}")]).await;
    tab.welcome().await;
    let original = detail(&mut david, PLANNED_POST).await.thread.work.unwrap();
    let expected = change_time(&clock, 0);
    let reply = david
        .write(json_body(
            Method::POST,
            &format!("/api/v1/threads/{PLANNED_POST}/work/handoff"),
            &json!({"receiverAgentId": AGENT, "summary": "Take the release", "links": [], "openQuestions": []}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let handoff: api::ThreadDetail = parse(&reply);
    let handed = handoff.thread.work.as_ref().unwrap();
    assert_eq!(handed.owner.as_ref().unwrap().id, BENDER);
    assert_eq!(handed.updated_at, expected);
    assert!(handed.updated_at > original.updated_at);
    same_copies(&mut david, &mut tab, &handoff.thread).await;
    let mut previous = expected;
    let authorization = format!("Bearer {}", crate::controllers::agent_http_tests::SECRET);
    for (index, (method, suffix, body)) in [
        (Method::PATCH, "", json!({"work_status": "in_progress"})),
        (Method::PATCH, "", json!({"work_status": "planned"})),
        (
            Method::PATCH,
            "",
            json!({"run_url": "https://ci.example/runs/revision"}),
        ),
        (Method::PATCH, "", json!({"run_url": null})),
        (
            Method::PUT,
            "/result",
            json!({"markdown": "Release **complete**"}),
        ),
        (Method::PUT, "/result", json!({"markdown": null})),
    ]
    .into_iter()
    .enumerate()
    {
        let expected = change_time(&clock, index + 1);
        let reply = a
            .anonymous()
            .send(
                json_body(
                    method,
                    &format!("/agents/work/{PLANNED_POST}{suffix}"),
                    &body,
                )
                .header("authorization", &authorization),
            )
            .await;
        assert_eq!(
            reply.status,
            StatusCode::OK,
            "change {index}: {}",
            reply.text()
        );
        let payload: serde_json::Value = parse(&reply);
        assert_eq!(
            payload["updated_at"], expected,
            "the existing agent reply uses the same source"
        );
        let read = detail(&mut david, PLANNED_POST).await;
        let work = read.thread.work.as_ref().unwrap();
        assert_eq!(work.updated_at, expected);
        assert!(work.updated_at > previous);
        previous = expected;
        match index {
            1 => assert_eq!(work.status, original.status),
            3 => assert_eq!(work.run_url, original.run_url),
            5 => assert!(read.work.as_ref().unwrap().result_markdown.is_none()),
            _ => {}
        }
        same_copies(&mut david, &mut tab, &read.thread).await;
    }
    let expected = change_time(&clock, 7);
    let reply = david
        .write(json_body(
            Method::PATCH,
            &format!("/api/v1/threads/{PLANNED_POST}/work"),
            &json!({"ownerId": DAVID}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let returned: api::ThreadDetail = parse(&reply);
    let work = returned.thread.work.as_ref().unwrap();
    assert_eq!(work.owner.as_ref().unwrap().id, DAVID);
    assert_eq!(work.updated_at, expected);
    assert!(work.updated_at > original.updated_at);
    same_copies(&mut david, &mut tab, &returned.thread).await;
    server.abort();
}

#[tokio::test]
async fn tracked_thread_created_revision_agrees_with_get() {
    let (a, clock) = app_with_clock().await;
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut tab = Sync::connect(addr, &david.cookie_header(), &[format!("room:{BOARD}")]).await;
    tab.welcome().await;
    let expected = change_time(&clock, 0);
    let thread = a
        .db()
        .write(|tx| {
            ChannelThread::create_board_post(
                tx,
                NewChannelThread {
                    room_id: BOARD,
                    creator_id: DAVID,
                    name: Some("Revision proof".into()),
                    work_status: Some("planned".into()),
                    ..Default::default()
                },
                None,
            )
        })
        .await
        .unwrap();
    let id = thread.id;
    a.booted.app.broadcasts.thread_created(id);
    let event = tab.until(|event| matches!(&event.payload, api::SyncPayload::ThreadCreated(thread) if thread.id == id), |_| false).await;
    let api::SyncPayload::ThreadCreated(created) = event.payload else {
        unreachable!()
    };
    let read = detail(&mut david, id).await;
    assert_eq!(created.work, read.thread.work);
    assert_eq!(created.work.as_ref().unwrap().updated_at, expected);
    assert_eq!(thread.updated_at.to_wire(), expected);
    server.abort();
}

#[tokio::test]
async fn link_add_remove_cannot_regress_the_revision_without_a_surviving_link_timestamp() {
    let (a, clock) = app_with_clock().await;
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut tab = Sync::connect(addr, &david.cookie_header(), &[format!("room:{BOARD}")]).await;
    tab.welcome().await;
    let original = detail(&mut david, PLANNED_POST).await.thread.work.unwrap();
    change_time(&clock, 0);
    let link = a
        .db()
        .write(|tx| {
            WorkThreadLink::create(
                tx,
                NewWorkThreadLink {
                    channel_thread_id: PLANNED_POST,
                    created_by_id: DAVID,
                    kind: Some("drive_file".into()),
                    url: Some("https://docs.google.com/document/d/revision".into()),
                    title: Some("Plan".into()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let added = detail(&mut david, PLANNED_POST).await;
    let work = added.thread.work.as_ref().unwrap();
    assert!(work.links.iter().any(|item| item.id == link.id));
    assert_eq!(
        work.updated_at, original.updated_at,
        "link writes leave the ordering timestamp alone"
    );
    same_copies(&mut david, &mut tab, &added.thread).await;
    change_time(&clock, 1);
    a.db().write(move |tx| link.destroy(tx)).await.unwrap();
    let removed = detail(&mut david, PLANNED_POST).await;
    assert_eq!(removed.thread.work.as_ref().unwrap().links, original.links);
    assert_eq!(
        removed.thread.work.as_ref().unwrap().updated_at,
        original.updated_at,
        "using live link timestamps would go backwards here"
    );
    same_copies(&mut david, &mut tab, &removed.thread).await;
    server.abort();
}

#[tokio::test]
async fn owner_hard_delete_cannot_regress_the_revision_after_live_profile_changes() {
    use campfire_db::{Membership, NewUser, User, UserChanges};

    let (a, clock) = app_with_clock().await;
    let owner = a
        .db()
        .write(|tx| {
            let owner = User::create(
                tx,
                NewUser {
                    name: "Revision owner".into(),
                    ..Default::default()
                },
            )?;
            Membership::create_default(tx, BOARD, owner.id)?;
            Ok(owner)
        })
        .await
        .unwrap();
    let owner_id = owner.id;
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut tab = Sync::connect(addr, &david.cookie_header(), &[format!("room:{BOARD}")]).await;
    tab.welcome().await;
    let revision = change_time(&clock, 0);
    let reply = david
        .write(json_body(
            Method::PATCH,
            &format!("/api/v1/threads/{PLANNED_POST}/work"),
            &json!({"ownerId": owner_id}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let assigned: api::ThreadDetail = parse(&reply);
    same_copies(&mut david, &mut tab, &assigned.thread).await;
    for (index, name) in ["Renamed owner", "Revision owner"].into_iter().enumerate() {
        let user_revision = change_time(&clock, index + 1);
        let user_stamp = a
            .db()
            .write(move |tx| {
                let mut owner = User::find(tx.conn(), owner_id)?;
                owner.update(
                    tx,
                    UserChanges {
                        name: Some(name.into()),
                        ..Default::default()
                    },
                )?;
                Ok(owner.updated_at.to_wire())
            })
            .await
            .unwrap();
        assert_eq!(user_stamp, user_revision);
        let read = detail(&mut david, PLANNED_POST).await;
        let work = read.thread.work.as_ref().unwrap();
        assert_eq!(work.owner.as_ref().unwrap().name, name);
        assert_eq!(
            work.updated_at, revision,
            "live profile changes have no durable thread revision"
        );
    }
    change_time(&clock, 3);
    a.db()
        .write(move |tx| User::find(tx.conn(), owner_id)?.destroy(tx))
        .await
        .unwrap();
    let removed = detail(&mut david, PLANNED_POST).await;
    let work = removed.thread.work.as_ref().unwrap();
    assert!(
        work.owner.is_none(),
        "the foreign key clears the assignment"
    );
    assert!(!work.owner_active);
    assert_eq!(
        work.updated_at, revision,
        "the deletion must not restore an older revision"
    );
    // The implicit FK update has no work publication. Exercise the existing publication seam.
    a.booted.app.broadcasts.thread_updated(PLANNED_POST);
    same_copies(&mut david, &mut tab, &removed.thread).await;
    server.abort();
}
