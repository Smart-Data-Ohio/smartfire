//! Every broadcast point has a JSON twin on the SPA's sync socket, or is on the explicit list of
//! ones still to port (`frontend-plan.md` §6.3); every sync event the contract defines is
//! published by a twin, or is on the explicit list of ones not emitted yet. Reads the sources, so
//! a new sink kind, `Broadcasts` method or `SyncPayload` variant fails here until it's listed in
//! `campfire_app::cable::sync`.

use std::collections::BTreeSet;
use std::path::Path;

use campfire_app::cable::sync::{NOT_YET_EMITTED, NOT_YET_TWINNED, TWINS};
use regex::Regex;

/// `Broadcasts`' building blocks, which the named broadcast points call: not points themselves.
const PRIMITIVES: &[&str] = &[
    "new",
    "install_sync_renderer",
    "sync_message",
    "sync_thread_indicator",
    "sync_unread_rows",
    "sync_membership_row",
    "sync_read_row",
    "sync_activity_stream",
    "sync_activity_item",
    "sync_organized",
    "sync_activity_removed",
    "sync_scheduled",
    "sync_agent_status",
    "sync_agent_steps",
    "sync_approval",
    "sync_poll",
    "sync_message_cards",
    "turbo",
    "append",
    "prepend",
    "replace",
    "update",
    "remove",
    "channel",
];

/// Points the sources don't declare in a way this test reads: the free function every read goes
/// through, the channel whose frames the clients send, and the API's saved items (the classic
/// app has no broadcast for them).
const FIXED: &[&str] = &[
    "broadcasts::read_room",
    "TypingNotificationsChannel",
    "campfire_api::saved_items",
];

fn read(path: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    std::fs::read_to_string(root.join(path)).unwrap_or_else(|error| panic!("{path}: {error}"))
}

/// The body of the item that starts at `start`, up to the first line that is just `}`.
fn item<'a>(source: &'a str, start: &str) -> &'a str {
    let body = &source[source.find(start).unwrap_or_else(|| panic!("{start}"))..];
    &body[..body.find("\n}\n").unwrap_or_else(|| panic!("the end of {start}"))]
}

/// The kinds the cable sink's `broadcast` matches on, named by their type's path.
fn sink_kinds() -> BTreeSet<String> {
    let source = read("channels/src/channels/sink.rs");
    let arm = Regex::new(r"(?m)^\s*([A-Za-z_][A-Za-z0-9_:]*)::KIND\s*=>").unwrap();
    arm.captures_iter(item(&source, "fn broadcast("))
        .map(|captures| {
            let path = &captures[1];
            ["campfire_db::models::", "crate::integrations::", "campfire_db::"]
                .iter()
                .find_map(|prefix| path.strip_prefix(prefix))
                .unwrap_or(path)
                .to_string()
        })
        .collect()
}

/// `Broadcasts`' public methods, less its primitives.
fn named_broadcasts() -> BTreeSet<String> {
    let source = read("app/src/cable/broadcasts.rs");
    let method = Regex::new(r"(?m)^    pub fn ([a-z_0-9]+)").unwrap();
    method
        .captures_iter(item(&source, "impl Broadcasts {"))
        .map(|captures| captures[1].to_string())
        .filter(|name| !PRIMITIVES.contains(&name.as_str()))
        .map(|name| format!("Broadcasts::{name}"))
        .collect()
}

