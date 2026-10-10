//! The screen map: its rows against the route table, both directions of the mapping, and the
//! copy the SPA reads.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::*;

/// `pattern` with every parameter filled by `ids`, in order.
fn sample(pattern: &str, ids: &[u64]) -> String {
    let named: Vec<(&str, u64)> = params(pattern)
        .into_iter()
        .zip(ids.iter().copied())
        .collect();
    fill(pattern, &named)
}

/// A route table spec with its `/users/:user_id/` spelled `/users/me/`, as the screen map spells a
/// person's own pages.
fn own_pages(spec: &str) -> String {
    spec.replacen("/users/:user_id/", "/users/me/", 1)
}

#[test]
fn every_row_is_a_get_route_of_the_classic_table() {
    for screen in SCREENS {
        let spec = if screen.classic == "/" {
            "/".to_string()
        } else {
            format!("{}(.:format)", screen.classic)
        };
        assert!(
            campfire_routes::TABLE
                .iter()
                .any(|route| route.verb == "GET"
                    && route.endpoint == screen.endpoint
                    && (route.spec == spec || own_pages(route.spec) == spec)),
            "{} {spec} isn't in the route table",
            screen.endpoint
        );
    }
}

#[test]
fn each_row_maps_the_same_parameters_both_ways() {
    for screen in SCREENS {
        let classic: BTreeSet<_> = params(screen.classic).into_iter().collect();
        let spa: BTreeSet<_> = params(screen.spa).into_iter().collect();
        assert_eq!(classic, spa, "{screen:?}");
        assert!(screen.spa.starts_with("/app/"), "{screen:?}");
    }
}

/// The row a classic page redirects to: the first for its endpoint and pattern.
fn first_for(screen: &Screen) -> &'static Screen {
    SCREENS
        .iter()
        .find(|row| row.endpoint == screen.endpoint && row.classic == screen.classic)
        .unwrap()
}

/// The classic fallback for an SPA URL, including aliases with differently named parameters.
fn first_for_spa(screen: &Screen) -> &'static Screen {
    let path = sample(screen.spa, &[7, 8, 9]);
    SCREENS
        .iter()
        .find(|row| captures(row.spa, &path).is_some())
        .unwrap()
}

#[test]
fn only_intentional_aliases_share_a_url() {
    let rows: BTreeSet<_> = SCREENS
        .iter()
        .map(|screen| (screen.endpoint, screen.classic, screen.spa))
        .collect();
    assert_eq!(rows.len(), SCREENS.len(), "no duplicate rows");
    let mut aliases = BTreeMap::<_, Vec<_>>::new();
    for screen in SCREENS {
        aliases
            .entry(sample(screen.spa, &[7, 8, 9]))
            .or_default()
            .push((screen.endpoint, screen.classic));
    }
    aliases.retain(|_, rows| rows.len() > 1);
    assert_eq!(
        aliases,
        BTreeMap::from([
            (
                "/app/settings".to_string(),
                vec![
                    ("users/profiles#show", "/users/me/profile"),
                    ("users/profiles#edit", "/users/me/profile/edit"),
                ],
            ),
            (
                "/app/people/7".to_string(),
                vec![
                    ("users#show", "/users/:id"),
                    ("users/profiles#show", "/users/:user_id/profile"),
                    ("users/profiles#edit", "/users/:user_id/profile/edit"),
                ],
            ),
            (
                "/app/r/7/m/8".to_string(),
                vec![
                    ("rooms#show", "/rooms/:room_id/@:message_id"),
                    ("messages#show", "/rooms/:room_id/messages/:id"),
                    ("messages#edit", "/rooms/:room_id/messages/:id/edit"),
                ],
            ),
            (
                "/app/m/7".to_string(),
                vec![
                    ("messages#show", "/messages/:id"),
                    ("messages#edit", "/messages/:id/edit"),
                    ("messages/boosts#index", "/messages/:message_id/boosts"),
                    ("messages/boosts#new", "/messages/:message_id/boosts/new"),
                ],
            ),
            (
                "/app/r/7/settings".to_string(),
                vec![
                    ("rooms#show", "/rooms/:id"),
                    ("rooms/opens#edit", "/rooms/opens/:id/edit"),
                    ("rooms/closeds#edit", "/rooms/closeds/:id/edit"),
                    ("rooms/voices#edit", "/rooms/voices/:id/edit"),
                    ("rooms/stages#edit", "/rooms/stages/:id/edit"),
                    ("rooms/boards#edit", "/rooms/boards/:id/edit"),
                ],
            ),
        ]),
        "only the message, room settings and profile aliases may share SPA URLs, \
         with their fallback first"
    );
    assert!(SCREENS.iter().all(|screen| {
        std::ptr::eq(first_for_spa(screen), screen)
            || (screen.ported && first_for_spa(screen).ported)
    }));
    // A classic page shared by several rows: only the profile page's sections, the account
    // page's people list and the room page as the settings screen's fallback, all ported.
    for screen in SCREENS {
        let first = first_for(screen);
        if !std::ptr::eq(first, screen) {
            assert!(
                ["users/profiles#show", "accounts#edit", "rooms#show"].contains(&screen.endpoint),
                "{screen:?}"
            );
            assert!(screen.ported && first.ported, "{screen:?}");
        }
    }
}

