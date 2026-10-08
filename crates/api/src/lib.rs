//! The single-page app's server side: the `/api/v1` JSON endpoints and the `/api/v1/sync`
//! socket. Mounted (with `/app`) only when `SPA_ENABLED` is set.
//!
//! The wire types are `campfire_api_types`'. The socket's protocol, replay ring and batching
//! are `campfire_cable::sync`; the JSON twins of the classic broadcasts are published from
//! `campfire_app::cable::sync`, rendered by this crate's [`sync::Renderer`].

/// A `/api/v1` endpoint: `$body` runs with the JSON error envelope around it.
macro_rules! endpoint {
    ($(#[$doc:meta])* $name:ident => $body:ident) => {
        $(#[$doc])*
        pub async fn $name(c: &mut campfire_kit::Ctx) -> campfire_kit::Result {
            crate::error::prepare(c);
            let result = $body(c).await;
            crate::error::respond(c, result)
        }
    };
}

pub mod activity;
pub mod agents;
pub mod cards;
pub mod fizzy;
pub mod github;
pub mod composer;
mod cursor;
pub mod directory;
mod dto;
pub mod endpoints;
pub mod events;
mod error;
pub mod admin;
pub mod bots;
pub mod boards;
pub mod huddles;
pub mod join;
pub mod message_actions;
pub mod organize;
pub mod room_management;
pub mod people;
pub mod search;
pub mod settings;
pub mod slack;
pub mod stage;
pub mod sync;
pub mod threads;
pub mod work;
#[cfg(feature = "test-support")]
pub mod test_hooks;

use std::sync::Arc;

use axum::Router;
use axum::routing::{delete, get, patch, post, put};
use campfire_app::app::AppState;
use campfire_kit::{Kit, action, unparsed_action};

/// Where the sync socket is mounted.
pub const SYNC_PATH: &str = "/api/v1/sync";

/// Starts the sync engine on the app's cable server and has the broadcast points publish their
/// JSON twins. Call once, at boot, on the runtime the app serves from.
pub fn install(app: &Arc<AppState>) {
    let runtime = tokio::runtime::Handle::current();
    app.broadcasts
        .install_sync_renderer(Arc::new(sync::Renderer::new(app, runtime)));
    app.cable.install_sync(
        sync::Handler::new(app),
        campfire_cable::sync::SyncConfig::default(),
    );
}

/// The `/api/v1` routes and the sync socket.
pub fn routes(app: &AppState) -> Router<Kit> {
    Router::new()
        .route("/api/v1/me", get(action(endpoints::me)))
        .route(
            "/api/v1/rooms/{room_id}/messages/{message_id}/fizzy_cards/new",
            get(action(fizzy::new)),
        )
        .route(
            "/api/v1/rooms/{room_id}/threads/{thread_id}/messages/{message_id}/fizzy_cards/new",
            get(action(fizzy::new)),
        )
        .route(
            "/api/v1/rooms/{room_id}/messages/{message_id}/fizzy_cards",
            post(unparsed_action(fizzy::create)),
        )
        .route(
            "/api/v1/rooms/{room_id}/threads/{thread_id}/messages/{message_id}/fizzy_cards",
            post(unparsed_action(fizzy::create)),
        )
        .route("/api/v1/sidebar", get(action(endpoints::sidebar)))
        .route("/api/v1/rooms", post(unparsed_action(room_management::create)))
        .route("/api/v1/rooms/new", get(action(room_management::new)))
        .route("/api/v1/rooms/{room_id}/edit", get(action(room_management::edit)))
        .route("/api/v1/rooms/{room_id}/membership", delete(action(room_management::leave)))
        .route("/api/v1/rooms/{room_id}/preview", get(action(join::preview)))
        .route("/api/v1/rooms/{room_id}/join", post(action(join::join)))
        .route("/api/v1/rooms/{room_id}", get(action(endpoints::room)).patch(unparsed_action(room_management::update)).delete(action(room_management::destroy)))
        .route(
            "/api/v1/rooms/{room_id}/messages",
            get(action(endpoints::messages)).post(unparsed_action(endpoints::create_message)),
        )
        .route(
            "/api/v1/rooms/{room_id}/read",
            post(unparsed_action(endpoints::mark_read))
                .delete(unparsed_action(endpoints::mark_unread)),
        )
        .route(
            "/api/v1/rooms/{room_id}/pins",
            get(action(message_actions::pins)),
        )
        .route(
            "/api/v1/messages/{message_id}",
            get(action(message_actions::show))
                .patch(unparsed_action(message_actions::update))
                .delete(action(message_actions::destroy)),
        )
        .route(
            "/api/v1/messages/{message_id}/source",
            get(action(message_actions::source)),
        )
        .route(
            "/api/v1/messages/{message_id}/boosts",
            post(unparsed_action(message_actions::create_boost)),
        )
        .route(
            "/api/v1/messages/{message_id}/boosts/{boost_id}",
            delete(action(message_actions::destroy_boost)),
        )
        .route(
            "/api/v1/messages/{message_id}/pin",
            post(action(message_actions::pin)).delete(action(message_actions::unpin)),
        )
        .route(
            "/api/v1/messages/{message_id}/forwards",
            post(unparsed_action(message_actions::forward)),
        )
        .route(
            "/api/v1/forward_destinations",
            get(action(message_actions::forward_destinations)),
        )
        .route(
            "/api/v1/saved",
            get(action(message_actions::saved)).post(unparsed_action(message_actions::save)),
        )
        .route(
            "/api/v1/saved/{saved_id}",
            patch(unparsed_action(message_actions::update_saved))
                .delete(action(message_actions::unsave)),
        )
        .route(
            "/api/v1/room_categories",
            post(unparsed_action(organize::create_category)),
        )
        .route(
            "/api/v1/room_categories/order",
            put(unparsed_action(organize::order_categories)),
        )
        .route(
            "/api/v1/room_categories/{category_id}",
            patch(unparsed_action(organize::update_category))
                .delete(action(organize::destroy_category)),
        )
        .route(
            "/api/v1/rooms/{room_id}/category",
            put(unparsed_action(organize::assign_category)),
        )
        .route(
            "/api/v1/rooms/{room_id}/favorite",
            post(action(organize::favorite))
                .patch(unparsed_action(organize::move_favorite))
                .delete(action(organize::unfavorite)),
        )
        .route(
            "/api/v1/rooms/{room_id}/involvement",
            put(unparsed_action(organize::involvement)),
        )
        .route(
            "/api/v1/rooms/{room_id}/polls",
            post(unparsed_action(cards::create_poll)),
        )
        .route(
            "/api/v1/rooms/{room_id}/polls/{poll_id}",
            get(action(cards::poll)),
        )
        .route(
            "/api/v1/rooms/{room_id}/polls/{poll_id}/vote",
            post(unparsed_action(cards::vote)),
        )
        .route(
            "/api/v1/rooms/{room_id}/events",
            get(action(events::index)).post(unparsed_action(events::create)),
        )
        .route(
            "/api/v1/rooms/{room_id}/events/new",
            get(action(events::new)),
        )
        .route(
            "/api/v1/rooms/{room_id}/events/{event_id}",
            get(action(events::show)).patch(unparsed_action(events::update)),
        )
        .route(
            "/api/v1/rooms/{room_id}/events/{event_id}/edit",
            get(action(events::edit)),
        )
        .route(
            "/api/v1/rooms/{room_id}/events/{event_id}/cancel",
            patch(unparsed_action(events::cancel)),
        )
        .route(
            "/api/v1/rooms/{room_id}/events/{event_id}/attendance",
            get(action(cards::attendance)).put(unparsed_action(cards::respond)),
        )
        .route(
            "/api/v1/rooms/{room_id}/github/pull_requests/{id}/card",
            get(action(cards::github_card)),
        )
        .route(
            "/api/v1/rooms/{room_id}/github/pull_requests/{id}/actions",
            get(action(github::github_actions)),
        )
        .route(
            "/api/v1/rooms/{room_id}/github/pull_requests/{id}/comments",
            post(unparsed_action(github::github_comment)),
        )
        .route(
            "/api/v1/rooms/{room_id}/github/pull_requests/{id}/reviews",
            post(unparsed_action(github::github_review)),
        )
        .route(
            "/api/v1/rooms/{room_id}/github/pull_requests/{id}/review_requests",
            post(unparsed_action(github::github_review_request)),
        )
        .route(
            "/api/v1/rooms/{room_id}/github/pull_requests/{id}/discussion",
            post(unparsed_action(github::github_discussion)),
        )
        .route(
            "/api/v1/rooms/{room_id}/fizzy/cards/{id}/card",
            get(action(cards::fizzy_card)),
        )
        .route(
            "/api/v1/rooms/{room_id}/message_links/{reference_id}/card",
            get(action(cards::quote_card)),
        )
        .route("/api/v1/search", get(action(search::index)))
        .route(
            "/api/v1/search/recents",
            get(action(search::recents))
                .post(unparsed_action(search::record))
                .delete(action(search::clear)),
        )
        .route("/api/v1/activity", get(action(activity::index)))
        .route(
            "/api/v1/activity/unread_count",
            get(action(activity::unread_count)),
        )
        .route(
            "/api/v1/activity/{id}",
            patch(unparsed_action(activity::update)),
        )
        .route(
            "/api/v1/activity/{id}/open",
            post(action(activity::open)),
        )
        .route(
            "/api/v1/rooms/{room_id}/threads",
            get(action(threads::threads)).post(unparsed_action(threads::create)),
        )
        .route("/api/v1/rooms/{room_id}/board", get(action(boards::index)))
        .route("/api/v1/rooms/{room_id}/posts/new", get(action(boards::new)))
        .route("/api/v1/rooms/{room_id}/posts", post(unparsed_action(boards::create)))
        .route(
            "/api/v1/threads/{thread_id}",
            get(action(threads::thread))
                .patch(unparsed_action(threads::update))
                .delete(action(threads::destroy)),
        )
        .route(
            "/api/v1/threads/{thread_id}/messages",
            get(action(threads::messages)).post(unparsed_action(threads::reply)),
        )
        .route(
            "/api/v1/threads/{thread_id}/join",
            post(unparsed_action(threads::join)).delete(action(threads::leave)),
        )
        .route(
            "/api/v1/threads/{thread_id}/read",
            post(action(threads::read)),
        )
        .route(
            "/api/v1/threads/{thread_id}/work",
            patch(unparsed_action(work::update)),
        )
        .route(
            "/api/v1/threads/{thread_id}/work/handoff",
            post(unparsed_action(work::handoff)),
        )
        .route("/api/v1/work", get(action(work::index)))
        .route("/api/v1/uploads", post(unparsed_action(composer::upload)))
        .route(
            "/api/v1/autocomplete/users",
            get(action(composer::autocomplete_users)),
        )
        .route(
            "/api/v1/autocomplete/icons",
            get(action(composer::autocomplete_icons)),
        )
        .route("/api/v1/icons", get(action(composer::icons)))
        .route(
            "/api/v1/rooms/{room_id}/slash_commands",
            get(action(composer::slash_commands))
                .post(unparsed_action(composer::run_slash_command)),
        )
        .route(
            "/api/v1/rooms/{room_id}/messages/preview",
            post(unparsed_action(composer::preview)),
        )
        .route(
            "/api/v1/rooms/{room_id}/scheduled_messages",
            post(unparsed_action(composer::schedule)),
        )
        .route(
            "/api/v1/scheduled_messages",
            get(action(composer::scheduled_messages)),
        )
        .route(
            "/api/v1/scheduled_messages/{id}",
            patch(unparsed_action(composer::update_scheduled))
                .delete(action(composer::cancel_scheduled)),
        )
        .route(
            "/api/v1/scheduled_messages/{id}/send_now",
            post(action(composer::send_scheduled_now)),
        )
        .route(
            "/api/v1/directs/candidates",
            get(action(directory::direct_candidates)),
        )
        .route(
            "/api/v1/directs",
            post(unparsed_action(directory::create_direct)),
        )
        .route(
            "/api/v1/directs/{room_id}",
            patch(unparsed_action(directory::rename_direct)),
        )
        .route(
            "/api/v1/directs/{room_id}/members",
            post(unparsed_action(directory::add_direct_members)),
        )
        .route(
            "/api/v1/rooms/{room_id}/members",
            get(action(directory::members)),
        )
        .route("/api/v1/rooms/{room_id}/files", get(action(directory::files)))
        .route(
            "/api/v1/users/{user_id}/star",
            put(action(directory::star)).delete(action(directory::unstar)),
        )
        .route("/api/v1/switcher", get(action(directory::switcher)))
        .route("/api/v1/users", get(action(endpoints::users)))
        .route("/api/v1/presence", get(action(endpoints::presence)))
        .route("/api/v1/huddles", get(action(huddles::index)))
        .route(
            "/api/v1/rooms/{room_id}/huddle",
            get(action(huddles::show)).post(unparsed_action(huddles::join)),
        )
        .route(
            "/api/v1/rooms/{room_id}/huddle/leave",
            axum::routing::post(unparsed_action(huddles::leave)),
        )
        .route(
            "/api/v1/rooms/{room_id}/huddle/moderation",
            axum::routing::post(unparsed_action(huddles::moderate)),
        )
        .route("/api/v1/rooms/{room_id}/stage", get(action(stage::show)))
        .route(
            "/api/v1/rooms/{room_id}/stage/members/{membership_id}",
            axum::routing::patch(unparsed_action(stage::change_role)),
        )
        .route(
            "/api/v1/rooms/{room_id}/stage/hand",
            axum::routing::post(unparsed_action(stage::raise_hand))
                .delete(unparsed_action(stage::lower_hand)),
        )
        .route(
            "/api/v1/rooms/{room_id}/stage/stream",
            axum::routing::post(unparsed_action(stage::start_stream))
                .delete(unparsed_action(stage::stop_stream)),
        )
        .route("/api/v1/agents", get(action(agents::index)))
        .route("/api/v1/agents/{agent_id}", get(action(agents::show)))
        .route(
            "/api/v1/agents/{agent_id}/approvals",
            get(action(agents::approvals)),
        )
        .route(
            "/api/v1/agents/{agent_id}/events",
            get(action(agents::events)),
        )
        .route(
            "/api/v1/agent_approvals/{id}",
            patch(unparsed_action(agents::decide)),
        )
        .merge(people::routes())
        .merge(settings::routes())
        .merge(admin::routes())
        .merge(bots::routes())
        .merge(slack::routes())
        .merge(app.cable.sync_router::<Kit>(SYNC_PATH))
}
