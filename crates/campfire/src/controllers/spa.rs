//! `controllers::spa` lives in the campfire_controllers crate (plans/crate-split-plan.md).
//! This module re-exports it under its old path and mounts the tests that still need the whole
//! app.

pub use campfire_controllers::controllers::spa::*;

#[cfg(test)]
#[path = "spa_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "spa_api_tests.rs"]
mod api_tests;

#[cfg(test)]
#[path = "spa_api_s2_tests.rs"]
mod api_s2_tests;
#[cfg(test)]
#[path = "spa_api_threads_tests.rs"]
mod api_threads_tests;

#[cfg(test)]
#[path = "spa_settings_tests.rs"]
mod settings_tests;