#[test]
fn the_profile_sections_map_back_to_the_profile_page() {
    for section in ["notifications", "appearance", "calls", "integrations"] {
        assert_eq!(
            classic_url(&format!("/app/settings/{section}"), None).as_deref(),
            Some("/users/me/profile"),
            "{section}"
        );
    }
    assert_eq!(
        spa_url("users/profiles#show", "/users/me/profile", None).as_deref(),
        Some("/app/settings")
    );
}

#[test]
fn the_account_pages_map_to_the_admin_sections() {
    for (endpoint, classic, spa) in [
        ("accounts#edit", "/account/edit", "/app/admin"),
        ("accounts/icons#index", "/account/icons", "/app/admin/icons"),
        (
            "accounts/custom_styles#edit",
            "/account/custom_styles/edit",
            "/app/admin/styles",
        ),
        (
            "accounts/audit_logs#show",
            "/account/audit_log",
            "/app/admin/audit-log",
        ),
        (
            "accounts/integrations_health#show",
            "/account/integrations_health",
            "/app/admin/integrations",
        ),
    ] {
        assert_eq!(spa_url(endpoint, classic, None).as_deref(), Some(spa));
        assert_eq!(classic_url(spa, None).as_deref(), Some(classic));
    }
    // The people list is the account page's lower half.
    assert_eq!(
        classic_url("/app/admin/people", None).as_deref(),
        Some("/account/edit")
    );
}

#[test]
fn ported_classic_pages_map_to_their_spa_urls() {
    assert_eq!(spa_url("welcome#show", "/", None).as_deref(), Some("/app/"));
    assert_eq!(
        spa_url("rooms#show", "/rooms/12", None).as_deref(),
        Some("/app/r/12")
    );
    assert_eq!(
        spa_url("rooms#show", "/rooms/12/@345", None).as_deref(),
        Some("/app/r/12/m/345")
    );
    assert_eq!(
        spa_url("channel_threads#show", "/rooms/12/threads/9", None).as_deref(),
        Some("/app/r/12/t/9")
    );
    // Journey's normalization: repeated and trailing slashes.
    assert_eq!(
        spa_url("rooms#show", "//rooms/12/", None).as_deref(),
        Some("/app/r/12")
    );
}

#[test]
fn board_pages_and_workspace_work_are_ported() {
    for (endpoint, classic, spa) in [
        ("rooms#show", "/rooms/12", "/app/r/12"),
        (
            "channel_threads#show",
            "/rooms/12/threads/9",
            "/app/r/12/t/9",
        ),
        (
            "channel_threads#new",
            "/rooms/12/threads/new",
            "/app/r/12/posts/new",
        ),
        (
            "rooms/boards/automations#show",
            "/rooms/boards/12/automations",
            "/app/r/12/automations",
        ),
        ("work_threads#index", "/work", "/app/work"),
        (
            "threads/work/handoffs#new",
            "/threads/9/work/handoff/new",
            "/app/t/9/handoff",
        ),
        (
            "threads/work/links#index",
            "/threads/9/work/links",
            "/app/t/9/links",
        ),
    ] {
        assert_eq!(spa_url(endpoint, classic, None).as_deref(), Some(spa));
        assert_eq!(classic_url(spa, None).as_deref(), Some(classic));
    }
    // The work filter carries over both ways.
    assert_eq!(
        spa_url("work_threads#index", "/work", Some("state=agents")).as_deref(),
        Some("/app/work?state=agents")
    );
    assert_eq!(
        classic_url("/app/work", Some("state=done")).as_deref(),
        Some("/work?state=done")
    );
    // Creating the handoff stays a classic form post; only its page moves.
    assert_eq!(
        spa_url(
            "threads/work/handoffs#create",
            "/threads/9/work/handoff",
            None
        ),
        None
    );
}

