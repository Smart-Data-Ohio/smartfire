//! The app layer of the Campfire server: the state every request, channel, job and integration
//! shares ([`app`]), its configuration, the job queue's enqueueing side, the cable's typed
//! broadcasts, and the integrations' clients. The crates above build controllers, channels and
//! jobs on it (plans/crate-split-plan.md).

pub mod account_security;
pub mod app;
// The cable server's user, stream names and typed broadcasts.
pub mod cable;
pub mod config;
pub mod errors;
pub mod huddle;
// Slash launch readiness delegates to the WS13 configuration API.
pub mod huddle_readiness;
// The icon catalog: config/icons.yml's brands and the workspace's own.
pub mod icons;
pub mod integrations;
// The outbound HTTP stack every client shares.
pub mod net;
pub mod picker_configuration;
pub mod public_policy;
// The job queue's enqueueing side: job classes, queues and the durable sink.
pub mod queue;
// Ruby core conversions.
pub mod ruby;
pub mod security;
// App state whose behaviour lives in the layers above.
pub mod state;

// Bounded waits and server startup for tests, here and in the crates above.
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

pub mod json_cache;
