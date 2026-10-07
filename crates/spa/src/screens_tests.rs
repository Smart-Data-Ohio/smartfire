//! The screen map: its rows against the route table, both directions of the mapping, and the
//! copy the SPA reads.

use std::collections::BTreeSet;
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
                    && route.spec == spec),
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

#[test]
fn no_two_rows_claim_the_same_url() {
    let classic: BTreeSet<_> = SCREENS
        .iter()
        .map(|screen| (screen.endpoint, screen.classic))
        .collect();
    let spa: BTreeSet<_> = SCREENS.iter().map(|screen| screen.spa).collect();
    assert_eq!(classic.len(), SCREENS.len());
    assert_eq!(spa.len(), SCREENS.len());
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
fn every_spa_url_maps_back_to_its_classic_page() {
    for screen in SCREENS {
        let ids = [7, 8, 9];
        let (classic, spa) = (sample(screen.classic, &ids), sample(screen.spa, &ids));
        assert_eq!(
            classic_url(&spa, None).as_deref(),
            Some(classic.as_str()),
            "{screen:?}"
        );
        if screen.ported {
            assert_eq!(
                spa_url(screen.endpoint, &classic, None).as_deref(),
                Some(spa.as_str()),
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

#[test]
fn classic_1_bypasses_the_redirect() {
    for query in ["classic=1", "a=b&classic=1", "classic", "classic=true"] {
        assert!(bypassed(Some(query)), "{query}");
    }
    for query in [
        None,
        Some(""),
        Some("classic=0"),
        Some("classic="),
        Some("classical=1"),
        Some("x=classic"),
    ] {
        assert!(!bypassed(query), "{query:?}");
    }
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
