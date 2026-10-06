//! The channels (crates/channels), at the path they had in this crate. This module also holds
//! the tests of theirs that boot the whole app or reach the layers above, until the test crate
//! takes them (plans/crate-split-plan.md, "Tests").

pub(crate) use campfire_channels::channels::*;

#[cfg(test)]
mod huddle_effects_tests;
#[cfg(test)]
pub(crate) mod tests;

#[cfg(test)]
pub(crate) mod message_features {
    pub(crate) use campfire_channels::channels::message_features::*;

    mod tests;
}

#[cfg(test)]
pub(crate) mod sink {
    pub(crate) use campfire_channels::channels::sink::*;

    mod stream_tests;
    mod stream_remaining_cases;
}
