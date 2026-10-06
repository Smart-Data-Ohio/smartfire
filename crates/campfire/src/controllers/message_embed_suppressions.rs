//! `controllers::message_embed_suppressions` lives in the campfire_controllers crate (plans/crate-split-plan.md).
//! This module re-exports it under its old path and mounts the tests that still need the whole
//! app.

pub use campfire_controllers::controllers::message_embed_suppressions::*;

#[cfg(test)]
mod tests;