#[test]
fn message_aliases_and_room_tools_map_to_their_ported_screens() {
    for (endpoint, classic, spa, fallback) in [
        (
            "messages#show",
            "/rooms/12/messages/345",
            "/app/r/12/m/345",
            "/rooms/12/@345",
        ),
        (
            "messages#edit",
            "/rooms/12/messages/345/edit",
            "/app/r/12/m/345",
            "/rooms/12/@345",
        ),
        (
            "messages#show",
            "/messages/345",
            "/app/m/345",
            "/messages/345",
        ),
        (
            "messages#edit",
            "/messages/345/edit",
            "/app/m/345",
            "/messages/345",
        ),
        (
            "messages/boosts#index",
            "/messages/345/boosts",
            "/app/m/345",
            "/messages/345",
        ),
        (
            "messages/boosts#new",
            "/messages/345/boosts/new",
            "/app/m/345",
            "/messages/345",
        ),
        (
            "channel_threads#index",
            "/rooms/12/threads",
            "/app/r/12/threads",
            "/rooms/12/threads",
        ),
        (
            "rooms/files#index",
            "/rooms/12/files",
            "/app/r/12/files",
            "/rooms/12/files",
        ),
        (
            "rooms/pins#index",
            "/rooms/12/pins",
            "/app/r/12/pins",
            "/rooms/12/pins",
        ),
        (
            "rooms/involvements#show",
            "/rooms/12/involvement",
            "/app/r/12/notifications",
            "/rooms/12/involvement",
        ),
    ] {
        assert_eq!(
            spa_url(endpoint, classic, None).as_deref(),
            Some(spa),
            "{classic}"
        );
        assert_eq!(
            classic_url(spa, None).as_deref(),
            Some(fallback),
            "{classic}"
        );
        assert_eq!(
            spa_url(endpoint, classic, Some("a=1&classic=0")).as_deref(),
            Some(format!("{spa}?a=1").as_str()),
            "{classic}"
        );
        assert_eq!(
            classic_url(spa, Some("a=1&classic=1")).as_deref(),
            Some(format!("{fallback}?a=1").as_str()),
            "{classic}"
        );
    }
}

#[test]
fn room_new_and_edit_pages_open_the_create_dialog_and_the_settings() {
    for (endpoint, classic, spa) in [
        ("rooms/opens#new", "/rooms/opens/new", "/app/rooms/new/open"),
        (
            "rooms/closeds#new",
            "/rooms/closeds/new",
            "/app/rooms/new/closed",
        ),
        (
            "rooms/voices#new",
            "/rooms/voices/new",
            "/app/rooms/new/voice",
        ),
        (
            "rooms/stages#new",
            "/rooms/stages/new",
            "/app/rooms/new/stage",
        ),
        (
            "rooms/boards#new",
            "/rooms/boards/new",
            "/app/rooms/new/board",
        ),
    ] {
        assert_eq!(spa_url(endpoint, classic, None).as_deref(), Some(spa));
        assert_eq!(classic_url(spa, None).as_deref(), Some(classic));
    }
    for (endpoint, classic) in [
        ("rooms/opens#edit", "/rooms/opens/12/edit"),
        ("rooms/closeds#edit", "/rooms/closeds/12/edit"),
        ("rooms/voices#edit", "/rooms/voices/12/edit"),
        ("rooms/stages#edit", "/rooms/stages/12/edit"),
        ("rooms/boards#edit", "/rooms/boards/12/edit"),
    ] {
        assert_eq!(
            spa_url(endpoint, classic, None).as_deref(),
            Some("/app/r/12/settings")
        );
    }
    // The room page still opens the room; its settings fall back to the room page, never to
    // one kind's edit form.
    assert_eq!(
        spa_url("rooms#show", "/rooms/12", None).as_deref(),
        Some("/app/r/12")
    );
    assert_eq!(
        classic_url("/app/r/12/settings", None).as_deref(),
        Some("/rooms/12")
    );
    assert_eq!(
        spa_url("rooms/directs#edit", "/rooms/directs/12/edit", None),
        None
    );
}

