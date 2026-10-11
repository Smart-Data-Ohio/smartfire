//! `controllers::two_factor` lives in the campfire_people crate (plans/crate-split-plan.md); this
//! module re-exports it under its old path.

pub use campfire_people::controllers::two_factor::*;

#[cfg(test)]
mod setup_contracts_tests;
