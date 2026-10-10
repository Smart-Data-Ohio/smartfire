//! `MessagesController` (crates/messages), at the path it had in this crate. This module also
//! holds its tests that boot the whole app, until the test crate takes them
//! (plans/crate-split-plan.md, "Tests").

pub use campfire_messages::controllers::messages::*;

#[cfg(test)]
pub(crate) mod boosts_tests;
#[cfg(test)]
mod upload_tests;
#[cfg(test)]
pub(crate) mod attachment_processing_tests;
#[cfg(test)]
mod review_tests;
#[cfg(test)]
mod root_tests;
#[cfg(test)]
mod paging_tests;
#[cfg(test)]
mod cache_reaction_review_tests;
#[cfg(test)]
mod csrf_tests;
#[cfg(test)]
mod declaration_tests;
#[cfg(test)]
pub(crate) mod drive_tests;
#[cfg(test)]
#[cfg(test)]
mod tests;
#[cfg(test)]
mod http_tests;
#[cfg(test)]
#[path = "messages/ws17_activity_tests.rs"]
mod ws17_activity_tests;
