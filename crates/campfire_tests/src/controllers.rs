//! The tests of `controllers` (the router root in campfire and the controller modules of the
//! crates split from it) that boot the whole app, at the paths they had in campfire
//! (plans/crate-split-plan.md, "Tests"). The modules below re-export each controller module and
//! mount its tests.

pub(crate) use campfire::controllers::*;

pub mod accounts;
pub mod activity_items;
pub mod agent_approvals;
pub mod channel_thread_messages;
pub mod channel_threads;
pub mod embeds;
pub mod first_runs;
pub mod fizzy_cards;
pub mod fizzy_connections;
pub mod fizzy_message_cards;
pub mod github;
pub mod message_embed_suppressions;
#[cfg(test)]
pub(crate) mod message_features;
#[cfg(test)]
pub(crate) mod message_forwards_tests;
pub mod messages;
pub mod presenters {
    pub(crate) use campfire_web::controllers::presenters::*;

    #[cfg(test)]
    pub mod test_support;
    #[cfg(test)]
    mod message_links;
    #[cfg(test)]
    pub(crate) mod sql_probe;
    #[cfg(test)]
    mod chrome_tests;
    #[cfg(test)]
    pub(crate) mod accounts {
        pub(crate) use campfire_web::controllers::presenters::accounts::*;
        mod tests;
    }
    #[cfg(test)]
    pub(crate) mod agent_payload {
        pub(crate) use campfire_web::controllers::presenters::agent_payload::*;
        mod tests;
        mod callback_tests;
        mod finalization_tests;
        mod reference_callback_tests;
        mod live_tests;
    }
    #[cfg(test)]
    pub(crate) mod attachments {
        pub(crate) use campfire_web::controllers::presenters::attachments::*;
        mod tests;
        mod avatar_logo_tests;
    }
    #[cfg(test)]
    pub(crate) mod link_embeds {
        pub(crate) use campfire_web::controllers::presenters::link_embeds::*;
        mod tests;
        mod linkedin_tests;
    }
    #[cfg(test)]
    pub(crate) mod message_payload {
        pub(crate) use campfire_web::controllers::presenters::message_payload::*;
        mod unicode_tests;
    }
    #[cfg(test)]
    pub(crate) mod twitter_cards {
        pub(crate) use campfire_web::controllers::presenters::twitter_cards::*;
        mod tests;
    }
}
#[cfg(test)]
mod activity_domain_tests;
#[cfg(test)]
mod human_work_tests;
pub mod public_pages;
pub mod pwa;
pub mod qr_code;
pub mod rooms;
pub mod saved_items;
pub mod scheduled_messages;
pub mod searches;
pub mod slack;
pub mod spa;
pub mod sudos;
pub mod switchers;
pub mod two_factor;
pub mod unfurl_links;
pub mod users;
pub mod welcome;
#[cfg(test)]
mod internal_huddle_tests;
#[cfg(test)]
mod internal_huddle_declaration_tests;

#[cfg(test)]
mod agent_http_tests;
#[cfg(test)]
mod agent_mcp_tests;

#[cfg(test)]
mod agent_surface_tests;

#[cfg(test)]
mod bot_http_tests;
#[cfg(test)]
mod agent_legacy_bot_tests;

#[cfg(test)]
mod agent_conversation_tests;

#[cfg(test)]
mod agent_fizzy_tests;

#[cfg(test)]
mod agent_fizzy_action_tests;

#[cfg(test)]
mod agent_reads_tests;

#[cfg(test)]
mod agent_pins_tests;

#[cfg(test)]
mod agent_polls_tests;
#[cfg(test)]
mod agent_permissions_tests;

#[cfg(test)]
mod agent_reactions_tests;

#[cfg(test)]
mod agent_polling_tests;

#[cfg(test)]
mod agent_work_validation_tests;
#[cfg(test)]
mod agent_work_writes_tests;
#[cfg(test)]
mod agent_work_named_tests;
#[cfg(test)]
mod agent_next6_named_tests;

#[cfg(test)]
mod agent_pr227_tests;
#[cfg(test)]
mod ws12_agent_work_query_tests;
#[cfg(test)]
mod ws12_activity_helper_named_tests;
#[cfg(test)]
mod ws12_huddle_copy_tests;
#[cfg(test)]
mod ws12_unread_broadcast_tests;

#[cfg(test)]
mod agent_attachments_tests;

#[cfg(test)]
mod agent_review_tests;

#[cfg(test)]
mod agent_review_r2_tests;
#[cfg(test)]
mod agent_review_r3_tests;
#[cfg(test)]
mod agent_review_r4_tests;
#[cfg(test)]
mod agent_review_r5_tests;

#[cfg(test)]
mod agent_array_read_tests;

#[cfg(test)]
mod agent_array_shape_tests;

#[cfg(test)]
mod agent_budget_notice_tests;

#[cfg(test)]
mod ws12_handled_sequence_tests;

#[cfg(test)]
mod ws12_private_pr_owner_tests;

#[cfg(test)]
mod ws12_inbox_remaining_tests;

#[cfg(test)]
mod ws12_work_remaining_tests;

#[cfg(test)]
mod ws12_browser_remaining_tests;

#[cfg(test)]
mod ws11ui_original_browser_tests;

#[cfg(test)]
pub(crate) mod ledger_browser_tests;

#[cfg(test)]
mod template_coverage_tests;

#[cfg(test)]
mod drive_browser_tests;
