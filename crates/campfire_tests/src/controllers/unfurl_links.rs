//! `controllers::unfurl_links` lives in the campfire_controllers crate (plans/crate-split-plan.md).
//! Its tests that need the whole app are mounted here, at their old path.

#[cfg(test)]
#[path = "unfurl_links/rails_tests.rs"]
mod rails_tests;

#[cfg(test)]
mod tests;