#[test]
fn only_record_ids_match_a_parameter() {
    // `GET /rooms/opens` is rooms#show with id "opens" in Rails; the SPA can't open it.
    for path in [
        "/rooms/opens",
        "/rooms/0",
        "/rooms/-1",
        "/rooms/1e3",
        "/rooms/12.json",
        "/rooms/",
        "/rooms/9007199254740992",
    ] {
        assert_eq!(spa_url("rooms#show", path, None), None, "{path}");
    }
    assert_eq!(
        spa_url("rooms#show", "/rooms/9007199254740991", None).as_deref(),
        Some("/app/r/9007199254740991")
    );
    assert_eq!(
        spa_url("rooms#show", "/rooms/12/345", None),
        None,
        "the @ is part of the segment"
    );
}

#[test]
fn profile_aliases_map_to_people_and_own_sections_match_as_me() {
    assert_eq!(
        spa_url("users/profiles#show", "/users/me/profile", None).as_deref(),
        Some("/app/settings")
    );
    assert_eq!(
        spa_url(
            "users/sessions#index",
            "/users/me/sessions",
            Some("classic=0")
        )
        .as_deref(),
        Some("/app/settings/sessions")
    );
    assert_eq!(
        spa_url(
            "users/push_subscriptions#index",
            "/users/me/push_subscriptions",
            None
        )
        .as_deref(),
        Some("/app/settings/devices")
    );
    assert_eq!(
        spa_url("users/statuses#edit", "/users/me/status/edit", None).as_deref(),
        Some("/app/settings/status")
    );
    assert_eq!(
        spa_url("users/profiles#show", "/users/7/profile", None).as_deref(),
        Some("/app/people/7")
    );
    assert_eq!(
        classic_url("/app/settings", None).as_deref(),
        Some("/users/me/profile")
    );
    assert_eq!(
        classic_url("/app/settings/devices", None).as_deref(),
        Some("/users/me/push_subscriptions")
    );
    // Sections of the profile page map back to it.
    assert_eq!(
        classic_url("/app/settings/appearance", None).as_deref(),
        Some("/users/me/profile")
    );
}

#[test]
fn numeric_profile_aliases_resolve_for_the_viewer() {
    for (endpoint, path) in [
        ("users#show", "/users/7"),
        ("users/profiles#show", "/users/7/profile"),
        ("users/profiles#edit", "/users/7/profile/edit"),
    ] {
        assert_eq!(
            profile_url(endpoint, path, Some("source=profile&classic=0"), 7).as_deref(),
            Some("/app/settings?source=profile"),
            "{path}"
        );
        assert_eq!(
            profile_url(endpoint, path, None, 8).as_deref(),
            Some("/app/people/7"),
            "{path}"
        );
    }
    assert_eq!(
        profile_url("users/profiles#edit", "/users/me/profile/edit", None, 7).as_deref(),
        Some("/app/settings")
    );
}

#[test]
fn the_endpoint_must_match_too() {
    // `/rooms/new` precedes `/rooms/:id` in the table, so it is rooms#new.
    assert_eq!(spa_url("rooms#new", "/rooms/12", None), None);
    assert_eq!(spa_url("rooms#show", "/rooms/12/threads/9", None), None);
}

#[test]
fn unported_screens_never_redirect() {
    for screen in SCREENS.iter().filter(|screen| !screen.ported) {
        let path = sample(screen.classic, &[3, 4, 5]);
        assert_eq!(spa_url(screen.endpoint, &path, None), None, "{screen:?}");
    }
}

#[test]
fn room_notification_queries_open_the_thread_or_message() {
    assert_eq!(
        spa_url("rooms#show", "/rooms/12", Some("thread=9&message_id=4")).as_deref(),
        Some("/app/r/12/t/9?m=4")
    );
    assert_eq!(
        spa_url("rooms#show", "/rooms/12", Some("thread=9")).as_deref(),
        Some("/app/r/12/t/9")
    );
    assert_eq!(
        spa_url("rooms#show", "/rooms/12", Some("message_id=4")).as_deref(),
        Some("/app/r/12/m/4")
    );
    assert_eq!(
        spa_url(
            "rooms#show",
            "/rooms/12",
            Some("x=1&thread=9&classic=1&y=2")
        )
        .as_deref(),
        Some("/app/r/12/t/9?x=1&y=2")
    );
    assert_eq!(
        spa_url("rooms#show", "/rooms/12", Some("thread=nope&x=1")).as_deref(),
        Some("/app/r/12?thread=nope&x=1")
    );
}

