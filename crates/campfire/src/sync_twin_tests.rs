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
    "sync_activity_stream",
    "sync_activity_item",
    "sync_organized",
    "sync_activity_removed",
    "sync_scheduled",
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
    use campfire_api_types as api;
    use crate::controllers::presenters::test_support::{BENDER, DAVID, Req, TestApp, seed_clock};

    let Some(a) = TestApp::boot_seed_with_env("default", seed_clock(), &[("SPA_ENABLED", "1")]).await
    else { return };
    a.db().write(|tx| {
        tx.conn().execute(
            "UPDATE users SET updated_at=? WHERE id IN (?,?)",
            rusqlite::params![tx.now(), BENDER, DAVID],
        )?;
        Ok(())
    }).await.unwrap();
    let mut b = a.sign_in(DAVID).await;
    let reply = b.send(
        Req::new(axum::http::Method::GET, &format!("/api/v1/users?ids={BENDER},{DAVID}"))
            .header("accept", "application/json"),
    ).await;
    assert_eq!(reply.status, axum::http::StatusCode::OK, "{}", reply.text());
    let users: api::UserList = serde_json::from_slice(&reply.body).unwrap();
    let now = a.db().env().now();
    let timestamps = a.db().read(move |conn| {
        [BENDER, DAVID].into_iter().map(|id| {
            campfire_db::User::find(conn, id).map(|user| (id, user.updated_at.to_wire()))
        }).collect::<campfire_db::Result<Vec<_>>>()
    }).await.unwrap();
    let payload = api::SyncPayload::ApprovalUpdated(api::ApprovalUpdated {
        approval: api::AgentApproval {
            id: 1,
            agent_id: 1,
            agent_user_id: BENDER,
            room_id: None,
            room_name: None,
            action: "github.merge_pull_request".into(),
            summary: "Merge the reviewed change".into(),
            status: api::AgentApprovalStatus::Approved,
            expires_at: now.to_wire(),
            created_at: now.to_wire(),
            decided_by_id: Some(DAVID),
            decided_at: Some(now.to_wire()),
            decision_note: None,
            github_login: None,
            fizzy_user_name: None,
            admin_only: true,
            approvable: true,
            deniable: true,
        },
        users: users.users,
    });
    let wire = serde_json::to_value(&payload).unwrap();
    assert_eq!(wire["type"], "approval.updated");
    let users = wire["data"]["users"].as_array().unwrap();
    assert_eq!(users.len(), timestamps.len());
    for (id, updated_at) in timestamps {
        let user = users.iter().find(|user| user["id"] == id).unwrap();
        assert_eq!(user["updatedAt"], updated_at);
    }
    assert_eq!(serde_json::from_value::<api::SyncPayload>(wire).unwrap(), payload);
}
