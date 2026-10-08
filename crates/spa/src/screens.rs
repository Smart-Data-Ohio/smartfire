//! The screen map: which classic pages have an SPA equivalent, and where each one lives under
//! `/app/`. It drives both directions of the coexistence (plan §5.0):
//!
//! - A person who uses the new UI and opens a **ported** classic page (`GET`, an HTML
//!   navigation) is sent to its SPA URL ([`spa_url`]); `?classic=1` keeps them on the classic page.
//! - The SPA reads the same table (`frontend/src/gen/screens.json`, written from [`SCREENS`] by
//!   [`json`]) to open classic links it has ported in place, and to send a destination it hasn't
//!   ported yet to its classic page with a full page load ([`classic_url`] is the same mapping).
//!
//! One classic page can hold several SPA screens (the profile page's sections): each has a row, all
//! map back to it, and its redirect goes to the first. Several classic links can also share an SPA
//! screen (a message's edit and boost links): its classic fallback is the first matching row.
//! A slice that ports a screen adds its SPA
//! route and flips `ported` here in the same PR. Unported
//! rows name where a screen will live, so the SPA can link there already: the server never
//! redirects to them, and the SPA forwards them to the classic page.
//!
//! Patterns use the route table's syntax without `(.:format)`: a segment is a literal, or a
//! literal prefix and a `:param` (`@:message_id`). Every parameter is a record id, so it matches
//! only a positive integer: `/rooms/new` and `/rooms/5.json` aren't `/rooms/:id`. The one
//! exception is the route table's `:user_id` on a person's own pages, which the classic app links
//! as `me` (`/users/me/profile`): a row spells it `me`, a literal, so other people's ids never
//! match it.

/// One classic page and its SPA URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Screen {
    /// The classic route's `controller#action` (the route table's endpoint).
    pub endpoint: &'static str,
    /// The classic route's path pattern, as the route table spells it, without `(.:format)`.
    pub classic: &'static str,
    /// The SPA path pattern, with the same parameters.
    pub spa: &'static str,
    /// Whether the SPA has built it.
    pub ported: bool,
}

const fn screen(
    endpoint: &'static str,
    classic: &'static str,
    spa: &'static str,
    ported: bool,
) -> Screen {
    Screen {
        endpoint,
        classic,
        spa,
        ported,
    }
}

