//! Every broadcast point has a JSON twin on the SPA's sync socket, or is on the explicit list of
//! ones still to port (`frontend-plan.md` §6.3). Reads the sources, so a new sink kind or a new
//! `Broadcasts` method fails here until it's listed in `campfire_app::cable::sync`.

use std::collections::BTreeSet;
use std::path::Path;

use campfire_app::cable::sync::{NOT_YET_TWINNED, TWINS};
use regex::Regex;

/// `Broadcasts`' building blocks, which the named broadcast points call: not points themselves.
const PRIMITIVES: &[&str] = &[
    "new",
    "install_sync_renderer",
    "sync_message",
    "turbo",
    "append",
    "prepend",
    "replace",
    "update",
    "remove",
    "channel",
];

/// Points the sources don't declare in a way this test reads: the free function every read goes
/// through, and the channel whose frames the clients send.
const FIXED: &[&str] = &["broadcasts::read_room", "TypingNotificationsChannel"];

/// The sync events the contract defines (`SyncPayload`'s `type` tags).
const EVENTS: &[&str] = &[
    "message.created",
    "message.updated",
    "message.removed",
    "typing",
    "room.unread",
    "room.read",
    "sidebar.row.upserted",
    "sidebar.row.removed",
    "presence",
];

fn read(path: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    std::fs::read_to_string(root.join(path)).unwrap_or_else(|error| panic!("{path}: {error}"))
}

/// The kinds the cable sink's `broadcast` matches on, named by their type's path.
fn sink_kinds() -> BTreeSet<String> {
    let source = read("channels/src/channels/sink.rs");
    let body = &source[source
        .find("fn broadcast(")
        .expect("the sink's broadcast fn")..];
    let body = &body[..body.find("\n}\n").expect("the end of broadcast")];
    let arm = Regex::new(r"(?m)^\s*([A-Za-z_][A-Za-z0-9_:]*)::KIND\s*=>").unwrap();
    arm.captures_iter(body)
        .map(|captures| {
            let path = &captures[1];
            [
                "campfire_db::models::",
                "crate::integrations::",
                "campfire_db::",
            ]
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
    let body = &source[source.find("impl Broadcasts {").expect("impl Broadcasts")..];
    let body = &body[..body.find("\n}\n").expect("the end of impl Broadcasts")];
    let method = Regex::new(r"(?m)^    pub fn ([a-z_0-9]+)").unwrap();
    method
        .captures_iter(body)
        .map(|captures| captures[1].to_string())
        .filter(|name| !PRIMITIVES.contains(&name.as_str()))
        .map(|name| format!("Broadcasts::{name}"))
        .collect()
}

#[test]
fn every_broadcast_point_has_a_sync_event_or_is_listed_as_not_yet_twinned() {
    let kinds = sink_kinds();
    let methods = named_broadcasts();
    // A read that found nothing would pass vacuously.
    assert!(
        kinds.len() >= 20,
        "found only {} sink kinds: {kinds:?}",
        kinds.len()
    );
    assert!(
        methods.len() >= 15,
        "found only {} Broadcasts methods: {methods:?}",
        methods.len()
    );
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
    let stale: Vec<&&str> = twinned
        .iter()
        .chain(&pending)
        .filter(|point| !points.contains(**point))
        .collect();
    assert!(
        stale.is_empty(),
        "listed broadcast points that no longer exist: {stale:?}"
    );
    for (point, events) in TWINS {
        assert!(
            !pending.contains(point),
            "{point} is both twinned and not yet twinned"
        );
        for event in *events {
            assert!(
                EVENTS.contains(event),
                "{point} names an unknown sync event {event}"
            );
        }
    }
}

#[test]
fn the_events_listed_are_the_contracts() {
    use campfire_api_types::{RoomRead, SyncPayload};
    // Every tag in EVENTS parses as a SyncPayload (the one with no data checked here stands in
    // for the rest: an unknown tag fails before the data is looked at).
    for event in EVENTS {
        let parsed =
            serde_json::from_value::<SyncPayload>(serde_json::json!({"type": event, "data": {}}));
        if let Err(error) = parsed {
            assert!(
                !error.to_string().contains("unknown variant"),
                "{event}: {error}"
            );
        }
    }
    let parsed: SyncPayload =
        serde_json::from_value(serde_json::json!({"type": "room.read", "data": {"roomId": 1}}))
            .unwrap();
    assert_eq!(parsed, SyncPayload::RoomRead(RoomRead { room_id: 1 }));
}
