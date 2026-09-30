//! The route table from our `config/routes.rb` (plus the framework routes `bin/rails routes`
//! lists: Active Storage, Turbo Native navigation, Action Mailbox, `/up`), and a router that
//! matches the way Rails' Journey does.
//!
//! Axum only sees one catch-all route; [`dispatch`] then picks the *first* route in table order
//! whose verb and pattern match, exactly as Rails does. That matters: `GET /rooms/opens` is
//! `rooms#show` with `id: "opens"` because `resources :rooms` is drawn before
//! `namespace :rooms`. Patterns use Rails syntax: `:param` matches `[^/.?]+`, `*glob` matches
//! lazily, and `(.:format)` is the optional format suffix (so `/rooms/1.json` sets
//! `params[:format]`). Path params are percent-decoded and merged over query and body params,
//! together with `controller` and `action`.
//!
//! The rows come from `campfire_routes::TABLE`, which is generated from our Rails
//! (`reference-tools/routes/routes.rb`), so they can't drift from `config/routes.rb`. Only the
//! action each row runs is chosen here:
//! - a declared route whose action our Rails doesn't define answers 404, like
//!   `AbstractController::ActionNotFound` ([`action_not_found`]);
//! - one whose controller doesn't exist answers 500 ([`missing_controller`]);
//! - a real action runs its port from [`ported`], or [`not_yet_ported`], a 501 that names the
//!   endpoint, so an unported page never passes for a Rails 404.
//!
//! ## Porting a controller
//!
//! Add `pub mod rooms;` (etc.) below, write `pub async fn show(c: &mut Ctx) -> Result` actions
//! that start with their before-action chain (see `crate::concerns`), and add the endpoint to
//! [`ported`]. The tests below check the table against `vectors/campfire_routes.json` (from
//! `reference-tools/campfire/routes.rb`) and every [`ported`] endpoint against the table.

use std::sync::{Arc, LazyLock};

use campfire_kit::{Ctx, Error, Method, Param, ParamMap, Result, StatusCode};
use futures_util::future::BoxFuture;
use campfire_routes::{ActionStatus, TableRoute};
use regex::Regex;

use crate::active_storage;

// Controller modules (one per Rails controller namespace), plus the presenters that map rows to
// view models. Controller agents add their `pub mod` lines here.
pub mod accounts;
pub mod autocompletable;
pub mod csp_reports;
pub mod first_runs;
pub mod messages;
pub mod presenters;
pub mod pwa;
pub mod qr_code;
pub mod rooms;
pub mod searches;
pub mod sessions;
pub mod unfurl_links;
pub mod users;
pub mod welcome;
pub mod internal_huddle;
#[cfg(test)]
mod internal_huddle_tests;
#[cfg(test)]
mod internal_huddle_declaration_tests;

/// Anything that can serve a route: every `async fn(&mut Ctx) -> Result` qualifies.
pub trait Action: Send + Sync + 'static {
    fn call<'a>(&'a self, c: &'a mut Ctx) -> BoxFuture<'a, Result>;
}

impl<F> Action for F
where
    F: for<'a> campfire_kit::ActionFn<'a>,
{
    fn call<'a>(&'a self, c: &'a mut Ctx) -> BoxFuture<'a, Result> {
        Box::pin(campfire_kit::ActionFn::call(self, c))
    }
}

pub struct Route {
    pub verb: Method,
    /// The Rails path spec, e.g. `/rooms/:room_id/messages(.:format)`. Compiled into `regex`; kept
    /// for the test that compares the table with `bin/rails routes`.
    #[cfg_attr(not(test), allow(dead_code))]
    pub pattern: &'static str,
    /// `controller#action`, as `bin/rails routes` prints it.
    pub endpoint: &'static str,
    pub defaults: &'static [(&'static str, &'static str)],
    action: Arc<dyn Action>,
    regex: Regex,
    names: Vec<String>,
}

impl Route {
    fn new(spec: &'static TableRoute, action: Arc<dyn Action>) -> Self {
        let (regex, names) = compile(spec.spec);
        let verb = Method::from_bytes(spec.verb.as_bytes()).expect("a route verb");
        Self { verb, pattern: spec.spec, endpoint: spec.endpoint, defaults: spec.defaults, action, regex, names }
    }

