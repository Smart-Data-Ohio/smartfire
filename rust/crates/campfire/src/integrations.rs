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

pub mod google;
pub mod fizzy;
#[allow(dead_code)] // Staged until WS16's runner and HTTP controllers are ported.
pub mod slack;
pub mod twitter;
pub mod image_proxy;
mod jobs;
mod agent_jobs;
pub mod agent_repositories;
mod agent_streaming;
pub mod action_claims;
// WS15g installs the GitHub account, fetcher, notifier and approved-action consumers.
#[allow(dead_code)]
pub mod github;
pub mod health;
pub mod link_embed;
#[allow(dead_code)]
pub mod linkedin;
pub mod net;
pub mod opengraph;
pub mod web_push;
pub mod webhook;

#[cfg(test)]
pub(crate) mod test_support;

pub use jobs::{register_jobs, web_push_pool};


/// SQL-only reference callbacks invoked from WS8's message transaction.
pub fn sync_message_references(tx: &mut campfire_db::Tx<'_>, message: &campfire_db::Message, enqueue: bool, crypto: Option<&rails_compat::ar_encryption::ArEncryption>) -> campfire_db::Result<()> {
    use campfire_db::callbacks::Phase;
    for phase in [Phase::MessageFizzyReferences,Phase::MessageTwitterReferences,Phase::MessageLinkReferences] {
        sync_message_reference_phase(tx,message,phase,enqueue,crypto)?;
    }
    Ok(())
}

/// The actual adapters run adjacent to their callback, before or after quotes as Rails declares.
pub fn sync_message_reference_phase(tx: &mut campfire_db::Tx<'_>, message: &campfire_db::Message, phase: campfire_db::callbacks::Phase, enqueue: bool, crypto: Option<&rails_compat::ar_encryption::ArEncryption>) -> campfire_db::Result<()> {
    use campfire_db::callbacks::Phase;
    match phase {
        Phase::MessageFizzyReferences => fizzy::cards::sync_message(tx,message,enqueue,crypto),
        Phase::MessageTwitterReferences => twitter::references::sync_message(tx,message,enqueue),
        Phase::MessageLinkReferences => link_embed::sync_message(tx,message,enqueue),
        _ => Ok(()), // Event references remain the flagged WS14 adapter in the registry.
    }
}
