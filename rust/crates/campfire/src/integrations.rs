//! Campfire's outbound HTTP, and its search query preprocessing, each matching the Rails app:
//!
//! - [`web_push`]: `WebPush::Pool`/`WebPush::Notification` (reference/lib/web_push,
//!   reference/config/initializers/web_push.rb) and `Room::MessagePusher`'s delivery: permitted
//!   push services only, the endpoint resolved through the private network guard and pinned.
//! - [`opengraph`]: `UnfurlLinksController#create` over `Opengraph::*`: every address guarded
//!   and pinned, every redirect re-checked, 10 responses and 5MB at most.
//! - [`webhook`]: `Webhook#deliver` for bots: public addresses pinned, signed payloads and 7-second timeouts.
//! - [`search`]: the query sanitizing in `SearchesController#query`.
//! - [`register_jobs`]: `Room::PushMessageJob` and `Bot::WebhookJob` for the job runner.
//!
//! The three HTTP clients share only plumbing ([`net`]); each keeps its own policy (see
//! plans/rust-conversion.md, "HTTP clients: three distinct policies"). Oracles for the tests
//! (Ruby scripts run in the reference) live in testdata/oracle.

pub mod fizzy;
pub mod twitter;
pub mod image_proxy;
mod jobs;
pub mod agent_repositories;
mod agent_streaming;
pub mod action_claims;
// Account, fetcher and notifier consumers remain staged (WS15g continuation).
#[allow(dead_code)]
pub mod github;
pub mod health;
pub mod link_embed;
#[allow(dead_code)]
pub mod linkedin;
pub mod net;
pub mod opengraph;
pub mod search;
pub mod web_push;
pub mod webhook;

#[cfg(test)]
pub(crate) mod test_support;

pub use jobs::{register_jobs, web_push_pool};


/// SQL-only reference callbacks invoked from WS8's message transaction.
pub fn sync_message_references(tx: &mut campfire_db::Tx<'_>, message: &campfire_db::Message, enqueue: bool, crypto: Option<&rails_compat::ar_encryption::ArEncryption>) -> campfire_db::Result<()> {
    link_embed::sync_message(tx, message, enqueue)?;
    fizzy::cards::sync_message(tx, message, enqueue, crypto)?;
    twitter::references::sync_message(tx, message, enqueue)
}
