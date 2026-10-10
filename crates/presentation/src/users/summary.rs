// The user view-model shared by every users/accounts/autocompletable template.

use crate::helpers as h;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Role {
    #[default]
    Member,
    Administrator,
    Bot,
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
    /// Read-only WS14 Google sign-in metadata for account member controls.
    pub google_identity_email: Option<String>,
    pub email_self_changed: bool,
    pub google_email_link_allowed: bool,
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

impl UserSummary {
    pub fn two_factor_reset_confirmation(&self) -> String {
        format!(
            "Reset two-step sign-in for {}? They will sign out everywhere and set it up again at next sign-in.",
            self.name
        )
    }
    pub fn offer_google_email_link(&self) -> bool {
        (self.email_self_changed || !self.google_email_link_allowed)
            && self
                .email_address
                .as_deref()
                .is_some_and(|email| !h::is_blank(email))
    }
    pub fn google_unlink_title(&self, email: &str) -> String {
        format!("Unlink Google sign-in ({email})")
    }
    pub fn google_unlink_confirmation(&self) -> String {
        format!(
            "Unlink Google sign-in from {}? That Google account will no longer sign in as them.",
            self.name
        )
    }
    pub fn google_link_title(&self) -> String {
        format!(
            "Allow Google sign-in to link {}",
            self.email_address.as_deref().unwrap_or_default()
        )
    }
    pub fn google_link_confirmation(&self) -> String {
        format!(
            "{} chose the email {} themselves. Allow the Google account with that address to sign in as them?",
            self.name,
            self.email_address.as_deref().unwrap_or_default()
        )
    }
    pub fn active(&self) -> bool {
        self.status == Status::Active
    }
    pub fn banned(&self) -> bool {
        self.status == Status::Banned
    }
    pub fn deactivated(&self) -> bool {
        self.status == Status::Deactivated
    }
    pub fn bot(&self) -> bool {
        self.role == Role::Bot
    }
    pub fn administrator(&self) -> bool {
        self.role == Role::Administrator
    }

    /// `name.split(' ')[0]` (awk-style split on ASCII whitespace).
    pub fn first_name(&self) -> &str {
        self.name_parts().next().unwrap_or("")
    }

    /// `name.split(' ')`.
    pub fn name_parts(&self) -> impl Iterator<Item = &str> {
        self.name
            .split(|c: char| c.is_ascii_whitespace())
            .filter(|part| !part.is_empty())
    }
}
