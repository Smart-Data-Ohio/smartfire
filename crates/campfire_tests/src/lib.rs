//! The tests that boot the whole app or reach a layer above their own, at the module paths they
//! had in the campfire crate, so their names don't change (plans/crate-split-plan.md, "Tests").
//! Everything here is test code.

#![cfg(test)]

mod admin {
    pub(crate) use campfire::admin::*;
    // What `twitter_tests` names through `super::`.
    use campfire_db::{Connection, schema};

    mod twitter_tests;
}
mod channels;
mod controllers;
mod jobs;
use campfire::server;

// The app layer (crates/app), at the paths its modules had in the campfire crate. `app`, `huddle`
// and `integrations` hold the tests of theirs that boot the whole app or reach the layers above.
use campfire_app::{config, errors, net, queue, security};
#[cfg(test)]
use campfire_app::{cable, state, test_support};
#[cfg(test)]
use campfire_web::{authentication, messaging};

// The web layer (crates/web), likewise. `concerns`, `mail` and `controllers::presenters` (in
// `controllers`) hold tests of theirs that boot the whole app.
use campfire_web::{active_storage, rich_text};

mod concerns {
    pub use campfire_web::concerns::*;

    #[cfg(test)]
    mod bot_model_cases;
}

mod mail {
    pub(crate) use campfire_web::mail::*;

    #[cfg(test)]
    mod tests;
}

mod app {
    pub(crate) use campfire_app::app::*;

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
    mod ws14_profile_status_tests;
    #[cfg(test)]
    mod google_push_channel_tests;
    #[cfg(test)]
    pub(crate) mod google_test_support;
    #[cfg(test)]
    #[path = "../../../../test-support/asset_goldens.rs"]
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
}

#[cfg(test)]
mod huddle {
    pub(crate) use campfire_app::huddle::*;

    #[cfg(test)]
    mod tests;
}

mod integrations {
    pub(crate) use campfire_app::integrations::*;

    #[cfg(test)]
    pub(crate) mod action_claims {
        pub(crate) use campfire_app::integrations::action_claims::*;

        mod tests;
    }
    #[cfg(test)]
    pub(crate) mod agent_repositories {
        pub(crate) use campfire_app::integrations::agent_repositories::*;

        mod tests;
        mod live_tests;
        mod bot_plaintext_cases;
        mod review_tests;
    }
    #[cfg(test)]
    pub(crate) mod agent_streaming {
        // Nothing here names the app module's items now that the channels have moved out, but
        // the path keeps reaching them.
        #[allow(unused_imports)]
        pub(crate) use campfire_app::integrations::agent_streaming::*;

        mod tests;
        mod case_tests;
    }
    pub(crate) mod fizzy {
        pub(crate) use campfire_app::integrations::fizzy::*;

        #[cfg(test)]
        mod tests;
        #[cfg(test)]
        pub(crate) mod accounts {
            pub(crate) use campfire_app::integrations::fizzy::accounts::*;

            mod tests;
        }
        #[cfg(test)]
        pub(crate) mod agent_job {
            pub(crate) use campfire_app::integrations::fizzy::agent_job::*;

            mod tests;
        }
        #[cfg(test)]
        pub(crate) mod agent_reads {
            pub(crate) use campfire_app::integrations::fizzy::agent_reads::*;

            mod tests;
        }
        #[cfg(test)]
        pub(crate) mod agent_requests {
            pub(crate) use campfire_app::integrations::fizzy::agent_requests::*;

            mod tests;
        }
        #[cfg(test)]
        pub(crate) mod cards {
            pub(crate) use campfire_app::integrations::fizzy::cards::*;

            mod tests;
        }
    }
    pub(crate) mod github {
        pub(crate) use campfire_app::integrations::github::*;

        #[cfg(test)]
        pub(crate) mod tests;
        #[cfg(test)]
        pub(crate) mod agent_actions {
            pub(crate) use campfire_app::integrations::github::agent_actions::*;

            mod tests;
        }
        #[cfg(test)]
        pub(crate) mod fetcher {
            pub(crate) use campfire_app::integrations::github::fetcher::*;

