//! The Campfire server: controllers, channels, jobs and integrations wired over the crates.

mod admin;
mod channels;
mod controllers;
mod jobs;
// Boot, the HTTP stack and the binary's commands.
mod server;

// The app layer (crates/app), at the paths its modules had in this crate. `app`, `huddle` and
// `integrations` also hold the tests of theirs that boot the whole app or reach the layers
// above, until the test crate takes them (plans/crate-split-plan.md, "Tests").
use campfire_app::{account_security, cable, config, errors, net, queue, security};
#[cfg(test)]
use campfire_app::{state, test_support};

// The web layer (crates/web), likewise. `concerns`, `mail` and `controllers::presenters` (with
// `controllers::messages::rendered`, mirrored in `controllers`) also hold tests of theirs that
// boot the whole app.
use campfire_web::{active_storage, authentication, messaging, rich_text};

mod concerns {
    pub use campfire_web::concerns::*;

    #[cfg(test)]
    mod bot_model_cases;
}

mod mail {
    pub(crate) use campfire_web::mail::*;

    #[cfg(test)]
    mod tests;
}

mod app {
    pub(crate) use campfire_app::app::*;

    #[cfg(test)]
    mod security_tests;
    #[cfg(test)]
    mod sudo_tests;
    #[cfg(test)]
    mod two_factor_tests;
    #[cfg(test)]
    mod challenge_tests;
    #[cfg(test)]
    mod enforcement_tests;
    #[cfg(test)]
    mod direct_upload_tests;
    #[cfg(test)]
    mod session_management_tests;
    #[cfg(test)]
    mod admin_two_factor_tests;
    #[cfg(test)]
    mod full_page_tests;
    #[cfg(test)]
    mod profile_security_tests;
    #[cfg(test)]
    mod round_four_security_tests;
    #[cfg(test)]
    mod round_three_security_tests;
    #[cfg(test)]
    mod tests;
    #[cfg(test)]
    mod google_tests;
    #[cfg(test)]
    mod google_webhook_tests;
    #[cfg(test)]
    pub(crate) mod google_api_tests;
    #[cfg(test)]
    mod google_connection_tests;
    #[cfg(test)]
    mod google_drive_tests;
    #[cfg(test)]
    mod google_calendar_job_tests;
    #[cfg(test)]
    mod google_meeting_refresh_tests;
    #[cfg(test)]
    mod ws14_profile_status_tests;
    #[cfg(test)]
    mod google_push_channel_tests;
    #[cfg(test)]
    pub(crate) mod google_test_support;
    #[cfg(test)]
    #[path = "../../../../test-support/asset_goldens.rs"]
    pub(crate) mod asset_goldens;
    #[cfg(test)]
    mod google_review_tests;
    #[cfg(test)]
    mod google_consumer_tests;
    #[cfg(test)]
    mod google_lifecycle_tests;
    #[cfg(test)]
    mod google_admin_tests;
    #[cfg(test)]
    mod google_page_tests;
    #[cfg(test)]
    mod google_reporting_tests;
    #[cfg(test)]
    mod google_message_tests;
    #[cfg(test)]
    pub(crate) mod cutover_c_tests;
    #[cfg(test)]
    mod cutover_d_tests;
}

mod huddle {
    pub(crate) use campfire_app::huddle::*;

    #[cfg(test)]
    mod tests;
}

mod integrations {
    pub(crate) use campfire_app::integrations::*;

    #[cfg(test)]
    pub(crate) mod action_claims {
        pub(crate) use campfire_app::integrations::action_claims::*;

        mod tests;
    }
    #[cfg(test)]
    pub(crate) mod agent_repositories {
        pub(crate) use campfire_app::integrations::agent_repositories::*;

        mod tests;
        mod live_tests;
        mod bot_plaintext_cases;
        mod review_tests;
    }
    #[cfg(test)]
    pub(crate) mod agent_streaming {
        pub(crate) use campfire_app::integrations::agent_streaming::*;

        mod tests;
        mod case_tests;
    }
    pub(crate) mod fizzy {
        pub(crate) use campfire_app::integrations::fizzy::*;

