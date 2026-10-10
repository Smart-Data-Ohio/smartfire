//! `controllers::channel_threads` lives in the campfire_controllers crate (plans/crate-split-plan.md).
//! This module re-exports it under its old path and mounts the tests that still need the whole
//! app.

pub use campfire_controllers::controllers::channel_threads::*;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod page_tests;

#[cfg(test)]
mod agent_work_tests;


#[cfg(test)]
mod board_write_tests;

#[cfg(test)]
pub(crate) mod write_tests;




#[cfg(test)]
mod declaration_tests;
