//! The single-page app's server side: the `/api/v1` JSON endpoints and the `/api/v1/sync`
//! socket. Mounted (with `/app`) only when `SPA_ENABLED` is set.
//!
//! The wire types are `campfire_api_types`'. The socket's protocol, replay ring and batching
//! are `campfire_cable::sync`; the JSON twins of the classic broadcasts are published from
//! `campfire_app::cable::sync`, rendered by this crate's [`sync::Renderer`].

mod dto;
pub mod endpoints;
mod error;
pub mod sync;
#[cfg(feature = "test-support")]
pub mod test_hooks;

use std::sync::Arc;

use axum::Router;
use axum::routing::get;
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
            axum::routing::post(unparsed_action(endpoints::mark_read))
                .delete(unparsed_action(endpoints::mark_unread)),
        )
        .route("/api/v1/users", get(action(endpoints::users)))
        .route("/api/v1/presence", get(action(endpoints::presence)))
        .merge(app.cable.sync_router::<Kit>(SYNC_PATH))
}
