//! The background jobs (crates/channels), at the path they had in this crate. This module also
//! holds the tests of theirs that boot the whole app or reach the layers above, until the test
//! crate takes them (plans/crate-split-plan.md, "Tests").

pub(crate) use campfire_channels::jobs::*;
// What `reminders` and `huddle_render_tests` name through `super::`.
#[cfg(test)]
use crate::queue::{Registry, discard_missing};

#[cfg(test)]
pub(crate) mod reminders;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod peer_callback_tests;
#[cfg(test)]
mod huddle_render_tests;
#[cfg(test)]
mod huddle_policy_integration_tests;
#[cfg(test)]
mod huddle_neighbor_mention_test;

#[cfg(test)]
pub(crate) mod agent_jobs {
    pub(crate) use campfire_channels::jobs::agent_jobs::*;

    mod tests;
    mod publication_tests;
    mod indicator_tests;
    #[path = "../agent_payload_tests.rs"]
    mod payload_tests;
    mod case_tests;
    mod webhook_cases;
    mod recovery_cases;
    mod delivery_path_cases;
    mod webhook_key_cases;
    mod drive_attachment_cases;
    mod message_controller_tests;
    mod remaining_cases;
    pub(crate) mod next6_named;
}

#[cfg(test)]
pub(crate) mod huddle {
    pub(crate) use campfire_channels::jobs::huddle::*;

    mod review_tests;
    mod reviewer_probes;
    mod reviewer_r3_job_probes;
    mod ring_matrix_tests;
}

#[cfg(test)]
pub(crate) mod integrations {
    pub(crate) use campfire_channels::jobs::integrations::*;

    mod tests;
}
