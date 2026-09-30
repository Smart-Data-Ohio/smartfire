//! The generated table and helpers against what our Rails dumped (`routes.json`), including the
//! awkward-argument, format and query samples.

use super::*;

#[derive(serde::Deserialize)]
struct Routes {
    named_routes: Vec<Named>,
    directs: std::collections::BTreeMap<String, Vec<String>>,
}

#[derive(serde::Deserialize)]
struct Named {
    name: String,
    samples: Vec<Sample>,
}

#[derive(serde::Deserialize)]
struct Sample {
    args: Vec<String>,
    options: serde_json::Map<String, serde_json::Value>,
    path: String,
    default_override: Option<String>,
}

#[test]
fn every_route_default_can_be_overridden() {
    let mut checked = 0;
    let mut failed = Vec::new();
    for route in routes().named_routes {
        for sample in route
            .samples
            .iter()
            .filter(|s| s.default_override.is_some())
        {
            let args: Vec<&dyn Display> =
                sample.args.iter().map(|arg| arg as &dyn Display).collect();
            let format = sample.options.get("format").and_then(|f| f.as_str());
            let query: Vec<(&str, Option<&str>)> = sample
                .options
                .iter()
                .filter(|(key, _)| *key != "format")
                .map(|(key, value)| (key.as_str(), value.as_str()))
                .collect();
            let actual = named(&route.name).path_with(&args, format, &query);
            if actual != sample.path {
                failed.push(format!("{}: {actual} != {}", route.name, sample.path));
            }
            checked += 1;
        }
    }
    assert!(checked > 0, "Rails default override vectors must exist");
    assert!(
        failed.is_empty(),
        "{} default overrides differed: {failed:#?}",
        failed.len()
    );
}

fn routes() -> Routes {
    serde_json::from_str(include_str!("../routes.json")).unwrap()
}

fn named(name: &str) -> &'static NamedRoute {
    NAMED_ROUTES
        .iter()
        .find(|route| route.name == name)
        .unwrap_or_else(|| panic!("no {name} route"))
}

#[test]
fn every_named_route_builds_the_paths_rails_does() {
    let routes = routes();
    assert_eq!(routes.named_routes.len(), NAMED_ROUTES.len());
    for route in &routes.named_routes {
        let ours = named(&route.name);
        for sample in &route.samples {
            let args: Vec<&dyn Display> =
                sample.args.iter().map(|arg| arg as &dyn Display).collect();
            let format = sample.options.get("format").and_then(|f| f.as_str());
            let query: Vec<(&str, Option<&str>)> = sample
                .options
                .iter()
                .filter(|(key, _)| *key != "format")
                .map(|(key, value)| (key.as_str(), value.as_str()))
                .collect();
            assert_eq!(
                ours.path_with(&args, format, &query),
                sample.path,
                "{}_path{:?}",
                route.name,
                sample.args
            );
        }
    }
}

#[test]
fn helper_functions_are_the_named_routes() {
    assert_eq!(room(1), "/rooms/1");
    assert_eq!(room_at_message(1, 2), "/rooms/1/@2");
    assert_eq!(user_sidebar(), "/users/me/sidebar");
    assert_eq!(join("a b/c"), "/join/a%20b%2Fc");
    assert_eq!(
        rails_service_blob("x", "a/b c.png"),
        "/rails/active_storage/blobs/redirect/x/a/b%20c.png"
    );
}

#[test]
fn direct_routes_build_what_rails_does() {
    let directs = routes().directs;
    assert_eq!(
        vec![
            fresh_account_logo(Some("20260302160000"), None),
            fresh_account_logo(Some("20260302160000"), Some("small"))
        ],
        directs["fresh_account_logo"]
    );
    assert_eq!(
        vec![fresh_user_avatar("tok/en+x", "20260302160000")],
        directs["fresh_user_avatar"]
    );
    assert_eq!(
        vec![
            fresh_account_logo(None, None),
            fresh_account_logo(None, Some("large"))
        ],
        directs["fresh_account_logo_without_account"]
    );
}

#[test]
fn the_table_has_every_route() {
    assert_eq!(TABLE.len(), 469);
    assert_eq!(TABLE[0].endpoint, "welcome#show");
    assert!(TABLE.iter().any(
        |r| r.endpoint == "rooms/settings#show" && r.action == ActionStatus::MissingController
    ));
}
