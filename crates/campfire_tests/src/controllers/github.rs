//! `controllers::github` lives in the campfire_controllers crate (plans/crate-split-plan.md). Its
//! tests that need the whole app are mounted here, at their old path.

#[cfg(test)]
mod card_tests;

#[cfg(test)]
mod subscription_tests;

#[cfg(test)]
mod connection_tests;

#[cfg(test)]
mod test_support;

#[cfg(test)]
mod health_tests;

#[cfg(test)]
mod discussion_tests;

#[cfg(test)]
mod write_tests;

#[cfg(test)]
mod lifecycle_tests;

#[cfg(test)]
mod agent_tests;

#[cfg(test)]
mod fragment_tests;

#[cfg(test)]
mod room_card_tests;

#[cfg(test)]
mod cutover_d_tests;

#[cfg(test)]
pub(crate) mod webhooks {
    use crate::integrations::github::webhooks;

    mod tests;
}
