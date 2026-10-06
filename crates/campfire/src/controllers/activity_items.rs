//! `controllers::activity_items` lives in the campfire_controllers crate (plans/crate-split-plan.md).
//! This module re-exports it under its old path and mounts the tests that still need the whole
//! app.

pub use campfire_controllers::controllers::activity_items::*;

#[cfg(test)]
mod tests;
