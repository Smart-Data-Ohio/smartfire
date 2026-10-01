//! Views for `reference/app/views/users`.

use askama::Template;

use crate::ViewContext;
use crate::accounts::HelpContact;
use crate::helpers::{self as h, filters};
use crate::layouts::Page;

pub mod google;
mod sidebar;
pub use sidebar::*;
mod summary;
pub use summary::*;
mod settings;
pub use settings::*;
pub mod statuses;

#[derive(Clone)]
pub struct UserSession {
    pub id: i64,
    pub current: bool,
    pub description: String,
    pub ip_address: Option<String>,
    pub last_active_at: jiff::Timestamp,
    pub created_at: jiff::Timestamp,
}
#[derive(Template)]
#[template(path="users/sessions/index.html",blocks=["head","nav","content"])]
pub struct SessionsIndex<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub sessions: Vec<UserSession>,
    pub now: jiff::Timestamp,
}
impl Page for SessionsIndex<'_> {
    fn page_title(&self) -> Option<String> { Some("Your sessions".into()) }
}
impl SessionsIndex<'_> {
    fn ip<'s>(&self,s: &'s UserSession) -> Option<&'s str> { s.ip_address.as_deref().filter(|v| !v.chars().all(char::is_whitespace)) }
    fn last_active(&self,s: &UserSession) -> String { h::time_ago_in_words(&self.ctx.time_zone,s.last_active_at,self.now) }
    fn signed_in(&self,s: &UserSession) -> h::Html {
        h::local_datetime_tag(&self.ctx.time_zone,s.created_at,"date",h::attrs(),&self.ctx.time_zone.to_fs(s.created_at,"short"))
    }
}

#[derive(Template)]
#[template(path="users/profiles/_sessions.html")]
pub struct ProfileSessions<'a> { pub ctx: &'a ViewContext<'a> }

/// `users/new.html.erb` (the join page).
#[derive(Template)]
#[template(path = "users/new.html", blocks = ["head", "content"])]
pub struct New<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub join_code: String,
    pub help_contact: Option<HelpContact>,
}

impl Page for New<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Sign up".into())
    }
    fn body_class(&self) -> Option<&str> {
        Some("signup")
    }
}

/// `users/show.html.erb`.
#[derive(Template)]
#[template(path = "users/show.html", blocks = ["head", "nav", "content"])]
pub struct Show<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub user: UserSummary,
    /// `user.transfer_id`, for `users/profiles/_transfer` (shown to administrators).
    pub transfer_id: String,
    pub profile_status: Option<statuses::ProfileStatus>,
}

impl Show<'_> {
    fn status_section(&self, status: &statuses::ProfileStatus) -> h::Html {
        h::raw(
            statuses::ProfileStatusSection { status }
                .render()
                .expect("profile status"),
        )
    }
    fn allowance(&self, status: &statuses::ProfileStatus) -> h::Html {
        h::raw(
            statuses::DndAllowance {
                ctx: self.ctx,
                status,
            }
            .render()
            .expect("DND allowance"),
        )
    }
}

impl Page for Show<'_> {
    fn page_title(&self) -> Option<String> {
        Some(self.user.name.clone())
    }
}

/// `users/_ban_button.html.erb` on its own.
#[derive(Template)]
#[template(path = "users/_ban_button.html")]
pub struct BanButton<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub user: UserSummary,
}

/// A user as `users/_mention` and the autocompletable views see it.
#[derive(Clone, Debug, Default)]
pub struct MentionUser {
    pub user: UserSummary,
    /// `user.attachable_sgid`.
    pub attachable_sgid: String,
}

impl std::ops::Deref for MentionUser {
    type Target = UserSummary;
    fn deref(&self) -> &UserSummary {
        &self.user
    }
}

/// `users/_mention.html.erb`: the mention attachment's HTML.
#[derive(Template)]
#[template(path = "users/_mention.html")]
pub struct Mention<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub user: MentionUser,
}

/// `users/autocompletables/_template.html.erb`.
#[derive(Template)]
#[template(path = "users/autocompletables/_template.html")]
pub struct AutocompletableTemplate<'a> {
    pub ctx: &'a ViewContext<'a>,
}