    pub fn controller(&self) -> &'static str {
        self.endpoint.split_once('#').map_or(self.endpoint, |(controller, _)| controller)
    }

    pub fn action_name(&self) -> &'static str {
        self.endpoint.split_once('#').map_or("", |(_, action)| action)
    }
}

/// The route that matched the current request (`request.path_parameters` plus the endpoint),
/// available to actions as `c.current::<MatchedRoute>()`.
#[derive(Debug, Clone)]
pub struct MatchedRoute {
    pub endpoint: &'static str,
}

/// Every route, in `config/routes.rb` order.
pub fn routes() -> &'static [Route] {
    &ROUTES
}

static ROUTES: LazyLock<Vec<Route>> = LazyLock::new(|| {
    campfire_routes::TABLE
        .iter()
        .map(|spec| {
            let action = match spec.action {
                ActionStatus::ActionNotFound => arc(action_not_found),
                ActionStatus::MissingController => arc(missing_controller),
                ActionStatus::Defined | ActionStatus::Implicit => {
                    ported(spec.endpoint).unwrap_or_else(|| arc(not_yet_ported))
                }
            };
            Route::new(spec, action)
        })
        .collect()
});

fn arc(action: impl Action) -> Arc<dyn Action> {
    Arc::new(action)
}

/// The Rust action for each endpoint that has been ported. An endpoint serves every route that
/// names it (`rooms#show` is both `/rooms/:id` and `/rooms/:room_id/@:message_id`).
///
/// Many of these are still upstream Campfire's actions, which the domain workstreams replace
/// with ports of ours.
fn ported(endpoint: &str) -> Option<Arc<dyn Action>> {
    Some(match endpoint {
        "rooms/voices#show" => arc(rooms::call_channels::show),
        "rooms/voices#new" => arc(rooms::call_channels::new),
        "rooms/voices#create" => arc(rooms::call_channels::create),
        "rooms/voices#edit" => arc(rooms::call_channels::edit),
        "rooms/voices#update" => arc(rooms::call_channels::update),
        "rooms/voices#index" => arc(rooms::index),
        "rooms/voices#destroy" => arc(rooms::destroy_without_room),
        "rooms/stages#show" => arc(rooms::call_channels::show),
        "rooms/stages#new" => arc(rooms::call_channels::new),
        "rooms/stages#create" => arc(rooms::call_channels::create),
        "rooms/stages#edit" => arc(rooms::call_channels::edit),
        "rooms/stages#update" => arc(rooms::call_channels::update),
        "rooms/stages#index" => arc(rooms::index),
        "rooms/stages#destroy" => arc(rooms::destroy_without_room),

        "rooms/huddles#show" => arc(rooms::huddles::show),
        "rooms/huddles#create" => arc(rooms::huddles::create),
        "rooms/huddles#participants" => arc(rooms::huddles::participants),
        "rooms/huddles#leave" => arc(rooms::huddles::leave),
        "users/huddle_presence#show" => arc(rooms::huddles::presence),
        "rooms/stage/roles#update" => arc(rooms::stage_participation::role),
        "rooms/stage/hands#create" => arc(rooms::stage_participation::raise),
        "rooms/stage/hands#destroy" => arc(rooms::stage_participation::lower),
        "rooms/stage/streams#create" => arc(rooms::stage_streams::create),
        "rooms/stage/streams#destroy" => arc(rooms::stage_streams::destroy),
        "rooms/call_moderation#mute" => arc(rooms::call_moderation::mute),
        "rooms/call_moderation#unmute" => arc(rooms::call_moderation::unmute),
        "rooms/call_moderation#disconnect" => arc(rooms::call_moderation::disconnect),
        "internal/huddle#authorize" => arc(internal_huddle::authorize),
        "internal/huddle#show" => arc(internal_huddle::show),
        "internal/huddle#left" => arc(internal_huddle::left),
        "welcome#show" => arc(welcome::show),
        "first_runs#show" => arc(first_runs::show),
        "first_runs#create" => arc(first_runs::create),
        "sessions/transfers#show" => arc(sessions::transfers::show),
        "sessions/transfers#update" => arc(sessions::transfers::update),
        "sessions#new" => arc(sessions::new),
        "sessions#create" => arc(sessions::create),
        "sessions#destroy" => arc(sessions::destroy),
        "content_security_policy_reports#create" => arc(csp_reports::create),
        "accounts/users#index" => arc(accounts::users::index),
        "accounts/users#update" => arc(accounts::users::update),
        "accounts/users#destroy" => arc(accounts::users::destroy),
        "accounts/bots/keys#update" => arc(accounts::bots::keys::update),
        "accounts/bots#index" => arc(accounts::bots::index),
        "accounts/bots#create" => arc(accounts::bots::create),
        "accounts/bots#new" => arc(accounts::bots::new),
        "accounts/bots#edit" => arc(accounts::bots::edit),
        "accounts/bots#update" => arc(accounts::bots::update),
        "accounts/bots#destroy" => arc(accounts::bots::destroy),
        "accounts/join_codes#create" => arc(accounts::join_codes::create),
        "accounts/logos#show" => arc(accounts::logos::show),
        "accounts/logos#destroy" => arc(accounts::logos::destroy),
        "accounts/custom_styles#edit" => arc(accounts::custom_styles::edit),
        "accounts/custom_styles#update" => arc(accounts::custom_styles::update),
        "accounts#edit" => arc(accounts::edit),
        "accounts#update" => arc(accounts::update),
        "users#new" => arc(users::new),
        "users#create" => arc(users::create),
        "users#show" => arc(users::show),
        "qr_code#show" => arc(qr_code::show),
        "users/avatars#show" => arc(users::avatars::show),
        "users/avatars#destroy" => arc(users::avatars::destroy),
        "users/bans#create" => arc(users::bans::create),
        "users/bans#destroy" => arc(users::bans::destroy),
        "users/sidebars#show" => arc(users::sidebars::show),
        "users/profiles#show" => arc(users::profiles::show),
        "users/profiles#update" => arc(users::profiles::update),
        "users/push_subscriptions/test_notifications#create" => arc(users::push_subscriptions::test_notifications::create),
        "users/push_subscriptions#index" => arc(users::push_subscriptions::index),
        "users/push_subscriptions#create" => arc(users::push_subscriptions::create),
        "users/push_subscriptions#destroy" => arc(users::push_subscriptions::destroy),
        "autocompletable/users#index" => arc(autocompletable::users::index),
        "messages#index" => arc(messages::index),
        "messages#create" => arc(messages::create),
        "messages#edit" => arc(messages::edit),
        "messages#show" => arc(messages::show),
        "messages#update" => arc(messages::update),
        "messages#destroy" => arc(messages::destroy),
        "messages/boosts/by_bots#create" => arc(messages::boosts::by_bots::create),
        "messages/boosts/by_bots#destroy" => arc(messages::boosts::by_bots::destroy),
        "messages/by_bots#index" => arc(messages::by_bots::index),
        "messages/by_bots#create" => arc(messages::by_bots::create),
        "messages/by_bots#update" => arc(messages::by_bots::update),
        "messages/by_bots#destroy" => arc(messages::by_bots::destroy),
        "messages/boosts#index" => arc(messages::boosts::index),
        "messages/boosts#create" => arc(messages::boosts::create),
        "messages/boosts#new" => arc(messages::boosts::new),
        "messages/boosts#destroy" => arc(messages::boosts::destroy),
        "rooms/refreshes#show" => arc(rooms::refreshes::show),
        "rooms/involvements#show" => arc(rooms::involvements::show),
        "rooms/involvements#update" => arc(rooms::involvements::update),
        "rooms#index" => arc(rooms::index),
        "rooms#show" => arc(rooms::show),
        "rooms#destroy" => arc(rooms::destroy),
        "rooms/opens#index" | "rooms/closeds#index" | "rooms/directs#index" => arc(rooms::index),
        "rooms/opens#create" => arc(rooms::opens::create),
        "rooms/opens#new" => arc(rooms::opens::new),
        "rooms/opens#edit" => arc(rooms::opens::edit),
        "rooms/opens#show" => arc(rooms::opens::show),
        "rooms/opens#update" => arc(rooms::opens::update),
        "rooms/opens#destroy" | "rooms/closeds#destroy" => arc(rooms::destroy_without_room),
        "rooms/closeds#create" => arc(rooms::closeds::create),
        "rooms/closeds#new" => arc(rooms::closeds::new),
        "rooms/closeds#edit" => arc(rooms::closeds::edit),
        "rooms/closeds#show" => arc(rooms::closeds::show),
        "rooms/closeds#update" => arc(rooms::closeds::update),
        "rooms/directs#create" => arc(rooms::directs::create),
        "rooms/directs#new" => arc(rooms::directs::new),
        "rooms/directs#edit" => arc(rooms::directs::edit),
        "rooms/directs#show" => arc(rooms::directs::show),
        "rooms/directs#destroy" => arc(rooms::directs::destroy),
        "searches#index" => arc(searches::index),
        "searches#create" => arc(searches::create),
        "searches#clear" => arc(searches::clear),
        "unfurl_links#create" => arc(unfurl_links::create),
        "pwa#manifest" => arc(pwa::manifest),
        "pwa#service_worker" => arc(pwa::service_worker),
        "rails/health#show" => arc(health::show),
        "turbo/native/navigation#recede" => arc(turbo_native::recede),
        "turbo/native/navigation#resume" => arc(turbo_native::resume),
        "turbo/native/navigation#refresh" => arc(turbo_native::refresh),
        // `config.action_mailbox.ingress = :relay`: the other ingresses aren't configured. The
        // relay endpoint is wired to WS10 below.
        "action_mailbox/ingresses/relay/inbound_emails#create" => arc(crate::mail::relay),
        "action_mailbox/ingresses/postmark/inbound_emails#create"
        | "action_mailbox/ingresses/sendgrid/inbound_emails#create"
        | "action_mailbox/ingresses/mandrill/inbound_emails#health_check"
        | "action_mailbox/ingresses/mandrill/inbound_emails#create"
        | "action_mailbox/ingresses/mailgun/inbound_emails#create" => arc(mailbox::ingress_not_configured),
        "rails/conductor/action_mailbox/inbound_emails#index"
        | "rails/conductor/action_mailbox/inbound_emails#create"
        | "rails/conductor/action_mailbox/inbound_emails#new"
        | "rails/conductor/action_mailbox/inbound_emails#show"
        | "rails/conductor/action_mailbox/inbound_emails/sources#new"
        | "rails/conductor/action_mailbox/inbound_emails/sources#create"
        | "rails/conductor/action_mailbox/reroutes#create"
        | "rails/conductor/action_mailbox/incinerates#create" => arc(mailbox::conductor),
        "active_storage/blobs/redirect#show" => arc(active_storage::blobs_redirect),
        "active_storage/blobs/proxy#show" => arc(active_storage::blobs_proxy),
        "active_storage/representations/redirect#show" => arc(active_storage::representations_redirect),
        "active_storage/representations/proxy#show" => arc(active_storage::representations_proxy),
        "active_storage/disk#show" => arc(active_storage::disk_show),
        "active_storage/disk#update" => arc(active_storage::disk_update),
        "active_storage/direct_uploads#create" => arc(active_storage::direct_uploads_create),
        _ => return None,
    })
}

