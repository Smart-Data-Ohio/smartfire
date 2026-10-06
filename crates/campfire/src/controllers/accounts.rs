//! `controllers::accounts` lives in the campfire_people crate (plans/crate-split-plan.md). This
//! module re-exports it under its old path and mounts the tests that still need the whole app.

pub use campfire_people::controllers::accounts::*;

#[cfg(test)]
mod mutation_tests;

#[cfg(test)]
mod view_tests;

#[cfg(test)]
mod attachment_tests;

#[cfg(test)]
mod original_control_tests;

#[cfg(test)]
pub(crate) mod audit_logs {
    pub(crate) use campfire_people::controllers::accounts::audit_logs::*;

    mod tests;
    mod original_tests;
    mod original_caps_tests;
}

#[cfg(test)]
pub(crate) mod bots {
    pub(crate) use campfire_people::controllers::accounts::bots::*;

    mod access_boundary_tests;
    mod coercion_tests;
    mod mutation_boundary_tests;
    mod interleaving_tests;
    mod normalized_tests;
    mod render_replay_tests;
    mod casting_followups_tests;
}

#[cfg(test)]
pub(crate) mod icons {
    pub(crate) use campfire_people::controllers::accounts::icons::*;

    mod tests;
    mod original_tests;
}

#[cfg(test)]
pub(crate) mod logos {
    pub(crate) use campfire_people::controllers::accounts::logos::*;

    mod tests;
}