/// The map, in the order the SPA tries its rows.
pub const SCREENS: &[Screen] = &[
    // S1/S2: the shell, rooms, permalinks and threads.
    screen("welcome#show", "/", "/app/", true),
    screen("rooms#show", "/rooms/:id", "/app/r/:id", true),
    screen(
        "rooms#show",
        "/rooms/:room_id/@:message_id",
        "/app/r/:room_id/m/:message_id",
        true,
    ),
    screen(
        "channel_threads#show",
        "/rooms/:room_id/threads/:id",
        "/app/r/:room_id/t/:id",
        true,
    ),
    // S8: message aliases share the permalink; the room's permalink above wins on opt-out.
    screen(
        "messages#show",
        "/rooms/:room_id/messages/:id",
        "/app/r/:room_id/m/:id",
        true,
    ),
    screen(
        "messages#edit",
        "/rooms/:room_id/messages/:id/edit",
        "/app/r/:room_id/m/:id",
        true,
    ),
    // Bare links use the message resolver; edit and boost links fall back to its permalink.
    screen("messages#show", "/messages/:id", "/app/m/:id", true),
    screen("messages#edit", "/messages/:id/edit", "/app/m/:id", true),
    screen(
        "messages/boosts#index",
        "/messages/:message_id/boosts",
        "/app/m/:message_id",
        true,
    ),
    screen(
        "messages/boosts#new",
        "/messages/:message_id/boosts/new",
        "/app/m/:message_id",
        true,
    ),
    screen(
        "channel_threads#index",
        "/rooms/:room_id/threads",
        "/app/r/:room_id/threads",
        true,
    ),
    screen(
        "rooms/files#index",
        "/rooms/:room_id/files",
        "/app/r/:room_id/files",
        true,
    ),
    screen(
        "rooms/pins#index",
        "/rooms/:room_id/pins",
        "/app/r/:room_id/pins",
        true,
    ),
    screen(
        "rooms/involvements#show",
        "/rooms/:room_id/involvement",
        "/app/r/:room_id/notifications",
        true,
    ),
    // S8: making rooms. Each kind's new page opens the create dialog on that kind (the installed
    // app's "New chat room" shortcut is the open one).
    screen(
        "rooms/opens#new",
        "/rooms/opens/new",
        "/app/rooms/new/open",
        true,
    ),
    screen(
        "rooms/closeds#new",
        "/rooms/closeds/new",
        "/app/rooms/new/closed",
        true,
    ),
    screen(
        "rooms/voices#new",
        "/rooms/voices/new",
        "/app/rooms/new/voice",
        true,
    ),
    screen(
        "rooms/stages#new",
        "/rooms/stages/new",
        "/app/rooms/new/stage",
        true,
    ),
    screen(
        "rooms/boards#new",
        "/rooms/boards/new",
        "/app/rooms/new/board",
        true,
    ),
    // S8: a room's settings. The classic edit pages are one per kind, and a kind's form saved on
    // another kind's room would convert it, so the shared settings screen falls back to the room
    // itself. Boards keep their classic edit page until boards are ported.
    screen("rooms#show", "/rooms/:id", "/app/r/:id/settings", true),
    screen(
        "rooms/opens#edit",
        "/rooms/opens/:id/edit",
        "/app/r/:id/settings",
        true,
    ),
    screen(
        "rooms/closeds#edit",
        "/rooms/closeds/:id/edit",
        "/app/r/:id/settings",
        true,
    ),
    screen(
        "rooms/voices#edit",
        "/rooms/voices/:id/edit",
        "/app/r/:id/settings",
        true,
    ),
    screen(
        "rooms/stages#edit",
        "/rooms/stages/:id/edit",
        "/app/r/:id/settings",
        true,
    ),
    // S8: "Create Fizzy card" on a message, a dialog over its room or its thread.
    screen(
        "rooms/fizzy/message_cards#new",
        "/rooms/:room_id/messages/:message_id/fizzy_cards/new",
        "/app/r/:room_id/m/:message_id/fizzy/new",
        true,
    ),
    screen(
        "rooms/fizzy/message_cards#new",
        "/rooms/:room_id/threads/:thread_id/messages/:message_id/fizzy_cards/new",
        "/app/r/:room_id/t/:thread_id/m/:message_id/fizzy/new",
        true,
    ),
    // S3: workspace destinations (plan §4.7).
    screen("activity_items#index", "/activity", "/app/activity", true),
    screen("saved_items#index", "/saved", "/app/saved", true),
    screen(
        "scheduled_messages#index",
        "/scheduled_messages",
        "/app/scheduled",
        true,
    ),
    screen("searches#index", "/searches", "/app/search", true),
    screen("work_threads#index", "/work", "/app/work", false),
    // S7: the signed-in person's own settings (`/users/me/...`).
    screen(
        "users/profiles#show",
        "/users/me/profile",
        "/app/settings",
        true,
    ),
    // Sections of the classic profile page: they map back to it ("Switch to classic"), and its
    // redirect goes to the row above.
    screen(
        "users/profiles#show",
        "/users/me/profile",
        "/app/settings/notifications",
        true,
    ),
    screen(
        "users/profiles#show",
        "/users/me/profile",
        "/app/settings/appearance",
        true,
    ),
    screen(
        "users/profiles#show",
        "/users/me/profile",
        "/app/settings/calls",
        true,
    ),
    screen(
        "users/profiles#show",
        "/users/me/profile",
        "/app/settings/integrations",
        true,
    ),
    screen(
        "users/profiles#show",
        "/users/me/profile",
        "/app/settings/rooms",
        true,
    ),
    screen(
        "users/profiles#show",
        "/users/me/profile",
        "/app/settings/security",
        true,
    ),
    screen(
        "users/statuses#edit",
        "/users/me/status/edit",
        "/app/settings/status",
        true,
    ),
    screen(
        "users/sessions#index",
        "/users/me/sessions",
        "/app/settings/sessions",
        true,
    ),
    screen(
        "users/push_subscriptions#index",
        "/users/me/push_subscriptions",
        "/app/settings/devices",
        true,
    ),
    // S7: the people directory and a person's page.
    screen("users#index", "/users", "/app/people", true),
    screen("users#show", "/users/:id", "/app/people/:id", true),
    // S7: the workspace's account pages. The people list is the account page's lower half: it
    // maps back to the page, whose redirect goes to the workspace row above it.
    screen("accounts#edit", "/account/edit", "/app/admin", true),
    screen("accounts#edit", "/account/edit", "/app/admin/people", true),
    screen("accounts/icons#index", "/account/icons", "/app/admin/icons", true),
    screen(
        "accounts/custom_styles#edit",
        "/account/custom_styles/edit",
        "/app/admin/styles",
        true,
    ),
    screen(
        "accounts/audit_logs#show",
        "/account/audit_log",
        "/app/admin/audit-log",
        true,
    ),
    screen(
        "accounts/integrations_health#show",
        "/account/integrations_health",
        "/app/admin/integrations",
        true,
    ),
    // S7: the chat bot pages. Their writes stay classic; only the pages move.
    screen("accounts/bots#index", "/account/bots", "/app/admin/bots", true),
    screen("accounts/bots#new", "/account/bots/new", "/app/admin/bots/new", true),
    screen(
        "accounts/bots#edit",
        "/account/bots/:id/edit",
        "/app/admin/bots/:id",
        true,
    ),
    screen(
        "accounts/bots/credentials#index",
        "/account/bots/:bot_id/credentials",
        "/app/admin/bots/:bot_id/credentials",
        true,
    ),
    screen(
        "accounts/bots/grants#index",
        "/account/bots/:bot_id/grants",
        "/app/admin/bots/:bot_id/grants",
        true,
    ),
    // S7: the Slack importer, the administrator's pages and everyone's own.
    screen(
        "accounts/slack_imports#show",
        "/account/slack_import",
        "/app/admin/slack",
        true,
    ),
    screen(
        "accounts/slack_import_runs#index",
        "/account/slack_import/runs",
        "/app/admin/slack/runs",
        true,
    ),
    screen(
        "accounts/slack_import_runs#show",
        "/account/slack_import/runs/:id",
        "/app/admin/slack/runs/:id",
        true,
    ),
    screen(
        "accounts/slack_import_runs#plan",
        "/account/slack_import/runs/:id/plan",
        "/app/admin/slack/runs/:id/plan",
        true,
    ),
    screen("slack/imports#index", "/slack/imports", "/app/settings/slack", true),
    screen(
        "slack/imports#show",
        "/slack/imports/:id",
        "/app/settings/slack/:id",
        true,
    ),
];