        #[cfg(test)]
        mod tests;
        #[cfg(test)]
        pub(crate) mod accounts {
            pub(crate) use campfire_app::integrations::fizzy::accounts::*;

            mod tests;
        }
        #[cfg(test)]
        pub(crate) mod agent_job {
            pub(crate) use campfire_app::integrations::fizzy::agent_job::*;

            mod tests;
        }
        #[cfg(test)]
        pub(crate) mod agent_reads {
            pub(crate) use campfire_app::integrations::fizzy::agent_reads::*;

            mod tests;
        }
        #[cfg(test)]
        pub(crate) mod agent_requests {
            pub(crate) use campfire_app::integrations::fizzy::agent_requests::*;

            mod tests;
        }
        #[cfg(test)]
        pub(crate) mod cards {
            pub(crate) use campfire_app::integrations::fizzy::cards::*;

            mod tests;
        }
    }
    pub(crate) mod github {
        pub(crate) use campfire_app::integrations::github::*;

        #[cfg(test)]
        pub(crate) mod tests;
        #[cfg(test)]
        pub(crate) mod agent_actions {
            pub(crate) use campfire_app::integrations::github::agent_actions::*;

            mod tests;
        }
        #[cfg(test)]
        pub(crate) mod fetcher {
            pub(crate) use campfire_app::integrations::github::fetcher::*;

            mod tests;
        }
        #[cfg(test)]
        pub(crate) mod health {
            pub(crate) use campfire_app::integrations::github::health::*;

            mod tests;
        }
        #[cfg(test)]
        pub(crate) mod notifier {
            pub(crate) use campfire_app::integrations::github::notifier::*;

            mod tests;
        }
        #[cfg(test)]
        pub(crate) mod pull_requests {
            pub(crate) use campfire_app::integrations::github::pull_requests::*;

            mod tests;
        }
        #[cfg(test)]
        pub(crate) mod references {
            pub(crate) use campfire_app::integrations::github::references::*;

            pub(super) mod tests;
        }
    }
    #[cfg(test)]
    pub(crate) mod image_proxy {
        pub(crate) use campfire_app::integrations::image_proxy::*;

        mod tests;
    }
    pub(crate) mod link_embed {
        pub(crate) use campfire_app::integrations::link_embed::*;

        #[cfg(test)]
        mod rails_reference_tests;
        #[cfg(test)]
        mod rails_fetcher_tests;
        #[cfg(test)]
        pub(crate) mod fetcher {
            pub(crate) use campfire_app::integrations::link_embed::fetcher::*;

            mod tests;
        }
        #[cfg(test)]
        pub(crate) mod store {
            pub(crate) use campfire_app::integrations::link_embed::store::*;

            mod tests;
        }
    }
    pub(crate) mod slack {
        pub(crate) use campfire_app::integrations::slack::*;

        #[cfg(test)]
        mod sequence_tests;
        #[cfg(test)]
        pub(crate) mod client {
            pub(crate) use campfire_app::integrations::slack::client::*;

            pub(crate) mod tests;
        }
        #[cfg(test)]
        pub(crate) mod conversations {
            pub(crate) use campfire_app::integrations::slack::conversations::*;

            mod tests;
        }
        #[cfg(test)]
        pub(crate) mod jobs {
            pub(crate) use campfire_app::integrations::slack::jobs::*;

            pub(crate) mod tests;
        }
        #[cfg(test)]
        pub(crate) mod oauth {
            pub(crate) use campfire_app::integrations::slack::oauth::*;

            mod tests;
        }
        #[cfg(test)]
        mod payload {
            mod tests;
        }
        #[cfg(test)]
        pub(crate) mod runner {
            pub(crate) use campfire_app::integrations::slack::runner::*;

            mod tests;
        }
        #[cfg(test)]
        pub(crate) mod store {
            pub(crate) use campfire_app::integrations::slack::store::*;

            pub(crate) mod tests;
            mod lifecycle_tests;
        }
        #[cfg(test)]
        pub(crate) mod undoer {
            pub(crate) use campfire_app::integrations::slack::undoer::*;

