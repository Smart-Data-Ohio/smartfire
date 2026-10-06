//! `controllers::channel_thread_messages` lives in the campfire_controllers crate (plans/crate-split-plan.md).
//! This module re-exports it under its old path and mounts the tests that still need the whole
//! app.

pub use campfire_controllers::controllers::channel_thread_messages::*;

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) mod write_tests;