/// The SPA URL for a classic `GET` of `endpoint` at `path`, when the SPA has ported that screen.
/// `query` (without its `?`) carries over, less `classic`.
pub fn spa_url(endpoint: &str, path: &str, query: Option<&str>) -> Option<String> {
    SCREENS
        .iter()
        .filter(|screen| screen.ported && screen.endpoint == endpoint)
        .find_map(|screen| {
            Some(with_query(
                fill(screen.spa, &captures(screen.classic, path)?),
                query,
            ))
        })
}

/// The classic URL for an SPA `path` (under `/app/`), ported or not: where "Switch to classic"
/// lands, and where the SPA sends a destination it hasn't built. `query` carries over, less
/// `classic`.
pub fn classic_url(path: &str, query: Option<&str>) -> Option<String> {
    SCREENS.iter().find_map(|screen| {
        Some(with_query(
            fill(screen.classic, &captures(screen.spa, path)?),
            query,
        ))
    })
}

/// `SCREENS` as the JSON the SPA reads (`frontend/src/gen/screens.json`).
pub fn json() -> String {
    let rows: Vec<String> = SCREENS
        .iter()
        .map(|screen| {
            let string = |value: &str| serde_json::to_string(value).expect("a string serializes");
            format!(
                "  {{ \"endpoint\": {}, \"classic\": {}, \"spa\": {}, \"ported\": {} }}",
                string(screen.endpoint),
                string(screen.classic),
                string(screen.spa),
                screen.ported
            )
        })
        .collect();
    format!("[\n{}\n]\n", rows.join(",\n"))
}

