//! `controllers::fizzy_connections` lives in the campfire_controllers crate (plans/crate-split-plan.md).
//! This module re-exports it under its old path and mounts the tests that still need the whole
//! app.

pub use campfire_controllers::controllers::fizzy_connections::*;

#[cfg(test)]
mod tests;
