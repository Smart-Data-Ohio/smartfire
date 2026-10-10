//! `controllers::spa` lives in the campfire_controllers crate (plans/crate-split-plan.md).
//! This module re-exports it under its old path and mounts the tests that still need the whole
//! app.

pub use campfire_controllers::controllers::spa::*;

#[cfg(test)]
#[path = "spa_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "url_contract_tests.rs"]
mod url_contract_tests;

#[cfg(test)]
#[path = "url_contract_fixtures.rs"]
mod url_contract_fixtures;

#[cfg(test)]
#[path = "spa_api_tests.rs"]
mod api_tests;

#[cfg(test)]
#[path = "spa_api_s2_tests.rs"]
mod api_s2_tests;
#[cfg(test)]
#[path = "spa_api_threads_tests.rs"]
mod api_threads_tests;
#[cfg(test)]
#[path = "spa_api_composer_tests.rs"]
mod api_composer_tests;
#[cfg(test)]
#[path = "spa_upload_size_tests.rs"]
mod upload_size_tests;
#[cfg(test)]
#[path = "spa_api_directory_tests.rs"]
mod api_directory_tests;
#[cfg(test)]
#[path = "spa_api_activity_tests.rs"]
mod api_activity_tests;
#[cfg(test)]
#[path = "spa_api_agents_tests.rs"]
mod api_agents_tests;
#[cfg(test)]
#[path = "spa_api_work_tests.rs"]
mod api_work_tests;
#[cfg(test)]
#[path = "spa_api_work_links_tests.rs"]
mod spa_api_work_links_tests;
#[cfg(test)]
#[path = "spa_api_board_tests.rs"]
mod spa_api_board_tests;
#[cfg(test)]
#[path = "spa_api_board_automations_tests.rs"]
mod spa_api_board_automations_tests;
#[cfg(test)]
#[path = "spa_settings_tests.rs"]
mod settings_tests;

#[cfg(test)]
#[path = "spa_push_enrollment_tests.rs"]
mod push_enrollment_tests;

#[cfg(test)]
#[path = "spa_coexistence_tests.rs"]
mod coexistence_tests;
#[cfg(test)]
#[path = "spa_huddle_tests.rs"]
mod huddle_tests;
#[cfg(test)]
#[path = "spa_api_search_tests.rs"]
mod api_search_tests;
#[cfg(test)]
#[path = "spa_api_organize_tests.rs"]
mod api_organize_tests;
#[cfg(test)]
#[path = "spa_api_cards_tests.rs"]
mod api_cards_tests;

#[cfg(test)]
#[path = "spa_admin_tests.rs"]
mod admin_tests;
#[cfg(test)]
#[path = "spa_bots_tests.rs"]
mod bots_tests;
#[cfg(test)]
#[path = "spa_slack_tests.rs"]
mod slack_tests;
#[cfg(test)]
#[path = "spa_workspace_branding_tests.rs"]
mod workspace_branding_tests;

#[cfg(test)]
#[path = "spa_integrations_tests.rs"]
mod integrations_tests;

#[cfg(test)]
#[path = "spa_account_tests.rs"]
mod account_tests;

#[cfg(test)]
#[path = "spa_people_tests.rs"]
mod people_tests;

#[cfg(test)]
#[path = "spa_api_room_management_tests.rs"]
mod api_room_management_tests;

#[cfg(test)]
#[path = "spa_api_room_integrations_tests.rs"]
mod api_room_integrations_tests;

#[cfg(test)]
#[path = "spa_api_room_join_tests.rs"]
mod api_room_join_tests;

#[cfg(test)]
#[path = "spa_api_events_tests.rs"]
mod spa_api_events_tests;

#[cfg(test)]
#[path = "spa_api_fizzy_tests.rs"]
mod api_fizzy_tests;

#[cfg(test)]
#[path = "spa_api_drive_tests.rs"]
mod api_drive_tests;

#[cfg(test)]
#[path = "spa_tour_tests.rs"]
mod tour_tests;
