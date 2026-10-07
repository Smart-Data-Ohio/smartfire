//! `controllers::public_pages` lives in the campfire_controllers crate (plans/crate-split-plan.md).
//! Its tests that need the whole app are mounted here, at their old path.

#[cfg(test)]
#[path = "public_pages/sign_in_google_tests.rs"]
mod sign_in_google_tests;

#[cfg(test)]
mod tests;