/// `users/avatars/show.svg.erb`: the initials avatar for users without an uploaded one.
#[derive(Template)]
#[template(path = "users/avatars/show.svg")]
pub struct AvatarSvg {
    pub user_id: i64,
    /// `User#initials`.
    pub initials: String,
}

/// A membership row on the profile (`users/profiles/_membership`).
#[derive(Clone, Debug)]
pub struct ProfileMembership {
    pub room_id: i64,
    /// "rooms_open", "rooms_closed" or "rooms_direct".
    pub room_param_key: String,
    /// `room_display_name(membership.room)`.
    pub room_display_name: String,
    pub involvement: String,
    pub direct: bool,
}

impl ProfileMembership {
    pub fn involvement_room(&self) -> h::InvolvementRoom<'_> {
        h::InvolvementRoom {
            id: self.room_id,
            param_key: &self.room_param_key,
            direct: self.direct,
        }
    }
}

/// `users/profiles/show.html.erb`.
#[derive(Template)]
#[template(path = "users/profiles/show.html", blocks = ["head", "content"])]
pub struct ProfileShow<'a> {
    pub google_calendar: google::CalendarData,
    pub google_sign_in: google::SignInData,
    pub github: crate::github::connections::Connection,
    pub has_password: bool,
    pub current_password_error: Option<&'a str>,
    pub security: crate::two_factor::ProfileData,
    pub now: jiff::Timestamp,
    pub ctx: &'a ViewContext<'a>,
    pub user: UserSummary,
    pub avatar_attached: bool,
    pub transfer_id: String,
    pub shared_memberships: Vec<ProfileMembership>,
    pub direct_memberships: Vec<ProfileMembership>,
    pub settings: SettingsFormData,
}

impl<'a> ProfileShow<'a> {
    fn google_calendar_panel(&self) -> h::Html { h::raw(google::Calendar{data:self.google_calendar.clone()}.render().unwrap()) }
    fn google_sign_in_panel(&self) -> h::Html { h::raw(google::SignIn{data:self.google_sign_in.clone()}.render().unwrap()) }
    fn github_panel(&self) -> h::Html {
        h::raw(crate::github::connections::profile(&self.github))
    }
    fn status_form(&self) -> h::Html {
        h::raw(StatusForm { ctx: self.ctx, data: &self.settings }.render().expect("status form renders"))
    }
    fn notification_form(&self) -> h::Html {
        h::raw(NotificationForm { ctx: self.ctx, data: &self.settings }.render().expect("notification form renders"))
    }
    fn appearance_form(&self) -> h::Html {
        h::raw(AppearanceForm { ctx: self.ctx, data: &self.settings }.render().expect("appearance form renders"))
    }
    fn security_panel(&self) -> h::Html {
        h::raw(
            crate::two_factor::Profile {
                ctx: self.ctx,
                data: self.security.clone(),
                now: self.now,
            }
            .render()
            .unwrap(),
        )
    }
    /// `profile_form_with(@user, **params)`.
    fn profile_form(&self) -> h::FormWith {
        h::form_with(h::routes::user_profile())
            .model("user")
            .method("patch")
            .data("controller", "form")
    }
}

impl Page for ProfileShow<'_> {
    fn page_title(&self) -> Option<String> {
        Some(self.user.name.clone())
    }
}

/// `users/profiles/_transfer.html.erb` on its own.
#[derive(Template)]
#[template(path = "users/profiles/_transfer.html")]
pub struct Transfer<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub user: UserSummary,
    pub transfer_id: String,
}

/// A `Push::Subscription`, with its user agent parsed (`UserAgent.parse`).
#[derive(Clone, Debug)]
pub struct PushSubscription {
    pub id: i64,
    pub endpoint: String,
    pub browser: String,
    pub version: String,
    pub platform: String,
}

/// `users/push_subscriptions/index.html.erb`.
#[derive(Template)]
#[template(path = "users/push_subscriptions/index.html", blocks = ["head", "content"])]
pub struct PushSubscriptionsIndex<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub push_subscriptions: Vec<PushSubscription>,
}

impl Page for PushSubscriptionsIndex<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Push notification subscriptions".into())
    }
}
