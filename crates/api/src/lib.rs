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

mod dto;
pub mod endpoints;
mod error;
pub mod admin;
pub mod bots;
pub mod message_actions;
pub mod settings;
pub mod slack;
pub mod sync;
pub mod threads;
#[cfg(feature = "test-support")]
pub mod test_hooks;

use std::sync::Arc;

use axum::Router;
use axum::routing::{delete, get, patch, post};
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
        .route("/api/v1/sidebar", get(action(endpoints::sidebar)))
        .route("/api/v1/rooms/{room_id}", get(action(endpoints::room)))
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
            patch(unparsed_action(message_actions::update))
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
        .route("/api/v1/saved", post(unparsed_action(message_actions::save)))
        .route(
            "/api/v1/saved/{saved_id}",
            delete(action(message_actions::unsave)),
        )
        .route(
            "/api/v1/rooms/{room_id}/threads",
            get(action(threads::threads)).post(unparsed_action(threads::create)),
        )
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
        .route("/api/v1/users", get(action(endpoints::users)))
        .route("/api/v1/presence", get(action(endpoints::presence)))
        .merge(settings::routes())
        .merge(admin::routes())
        .merge(bots::routes())
        .merge(slack::routes())
        .merge(app.cable.sync_router::<Kit>(SYNC_PATH))
}