#[test]
fn room_notification_queries_decode_and_use_the_classic_duplicate() {
    // `%39` is `9`. `thread` is the first value (the thread panel's `URLSearchParams.get`).
    // A non-empty `thread` makes `message_id` the first value too. With no thread, `message_id`
    // is the last value (the room controller's params). An effective value that isn't an id is
    // not replaced by another duplicate.
    assert_eq!(
        spa_url("rooms#show", "/rooms/12", Some("thread=%39")).as_deref(),
        Some("/app/r/12/t/9")
    );
    assert_eq!(
        spa_url("rooms#show", "/rooms/12", Some("message_id=%34")).as_deref(),
        Some("/app/r/12/m/4")
    );
    assert_eq!(
        spa_url("rooms#show", "/rooms/12", Some("th%72ead=%39")).as_deref(),
        Some("/app/r/12/t/9")
    );
    assert_eq!(
        spa_url("rooms#show", "/rooms/12", Some("thread=9&thread=8")).as_deref(),
        Some("/app/r/12/t/9")
    );
    assert_eq!(
        spa_url("rooms#show", "/rooms/12", Some("thread=%39&thread=8")).as_deref(),
        Some("/app/r/12/t/9")
    );
    assert_eq!(
        spa_url("rooms#show", "/rooms/12", Some("thread=nope&thread=9")).as_deref(),
        Some("/app/r/12?thread=nope&thread=9")
    );
    assert_eq!(
        spa_url("rooms#show", "/rooms/12", Some("message_id=4&message_id=5")).as_deref(),
        Some("/app/r/12/m/5")
    );
    assert_eq!(
        spa_url(
            "rooms#show",
            "/rooms/12",
            Some("message_id=4&message_id=nope")
        )
        .as_deref(),
        Some("/app/r/12?message_id=4&message_id=nope")
    );
    assert_eq!(
        spa_url(
            "rooms#show",
            "/rooms/12",
            Some("thread=9&thread=8&message_id=4&message_id=5")
        )
        .as_deref(),
        Some("/app/r/12/t/9?m=4")
    );
    assert_eq!(
        spa_url(
            "rooms#show",
            "/rooms/12",
            Some("thread=9&message_id=nope&message_id=5")
        )
        .as_deref(),
        Some("/app/r/12/t/9")
    );
    // `%ZZ` is not an id. HTTP rejects that escape with 400 before routing; this only pins
    // the translator when such a query is handed to it.
    assert_eq!(
        spa_url("rooms#show", "/rooms/12", Some("thread=%ZZ&x=1")).as_deref(),
        Some("/app/r/12?thread=%ZZ&x=1")
    );
    let refused = ConfirmedRoomQuery {
        thread: None,
        message: None,
    };
    assert_eq!(
        spa_url_confirmed("rooms#show", "/rooms/12", Some("thread=9&x=1"), refused).as_deref(),
        Some("/app/r/12?thread=9&x=1")
    );
    // Confirmation applies to the effective value only. Naming the other duplicate does not
    // make it the one that opens.
    let other_thread = ConfirmedRoomQuery {
        thread: Some(8),
        message: None,
    };
    assert_eq!(
        spa_url_confirmed(
            "rooms#show",
            "/rooms/12",
            Some("thread=9&thread=8"),
            other_thread
        )
        .as_deref(),
        Some("/app/r/12?thread=9&thread=8")
    );
    let other_message = ConfirmedRoomQuery {
        thread: None,
        message: Some(4),
    };
    assert_eq!(
        spa_url_confirmed(
            "rooms#show",
            "/rooms/12",
            Some("message_id=4&message_id=5"),
            other_message
        )
        .as_deref(),
        Some("/app/r/12?message_id=4&message_id=5")
    );
    assert_eq!(
        room_query_ids(Some("thread=%39&thread=8&message_id=4&message_id=5")),
        RoomQueryIds {
            thread: Some(9),
            message: Some(4),
        }
    );
    assert_eq!(
        room_query_ids(Some("message_id=4&message_id=5")),
        RoomQueryIds {
            thread: None,
            message: Some(5),
        }
    );
    // `thread=nope` is still present, so the first `message_id` is the one that counts.
    assert_eq!(
        room_query_ids(Some("thread=nope&thread=9&message_id=4&message_id=nope")),
        RoomQueryIds {
            thread: None,
            message: Some(4),
        }
    );
    assert_eq!(
        room_query_ids(Some("thread=9&message_id=nope&message_id=4")),
        RoomQueryIds {
            thread: Some(9),
            message: None,
        }
    );
}

