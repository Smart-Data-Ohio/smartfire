//! The app's state: [`AppState`] is what controllers, channels, jobs and integrations share.
//! Actions reach it with `c.app()` ([`AppCtx`]). [`crate::server`] boots it.

use std::sync::Arc;

use campfire_db::Database;
use campfire_kit::{Ctx, SharedClock};
use campfire_storage::Storage;
use crate::json_cache::JsonCache;
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
    /// Bounded cache for legacy JSON serializers.
    pub json_cache: Arc<JsonCache>,
}

impl AppState {
    /// The original system helpers change LiveKit ENV inside a test body.
    /// Rebind only that real configuration, keeping the booted model/service
    /// dependencies and all installed adapter contents in the private host.
    #[cfg(any(test, feature = "test-support"))]
    pub fn fixture_huddle_config(&self, lookup: impl Fn(&str) -> Option<String>) -> App {
        let mut config = self.config.clone();
        config.livekit_url = lookup("LIVEKIT_URL");
        config.huddle = crate::huddle::Config::from_lookup(&lookup);
        config.huddles_configured = crate::huddle_readiness::huddles_configured(&lookup);
        Arc::new(Self {
            config,
            secrets: self.secrets.clone(),
            ar_encryption: self.ar_encryption.clone(),
            clock: self.clock.clone(),
            db: self.db.clone(),
            storage: self.storage.clone(),
            cable: self.cable.clone(),
            broadcasts: self.broadcasts.clone(),
            jobs: self.jobs.clone(),
            mail: self.mail.fixture_snapshot(),
            fizzy: crate::integrations::fizzy::State {
                network: self.fizzy.network.clone(), base: self.fizzy.base.clone(),
            },
            agent_message_payload: self.agent_message_payload.fixture_snapshot(),
            agent_repositories: self.agent_repositories.fixture_snapshot(),
            sudo: self.sudo.fixture_snapshot(),
            two_factor: self.two_factor.fixture_snapshot(),
            google: self.google.clone(),
            errors: self.errors.clone(),
            web_push: self.web_push.clone(),
            github_accounts: self.github_accounts.clone(),
            github_app: self.github_app.clone(),
            github_read: self.github_read.clone(),
            subscription_network: self.subscription_network.clone(),
            slack_network: self.slack_network.clone(),
            json_cache: self.json_cache.clone(),
        })
    }

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
