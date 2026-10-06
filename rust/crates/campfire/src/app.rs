//! The app's state: [`AppState`] is what controllers, channels, jobs and integrations share.
//! Actions reach it with `c.app()` ([`AppCtx`]). [`crate::server`] boots it.

use std::sync::Arc;

use campfire_db::Database;
use campfire_kit::{Ctx, SharedClock};
use campfire_storage::Storage;
use campfire_views::fragment_cache::FragmentCache;
use rails_compat::Secrets;

use crate::config::Config;

pub use crate::cable::Cable;

/// Everything that outlives a request. Cheap to share as [`App`].
pub struct AppState {
    pub config: Config,
    pub secrets: Arc<Secrets>,
    pub ar_encryption: Arc<rails_compat::ar_encryption::ArEncryption>,
    pub clock: SharedClock,
    pub db: Database,
    pub storage: Arc<Storage>,
    pub cable: Cable,
    pub broadcasts: crate::cable::Broadcasts,
    pub jobs: crate::queue::Jobs,
    pub mail: crate::state::mail::State,
    pub fizzy: crate::integrations::fizzy::State,
    pub agent_message_payload: crate::state::agent_payload::State,
    pub agent_repositories: crate::integrations::agent_repositories::State,
    pub sudo: crate::state::sudo::State,
    pub two_factor: crate::state::two_factor::State,
    pub google: crate::integrations::google::State,
    pub errors: crate::errors::Reporter,
    /// `config.x.web_push_pool`; `None` when Web Push is off (no valid VAPID keys).
    pub web_push: Option<crate::integrations::web_push::Pool>,
    pub github_accounts: crate::integrations::github::accounts::Accounts,
    pub github_app: crate::integrations::github::client::AppClient,
    pub github_read: crate::integrations::github::client::ReadClient,
    pub subscription_network: crate::net::Network,
    pub slack_network: crate::net::Network,
    /// `Rails.cache` for view fragments (`cache message do`), current during every request
    /// and every render outside one.
    pub fragment_cache: Arc<FragmentCache>,
}

impl AppState {
    /// The key pages offer browsers to subscribe with: none while Web Push is off, so that browsers
    /// don't subscribe to notifications that would never be sent.
    pub fn vapid_public_key(&self) -> Option<String> {
        self.web_push
            .as_ref()
            .and(self.config.vapid_public_key.clone())
    }
}

pub type App = Arc<AppState>;

/// `c.app()` in actions.
pub trait AppCtx {
    fn app(&self) -> &App;
}

impl AppCtx for Ctx {
    fn app(&self) -> &App {
        self.state::<App>()
    }
}

#[cfg(test)]
mod security_tests;

#[cfg(test)]
mod sudo_tests;

#[cfg(test)]
mod two_factor_tests;

#[cfg(test)]
mod challenge_tests;

#[cfg(test)]
mod enforcement_tests;
#[cfg(test)]
mod direct_upload_tests;

#[cfg(test)]
mod session_management_tests;

#[cfg(test)]
mod admin_two_factor_tests;
#[cfg(test)]
mod full_page_tests;
#[cfg(test)]
mod profile_security_tests;
#[cfg(test)]
mod round_four_security_tests;
#[cfg(test)]
mod round_three_security_tests;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod google_tests;

#[cfg(test)]
mod google_webhook_tests;

#[cfg(test)]
pub(crate) mod google_api_tests;

#[cfg(test)]
mod google_connection_tests;

#[cfg(test)]
mod google_drive_tests;

#[cfg(test)]
mod google_calendar_job_tests;
#[cfg(test)]
mod google_meeting_refresh_tests;
#[cfg(test)]
mod google_push_channel_tests;

#[cfg(test)]
pub(crate) mod google_test_support;

#[cfg(test)]
#[path = "../../../test-support/asset_goldens.rs"]
pub(crate) mod asset_goldens;

#[cfg(test)]
mod google_review_tests;
#[cfg(test)]
mod google_consumer_tests;

#[cfg(test)]
mod google_lifecycle_tests;

#[cfg(test)]
mod google_admin_tests;

#[cfg(test)]
mod google_page_tests;

#[cfg(test)]
mod google_reporting_tests;

#[cfg(test)]
mod google_message_tests;

#[cfg(test)]
pub(crate) mod cutover_c_tests;

#[cfg(test)]
mod cutover_d_tests;