/// The single Axum entry point: find the route, install its path params, run its action.
pub async fn dispatch(c: &mut Ctx) -> Result {
    let path = normalize_path(c.request.path());
    let Some((route, path_params)) = recognize(&c.request.method, &path)? else {
        return Err(Error::NotFound);
    };
    install_path_params(c, path_params);
    c.set_current(MatchedRoute { endpoint: route.endpoint });
    route.action.call(c).await
}

/// The first route matching `method` and the normalized `path`, with its path parameters
/// (defaults, then captures, then `controller`/`action`). HEAD requests match GET routes.
pub fn recognize(method: &Method, path: &str) -> Result<Option<(&'static Route, ParamMap)>> {
    let verb = if *method == Method::HEAD { &Method::GET } else { method };
    for route in routes() {
        if route.verb != *verb {
            continue;
        }
        let Some(captures) = route.regex.captures(path) else { continue };
        let mut params = ParamMap::new();
        for (name, value) in route.defaults {
            params.insert(*name, Param::Str(value.to_string()));
        }
        for (i, name) in route.names.iter().enumerate() {
            if let Some(value) = captures.get(i + 1) {
                params.insert(name.clone(), Param::Str(unescape_uri(value.as_str())?));
            }
        }
        params.insert("controller", Param::Str(route.controller().to_string()));
        params.insert("action", Param::Str(route.action_name().to_string()));
        return Ok(Some((route, params)));
    }
    Ok(None)
}

