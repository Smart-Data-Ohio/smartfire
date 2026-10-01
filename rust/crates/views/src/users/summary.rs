//! The user view-model shared by every users/accounts/autocompletable template.

use crate::helpers as h;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Role {
    #[default]
    Member,
    Administrator,
    Bot,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Member => "member",
            Role::Administrator => "administrator",
            Role::Bot => "bot",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Status {
    #[default]
    Active,
    Deactivated,
    Banned,
}

/// A `User` row as the views see it.
#[derive(Clone, Debug, Default)]
pub struct UserSummary {
    pub id: i64,
    pub name: String,
    pub bio: Option<String>,
    pub email_address: Option<String>,
    pub role: Role,
    pub status: Status,
    /// `fresh_user_avatar_path(user)`.
    pub avatar_path: String,
    pub two_factor_enabled: bool,
    /// Account-list controls; Calendar grants do not establish a login identity.
    pub google_identity_email: Option<String>,
    pub google_link_untrusted: bool,
}

impl UserSummary {
    pub fn sidebar_profile_button_attrs(&self) -> h::Attrs {
        h::attrs().type_("button").class("avatar profile-card-avatar").aria("label",format!("View profile of {}",self.name)).merge(h::profile_card_trigger(self.id,false))
    }
    pub fn two_factor_reset_confirmation(&self) -> String {
        format!("Reset two-step sign-in for {}? They will sign out everywhere and set it up again at next sign-in.",self.name)
    }
    pub fn google_unlink_title(&self) -> String {
        format!("Unlink Google sign-in ({})", self.google_identity_email.as_deref().unwrap_or_default())
    }
    pub fn google_unlink_confirmation(&self) -> String {
        format!("Unlink Google sign-in from {}? That Google account will no longer sign in as them.", self.name)
    }
    pub fn google_allow_title(&self) -> String {
        format!("Allow Google sign-in to link {}", self.email_address.as_deref().unwrap_or_default())
    }
    pub fn google_allow_confirmation(&self) -> String {
        format!("{} chose the email {} themselves. Allow the Google account with that address to sign in as them?", self.name, self.email_address.as_deref().unwrap_or_default())
    }
    pub fn google_allow_link(&self) -> bool {
        self.google_link_untrusted && self.email_address.as_ref().is_some_and(|email| !h::is_blank(email))
    }
    pub fn active(&self) -> bool { self.status == Status::Active }
    pub fn banned(&self) -> bool { self.status == Status::Banned }
    pub fn deactivated(&self) -> bool { self.status == Status::Deactivated }
    pub fn bot(&self) -> bool { self.role == Role::Bot }
    pub fn administrator(&self) -> bool { self.role == Role::Administrator }

    /// `User#title`.
    pub fn title(&self) -> String {
        h::user_title(&self.name, self.bio.as_deref())
    }

    /// `User#initials`.
    pub fn initials(&self) -> String {
        h::initials(&self.name)
    }

    pub fn avatar(&self) -> h::AvatarUser {
        h::AvatarUser { id: self.id, title: self.title(), avatar_path: self.avatar_path.clone() }
    }

    /// `name.split(' ')[0]` (awk-style split on ASCII whitespace).
    pub fn first_name(&self) -> &str {
        self.name_parts().next().unwrap_or("")
    }

    /// `name.split(' ')`.
    pub fn name_parts(&self) -> impl Iterator<Item = &str> {
        self.name.split(|c: char| c.is_ascii_whitespace()).filter(|part| !part.is_empty())
    }
}