#[test]
fn the_query_carries_over_without_classic() {
    assert_eq!(
        spa_url("rooms#show", "/rooms/12", Some("a=1&classic=0&b=2")).as_deref(),
        Some("/app/r/12?a=1&b=2")
    );
    assert_eq!(
        spa_url("rooms#show", "/rooms/12", Some("classic")).as_deref(),
        Some("/app/r/12")
    );
    assert_eq!(
        spa_url("rooms#show", "/rooms/12", Some("")).as_deref(),
        Some("/app/r/12")
    );
    assert_eq!(
        classic_url("/app/search", Some("q=fire&classic=1")).as_deref(),
        Some("/searches?q=fire")
    );
}

#[test]
fn every_spa_url_maps_back_to_its_first_classic_page() {
    for screen in SCREENS {
        let ids = [7, 8, 9];
        let (classic, spa) = (sample(screen.classic, &ids), sample(screen.spa, &ids));
        assert_eq!(
            classic_url(&spa, None).as_deref(),
            Some(sample(first_for_spa(screen).classic, &ids).as_str()),
            "{screen:?}"
        );
        if screen.ported {
            assert_eq!(
                spa_url(screen.endpoint, &classic, None).as_deref(),
                Some(sample(first_for(screen).spa, &ids).as_str()),
                "{screen:?}"
            );
        }
    }
    assert_eq!(classic_url("/app", None).as_deref(), Some("/"));
    assert_eq!(
        classic_url("/app/r/12/t/9", Some("m=4")).as_deref(),
        Some("/rooms/12/threads/9?m=4")
    );
    assert_eq!(classic_url("/app/nowhere", None), None);
    assert_eq!(classic_url("/app/r/general", None), None);
}

/// `frontend/src/gen/screens.json` is [`json`]'s output. `UPDATE_SCREENS=1` (or `pnpm gen` in
/// `frontend/`) rewrites it.
#[test]
fn export_screens() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../frontend/src/gen/screens.json");
    if std::env::var_os("UPDATE_SCREENS").is_some() {
        std::fs::write(&path, json()).unwrap();
    }
    let committed = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        committed == json(),
        "frontend/src/gen/screens.json is out of date with crates/spa/src/screens.rs; run `pnpm gen` in frontend/ (or UPDATE_SCREENS=1 cargo test -p campfire_spa export_screens)"
    );
    let parsed: serde_json::Value = serde_json::from_str(&committed).unwrap();
    assert_eq!(parsed.as_array().map(Vec::len), Some(SCREENS.len()));
}

#[test]
fn event_pages_open_in_the_spa_and_fall_back_to_classic() {
    for (endpoint, classic, spa) in [
        ("rooms/events#index", "/rooms/12/events", "/app/r/12/events"),
        (
            "rooms/events#new",
            "/rooms/12/events/new",
            "/app/r/12/events/new",
        ),
        (
            "rooms/events#show",
            "/rooms/12/events/34",
            "/app/r/12/events/34",
        ),
        (
            "rooms/events#edit",
            "/rooms/12/events/34/edit",
            "/app/r/12/events/34/edit",
        ),
        (
            "rooms/events/attendances#show",
            "/rooms/12/events/34/attendance",
            "/app/r/12/events/34/attendance",
        ),
    ] {
        assert_eq!(
            spa_url(endpoint, classic, None).as_deref(),
            Some(spa),
            "{classic}"
        );
        assert_eq!(classic_url(spa, None).as_deref(), Some(classic), "{spa}");
    }

    // `new` is not an event id: the show row doesn't take it.
    assert_eq!(
        spa_url("rooms/events#show", "/rooms/12/events/new", None),
        None
    );
}

#[test]
fn fizzy_card_forms_open_over_the_conversation_and_fall_back_to_classic() {
    for (classic, spa) in [
        (
            "/rooms/12/messages/34/fizzy_cards/new",
            "/app/r/12/m/34/fizzy/new",
        ),
        (
            "/rooms/12/threads/5/messages/34/fizzy_cards/new",
            "/app/r/12/t/5/m/34/fizzy/new",
        ),
    ] {
        assert_eq!(
            spa_url("rooms/fizzy/message_cards#new", classic, None).as_deref(),
            Some(spa),
            "{classic}"
        );
        assert_eq!(classic_url(spa, None).as_deref(), Some(classic), "{spa}");
    }

    // The room's permalink and thread rows don't take the form's longer URLs.
    assert_eq!(
        classic_url("/app/r/12/m/34", None).as_deref(),
        Some("/rooms/12/@34")
    );
    assert_eq!(
        spa_url(
            "rooms/fizzy/message_cards#new",
            "/rooms/12/messages/new/fizzy_cards/new",
            None
        ),
        None
    );
}
