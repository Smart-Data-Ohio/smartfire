//! `controllers::channel_thread_messages` lives in the campfire_controllers crate (plans/crate-
//! split-plan.md). Its tests that need the whole app are mounted here, at their old path.

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) mod write_tests;
