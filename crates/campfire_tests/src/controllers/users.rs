//! `controllers::users` lives in the campfire_people crate (plans/crate-split-plan.md). Its tests
//! that need the whole app are mounted here, at their old path.

#[cfg(test)]
mod preferences_tests;

#[cfg(test)]
pub(crate) mod people_tests;

#[cfg(test)]
mod profile_settings_tests;

#[cfg(test)]
mod profile_page_tests;

#[cfg(test)]
mod joining_tests;

#[cfg(test)]
mod profile_sections_tests;

#[cfg(test)]
mod status_popup_tests;

#[cfg(test)]
mod profile_security_tests;

#[cfg(test)]
mod ban_lifecycle_tests;

#[cfg(test)]
mod layout_preferences_tests;

#[cfg(test)]
mod profile_effective_ooo_tests;

#[cfg(test)]
mod agent_profile_tests;

#[cfg(test)]
mod fizzy_profile_tests;

#[cfg(test)]
mod stars_tests;

#[cfg(test)]
mod cutover_receipts_tests;

#[cfg(test)]
mod sidebar_original_tests;

#[cfg(test)]
mod profile_gap_original_tests;

#[cfg(test)]
pub(crate) mod avatars {
    pub(crate) use campfire_people::controllers::users::avatars::*;

    #[path = "../avatar_image_tests.rs"]
    mod avatar_image_tests;
    mod tests;
    mod original_tests;
}

#[cfg(test)]
pub(crate) mod dnd_allowances {
    mod tests;
}

#[cfg(test)]
pub(crate) mod notification_settings {
    pub(crate) use campfire_people::controllers::users::notification_settings::*;

    mod tests;
}

#[cfg(test)]
pub(crate) mod presences {
    pub(crate) use campfire_people::controllers::users::presences::*;

    mod tests;
}

#[cfg(test)]
pub(crate) mod profiles {
    mod ws17_tests;
}

#[cfg(test)]
pub(crate) mod push_subscriptions {
    mod ws17_tests;

    pub(crate) mod test_notifications {
        use campfire_db::models::push_subscription::TestNotificationJob;

        mod tests;
    }
}

#[cfg(test)]
pub(crate) mod sidebars {
    pub(crate) use campfire_people::controllers::users::sidebars::*;

    #[path = "../sidebars_tests.rs"]
    mod tests;
    #[path = "tests.rs"]
    mod composition_tests;
}

#[cfg(test)]
pub(crate) mod statuses {
    mod tests;
}
