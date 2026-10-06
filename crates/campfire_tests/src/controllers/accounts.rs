//! `controllers::accounts` lives in the campfire_people crate (plans/crate-split-plan.md). Its
//! tests that need the whole app are mounted here, at their old path.

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
    mod tests;
    mod original_tests;
}

#[cfg(test)]
pub(crate) mod logos {
    mod tests;
}