            pub(crate) mod tests;
        }
        #[cfg(test)]
        pub(crate) mod users {
            pub(crate) use campfire_app::integrations::slack::users::*;

            mod tests;
        }
        #[cfg(test)]
        pub(crate) mod writer {
            pub(crate) use campfire_app::integrations::slack::writer::*;

            mod tests;
        }
    }
    pub(crate) mod twitter {
        pub(crate) use campfire_app::integrations::twitter::*;

        #[cfg(test)]
        pub(crate) mod fetcher {
            pub(crate) use campfire_app::integrations::twitter::fetcher::*;

            mod tests;
        }
        #[cfg(test)]
        pub(crate) mod references {
            pub(crate) use campfire_app::integrations::twitter::references::*;

            mod tests;
        }
    }
    pub(crate) mod web_push {
        pub(crate) use campfire_app::integrations::web_push::*;

        #[cfg(test)]
        mod tests;
    }
}

/// jemalloc: the room page alone makes thousands of allocations per request, across as many
/// threads as the blocking pool grows to.
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

/// jemalloc's options (`malloc_conf`, under tikv-jemallocator's `_rjem_` prefix), read when it
/// starts, before `main`: no transparent huge pages for its regions either (see
/// [`disable_transparent_huge_pages`]). jemalloc declares it `const char *`, so it's a thin pointer,
/// declared as tikv-jemalloc-sys does.
#[cfg(target_os = "linux")]
#[unsafe(export_name = "_rjem_malloc_conf")]
pub static JEMALLOC_CONF: Option<&'static std::ffi::c_char> =
    // SAFETY: points at the first byte of a static, NUL-terminated string.
    Some(unsafe { &*c"thp:never".as_ptr() });

fn main() -> anyhow::Result<()> {
    disable_transparent_huge_pages();
    if let Some(status) = admin::run(&std::env::args().skip(1).collect::<Vec<_>>()) {
        std::process::exit(status);
    }
    server::run()
}

/// On kernels with transparent huge pages set to `always` (Debian's and Arch's default), every
/// thread's 2 MB stack and each of jemalloc's regions get backed by whole 2 MB pages as soon as
/// they're touched: an idle server took 160 MB on 32 cores instead of 15 MB. Nothing here is big
/// enough to gain from huge pages, so the process (and ffmpeg, which inherits it) opts out.
fn disable_transparent_huge_pages() {
    #[cfg(target_os = "linux")]
    {
        let off: libc::c_ulong = 1;
        // SAFETY: PR_SET_THP_DISABLE takes machine-width integer arguments and only changes this
        // process's memory policy.
        if unsafe { libc::prctl(libc::PR_SET_THP_DISABLE, off, 0 as libc::c_ulong, 0 as libc::c_ulong, 0 as libc::c_ulong) } != 0 {
            eprintln!("couldn't disable transparent huge pages: {}", std::io::Error::last_os_error());
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    #[test]
    fn transparent_huge_pages_are_disabled() {
        super::disable_transparent_huge_pages();
        let zero: libc::c_ulong = 0;
        // SAFETY: as in `disable_transparent_huge_pages`.
        assert_eq!(unsafe { libc::prctl(libc::PR_GET_THP_DISABLE, zero, zero, zero, zero) }, 1);
    }

    #[test]
    fn jemalloc_reads_its_options() {
        let mut thp: *const std::ffi::c_char = std::ptr::null();
        let mut len = std::mem::size_of_val(&thp);
        // SAFETY: `opt.thp` is a `const char *`, written into a variable of that type and size.
        let status = unsafe {
            tikv_jemalloc_sys::mallctl(c"opt.thp".as_ptr(), (&raw mut thp).cast(), &mut len, std::ptr::null_mut(), 0)
        };
        assert_eq!(status, 0);
        // SAFETY: jemalloc returned a pointer to one of its static option names.
        assert_eq!(unsafe { std::ffi::CStr::from_ptr(thp) }, c"never");
    }
}

#[cfg(test)]
mod layering_tests;
#[cfg(test)]
mod slash_commands_tests;

