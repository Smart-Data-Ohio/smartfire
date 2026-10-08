//! Work link JSON writers retain the classic builders, viewing authority and atomic PR fetch.

use std::time::Duration;

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use serde_json::{Value, json};

use super::api_tests::{Sync, app, get, json_body, parse, serve, tag};
use crate::controllers::presenters::test_support::{Browser, DAVID, KEVIN, Reply, TestApp};

const THREAD: i64 = 1;
const ROOM: i64 = 654632876;
const BOARD_THREAD: i64 = 4;
const EVENT: i64 = 9000001;
const FOREIGN_EVENT: i64 = 9000006;

fn path(thread_id: i64) -> String {
    format!("/api/v1/threads/{thread_id}/work/links")
}

async fn sql(a: &TestApp, statements: impl Into<String>) {
    let statements = statements.into();
    a.db()
        .write(move |tx| {
            tx.conn().execute_batch(&statements)?;
            Ok(())
        })
        .await
        .unwrap();
}

async fn track(a: &TestApp) {
    sql(
        a,
        "UPDATE channel_threads SET work_status='planned',work_owner_id=NULL WHERE id IN (1,8)",
    )
    .await;
}

async fn events(a: &TestApp) {
    sql(a, format!("INSERT INTO events (id,room_id,organizer_id,title,starts_at,ends_at,time_zone,cancelled_at,created_at,updated_at) VALUES
        (9000001,{ROOM},{DAVID},'Review','2026-03-02 17:00:00',NULL,'America/New_York',NULL,'2026-03-02 15:00:00','2026-03-02 15:00:00'),
        (9000002,{ROOM},{DAVID},'Same start','2026-03-02 17:00:00',NULL,'UTC',NULL,'2026-03-02 15:00:00','2026-03-02 15:00:00'),
        (9000003,{ROOM},{DAVID},'In progress','2026-03-02 15:00:00','2026-03-02 18:00:00','UTC',NULL,'2026-03-02 15:00:00','2026-03-02 15:00:00'),
        (9000004,{ROOM},{DAVID},'Past','2026-03-01 15:00:00',NULL,'UTC',NULL,'2026-03-02 15:00:00','2026-03-02 15:00:00'),
        (9000005,{ROOM},{DAVID},'Cancelled','2026-03-02 18:00:00',NULL,'UTC','2026-03-02 15:00:00','2026-03-02 15:00:00','2026-03-02 15:00:00'),
        (9000006,699448332,{DAVID},'Foreign','2026-03-02 19:00:00',NULL,'UTC',NULL,'2026-03-02 15:00:00','2026-03-02 15:00:00')")).await;
}

fn validation(reply: &Reply, field: &str, expected: &str) {
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.text()
    );
    let api::ApiError::Validation { message, fields } = parse::<api::ApiErrorResponse>(reply).error
    else {
        panic!("{}", reply.text())
    };
    assert_eq!(message, expected);
    assert_eq!(fields, [(field.into(), vec![expected.into()])].into());
}

async fn add(browser: &mut Browser<'_>, thread_id: i64, input: Value) -> api::ThreadDetail {
    let reply = browser
        .write(json_body(Method::POST, &path(thread_id), &input))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    parse(&reply)
}

fn links(detail: &api::ThreadDetail) -> &[api::WorkLink] {
    &detail.thread.work.as_ref().unwrap().links
}

#[tokio::test]
async fn spa_api_work_links_scope_checks_visibility_then_tracking_for_every_route() {
    let Some(a) = app(true).await else { return };
    let mut kevin = a.sign_in(KEVIN).await;
    for (thread, expected) in [
        (THREAD, StatusCode::UNPROCESSABLE_ENTITY),
        (BOARD_THREAD, StatusCode::NOT_FOUND),
        (9999999, StatusCode::NOT_FOUND),
    ] {
        let replies = [
            kevin.send(get(&format!("{}/new", path(thread)))).await,
            kevin
                .write(json_body(
                    Method::POST,
                    &path(thread),
                    &json!({"kind":"event"}),
                ))
                .await,
            kevin
                .write(json_body(
                    Method::DELETE,
                    &format!("{}/1", path(thread)),
                    &json!({}),
                ))
                .await,
        ];
        for reply in replies {
            assert_eq!(reply.status, expected, "{}", reply.text());
            if expected == StatusCode::UNPROCESSABLE_ENTITY {
                validation(&reply, "base", "This thread isn't tracked as work");
            } else {
                assert_eq!(tag(&reply), "NotFound");
            }
        }
    }
    track(&a).await;
    let detail: api::ThreadDetail = parse(&kevin.send(get("/api/v1/threads/1")).await);
    assert!(
        !detail.permissions.can_manage_work,
        "a viewer has no management rights"
    );
    assert_eq!(
        kevin
            .send(get(&format!("{}/new", path(THREAD))))
            .await
            .status,
        StatusCode::OK
    );
    let detail = add(&mut kevin, THREAD, json!({"kind":"drive_file","driveUrl":"https://docs.google.com/document/d/1234567890/edit"})).await;
    let reply = kevin
        .write(json_body(
            Method::DELETE,
            &format!("{}/{}", path(THREAD), links(&detail)[0].id),
            &json!({}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert!(links(&parse(&reply)).is_empty());
}

#[tokio::test]
async fn spa_api_work_links_picker_and_event_builder_match_classic() {
    let Some(a) = app(true).await else { return };
    track(&a).await;
    events(&a).await;
    let mut viewer = a.sign_in(KEVIN).await;
    let reply = viewer.send(get(&format!("{}/new", path(THREAD)))).await;
    assert_eq!(reply.header("cache-control"), Some("no-store"));
    let form: api::WorkLinkForm = parse(&reply);
    let candidates: Vec<_> = form
        .events
        .iter()
        .filter(|event| (EVENT..=FOREIGN_EVENT).contains(&event.id))
        .collect();
    assert_eq!(
        candidates.iter().map(|event| event.id).collect::<Vec<_>>(),
        [9000003, EVENT, 9000002]
    );
    assert_eq!(
        (
            &candidates[1].title,
            &candidates[1].starts_at,
            &candidates[1].time_zone
        ),
        (
            &"Review".to_string(),
            &"2026-03-02T17:00:00.000Z".to_string(),
            &"America/New_York".to_string()
        )
    );
    let detail = add(&mut viewer, THREAD, json!({"kind":"event","eventId":EVENT})).await;
    let link = &links(&detail)[0];
    assert_eq!(
        (link.kind, link.label.as_str(), link.url.as_str()),
        (
            api::WorkLinkKind::Event,
            "Review",
            "/rooms/654632876/events/9000001"
        )
    );
    assert_eq!(
        link.event_starts_at.as_deref(),
        Some("2026-03-02T17:00:00.000Z")
    );
    let form: api::WorkLinkForm = parse(&viewer.send(get(&format!("{}/new", path(THREAD)))).await);
    assert!(!form.events.iter().any(|event| event.id == EVENT));
    for event in [9000004, 9000005] {
        let detail = add(&mut viewer, THREAD, json!({"kind":"event","eventId":event})).await;
        assert!(
            links(&detail)
                .iter()
                .any(|link| link.url.ends_with(&format!("/{event}")))
        );
    }
    for event in [FOREIGN_EVENT, 9999999] {
        let reply = viewer
            .write(json_body(
                Method::POST,
                &path(THREAD),
                &json!({"kind":"event","eventId":event}),
            ))
            .await;
        assert_eq!(
            (reply.status, tag(&reply)),
            (StatusCode::NOT_FOUND, "NotFound".into())
        );
    }
}

#[tokio::test]
async fn spa_api_work_links_builders_preserve_classic_alerts_and_duplicates() {
    let Some(a) = app(true).await else { return };
    let a = a.without_job_runner().await;
    track(&a).await;
    events(&a).await;
    let mut viewer = a.sign_in(KEVIN).await;
    for (input, field, message) in [
        (
            json!({"kind":"pull_request"}),
            "pullRequestUrl",
            "Enter a GitHub pull request URL, like https://github.com/owner/repo/pull/123.",
        ),
        (
            json!({"kind":"pull_request","pullRequestUrl":"http://github.com/owner/repo/pull/123"}),
            "pullRequestUrl",
            "Enter a GitHub pull request URL, like https://github.com/owner/repo/pull/123.",
        ),
        (
            json!({"kind":"event"}),
            "eventId",
            "Choose an event to link.",
        ),
        (
            json!({"kind":"event","eventId":null}),
            "eventId",
            "Choose an event to link.",
        ),
        (
            json!({"kind":"drive_file"}),
            "driveUrl",
            "Enter a Google Drive, Docs, Sheets, Slides, or Forms link.",
        ),
        (
            json!({"kind":"drive_file","driveUrl":"https://example.com/file/d/1234567890"}),
            "driveUrl",
            "Enter a Google Drive, Docs, Sheets, Slides, or Forms link.",
        ),
        (
            json!({}),
            "kind",
            "Choose a pull request, event, or Drive file to link.",
        ),
        (
            json!({"kind":"other"}),
            "kind",
            "Choose a pull request, event, or Drive file to link.",
        ),
    ] {
        validation(
            &viewer
                .write(json_body(Method::POST, &path(THREAD), &input))
                .await,
            field,
            message,
        );
    }
    for (input, field) in [
        (
            json!({"kind":"pull_request","pullRequestUrl":"See https://github.com/Smartfire/Test/pulls/00123/files"}),
            "pullRequestUrl",
        ),
        (json!({"kind":"event","eventId":EVENT}), "eventId"),
        (
            json!({"kind":"drive_file","driveUrl":"  https://docs.google.com/document/d/1234567890/edit \n"}),
            "driveUrl",
        ),
    ] {
        let detail = add(&mut viewer, THREAD, input.clone()).await;
        let link = links(&detail).last().unwrap();
        match field {
            "pullRequestUrl" => {
                assert_eq!(link.label, "smartfire/test#123");
                assert_eq!(link.url, "https://github.com/smartfire/test/pull/123");
            }
            "driveUrl" => {
                assert_eq!(
                    link.url,
                    "https://docs.google.com/document/d/1234567890/edit"
                );
                assert_eq!(link.label, link.url);
            }
            _ => (),
        }
        validation(
            &viewer
                .write(json_body(Method::POST, &path(THREAD), &input))
                .await,
            field,
            "That is already linked to this work thread.",
        );
    }
    validation(&viewer.write(json_body(Method::POST, &path(THREAD), &json!({"kind":"pull_request","pullRequestUrl":"https://github.com/owner/repo/pull/0"}))).await,"pullRequestUrl","Number must be greater than 0");
}

#[tokio::test]
async fn spa_api_work_links_destroy_is_scoped_to_the_thread() {
    let Some(a) = app(true).await else { return };
    track(&a).await;
    let mut viewer = a.sign_in(KEVIN).await;
    let detail = add(
        &mut viewer,
        8,
        json!({"kind":"drive_file","driveUrl":"https://drive.google.com/file/d/1234567890/view"}),
    )
    .await;
    let id = links(&detail)[0].id;
    for target in [id, 9999999] {
        let reply = viewer
            .write(json_body(
                Method::DELETE,
                &format!("{}/{target}", path(THREAD)),
                &json!({}),
            ))
            .await;
        assert_eq!(
            (reply.status, tag(&reply)),
            (StatusCode::NOT_FOUND, "NotFound".into())
        );
    }
    let reply = viewer
        .write(json_body(
            Method::DELETE,
            &format!("{}/{id}", path(8)),
            &json!({}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert!(links(&parse(&reply)).is_empty());
}

#[tokio::test]
async fn spa_api_work_links_pr_fetch_is_claimed_once_and_rolls_back_with_link() {
    let Some(a) = app(true).await else { return };
    let a = a.without_job_runner().await;
    track(&a).await;
    let mut viewer = a.sign_in(KEVIN).await;
    for thread in [THREAD, 8] {
        add(&mut viewer,thread,json!({"kind":"pull_request","pullRequestUrl":"https://github.com/link-test/repo/pull/123"})).await;
    }
    a.db().read(|conn| {
        let id: i64 = conn.query_row("SELECT id FROM github_pull_requests WHERE owner='link-test' AND repo='repo' AND number=123", [], |row| row.get(0))?;
        assert!(crate::integrations::github::pull_requests::PullRequest::find(conn,id)?.fetch_requested_at.is_some());
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Github::FetchPullRequestJob' AND json_extract(arguments,'$.pull_request_id')=?",[id],|row|row.get::<_,i64>(0))?,1);
        Ok(())
    }).await.unwrap();
    sql(&a,"CREATE TRIGGER reject_api_link_fetch BEFORE INSERT ON background_jobs WHEN NEW.job_class='Github::FetchPullRequestJob' BEGIN SELECT RAISE(ABORT,'rejected API link fetch'); END").await;
    let reply = viewer.write(json_body(Method::POST,&path(THREAD),&json!({"kind":"pull_request","pullRequestUrl":"https://github.com/link-test/repo/pull/124"}))).await;
    assert_eq!(reply.status, StatusCode::INTERNAL_SERVER_ERROR);
    a.db().read(|conn| {
        let id: i64 = conn.query_row("SELECT id FROM github_pull_requests WHERE owner='link-test' AND repo='repo' AND number=124",[],|row|row.get(0))?;
        assert!(crate::integrations::github::pull_requests::PullRequest::find(conn,id)?.fetch_requested_at.is_none());
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM work_thread_links WHERE github_pull_request_id=?",[id],|row|row.get::<_,i64>(0))?,0);
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Github::FetchPullRequestJob' AND json_extract(arguments,'$.pull_request_id')=?",[id],|row|row.get::<_,i64>(0))?,0);
        Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn spa_api_work_links_writes_publish_thread_updated_once_per_topic() {
    let Some(a) = app(true).await else { return };
    track(&a).await;
    let mut viewer = a.sign_in(KEVIN).await;
    let (addr, server) = serve(&a).await;
    let topics = [format!("room:{ROOM}"), format!("thread:{THREAD}")];
    let mut sync = Sync::connect(addr, &viewer.cookie_header(), &topics).await;
    sync.welcome().await;
    let detail = add(
        &mut viewer,
        THREAD,
        json!({"kind":"drive_file","driveUrl":"https://drive.google.com/file/d/1234567890/view"}),
    )
    .await;
    let id = links(&detail)[0].id;
    for index in 0..2 {
        let expected = if index == 0 {
            detail.thread.clone()
        } else {
            let reply = viewer
                .write(json_body(
                    Method::DELETE,
                    &format!("{}/{id}", path(THREAD)),
                    &json!({}),
                ))
                .await;
            assert_eq!(reply.status, StatusCode::OK);
            parse::<api::ThreadDetail>(&reply).thread
        };
        let mut seen = Vec::new();
        while seen.len() < topics.len() {
            let event = sync.until(|event| topics.contains(&event.topic) && !seen.contains(&event.topic) && matches!(&event.payload,api::SyncPayload::ThreadUpdated(thread) if thread.id == THREAD), |event| seen.contains(&event.topic) && matches!(&event.payload,api::SyncPayload::ThreadUpdated(thread) if thread.id == THREAD)).await;
            seen.push(event.topic);
            let api::SyncPayload::ThreadUpdated(thread) = event.payload else {
                unreachable!()
            };
            assert_eq!(
                thread.work.as_ref().unwrap().links.len(),
                if index == 0 { 1 } else { 0 }
            );
            assert_eq!(thread.work, expected.work);
        }
    }
    tokio::time::timeout(
        Duration::from_secs(5),
        a.booted.app.broadcasts.settle_sync(),
    )
    .await
    .unwrap();
    viewer
        .write(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{ROOM}/messages"),
            &json!({"clientMessageId":"links-marker","markdownSource":"links-marker"}),
        ))
        .await;
    sync.until(|event|matches!(&event.payload,api::SyncPayload::MessageCreated(message) if message.client_message_id == "links-marker"),|event|matches!(&event.payload,api::SyncPayload::ThreadUpdated(thread) if thread.id == THREAD)).await;
    server.abort();
}