            mod tests;
        }
        #[cfg(test)]
        pub(crate) mod health {
            pub(crate) use campfire_app::integrations::github::health::*;

            mod tests;
        }
        #[cfg(test)]
        pub(crate) mod notifier {
            pub(crate) use campfire_app::integrations::github::notifier::*;

            mod tests;
        }
        #[cfg(test)]
        pub(crate) mod pull_requests {
            pub(crate) use campfire_app::integrations::github::pull_requests::*;

            mod tests;
        }
        #[cfg(test)]
        pub(crate) mod references {
            pub(crate) use campfire_app::integrations::github::references::*;

            pub(super) mod tests;
        }
    }
    #[cfg(test)]
    pub(crate) mod image_proxy {
        pub(crate) use campfire_app::integrations::image_proxy::*;

        mod tests;
    }
    #[cfg(test)]
    pub(crate) mod link_embed {
        pub(crate) use campfire_app::integrations::link_embed::*;

        mod rails_reference_tests;
        mod rails_fetcher_tests;
        pub(crate) mod fetcher {
            pub(crate) use campfire_app::integrations::link_embed::fetcher::*;

            mod tests;
        }
        pub(crate) mod store {
            pub(crate) use campfire_app::integrations::link_embed::store::*;

            mod tests;
        }
    }
    #[cfg(test)]
    pub(crate) mod slack {
        pub(crate) use campfire_app::integrations::slack::*;

        #[cfg(test)]
        mod sequence_tests;
        #[cfg(test)]
        pub(crate) mod client {
            pub(crate) use campfire_app::integrations::slack::client::*;

            pub(crate) mod tests;
        }
        #[cfg(test)]
        pub(crate) mod conversations {
            pub(crate) use campfire_app::integrations::slack::conversations::*;

            mod tests;
        }
        #[cfg(test)]
        pub(crate) mod jobs {
            pub(crate) use campfire_app::integrations::slack::jobs::*;

            pub(crate) mod tests;
        }
        #[cfg(test)]
        pub(crate) mod oauth {
            pub(crate) use campfire_app::integrations::slack::oauth::*;

            mod tests;
        }
        #[cfg(test)]
        mod payload {
            mod tests;
        }
        #[cfg(test)]
        pub(crate) mod runner {
            pub(crate) use campfire_app::integrations::slack::runner::*;

            mod tests;
        }
        #[cfg(test)]
        pub(crate) mod store {
            pub(crate) use campfire_app::integrations::slack::store::*;

            pub(crate) mod tests;
            mod lifecycle_tests;
        }
        #[cfg(test)]
        pub(crate) mod undoer {
            pub(crate) use campfire_app::integrations::slack::undoer::*;

            pub(crate) mod tests;
        }
        #[cfg(test)]
        pub(crate) mod users {
            pub(crate) use campfire_app::integrations::slack::users::*;

            mod tests;
        }
        #[cfg(test)]
        pub(crate) mod writer {
            pub(crate) use campfire_app::integrations::slack::writer::*;

            mod tests;
        }
    }
    pub(crate) mod twitter {
        pub(crate) use campfire_app::integrations::twitter::*;

        #[cfg(test)]
        pub(crate) mod fetcher {
            pub(crate) use campfire_app::integrations::twitter::fetcher::*;

            mod tests;
        }
        #[cfg(test)]
        pub(crate) mod references {
            pub(crate) use campfire_app::integrations::twitter::references::*;

            mod tests;
        }
    }
    #[cfg(test)]
    pub(crate) mod web_push {
        pub(crate) use campfire_app::integrations::web_push::*;

        mod tests;
    }
}

/// The binary's allocator, which these tests ran under in the campfire crate.
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

/// And its options (see campfire's main.rs).
#[cfg(target_os = "linux")]
#[unsafe(export_name = "_rjem_malloc_conf")]
pub static JEMALLOC_CONF: Option<&'static std::ffi::c_char> =
    // SAFETY: points at the first byte of a static, NUL-terminated string.
    Some(unsafe { &*c"thp:never".as_ptr() });

mod sync_twin_tests;
mod slash_commands_tests;
