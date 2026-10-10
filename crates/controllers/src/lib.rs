//! The remaining controllers of the Campfire server: agents and their approvals, channel
//! threads, GitHub, Fizzy, Google, Slack, searches, switchers, embeds and the other pages. The
//! campfire bin routes to them (plans/crate-split-plan.md). Modules keep their paths from the
//! campfire crate, so `controllers::agents` lives at `campfire_controllers::controllers::agents`.

pub mod controllers {
    pub mod activity_items;
    pub mod agent_approvals;
    pub mod agents;
    pub mod autocompletable;
    pub mod channel_thread_messages;
    pub mod channel_threads;
    pub mod csp_reports;
    pub mod embeds;
    pub mod first_runs;
    pub mod fizzy_cards;
    pub mod fizzy_connections;
    pub mod fizzy_message_cards;
    pub mod github;
    pub mod google_calendar;
    pub mod google_connections;
    pub mod google_drive;
    pub mod google_sign_in;
    pub mod internal_huddle;
    pub mod message_embed_suppressions;
    pub mod message_forwards;
    pub mod public_pages;
    pub mod pwa;
    pub mod saved_items;
    pub mod scheduled_messages;
    pub mod searches;
    pub mod slack;
    pub mod spa;
    pub mod switchers;
    pub mod unfurl_links;
    pub mod welcome;
    pub mod work_threads;
    pub mod workspace_icons;

    // The layers below, under the paths this code names them by.
    pub(crate) use campfire_messages::controllers::{message_features, messages};
    pub(crate) use campfire_people::controllers::{accounts, sessions, sudos, two_factor};
    pub(crate) use campfire_rooms::controllers::rooms;
    pub(crate) use campfire_web::controllers::presenters;
}

// The app, web and channels layers, under the paths this code used inside the campfire crate.
use campfire_app::{app, huddle, integrations, net, security};
#[cfg(any(test, feature = "test-support"))]
use campfire_app::test_support;
use campfire_channels::channels;
use campfire_runtime::{authentication, concerns, messaging, rich_text};