/// `params` is body params, then query params, then path params (`request.parameters`).
fn install_path_params(c: &mut Ctx, path_params: ParamMap) {
    let mut params = c.request_params.clone();
    params.merge(&c.query_params);
    params.merge(&path_params);
    c.params = params;
    c.path_params = path_params;
}

/// `Journey::Router::Utils.normalize_path`: one leading slash, repeated slashes squeezed,
/// trailing slashes dropped, percent-escapes upcased.
pub fn normalize_path(path: &str) -> String {
    let mut normalized = String::with_capacity(path.len() + 1);
    for c in format!("/{path}").chars() {
        if c == '/' && normalized.ends_with('/') {
            continue;
        }
        normalized.push(c);
    }
    while normalized.len() > 1 && normalized.ends_with('/') {
        normalized.pop();
    }
    static ESCAPE: LazyLock<Regex> = LazyLock::new(|| Regex::new("%[a-fA-F0-9]{2}").unwrap());
    ESCAPE.replace_all(&normalized, |m: &regex::Captures| m[0].to_uppercase()).into_owned()
}

/// `Journey::Router::Utils.unescape_uri`, then Rails' check that the parameter is valid UTF-8
/// (`ActionController::BadRequest` otherwise).
fn unescape_uri(value: &str) -> Result<String> {
    let bytes: Vec<u8> = percent_encoding::percent_decode_str(value).collect();
    String::from_utf8(bytes).map_err(|_| Error::BadRequest("Invalid path parameters: Invalid encoding".into()))
}

