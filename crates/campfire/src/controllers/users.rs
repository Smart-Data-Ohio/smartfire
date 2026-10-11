//! `controllers::users` lives in the campfire_people crate (plans/crate-split-plan.md). This
//! module re-exports it under its old path and mounts the tests that still need the whole app.

pub use campfire_people::controllers::users::*;

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
mod workspace_invite_tests;

#[cfg(test)]
mod join_contract_tests;


#[cfg(test)]
mod status_popup_tests;

#[cfg(test)]
mod ban_lifecycle_tests;





#[cfg(test)]
mod stars_tests;

#[cfg(test)]
mod cutover_receipts_tests;


#[cfg(test)]
mod profile_gap_original_tests;

#[cfg(test)]
pub(crate) mod avatars {
    pub(crate) use campfire_people::controllers::users::avatars::*;

    #[path = "../avatar_image_tests.rs"]
    mod avatar_image_tests;
    mod original_tests;
    mod tests;
}

#[cfg(test)]
pub(crate) mod dnd_allowances {
    pub(crate) use campfire_people::controllers::users::dnd_allowances::*;

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
    pub(crate) use campfire_people::controllers::users::profiles::*;

    mod ws17_tests;
}

#[cfg(test)]
pub(crate) mod push_subscriptions {
    pub(crate) use campfire_people::controllers::users::push_subscriptions::*;

    mod ws17_tests;

    pub(crate) mod test_notifications {
        use campfire_db::models::push_subscription::TestNotificationJob;
        pub(crate) use campfire_people::controllers::users::push_subscriptions::test_notifications::*;


        mod tests;
}
}



#[cfg(test)]
pub(crate) mod statuses {
    pub(crate) use campfire_people::controllers::users::statuses::*;

    mod tests;
}