/// A pattern's parameters, in order.
pub fn params(pattern: &str) -> Vec<&str> {
    segments(pattern)
        .filter_map(|segment| segment.split_once(':').map(|(_, name)| name))
        .collect()
}

/// The non-empty segments of a path (repeated and trailing slashes don't count, as Journey's
/// `normalize_path` has it).
fn segments(path: &str) -> impl Iterator<Item = &str> {
    path.split('/').filter(|segment| !segment.is_empty())
}

/// `path`'s parameters by name when it matches `pattern`.
fn captures<'p>(pattern: &'p str, path: &str) -> Option<Vec<(&'p str, u64)>> {
    let mut pattern_segments = segments(pattern);
    let mut path_segments = segments(path);
    let mut captured = Vec::new();
    loop {
        match (pattern_segments.next(), path_segments.next()) {
            (None, None) => return Some(captured),
            (Some(expected), Some(actual)) => match expected.split_once(':') {
                None if expected == actual => {}
                None => return None,
                Some((prefix, name)) => captured.push((name, id(actual.strip_prefix(prefix)?)?)),
            },
            _ => return None,
        }
    }
}

/// A record id: a positive integer below 2^53 (the SPA's ids are JavaScript numbers).
fn id(segment: &str) -> Option<u64> {
    if segment.is_empty() || !segment.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    segment.parse().ok().filter(|id| (1..1 << 53).contains(id))
}

/// `pattern` with each `:param` replaced by its captured value.
fn fill(pattern: &str, captured: &[(&str, u64)]) -> String {
    let mut out = String::new();
    for segment in segments(pattern) {
        out.push('/');
        match segment.split_once(':') {
            None => out.push_str(segment),
            Some((prefix, name)) => {
                let value = captured
                    .iter()
                    .find(|(captured, _)| *captured == name)
                    .map(|(_, value)| *value);
                out.push_str(prefix);
                out.push_str(
                    &value
                        .expect("a pattern's parameters are its pair's")
                        .to_string(),
                );
            }
        }
    }
    if out.is_empty() || pattern.ends_with('/') {
        out.push('/');
    }
    out
}

/// `url` with `query`'s pairs, less any `classic`.
fn with_query(mut url: String, query: Option<&str>) -> String {
    let kept: Vec<&str> = query
        .unwrap_or("")
        .split('&')
        .filter(|pair| !pair.is_empty() && pair.split('=').next() != Some("classic"))
        .collect();
    if !kept.is_empty() {
        url.push('?');
        url.push_str(&kept.join("&"));
    }
    url
}

/// Whether a request's query asks to stay on the classic page: a `classic` parameter, any value
/// but `0` or empty.
pub fn bypassed(query: Option<&str>) -> bool {
    query.unwrap_or("").split('&').any(|pair| {
        let (name, value) = pair.split_once('=').unwrap_or((pair, "1"));
        name == "classic" && !matches!(value, "" | "0")
    })
}

#[cfg(test)]
#[path = "screens_tests.rs"]
mod tests;