/// A Journey path spec as an anchored regex, plus its parameter names in order.
fn compile(pattern: &str) -> (Regex, Vec<String>) {
    let mut regex = String::from("^");
    let mut names = Vec::new();
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '(' => regex.push_str("(?:"),
            ')' => regex.push_str(")?"),
            ':' | '*' => {
                let mut name = String::new();
                while let Some(&n) = chars.peek().filter(|n| n.is_ascii_alphanumeric() || **n == '_') {
                    name.push(n);
                    chars.next();
                }
                regex.push_str(if c == ':' { "([^/.?]+)" } else { "(.+?)" });
                names.push(name);
            }
            other => regex.push_str(&regex::escape(&other.to_string())),
        }
    }
    regex.push('$');
    (Regex::new(&regex).expect("valid route pattern"), names)
}

/// A route whose Rails action exists but hasn't been ported yet: a 501 naming the endpoint (also
/// in the `x-campfire-not-ported` header), never a 404 that could pass for Rails' answer.
pub async fn not_yet_ported(c: &mut Ctx) -> Result {
    let endpoint = c.current::<MatchedRoute>().map_or("?", |route| route.endpoint);
    tracing::warn!(endpoint, "route not yet ported");
    c.set_header("x-campfire-not-ported", endpoint);
    Ok(c.render_as(StatusCode::NOT_IMPLEMENTED, "text/plain", format!("Not yet ported: {endpoint}\n")))
}

/// A declared route whose action the controller doesn't define (`AbstractController::ActionNotFound`).
pub async fn action_not_found(_c: &mut Ctx) -> Result {
    Err(Error::NotFound)
}