/// The sync events the contract defines: `SyncPayload`'s `type` tags.
fn events() -> BTreeSet<String> {
    let source = read("api_types/src/sync.rs");
    let tag = Regex::new(r#"#\[serde\(rename = "([a-z.]+)"\)\]"#).unwrap();
    tag.captures_iter(item(&source, "pub enum SyncPayload {"))
        .map(|captures| captures[1].to_string())
        .collect()
}

#[test]
fn every_broadcast_point_has_a_sync_event_or_is_listed_as_not_yet_twinned() {
    let kinds = sink_kinds();
    let methods = named_broadcasts();
    // A read that found nothing would pass vacuously.
    assert!(kinds.len() >= 20, "found only {} sink kinds: {kinds:?}", kinds.len());
    assert!(methods.len() >= 15, "found only {} Broadcasts methods: {methods:?}", methods.len());
    let points: BTreeSet<String> = kinds
        .into_iter()
        .chain(methods)
        .chain(FIXED.iter().map(|point| point.to_string()))
        .collect();
    let twinned: BTreeSet<&str> = TWINS.iter().map(|(point, _)| *point).collect();
    let pending: BTreeSet<&str> = NOT_YET_TWINNED.iter().copied().collect();

    let missing: Vec<&String> = points
        .iter()
        .filter(|point| !twinned.contains(point.as_str()) && !pending.contains(point.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "broadcast points with no sync event: give each a twin (campfire_app::cable::sync::TWINS) or list it in NOT_YET_TWINNED: {missing:?}"
    );
    let stale: Vec<&&str> = twinned.iter().chain(&pending).filter(|point| !points.contains(**point)).collect();
    assert!(stale.is_empty(), "listed broadcast points that no longer exist: {stale:?}");
    let events = events();
    for (point, named) in TWINS {
        assert!(!pending.contains(point), "{point} is both twinned and not yet twinned");
        for event in *named {
            assert!(events.contains(*event), "{point} names an unknown sync event {event}");
        }
    }
}

#[test]
fn every_sync_event_is_published_or_listed_as_not_yet_emitted() {
    let events = events();
    assert!(events.len() >= 18, "found only {} sync events: {events:?}", events.len());
    let published: BTreeSet<&str> = TWINS.iter().flat_map(|(_, named)| named.iter().copied()).collect();
    let pending: BTreeSet<&str> = NOT_YET_EMITTED.iter().copied().collect();
    let silent: Vec<&String> = events
        .iter()
        .filter(|event| !published.contains(event.as_str()) && !pending.contains(event.as_str()))
        .collect();
    assert!(
        silent.is_empty(),
        "sync events no broadcast point publishes: twin one (TWINS) or list it in NOT_YET_EMITTED: {silent:?}"
    );
    let both: Vec<&&str> = published.intersection(&pending).collect();
    assert!(both.is_empty(), "published sync events still listed in NOT_YET_EMITTED: {both:?}");
    let stale: Vec<&&str> = pending.iter().filter(|event| !events.contains(**event)).collect();
    assert!(stale.is_empty(), "NOT_YET_EMITTED names events the contract doesn't define: {stale:?}");
}

#[test]
fn the_events_read_are_the_contracts() {
    use campfire_api_types::{RoomRead, SyncPayload};
    // Every tag read from the source parses as a SyncPayload: an unknown tag fails before the
    // data is looked at.
    for event in events() {
        let parsed = serde_json::from_value::<SyncPayload>(serde_json::json!({"type": event, "data": {}}));
        if let Err(error) = parsed {
            assert!(!error.to_string().contains("unknown variant"), "{event}: {error}");
        }
    }
    let parsed: SyncPayload = serde_json::from_value(serde_json::json!({"type": "room.read", "data": {"roomId": 1}})).unwrap();
    assert_eq!(parsed, SyncPayload::RoomRead(RoomRead { room_id: 1 }));
}

#[tokio::test]
async fn approval_sync_users_carry_their_row_timestamps() {
    use crate::controllers::presenters::test_support::{
        BENDER, DAVID, JASON, Req, TestApp, seed_clock,
    };
    use axum::http::{Method, StatusCode};
    use campfire_api_types as api;
    use futures_util::{SinkExt, StreamExt};
    use serde_json::json;
    use std::collections::BTreeMap;
    use std::time::Duration;
    use tokio_tungstenite::{
        connect_async,
        tungstenite::{Message, client::IntoClientRequest},
    };

    let Some(a) =
        TestApp::boot_seed_with_env("default", seed_clock(), &[("SPA_ENABLED", "1")]).await
    else {
        return;
    };
    let mut david = a.sign_in(DAVID).await;
    let jason = a.sign_in(JASON).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let router = a.booted.router.clone();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut request = format!("ws://{addr}/api/v1/sync")
        .into_client_request()
        .unwrap();
    request
        .headers_mut()
        .insert("cookie", jason.cookie_header().parse().unwrap());
    request
        .headers_mut()
        .insert("host", "campfire.test".parse().unwrap());
    request
        .headers_mut()
        .insert("origin", "http://campfire.test".parse().unwrap());
    let (mut socket, _) = connect_async(request).await.unwrap();
    socket
        .send(Message::Text(
            json!({"t": "hello", "v": 1, "resume": null, "topics": []})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    let welcome = tokio::time::timeout(Duration::from_secs(10), socket.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let Message::Text(welcome) = welcome else {
        panic!("expected sync welcome")
    };
    assert!(matches!(
        serde_json::from_str::<api::ServerFrame>(&welcome).unwrap(),
        api::ServerFrame::Welcome { .. }
    ));

    a.db()
        .write(|tx| {
            for (id, at) in [
                (BENDER, "2026-03-02T15:00:00.123456Z"),
                (DAVID, "2026-03-02T15:00:00.123789Z"),
            ] {
                tx.conn().execute(
                    "UPDATE users SET updated_at=? WHERE id=?",
                    rusqlite::params![campfire_db::Timestamp::from_jiff(at.parse().unwrap()), id],
                )?;
            }
            Ok(())
        })
        .await
        .unwrap();
    let reply = david
        .write(
            Req::new(Method::PATCH, "/api/v1/agent_approvals/1")
                .header("accept", "application/json")
                .header("content-type", "application/json")
                .body(json!({"decision": "approved", "note": "Ship it"}).to_string()),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());

    // Read Jason's real sync socket. A missing publisher times out instead of passing on an HTTP DTO.
    let event = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let frame = socket.next().await.expect("sync stays open").expect("sync frame");
            if let Message::Text(text) = frame
                && let api::ServerFrame::Batch { events } = serde_json::from_str(&text).unwrap()
                && let Some(event) = events.into_iter().find(|event| {
                    matches!(&event.payload, api::SyncPayload::ApprovalUpdated(update) if update.approval.id == 1)
                })
            {
                break event;
            }
        }
    }).await.expect("approval.updated must actually be published");
    assert_eq!(event.topic, "user");
    let api::SyncPayload::ApprovalUpdated(update) = event.payload else {
        unreachable!()
    };
    assert_eq!(update.approval.status, api::AgentApprovalStatus::Approved);
    assert_eq!(update.approval.decided_by_id, Some(DAVID));
    let expected = a
        .db()
        .read(|conn| {
            [BENDER, DAVID]
                .into_iter()
                .map(|id| {
                    campfire_db::User::find(conn, id).map(|user| {
                        (
                            id,
                            user.updated_at
                                .jiff()
                                .strftime("%Y-%m-%dT%H:%M:%S%.6fZ")
                                .to_string(),
                        )
                    })
                })
                .collect::<campfire_db::Result<BTreeMap<_, _>>>()
        })
        .await
        .unwrap();
    assert_eq!(expected[&BENDER], "2026-03-02T15:00:00.123456Z");
    assert_eq!(expected[&DAVID], "2026-03-02T15:00:00.123789Z");
    assert_eq!(update.users.len(), expected.len());
    let published: BTreeMap<_, _> = update
        .users
        .into_iter()
        .map(|user| (user.id, user.updated_at))
        .collect();
    assert_eq!(published, expected);
    server.abort();
}
