//! `controllers::rooms` lives in the campfire_rooms crate (plans/crate-split-plan.md). This
//! module re-exports it under its old path and mounts the tests that still need the whole app.
//! `shell` stays here: it is the test-only state loader those tests call.

pub use campfire_rooms::controllers::rooms::*;

#[cfg(test)]
use crate::controllers::presenters::call_navigation;

#[cfg(test)]
mod call_lifecycle_tests;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod parity_tests;

#[cfg(test)]
mod reads_tests;

#[cfg(test)]
mod join_tests;

#[cfg(test)]
mod channel_audits_tests;

#[cfg(test)]
mod coercions_tests;

#[cfg(test)]
mod direct_selection_tests;

#[cfg(test)]
mod icons_tests;

#[cfg(test)]
mod inbound_tests;

#[cfg(test)]
mod inbound_rails_cases;

#[cfg(test)]
mod direct_forms_tests;

#[cfg(test)]
mod direct_rename_tests;

#[cfg(test)]
mod directs_rails_cases;

#[cfg(test)]
mod rooms_rails_cases;

#[cfg(test)]
mod opens_rails_cases;

#[cfg(test)]
mod closeds_rails_cases;

#[cfg(test)]
#[path = "rooms/ws17_ooo_tests.rs"]
mod ws17_ooo_tests;

#[cfg(test)]
#[path = "rooms/members_rails_cases.rs"]
mod members_rails_cases;

#[cfg(test)]
#[path = "rooms/refreshes_rails_cases.rs"]
mod refreshes_rails_cases;

#[cfg(test)]
mod sidebars_rails_cases;

#[cfg(test)]
mod involvements_rails_cases;

#[cfg(test)]
mod reads_rails_cases;

#[cfg(test)]
mod organization_rails_support;

#[cfg(test)]
mod favorites_rails_cases;

#[cfg(test)]
mod room_categories_rails_cases;

#[cfg(test)]
mod categories_rails_cases;

#[cfg(test)]
mod switchers_rails_cases;

#[cfg(test)]
#[path = "rooms/native_integration_tests.rs"]
mod native_integration_tests;

#[cfg(test)]
pub(crate) mod query_probe;

#[cfg(test)]
#[path = "rooms/owner_panel_tests.rs"]
mod owner_panel_tests;

#[cfg(test)]
mod full_page_tests;

#[cfg(test)]
mod queue_recovery_tests;

#[cfg(test)]
mod review_cache_tests;

#[cfg(test)]
mod public_huddle_tests;

#[cfg(test)]
mod huddle_declaration_tests;

#[cfg(test)]
mod call_channel_tests;

#[cfg(test)]
mod review_tests;

#[cfg(test)]
mod call_channel_declaration_tests;

#[cfg(test)]
mod call_page_tests;

pub(crate) mod shell;

#[cfg(test)]
mod stream_controller_tests;

#[cfg(test)]
mod stage_page_tests;

#[cfg(test)]
mod remaining_call_tests;

#[cfg(test)]
mod remaining_query_tests;

#[cfg(test)]
mod remaining_presence_tests;

#[cfg(test)]
mod room_composition_tests;

#[cfg(test)]
mod room_shell_tests;

#[cfg(test)]
mod full_room_tests;

#[cfg(test)]
mod row_broadcast_tests;

#[cfg(test)]
mod system_browser_tests;

#[cfg(test)]
pub(super) mod call_channel_broadcast_tests;

#[cfg(test)]
mod boards_read_tests;

#[cfg(test)]
mod boards_domain_tests;

#[cfg(test)]
mod boards_rails_cases;

#[cfg(test)]
mod board_destroy_tests;

#[cfg(test)]
mod board_automation_tests;

#[cfg(test)]
mod original_audit_tests;


#[cfg(test)]
pub(crate) mod directs {
    mod picker_tests;
}

#[cfg(test)]
pub(crate) mod events {
    mod tests;
}
