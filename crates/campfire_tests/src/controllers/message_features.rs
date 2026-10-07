//! The message features (crates/messages), at the path they had in this crate. This module also
//! holds their tests that boot the whole app, until the test crate takes them
//! (plans/crate-split-plan.md, "Tests").

pub(crate) use campfire_messages::controllers::message_features::*;

#[cfg(test)]
mod search_header_tests;
#[cfg(test)]
mod review_229_tests;
#[cfg(test)]
mod list_scaling_tests;
#[cfg(test)]
mod extreme_range_tests;
#[cfg(test)]
mod private_provider_tests;
#[cfg(test)]
mod provider_batch_tests;
#[cfg(test)]
mod ws12_consumer_tests;
#[cfg(test)]
mod saved_tests;
#[cfg(test)]
mod coercion_tests;
#[cfg(test)]
mod scheduled_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod review_tests;
#[cfg(test)]
mod rescue_format_tests;
#[cfg(test)]
mod slash_tests;
#[cfg(test)]
mod links_files_tests;
#[cfg(test)]
mod reminder_tests;
#[cfg(test)]
mod quote_integration_tests;
#[cfg(test)]
mod root_cache_tests;
#[cfg(test)]
mod panel_tests;
#[cfg(test)]
mod pin_poll_scaling_tests;
#[cfg(test)]
mod exceptional_input_tests;
#[cfg(test)]
mod slash_named_tests;
#[cfg(test)]
mod date_tests;
#[cfg(test)]
mod provider_tests;
#[cfg(test)]
mod composer_tests;
#[cfg(test)]
mod older_provider_tests;
#[cfg(test)]
mod bounded_provider_tests;
#[cfg(test)]
mod mapped_provider_tests;
#[cfg(test)]
mod older_owner_tests;
#[cfg(test)]
mod older_calendar_tests;
#[cfg(test)]
mod older_embed_job_tests;
#[cfg(test)]
mod older_calendar_execution_tests;
#[cfg(test)]
mod older_embed_children_tests;
#[cfg(test)]
mod older_embed_failure_tests;
#[cfg(test)]
mod comparison_support;
#[cfg(test)]
mod final_state_sibling_tests;
#[cfg(test)]
mod calendar_retry_consumer_tests;
#[cfg(test)]
mod container_input_tests;
#[cfg(test)]
mod wide_html_tests;
#[cfg(test)]
mod periodic_delivery_tests;