/// A declared route whose controller doesn't exist, e.g. `resource :settings` under rooms: the
/// reference answers 500 (the controller constant fails to load).
pub async fn missing_controller(c: &mut Ctx) -> Result {
    let endpoint = c.current::<MatchedRoute>().map_or("?", |route| route.endpoint);
    Err(Error::internal(anyhow::anyhow!("uninitialized constant for {endpoint}")))
}

/// `Rails::HealthController`
mod health {
    use campfire_kit::format;

    use super::*;

    pub async fn show(c: &mut Ctx) -> Result {
        match c.respond_to(&[&format::HTML, &format::JSON])? {
            f if *f == format::JSON => {
                let timestamp = jiff::Timestamp::from_second(c.now().as_second()).unwrap_or(c.now());
                c.json(StatusCode::OK, &serde_json::json!({ "status": "up", "timestamp": timestamp.to_string() }))
            }
            _ => Ok(c.html(r#"<!DOCTYPE html><html><body style="background-color: green"></body></html>"#)),
        }
    }
}

/// `Turbo::Native::NavigationController` (turbo-rails)
mod turbo_native {
    use super::*;

    pub async fn recede(c: &mut Ctx) -> Result {
        Ok(c.html("Going back…"))
    }

    pub async fn resume(c: &mut Ctx) -> Result {
        Ok(c.html("Staying put…"))
    }

    pub async fn refresh(c: &mut Ctx) -> Result {
        Ok(c.html("Refreshing…"))
    }
}

/// Action Mailbox's ingress and conductor routes, which Campfire doesn't use.
mod mailbox {
    use super::*;

    /// `ActionMailbox::BaseController#ensure_configured`: no ingress is configured.
    pub async fn ingress_not_configured(c: &mut Ctx) -> Result {
        Ok(c.head(StatusCode::NOT_FOUND))
    }

    /// `Rails::Conductor::BaseController`: CSRF first, then `ensure_development_env`.
    pub async fn conductor(c: &mut Ctx) -> Result {
        c.verify_authenticity_token()?;
        Ok(c.head(StatusCode::FORBIDDEN))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Deserialize)]
    struct Vectors {
        routes: Vec<RailsRoute>,
        recognitions: Vec<Recognition>,
    }

    #[derive(serde::Deserialize)]
    struct RailsRoute {
        verb: String,
        path: String,
        endpoint: String,
        defaults: std::collections::BTreeMap<String, String>,
    }

    #[derive(serde::Deserialize)]
    struct Recognition {
        verb: String,
        path: String,
        endpoint: Option<String>,
        params: std::collections::BTreeMap<String, String>,
    }

    fn vectors() -> Vectors {
        let json = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../vectors/campfire_routes.json"));
        serde_json::from_str(json).unwrap()
    }

    /// Controllers the reference routes to but doesn't define (recognize_path raises for them).
    const MISSING_CONTROLLERS: &[&str] = &["rooms/settings"];

    #[test]
    fn the_table_is_rails_routes_in_order() {
        let rails = vectors().routes;
        let ours = routes();
        for (i, (rails, ours)) in rails.iter().zip(ours).enumerate() {
            let defaults: std::collections::BTreeMap<String, String> =
                ours.defaults.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
            assert_eq!(
                (rails.verb.as_str(), rails.path.as_str(), rails.endpoint.as_str(), &rails.defaults),
                (ours.verb.as_str(), ours.pattern, ours.endpoint, &defaults),
                "route #{i}"
            );
        }
        assert_eq!(rails.len(), ours.len(), "every Rails route is in the table, and nothing else");
    }

    #[test]
    fn recognizes_paths_like_rails() {
        for sample in vectors().recognitions {
            let method = Method::from_bytes(sample.verb.as_bytes()).unwrap();
            let path = normalize_path(&sample.path);
            let recognized = recognize(&method, &path).unwrap();
            match (&sample.endpoint, recognized) {
                (Some(endpoint), Some((route, params))) => {
                    assert_eq!(route.endpoint, endpoint, "{} {}", sample.verb, sample.path);
                    let mut params: std::collections::BTreeMap<String, String> =
                        params.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string())).collect();
                    params.remove("controller");
                    params.remove("action");
                    assert_eq!(params, sample.params, "{} {}", sample.verb, sample.path);
                }
                (None, Some((route, _))) => {
                    assert!(MISSING_CONTROLLERS.contains(&route.controller()), "{} {} matched {}", sample.verb, sample.path, route.endpoint)
                }
                (None, None) => {}
                (Some(endpoint), None) => panic!("{} {} should be {endpoint}", sample.verb, sample.path),
            }
        }
    }

    #[test]
    fn every_ported_endpoint_is_a_real_rails_action() {
        let actions: std::collections::HashMap<&str, ActionStatus> =
            campfire_routes::TABLE.iter().map(|route| (route.endpoint, route.action)).collect();
        let mut ported_count = 0;
        for route in campfire_routes::TABLE {
            if ported(route.endpoint).is_some() {
                ported_count += 1;
                assert!(
                    matches!(actions[route.endpoint], ActionStatus::Defined | ActionStatus::Implicit),
                    "{} is ported but Rails answers {:?}",
                    route.endpoint,
                    route.action
                );
            }
        }
        assert!(ported_count > 0);
        // A typo in `ported` would silently leave the endpoint on `not_yet_ported`.
        for endpoint in PORTED_ENDPOINTS {
            assert!(actions.contains_key(endpoint), "{endpoint} isn't in config/routes.rb");
            assert!(ported(endpoint).is_some(), "{endpoint}");
        }
    }

    /// Every endpoint `ported` maps, so the test above can check each exists in the table.
    const PORTED_ENDPOINTS: &[&str] = &[
        "rooms/voices#index",
        "rooms/voices#show",
        "rooms/voices#new",
        "rooms/voices#create",
        "rooms/voices#edit",
        "rooms/voices#update",
        "rooms/voices#destroy",
        "rooms/stages#index",
        "rooms/stages#show",
        "rooms/stages#new",
        "rooms/stages#create",
        "rooms/stages#edit",
        "rooms/stages#update",
        "rooms/stages#destroy",
        "rooms/huddles#show", "rooms/huddles#create", "rooms/huddles#participants", "rooms/huddles#leave", "users/huddle_presence#show",
        "welcome#show", "first_runs#show", "first_runs#create", "sessions/transfers#show",
        "sessions/transfers#update", "sessions#new", "sessions#create", "sessions#destroy",
        "content_security_policy_reports#create", "accounts/users#index", "accounts/users#update",
        "accounts/users#destroy", "accounts/bots/keys#update", "accounts/bots#index", "accounts/bots#create",
        "accounts/bots#new", "accounts/bots#edit", "accounts/bots#update", "accounts/bots#destroy",
        "accounts/join_codes#create", "accounts/logos#show", "accounts/logos#destroy",
        "accounts/custom_styles#edit", "accounts/custom_styles#update", "accounts#edit", "accounts#update",
        "users#new", "users#create", "users#show", "qr_code#show", "users/avatars#show",
        "users/avatars#destroy", "users/bans#create", "users/bans#destroy", "users/sidebars#show",
        "users/profiles#show", "users/profiles#update", "users/push_subscriptions/test_notifications#create",
        "users/push_subscriptions#index", "users/push_subscriptions#create", "users/push_subscriptions#destroy",
        "autocompletable/users#index", "messages#index", "messages#create", "messages#edit", "messages#show",
        "messages#update", "messages#destroy", "messages/boosts/by_bots#create",
        "messages/boosts/by_bots#destroy", "messages/by_bots#index", "messages/by_bots#create",
        "messages/by_bots#update", "messages/by_bots#destroy", "messages/boosts#index",
        "messages/boosts#create", "messages/boosts#new", "messages/boosts#destroy", "rooms/refreshes#show",
        "rooms/involvements#show", "rooms/involvements#update", "rooms#index", "rooms#show", "rooms#destroy",
        "rooms/opens#index", "rooms/closeds#index", "rooms/directs#index", "rooms/opens#create",
        "rooms/opens#new", "rooms/opens#edit", "rooms/opens#show", "rooms/opens#update", "rooms/opens#destroy",
        "rooms/closeds#destroy", "rooms/closeds#create", "rooms/closeds#new", "rooms/closeds#edit",
        "rooms/closeds#show", "rooms/closeds#update", "rooms/directs#create", "rooms/directs#new",
        "rooms/directs#edit", "rooms/directs#show", "rooms/directs#destroy", "searches#index",
        "searches#create", "searches#clear", "unfurl_links#create", "pwa#manifest", "pwa#service_worker",
        "rails/health#show", "turbo/native/navigation#recede", "turbo/native/navigation#resume",
        "turbo/native/navigation#refresh", "action_mailbox/ingresses/postmark/inbound_emails#create",
        "action_mailbox/ingresses/sendgrid/inbound_emails#create",
        "action_mailbox/ingresses/mandrill/inbound_emails#health_check",
        "action_mailbox/ingresses/mandrill/inbound_emails#create",
        "action_mailbox/ingresses/mailgun/inbound_emails#create",
        "rails/conductor/action_mailbox/inbound_emails#index",
        "rails/conductor/action_mailbox/inbound_emails#create",
        "rails/conductor/action_mailbox/inbound_emails#new",
        "rails/conductor/action_mailbox/inbound_emails#show",
        "rails/conductor/action_mailbox/inbound_emails/sources#new",
        "rails/conductor/action_mailbox/inbound_emails/sources#create",
        "rails/conductor/action_mailbox/reroutes#create", "rails/conductor/action_mailbox/incinerates#create",
        "active_storage/blobs/redirect#show", "active_storage/blobs/proxy#show",
        "active_storage/representations/redirect#show", "active_storage/representations/proxy#show",
        "active_storage/disk#show", "active_storage/disk#update", "active_storage/direct_uploads#create",
    ];

    fn dispatch_router() -> axum::Router {
        use campfire_kit::{Kit, KitConfig, testing};
        let kit = Kit::new(KitConfig::default(), testing::crypto(), testing::frozen_clock(), ());
        let routes = axum::Router::new()
            .route("/", axum::routing::any(campfire_kit::action(dispatch)))
            .route("/{*path}", axum::routing::any(campfire_kit::action(dispatch)));
        campfire_kit::app(routes, kit)
    }

    async fn request(method: &str, path: &str) -> (StatusCode, Option<String>, String) {
        use tower::ServiceExt;
        let request = axum::http::Request::builder()
            .method(method)
            .uri(path)
            .header("host", "campfire.test")
            .body(axum::body::Body::empty())
            .unwrap();
        let response = dispatch_router().oneshot(request).await.unwrap();
        let status = response.status();
        let header = response.headers().get("x-campfire-not-ported").map(|v| v.to_str().unwrap().to_string());
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, header, String::from_utf8_lossy(&body).into_owned())
    }

    #[tokio::test]
    async fn unported_actions_say_so_instead_of_404ing() {
        let unported = campfire_routes::TABLE
            .iter()
            .find(|route| {
                let path = route.spec.trim_end_matches("(.:format)");
                route.verb == "GET"
                    && route.action == ActionStatus::Defined
                    && ported(route.endpoint).is_none()
                    && !path.contains([':', '*', '('])
            })
            .expect("an unported GET route without params");
        let path = unported.spec.trim_end_matches("(.:format)");
        let (status, header, body) = request("GET", path).await;
        assert_eq!(status, StatusCode::NOT_IMPLEMENTED, "{path}");
        assert_eq!(header.as_deref(), Some(unported.endpoint));
        assert_eq!(body, format!("Not yet ported: {}\n", unported.endpoint));

        // A route Rails declares without the action stays Rails' 404, and unknown paths too.
        let (status, header, _) = request("GET", "/first_run/new").await;
        assert_eq!((status, header), (StatusCode::NOT_FOUND, None));
        let (status, header, _) = request("GET", "/nope/nope").await;
        assert_eq!((status, header), (StatusCode::NOT_FOUND, None));
    }

    #[test]
    fn normalizes_paths() {
        assert_eq!(normalize_path(""), "/");
        assert_eq!(normalize_path("//rooms//1//"), "/rooms/1");
        assert_eq!(normalize_path("/a%2fb"), "/a%2Fb");
    }
}
