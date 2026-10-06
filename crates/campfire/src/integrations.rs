//! Campfire's outbound HTTP, and its search query preprocessing, each matching the Rails app:
//!
//! - [`web_push`]: `WebPush::Pool`/`WebPush::Notification` (reference/lib/web_push,
//!   reference/config/initializers/web_push.rb) and `Room::MessagePusher`'s delivery: permitted
//!   push services only, the endpoint resolved through the private network guard and pinned.
//! - [`opengraph`]: `UnfurlLinksController#create` over `Opengraph::*`: every address guarded
//!   and pinned, every redirect re-checked, 10 responses and 5MB at most.
//! - [`webhook`]: `Webhook#deliver` for bots: public addresses pinned, signed payloads and 7-second timeouts.
//! - [`search`]: the query sanitizing in `SearchesController#query`.
//! - [`web_push_pool`]: the Web Push pool the app boots with.
//!
//! The three HTTP clients share only plumbing ([`net`]); each keeps its own policy (see
//! plans/rust-conversion.md, "HTTP clients: three distinct policies"). The tests' expected
//! outputs in testdata/ were recorded by Ruby scripts run in the Rails app, and are frozen.

pub mod google;
pub mod fizzy;
#[allow(dead_code)] // Staged until WS16's runner and HTTP controllers are ported.
pub mod slack;
pub mod twitter;
pub mod image_proxy;
pub mod agent_repositories;
pub(crate) mod agent_streaming;
pub mod action_claims;
// WS15g installs the GitHub account, fetcher, notifier and approved-action consumers.
#[allow(dead_code)]
pub mod github;
pub mod health;
pub mod link_embed;
#[allow(dead_code)]
pub mod linkedin;
pub(crate) use crate::net;
pub mod opengraph;
pub mod web_push;
pub mod webhook;

#[cfg(test)]
pub(crate) mod test_support;



/// config/initializers/web_push.rb (`config.x.web_push_pool`): the pool, whose invalid
/// subscription handler destroys the subscription (`Push::Subscription.find_by(id:)&.destroy`).
/// `None`, and Web Push is off, when the VAPID keys are missing or invalid. Call from inside the
/// runtime.
pub fn web_push_pool(config: &crate::config::Config, db: &campfire_db::Database) -> Option<web_push::Pool> {
    let vapid = match web_push::VapidConfig::from_config(config) {
        Ok(vapid) => vapid,
        Err(error @ web_push::VapidError::Missing) => {
            tracing::warn!("Web Push is off: {error}");
            return None;
        }
        Err(error) => {
            tracing::error!("Web Push is off: {error}");
            return None;
        }
    };
    let db = db.clone();
    Some(web_push::Pool::new(crate::net::Network::system(), vapid, move |id| {
        db.write_blocking(move |tx| match campfire_db::PushSubscription::find(tx.conn(), id) {
            Ok(subscription) => subscription.destroy(tx),
            Err(campfire_db::Error::RecordNotFound(_)) => Ok(()),
            Err(error) => Err(error),
        })
    }))
}

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

pub(crate) mod message_batches;

#[cfg(test)]
pub(crate) use agent_streaming::run_trailing_fixture;

