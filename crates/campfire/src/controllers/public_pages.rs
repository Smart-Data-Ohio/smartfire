//! `controllers::public_pages` lives in the campfire_controllers crate (plans/crate-split-plan.md).
//! This module re-exports it under its old path and mounts the tests that still need the whole
//! app.

pub use campfire_controllers::controllers::public_pages::*;

#[cfg(test)]
#[path = "public_pages/sign_in_google_tests.rs"]
mod sign_in_google_tests;

#[cfg(test)]
#[path = "public_pages/contract_tests.rs"]
mod contract_tests;

#[cfg(test)]
mod tests;
