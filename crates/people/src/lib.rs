//! The people controllers of the Campfire server: users (profiles, avatars, statuses,
//! sidebars, push subscriptions and the rest), the account settings and bots, sign-in sessions,
//! sudo mode, two-factor and its QR codes. The crates above route to them
//! (plans/crate-split-plan.md). Modules keep their paths from the campfire crate, so
//! `controllers::users` lives at `campfire_people::controllers::users`.

pub mod controllers {
    pub mod accounts;
    pub mod qr_code;
    pub mod sessions;
    pub mod sudos;
    pub mod two_factor;
    pub mod users;

    // The web layer's presenters, under the path this code names them by.
    pub(crate) use campfire_web::controllers::presenters;
}

// The app, web and channels layers, under the paths this code used inside the campfire crate.
use campfire_app::{account_security, app, integrations, net};
use campfire_runtime::{authentication, concerns, rich_text};
